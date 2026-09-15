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
use crate::state::{EditorUi, Menu, MultiPick, SIZE};
use crate::Ctx;
use editor_core::command::{BatchPlace, PaintTile, PlaceArmy, PlaceBuilding, PlaceDeco};
use editor_core::CommandResult;
use egui::{
    Align2, Color32, FontId, Frame, Layout, RichText, Sense, Stroke, Ui, Vec2,
};
use winit::keyboard::KeyCode;

/// Директория тайлов относительно cwd (бинарник запускается из dt/, как клиент).
const ASSETS_TERRAIN: &str = "assets/Terrain";

/// Открыть/закрыть экран: возврат false = выйти в главное меню.
pub fn editor_screen(ctx: &mut Ctx) -> bool {
    // Дефолтная карта при первом входе (путь из каталога запуска dt/),
    // чтобы редактор стартовал с живыми данными, а не пустым 50×50.
    if !ctx.editor.baked && ctx.editor.project_path.is_none() {
        let default_map = std::path::PathBuf::from("Maps_Rus/Другой берег.DTm");
        if default_map.exists() {
            // Раздельные &mut-заимствования: загрузка трогает только
            // registry+editor; RT-запек ниже перечитает свежий state.
            let Ctx {
                registry,
                editor,
                ..
            } = ctx;
            open_project_parts(registry, editor, &default_map);
        }
    }
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

/// Хоткеи редактора: Ctrl+Z / Ctrl+Shift+Z / Ctrl+Y, Escape — выход.
///
/// Повтор при удержании: первая отмена — по нажатию, далее автоповтор
/// 4 раза/сек (задержка до первого повтора 0.4с), таймер в
/// EditorUi.hotkey_repeat_at (crate::time_secs).
const REPEAT_DELAY: f64 = 0.4;
const REPEAT_INTERVAL: f64 = 0.25; // 4 повтора/сек

fn handle_hotkeys(ctx: &mut Ctx) {
    let ctrl = ctx.input.key_down(KeyCode::ControlLeft)
        || ctx.input.key_down(KeyCode::ControlRight);
    let shift = ctx.input.key_down(KeyCode::ShiftLeft)
        || ctx.input.key_down(KeyCode::ShiftRight);
    let undo_held = ctrl && ctx.input.key_down(KeyCode::KeyZ) && !shift;
    let redo_held = (ctrl && ctx.input.key_down(KeyCode::KeyY))
        || (ctrl && ctx.input.key_down(KeyCode::KeyZ) && shift);
    let now = crate::time_secs();

    // Один и тот же повтор для undo/redo: действие выбирается зажатой
    // клавишей; приоритет у redo при удержании обоих.
    let undo_pressed = ctrl && ctx.input.key_pressed(KeyCode::KeyZ) && !shift;
    let redo_pressed = (ctrl && ctx.input.key_pressed(KeyCode::KeyY))
        || (ctrl && ctx.input.key_pressed(KeyCode::KeyZ) && shift);
    let mut fire_undo = false;
    let mut fire_redo = false;
    if undo_pressed || redo_pressed {
        // Нажатие: мгновенное срабатывание + старт таймера повтора.
        fire_undo = undo_pressed;
        fire_redo = redo_pressed && !undo_pressed;
        ctx.editor.hotkey_repeat_at = Some(now);
    } else if undo_held || redo_held {
        // Удержание: автоповтор по таймеру (задержка, затем интервал).
        if let Some(last) = ctx.editor.hotkey_repeat_at {
            let elapsed = now - last;
            let due = if elapsed >= REPEAT_DELAY {
                (elapsed - REPEAT_DELAY) / REPEAT_INTERVAL >= 1.
            } else {
                false
            };
            if due {
                fire_redo = redo_held;
                fire_undo = undo_held && !redo_held;
                ctx.editor.hotkey_repeat_at = Some(now - REPEAT_DELAY);
            }
        } else {
            // Клавиша была зажата до входа в редактор: старт таймера.
            ctx.editor.hotkey_repeat_at = Some(now);
        }
    } else {
        ctx.editor.hotkey_repeat_at = None;
    }

    if fire_undo {
        let editor = &mut ctx.editor;
        if let Some(name) = editor.history.undo(&mut editor.state) {
            editor.status = format!("Undo: {name}");
            editor.bake_dirty = true;
        }
    }
    if fire_redo {
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
    // армии (модельки/корабли на воде). Настройки слоёв — вкладка «Рендер»
    // (blend_mode, строения, армии); маркеры редактора — egui-слой.
    crate::bake::bake_map_textures(
        gfx,
        editor.rt.as_ref().expect("rt just created"),
        assets,
        &project.map,
        tile_pixels,
        editor.render_settings.blend_mode,
    );
    crate::bake::render_decos_layer(
        gfx,
        editor.decos_rt.as_ref().expect("rt just created"),
        assets,
        &project.map,
        registry,
        0,
        editor.render_settings.buildings,
        editor.render_settings.armies,
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
    // Маркеры E1..E4: родные egui-текстуры из PNG на диске (load_texture —
    // premultiplied alpha; PNG через нативную регистрацию wgpu рисовался
    // белым фоном: straight-alpha ≠ формат egui-рендерера; readback GPU
    // недопустим — текстуры ассетов без COPY_SRC).
    for name in ["E1.png", "E2.png", "E3.png", "E4.png"] {
        if editor.marker_handles.contains_key(name) {
            continue;
        }
        let Ok(img) = image::open(format!("assets_editor/{name}")) else {
            continue;
        };
        let img = img.to_rgba8();
        let (w, h) = img.dimensions();
        let image = egui::ColorImage::from_rgba_unmultiplied(
            [w as usize, h as usize],
            img.as_raw(),
        );
        let handle = egui
            .ctx
            .load_texture(name, image, egui::TextureOptions::NEAREST);
        editor.marker_handles.insert(name.to_string(), handle);
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
    egui_layer.run(input, 1.0, |ui| {
        ui.ctx().all_styles_mut(|style| {
            style.visuals.panel_fill = Color32::from_rgb(24, 24, 28);
        });
        // Результаты async-диалогов (открыть/сохранить) — до панелей.
        poll_dialogs(&mut rest);
        // Отложенные goto-definition открытия событий — до панелей.
        flush_pending_event_opens(&mut rest);
        egui::Panel::top("editor_top").show(ui, |ui| {
            top_bar(ui, &mut rest, &mut state);
        });
        egui::Panel::bottom("editor_status").show(ui, |ui| {
            status_bar(ui, &mut rest);
        });
        egui::Panel::left("editor_tools")
            .default_size(240.)
            .min_size(200.)
            .resizable(true)
            .show(ui, |ui| {
                tool_panel(ui, &mut rest);
            });
        egui::Panel::right("editor_info_dock_panel")
            .default_size(300.)
            .min_size(220.)
            .resizable(true)
            .show_inside(ui, |ui| {
                egui::ScrollArea::vertical()
                    .id_salt("info_dock_scroll")
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        info_dock_panel(ui, &mut rest);
                    });
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
        // «Рендер»: поповер с настройками слоёв канваса.
        let render_resp = ui.button("Рендер");
        egui::Popup::from_toggle_button_response(&render_resp)
            .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
            .show(|ui| render_settings_popup(ui, ectx));
        ui.separator();
        if ui.button("← Меню").clicked() {
            *keep_open = false;
        }
        ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
            ui.label(RichText::new("РЕДАКТОР КАРТ").strong());
        });
    });
}

/// Вкладка панели инструментов. Свойства точки событий — в инфоокне
/// (интеракт); настройки рендера — поповер в тулбаре.
#[derive(Clone, Copy, PartialEq, Eq)]
enum PaletteTab {
    /// Палитра активного инструмента (тайлы/декор/строения/армии).
    Tool,
    /// Палитра точек событий/фонариков (постановка на карту).
    Events,
}

impl PaletteTab {
    fn label(self) -> &'static str {
        match self {
            PaletteTab::Tool => "Палитра",
            PaletteTab::Events => "События",
        }
    }
}


/// Активная вкладка панели (нетабличное состояние кадра — egui id).
const TAB_STATE: &str = "editor_palette_tab";

/// Панель инструментов слева: сегмент режимов РИСОВАНИЕ/ИНТЕРАКТ,
/// настройки кисти (в рисовании), вкладки палитры (гриды иконок
/// с поиском и фильтрами категорий).
fn tool_panel(ui: &mut Ui, ectx: &mut EditorCtx) {
    ui.heading("Инструменты");
    // Сегмент режима: РИСОВАНИЕ (кисть/заливка/декор/строения/армии)
    // против ИНТЕРАКТ (выбор/перенос объектов). Внутри рисования —
    // конкретный инструмент; интеракт — сам инструмент.
    let mode = if ectx.editor.tool.is_paint() {
        Mode::Paint
    } else {
        Mode::Interact
    };
    ui.horizontal(|ui| {
        for (m, label) in [(Mode::Paint, "РИСОВАНИЕ"), (Mode::Interact, "ИНТЕРАКТ")] {
            if ui
                .selectable_label(ectx.editor.tool.is_paint() == m.is_paint(), label)
                .clicked()
            {
                ectx.editor.tool = match m {
                    Mode::Paint => editor_core::Tool::Brush,
                    Mode::Interact => editor_core::Tool::Interact,
                };
            }
        }
    });
    if mode == Mode::Paint {
        ui.separator();
        ui.horizontal(|ui| {
            for tool in [
                editor_core::Tool::Brush,
                editor_core::Tool::BucketFill,
                editor_core::Tool::Deco,
                editor_core::Tool::Building,
                editor_core::Tool::Army,
            ] {
                let active = ectx.editor.tool == tool;
                if ui.selectable_label(active, tool.label()).clicked() {
                    ectx.editor.tool = tool;
                }
            }
        });
        brush_settings(ui, ectx);
    } else {
        ui.separator();
        ui.label(
            RichText::new("ЛКМ — выбрать объект (инфоокно),\nПКМ — перенос (клик-клик или драг),\nEsc — отмена переноса.")
                .weak()
                .small(),
        );
    }
    // Внутри интеракта палитра не нужна; вкладки и undo-строка — общие.
    if mode == Mode::Paint {
        ui.separator();
        let mut tab: i32 = ui
            .ctx()
            .data_mut(|d| d.get_temp(egui::Id::new(TAB_STATE)))
            .unwrap_or(0);
        ui.horizontal(|ui| {
            for (i, t) in [PaletteTab::Tool, PaletteTab::Events].iter().enumerate() {
                if ui
                    .selectable_label(tab == i as i32, t.label())
                    .clicked()
                {
                    tab = i as i32;
                }
            }
        });
        ui.ctx()
            .data_mut(|d| d.insert_temp(egui::Id::new(TAB_STATE), tab));
        match tab {
            1 => events_palette(ui, ectx),
            _ => palette_tab(ui, ectx),
        }
    }
    ui.separator();
    ui.label(format!(
        "undo: {} redo: {}",
        ectx.editor.history.undo_len(),
        if ectx.editor.history.can_redo() { "+" } else { "-" },
    ));
}

/// Режим тулбара: рисование (кисть/заливка/постановка) или интеракт.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Mode {
    Paint,
    Interact,
}

impl Mode {
    fn is_paint(self) -> bool {
        self == Mode::Paint
    }
}

/// Настройки кисти (п.3): форма, размер, заливка, мульти-выбор.
fn brush_settings(ui: &mut Ui, ectx: &mut EditorCtx) {
    let (shape, size) = {
        let b = &mut ectx.editor.brush;
        ui.separator();
        ui.label("Кисть:");
        ui.horizontal(|ui| {
            for s in crate::state::BrushShape::ALL {
                if ui.selectable_label(b.shape == s, s.label()).clicked() {
                    b.shape = s;
                }
            }
        });
        ui.horizontal(|ui| {
            ui.label("Размер:");
            ui.add(
                egui::Slider::new(&mut b.size, 1..=16)
                    .clamping(egui::SliderClamping::Always),
            );
        });
        (b.shape, b.size)
    };
    let _ = (shape, size);
    // Заливка: параметры актуальны только для BucketFill.
    {
        let b = &mut ectx.editor.brush;
        if ectx.editor.tool == editor_core::Tool::BucketFill {
            ui.horizontal(|ui| {
                ui.label("Заливка: дальность");
                ui.add(
                    egui::DragValue::new(&mut b.fill_max_range)
                        .range(0..=256)
                        .speed(1.),
                );
                ui.label("(0 = ∞)");
            });
            ui.horizontal(|ui| {
                ui.label("макс. объём");
                ui.add(
                    egui::DragValue::new(&mut b.fill_max_volume)
                        .range(0..=100_000)
                        .speed(100.),
                );
                ui.label("(0 = ∞)");
            });
        }
    }
    // Мульти-выбор: список активных элементов.
    ui.separator();
    let picks = ectx.editor.brush.multi_select.clone();
    let order = ectx.editor.brush.multi_order;
    ui.label(format!("Мульти-выбор ({}):", picks.len()));
    for (i, pick) in picks.iter().enumerate() {
        let label = multi_pick_label(ectx, *pick);
        ui.horizontal(|ui| {
            ui.label(format!("  {}. {}", i + 1, label));
            if ui.button("✕").clicked() {
                ectx.editor.brush.multi_select.remove(i);
            }
        });
    }
    if !picks.is_empty() {
        ui.horizontal(|ui| {
            ui.label("Порядок:");
            for o in crate::state::MultiOrder::ALL {
                if ui.selectable_label(order == o, o.label()).clicked() {
                    ectx.editor.brush.multi_order = o;
                }
            }
        });
        if ui.button("Очистить мульти-выбор").clicked() {
            ectx.editor.brush.multi_select.clear();
        }
    }
    ui.label(
        RichText::new(
            "Галочка в палитре добавляет элемент;\nкогда список непуст — рисует он (случайно/последовательно).",
        )
        .weak()
        .small(),
    );
}

/// Человекочитаемая подпись элемента мульти-выбора.
fn multi_pick_label(ectx: &EditorCtx, pick: crate::state::MultiPick) -> String {
    match pick {
        crate::state::MultiPick::Tile(t) => {
            let name = dt_lib::map::tile::TILES
                .get(t)
                .map(|t| t.sprite().trim_end_matches(".png").to_string())
                .unwrap_or_else(|| "?".into());
            format!("тайл {} ({})", name, t)
        }
        crate::state::MultiPick::Deco(idx) => {
            let name = ectx
                .registry
                .objects
                .inner
                .get(idx)
                .map(|o| o.name.clone())
                .unwrap_or_else(|| "?".into());
            format!("декор {} ({})", name, idx)
        }
        crate::state::MultiPick::Building(idx) => {
            let name = ectx
                .registry
                .objects
                .inner
                .get(idx)
                .map(|o| o.name.clone())
                .unwrap_or_else(|| "?".into());
            format!("строение {} ({})", name, idx)
        }
        crate::state::MultiPick::Army(unit_id) => {
            let name = ectx
                .registry
                .units
                .get(unit_id)
                .map(|u| u.name.clone())
                .unwrap_or_else(|| "?".into());
            format!("армия {} ({})", name, unit_id)
        }
    }
}


/// Вкладка «События» палитры: два элемента постановки точек.
/// «Фонарик» — map_model=8, активен с начала, radius=3 (обычный свет).
/// «Точка локальных событий» — map_model=9, radius=3 (видна как E4 при
/// событиях; радиус > 0 по дефолту — новую точку видно на карте).
/// Выбор → ЛКМ на канвасе ставит (PlaceLantern); дубль в клетке no-op.
fn events_palette(ui: &mut Ui, ectx: &mut EditorCtx) {
    ui.heading("Точки событий");
    let kind = ectx.editor.active_lantern_kind;
    if ui
        .selectable_label(kind == Some(true), "Фонарик (свет, активен)")
        .clicked()
    {
        ectx.editor.active_lantern_kind = Some(true);
    }
    ui.label(
        RichText::new("map_model=8, активен с начала, радиус 3")
            .weak()
            .small(),
    );
    ui.separator();
    if ui
        .selectable_label(kind == Some(false), "Точка локальных событий")
        .clicked()
    {
        ectx.editor.active_lantern_kind = Some(false);
    }
    ui.label(
        RichText::new("map_model=9, радиус 3; события — в инфоокне точки")
            .weak()
            .small(),
    );
    ui.separator();
    ui.label("Выберите тип и кликните ЛКМ по канвасу.");
    if ectx.editor.active_lantern_kind.is_none() {
        ectx.editor.active_lantern_kind = Some(true);
    }
}

/// Поповер «Рендер» тулбара: переключатели слоёв канваса и наплывы.
fn render_settings_popup(ui: &mut Ui, ectx: &mut EditorCtx) {
    // Слои RT-запека (buildings/armies/blend) применяются перезапеком:
    // любое изменение — bake_dirty, на следующем кадре карта перезапечётся
    // (egui-слои markers/grid/ownership живут в canvas и перезапека не
    // требуют, но перезапек безвреден).
    let rs = &mut ectx.editor.render_settings;
    let before = (rs.markers, rs.grid, rs.ownership, rs.buildings, rs.armies, rs.blend_mode);
    ui.checkbox(&mut rs.markers, "Маркеры E1/E2/E4");
    ui.checkbox(&mut rs.grid, "Сетка");
    ui.checkbox(&mut rs.ownership, "Принадлежность строений");
    ui.checkbox(&mut rs.buildings, "Строения");
    ui.checkbox(&mut rs.armies, "Армии");
    ui.separator();
    ui.label("Наплывы тайлов:");
    let mut blend = rs.blend_mode;
    egui::ComboBox::from_id_salt("blend_mode")
        .selected_text(rs.blend_mode.label())
        .show_ui(ui, |ui| {
            for mode in crate::state::BlendMode::ALL {
                ui.selectable_value(&mut blend, mode, mode.label());
            }
        });
    rs.blend_mode = blend;
    let after = (rs.markers, rs.grid, rs.ownership, rs.buildings, rs.armies, rs.blend_mode);
    if before != after {
        ectx.editor.bake_dirty = true;
        ectx.editor.status = "Рендер: перезапек".into();
    }
}

/// Вкладка палитры: грид иконок активного инструмента. Тайлы — иконки
/// TILES; декорации/строения — иконки registry.objects (по типу объекта);
/// армии — 4 базовых шаблона по Nature + заглушка «свои шаблоны».
fn palette_tab(ui: &mut Ui, ectx: &mut EditorCtx) {
    match ectx.editor.tool {
        editor_core::Tool::Brush => tiles_palette(ui, ectx),
        editor_core::Tool::BucketFill => tiles_palette(ui, ectx),
        editor_core::Tool::Deco => objects_palette(ui, ectx, false),
        editor_core::Tool::Building => objects_palette(ui, ectx, true),
        editor_core::Tool::Army => armies_palette(ui, ectx),
        editor_core::Tool::Interact => {
            ui.label("Интеракт: ЛКМ — выбрать объект,");
            ui.label("ПКМ на объекте — перенос (клик-клик или драг),");
            ui.label("Esc — отмена переноса.");
        }
    }
}

/// Подпись поиска над гридом: фильтрует по подстроке имени или id.
fn search_field(ui: &mut Ui, ectx: &mut EditorCtx) {
    ui.horizontal(|ui| {
        ui.label("Поиск:");
        ui.text_edit_singleline(&mut ectx.editor.palette_search);
        if ui.button("×").clicked() {
            ectx.editor.palette_search.clear();
        }
    });
}

/// Режим мультивыбора палитры: активен, пока список мульти-выбора
/// непуст (клик по ячейке тогда тогглит, а не выбирает одиночно).
fn palette_multi_mode(editor: &mut EditorUi) -> bool {
    !editor.brush.multi_select.is_empty()
}

/// Тоггл элемента в мульти-выборе: есть — убрать, нет — добавить.
fn palette_toggle_multi(editor: &mut EditorUi, pick: crate::state::MultiPick) {
    if let Some(pos) = editor
        .brush
        .multi_select
        .iter()
        .position(|p| *p == pick)
    {
        editor.brush.multi_select.remove(pos);
    } else {
        editor.brush.multi_select.push(pick);
    }
}

/// Пропускает ли объект поисковый фильтр (имя или id как подстрока).
fn matches_search(search: &str, name: &str, id: usize) -> bool {
    let search = search.trim();
    if search.is_empty() {
        return true;
    }
    // Числовой запрос — точное совпадение id (не подстрока имени).
    if let Ok(num) = search.parse::<usize>() {
        return num == id;
    }
    name.to_lowercase().contains(&search.to_lowercase())
}

/// Категория объекта = первое слово имени (Tree/GreenHills/Church/...):
/// поле category в Objects.ini пустое.
fn object_category(obj: &dt_lib::map::object::ObjectInfo) -> String {
    obj.name
        .split(|c: char| c.is_ascii_digit())
        .next()
        .unwrap_or("")
        .trim()
        .to_string()
}

/// Грид тайлов: иконки TILES (спрайты уже в gfx-ассетах assets/Terrain).
/// Иконки предзагружаются в palette_tex ДО отрисовки грида (п.5 ТЗ):
/// битый лейаут при ленивой загрузке исключён, иконка — aspect-fit
/// по ячейке.
fn tiles_palette(ui: &mut Ui, ectx: &mut EditorCtx) {
    search_field(ui, ectx);
    // Предзагрузка всех спрайтов TILES (16 штук): первый кадр — один
    // синхронный проход по диску, дальше — кэш palette_tex.
    let ctx = ui.ctx().clone();
    for tile in 0..editor_core::project::TILE_COUNT {
        let sprite = dt_lib::map::tile::TILES[tile].sprite();
        if !ectx.editor.palette_tex.contains_key(sprite) {
            palette_icon_cache(
                &ctx,
                ectx.editor,
                sprite,
                &format!("{ASSETS_TERRAIN}/{sprite}"),
            );
        }
    }
    egui::ScrollArea::vertical()
        .id_salt("tiles_palette")
        .auto_shrink([false, false])
        .show(ui, |ui| {
            let names: Vec<String> = dt_lib::map::tile::TILES
                .iter()
                .map(|t| t.sprite().trim_end_matches(".png").to_string())
                .collect();
            ui.horizontal_wrapped(|ui| {
                for tile in 0..editor_core::project::TILE_COUNT {
                    if !matches_search(
                        &ectx.editor.palette_search,
                        &names[tile],
                        tile,
                    ) {
                        continue;
                    }
                    let selected = ectx.editor.active_tile == tile;
                    let sprite = dt_lib::map::tile::TILES[tile].sprite();
                    let cell_size = egui::vec2(72., 80.);
                    let (cell, icon_rect) = palette_cell(ui, cell_size);
                    // Иконка по ячейке (aspect-fit, верх ячейки).
                    if let Some(tex) = ectx.editor.palette_tex.get(sprite) {
                        draw_icon_fit(ui.painter(), tex, icon_rect);
                    }
                    // Подпись: фиксированная высота 30, усечение — ряды
                    // ячеек ровные при любых именах (лесенка исключена).
                    ui.put(
                        egui::Rect::from_min_size(
                            cell.rect.left_bottom() - egui::vec2(0., 30.),
                            egui::vec2(cell_size.x, 30.),
                        ),
                        egui::Label::new(
                            RichText::new(format!("{}\n{}", names[tile], tile))
                                .color(if selected {
                                    Color32::YELLOW
                                } else {
                                    Color32::LIGHT_GRAY
                                })
                                .small(),
                        )
                        .truncate(),
                    );
                    let cell = cell.on_hover_text(format!("{} ({})", names[tile], tile));
                    // Единый клик-таргет: мультивыбор включён — клик
                    // тогглит элемент в списке, иначе — одиночный выбор.
                    if cell.clicked() {
                        if palette_multi_mode(&mut ectx.editor) {
                            palette_toggle_multi(&mut ectx.editor, MultiPick::Tile(tile));
                        } else {
                            ectx.editor.active_tile = tile;
                            ectx.editor.state.set_active_tile(tile);
                        }
                    }
                    if selected {
                        ui.painter().rect_stroke(
                            cell.rect,
                            3.,
                            Stroke::new(2., Color32::YELLOW),
                            egui::StrokeKind::Inside,
                        );
                    }
                }
            });
        });
}

/// Ячейка палитры фиксированного размера: возвращает реакцию всей ячейки
/// и прямоугольник иконки (верх ячейки, минус подпись).
fn palette_cell(ui: &mut Ui, cell_size: egui::Vec2) -> (egui::Response, egui::Rect) {
    let (rect, resp) = ui.allocate_exact_size(cell_size, Sense::click());
    let icon_rect = egui::Rect::from_min_size(
        rect.left_top() + egui::vec2(4., 4.),
        egui::vec2(cell_size.x - 8., cell_size.y - 34.),
    );
    (resp, icon_rect)
}

/// Рисует текстуру aspect-fit в прямоугольнике (центрирование).
fn draw_icon_fit(painter: &egui::Painter, tex: &egui::TextureHandle, rect: egui::Rect) {
    let [tw, th] = tex.size();
    let (tw, th) = (tw as f32, th as f32);
    if tw <= 0. || th <= 0. {
        return;
    }
    let scale = (rect.width() / tw).min(rect.height() / th);
    let (w, h) = (tw * scale, th * scale);
    let min = rect.center() - egui::vec2(w * 0.5, h * 0.5);
    let icon_rect = egui::Rect::from_min_size(min, egui::vec2(w, h));
    painter.image(
        tex.id(),
        icon_rect,
        egui::Rect::from_min_max(egui::pos2(0., 0.), egui::pos2(1., 1.)),
        Color32::WHITE,
    );
}
/// Грид декораций (buildings=false) или строений (buildings=true):
/// иконки registry.objects по obj_type, поиск + фильтр категории.
fn objects_palette(ui: &mut Ui, ectx: &mut EditorCtx, buildings: bool) {
    search_field(ui, ectx);
    let mut categories: Vec<String> = ectx
        .registry
        .objects
        .inner
        .iter()
        .filter(|o| {
            matches!(
                o.obj_type,
                dt_lib::map::object::ObjectType::Building { .. }
                    | dt_lib::map::object::ObjectType::Bridge { .. }
            ) == buildings
                || (!buildings
                    && matches!(
                        o.obj_type,
                        dt_lib::map::object::ObjectType::MapDeco { .. }
                    ))
        })
        .map(object_category)
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .collect();
    categories.dedup();
    let current = if buildings {
        &mut ectx.editor.building_category
    } else {
        &mut ectx.editor.deco_category
    };
    egui::ComboBox::from_id_salt(if buildings {
        "building_category"
    } else {
        "deco_category"
    })
    .selected_text(current.clone().unwrap_or_else(|| "все".into()))
    .show_ui(ui, |ui| {
        ui.selectable_value(current, None, "все");
        for cat in &categories {
            ui.selectable_value(current, Some(cat.clone()), cat);
        }
    });
    let category = current.clone();
    egui::ScrollArea::vertical()
        .id_salt(if buildings {
            "buildings_palette"
        } else {
            "decos_palette"
        })
        .auto_shrink([false, false])
        .show(ui, |ui| {
            let entries: Vec<(usize, &dt_lib::map::object::ObjectInfo)> = ectx
                .registry
                .objects
                .inner
                .iter()
                .enumerate()
                .filter(|(_, o)| {
                    let is_building = matches!(
                        o.obj_type,
                        dt_lib::map::object::ObjectType::Building { .. }
                            | dt_lib::map::object::ObjectType::Bridge { .. }
                    );
                    is_building == buildings
                        && category
                            .as_ref()
                            .is_none_or(|c| object_category(o) == *c)
                        && matches_search(&ectx.editor.palette_search, &o.name, o.index)
                })
                .collect();
            // Предзагрузка иконок ДО отрисовки (п.5): весь фильтрованный
            // список — один синхронный проход по кэшу; иконка рисуется
            // aspect-fit по фиксированной ячейке.
            let ctx = ui.ctx().clone();
            for (_, obj) in &entries {
                if !ectx.editor.palette_tex.contains_key(&obj.path) {
                    palette_icon_cache(
                        &ctx,
                        ectx.editor,
                        &obj.path,
                        &format!("assets/Objects/{}", obj.path),
                    );
                }
            }
            ui.horizontal_wrapped(|ui| {
                for (idx, obj) in entries {
                    let selected = if buildings {
                        ectx.editor.active_building == Some(idx)
                    } else {
                        ectx.editor.active_deco == Some(idx)
                    };
                    let cell_size = egui::vec2(72., 80.);
                    let (resp, icon_rect) = palette_cell(ui, cell_size);
                    if let Some(tex) = ectx.editor.palette_tex.get(&obj.path) {
                        draw_icon_fit(ui.painter(), tex, icon_rect);
                    }
                    // Подпись: фикс. высота 30, усечение (лесенка исключена).
                    ui.put(
                        egui::Rect::from_min_size(
                            resp.rect.left_bottom() - egui::vec2(0., 30.),
                            egui::vec2(cell_size.x, 30.),
                        ),
                        egui::Label::new(
                            egui::RichText::new(format!("{}\n({})", obj.name, obj.index))
                                .small()
                                .color(if selected {
                                    egui::Color32::YELLOW
                                } else {
                                    egui::Color32::LIGHT_GRAY
                                }),
                        )
                        .truncate(),
                    );
                    let resp = resp.on_hover_text(format!(
                        "{} ({}) {:?}",
                        obj.name, obj.index, obj.size
                    ));
                    // Единый клик-таргет: мультивыбор включён — тоггл,
                    // иначе одиночный выбор инструмента.
                    if resp.clicked() {
                        let pick = if buildings {
                            crate::state::MultiPick::Building(idx)
                        } else {
                            crate::state::MultiPick::Deco(idx)
                        };
                        if palette_multi_mode(&mut ectx.editor) {
                            palette_toggle_multi(&mut ectx.editor, pick);
                        } else if buildings {
                            ectx.editor.active_building = Some(idx);
                        } else {
                            ectx.editor.active_deco = Some(idx);
                        }
                    }
                    if selected {
                        ui.painter().rect_stroke(
                            resp.rect,
                            3.,
                            egui::Stroke::new(2., egui::Color32::YELLOW),
                            egui::StrokeKind::Inside,
                        );
                    }
                }
            });
        });
}

/// Палитра армий: 4 базовых шаблона (феодал/разбойник/деревенщина/нежить)
/// по характерному юниту из реестра + заглушка «свои шаблоны».
fn armies_palette(ui: &mut Ui, ectx: &mut EditorCtx) {
    ui.label("Базовые шаблоны:");
    for nature in crate::state::ArmyNature::ALL {
        let Some(unit_id) = ectx
            .registry
            .units
            .str_to_id(&nature.template_unit_name().to_string())
        else {
            continue;
        };
        let unit = &ectx.registry.units[unit_id];
        let selected = ectx.editor.active_army_template == Some(unit_id);
        let icon_name = format!("unit_{}.png", unit.icon_index - 1);
        ui.horizontal(|ui| {
            if let Some(tex) = palette_icon_cache(
                ui.ctx(),
                ectx.editor,
                &icon_name,
                &format!("assets/Icons/{icon_name}"),
            ) {
                ui.image((tex, egui::vec2(40., 40.)));
            }
            let label = format!("{} — {} ({})", nature.label(), unit.name, unit_id);
            if ui.selectable_label(selected, label).clicked() {
                let pick = crate::state::MultiPick::Army(unit_id);
                if palette_multi_mode(&mut ectx.editor) {
                    palette_toggle_multi(&mut ectx.editor, pick);
                } else {
                    ectx.editor.active_army_template = Some(unit_id);
                }
            }
        });
    }
    ui.separator();
    ui.label("Свои шаблоны:");
    ui.label(RichText::new("(добавьте свои)").weak().small());
}

/// Кэш egui-текстур иконок палитры (ключ = имя ассета; load_texture с
/// диска через egui-пайплайн — premultiplied alpha, как marker_handles).
fn palette_icon_cache(
    ctx: &egui::Context,
    editor: &mut EditorUi,
    asset_name: &str,
    path: &str,
) -> Option<egui::TextureId> {
    if let Some(handle) = editor.palette_tex.get(asset_name) {
        return Some(handle.id());
    }
    let img = image::open(path).ok()?.to_rgba8();
    let (w, h) = img.dimensions();
    let image = egui::ColorImage::from_rgba_unmultiplied([w as usize, h as usize], img.as_raw());
    let handle = ctx.load_texture(asset_name, image, egui::TextureOptions::NEAREST);
    let id = handle.id();
    editor.palette_tex.insert(asset_name.to_string(), handle);
    Some(id)
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
    // Слои: тайлы (RT0) и декорации/строения/армий (RT1) — обе нативные
    // egui-текстуры (zero-copy), порядок как в игре: тайлы, затем декор.
    let draw_min = world_to_screen([0., 0.]);
    let draw_max = world_to_screen([world_w, world_h]);
    let draw_rect = egui::Rect::from_min_max(draw_min, draw_max);
    let uv = egui::Rect::from_min_max(egui::pos2(0., 0.), egui::pos2(1., 1.));
    painter.image(tex_id, draw_rect, uv, Color32::WHITE);
    if let Some(decos_tex) = ectx.editor.decos_tex {
        painter.image(decos_tex, draw_rect, uv, Color32::WHITE);
    }
    // Ownership-заливка строений (вкладка «Рендер»): полупрозрачные
    // прямоугольники спанов строений, цвет — HSV-хеш индекса (как F9
    // в игре, building_ownership_color).
    if ectx.editor.render_settings.ownership {
        // Спаны строений через calc_hitboxes (проектный hitmap мог
        // протухнуть после правок; пересчёт на клоне).
        let mut map = ectx.editor.state.project().map.clone();
        map.calc_hitboxes(&ectx.registry.objects.inner);
        let map_size = map.hitmap.size;
        if map_size == size && !map.hitmap.inner.is_empty() {
            let flat_iter = map.hitmap.inner.iter().enumerate();
            for (flat, hit) in flat_iter {
                if let Some(building) = hit.building {
                    // TileMap::index((x, y)) = inner[y + x*size] → по
                    // linear индексу x = flat/size, y = flat%size.
                    let (tx, ty) = (flat / map_size, flat % map_size);
                    let a = world_to_screen([tx as f32 * SIZE.0, ty as f32 * SIZE.1]);
                    let b = world_to_screen([
                        (tx + 1) as f32 * SIZE.0,
                        (ty + 1) as f32 * SIZE.1,
                    ]);
                    let [r, g, bch, _] = building_ownership_color(building);
                    let color = Color32::from_rgb(
                        (r * 255.) as u8,
                        (g * 255.) as u8,
                        (bch * 255.) as u8,
                    );
                    painter.rect_filled(
                        egui::Rect::from_min_max(a, b),
                        0.,
                        color.gamma_multiply(0.35),
                    );
                }
            }
        }
    }
    // Маркеры редактора (E1 армии, E2/E4 фонарики) поверх слоёв.
    if ectx.editor.render_settings.markers {
        let markers = ectx.editor.marker_handles.clone();
        markers_layer(ui, ectx, world_to_screen, &markers);
    }
    // Сетка (вкладка «Рендер»).
    let grid_step = 8.0 / zoom;
    if zoom > 1.0 && ectx.editor.render_settings.grid {
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
    // Ховер тайла + превью фигуры кисти (в рисовании, размер > 1).
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
            // Превью фигуры: полупрозрачная заливка клеток фигуры кисти.
            let painting_tool = ectx.editor.tool.is_paint();
            if painting_tool && ectx.editor.brush.size > 1 {
                for (dx, dy) in brush_cells(ectx.editor.brush.shape, ectx.editor.brush.size)
                {
                    let x = tile.0 as i64 + dx as i64;
                    let y = tile.1 as i64 + dy as i64;
                    if x < 0 || y < 0 || x as usize >= size || y as usize >= size {
                        continue;
                    }
                    let a = world_to_screen([
                        x as f32 * SIZE.0,
                        y as f32 * SIZE.1,
                    ]);
                    let b = world_to_screen([
                        (x + 1) as f32 * SIZE.0,
                        (y + 1) as f32 * SIZE.1,
                    ]);
                    painter.rect_filled(
                        egui::Rect::from_min_max(a, b),
                        0.,
                        Color32::from_rgba_unmultiplied(255, 255, 0, 40),
                    );
                }
            }
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
        // Реверт к оригиналу (f96705f): zoom-фактор = cam.zoom[0]*world_w*0.5,
        // кламп в ЕДИНИЦААХ cam.zoom[0]: min = fit/world_w*0.5 (карта вдвое
        // меньше канваса — «подход»), max = 4.0 (карта ×4 — детальный вид).
        cam.zoom[0] = (cam.zoom[0] * factor).clamp(fit / world_w * 0.5, 4.0);
        cam.zoom[1] = cam.zoom[0];
        cam.target = [
            anchor_world[0] + (cam.target[0] - anchor_world[0]) / factor,
            anchor_world[1] + (cam.target[1] - anchor_world[1]) / factor,
        ];
    }
    // ---------------- Hit-test объектов интеракта ----------------
    // Хелперы — чистая геометрия + поиск по проекту на месте вызова
    // (fn, не замыкания: ectx нужен мутабельно позже в кадре).
    fn object_at(
        ectx: &EditorCtx,
        cx: usize,
        cy: usize,
    ) -> Option<crate::state::Selection> {
        let project = ectx.editor.state.project();
        if let Some(i) = project
            .lanterns
            .iter()
            .position(|l| l.x == cx && l.y == cy)
        {
            return Some(crate::state::Selection::Lantern(i));
        }
        if let Some(i) = project
            .map
            .armys
            .iter()
            .position(|a| a.pos.0 == cx && a.pos.1 == cy)
        {
            return Some(crate::state::Selection::Army(i));
        }
        // Строение: клетка якоря или спан (якорь — правый-нижний угол,
        // спан влево-вверх; максимальный спан 8×7 — запас 8).
        if let Some(i) = project.map.buildings.iter().position(|b| {
            (b.pos.0 == cx && b.pos.1 == cy)
                || (b.pos.0 >= cx && b.pos.0 < cx + 8 && b.pos.1 >= cy && b.pos.1 < cy + 8)
        }) {
            return Some(crate::state::Selection::Building(i));
        }
        None
    }
    let cell_at = |screen: egui::Pos2| -> Option<(usize, usize)> {
        let w = screen_to_world(screen);
        let tx = (w[0] / SIZE.0).floor();
        let ty = (w[1] / SIZE.1).floor();
        (tx >= 0. && ty >= 0. && (tx as usize) < size && (ty as usize) < size)
            .then(|| (tx as usize, ty as usize))
    };
    let interact = ectx.editor.tool == editor_core::Tool::Interact;

    // Esc — отмена переноса (клик-клик).
    if ectx.editor.carrying.is_some() && ectx.input.key_pressed(KeyCode::Escape) {
        ectx.editor.carrying = None;
        ectx.editor.status = "Перенос отменён".into();
    }

    // ЛКМ: интеракт — выбор объекта (рамка + инфоокно). Инструмент не
    // применяется по клику на объект.
    let mut object_clicked = false;
    if response.clicked_by(egui::PointerButton::Primary) {
        if let Some(pos) = response.interact_pointer_pos() {
            let cell = cell_at(pos);
            let sel = cell.and_then(|(cx, cy)| object_at(ectx, cx, cy));
            if interact {
                if let Some(sel) = sel {
                    ectx.editor.selection = Some(sel);
                    object_clicked = true;
                } else {
                    ectx.editor.selection = None;
                }
            }
        }
    }

    // ПКМ down: интеракт — захват объекта (драг ИЛИ клик-клик carrying).
    if response.drag_started_by(egui::PointerButton::Secondary) && interact {
        if let Some(pos) = response.interact_pointer_pos() {
            let sel = cell_at(pos).and_then(|(cx, cy)| object_at(ectx, cx, cy));
            if let Some(sel) = sel {
                let from = match sel {
                    crate::state::Selection::Lantern(i) => {
                        let l = &ectx.editor.state.project().lanterns[i];
                        (l.x, l.y)
                    }
                    crate::state::Selection::Army(i) => {
                        ectx.editor.state.project().map.armys[i].pos
                    }
                    crate::state::Selection::Event(_) => (0, 0),
                    crate::state::Selection::Building(i) => {
                        ectx.editor.state.project().map.buildings[i].pos
                    }
                };
                ectx.editor.carrying = Some(crate::state::CarriedObject {
                    kind: sel,
                    from,
                });
            }
        }
    }

    // Фиксация переноса (клик-клик): ПКМ down при активном carrying.
    // Драг при зажатой ПКМ тоже доезжает сюда через drag_stopped ниже.
    if let Some(carried) = ectx.editor.carrying {
        let fix = |ectx: &mut EditorCtx, to: (usize, usize)| {
            let from = carried.from;
            let editor = &mut ectx.editor;
            let command: Box<dyn editor_core::Command> = match carried.kind {
                crate::state::Selection::Lantern(i) => {
                    Box::new(editor_core::command::MoveLantern::new(i, from, to))
                }
                crate::state::Selection::Event(_) => return,
                crate::state::Selection::Building(i) => {
                    Box::new(editor_core::command::MoveBuilding::new(i, from, to))
                }
                crate::state::Selection::Army(i) => {
                    Box::new(editor_core::command::MoveArmy::new(i, from, to))
                }
            };
            editor.carrying = None;
            match editor.history.execute(command, &mut editor.state) {
                CommandResult::Applied => {
                    editor.status = format!("Перенос: {:?} → {:?}", from, to);
                    editor.bake_dirty = true;
                }
                CommandResult::Noop => {}
            }
        };
        // Клик-клик: ПКМ нажат в новой клетке (не по тому же объекту).
        if response.drag_started_by(egui::PointerButton::Secondary) {
            if let Some(pos) = response.interact_pointer_pos() {
                if let Some(to) = cell_at(pos) {
                    if to != carried.from {
                        fix(ectx, to);
                    }
                }
            }
        }
        // Обычный драг: отпустили ПКМ в новой клетке.
        if response.drag_stopped_by(egui::PointerButton::Secondary) {
            if let Some(pos) = response.interact_pointer_pos() {
                if let Some(to) = cell_at(pos) {
                    fix(ectx, to);
                }
            }
        }
    }

    // Подсветка выделения интеракта (рамка на канвасе).
    if let Some(sel) = ectx.editor.selection {
        let rect_of = |(x, y): (usize, usize), up: f32| -> egui::Rect {
            let min = world_to_screen([x as f32 * SIZE.0, (y as f32 - up) * SIZE.1]);
            let max = world_to_screen([(x + 1) as f32 * SIZE.0, (y + 1) as f32 * SIZE.1]);
            egui::Rect::from_min_max(min, max)
        };
        let (rect, label) = match sel {
            crate::state::Selection::Lantern(i) => {
                let l = &ectx.editor.state.project().lanterns[i];
                (rect_of((l.x, l.y), 1.), format!("Точка #{}", l.id))
            }
            crate::state::Selection::Army(i) => {
                let a = &ectx.editor.state.project().map.armys[i];
                (rect_of(a.pos, 0.), format!("Армия «{}»", a.stats.army_name))
            }
            crate::state::Selection::Event(i) => {
                // Событие не имеет клетки: рамки нет.
                let _ = i;
                (egui::Rect::NOTHING, String::new())
            }
            crate::state::Selection::Building(i) => {
                let b = &ectx.editor.state.project().map.buildings[i];
                (rect_of(b.pos, 0.), format!("Строение #{}", b.id))
            }
        };
        painter.rect_stroke(rect, 2., Stroke::new(2.5, Color32::LIGHT_GREEN), egui::StrokeKind::Inside);
        painter.debug_text(
            rect.left_top() + egui::vec2(2., -14.),
            egui::Align2::LEFT_BOTTOM,
            Color32::LIGHT_GREEN,
            label,
        );
    }

    // Ghost переносимого объекта (реалтайм, клик-клик И драг): текстура
    // объекта (моделька армии / спрайт строения / маркер точки) в клетке
    // курсора + рамка. Рисуется по ПОСЛЕДНЕЙ известной клетке курсора
    // каждый кадр — без прыжка при фиксации (команда Move* ставит объект
    // ровно в ту же клетку).
    if ectx.editor.carrying.is_some() {
        if let Some(pos) = response.hover_pos().or(response.interact_pointer_pos()) {
            if let Some(cell) = cell_at(pos) {
                let rect = egui::Rect::from_min_max(
                    world_to_screen([cell.0 as f32 * SIZE.0, cell.1 as f32 * SIZE.1]),
                    world_to_screen([
                        (cell.0 + 1) as f32 * SIZE.0,
                        (cell.1 + 1) as f32 * SIZE.1,
                    ]),
                );
                // Текстура ghost: тот же спрайт, что рисует запечка.
                // palette_tex-кэш (egui, с диска); не готова — рамка одна.
                if let Some(tex) = carried_texture(ui, ectx) {
                    let tex_size = tex.size_vec2();
                    // Армии и точки — 2 клетки в высоту (низ на нижней
                    // грани), строения — во всю клетку-якорь.
                    let h = match ectx.editor.carrying {
                        Some(crate::state::CarriedObject {
                            kind: crate::state::Selection::Building(_),
                            ..
                        }) => SIZE.1,
                        _ => SIZE.1 * 2.,
                    };
                    let w = if tex_size.y > 0. {
                        tex_size.x * (h / tex_size.y)
                    } else {
                        SIZE.0
                    };
                    let min_world = [
                        cell.0 as f32 * SIZE.0 + SIZE.0 * 0.5 - w * 0.5,
                        (cell.1 + 1) as f32 * SIZE.1 - h,
                    ];
                    let ghost_rect = egui::Rect::from_min_max(
                        world_to_screen(min_world),
                        world_to_screen([
                            min_world[0] + w,
                            min_world[1] + h,
                        ]),
                    );
                    painter.image(
                        tex.id(),
                        ghost_rect,
                        egui::Rect::from_min_max(egui::pos2(0., 0.), egui::pos2(1., 1.)),
                        Color32::from_rgba_unmultiplied(255, 255, 255, 180),
                    );
                }
                painter.rect_stroke(
                    rect,
                    2.,
                    Stroke::new(2., Color32::LIGHT_YELLOW),
                    egui::StrokeKind::Inside,
                );
            }
        }
    }

    // Пан ПКМ-драгом: отменён при драге точки и переносе объектов.
    if response.dragged_by(egui::PointerButton::Secondary)
        && ectx.editor.lantern_drag.is_none()
        && !interact
    {
        let delta = response.drag_delta();
        cam.target[0] -= delta.x / zoom;
        cam.target[1] -= delta.y / zoom;
    }
    // В интеракте пан по ПКМ остаётся на пустом месте: если кнопка
    // зажата дольше одного клика и объекта под курсором нет.
    if response.dragged_by(egui::PointerButton::Secondary)
        && interact
        && ectx.editor.carrying.is_none()
        && ectx.editor.lantern_drag.is_none()
        && response
            .interact_pointer_pos()
            .and_then(cell_at)
            .and_then(|(cx, cy)| object_at(ectx, cx, cy))
            .is_none()
    {
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
    let painting = (response.clicked() || response.dragged_by(egui::PointerButton::Primary))
        && !object_clicked
        && ectx.editor.tool != editor_core::Tool::Interact;
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
    ectx.editor.cam = cam;
    clicked
}
/// egui-текстура переносимого объекта (ghost): тот же спрайт, что рисует
/// запечка — моделька армии (Unit_40), спрайт строения (registry.objects
/// по id), маркер точки (E2/E3/E4 из marker_handles). None — спрайт ещё
/// не в кэше (ghost рисуется только рамкой).
fn carried_texture(ui: &mut Ui, ectx: &mut EditorCtx) -> Option<egui::TextureHandle> {
    let carried = ectx.editor.carrying?;
    let asset: String = match carried.kind {
        crate::state::Selection::Event(_) => return None,
        crate::state::Selection::Lantern(i) => {
            let l = ectx.editor.state.project().lanterns.get(i)?;
            let marker = if !l.events.is_empty() {
                if l.light_radius > 0 {
                    "E4.png"
                } else {
                    "E3.png"
                }
            } else {
                "E2.png"
            };
            return ectx.editor.marker_handles.get(marker).cloned();
        }
        crate::state::Selection::Army(i) => {
            let army = ectx.editor.state.project().map.armys.get(i)?;
            // Моделька по типу первого юнита — как в bake (статичный кадр 40).
            let unit_type = army
                .troops
                .first()
                .map(|tr| tr.get().unit.get_info(&ectx.registry.units).unit_type)
                .unwrap_or(dt_lib::units::unit::UnitType::People);
            let model = match (&army.control, unit_type) {
                (dt_lib::battle::control::Control::Player(_), _) => "ГГследопыт",
                (_, dt_lib::units::unit::UnitType::Undead) => "мертвяк",
                (_, dt_lib::units::unit::UnitType::Rogue) => "разбойник",
                (_, dt_lib::units::unit::UnitType::Hero
                | dt_lib::units::unit::UnitType::People) => "феодал",
                _ => "некромант",
            };
            format!("{model}/Unit_40.png")
        }
        crate::state::Selection::Building(i) => {
            let b = ectx.editor.state.project().map.buildings.get(i)?;
            let obj = ectx
                .registry
                .objects
                .inner
                .iter()
                .find(|o| o.index == b.id)?;
            obj.path.clone()
        }
    };
    let path = format!("assets/{asset}");
    let ctx = ui.ctx().clone();
    palette_icon_cache(&ctx, ectx.editor, &asset, &path)?;
    ectx.editor.palette_tex.get(&asset).cloned()
}

/// Слой редакторских маркеров поверх канваса (egui painter):
/// E1 — неактивные армии (active=false), активные рисуются модельками
/// в RT-слое; точки событий/фонарики — единая структура MapLantern
/// (GameMap.lanterns): обычный фонарик → E2; фонарик с событиями
/// (events непусты) → E4; точка событий без подсветки (light_radius==0
/// и events непусты) → E3. Один маркер на точку.
///
/// Геометрия: маркер масштабируется до высоты в 2 клетки (ширина
/// пропорционально аспекту текстуры), низ — на нижней грани клетки,
/// X — по центру клетки (как строения/армии в bake):
///   h = SIZE.1 * 2; w = tex_w * (h / tex_h)
///   min = [ i*SIZE.0 + SIZE.0/2 - w/2 , (j+1)*SIZE.1 - h ]
/// Оба угла через world_to_screen: rect строится из world-min и
/// world-max, масштабируется зумом вместе с картой.
fn markers_layer(
    ui: &mut Ui,
    ectx: &mut EditorCtx,
    world_to_screen: impl Fn([f32; 2]) -> egui::Pos2,
    markers: &std::collections::HashMap<String, egui::TextureHandle>,
) {
    let Some(e1) = markers.get("E1.png") else { return; };
    let e2 = markers.get("E2.png");
    let e3 = markers.get("E3.png");
    let e4 = markers.get("E4.png");
    let project = ectx.editor.state.project();
    let painter = ui.painter();
    let size = project.size();

    // Прямоугольник маркера 1×2 клетки: оба угла в world → screen.
    let marker_rect = |i: usize, j: usize, tex: &egui::TextureHandle| -> egui::Rect {
        let tex_size = tex.size_vec2();
        let (tw, th) = (tex_size.x, tex_size.y);
        let h = SIZE.1;
        let w = if th > 0. { tw * (h / th) } else { SIZE.0 };
        let min_world = [
            i as f32 * SIZE.0 + SIZE.0 * 0.5 - w * 0.5,
            (j + 1) as f32 * SIZE.1 - h,
        ];
        let max_world = [min_world[0] + w, min_world[1] + h];
        egui::Rect::from_min_max(world_to_screen(min_world), world_to_screen(max_world))
    };
    let uv = egui::Rect::from_min_max(egui::pos2(0., 0.), egui::pos2(1., 1.));

    // E1: неактивные армии (активные — игровой моделькой в RT-слое).
    for army in &project.map.armys {
        if army.active {
            continue;
        }
        let (i, j) = army.pos;
        if i >= size || j >= size {
            continue;
        }
        painter.image(e1.id(), marker_rect(i, j, e1), uv, Color32::WHITE);
    }

    // Точки событий/фонарики (MapLantern): один маркер на точку.
    // При активном ПКМ-драге маркер рисуется в ТЕКУЩЕЙ клетке курсора
    // (editor.lantern_drag), а не в клетке проекта — до завершения
    // драга проект не мутируется (мутация — командой MoveLantern).
    let drag = ectx.editor.lantern_drag;
    for (i, lantern) in project.map.lanterns.iter().enumerate() {
        let (lx, ly) = match drag {
            Some((di, _, cur)) if di == i => cur,
            _ => (lantern.x, lantern.y),
        };
        if lx >= size || ly >= size {
            continue;
        }
        // E4 — фонарик с событиями; E3 — точка событий без подсветки;
        // E2 — обычный фонарик (без событий).
        let tex = if !lantern.events.is_empty() {
            if lantern.light_radius > 0 {
                e4
            } else {
                e3
            }
        } else {
            e2
        };
        let Some(tex) = tex else { continue; };
        let dragging = matches!(drag, Some((di, _, _)) if di == i);
        painter.image(
            tex.id(),
            marker_rect(lx, ly, tex),
            uv,
            if dragging { Color32::LIGHT_YELLOW } else { Color32::WHITE },
        );
    }
}

/// Детерминированный цвет строения для ownership-слоя (как F9 в игре,
/// map_view::building_ownership_color): HSV-хеш индекса, золотое сечение.
fn building_ownership_color(building: usize) -> [f32; 4] {
    let hue = (building as f32 * 0.618_034) % 1.;
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


/// Клетки фигуры кисти с центром в (0,0) (смещения относительно центра).
/// Круг — диск dx² + dy² ≤ r²; кольцо — диск r минус диск r-2 (толщина
/// 2 при чётном, 1 при нечётном); квадрат — (2r-1)²; периметр — рамка
/// толщиной 1.
pub fn brush_cells(shape: crate::state::BrushShape, r: u32) -> Vec<(i32, i32)> {
    let r = r.max(1) as i32;
    let mut cells = Vec::new();
    let square_side = 2 * r - 1;
    match shape {
        crate::state::BrushShape::Circle => {
            let rr = r * r;
            for dy in -(r - 1)..=(r - 1) {
                for dx in -(r - 1)..=(r - 1) {
                    if dx * dx + dy * dy <= rr {
                        cells.push((dx, dy));
                    }
                }
            }
        }
        crate::state::BrushShape::Ring => {
            let outer = r * r;
            let inner = (r - 2).max(0).pow(2);
            for dy in -(r - 1)..=(r - 1) {
                for dx in -(r - 1)..=(r - 1) {
                    let d2 = dx * dx + dy * dy;
                    if d2 <= outer && d2 > inner {
                        cells.push((dx, dy));
                    }
                }
            }
        }
        crate::state::BrushShape::Square => {
            for dy in -(r - 1)..=(r - 1) {
                for dx in -(r - 1)..=(r - 1) {
                    cells.push((dx, dy));
                }
            }
        }
        crate::state::BrushShape::Perimeter => {
            if r == 1 {
                cells.push((0, 0));
            } else {
                let h = square_side / 2;
                for dy in -h..=h {
                    for dx in -h..=h {
                        if dx == -h || dx == h || dy == -h || dy == h {
                            cells.push((dx, dy));
                        }
                    }
                }
            }
        }
    }
    cells
}

/// Заливка (flood-fill) от стартовой клетки: 4-связность, совпадение
/// исходного тайла. `range` — 0 = вся связная область, иначе BFS-дистанция
/// ≤ range. `max_volume` — 0 = без лимита, иначе максимум клеток.
fn flood_fill(
    project: &editor_core::MapProject,
    start: (usize, usize),
    range: usize,
    max_volume: usize,
) -> Vec<(usize, usize)> {
    let size = project.size();
    let Some(source) = project.tile(start) else {
        return Vec::new();
    };
    let mut visited = vec![false; size * size];
    let mut result = Vec::new();
    let mut queue = std::collections::VecDeque::new();
    visited[start.1 * size + start.0] = true;
    queue.push_back((start, 0usize));
    while let Some(((x, y), dist)) = queue.pop_front() {
        if project.tile((x, y)) != Some(source) {
            continue;
        }
        result.push((x, y));
        if max_volume > 0 && result.len() >= max_volume {
            break;
        }
        if range > 0 && dist >= range {
            continue;
        }
        for (nx, ny) in [
            (x.wrapping_sub(1), y),
            (x + 1, y),
            (x, y.wrapping_sub(1)),
            (x, y + 1),
        ] {
            if nx < size && ny < size {
                let flat = ny * size + nx;
                if !visited[flat] {
                    visited[flat] = true;
                    queue.push_back(((nx, ny), dist + 1));
                }
            }
        }
    }
    result
}

/// Тайл для клетки при мульти-выборе: Random — детерминированный hash
/// координат (одна клетка всегда даёт один и тот же элемент), Sequential —
/// по кругу (порядок выбора в списке). None — мульти-выбор не активен.
fn multi_pick_tile(
    editor: &EditorUi,
    cell: (usize, usize),
    seq_index: usize,
) -> Option<usize> {
    let picks = &editor.brush.multi_select;
    if picks.is_empty() {
        return None;
    }
    let pick = match editor.brush.multi_order {
        crate::state::MultiOrder::Random => {
            let mut h = (cell.0 as u64)
                .wrapping_mul(0x9E37_79B9_7F4A_7C15)
                .wrapping_add((cell.1 as u64).rotate_left(32))
                .wrapping_add(editor.brush.multi_seed_salt);
            h ^= h >> 33;
            h = h.wrapping_mul(0xFF51_AFD7_ED55_8CCD);
            h ^= h >> 33;
            picks[(h % picks.len() as u64) as usize]
        }
        crate::state::MultiOrder::Sequential => picks[seq_index % picks.len()],
    };
    match pick {
        crate::state::MultiPick::Tile(t) => Some(t),
        _ => None,
    }
}

/// Применить активный инструмент: построить команду и выполнить.
fn apply_tool(ectx: &mut EditorCtx, tile: (usize, usize)) {
    let editor = &mut ectx.editor;
    // Мульти-выбор активен: элементы списка применяются к клеткам фигуры
    // (BatchPlace — одна undo-запись).
    if !editor.brush.multi_select.is_empty()
        && matches!(
            editor.tool,
            editor_core::Tool::Brush | editor_core::Tool::BucketFill
        )
    {
        let cells = if editor.tool == editor_core::Tool::BucketFill {
            flood_fill(
                editor.state.project(),
                tile,
                editor.brush.fill_max_range,
                editor.brush.fill_max_volume,
            )
        } else {
            let size = editor.state.project().size();
            brush_cells(editor.brush.shape, editor.brush.size)
                .into_iter()
                .filter_map(|(dx, dy)| {
                    let x = tile.0 as i64 + dx as i64;
                    let y = tile.1 as i64 + dy as i64;
                    (x >= 0 && y >= 0 && (x as usize) < size && (y as usize) < size)
                        .then_some((x as usize, y as usize))
                })
                .collect::<Vec<_>>()
        };
        let edits: Vec<(usize, usize, usize)> = cells
            .into_iter()
            .enumerate()
            .filter_map(|(k, cell)| {
                multi_pick_tile(editor, cell, k).map(|t| (cell.0, cell.1, t))
            })
            .collect();
        if edits.is_empty() {
            editor.status = "Мульти-выбор не содержит тайлов".into();
            return;
        }
        let count = edits.len();
        match editor
            .history
            .execute(Box::new(BatchPlace::new(edits)), &mut editor.state)
        {
            CommandResult::Applied => {
                editor.status = format!("Мульти-кисть: {} клеток", count);
                editor.bake_dirty = true;
            }
            CommandResult::Noop => {}
        }
        return;
    }
    let command: Option<Box<dyn editor_core::Command>> = match editor.tool {
        editor_core::Tool::Brush => {
            // Фигура кисти под курсором: клетки фигуры через BatchPlace.
            if editor.brush.size <= 1 && editor.brush.shape == crate::state::BrushShape::Square {
                Some(Box::new(PaintTile::new(tile, editor.active_tile)))
            } else {
                let size = editor.state.project().size();
                let edits: Vec<(usize, usize, usize)> =
                    brush_cells(editor.brush.shape, editor.brush.size)
                        .into_iter()
                        .filter_map(|(dx, dy)| {
                            let x = tile.0 as i64 + dx as i64;
                            let y = tile.1 as i64 + dy as i64;
                            (x >= 0 && y >= 0 && (x as usize) < size && (y as usize) < size)
                                .then_some((x as usize, y as usize, editor.active_tile))
                        })
                        .collect();
                Some(Box::new(BatchPlace::new(edits)))
            }
        }
        editor_core::Tool::BucketFill => {
            // Flood-fill по совпадению тайла (4-связность), дальность и
            // объём — из настроек кисти (0 = ∞).
            let cells = flood_fill(
                editor.state.project(),
                tile,
                editor.brush.fill_max_range,
                editor.brush.fill_max_volume,
            );
            let edits: Vec<(usize, usize, usize)> = cells
                .into_iter()
                .map(|(x, y)| (x, y, editor.active_tile))
                .collect();
            Some(Box::new(BatchPlace::new(edits)))
        }
        editor_core::Tool::Deco => editor.active_deco.and_then(|idx| {
            ectx.registry
                .objects
                .inner
                .get(idx)
                .map(|obj| match obj.obj_type {
                    dt_lib::map::object::ObjectType::MapDeco { id } => (id, object_category(obj)),
                    _ => (0, String::new()),
                })
        })
        .map(|(deco_id, category)| {
            // Одна декорация на категорию в клетке: индексы декораций
            // той же категории (первое слово имени) для замены.
            let same_category: Vec<usize> = ectx
                .registry
                .objects
                .inner
                .iter()
                .filter(|o| {
                    matches!(o.obj_type, dt_lib::map::object::ObjectType::MapDeco { .. })
                        && object_category(o) == category
                })
                .filter_map(|o| match o.obj_type {
                    dt_lib::map::object::ObjectType::MapDeco { id } => Some(id),
                    _ => None,
                })
                .collect();
            Box::new(PlaceDeco::new(tile, deco_id).with_category(same_category))
                as Box<dyn editor_core::Command>
        }),
        editor_core::Tool::Building => editor
            .active_building
            .and_then(|idx| ectx.registry.objects.inner.get(idx))
            .map(|obj| {
                Box::new(PlaceBuilding::new(tile, obj.index))
                    as Box<dyn editor_core::Command>
            }),
        editor_core::Tool::Army => {
            // Шаблон: выбранный юнит палитры (или пустая армия-заглушка).
            let template = editor.active_army_template;
            let units = template.map(|u| vec![u]).unwrap_or_default();
            editor.counter += 1;
            let name = template
                .map(|u| {
                    format!("{} {}", ectx.registry.units[u].name, editor.counter)
                })
                .unwrap_or_else(|| format!("Армия {}", editor.counter));
            Some(Box::new(PlaceArmy::with_units(
                tile,
                name,
                units,
                ectx.registry,
            )))
        }
        editor_core::Tool::Interact => None,
    };
    // Вкладка «События»: выбранный тип точки (active_lantern_kind)
    // перекрывает инструмент — ЛКМ ставит фонарик/точку событий
    // (map_model 8/9, радиус 3 — новую точку видно на карте).
    let command = match (command, editor.active_lantern_kind) {
        (_, Some(lamp)) => {
            let (map_model, active) = if lamp { (8, true) } else { (9, false) };
            Some(Box::new(
                editor_core::command::PlaceLantern::new(tile.0, tile.1, map_model, 3, active),
            ) as Box<dyn editor_core::Command>)
        }
        (cmd, None) => cmd,
    };
    let Some(command) = command else {
        editor.status = "Выберите элемент в палитре".into();
        return;
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
    // Размер карты изменился — пересоздать RT.
    editor.rt = None;
    editor.decos_rt = None;
    editor.baked = false;
    editor.status = format!("Новая карта {size}×{size}");
}

/// рассечены), и из диалога.
fn open_project_parts(
    registry: &mut dt_lib::registry::GameInfo,
    editor: &mut EditorUi,
    path: &std::path::Path,
) {
    let result = (|| -> Result<editor_core::MapProject, String> {
        let data = dt_lib::map::convert::parse_dtm_map(path)
            .map_err(|_| "parse_dtm_map: не удалось разобрать файл".to_string())?;
        let (map, events) =
            dt_lib::map::convert::convert_dtm_map(data, registry);
        // Точки событий/фонарики — единый источник map.lanterns
        // (MapLantern), дублирующего Vec в проекте больше нет.
        let lanterns = map.lanterns.clone();
        Ok(editor_core::MapProject {
            map,
            events,
            lanterns,
            ..Default::default()
        })
    })();
    match result {
        Ok(project) => {
            let size = project.size();
            editor.state = editor_core::EditorState::new(project);
            editor.history = editor_core::CommandHistory::new();
            editor.project_path = Some(path.to_path_buf());
            editor.issues.clear();
            editor.bake_dirty = true;
            editor.selected = None;
            editor.rt = None;
            editor.decos_rt = None;
            editor.baked = false;
            editor.cam = Camera::from_display_rect(
                0.,
                SIZE.1 * size as f32,
                SIZE.0 * size as f32,
                -SIZE.1 * size as f32,
            );
            editor.status = "Карта открыта".into();
        }
        Err(err) => editor.status = err,
    }
}

/// Open .dtm: parse_dtm_map → convert_dtm_map (registry уже загружен игрой).
fn open_project(ectx: &mut EditorCtx, path: std::path::PathBuf) {
    open_project_parts(ectx.registry, ectx.editor, &path);
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

struct InfoDockBehavior<'a, 'b> {
    ectx: &'a mut EditorCtx<'b>,
    /// Индексы протухших панелей — снос после прохода дерева.
    pub stale_drop: Vec<crate::state::Selection>,
}

impl<'a, 'b> InfoDockBehavior<'a, 'b> {
    /// Снос накопленных протухших панелей (вызвать ПОСЛЕ tree.ui).
    fn drop_stale(&mut self, tree: &mut egui_tiles::Tree<crate::state::Selection>) {
        for sel in self.stale_drop.drain(..) {
            let id = tree
                .tiles
                .iter()
                .find(|(_, tile)| matches!(tile, egui_tiles::Tile::Pane(p) if *p == sel))
                .map(|(id, _)| *id);
            if let Some(id) = id {
                tree.remove_recursively(id);
            }
        }
    }
}

impl<'a, 'b> egui_tiles::Behavior<crate::state::Selection> for InfoDockBehavior<'a, 'b> {
    fn pane_ui(
        &mut self,
        ui: &mut Ui,
        tile_id: egui_tiles::TileId,
        pane: &mut crate::state::Selection,
    ) -> egui_tiles::UiResponse {
        // Протухший индекс (объект удалён undo/redo) — панель на снос:
        // мутация дерева во время прохода запрещена, чистим после ui().
        if !selection_exists(self.ectx, *pane) {
            self.stale_drop.push(*pane);
            return egui_tiles::UiResponse::None;
        }
        // Pin-галочка: закреплённая панель живёт и без выделения.
        let pinned = self.ectx.editor.pinned.contains(pane);
        let mut pin_now = pinned;
        ui.horizontal(|ui| {
            ui.checkbox(&mut pin_now, "pin");
        });
        if pin_now != pinned {
            if pin_now {
                self.ectx.editor.pinned.push(*pane);
            } else {
                self.ectx.editor.pinned.retain(|p| p != pane);
            }
        }
        ui.separator();
        match *pane {
            crate::state::Selection::Lantern(_) => lantern_info(ui, self.ectx, *pane),
            crate::state::Selection::Army(_) => army_info(ui, self.ectx, *pane),
            crate::state::Selection::Building(_) => building_info(ui, self.ectx, *pane),
            crate::state::Selection::Event(_) => event_info(ui, self.ectx, *pane),
        }
        egui_tiles::UiResponse::None
    }

    fn tab_title_for_pane(&mut self, pane: &crate::state::Selection) -> egui::WidgetText {
        selection_title(self.ectx, *pane).into()
    }

    // Контент инфоокна определяет высоту: без вертикального растяжения.
    fn simplification_options(&self) -> egui_tiles::SimplificationOptions {
        egui_tiles::SimplificationOptions {
            all_panes_must_have_tabs: true,
            ..Default::default()
        }
    }
}

/// Жив ли объект по индексу (после undo/redo индекс мог протухнуть).
fn selection_exists(ectx: &EditorCtx, sel: crate::state::Selection) -> bool {
    let project = ectx.editor.state.project();
    match sel {
        crate::state::Selection::Lantern(i) => i < project.lanterns.len(),
        crate::state::Selection::Army(i) => i < project.map.armys.len(),
        crate::state::Selection::Building(i) => i < project.map.buildings.len(),
        crate::state::Selection::Event(i) => i < project.events.len(),
    }
}

/// Заголовок таба = имя объекта.
fn selection_title(ectx: &EditorCtx, sel: crate::state::Selection) -> String {
    let project = ectx.editor.state.project();
    match sel {
        crate::state::Selection::Lantern(i) => project
            .lanterns
            .get(i)
            .map(|l| format!("Точка #{}", l.id))
            .unwrap_or_else(|| "Точка (удалена)".into()),
        crate::state::Selection::Army(i) => project
            .map
            .armys
            .get(i)
            .map(|a| format!("Армия «{}»", a.stats.army_name))
            .unwrap_or_else(|| "Армия (удалена)".into()),
        crate::state::Selection::Event(i) => project
            .events
            .get(i)
            .map(|e| format!("Событие: {}", e.name))
            .unwrap_or_else(|| "Событие".into()),
        crate::state::Selection::Building(i) => project
            .map
            .buildings
            .get(i)
            .map(|b| format!("Строение #{}", b.id))
            .unwrap_or_else(|| "Строение (удалено)".into()),
    }
}

/// Правая dock-панель: egui_tiles-дерево инфоокон + pin «по умолчанию».
/// Панель рисуется всегда при ненулевом дереве; новый выбор (selection)
/// добавляется в дерево как новая вкладка.
fn info_dock_panel(ui: &mut Ui, ectx: &mut EditorCtx) {
    // Новый выбор → новая вкладка в дереве (дубль не добавляем).
    if let Some(sel) = ectx.editor.selection {
        let tree = ectx.editor.info_tree.get_or_insert_with(|| {
            egui_tiles::Tree::new_tabs("editor_info_dock", vec![])
        });
        let already = tree.tiles.iter().any(|(_, tile)| {
            matches!(tile, egui_tiles::Tile::Pane(p) if *p == sel)
        });
        if !already {
            let pane_id = tree.tiles.insert_pane(sel);
            if let Some(root) = tree.root() {
                tree.move_tile_to_container(pane_id, root, usize::MAX, true);
            } else {
                tree.tiles.set_visible(pane_id, true);
            }
        }
    }
    // Галочка «закреплять по умолчанию» (пин новых выборов автоматически).
    ui.horizontal(|ui| {
        let mut pin = ectx.editor.pin_by_default;
        ui.checkbox(&mut pin, "Закреплять по умолчанию");
        ectx.editor.pin_by_default = pin;
        if ectx.editor.selection.is_some()
            && ui.button("Открепить всё").clicked()
        {
            ectx.editor.pinned.clear();
        }
    });
    ui.separator();
    if let Some(mut tree) = ectx.editor.info_tree.take() {
        let mut behavior = InfoDockBehavior {
            ectx,
            stale_drop: Vec::new(),
        };
        tree.ui(&mut behavior, ui);
        behavior.drop_stale(&mut tree);
        // Возвращаем дерево (включая правки pane_ui) обратно в состояние.
        ectx.editor.info_tree = Some(tree);
    }
}

/// Содержимое инфоокна точки событий: тип, клетка, радиус, чекбокс
/// активна-с-начала, интерактивный список событий (п.6).
fn lantern_info(ui: &mut Ui, ectx: &mut EditorCtx, sel: crate::state::Selection) {
    let crate::state::Selection::Lantern(index) = sel else {
        return;
    };
    let Some(lantern) = ectx.editor.state.project().lanterns.get(index).cloned() else {
        ectx.editor.selection = None;
        return;
    };
    let kind = if !lantern.events.is_empty() {
        if lantern.light_radius > 0 {
            "фонарик с событиями"
        } else {
            "точка событий без подсветки"
        }
    } else if lantern.active_from_start {
        "обычный фонарик (активен с начала)"
    } else {
        "фонарик"
    };
    ui.label(format!("Тип: {kind}"));
    ui.label(format!("Клетка: ({}, {})", lantern.x, lantern.y));
    ui.label(format!("map_model: {} (9 = точка событий)", lantern.map_model));

    // Радиус: DragValue 0..=24 через команду SetLanternRadius.
    let mut radius = lantern.light_radius;
    ui.horizontal(|ui| {
        ui.label("Радиус света:");
        ui.add(egui::DragValue::new(&mut radius).range(0..=24).speed(1.));
    });
    if radius != lantern.light_radius {
        let editor = &mut ectx.editor;
        let command =
            Box::new(editor_core::command::SetLanternRadius::new(index, radius));
        if let CommandResult::Applied = editor.history.execute(command, &mut editor.state) {
            editor.status = format!("Радиус точки #{}: {radius}", lantern.id);
        }
    }

    // Активна с начала: чекбокс (мутация только командой — упрощённо
    // прямой сеттер отсутствует, правка через Remove+Place не атомарна;
    // чекбокс показывает состояние, правка — будущая команда).
    let mut active = lantern.active_from_start;
    ui.add_enabled(false, egui::Checkbox::new(&mut active, "Активна с начала"));

    // ---------------- Список событий точки (п.6) ----------------
    ui.separator();
    ui.label(format!("События ({}):", lantern.events.len()));
    for row in (0..lantern.events.len()).rev() {
        let id = lantern.events[row];
        let name = ectx
            .editor
            .state
            .project()
            .events
            .get(id)
            .map(|e| e.name.clone())
            .unwrap_or_else(|| "?".into());
        ui.horizontal(|ui| {
            ui.label(format!("{}. [{}] {}", row + 1, id, name));
            // goto-definition: открыть инфо-тайл события.
            let mut dock = LanternDock { index };
            let mut cur = Some(id);
            let opts = vec![(id, name.clone())];
            let r = crate::editor_ui::object_ref::ref_selector(
                ui,
                &mut cur,
                &opts,
                &mut dock,
                crate::editor_ui::object_ref::GameRef::Event,
            );
            let _ = r;
            if ui.button("✕").clicked() {
                let editor = &mut ectx.editor;
                let command = Box::new(
                    editor_core::command::RemoveLanternEvent::new(index, row),
                );
                if let CommandResult::Applied =
                    editor.history.execute(command, &mut editor.state)
                {
                    editor.status = format!("Событие [{id}] отвязано от точки");
                }
            }
        });
    }
    // «+ Добавить»: селектор СУЩЕСТВУЮЩИХ событий карты (id + имя)
    // со встроенным goto-definition; выбор — AddLanternEvent (ссылка).
    let events: Vec<(usize, String)> = ectx
        .editor
        .state
        .project()
        .events
        .iter()
        .enumerate()
        .map(|(id, e)| (id, e.name.clone()))
        .collect();
    if events.is_empty() {
        ui.label(
            RichText::new("+ Добавить событие: список пуст — создайте события")
                .weak()
                .small(),
        );
    } else {
        let mut none: Option<usize> = None;
        let mut dock = LanternDock { index };
        let r = crate::editor_ui::object_ref::ref_selector(
            ui,
            &mut none,
            &events,
            &mut dock,
            crate::editor_ui::object_ref::GameRef::Event,
        );
        if let Some(pick) = none {
            let _ = r;
            let editor = &mut ectx.editor;
            let command =
                Box::new(editor_core::command::AddLanternEvent::new(index, pick));
            if let CommandResult::Applied = editor.history.execute(command, &mut editor.state) {
                editor.status = format!("Событие [{pick}] привязано к точке");
            }
        }
    }
}

/// Dock-адаптер точки: «→» в селекторе открывает инфо-тайл события.
struct LanternDock {
    index: usize,
}

impl crate::editor_ui::object_ref::PropertiesDock for LanternDock {
    fn open(&mut self, r: crate::editor_ui::object_ref::GameRef) {
        if let crate::editor_ui::object_ref::GameRef::Event(id) = r {
            crate::editor_view::open_event_info(id);
        }
    }
}

/// Содержимое инфоокна армии: имя, природа (реестр), active, отряды.
fn army_info(ui: &mut Ui, ectx: &mut EditorCtx, sel: crate::state::Selection) {
    let crate::state::Selection::Army(index) = sel else {
        return;
    };
    let Some(army) = ectx.editor.state.project().map.armys.get(index) else {
        ectx.editor.selection = None;
        return;
    };
    ui.label(format!("Имя: {}", army.stats.army_name));
    ui.label(format!("Клетка: {:?}", army.pos));
    let nature = army
        .troops
        .get(0)
        .map(|t| t.get().unit.id)
        .and_then(|id| ectx.registry.units.get(id))
        .map(|u| format!("{} (Nature {:?})", u.name, u.unit_type))
        .unwrap_or_else(|| "нет юнитов".into());
    ui.label(format!("Глава: {nature}"));
    ui.label(format!("Активна: {}", army.active));
    ui.separator();
    ui.label(format!("Отряды ({}):", army.troops.len()));
    for (i, troop) in army.troops.iter().enumerate() {
        let t = troop.get();
        let info = ectx.registry.units.get(t.unit.id);
        let name = info.map(|u| u.name.clone()).unwrap_or_else(|| "?".into());
        ui.label(format!("  {}. {} (hp {})", i + 1, name, t.unit.hp));
    }
}

/// Содержимое инфоокна строения: имя/id из реестра, клетка, владелец.
fn building_info(ui: &mut Ui, ectx: &mut EditorCtx, sel: crate::state::Selection) {
    let crate::state::Selection::Building(index) = sel else {
        return;
    };
    let Some(building) = ectx.editor.state.project().map.buildings.get(index) else {
        ectx.editor.selection = None;
        return;
    };
    // Имя спрайта строения: registry.objects по index (ObjectInfo.index).
    let obj_name = ectx
        .registry
        .objects
        .inner
        .iter()
        .find(|o| o.index == building.id)
        .map(|o| o.name.clone())
        .unwrap_or_else(|| "?".into());
    ui.label(format!("Объект: {obj_name} (id {})", building.id));
    ui.label(format!("Имя: «{}»", building.name));
    ui.label(format!("Клетка: {:?}", building.pos));
    let owner = match building.owner {
        Some(owner) => format!("армия #{owner}"),
        None => "нет".into(),
    };
    ui.label(format!("Владелец: {owner}"));
    ui.label(format!("Тип: {:?}", building.variant));
}

/// Инфо-панель события карты (read-only просмотр, открыта из селектора).
fn event_info(ui: &mut Ui, ectx: &mut EditorCtx, sel: crate::state::Selection) {
    let crate::state::Selection::Event(index) = sel else {
        return;
    };
    let Some(event) = ectx.editor.state.project().events.get(index) else {
        return;
    };
    ui.label(format!("Событие [{}]", index));
    ui.separator();
    ui.label(format!("Имя: {}", event.name));
    ui.label(format!(
        "Игроки: {}",
        if event.player.is_empty() {
            "—".to_string()
        } else {
            event.player.iter().map(|p| p.to_string()).collect::<Vec<_>>().join(", ")
        }
    ));
    ui.label(format!("Локация: {:?}", event.location));
    if let Some(message) = &event.message {
        ui.separator();
        egui::ScrollArea::vertical()
            .id_salt(("event_message", index))
            .max_height(160.)
            .show(ui, |ui| {
                ui.label(message);
            });
    }
}

/// goto-definition: открыть инфо-тайл события (или поднять существующий).
pub fn open_event_info(event_index: usize) {
    // Вставка в дерево через глобальную точку: инфо-дерево живёт в EditorUi,
    // досягаемом только из кадра egui — отложенный выбор через ресурс.
    PENDING_EVENT_OPEN.with(|cell| {
        cell.borrow_mut().push(event_index);
    });
}

/// Отложенные открытия событий (thread_local: вставка возможна только
/// в кадре, когда есть &mut EditorUi).
thread_local! {
    static PENDING_EVENT_OPEN: std::cell::RefCell<Vec<usize>> =
        const { std::cell::RefCell::new(Vec::new()) };
}

/// Применить отложенные открытия событий (вызывать в кадре egui,
/// когда EditorUi доступен).
fn flush_pending_event_opens(ectx: &mut EditorCtx) {
    let pending = PENDING_EVENT_OPEN.with(|cell| cell.borrow_mut().drain(..).collect::<Vec<_>>());
    for index in pending {
        let sel = crate::state::Selection::Event(index);
        if !selection_exists(ectx, sel) {
            continue;
        }
        let tree = ectx.editor.info_tree.get_or_insert_with(|| {
            egui_tiles::Tree::new_tabs("editor_info_dock", vec![])
        });
        let already = tree
            .tiles
            .iter()
            .any(|(_, tile)| matches!(tile, egui_tiles::Tile::Pane(p) if *p == sel));
        if !already {
            let pane_id = tree.tiles.insert_pane(sel);
            if let Some(root) = tree.root() {
                tree.move_tile_to_container(pane_id, root, usize::MAX, true);
            } else {
                tree.tiles.set_visible(pane_id, true);
            }
        }
    }
}
