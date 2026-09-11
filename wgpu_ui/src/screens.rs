// Экраны меню. Порт quad_ui main.rs:2244-2301 (Main), 2334-2451 (Message),
// 2452-2613 (Info), 2614-2644 (Atlas), 2645-2769 (Battle), 2770-2854
// (RoomCreation), 2855-3153 (BattleSetup).
//
// Правило заимствований: нужные поля ctx извлекаются в локалы ДО создания
// UiCtx (он держит &mut gfx/text/widgets); внутри window-замыканий рисование
// только через ui.gfx/ui.text, доступ к игре — через локал `game`.
use crate::battle_view::draw_battle;
use crate::gfx::{colors, Target, TexId, WHITE};
use crate::Ctx;
use crate::state::{
    get_unit_info_texture, get_unit_texture, process_event, ConnectionStatus, GameVariant, Menu,
    MapRenderSettings, Online, Scenario, CARD_SIZE, SIZE,
};
use crate::ui::{hash, UiCtx, Val};
use dt_lib::battle::battlefield::BattleInfo;
use dt_lib::items::item::Item;
use dt_lib::map::event::{Execute, Message};
use dt_lib::units::unit::{display_unit, Unit};
use dt_lib::units::unitstats::ModifyUnitStats;
use winit::keyboard::KeyCode;

const FONT: &str = crate::assets::BENGUIAT;

fn get_string(ctx: &Ctx, id: &str) -> String {
    match ctx.widgets.get(&hash(id)) {
        Some(Val::Str(s)) => s.clone(),
        _ => String::new(),
    }
}

pub fn main_menu(ctx: &mut Ctx) {
    let viewport = [crate::ui::UI_W, crate::ui::UI_H];
    ctx.gfx
        .begin_pass(Target::Screen, Some(colors::WHITE), &ctx.ui.camera);
    let skin = &ctx.skins.main;
    let game_name = ctx.registry.locale.get("menu_game_name");
    let start_title = ctx.registry.locale.get("menu_start_title");
    let pvp_title = ctx.registry.locale.get("menu_pvp_title");
    let pve_title = ctx.registry.locale.get("menu_pve_title");
    let language = ctx.registry.locale.get("menu_language");
    let portrait = ctx.assets.get("ГГследопыт/Unit_40.png");
    let input = ctx.input;
    let game = &mut *ctx.game;
    let menu = &mut ctx.ui.main;
    let locale = &mut ctx.registry.locale;
    let mut ui = UiCtx::new(ctx.gfx, ctx.text, input, skin, ctx.widgets);
    ui.window(0., 0., viewport[0], viewport[1], |ui| {
        ui.label(Some([50., 50.]), &game_name);
        if ui.button(Some([250., 300.]), &start_title) {
            *menu = Menu::Map(MapRenderSettings::default());
        }
        if ui.button(Some([50., 100.]), "Atlas") {
            *menu = Menu::Atlas;
        }
        if ui.button(Some([50., 100.]), &pvp_title) {
            *menu = Menu::RoomCreation;
        }
        if ui.button(Some([50., 150.]), &pve_title) {
            game.executor.battle = Some(BattleInfo::new(
                &mut game.executor.gamemap.armys,
                0,
                1,
            ));
            *menu = Menu::BattleSetup;
        }
        if ui.button(Some([50., 200.]), &language) {
            locale.switch_lang();
        }
        if ui.button(Some([50., 250.]), "Info") {
            *menu = Menu::Info;
        }
        ui.gfx
            .draw_texture(portrait, 500., 500., 10. * SIZE.0, 10. * SIZE.1 * 2., WHITE);
    });
}

pub fn atlas(ctx: &mut Ctx) {
    ctx.gfx
        .begin_pass(Target::Screen, Some(colors::WHITE), &ctx.ui.camera);
    let mut x = 0f32;
    let mut y = 0f32;
    let mut max_y = 0f32;
    let ids: Vec<_> = ctx.assets.inner.values().copied().collect();
    for tex in ids {
        let size = ctx.gfx.tex_size(tex);
        if size.1 > max_y {
            max_y = size.1;
        }
        ctx.gfx.draw_texture(tex, x, y, size.0, size.1, WHITE);
        if x + size.0 > crate::ui::UI_W {
            y += max_y;
            max_y = 0.;
            x = 0.;
        } else {
            x += size.0;
        }
    }
}

/// Порт мягкого переноса строки описания (quad_ui main.rs:2524-2535).
fn wrap_line(string: &mut String, max_width: f32, text_w: f32, font_size: f32) -> String {
    if text_w > max_width {
        let len = string.len();
        let line_break = string
            .char_indices()
            .skip((max_width / font_size) as usize - 2)
            .find_map(|(i, x)| (x.is_ascii_punctuation() || x.is_whitespace()).then_some(i))
            .unwrap_or(len / 2);
        string.split_off(line_break)
    } else {
        String::new()
    }
}

pub fn info(ctx: &mut Ctx) {
    let viewport = [crate::ui::UI_W, crate::ui::UI_H];
    ctx.gfx
        .begin_pass(Target::Screen, Some(colors::WHITE), &ctx.ui.camera);
    let skin = &ctx.skins.dop;
    let units: Vec<(usize, String, TexId)> = ctx
        .registry
        .units
        .inner
        .iter()
        .enumerate()
        .map(|(i, u)| (i, u.name.clone(), get_unit_info_texture(ctx.assets, u)))
        .collect();
    let items_list: Vec<(String, TexId)> = ctx
        .registry
        .items
        .inner
        .iter()
        .map(|it| (it.name.clone(), ctx.assets.get(&it.icon)))
        .collect();
    let item_strings: Vec<Vec<String>> = ctx
        .registry
        .items
        .inner
        .iter()
        .map(|it| it.display_strings(&ctx.registry.locale))
        .collect();
    let font_id = ctx.assets.get_font(FONT);
    let input = ctx.input;
    let registry = &*ctx.registry;
    let menu = &mut ctx.ui.main;
    let mut ui = UiCtx::new(ctx.gfx, ctx.text, input, skin, ctx.widgets);
    ui.window(0., 0., viewport[0], viewport[1], |ui| {
        if ui.button(None, "Items") {
            ui.set_usize("menu_info", 1);
        }
        if ui.button(None, "Units") {
            ui.set_usize("menu_info", 0);
        }
        let menu_tab = ui.get_usize("menu_info");
        match menu_tab {
            0 => {
                let menu_unit_index = ui.get_usize("menu_unit_info");
                let sel_tex = units
                    .iter()
                    .find(|(i, _, _)| *i == menu_unit_index)
                    .map(|(_, _, t)| *t);
                let change = if let Some(t) = sel_tex {
                    ui.texture_widget(t, 0., 0., CARD_SIZE, CARD_SIZE)
                } else {
                    false
                };
                if change {
                    let val = ui.get_usize("info_unit");
                    ui.set_usize("info_unit", if val == 1 { 0 } else { 1 });
                }
                if ui.get_usize("info_unit") == 1 {
                    let mut selected = menu_unit_index;
                    let mut cursor = [0., 0.];
                    ui.scroll_group("units_list", 0., 0., CARD_SIZE + 240., viewport[1] - 100., |ui, pos| {
                        cursor = pos;
                        for (i, name, t) in &units {
                            if ui.texture(*t, pos[0], cursor[1], CARD_SIZE, CARD_SIZE) {
                                selected = *i;
                            }
                            ui.label(Some([pos[0] + CARD_SIZE + 10., cursor[1] + 20.]), name);
                            cursor[1] += CARD_SIZE + 40.;
                        }
                    });
                    ui.set_usize("menu_unit_info", selected);
                }
                // Описание выбранного юнита (порт Group::new(...).position(...).ui).
                let menu_unit_index = ui.get_usize("menu_unit_info");
                if let Some(unit) = registry.units.inner.get(menu_unit_index) {
                    let info = display_unit(&From::from((unit.clone(), &registry.bonuses)), registry);
                    let mut x = CARD_SIZE + 150.;
                    let mut y = 100.;
                    let max_width = viewport[1] - CARD_SIZE - 150.;
                    for mut string in info {
                        let w = ui.text.measure(ui.gfx, &string, font_id, 32, 1.).width;
                        let line2 = wrap_line(&mut string, max_width, w, 32.);
                        ui.text
                            .draw_text(ui.gfx, &string, x, y, font_id, 32, 1., colors::BLACK);
                        y += 32.;
                        if !line2.is_empty() {
                            ui.text
                                .draw_text(ui.gfx, &line2, x, y, font_id, 32, 1., colors::BLACK);
                            y += 32.;
                        }
                    }
                }
            }
            _ => {
                let menu_item_index = ui.get_usize("menu_item_info");
                let sel_tex = items_list.get(menu_item_index).map(|(_, t)| *t);
                let change = if let Some(t) = sel_tex {
                    ui.texture_widget(t, 0., 0., 50., 50.)
                } else {
                    ui.button(None, "ui_item")
                };
                if change {
                    let val = ui.get_bool("menu_items");
                    ui.set_bool("menu_items", !val);
                }
                if ui.get_bool("menu_items") {
                    let mut sel = ui.get_usize("menu_item_info");
                    let mut cursor = [0., 0.];
                    ui.scroll_group("items_list", 0., 0., 220., viewport[1] - 100., |ui, pos| {
                        cursor = pos;
                        for (i, (name, t)) in items_list.iter().enumerate() {
                            if ui.texture(*t, pos[0], cursor[1], 50., 50.) {
                                sel = i;
                            }
                            ui.label(Some([pos[0] + 55., cursor[1] + 12.]), name);
                            cursor[1] += 58.;
                        }
                    });
                    ui.set_usize("menu_item_info", sel);
                }
                if let Some(strings) = item_strings.get(menu_item_index) {
                    let mut y = 100.;
                    for string in strings {
                        ui.text.draw_text(
                            ui.gfx, string, 200., y, font_id, 32, 1., colors::BLACK,
                        );
                        y += 32.;
                    }
                }
                ui.set_usize("menu_item_info", menu_item_index);
            }
        }
        let exit_x = viewport[0] - ui.text.measure(ui.gfx, "Exit", font_id, 32, 1.).width;
        if input.key_released(KeyCode::Escape) || ui.button(Some([exit_x, 0.]), "Exit") {
            *menu = Menu::Main;
        }
    });
}

pub fn message(ctx: &mut Ctx) {
    let viewport = [crate::ui::UI_W, crate::ui::UI_H];
    ctx.gfx
        .begin_pass(Target::Screen, Some(colors::WHITE), &ctx.ui.camera);
    // Фон Paper.png на весь экран (порт draw_texture_size в оригинале).
    ctx.gfx.draw_texture(
        ctx.skins.dop.window_tex,
        0.,
        0.,
        viewport[0],
        viewport[1],
        WHITE,
    );
    let Menu::Message(msg) = &mut ctx.ui.main else {
        return;
    };
    let skin = &ctx.skins.main;
    let font_id = ctx.assets.get_font(FONT);
    let max_line_length = 640.;
    // Снапшоты данных до UiCtx.
    let msg_text = msg.text.clone();
    let event_id = msg.corresponding_event_id;
    let added_icons: Vec<TexId> = event_id
        .and_then(|id| ctx.game.executor.events.get(id))
        .and_then(|event| event.result.add_units.as_ref().map(|added| {
            added
                .iter()
                .filter_map(|id| ctx.registry.units.inner.get(*id))
                .map(|u| get_unit_info_texture(ctx.assets, u))
                .collect()
        }))
        .unwrap_or_default();
    let variants: Vec<String> = msg.variants.clone();
    let next_msg: Option<Message> = {
        let events_len = ctx.game.executor.events.len();
        let event = event_id
            .and_then(|i| ctx.game.executor.events.get(i + 1))
            .or_else(|| ctx.game.executor.events.first());
        event.map(|e| Message::from_event(&e, (event_id.unwrap_or(0) + 1) % events_len))
    };
    let queue: Option<Message> = ctx.game.executor.players[0]
        .execution_queue
        .first()
        .cloned()
        .and_then(|item| match item {
            Execute::Message(m) => Some(m),
            _ => None,
        });

    let mut cur_y_pos = 100.;
    if !msg_text.is_empty() {
        let drawn = ctx.text.draw_multiline(
            ctx.gfx,
            &msg_text,
            (viewport[0] / 2. - 320., 100.),
            max_line_length,
            false,
            font_id,
            30,
            colors::BLACK,
        );
        cur_y_pos += drawn.1;
    }
    const SMALL_CARD_SIZE: f32 = 52.;
    if !added_icons.is_empty() {
        let offset = SMALL_CARD_SIZE + 20.;
        let width = added_icons.len() as f32 * offset;
        for (i, texture) in added_icons.iter().enumerate() {
            let pos_x = i as f32 * offset + viewport[0] / 2. - width / 2.;
            ctx.gfx
                .draw_texture(*texture, pos_x, cur_y_pos, SMALL_CARD_SIZE, SMALL_CARD_SIZE, WHITE);
        }
        cur_y_pos += SMALL_CARD_SIZE + 20.;
    }
    let mut advance = false;
    let mut to_next = false;
    {
        let input = ctx.input;
        let mut ui = UiCtx::new(ctx.gfx, ctx.text, input, skin, ctx.widgets);
        if ui.button_sized([50., 50.], [300., 40.], "Следующее событие")
            || input.key_pressed(KeyCode::ArrowRight)
        {
            to_next = true;
        }
        // Пропустить всю цепочку: вычистить очередь сообщений executor'а.
        if ui.button_sized([370., 50.], [300., 40.], "Пропустить всё") {
            ctx.game.executor.players[0].execution_queue.clear();
            ctx.ui.main = Menu::Map(ctx.map_settings.take().unwrap_or_default());
            return;
        }
        if to_next {
            if let Some(next) = next_msg {
                ctx.ui.main = Menu::Message(next);
            }
            return;
        }
        // Кнопки вариантов по измеренной ширине (порт 2402-2427): клик по любому
        // варианту просто продолжает (диспетчеризация вариантов в оригинале не
        // подключена — сохраняем поведение).
        if variants.is_empty() {
            advance = ui.button_sized(
                [viewport[0] / 2. - 100., cur_y_pos + 20.],
                [200., 40.],
                "Понятно",
            ) || input.key_pressed(KeyCode::Enter);
        } else {
            let total: f32 = variants
                .iter()
                .map(|v| ui.text.measure(ui.gfx, v, ui.skin.font, 32, 1.).width)
                .sum::<f32>()
                + variants.len() as f32 * (32. + 40.);
            let mut cur_x = viewport[0] / 2. - total / 2.;
            for variant in &variants {
                let size = ui.text.measure(ui.gfx, variant, ui.skin.font, 32, 1.).width;
                if ui.button_sized([cur_x, cur_y_pos + 20.], [size + 20., 40.], variant) {
                    advance = true;
                }
                cur_x += total + 40.;
            }
        }
    }
    if advance {
        if queue.is_some() {
            ctx.game.executor.players[0].execution_queue.remove(0);
            if let Some(next) = queue {
                ctx.ui.main = Menu::Message(next);
            }
        } else {
            // Возврат на карту с СОХРАНЕНИЕМ камеры/флагов.
            ctx.ui.main = Menu::Map(ctx.map_settings.take().unwrap_or_default());
        }
    }
}

pub fn room_creation(ctx: &mut Ctx) {
    let viewport = [crate::ui::UI_W, crate::ui::UI_H];
    ctx.gfx
        .begin_pass(Target::Screen, Some(colors::WHITE), &ctx.ui.camera);
    let connected = matches!(ctx.game.variant, GameVariant::Online(_));
    let skin = &ctx.skins.main;
    let game_name = ctx.registry.locale.get("menu_game_name");
    let room_code_label = ctx.registry.locale.get("ui_room_code");
    let connect_label = ctx.registry.locale.get("ui_connect");
    let awaiting = ctx.registry.locale.get("ui_awaiting");
    let cancel_label = ctx.registry.locale.get("ui_cancel_connection");
    let ip = format!("{}:{}", ctx.registry.settings.ip, ctx.registry.settings.port);
    let input = ctx.input;
    let font_id = ctx.assets.get_font(FONT);
    let game = &mut *ctx.game;
    let menu = &mut ctx.ui.main;
    let mut connecting = false;
    let mut ui = UiCtx::new(ctx.gfx, ctx.text, input, skin, ctx.widgets);
    ui.window(0., 0., viewport[0], viewport[1], |ui| {
        ui.label(Some([50., 50.]), &game_name);
        let mut room = get_string_snapshot();
        ui.input_text("ui_room_code", &room_code_label, &mut room);
        ROOM_CODE.with(|cell| *cell.borrow_mut() = room);
        if !connected
            && (ui.button(Some([150., 150.]), &connect_label)
                || input.key_released(KeyCode::Enter))
        {
            connecting = true;
        }
        if connected {
            ui.label(Some([150., 150.]), &awaiting);
            if ui.button(None, &cancel_label) {
                game.variant = GameVariant::Single(Scenario { events: vec![] });
            }
        }
        if let GameVariant::Online(online) = &game.variant {
            if online.status == ConnectionStatus::Full(false) {
                *menu = Menu::BattleSetup;
            }
        }
        let exit_x = viewport[0] - ui.text.measure(ui.gfx, "Exit", font_id, 32, 1.).width;
        if input.key_released(KeyCode::Escape) || ui.button(Some([exit_x, 0.]), "Exit") {
            *menu = Menu::Main;
            game.variant = GameVariant::Single(Scenario { events: vec![] });
        }
    });
    if connecting {
        let room = ROOM_CODE.with(|cell| cell.borrow().clone());
        let conn = ctx.rt.block_on(dt_client::connect(
            ip,
            room.clone(),
            dt_lib::hwid::get_id().unwrap(),
        ));
        let conn = if conn.is_err() {
            eprintln!("CAN'T CONNECT TO MAIN SERVER");
            ctx.rt.block_on(dt_client::connect(
                "147.185.221.26:58356".into(),
                room,
                dt_lib::hwid::get_id().unwrap(),
            ))
        } else {
            conn
        };
        if let Ok(conn) = conn {
            ctx.game.variant = GameVariant::Online(Online {
                conn,
                status: ConnectionStatus::NotFull,
                army: 0,
            });
        } else {
            eprintln!("CAN'T CONNECT TO TUNNEL");
        }
    }
    if let GameVariant::Online(online) = &mut ctx.game.variant {
        if let Some(mes) = online.conn.req_one() {
            process_event(ctx.game, mes);
        }
    }
}

// Код комнаты живёт между кадрами (в оригинале — локал главного цикла).
thread_local! {
    static ROOM_CODE: std::cell::RefCell<String> = const { std::cell::RefCell::new(String::new()) };
}
fn get_string_snapshot() -> String {
    ROOM_CODE.with(|cell| cell.borrow().clone())
}

pub fn battle(ctx: &mut Ctx) {
    let viewport = [crate::ui::UI_W, crate::ui::UI_H];
    ctx.gfx
        .begin_pass(Target::Screen, Some(colors::WHITE), &ctx.ui.camera);
    let winner = draw_battle(ctx, true);
    let mut my_army: Option<usize> = None;
    let mut go_back = false;
    let winner_text: Option<String> = match &mut ctx.game.variant {
        GameVariant::Online(online) => {
            my_army = Some(online.army);
            if let Some(winner) = winner {
                let t = if winner == online.army {
                    ctx.registry.locale.get("ui_winner")
                } else {
                    ctx.registry.locale.get("ui_loser")
                };
                online
                    .conn
                    .send_action(dt_client::dt_server::Incoming::Disconnect);
                go_back = true;
                Some(t)
            } else {
                None
            }
        }
        _ => {
            if winner.is_some() {
                go_back = true;
                Some(ctx.registry.locale.get("ui_winner"))
            } else {
                None
            }
        }
    };
    let font_id = ctx.assets.get_font(FONT);
    let input = ctx.input;
    if let Some(text) = winner_text {
        let size = ctx.text.measure(ctx.gfx, &text, font_id, 32, 1.);
        let pos = (
            CARD_SIZE * 6. / 2. - size.width / 2.,
            1080. / 2. - size.height / 2.,
        );
        let mut ui = UiCtx::new(ctx.gfx, ctx.text, input, &ctx.skins.main, ctx.widgets);
        ui.button(Some([pos.0, pos.1]), &text);
    }
    // Панель активного юнита — снапшот данных до UiCtx.
    let panel: Option<(TexId, Vec<String>, String)> =
        ctx.game.executor.battle.as_ref().and_then(|battle| {
            battle.active_unit.map(|active| {
                let troop = &ctx.game.executor.gamemap.armys[active.army].troops[active.index];
                let troop = troop.get();
                let texture = get_unit_texture(ctx.assets, &troop.unit, &ctx.registry.units);
                let lines = display_unit(&troop.unit, ctx.registry);
                let text = if Some(active.army) == my_army || my_army.is_none() {
                    ctx.registry.locale.get("ui_my_move")
                } else {
                    ctx.registry.locale.get("ui_not_my_move")
                };
                (texture, lines, text)
            })
        });
    let panel_x = CARD_SIZE * (12 / 2) as f32 / 1920. * viewport[0];
    let game = &mut *ctx.game;
    let menu = &mut ctx.ui.main;
    {
        let mut ui = UiCtx::new(ctx.gfx, ctx.text, input, &ctx.skins.dop, ctx.widgets);
        ui.window(panel_x, 0., viewport[0] - panel_x, viewport[1], |ui| {
            if let Some((texture, lines, move_text)) = &panel {
                ui.texture(*texture, 0., 0., CARD_SIZE, CARD_SIZE);
                let mut y = CARD_SIZE + 10.;
                for string in lines {
                    ui.text
                        .draw_text(ui.gfx, string, 0., y, font_id, 32, 1., colors::BLACK);
                    y += 32.;
                }
                ui.button(None, move_text);
            }
            let exit_x = (viewport[0] - panel_x)
                - ui.text.measure(ui.gfx, "Exit", font_id, 32, 1.).width;
            if ui.button(Some([exit_x, 0.]), "Exit") {
                game.variant = GameVariant::Single(Scenario { events: vec![] });
                *menu = Menu::Main;
                ui.set_bool("ready", false);
            }
        });
    }
    if go_back {
        ctx.game.variant = GameVariant::Single(Scenario { events: vec![] });
        ctx.ui.main = Menu::Main;
        ctx.widgets.insert(hash("ready"), Val::Bool(false));
    }
}

pub fn battle_setup(ctx: &mut Ctx) {
    let viewport = [crate::ui::UI_W, crate::ui::UI_H];
    ctx.gfx
        .begin_pass(Target::Screen, Some(colors::WHITE), &ctx.ui.camera);
    let _ = draw_battle(ctx, false);
    if let GameVariant::Online(online) = &mut ctx.game.variant {
        if matches!(online.status, ConnectionStatus::Full(true)) {
            online
                .conn
                .send_action(dt_client::dt_server::Incoming::GetState);
            ctx.ui.main = Menu::Battle;
            return;
        }
    }
    if ctx.input.key_released(KeyCode::Minus) {
        let size = ctx.window.inner_size();
        ctx.window.request_inner_size(winit::dpi::LogicalSize::new(
            size.width as f64 - 10.,
            size.height as f64 - 10.,
        ));
    }
    let panel_x = CARD_SIZE * (12 / 2) as f32 / 1920. * viewport[0];
    let font_id = ctx.assets.get_font(FONT);
    let input = ctx.input;
    let registry = &*ctx.registry;
    let units: Vec<(usize, String, TexId)> = ctx
        .registry
        .units
        .inner
        .iter()
        .enumerate()
        .map(|(i, u)| (i, u.name.clone(), get_unit_info_texture(ctx.assets, u)))
        .collect();
    let items_all: Vec<(String, TexId, bool)> = ctx
        .registry
        .items
        .inner
        .iter()
        .map(|it| (it.name.clone(), ctx.assets.get(&it.icon), true))
        .collect();
    let assets = ctx.assets;
    let l_units_menu = ctx.registry.locale.get("ui_units_menu");
    let l_items_menu = ctx.registry.locale.get("ui_items_menu");
    let l_start_menu = ctx.registry.locale.get("ui_start_menu");
    let l_displace = ctx.registry.locale.get("ui_displace");
    let l_unit = ctx.registry.locale.get("ui_unit");
    let l_item = ctx.registry.locale.get("ui_item");
    let l_ready = ctx.registry.locale.get("ui_ready");
    let l_not_ready = ctx.registry.locale.get("ui_not_ready");
    let l_awaiting = ctx.registry.locale.get("ui_awaiting");
    let game = &mut *ctx.game;
    let menu = &mut ctx.ui.main;
    let mut ui = UiCtx::new(ctx.gfx, ctx.text, input, &ctx.skins.dop, ctx.widgets);
    ui.window(panel_x, 0., viewport[0] - panel_x, viewport[1], |ui| {
        // Снапшоты данных до UI-вызовов (pick-замыкания мутают game напрямую).
        let focus = game.focus;
        let troop_snap = game.executor.gamemap.armys[focus.army]
            .get_troop(focus.pos)
            .map(|t| {
                let t = t.get();
                (
                    get_unit_texture_from(&t, assets, registry),
                    t.unit.inventory.items.len(),
                    registry
                        .items
                        .inner
                        .iter()
                        .filter(|it| it.can_equip(&t.unit, registry))
                        .map(|it| (assets.get(&it.icon), it.name.clone()))
                        .collect::<Vec<(TexId, String)>>(),
                    display_unit(&t.unit, registry),
                )
            });
        let ready = ui.get_bool("ready");

        if !ready {
            // Портрет юнита ячейки / кнопка установки.
            match &troop_snap {
                Some((portrait, items_len, item_rows, info_lines)) => {
                    let units_rows: Vec<(TexId, String)> = units
                        .iter()
                        .map(|(_, name, tex)| (*tex, name.clone()))
                        .collect();
                    // Клик по портрету -> дропдаун юнитов (скролл + подписи).
                    ui.dropdown(
                        "unit_pick",
                        *portrait,
                        [CARD_SIZE, CARD_SIZE],
                        64.,
                        &units_rows,
                        5,
                        |ui, unit_index| {
                            if let GameVariant::Online(online) = &mut game.variant {
                                online
                                    .conn
                                    .send_action(dt_client::dt_server::Incoming::SetUnit((
                                        focus,
                                        Some(unit_index),
                                    )));
                            } else {
                                let armies = &mut game.executor.gamemap.armys;
                                let army = focus.army;
                                armies[army].set_unit_at(Some(unit_index), focus, registry);
                            }
                        },
                    );
                    // Кнопка очистить слот персонажа.
                    if ui.button(None, &l_displace) || input.key_released(KeyCode::Delete) {
                        if let GameVariant::Online(online) = &mut game.variant {
                            online
                                .conn
                                .send_action(dt_client::dt_server::Incoming::SetUnit((focus, None)));
                        } else {
                            let armies = &mut game.executor.gamemap.armys;
                            let army = focus.army;
                            armies[army].set_unit_at(None, focus, registry);
                        }
                    }
                    // Описание юнита.
                    for string in info_lines {
                        ui.label(None, string);
                    }
                    // Инвентарь: слоты (обычно 4) с дропдауном предметов.
                    ui.label(None, "Инвентарь");
                    for i in 0..*items_len {
                        let slot_tex = game
                            .executor
                            .gamemap
                            .armys[focus.army]
                            .get_troop(focus.pos)
                            .and_then(|t| t.get().unit.inventory.items[i].clone())
                            .map(|it| assets.get(&it.get_info(&registry.items).icon));
                        let closed = slot_tex.unwrap_or_else(|| ui.skin.button_tex);
                        let picked_item = ui.dropdown(
                            &format!("item_pick_{i}"),
                            closed,
                            [50., 50.],
                            58.,
                            item_rows,
                            6,
                            |ui, item_index| {
                                if let GameVariant::Online(online) = &mut game.variant {
                                    online
                                        .conn
                                        .send_action(dt_client::dt_server::Incoming::SetItem((
                                            focus,
                                            (i, Some(item_index)),
                                        )));
                                } else {
                                    let troop_cell = game.executor.gamemap.armys
                                        [focus.army]
                                        .get_troop(focus.pos)
                                        .unwrap();
                                    let mut t = troop_cell.get();
                                    t.unit.swap_item(
                                        i,
                                        Some(dt_lib::items::item::Item { index: item_index }),
                                        registry,
                                    );
                                }
                            },
                        );
                        let _ = picked_item;
                        // Кнопка очистить слот предмета (только если занят).
                        if slot_tex.is_some() && ui.button(None, &l_displace) {
                            if let GameVariant::Online(online) = &mut game.variant {
                                online
                                    .conn
                                    .send_action(dt_client::dt_server::Incoming::SetItem((
                                        focus,
                                        (i, None),
                                    )));
                            } else {
                                let troop_cell = game.executor.gamemap.armys[focus.army]
                                    .get_troop(focus.pos)
                                    .unwrap();
                                let mut t = troop_cell.get();
                                t.unit.swap_item(i, None, registry);
                            }
                        }
                    }
                }
                None => {
                    // Слот пуст: кнопка-заглушка открывает дропдаун юнитов.
                    let units_rows: Vec<(TexId, String)> = units
                        .iter()
                        .map(|(_, name, tex)| (*tex, name.clone()))
                        .collect();
                    ui.dropdown(
                        "unit_pick_empty",
                        ui.skin.button_tex,
                        [160., 50.],
                        64.,
                        &units_rows,
                        5,
                        |ui, unit_index| {
                            if let GameVariant::Online(online) = &mut game.variant {
                                online
                                    .conn
                                    .send_action(dt_client::dt_server::Incoming::SetUnit((
                                        focus,
                                        Some(unit_index),
                                    )));
                            } else {
                                let armies = &mut game.executor.gamemap.armys;
                                let army = focus.army;
                                armies[army].set_unit_at(Some(unit_index), focus, registry);
                            }
                        },
                    );
                }
            }
        }

        // Готов / Не готов.
        let text = if ready { &l_not_ready } else { &l_ready };
        if ui.button(None, text) {
            let was_ready = ui.get_bool("ready");
            ui.set_bool("ready", !was_ready);
            if let GameVariant::Online(online) = &mut game.variant {
                online
                    .conn
                    .send_action(dt_client::dt_server::Incoming::Status(!was_ready));
            } else if !was_ready {
                eprintln!("DBG setup: -> Menu::Battle, calling battle.start()");
                *menu = Menu::Battle;
                game.executor.battle.as_mut().and_then(|x| {
                    Some(x.start(&mut game.executor.gamemap.armys, registry))
                });
            }
        }
        if ready {
            ui.label(None, &l_awaiting);
        }
        let exit_x = (viewport[0] - panel_x)
            - ui.text.measure(ui.gfx, "Exit", font_id, 32, 1.).width;
        if ui.button(Some([exit_x, 0.]), "Exit") {
            *menu = Menu::Main;
            game.variant = GameVariant::Single(Scenario { events: vec![] });
        }
    });
}

// Снапшот-хелперы, безопасные для заимствований внутри замыканий.
fn get_unit_texture_from(
    troop: &dt_lib::battle::troop::Troop,
    assets: &crate::assets::Assets,
    registry: &dt_lib::registry::GameInfo,
) -> TexId {
    get_unit_texture(assets, &troop.unit, &registry.units)
}

fn ctx_assets_get(
    it: &dt_lib::items::item::Item,
    assets: &crate::assets::Assets,
    registry: &dt_lib::registry::GameInfo,
) -> TexId {
    assets.get(&it.get_info(&registry.items).icon)
}

fn tex_for_item(item: &dt_lib::items::item::ItemInfo, assets: &crate::assets::Assets) -> TexId {
    assets.get(&item.icon)
}
