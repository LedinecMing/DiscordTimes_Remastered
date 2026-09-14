//! Экран «Редактор карт» (Menu::Editor): egui-оболочка над editor-core.
//!
//! ЕДИНЫЙ РЕНДЕР КАРТЫ: запекание идёт тем же bake-путём wgpu_ui, что и в
//! игре (tile_pixels + registry + GameMap → offscreen RT), а RT
//! регистрируется как нативная egui-текстура и рисуется в канвас.
//! Никакого второго софтверного рендера и GPU→CPU readback.
//!
//! Слои экрана: top bar (File/Edit/Lint), канвас с зумом/паном/ховом,
//! левая панель инструментов + палитра тайлов, статус-бар.

use crate::camera::{default_camera, Camera};
use crate::gfx::{colors, Target};
use crate::state::{EditorUi, Menu, SIZE};
use crate::Ctx;
use editor_core::command::{PaintTile, PlaceArmy, PlaceBuilding, PlaceDeco};
use editor_core::CommandResult;
use egui::{
    Align2, Color32, FontId, Frame, Layout, RichText, Sense, Stroke, Ui, Vec2,
};
use winit::keyboard::KeyCode;

/// Директория тайлов относительно cwd (бинарник запускается из dt/, как клиент).
const ASSETS_TERRAIN: &str = "assets/Terrain";

/// Открыть/закрыть экран: возврат false = выйти в главное меню.
pub fn editor_screen(ctx: &mut Ctx) -> bool {
    // Хоткеи до UI: Ctrl+Z undo, Ctrl+Shift+Z redo (Единая точка правды).
    handle_hotkeys(ctx);
    // Запек по грязному флагу: project.GameMap → bake.rs → RT (игровой путь);
    // RT регистрируется как нативная egui-текстура (zero-copy, без readback).
    {
        let Ctx {
            gfx,
            assets,
            tile_pixels,
            registry,
            editor,
            egui,
            ..
        } = ctx;
        rebake_if_dirty(gfx, assets, tile_pixels, registry, editor, &mut **egui);
    }
    let keep_open = draw_editor(ctx);
    if !keep_open {
        ctx.ui.main = Menu::Main;
    }
    keep_open
}

/// Хоткеи редактора: Ctrl+Z / Ctrl+Shift+Z, Escape — выход.
fn handle_hotkeys(ctx: &mut Ctx) {
    let ctrl = ctx.input.key_down(KeyCode::ControlLeft)
        || ctx.input.key_down(KeyCode::ControlRight);
    let shift = ctx.input.key_down(KeyCode::ShiftLeft)
        || ctx.input.key_down(KeyCode::ShiftRight);
    if ctrl && ctx.input.key_pressed(KeyCode::KeyZ) {
        let editor = &mut ctx.editor;
        if shift {
            if let Some(name) = editor.history.redo(&mut editor.state) {
                editor.status = format!("Redo: {name}");
                editor.bake_dirty = true;
            }
        } else if let Some(name) = editor.history.undo(&mut editor.state) {
            editor.status = format!("Undo: {name}");
            editor.bake_dirty = true;
        }
    }
    if ctrl && ctx.input.key_pressed(KeyCode::KeyY) {
        let editor = &mut ctx.editor;
        if let Some(name) = editor.history.redo(&mut editor.state) {
            editor.status = format!("Redo: {name}");
            editor.bake_dirty = true;
        }
    }
    if ctx.input.key_pressed(KeyCode::Escape) {
        // Escape покидает редактор (как с карты в меню).
        ctx.ui.main = Menu::Main;
    }
}

fn rebake_if_dirty(
    gfx: &mut crate::gfx::Gfx,
    assets: &crate::assets::Assets,
    tile_pixels: &[image::RgbaImage],
    registry: &dt_lib::registry::GameInfo,
    editor: &mut EditorUi,
    egui: &mut crate::egui_layer::EguiLayer,
) {
    if !editor.bake_dirty {
        return;
    }
    // Запекаем из ЖИВОГО state.project() — источник истины после команд
    // (PaintTile и т.п. меняют state, а editor.project — лишь снапшот
    // new/open; запек по снапшоту не отражал правки).
    let project = editor.state.project().clone();
    let size = project.size();
    // Два RT — как в игре: тайлы+наплывы и декорации+строения+армии
    // (единый рендер-путь bake_map_textures/render_decos_layer).
    let mut rt_recreated = false;
    if editor.rt.is_none() || editor.decos_rt.is_none() {
        editor.rt = Some(gfx.create_rt(
            (SIZE.0 * size as f32) as u32,
            (SIZE.1 * size as f32) as u32,
        ));
        editor.decos_rt = Some(gfx.create_rt(
            (SIZE.0 * size as f32) as u32,
            (SIZE.1 * size as f32) as u32,
        ));
        rt_recreated = true;
    }
    // ЕДИНЫЙ рендер-путь с игрой (bake.rs): тайлы, декорации, строения,
    // армии (модельки/корабли на воде). Маркеры редактора — egui-слой.
    crate::bake::bake_map_textures(
        gfx,
        editor.rt.as_ref().expect("rt just created"),
        assets,
        &project.map,
        tile_pixels,
        crate::state::BlendMode::Rounded,
    );
    crate::bake::render_decos_layer(
        gfx,
        editor.decos_rt.as_ref().expect("rt just created"),
        assets,
        &project.map,
        registry,
        0,
        true,
        true,
    );
    // egui-текстуры обоих слоёв (zero-copy, Nearest — пиксель-арт).
    let mut register = |gfx: &mut crate::gfx::Gfx,
                         egui: &mut crate::egui_layer::EguiLayer,
                         rt: &crate::gfx::Rt,
                         current: &mut Option<egui::TextureId>| {
        let view = gfx.rt_view(rt);
        match *current {
            Some(id) if !rt_recreated => {}
            Some(id) => egui.update_native_texture(&gfx.device, &view, wgpu::FilterMode::Nearest, id),
            None => *current = Some(egui.register_native_texture(&gfx.device, &view, wgpu::FilterMode::Nearest)),
        }
    };
    register(gfx, egui, editor.rt.as_ref().unwrap(), &mut editor.egui_tex);
    register(
        gfx,
        egui,
        editor.decos_rt.as_ref().unwrap(),
        &mut editor.decos_tex,
    );
    // Маркеры E1..E4: нативные egui-текстуры из gfx-ассетов (однократно,
    // кэш в EditorUi.marker_cache).
    for name in ["E1.png", "E2.png", "E3.png", "E4.png"] {
        if editor.marker_cache.contains_key(name) {
            continue;
        }
        if let Some(&tex) = assets.inner.get(name) {
            let view = gfx.texture_view(tex);
            let id = egui.register_native_texture(&gfx.device, &view, wgpu::FilterMode::Nearest);
            editor.marker_cache.insert(name.to_string(), id);
        }
    }
    // Сабмит ТОЛЬКО RT-пассов (запечка), БЕЗ экрана: end_frame заберёт
    // deferred_frame/презент — а экранный кадр этого же кадра ещё рисуется
    // (draw_editor + egui поверх). Это была причина чёрного канваса и
    // «недошедшего» egui: двойной end_frame ломал конвейер кадра.
    gfx.submit_rt_passes();
    editor.baked = true;
    editor.bake_dirty = false;
}

fn draw_editor(ctx: &mut Ctx) -> bool {
    let viewport = ctx.input.screen_size();
    // Фон экрана.
    ctx.gfx
        .begin_pass(Target::Screen, Some(colors::WHITE), &ctx.ui.camera);
    let keep_open = egui_screen_ui(ctx);
    let _ = viewport;
    keep_open
}

/// egui-каркас экрана: top bar / канвас / инструменты / статус.
/// Возвращает false, если нажали «выйти в меню».
fn egui_screen_ui(ctx: &mut Ctx) -> bool {
    // Borrow-split: egui-слой и editor-половина берутся как два независимых
    // &mut-заимствования разных полей Ctx (деструктуризация).
    let Ctx {
        egui,
        input,
        gfx,
        assets,
        tile_pixels,
        registry,
        editor,
        ui,
        rt,
        ..
    } = ctx;
    let mut rest = EditorCtx {
        gfx,
        input,
        assets,
        tile_pixels,
        registry,
        editor,
        ui,
        rt,
    };
    let egui_layer = &mut **egui;
    let mut state = ScreenState {
        keep_open: true,
        actions: Actions { click_tile: None },
    };
    if std::env::var("DT_EGUI_DEBUG").is_ok() {
        eprintln!("egui: run() start");
    }
    egui_layer.run(input, 1.0, |ui| {
        ui.ctx().all_styles_mut(|style| {
            style.visuals.panel_fill = Color32::from_rgb(24, 24, 28);
        });
        // Результаты async-диалогов (открыть/сохранить) — до панелей.
        poll_dialogs(&mut rest);
        egui::Panel::top("editor_top").show(ui, |ui| {
            top_bar(ui, &mut rest, &mut state);
        });
        egui::Panel::bottom("editor_status").show(ui, |ui| {
            status_bar(ui, &mut rest);
        });
        egui::Panel::left("editor_tools").show(ui, |ui| {
            tool_panel(ui, &mut rest);
        });
        egui::CentralPanel::default().show(ui, |ui| {
            state.actions.click_tile = canvas(ui, &mut rest);
        });
        // Модальное окно размера новой карты (поверх панелей).
        size_dialog_window(ui, &mut rest);
    });
    state.keep_open
}

/// Мутабельное состояние экрана внутри egui-замыкания.
struct ScreenState {
    keep_open: bool,
    actions: Actions,
}

/// Разделение Ctx: egui-слой отдельно от остального (editor-половина).
struct EditorCtx<'a> {
    gfx: &'a mut crate::gfx::Gfx,
    input: &'a crate::ui::InputState,
    assets: &'a crate::assets::Assets,
    tile_pixels: &'a [image::RgbaImage],
    registry: &'a mut dt_lib::registry::GameInfo,
    editor: &'a mut EditorUi,
    ui: &'a mut crate::state::UiState,
    rt: &'a tokio::runtime::Runtime,
}

/// Действия, собранные до egui-панелей (клики по канвасу, кнопки).
struct Actions {
    click_tile: Option<(usize, usize)>,
}

fn collect_actions(ctx: &mut Ctx) -> Actions {
    // Клик по канвасу собирается в canvas(); здесь заготовка.
    Actions { click_tile: None }
}

fn top_bar(ui: &mut Ui, ectx: &mut EditorCtx, state: &mut ScreenState) {
    let keep_open = &mut state.keep_open;
    ui.horizontal(|ui| {
        ui.menu_button("Файл", |ui| {
            if ui.button("Новая…").clicked() {
                ectx.editor.size_dialog = Some(crate::state::NewMapDialog { size: 50 });
            }
            if ui.button("Открыть…").clicked() {
                open_dialog(ectx);
            }
            if ui.button("Сохранить").clicked() {
                save_project(ectx);
            }
            if ui.button("Сохранить как…").clicked() {
                save_as_dialog(ectx);
            }
            if ui.button("Выйти в меню").clicked() {
                *keep_open = false;
            }
        });
        ui.menu_button("Правка", |ui| {
            let can_undo = ectx.editor.history.can_undo();
            let can_redo = ectx.editor.history.can_redo();
            if ui.add_enabled(can_undo, egui::Button::new("Отменить\tCtrl+Z")).clicked() {
                let editor = &mut ectx.editor;
                if let Some(name) = editor.history.undo(&mut editor.state) {
                    editor.status = format!("Undo: {name}");
                    editor.bake_dirty = true;
                }
            }
            if ui.add_enabled(can_redo, egui::Button::new("Повторить\tCtrl+Shift+Z")).clicked() {
                let editor = &mut ectx.editor;
                if let Some(name) = editor.history.redo(&mut editor.state) {
                    editor.status = format!("Redo: {name}");
                    editor.bake_dirty = true;
                }
            }
        });
        if ui.button("Lint").clicked() {
            let project = ectx.editor.state.project().clone();
            ectx.editor.issues = editor_validators::run_all(&project);
            let errors = ectx
                .editor
                .issues
                .iter()
                .filter(|i| i.severity == editor_validators::Severity::Error)
                .count();
            ectx.editor.status = format!("Lint: {errors} ошибок");
        }
        ui.separator();
        if ui.button("← Меню").clicked() {
            *keep_open = false;
        }
        ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
            ui.label(RichText::new("РЕДАКТОР КАРТ").strong());
        });
    });
}

/// Панель инструментов слева: инструменты, палитра тайлов, инфо.
fn tool_panel(ui: &mut Ui, ectx: &mut EditorCtx) {
    ui.heading("Инструменты");
    let active = ectx.editor.tool;
    for tool in editor_core::Tool::ALL {
        if ui
            .selectable_label(active == tool, tool.label())
            .clicked()
        {
            ectx.editor.tool = tool;
        }
    }
    ui.separator();
    ui.heading("Тайлы");
    ui.horizontal_wrapped(|ui| {
        for tile in 0..editor_core::project::TILE_COUNT {
            if ui
                .selectable_label(ectx.editor.active_tile == tile, format!("{tile}"))
                .clicked()
            {
                ectx.editor.active_tile = tile;
                ectx.editor.state.set_active_tile(tile);
            }
        }
    });
    ui.separator();
    ui.label(format!(
        "undo: {} redo: {}",
        ectx.editor.history.undo_len(),
        if ectx.editor.history.can_redo() { "+" } else { "-" },
    ));
}

/// Лимиты зума канваса в экранных единицах (screen-px на world-px):
/// MIN — карта занимает ≥ 1/50 канваса, MAX — ≤ 16 экранов на карту.
pub const ZOOM_MIN: f32 = 0.02;
pub const ZOOM_MAX: f32 = 16.0;

/// Канвас: RT-текстура карты, зум/пан колесом и ПКМ-драг, ховер, клик.
fn canvas(ui: &mut Ui, ectx: &mut EditorCtx) -> Option<(usize, usize)> {
    if !ectx.editor.baked {
        ui.label("Запекание…");
        return None;
    }
    let tex_id = ectx.editor.egui_tex.expect("baked => egui_tex зарегистрирован");
    let size = ectx.editor.state.project().size();
    let world_w = SIZE.0 * size as f32;
    let world_h = SIZE.1 * size as f32;
    let available = ui.available_size();
    // Камера: вся карта вписана; зум колесом вокруг курсора, пан ПКМ.
    let mut cam = ectx.editor.cam;
    let (response, painter) = ui.allocate_painter(available, Sense::click_and_drag());
    let rect = response.rect;
    // Базовый масштаб: вписать карту в прямоугольник канваса.
    let fit = (rect.width() / world_w).min(rect.height() / world_h);
    let zoom = cam.zoom[0] * world_w * 0.5;
    // world→screen: центр карты в центре канваса.
    // Копируем target в локал: замыкания не должны держать заём cam.
    let cam_target = cam.target;
    let world_to_screen = |p: [f32; 2]| -> egui::Pos2 {
        let cx = rect.center().x + (p[0] - cam_target[0]) * zoom;
        let cy = rect.center().y + (p[1] - cam_target[1]) * zoom;
        egui::pos2(cx, cy)
    };
    let screen_to_world = |p: egui::Pos2| -> [f32; 2] {
        [
            cam_target[0] + (p.x - rect.center().x) / zoom,
            cam_target[1] + (p.y - rect.center().y) / zoom,
        ]
    };
    // Слои: тайлы (RT0) и декорации/строения/армии (RT1) — обе нативные
    // egui-текстуры (zero-copy), порядок как в игре: тайлы, затем декор.
    let draw_min = world_to_screen([0., 0.]);
    let draw_max = world_to_screen([world_w, world_h]);
    let draw_rect = egui::Rect::from_min_max(draw_min, draw_max);
    let uv = egui::Rect::from_min_max(egui::pos2(0., 0.), egui::pos2(1., 1.));
    painter.image(tex_id, draw_rect, uv, Color32::WHITE);
    if let Some(decos_tex) = ectx.editor.decos_tex {
        painter.image(decos_tex, draw_rect, uv, Color32::WHITE);
    }
    // Маркеры редактора (E1 армии, E2/E4 фонарики) поверх слоёв.
    {
        let markers = ectx.editor.marker_cache.clone();
        markers_layer(ui, ectx, world_to_screen, &markers);
    }
    // Сетка.
    let grid_step = 8.0 / zoom;
    if zoom > 1.0 {
        let mut x = 0.;
        while x <= world_w {
            let a = world_to_screen([x, 0.]);
            let b = world_to_screen([x, world_h]);
            painter.line_segment([a, b], Stroke::new(1., Color32::from_black_alpha(40)));
            x += grid_step * 4.;
        }
        let mut y = 0.;
        while y <= world_h {
            let a = world_to_screen([0., y]);
            let b = world_to_screen([world_w, y]);
            painter.line_segment([a, b], Stroke::new(1., Color32::from_black_alpha(40)));
            y += grid_step * 4.;
        }
    }
    // Ховер тайла.
    let tile_at = |screen: egui::Pos2| -> Option<(usize, usize)> {
        let w = screen_to_world(screen);
        let tx = (w[0] / SIZE.0).floor();
        let ty = (w[1] / SIZE.1).floor();
        (tx >= 0. && ty >= 0. && (tx as usize) < size && (ty as usize) < size)
            .then_some((tx as usize, ty as usize))
    };
    if let Some(pos) = response.hover_pos() {
        if let Some(tile) = tile_at(pos) {
            let a = world_to_screen([tile.0 as f32 * SIZE.0, tile.1 as f32 * SIZE.1]);
            let b = world_to_screen([
                (tile.0 + 1) as f32 * SIZE.0,
                (tile.1 + 1) as f32 * SIZE.1,
            ]);
            painter.rect_stroke(
                egui::Rect::from_min_max(a, b),
                0.,
                Stroke::new(2., Color32::YELLOW),
                egui::StrokeKind::Inside,
            );
            ectx.editor.state.set_cursor(tile);
        }
    }
    // Выделенная клетка.
    if let Some((sx, sy)) = ectx.editor.selected {
        let a = world_to_screen([sx as f32 * SIZE.0, sy as f32 * SIZE.1]);
        let b = world_to_screen([(sx + 1) as f32 * SIZE.0, (sy + 1) as f32 * SIZE.1]);
        painter.rect_stroke(
            egui::Rect::from_min_max(a, b),
            0.,
            Stroke::new(2., Color32::from_rgb(255, 140, 0)),
            egui::StrokeKind::Inside,
        );
    }
    // Зум колесом вокруг курсора. Лимиты — в ЭКРАННОМ масштабе
    // (screen-px на world-px = zoom[0] * world_w): «подход» = карта
    // занимает ≥ 1/16 канваса, «детально» = ≤ 16 экранов на карту.
    if response.hovered() && ectx.input.wheel != 0. {
        let factor = 1.2f32.powf((ectx.input.wheel / 24.).clamp(-4., 4.));
        let anchor_world = ui
            .input(|i| i.pointer.hover_pos())
            .map(screen_to_world)
            .unwrap_or(cam.target);
        let scale = cam.zoom[0] * world_w;
        let new_scale = (scale * factor).clamp(ZOOM_MIN, ZOOM_MAX);
        let factor = new_scale / scale;
        cam.zoom[0] = new_scale / world_w;
        cam.zoom[1] = cam.zoom[0];
        cam.target = [
            anchor_world[0] + (cam.target[0] - anchor_world[0]) / factor,
            anchor_world[1] + (cam.target[1] - anchor_world[1]) / factor,
        ];
    }
    // Пан ПКМ-драгом.
    if response.dragged_by(egui::PointerButton::Secondary) {
        let delta = response.drag_delta();
        cam.target[0] -= delta.x / zoom;
        cam.target[1] -= delta.y / zoom;
    }
    // Клик/драг ЛКМ: выделение + команда активного инструмента. При
    // зажатой ЛКМ траектория указателя сэмплируется шагом в ПОЛКЛЕТКИ от
    // позиции прошлого кадра — кисть идёт сплошняком без зазоров, кривые
    // движения не срезаются прямыми линиями (сегмент кадра короткий,
    // кривизна внутри него ничтожна). Noop-команды в историю не попадают.
    let mut clicked: Option<(usize, usize)> = None;
    let painting = response.clicked() || response.dragged_by(egui::PointerButton::Primary);
    if painting {
        let pointer = response
            .interact_pointer_pos()
            .or_else(|| response.hover_pos());
        if let Some(pos) = pointer {
            let to = screen_to_world(pos);
            let from = ectx.editor.brush_pos.replace(to).unwrap_or(to);
            // Шаг сэмплирования — пол клетки по X; минимум один сэмпл.
            let step = SIZE.0 * 0.5;
            let dist = ((to[0] - from[0]).powi(2) + (to[1] - from[1]).powi(2)).sqrt();
            let steps = ((dist / step).ceil() as usize).clamp(1, 64);
            for k in 1..=steps {
                let t = k as f32 / steps as f32;
                let p = [
                    from[0] + (to[0] - from[0]) * t,
                    from[1] + (to[1] - from[1]) * t,
                ];
                let tx = (p[0] / SIZE.0).floor();
                let ty = (p[1] / SIZE.1).floor();
                if tx < 0. || ty < 0. || tx >= size as f32 || ty >= size as f32 {
                    continue;
                }
                let tile = (tx as usize, ty as usize);
                ectx.editor.selected = Some(tile);
                apply_tool(ectx, tile);
                clicked = Some(tile);
            }
        }
    } else {
        // Кисть отпущена: следующее нажатие начнёт новую траекторию.
        ectx.editor.brush_pos = None;
    }
    clicked
}

/// Слой редакторских маркеров поверх канваса (egui painter):
/// E1 — неактивные армии (active=false), активные рисуются модельками
/// в RT-слое; E2 — фонарик без событий; E4 — фонарик с событиями
/// (eventmap той же клетки непуст). E3 «чисто локальные события» —
fn markers_layer(
    ui: &mut Ui,
    ectx: &mut EditorCtx,
    world_to_screen: impl Fn([f32; 2]) -> egui::Pos2,
    markers: &std::collections::HashMap<String, egui::TextureId>,
) {
    let Some(e1) = markers.get("E1.png").copied() else { return; };
    let e2 = markers.get("E2.png").copied();
    let e4 = markers.get("E4.png").copied();
    let project = ectx.editor.state.project();
    let painter = ui.painter();
    let size = project.size();
    for army in &project.map.armys {
        if army.active {
            continue; // активные — игровой моделькой в RT-слое
        }
        let (i, j) = army.pos;
        if i >= size || j >= size {
            continue;
        }
        let pos = world_to_screen([i as f32 * SIZE.0, j as f32 * SIZE.1]);
        painter.image(
            e1,
            egui::Rect::from_min_size(pos, egui::vec2(SIZE.0, SIZE.1 * 2.)),
            egui::Rect::from_min_max(egui::pos2(0., 0.), egui::pos2(1., 1.)),
            Color32::WHITE,
        );
    }
    for light in &project.lights {
        if light.x >= size || light.y >= size {
            continue;
        }
        let pos = world_to_screen([light.x as f32 * SIZE.0, light.y as f32 * SIZE.1]);
        let has_event = !project.map.eventmap[(light.x, light.y)].is_empty();
        let tex = if has_event { e4 } else { e2 };
        let Some(tex) = tex else { continue; };
        painter.image(
            tex,
            egui::Rect::from_min_size(pos, egui::vec2(SIZE.0, SIZE.1 * 2.)),
            egui::Rect::from_min_max(egui::pos2(0., 0.), egui::pos2(1., 1.)),
            Color32::WHITE,
        );
    }
}

/// Применить активный инструмент: построить команду и выполнить.
fn apply_tool(ectx: &mut EditorCtx, tile: (usize, usize)) {
    let editor = &mut ectx.editor;
    let command: Box<dyn editor_core::Command> = match editor.tool {
        editor_core::Tool::Brush => {
            Box::new(PaintTile::new(tile, editor.active_tile))
        }
        editor_core::Tool::Deco => {
            editor.counter += 1;
            Box::new(PlaceDeco::new(tile, editor.counter))
        }
        editor_core::Tool::Building => {
            editor.counter += 1;
            Box::new(PlaceBuilding::new(tile, editor.counter % 14))
        }
        editor_core::Tool::Army => {
            editor.counter += 1;
            Box::new(PlaceArmy::new(tile, format!("Армия {}", editor.counter)))
        }
    };
    match editor.history.execute(command, &mut editor.state) {
        CommandResult::Applied => {
            editor.status = format!("{}: {:?}", editor.tool.label(), tile);
            editor.bake_dirty = true;
        }
        CommandResult::Noop => {
            editor.status = format!("{}: без изменений", editor.tool.label());
        }
    }
}

/// Новый проект: чистый EditorUi (без кэшей прошлой карты).
fn new_project(ectx: &mut EditorCtx, size: usize) {
    let editor = &mut ectx.editor;
    editor.state = editor_core::EditorState::new(editor_core::MapProject::new(size, 0));
    editor.history = editor_core::CommandHistory::new();
    editor.project_path = None;
    editor.issues.clear();
    editor.bake_dirty = true;
    editor.selected = None;
    editor.cam = Camera::from_display_rect(0., SIZE.1 * size as f32, SIZE.0 * size as f32, -SIZE.1 * size as f32);
    // Размер карты изменился — пересоздать RT.
    editor.rt = None;
    editor.baked = false;
    // egui-текстура перепривязается к новому RT при следующем запеке
    // (rt_recreated → update_native_texture).
    editor.status = format!("Новая карта {size}×{size}");
}

/// Open .dtm: parse_dtm_map → convert_dtm_map (registry уже загружен игрой).
fn open_project(ectx: &mut EditorCtx, path: std::path::PathBuf) {
    let result = (|| -> Result<editor_core::MapProject, String> {
        let data = dt_lib::map::convert::parse_dtm_map(&path)
            .map_err(|_| "parse_dtm_map: не удалось разобрать файл".to_string())?;
        let (map, events) =
            dt_lib::map::convert::convert_dtm_map(data, ectx.registry);
        Ok(editor_core::MapProject {
            map,
            events,
            ..Default::default()
        })
    })();
    match result {
        Ok(project) => {
            let size = project.size();
            let editor = &mut ectx.editor;
            editor.state = editor_core::EditorState::new(project);

            editor.history = editor_core::CommandHistory::new();
            editor.project_path = Some(path);
            editor.issues.clear();
            editor.bake_dirty = true;
            editor.selected = None;
            editor.rt = None;
            editor.baked = false;
            // egui-текстура перепривязается к новому RT при следующем запеке
            // (rt_recreated → update_native_texture).
            editor.cam = Camera::from_display_rect(
                0.,
                SIZE.1 * size as f32,
                SIZE.0 * size as f32,
                -SIZE.1 * size as f32,
            );
            editor.status = "Карта открыта".into();
        }
        Err(err) => ectx.editor.status = err,
    }
}

/// Save: валидаторы (Error блокируют) → gamemap_to_dtm → fs::write.
fn save_project(ectx: &mut EditorCtx) {
    let Some(path) = ectx.editor.project_path.clone() else {
        save_as_dialog(ectx);
        return;
    };
    do_save(ectx, path);
}

fn do_save(ectx: &mut EditorCtx, path: std::path::PathBuf) {
    let project = ectx.editor.state.project().clone();
    ectx.editor.issues = editor_validators::run_all(&project);
    if ectx
        .editor
        .issues
        .iter()
        .any(|i| i.severity == editor_validators::Severity::Error)
    {
        ectx.editor.status =
            "Сохранение заблокировано: есть ошибки валидаторов (см. Lint)".into();
        return;
    }
    // gamemap_to_dtm требует registry — синхронная конверсия с уже
    // загруженным реестром игры (parse_* — только при старте).
    let bytes = dt_lib::map::dtm_writer::gamemap_to_dtm(
        &project.map,
        &project.events,
        ectx.registry,
    );
    match bytes {
        Some(bytes) => match std::fs::write(&path, bytes) {
            Ok(()) => {
                ectx.editor.status = format!("Сохранено: {}", path.display());
                ectx.editor.project_path = Some(path);
            }
            Err(e) => ectx.editor.status = format!("write: {e}"),
        },
        None => {
            ectx.editor.status =
                "gamemap_to_dtm: данные нерепрезентабельны в dtm".into();
        }
    }
}


/// Статус-бар: координаты, инструмент, валидаторы, история, путь.
fn status_bar(ui: &mut Ui, ectx: &mut EditorCtx) {
    ui.horizontal(|ui| {
        let editor = &ectx.editor;
        let (cx, cy) = editor.state.cursor();
        ui.label(format!("x: {cx}  y: {cy}"));
        ui.separator();
        ui.label(editor.tool.label());
        ui.separator();
        let errors = editor
            .issues
            .iter()
            .filter(|i| i.severity == editor_validators::Severity::Error)
            .count();
        let warnings = editor
            .issues
            .iter()
            .filter(|i| i.severity == editor_validators::Severity::Warning)
            .count();
        ui.label(format!("ошибок: {errors}  предупреждений: {warnings}"));
        ui.separator();
        ui.label(format!(
            "undo: {}",
            editor.history.undo_len()
        ));
        ui.separator();
        let path = editor
            .project_path
            .as_ref()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|| "без имени".into());
        ui.label(path);
        if !editor.status.is_empty() {
            ui.separator();
            ui.label(RichText::new(&editor.status).color(Color32::from_rgb(255, 200, 100)));
        }
    });
}

// ---------------- rfd-диалоги: async на tokio-рантайме ----------------
//
// Блокирующий rfd::FileDialog на xdg-портале паниковал: zbus требует
// Tokio-реактор в потоке. AsyncFileDialog запускается на ctx.rt (State),
// результат приходит через oneshot-канал и забирается в кадре редактора.

/// Запуск неблокирующего диалога открытия: результат → editor.open_slot.
fn open_dialog(ectx: &mut EditorCtx) {
    if ectx.editor.open_slot.is_some() {
        return; // диалог уже открыт
    }
    let (tx, rx) = tokio::sync::oneshot::channel();
    ectx.editor.open_slot = Some(rx);
    ectx.rt.spawn(async move {
        let file = rfd::AsyncFileDialog::new()
            .set_title("Открыть карту Discord Times")
            .add_filter("Discord Times map", &["dtm", "DTm"])
            .pick_file()
            .await;
        let path = file.map(|f| f.path().to_path_buf());
        // Отмена диалога (None) тоже долетает — статус обновится.
        let _ = tx.send(path.unwrap_or_default());
    });
}

/// Запуск неблокирующего диалога сохранения: результат → editor.save_slot.
fn save_as_dialog(ectx: &mut EditorCtx) {
    if ectx.editor.save_slot.is_some() {
        return; // диалог уже открыт
    }
    let (tx, rx) = tokio::sync::oneshot::channel();
    ectx.editor.save_slot = Some(rx);
    ectx.rt.spawn(async move {
        let file = rfd::AsyncFileDialog::new()
            .set_title("Сохранить карту Discord Times")
            .add_filter("Discord Times map", &["dtm", "DTm"])
            .set_file_name("map.dtm")
            .save_file()
            .await;
        let path = file.map(|f| f.path().to_path_buf());
        let _ = tx.send(path.unwrap_or_default());
    });
}

/// Поллинг слотов диалогов в кадре: try_recv, без блокировки.
fn poll_dialogs(ectx: &mut EditorCtx) {
    let mut opened: Option<std::path::PathBuf> = None;
    let mut saved: Option<std::path::PathBuf> = None;
    if let Some(rx) = ectx.editor.open_slot.as_mut() {
        if let Ok(path) = rx.try_recv() {
            ectx.editor.open_slot = None;
            if !path.as_os_str().is_empty() {
                opened = Some(path);
            }
        }
    }
    if let Some(rx) = ectx.editor.save_slot.as_mut() {
        if let Ok(path) = rx.try_recv() {
            ectx.editor.save_slot = None;
            if !path.as_os_str().is_empty() {
                saved = Some(path);
            }
        }
    }
    if let Some(path) = opened {
        open_project(ectx, path);
    }
    if let Some(path) = saved {
        do_save(ectx, path);
    }
}

/// Окно «Новая карта»: размер 16..200 (слайдер + числовой ввод).
fn size_dialog_window(ui: &mut Ui, ectx: &mut EditorCtx) {
    let Some(dialog) = ectx.editor.size_dialog.as_mut() else {
        return;
    };
    let mut create = false;
    let mut close = false;
    egui::Window::new("Новая карта")
        .anchor(egui::Align2::CENTER_CENTER, [0., 0.])
        .collapsible(false)
        .resizable(false)
        .show(ui.ctx(), |ui| {
            ui.label("Размер стороны карты (тайлов):");
            ui.add(
                egui::Slider::new(&mut dialog.size, 16..=200)
                    .clamping(egui::SliderClamping::Always)
                    .text("тайлов"),
            );
            ui.add(egui::DragValue::new(&mut dialog.size).range(16..=200).speed(1.));
            ui.horizontal(|ui| {
                if ui.button("Создать").clicked() {
                    create = true;
                }
                if ui.button("Отмена").clicked() {
                    close = true;
                }
            });
        });
    if create {
        let size = ectx.editor.size_dialog.unwrap().size.clamp(16, 200);
        ectx.editor.size_dialog = None;
        new_project(ectx, size);
    } else if close {
        ectx.editor.size_dialog = None;
    }
}
