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

/// Вкладка панели инструментов (палитра по типу элемента + настройки).
#[derive(Clone, Copy, PartialEq, Eq)]
enum PaletteTab {
    /// Палитра активного инструмента (тайлы/декор/строения/армии).
    Tool,
    /// Настройки рендера редактора.
    Render,
}

impl PaletteTab {
    fn label(self) -> &'static str {
        match self {
            PaletteTab::Tool => "Палитра",
            PaletteTab::Render => "Рендер",
        }
    }
}

/// Активная вкладка панели (нетабличное состояние кадра — egui id).
const TAB_STATE: &str = "editor_palette_tab";

/// Панель инструментов слева: инструменты, вкладки палитры (гриды иконок
/// с поиском и фильтрами категорий) и настройки рендера.
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
    // Вкладки: палитра активного инструмента / настройки рендера.
    let mut tab: i32 = ui
        .ctx()
        .data_mut(|d| d.get_temp(egui::Id::new(TAB_STATE)))
        .unwrap_or(0);
    ui.horizontal(|ui| {
        for (i, t) in [PaletteTab::Tool, PaletteTab::Render].iter().enumerate() {
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
        1 => render_settings_tab(ui, ectx),
        _ => palette_tab(ui, ectx),
    }
    ui.separator();
    ui.label(format!(
        "undo: {} redo: {}",
        ectx.editor.history.undo_len(),
        if ectx.editor.history.can_redo() { "+" } else { "-" },
    ));
}

/// Вкладка «Рендер»: переключатели слоёв канваса и режим наплывов.
fn render_settings_tab(ui: &mut Ui, ectx: &mut EditorCtx) {
    let rs = &mut ectx.editor.render_settings;
    let mut rebake = false;
    ui.checkbox(&mut rs.markers, "Маркеры E1/E2/E4");
    ui.checkbox(&mut rs.grid, "Сетка");
    ui.checkbox(&mut rs.ownership, "Принадлежность строений");
    ui.checkbox(&mut rs.buildings, "Строения");
    ui.checkbox(&mut rs.armies, "Армии");
    ui.separator();
    ui.label("Наплывы тайлов:");
    egui::ComboBox::from_id_salt("blend_mode")
        .selected_text(rs.blend_mode.label())
        .show_ui(ui, |ui| {
            for mode in crate::state::BlendMode::ALL {
                ui.selectable_value(&mut rs.blend_mode, mode, mode.label());
            }
        });
    if ui.button("Перезапечь карту").clicked() {
        rebake = true;
    }
    if rebake {
        ectx.editor.bake_dirty = true;
    }
}

/// Вкладка палитры: грид иконок активного инструмента. Тайлы — иконки
/// TILES; декорации/строения — иконки registry.objects (по типу объекта);
/// армии — 4 базовых шаблона по Nature + заглушка «свои шаблоны».
fn palette_tab(ui: &mut Ui, ectx: &mut EditorCtx) {
    match ectx.editor.tool {
        editor_core::Tool::Brush => tiles_palette(ui, ectx),
        editor_core::Tool::Deco => objects_palette(ui, ectx, false),
        editor_core::Tool::Building => objects_palette(ui, ectx, true),
        editor_core::Tool::Army => armies_palette(ui, ectx),
    }
}

/// Подпись поиска над гридом: фильтрует по подстроке имени или id.
fn search_field(ui: &mut Ui, ectx: &mut EditorCtx) {
    ui.horizontal(|ui| {
        ui.label("Поиск:");
        ui.text_edit_singleline(&mut ectx.editor.palette_search);
        if ui.button("✕").clicked() {
            ectx.editor.palette_search.clear();
        }
    });
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
fn tiles_palette(ui: &mut Ui, ectx: &mut EditorCtx) {
    search_field(ui, ectx);
    egui::ScrollArea::vertical()
        .id_salt("tiles_palette")
        .auto_shrink([false, false])
        .show(ui, |ui| {
            let names: Vec<String> = dt_lib::map::tile::TILES
                .iter()
                .map(|t| {
                    t.sprite()
                        .trim_end_matches(".png")
                        .to_string()
                })
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
                    let cell = ui
                        .selectable_label(selected, format!("{}\n{}", names[tile], tile))
                        .on_hover_text(format!("{} ({})", names[tile], tile));
                    if cell.clicked() {
                        ectx.editor.active_tile = tile;
                        ectx.editor.state.set_active_tile(tile);
                    }
                }
            });
        });
}

/// Грид декораций (buildings=false) или строений (buildings=true):
/// иконки registry.objects по obj_type, поиск + фильтр категории.
fn objects_palette(ui: &mut Ui, ectx: &mut EditorCtx, buildings: bool) {
    search_field(ui, ectx);
    // Категории из префиксов имён объектов нужного типа, отсортированные.
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
            // Иконки: кэш на кадр по имени ассета (повторяющиеся объекты
            // одного спрайта не перечитывают PNG).
            let mut icons: std::collections::HashMap<String, Option<egui::TextureId>> =
                std::collections::HashMap::new();
            ui.horizontal_wrapped(|ui| {
                for (idx, obj) in entries {
                    let selected = if buildings {
                        ectx.editor.active_building == Some(idx)
                    } else {
                        ectx.editor.active_deco == Some(idx)
                    };
                    let cell_size = egui::vec2(72., 80.);
                    let path = obj.path.clone();
                    let tex = if let Some(t) = icons.get(&path) {
                        *t
                    } else {
                        let t = palette_icon_cache(
                            ui.ctx(),
                            ectx.editor,
                            &path,
                            &format!("assets/Objects/{path}"),
                        );
                        icons.insert(path, t);
                        t
                    };
                    let cell = egui::Button::new(
                        egui::RichText::new(format!("{}\n({})", obj.name, obj.index))
                            .small()
                            .color(if selected {
                                egui::Color32::YELLOW
                            } else {
                                egui::Color32::LIGHT_GRAY
                            }),
                    )
                    .min_size(cell_size);
                    let resp = ui.add_enabled(tex.is_some(), cell).on_hover_text(format!(
                        "{} ({}) {:?}",
                        obj.name, obj.index, obj.size
                    ));
                    if let Some(tex) = tex {
                        // Иконка в верхней части ячейки, над подписью.
                        let icon_rect = egui::Rect::from_min_size(
                            resp.rect.left_top() + egui::vec2(4., 2.),
                            egui::vec2(64., 44.),
                        );
                        ui.painter().image(
                            tex,
                            icon_rect,
                            egui::Rect::from_min_max(
                                egui::pos2(0., 0.),
                                egui::pos2(1., 1.),
                            ),
                            egui::Color32::WHITE,
                        );
                    }
                    if resp.clicked() {
                        if buildings {
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
                ectx.editor.active_army_template = Some(unit_id);
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
    ectx.editor.cam = cam;
    clicked
}

/// Слой редакторских маркеров поверх канваса (egui painter):
/// E1 — неактивные армии (active=false), активные рисуются модельками
/// в RT-слое; E2 — фонарик без событий; E4 — фонарик с событиями
/// (eventmap той же клетки непуст); E3 — «чисто локальные события»:
/// клетки eventmap с событиями, где НЕТ фонарика.
///
/// Геометрия: спрайты E*.png — 32×32; армия занимает 1×2 клетки
/// (SIZE.0×SIZE.1*2 = 32×44 world-px). Маркер масштабируется до высоты
/// в 2 клетки (ширина пропорционально аспекту), низ — на нижней грани
/// клетки, X — по центру клетки (как строения/армии в bake):
///   h = SIZE.1 * 2; w = tex_w * (h / tex_h)
///   min = [ i*SIZE.0 + SIZE.0/2 - w/2 , (j+1)*SIZE.1 - h ]
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
    // У новой карты (MapProject::new) eventmap пуст (size=0) — индексация
    // по (x,y) невозможна; у открытых .dtm eventmap всегда согласован.
    let eventmap_ok = project.map.eventmap.size == size
        && project.map.eventmap.inner.len() == size * size;
    let events_at = |x: usize, y: usize| -> bool {
        eventmap_ok && !project.map.eventmap[(x, y)].is_empty()
    };
    // Общий расчёт прямоугольника маркера 1×2 клетки, центрированного
    // по X клетки (i, j) и стоящего низом на её нижней грани.
    let marker_rect = |i: usize, j: usize, tex: &egui::TextureHandle| -> egui::Rect {
        let tex_size = tex.size_vec2();
        let (tw, th) = (tex_size.x, tex_size.y);
        let h = SIZE.1 * 2.;
        let w = if th > 0. { tw * (h / th) } else { SIZE.0 };
        let min = world_to_screen([
            i as f32 * SIZE.0 + SIZE.0 * 0.5 - w * 0.5,
            (j + 1) as f32 * SIZE.1 - h,
        ]);
        egui::Rect::from_min_size(min, egui::vec2(w, h))
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

    // E2/E4: фонарики (E4 — если в той же клетке есть события).
    for light in &project.lights {
        if light.x >= size || light.y >= size {
            continue;
        }
        let has_event = events_at(light.x, light.y);
        let tex = if has_event { e4 } else { e2 };
        let Some(tex) = tex else { continue; };
        painter.image(tex.id(), marker_rect(light.x, light.y, tex), uv, Color32::WHITE);
    }

    // E3: «чисто локальные события» — клетки eventmap с событиями, в
    // которых НЕТ фонарика (фонарик с событиями уже показан E4; без
    // событий — E2). Location::Place без координат в dt_lib пока не
    // несёт позицию — единственный координированный источник локальных
    // событий это eventmap (заполняется из lanterns map_model=9).
    if let Some(e3) = e3 {
        if !eventmap_ok {
            return;
        }
        let light_cells: std::collections::HashSet<(usize, usize)> =
            project.lights.iter().map(|l| (l.x, l.y)).collect();
        for flat in 0..size * size {
            if project.map.eventmap.inner[flat].is_empty() {
                continue;
            }
            // TileMap::index((x, y)) = inner[y + x*size] → инверсия:
            // x = flat / size, y = flat % size (как ownership-слой).
            let (x, y) = (flat / size, flat % size);
            if light_cells.contains(&(x, y)) {
                continue;
            }
            painter.image(e3.id(), marker_rect(x, y, e3), uv, Color32::WHITE);
        }
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

/// Применить активный инструмент: построить команду и выполнить.
fn apply_tool(ectx: &mut EditorCtx, tile: (usize, usize)) {
    let editor = &mut ectx.editor;
    let command: Option<Box<dyn editor_core::Command>> = match editor.tool {
        editor_core::Tool::Brush => Some(Box::new(PaintTile::new(tile, editor.active_tile))),
        editor_core::Tool::Deco => editor
            .active_deco
            .and_then(|idx| {
                ectx.registry
                    .objects
                    .inner
                    .get(idx)
                    .map(|obj| match obj.obj_type {
                        dt_lib::map::object::ObjectType::MapDeco { id } => id,
                        _ => 0,
                    })
            })
            .map(|deco_id| Box::new(PlaceDeco::new(tile, deco_id)) as Box<dyn editor_core::Command>),
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
        Ok(editor_core::MapProject {
            map,
            events,
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
