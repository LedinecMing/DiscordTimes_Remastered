// Экран карты: запечённая карта, камера (пан/зум/хоткеи), армии, HUD, переходы.
// Порт quad_ui main.rs:1802-2104 (+ rebake ветки из главного цикла 2302-2332).
use crate::bake::{bake_map_textures, render_decos_layer};
use crate::camera::{default_camera, Camera};
use crate::gfx::{colors, Target, WHITE};
use crate::Ctx;
use crate::state::{BuildingTab, Menu, MapRenderSettings, SIZE};
use dt_lib::battle::battlefield::BattleInfo;
use dt_lib::battle::control::Control;
use dt_lib::map::event::Execute;
use dt_lib::network::server::ClientMessage;
use dt_lib::units::unit::UnitType;
use winit::keyboard::KeyCode;

/// Максимальное приближение карты: кратное от вида «вся карта на экране».
const ZOOM_IN: f32 = 12.;

/// Кламп зума карты: с лимитами — «вся карта..ZOOM_IN×», без — legacy [8e-6; 0.02].
fn clamp_map_zoom(camera: &mut Camera, limited: bool, min: [f32; 2], max: [f32; 2]) {
    if limited {
        camera.clamp_zoom_between(min, max);
    } else {
        camera.clamp_zoom();
    }
}

pub fn map_screen(ctx: &mut Ctx) {
    let mut settings = match &ctx.ui.main {
        Menu::Map(s) => s.clone(),
        _ => return,
    };
    // Восстановление после просмотра событий: приоритет — сохранённая копия.
    if let Some(saved) = ctx.map_settings.take() {
        settings = saved;
    }
    // Rebake по флагам (порт веток decos_dirty/blend_dirty главного цикла).
    if settings.decos_dirty {
        render_decos_layer(
            ctx.gfx,
            &ctx.rts.1,
            ctx.assets,
            &ctx.game.executor.gamemap,
            ctx.registry,
            settings.seed,
            settings.buildings_render,
            settings.armies_render,
        );
        settings.decos_dirty = false;
    }
    if settings.blend_dirty {
        bake_map_textures(
            ctx.gfx,
            &ctx.rts.0,
            ctx.assets,
            &ctx.game.executor.gamemap,
            ctx.tile_pixels,
            settings.blend_mode,
        );
        settings.blend_dirty = false;
    }
    let next = draw_map(ctx, &mut settings);
    match next {
        Some(menu) => {
            // Уход с экрана: сохраняем камеру/флаги для возврата без сброса.
            *ctx.map_settings.borrow_mut() = Some(settings.clone());
            ctx.ui.main = menu;
        }
        None => ctx.ui.main = Menu::Map(settings),
    }
}

fn draw_map(ctx: &mut Ctx, settings: &mut MapRenderSettings) -> Option<Menu> {
    let viewport = ctx.input.screen_size();
    ctx.gfx
        .begin_pass(Target::Screen, Some(colors::WHITE), &settings.camera);

    let size = ctx.game.executor.gamemap.tilemap.size as u32;
    // prerender-путь (в исходнике prerender == true; мёртвая ветка без запечки удалена)
    let world_w = SIZE.0 * size as f32;
    let world_h = SIZE.1 * size as f32;
    ctx.gfx.draw_texture(ctx.textures.map, 0., 0., world_w, world_h, WHITE);
    ctx.gfx
        .draw_texture(ctx.textures.decos, 0., 0., world_w, world_h, WHITE);
    // Зелёная обводка строения: выбранного ЛКМ и цели дабл-клика.
    if let Some(building) = ctx.building_ui.lmb_building {
        outline_building(ctx, building, [0., 1., 0., 0.9]);
    }
    if let Some(building) = ctx.building_ui.pending_open {
        outline_building(ctx, building, [0., 1., 0.3, 0.9]);
    }
    // Подсветка тайла последнего клика GoTo: цель ходьбы видна явно.
    if let Some(tile) = ctx.building_ui.goto_tile {
        let pulse = 0.5 - 0.5 * (crate::time_secs() as f32 * std::f32::consts::TAU / 1.2).cos();
        ctx.gfx.draw_rect_lines(
            tile[0] as f32 * SIZE.0,
            tile[1] as f32 * SIZE.1,
            SIZE.0,
            SIZE.1,
            3.,
            crate::gfx::rgba(255, 220, 40, (140. + 100. * pulse) as u8),
        );
    }

    if settings.event_render {
        for i in 0..size {
            for j in 0..size {
                let events = &ctx.game.executor.gamemap.eventmap[(j as usize, i as usize)];
                if !events.is_empty() {
                    ctx.gfx.draw_rect(
                        i as f32 * SIZE.0,
                        j as f32 * SIZE.1,
                        SIZE.0,
                        SIZE.1,
                        colors::BLUE,
                    );
                }
            }
        }
    }
    // Слой принадлежности тайлов строениям (F9): полупрозрачная заливка
    // поверх террейна, до армий. Каждое строение — свой цвет из HSV-хеша
    // индекса; пикингу кликов не мешает (слой только рисуется).
    if settings.ownership_render {
        let map_size = ctx.game.executor.gamemap.hitmap.size;
        for (flat, hit) in ctx.game.executor.gamemap.hitmap.inner.iter().enumerate() {
            if let Some(building) = hit.building {
                // TileMap::index((x, y)) = inner[y + x*size], значит по linear
                // индексу x = flat / size, y = flat % size (НЕ наоборот —
                // перепутанные местами координаты зеркалили слой по диагонали,
                // баг «слой смещён»). Индексация как у eventmap/goto_tile.
                let (tx, ty) = (flat / map_size, flat % map_size);
                let [r, g, b, _] = building_ownership_color(building);
                ctx.gfx.draw_rect(
                    tx as f32 * SIZE.0,
                    ty as f32 * SIZE.1,
                    SIZE.0,
                    SIZE.1,
                    [r, g, b, 0.35],
                );
            }
        }
    }
    if settings.armies_render {
        let change = (crate::time_secs() * 1000. / 50.) as usize % 8;
        // Пересчёт как в оригинале: проход по hitmap, для каждой армии — кадр анимации.
        let mut army_ids: Vec<usize> = Vec::new();
        for hit in ctx.game.executor.gamemap.hitmap.inner.iter() {
            if let Some(army) = hit.army {
                if !army_ids.contains(&army) {
                    army_ids.push(army);
                }
            }
        }
        for army_idx in army_ids {
            let army = &ctx.game.executor.gamemap.armys[army_idx];
            let (i, j) = army.pos;
            let new = army.path.get(0).unwrap_or(&army.pos);
            let step = (new.0 as isize - i as isize, new.1 as isize - j as isize);
            let frame = match step {
                (-1, -1) => change,
                (0, -1) => 8 + change,
                (1, -1) => 16 + change,
                (1, 0) => 24 + change,
                (1, 1) => 32 + change,
                (0, 0) => 40,
                (0, 1) => 40 + change,
                (-1, 1) => 48 + change,
                (-1, 0) => 56 + change,
                _ => 40,
            };
            let unittype = army
                .troops
                .get(0)
                .and_then(|tr| {
                    Some(tr.get().unit.get_info(&ctx.registry.units).unit_type)
                })
                .unwrap_or(UnitType::People);
            let pic = format!(
                "{}/Unit_{frame}.png",
                match (&army.control, unittype) {
                    (Control::Player(_), _) => "ГГследопыт",
                    (_, UnitType::Undead) => "мертвяк",
                    (_, UnitType::Rogue) => "разбойник",
                    (_, UnitType::Hero | UnitType::People) => "феодал",
                    _ => "некромант",
                }
            );
            let texture = ctx.assets.get(&pic);
            ctx.gfx.draw_texture(
                texture,
                i as f32 * SIZE.0,
                j as f32 * SIZE.1 - SIZE.1,
                SIZE.0,
                SIZE.1 * 2.,
                WHITE,
            );
        }
    }

    // ---- хоткеи слоёв (порт is_key_pressed блоков) ----
    if ctx.input.key_pressed(KeyCode::KeyT) {
        settings.tiles_render = !settings.tiles_render;
    }
    if ctx.input.key_pressed(KeyCode::KeyF) {
        settings.armies_render = !settings.armies_render;
    }
    if ctx.input.key_pressed(KeyCode::KeyY) {
        settings.deco_render = !settings.deco_render;
    }
    if ctx.input.key_pressed(KeyCode::KeyE) {
        settings.err_render = !settings.err_render;
    }
    if ctx.input.key_pressed(KeyCode::KeyB) {
        settings.buildings_render = !settings.buildings_render;
        settings.decos_dirty = true;
    }
    if ctx.input.key_pressed(KeyCode::KeyN) {
        settings.event_render = !settings.event_render;
    }
    if ctx.input.key_pressed(KeyCode::F9) {
        settings.ownership_render = !settings.ownership_render;
    }
    if ctx.input.key_pressed(KeyCode::KeyR) {
        settings.seed = settings
            .seed
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        settings.decos_dirty = true;
    }
    if ctx.input.key_pressed(KeyCode::KeyG) {
        settings.blend_mode = settings.blend_mode.next();
        settings.blend_dirty = true;
    }
    if ctx.input.key_pressed(KeyCode::KeyZ) {
        settings.zoom_limits = !settings.zoom_limits;
    }
    // Переход к бою по Enter на карте УБРАН (пользователь): запускается
    // только через PvE в меню или PvP-сетап.
    if ctx.input.key_down(KeyCode::Escape) {
        *ctx.map_settings.borrow_mut() = Some(settings.clone());
        return Some(Menu::Main);
    }

    // ---- камера: WASD, правая кнопка, колесо (порт 1989-2040) ----
    let camera = &mut settings.camera;
    let sw = ctx.input.screen_width();
    let sh = ctx.input.screen_height();
    let px = camera.zoom[0] * sw * 0.5;
    let py = camera.zoom[1] * sh * 0.5;
    let pan_speed = 15.;
    if ctx.input.key_down(KeyCode::KeyW) {
        camera.target[1] -= pan_speed / py;
    }
    if ctx.input.key_down(KeyCode::KeyS) {
        camera.target[1] += pan_speed / py;
    }
    if ctx.input.key_down(KeyCode::KeyA) {
        camera.target[0] -= pan_speed / px;
    }
    if ctx.input.key_down(KeyCode::KeyD) {
        camera.target[0] += pan_speed / px;
    }
    if ctx.input.mouse_button_down(1) {
        let delta = ctx.input.mouse_delta;
        camera.target[0] -= delta[0] / px;
        camera.target[1] -= delta[1] / py;
    }
    // Границы зума: min — вся карта в кадре; max — ZOOM_IN-кратное приближение.
    let min_zoom = [2. / (SIZE.0 * size as f32), 2. / (SIZE.1 * size as f32)];
    let max_zoom = [min_zoom[0] * ZOOM_IN, min_zoom[1] * ZOOM_IN];
    let wheel = ctx.input.wheel;
    if wheel != 0. {
        let factor = 1.2f32.powf(wheel.clamp(-4., 4.));
        let anchor = camera.screen_to_world(ctx.input.mouse_position(), viewport);
        let zoom_before_x = camera.zoom[0];
        camera.zoom[0] *= factor;
        camera.zoom[1] *= factor;
        clamp_map_zoom(camera, settings.zoom_limits, min_zoom, max_zoom);
        let applied = camera.zoom[0] / zoom_before_x;
        if applied != 0. {
            camera.target = [
                anchor[0] + (camera.target[0] - anchor[0]) / applied,
                anchor[1] + (camera.target[1] - anchor[1]) / applied,
            ];
        }
    }
    if ctx.input.key_pressed(KeyCode::Equal) || ctx.input.key_pressed(KeyCode::NumpadAdd) {
        camera.zoom[0] *= 1.2;
        camera.zoom[1] *= 1.2;
    }
    if ctx.input.key_pressed(KeyCode::Minus) || ctx.input.key_pressed(KeyCode::NumpadSubtract) {
        camera.zoom[0] /= 1.2;
        camera.zoom[1] /= 1.2;
    }
    // Кламп каждый кадр: снап в границы при включении лимитов [Z] и удержание
    // стартового вида в пределах карты (границы — от фактического размера карты).
    clamp_map_zoom(camera, settings.zoom_limits, min_zoom, max_zoom);
    // Границы движения камеры: видимая область не выходит за карту.
    let (map_w, map_h) = (SIZE.0 * size as f32, SIZE.1 * size as f32);
    let (vis_w, vis_h) = (2. / camera.zoom[0], 2. / camera.zoom[1]);
    camera.target[0] = if vis_w >= map_w {
        map_w * 0.5
    } else {
        camera.target[0].clamp(vis_w * 0.5, map_w - vis_w * 0.5)
    };
    camera.target[1] = if vis_h >= map_h {
        map_h * 0.5
    } else {
        camera.target[1].clamp(vis_h * 0.5, map_h - vis_h * 0.5)
    };

    let map_size = ctx.game.executor.gamemap.tilemap.size;
    let pos = settings
        .camera
        .screen_to_world(ctx.input.mouse_position(), viewport);
    let tile = [(pos[0] / SIZE.0).floor(), (pos[1] / SIZE.1).floor()];
    // F10: дебаг-дамп цепочки клика (дабл-клик меню строения). Владелец
    // воспроизводит проблему и присылает stderr.
    if ctx.input.key_pressed(KeyCode::F10) {
        let hsize = ctx.game.executor.gamemap.hitmap.size;
        let t = (tile[0] as usize % hsize, tile[1] as usize % hsize);
        let hit = &ctx.game.executor.gamemap.hitmap[t];
        let army_in = hit.building.is_some_and(|b| army_inside_building(ctx, b));
        eprintln!(
            "F10 DBG: tile=({},{}) | hitmap[tile]: building={:?} army={:?} passable={} | \
             lmb_building={:?} last_click={:?} (сек) pending_open={:?} goto_tile={:?} | \
             army_in={} player_army={:?} battle={}",
            t.0,
            t.1,
            hit.building,
            hit.army,
            hit.passable(),
            ctx.building_ui.lmb_building,
            ctx.building_ui.last_click,
            ctx.building_ui.pending_open,
            ctx.building_ui.goto_tile,
            army_in,
            ctx.game.executor.players.first().map(|p| p.army),
            ctx.game.executor.battle.is_some(),
        );
    }
    if ctx.input.mouse_button_released(0) {
        let tile = [
            tile[0] as usize % map_size,
            tile[1] as usize % map_size,
        ];
        let now = crate::time_secs();
        let clicked_building = ctx.game.executor.gamemap.hitmap[(tile[0], tile[1])].building;
        // Обводка строения: любая клетка его хитбокса выделяет весь спрайт.
        ctx.building_ui.lmb_building = clicked_building;
        eprintln!(
            "map_click: tile=({},{}) building={:?} army_in={} now={:.3} last_click={:?}",
            tile[0],
            tile[1],
            clicked_building,
            clicked_building.is_some_and(|b| army_inside_building(ctx, b)),
            now,
            ctx.building_ui.last_click,
        );
        // Дабл-клик по тайлу ЧУЖОЙ армии (< 400 мс, тот же тайл) — приказ
        // преследования: армия игрока идёт к соседней клетке цели и
        // перезаказывает путь при её движении (см. Follow в server.rs).
        let clicked_army = ctx.game.executor.gamemap.hitmap[(tile[0], tile[1])].army;
        let player_army = ctx.game.executor.players.first().map(|p| p.army);
        let mut double_follow = false;
        if let (Some(target), Some(my)) = (clicked_army, player_army) {
            if target != my {
                if let Some((t, last_tile)) = ctx.building_ui.last_click {
                    let same_tile = last_tile == tile;
                    if now - t < 0.4 && same_tile {
                        double_follow = true;
                    }
                }
            }
        }
        // Дабл-клик по тайлу строения (< 400 мс, тот же тайл):
        // армия в хитбоксе — открыть окно; иначе — идти к строению и
        // открыть окно по прибытии (pending_open).
        let mut double_open = false;
        let mut double_go = false;
        if let Some(building) = clicked_building {
            // «Армия в хитбоксе строения»: ЛЮБОЙ тайл армии внутри хитбокса
            // (многотайловые армии — якорь может быть снаружи).
            let army_in = army_inside_building(ctx, building);
            if let Some((t, last_tile)) = ctx.building_ui.last_click {
                // Тот же хитбокс строения (любая его клетка), не только тот же тайл.
                let same_building = ctx.game.executor.gamemap.hitmap
                    [(last_tile[0], last_tile[1])]
                    .building == clicked_building;
                if now - t < 0.4 && same_building && clicked_building.is_some() {
                    double_open = army_in;
                    double_go = !army_in;
                }
            }
        }
        eprintln!(
            "map_click: double_follow={} double_open={} double_go={} (окно 0.4с)",
            double_follow, double_open, double_go
        );
        // ФИКС (баг владельца «дабл-клик не открывает меню»): last_click
        // нигде не устанавливался — только сбрасывался в None, поэтому
        // double_open/double_go были мёртвыми ветками. Ставим всегда.
        ctx.building_ui.last_click = Some((now, tile));
        if double_follow {
            // Преследование: одинарный клик уже заказал Follow; дабл-клик
            // переиздаёт приказ (перезапуск погони после сброса/возврата).
            ctx.building_ui.last_click = None;
            ctx.building_ui.pending_open = None;
            ctx.building_ui.goto_tile = Some(tile);
            ctx.game.executor.message_handler(
                ClientMessage::Follow(clicked_army.unwrap()),
                0,
                ctx.registry,
            );
        }
        if double_open {
            ctx.building_ui.last_click = None;
            ctx.building_ui.pending_open = None;
            return Some(Menu::Building(clicked_building.unwrap(), BuildingTab::Main));
        }
        if double_go {
            // Второй клик дабл-клика: первый уже отправил GoTo; фиксируем
            // намерение открыть окно по прибытии армии к строению.
            ctx.building_ui.pending_open = clicked_building;
        } else {
            ctx.building_ui.pending_open = None;
        }
        if clicked_building.is_none() {
            // Клик мимо строений сбрасывает выделение.
            ctx.building_ui.lmb_building = None;
        }
        if let (Some(target), Some(my)) = (clicked_army, player_army) {
            if target != my && !double_follow {
                // Одинарный клик по ЧУЖОЙ армии — сразу приказ преследования:
                // иначе дабл-клик недостижим (GoTo на занятую клетку не
                // строит путь). Своя армия — прежнее поведение (GoTo).
                ctx.building_ui.goto_tile = Some(tile);
                ctx.game.executor.message_handler(
                    ClientMessage::Follow(target),
                    0,
                    ctx.registry,
                );
                return None;
            }
        }
        ctx.building_ui.goto_tile = Some(tile);
        ctx.game
            .executor
            .message_handler(ClientMessage::GoTo((tile[0], tile[1])), 0, ctx.registry);
    }
    // Открытие окна по прибытии: армия стоит в хитбоксе строения, к которому
    // был дабл-клик (путь закончился).
    if let Some(building) = ctx.building_ui.pending_open {
        let player_army = ctx.game.executor.players[0].army;
        let army = &ctx.game.executor.gamemap.armys[player_army];
        let arrived = army.path.is_empty()
            && ctx.game.executor.gamemap.hitmap[(army.pos.0, army.pos.1)].building
                == Some(building);
        if arrived {
            ctx.building_ui.pending_open = None;
            ctx.building_ui.last_click = None;
            return Some(Menu::Building(building, BuildingTab::Main));
        }
    }
    if !ctx.game.executor.players[0].execution_queue.is_empty() {
        if let Execute::Message(msg) = ctx.game.executor.players[0].execution_queue.remove(0) {
            return Some(Menu::Message(msg));
        }
    }
    // ПКМ по тайлу: тоггл инфо-окна строения.
    handle_right_click(ctx, [tile[0] as usize % map_size, tile[1] as usize % map_size], map_size);

    // ---- HUD поверх карты: экранные координаты, не зумится ----
    let hud_camera = default_camera(viewport);
    ctx.gfx.begin_pass(Target::Screen, None, &hud_camera);
    let hud_font = ctx.assets.get_font(crate::assets::BENGUIAT);
    ctx.text.draw_text(
        ctx.gfx,
        &format!("Tile: {:?}", (tile[0] as i32, tile[1] as i32)),
        10.,
        70.,
        hud_font,
        40,
        1.,
        colors::DARKGRAY,
    );
    ctx.text.draw_text(
        ctx.gfx,
        &format!("{:.0} FPS", crate::fps()),
        10.,
        115.,
        hud_font,
        24,
        1.,
        colors::DARKGRAY,
    );
    ctx.text.draw_text(
        ctx.gfx,
        &format!(
            "[T] тайлы  [Y] декор  [B] билдинги  [N] ивенты  [R] сид: {}  [G] бленд: {:?}  [Z] зум-лимиты: {}",
            settings.seed,
            settings.blend_mode,
            if settings.zoom_limits { "вкл" } else { "выкл" }
        ),
        10.,
        145.,
        hud_font,
        40,
        1.,
        crate::gfx::rgba(30, 30, 30, 255),
    );
    // Накопление delta идёт в lib.rs::frame; здесь только тик логики (порт 2097-2101).
    if *ctx.delta > 0.2 {
        ctx.game.executor.tick(ctx.registry);
        *ctx.delta = 0.;
    }
    // Всплывающее окно информации о строении — последним поверх HUD.
    draw_building_info(ctx, &hud_camera, viewport);
    None
}
/// Зелёная обводка хитбокса строения (рисуется в мировых координатах карты).
fn outline_building(ctx: &mut Ctx, building: usize, color: [f32; 4]) {
    let Some(obj) = ctx
        .registry
        .objects
        .inner
        .iter()
        .find(|obj| obj.index == ctx.game.executor.gamemap.buildings[building].id)
    else {
        return;
    };
    let (bx, by) = ctx.game.executor.gamemap.buildings[building].pos;
    let (w, h) = (obj.size.0 as f32, obj.size.1 as f32);
    ctx.gfx.draw_rect_lines(
        (bx as f32 - w + 1.) * SIZE.0,
        (by as f32 - h + 1.) * SIZE.1,
        w * SIZE.0,
        h * SIZE.1,
        5.,
        color,
    );
}

/// ПКМ по тайлу строения — тоггл всплывающего окна информации о здании.
/// Возвращает true, если события ПКМ обработаны.
fn handle_right_click(ctx: &mut Ctx, tile: [usize; 2], map_size: usize) -> bool {
    if !ctx.input.mouse_button_released(1) {
        return false;
    }
    let clicked = ctx.game.executor.gamemap.hitmap[(tile[0], tile[1])].building;
    let reopen = match (ctx.building_ui.building_info_open, clicked) {
        (Some(prev), Some(b)) => prev != b,
        (Some(_), None) => false,
        (None, Some(_)) => true,
        (None, None) => false,
    };
    if reopen {
        ctx.building_ui.building_info_open = clicked;
        ctx.building_ui.building_info_tile = [tile[0].min(map_size - 1), tile[1].min(map_size - 1)];
        true
    } else if ctx.building_ui.building_info_open.is_some() {
        ctx.building_ui.building_info_open = None;
        true
    } else {
        false
    }
}

/// Детерминированный цвет строения для слоя принадлежности (F9):
/// HSV-хеш индекса (золотое сечение по hue), насыщенный, не белый.
fn building_ownership_color(building: usize) -> [f32; 4] {
    let hue = (building as f32 * 0.618_034) % 1.;
    // h 0..1, s=0.75, v=1.0 → rgb
    let h6 = hue * 6.;
    let sector = (h6.floor() as i32).rem_euclid(6);
    let frac = h6 - h6.floor();
    let (r, g, b) = match sector {
        0 => (1., frac, 0.),
        1 => (1. - frac, 1., 0.),
        2 => (0., 1., frac),
        3 => (0., 1. - frac, 1.),
        4 => (frac, 0., 1.),
        _ => (1., 0., 1. - frac),
    };
    let sat = 0.75;
    let mix = |c: f32| c * sat + (1. - sat);
    [mix(r), mix(g), mix(b), 1.]
}

/// Принадлежность строения: группа отношений (владелец = группа с player).
fn building_owner_label(building: &dt_lib::map::object::MapBuildingdata) -> String {
    if let Some(owner) = building.owner {
        return format!("Владелец: армия #{}", owner);
    }
    let rel = &building.relations;
    if rel.player != 0 {
        "Принадлежит: игрок".to_string()
    } else if rel.ally != 0 {
        "Принадлежит: союзник".to_string()
    } else if rel.neighbour != 0 {
        "Принадлежит: нейтральный сосед".to_string()
    } else if rel.enemy != 0 {
        "Принадлежит: враг".to_string()
    } else {
        "Принадлежит: ничейный".to_string()
    }
}

/// Всплывающее окно информации о строении: рисуется в HUD-пассе.
/// Рядом — отряд гарнизона (портреты юнитов реестра).
fn draw_building_info(ctx: &mut Ctx, hud_camera: &crate::camera::Camera, viewport: [f32; 2]) {
    let Some(building) = ctx.building_ui.building_info_open else {
        return;
    };
    let Some(b) = ctx.game.executor.gamemap.buildings.get(building) else {
        ctx.building_ui.building_info_open = None;
        return;
    };
    ctx.gfx.begin_pass(Target::Screen, None, hud_camera);
    let [tx, ty] = ctx.building_ui.building_info_tile;
    // Экранная позиция тайла строения (мир → экран карты).
    let settings = ctx
        .map_settings
        .borrow()
        .clone()
        .unwrap_or_else(crate::state::MapRenderSettings::default);
    let world = [
        tx as f32 * SIZE.0 + SIZE.0 / 2.,
        ty as f32 * SIZE.1 + SIZE.1 / 2.,
    ];
    let screen = settings.camera.world_to_screen(world, viewport);
    let (w, h) = (560., 360.);
    // Не вылезать за экран: рисуем правее тайла, с клампом.
    let x = (screen[0] + 24.).min(viewport[0] - w - 8.).max(8.);
    let y = (screen[1] - h / 2.).max(8.).min(viewport[1] - h - 8.);
    let font = ctx.assets.get_font(crate::assets::BENGUIAT);
    ctx.gfx.draw_rect(x, y, w, h, [0.05, 0.07, 0.05, 0.92]);
    ctx.gfx
        .draw_rect_lines(x, y, w, h, 3., [0.1, 0.9, 0.2, 0.9]);
    ctx.text.draw_text(
        ctx.gfx,
        &b.name,
        x + 16.,
        y + 44.,
        font,
        40,
        1.,
        colors::WHITE,
    );
    ctx.text.draw_text(
        ctx.gfx,
        &building_owner_label(b),
        x + 16.,
        y + 84.,
        font,
        28,
        1.,
        colors::DARKGRAY,
    );
    ctx.text.draw_multiline(
        ctx.gfx,
        &if b.desc.is_empty() {
            "(описание недоступно)".to_string()
        } else {
            b.desc.clone()
        },
        (x + 16., y + 120.),
        w - 32.,
        false,
        font,
        26,
        colors::WHITE,
    );
    // Гарнизон: строка портретов (рендер отряда).
    ctx.text.draw_text(
        ctx.gfx,
        if b.garrison.is_empty() {
            "Гарнизон: пуст"
        } else {
            "Гарнизон:"
        },
        x + 16.,
        y + h - 60.,
        font,
        28,
        1.,
        colors::WHITE,
    );
    let mut gx = x + 16. + 130.;
    for unit_id in b.garrison.iter().take(8) {
        let Some(info) = ctx.registry.units.inner.get(*unit_id) else {
            continue;
        };
        let tex = crate::state::get_unit_info_texture(ctx.assets, info);
        ctx.gfx.draw_texture(tex, gx, y + h - 104., 48., 60., colors::WHITE);
        gx += 52.;
        if gx > x + w - 60. {
            break;
        }
    }
}

/// Армия (любым своим тайлом) стоит в хитбоксе строения `building`?
fn army_inside_building(ctx: &Ctx, building: usize) -> bool {
    let player_army = ctx.game.executor.players[0].army;
    let b = &ctx.game.executor.gamemap.buildings[building];
    let (bx, by) = b.pos;
    let Some(obj) = ctx
        .registry
        .objects
        .inner
        .iter()
        .find(|obj| obj.index == b.id)
    else {
        return false;
    };
    let (w, h) = (obj.size.0 as usize, obj.size.1 as usize);
    for x in 0..w {
        for y in 0..h {
            // Хитбокс уходит влево-вверх от якоря и покрывает ровно спан
            // рендера [bx-w+1..bx] × [by-h+1..by] (calc_hitboxes, bake).
            let (tx, ty) = (bx as isize - x as isize, by as isize - y as isize);
            if tx < 0
                || ty < 0
                || tx as usize >= ctx.game.executor.gamemap.hitmap.size
                || ty as usize >= ctx.game.executor.gamemap.hitmap.size
            {
                continue;
            }
            let (tx, ty) = (tx as usize, ty as usize);
            if ctx.game.executor.gamemap.hitmap[(tx, ty)].army == Some(player_army) {
                return true;
            }
        }
    }
    false
}

