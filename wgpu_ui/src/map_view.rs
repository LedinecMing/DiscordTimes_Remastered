// Экран карты: запечённая карта, камера (пан/зум/хоткеи), армии, HUD, переходы.
// Порт quad_ui main.rs:1802-2104 (+ rebake ветки из главного цикла 2302-2332).
use crate::bake::{bake_map_textures, render_decos_layer};
use crate::camera::default_camera;
use crate::gfx::{colors, Target, WHITE};
use crate::Ctx;
use crate::state::{Menu, MapRenderSettings, SIZE};
use dt_lib::battle::battlefield::BattleInfo;
use dt_lib::battle::control::Control;
use dt_lib::map::event::Execute;
use dt_lib::network::server::ClientMessage;
use dt_lib::units::unit::UnitType;
use winit::keyboard::KeyCode;

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
            ctx.game,
            ctx.registry,
            settings.seed,
            settings.buildings_render,
        );
        settings.decos_dirty = false;
    }
    if settings.blend_dirty {
        bake_map_textures(
            ctx.gfx,
            &ctx.rts.0,
            ctx.assets,
            ctx.game,
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
    let wheel = ctx.input.wheel;
    if wheel != 0. {
        let factor = 1.2f32.powf(wheel.clamp(-4., 4.));
        let anchor = camera.screen_to_world(ctx.input.mouse_position(), viewport);
        let zoom_before_x = camera.zoom[0];
        camera.zoom[0] *= factor;
        camera.zoom[1] *= factor;
        camera.clamp_zoom();
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
        camera.clamp_zoom();
    }
    if ctx.input.key_pressed(KeyCode::Minus) || ctx.input.key_pressed(KeyCode::NumpadSubtract) {
        camera.zoom[0] /= 1.2;
        camera.zoom[1] /= 1.2;
        camera.clamp_zoom();
    }

    let map_size = ctx.game.executor.gamemap.tilemap.size;
    let pos = settings
        .camera
        .screen_to_world(ctx.input.mouse_position(), viewport);
    let tile = [(pos[0] / SIZE.0).floor(), (pos[1] / SIZE.1).floor()];
    if ctx.input.mouse_button_released(0) {
        ctx.game.executor.message_handler(
            ClientMessage::GoTo((
                tile[0] as usize % map_size,
                tile[1] as usize % map_size,
            )),
            0,
            ctx.registry,
        );
    }
    if !ctx.game.executor.players[0].execution_queue.is_empty() {
        if let Execute::Message(msg) = ctx.game.executor.players[0].execution_queue.remove(0) {
            return Some(Menu::Message(msg));
        }
    }

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
            "[T] тайлы  [Y] декор  [B] билдинги  [N] ивенты  [R] сид: {}  [G] бленд: {:?}",
            settings.seed, settings.blend_mode
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
    None
}
