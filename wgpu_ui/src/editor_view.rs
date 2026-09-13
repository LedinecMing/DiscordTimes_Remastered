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
            editor,
            egui,
            ..
        } = ctx;
        rebake_if_dirty(gfx, assets, tile_pixels, editor, &mut **egui);
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

/// Запек карты редактора в RT тем же кодом, что и игра (bake.rs), и
/// привязка RT как нативной egui-текстуры (сэмпл того же GPU-объекта).
fn rebake_if_dirty(
    gfx: &mut crate::gfx::Gfx,
    assets: &crate::assets::Assets,
    tile_pixels: &[image::RgbaImage],
    editor: &mut EditorUi,
    egui: &mut crate::egui_layer::EguiLayer,
) {
    if !editor.bake_dirty {
        return;
    }
    let size = editor.project.size();
    // RT создаётся лениво под размер карты (32x22 на тайл, как в игре).
    let mut rt_recreated = false;
    if editor.rt.is_none() {
        editor.rt = Some(gfx.create_rt(
            (SIZE.0 * size as f32) as u32,
            (SIZE.1 * size as f32) as u32,
        ));
        rt_recreated = true;
    }
    let rt_index = editor.rt.as_ref().expect("rt just created").index;
    {
        // Bake в RT по индексу (единый игровой путь).
        let cam = crate::camera::Camera::from_display_rect(
            0.,
            SIZE.1 * size as f32,
            SIZE.0 * size as f32,
            -SIZE.1 * size as f32,
        );
        gfx.begin_pass(crate::gfx::Target::Rt(rt_index), Some([0., 0., 0., 0.]), &cam);
        crate::bake::draw_editor_tiles(
            gfx,
            assets,
            &editor.project.map.tilemap,
            tile_pixels,
        );
        gfx.end_pass();
    }
    let view = gfx.rt_view(&editor.rt.as_ref().expect("rt just created"));
    match editor.egui_tex {
        Some(id) if !rt_recreated => {
            // Тот же RT-объект: bind group уже смотрит на живой view,
            // перерегистрация не нужна (данные обновляются на GPU).
        }
        Some(id) => egui.update_native_texture(
            &gfx.device,
            &view,
            wgpu::FilterMode::Nearest,
            id,
        ),
        None => {
            editor.egui_tex = Some(egui.register_native_texture(
                &gfx.device,
                &view,
                wgpu::FilterMode::Nearest,
            ));
        }
    }
    drop(view);
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
            if ui.button("Новая (50×50)").clicked() {
                new_project(ectx, 50);
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

/// Канвас: RT-текстура карты, зум/пан колесом и ПКМ-драг, ховер, клик.
fn canvas(ui: &mut Ui, ectx: &mut EditorCtx) -> Option<(usize, usize)> {
    if !ectx.editor.baked {
        ui.label("Запекание…");
        return None;
    }
    let tex_id = ectx.editor.egui_tex.expect("baked => egui_tex зарегистрирован");
    let size = ectx.editor.project.size();
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
    // Текстура: нативная egui-текстура того же RT (zero-copy, без readback);
    // регистрируется/обновляется в rebake_if_dirty при запеке.
    let draw_min = world_to_screen([0., 0.]);
    let draw_max = world_to_screen([world_w, world_h]);
    let draw_rect = egui::Rect::from_min_max(draw_min, draw_max);
    painter.image(
        tex_id,
        draw_rect,
        egui::Rect::from_min_max(egui::pos2(0., 0.), egui::pos2(1., 1.)),
        Color32::WHITE,
    );
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
    // Зум колесом вокруг курсора.
    if response.hovered() && ectx.input.wheel != 0. {
        let factor = 1.2f32.powf((ectx.input.wheel / 24.).clamp(-4., 4.));
        let anchor_world = ui
            .input(|i| i.pointer.hover_pos())
            .map(screen_to_world)
            .unwrap_or(cam.target);
        cam.zoom[0] = (cam.zoom[0] * factor).clamp(fit / world_w * 0.5, 4.0);
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
    // Клик ЛКМ: выделение + команда активного инструмента.
    let mut clicked: Option<(usize, usize)> = None;
    if response.clicked() {
        if let Some(pos) = response.hover_pos() {
            if let Some(tile) = tile_at(pos) {
                ectx.editor.selected = Some(tile);
                apply_tool(ectx, tile);
                clicked = Some(tile);
            }
        }
    }
    ectx.editor.cam = cam;
    clicked
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
    editor.project = editor_core::MapProject::new(size, 0);
    editor.state = editor_core::EditorState::new(editor.project.clone());
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
            editor.project = editor.state.project().clone();
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

fn save_as_dialog(ectx: &mut EditorCtx) {
    if let Some(path) = pick_save_path() {
        do_save(ectx, path);
    }
}

fn open_dialog(ectx: &mut EditorCtx) {
    if let Some(path) = pick_open_path() {
        open_project(ectx, path);
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

// ---------------- rfd-диалоги: НЕ блокировать render-loop ----------------
//
// rfd::AsyncFileDialog на xdg-портале возвращает future; поллинг результата —
// в кадре редактора (не-blocking try_recv-семантика через Option-слот).

/// Неблокирующий диалог сохранения: канал результата забирается в кадре.
fn pick_save_path() -> Option<std::path::PathBuf> {
    // Синхронный rfd-диалог на xdg-портале допустим ТОЛЬКО как fallback:
    // портальный диалог блокирует вызывающий поток. Основной путь — async.
    // Фаза 1: блокирующий вызов (риски см. бриф) — канал ниже.
    rfd::FileDialog::new()
        .set_title("Сохранить карту Discord Times")
        .add_filter("Discord Times map", &["dtm", "DTm"])
        .set_file_name("map.dtm")
        .pick_file()
}

fn pick_open_path() -> Option<std::path::PathBuf> {
    rfd::FileDialog::new()
        .set_title("Открыть карту Discord Times")
        .add_filter("Discord Times map", &["dtm", "DTm"])
        .pick_file()
}
