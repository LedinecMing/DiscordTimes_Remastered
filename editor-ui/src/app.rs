//! Приложение редактора: окно eframe, top menu, канвас-заглушка, статус-бар.
//!
//! Принципы ТЗ §2, реализованные в каркасе:
//! - п.2 «Меню действий сверху»: File (New/Open/Save/Save As/Quit), Edit (Undo/Redo);
//! - доп. «Статус-бар снизу»: координаты курсора, активный инструмент,
//!   число ошибок/предупреждений валидаторов, число команд в истории;
//! - п.1 (докинг), Command Palette и настраиваемые хоткеи — Phase 2+.

use std::path::PathBuf;

use eframe::egui;

use editor_core::command::{
    Command, CommandHistory, PaintTile, PlaceArmy, PlaceBuilding, PlaceDeco, ResizeMap,
};
use editor_core::render::BakedMap;
use editor_core::project::MapProject;
use editor_core::EditorState;
use editor_validators::run_all;

/// Размер тайла на канвасе в пикселях (Фаза 0: без зума).
const TILE_SIZE: f32 = 16.0;
const TILE_HALF: f32 = 8.0;

/// Активный инструмент (палитра ТЗ §3 Map View, срез — четыре инструмента).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tool {
    /// Кисть тайлов.
    Brush,
    /// Постановка декора.
    Deco,
    /// Постановка строения.
    Building,
    /// Постановка армии.
    Army,
}

impl Tool {
    fn label(self) -> &'static str {
        match self {
            Tool::Brush => "Кисть",
            Tool::Deco => "Декор",
            Tool::Building => "Строение",
            Tool::Army => "Армия",
        }
    }

    const ALL: [Tool; 4] = [Tool::Brush, Tool::Deco, Tool::Building, Tool::Army];
}

/// Приложение редактора.
pub struct MapEditorApp {
    state: EditorState,
    history: CommandHistory,
    project_path: Option<PathBuf>,
    tool: Tool,
    /// Счётчик для имён новых объектов.
    counter: usize,
    /// Кэш результата валидаторов (обновляется по кнопке Lint / перед сохранением).
    issues: Vec<editor_validators::Issue>,
    dirty: bool,
    /// Запечённая карта: RGBA-кэш + egui-текстура + флаг инвалидности.
    bake: Option<BakedMap>,
    bake_texture: Option<egui::TextureHandle>,
    bake_dirty: bool,
    /// Выделенная клетка (клик) — рамка поверх текстуры.
    selected: Option<editor_core::project::Pos>,
    /// Показывать сетку клеток.
    show_grid: bool,
}

/// Директория ассетов относительно cwd (бинарник запускать из dt/, как клиент).
const ASSETS_TERRAIN: &str = "assets/Terrain";
const ASSETS_OBJECTS: &str = "assets/Objects";

impl MapEditorApp {
    /// Новый редактор с картой 50x50, залитой водой (как старый редактор).
    pub fn new() -> Self {
        Self {
            state: EditorState::new(MapProject::new(50, 0)),
            history: CommandHistory::new(),
            project_path: None,
            tool: Tool::Brush,
            counter: 0,
            issues: Vec::new(),
            dirty: false,
            bake: None,
            bake_texture: None,
            bake_dirty: true,
            selected: None,
            show_grid: true,
        }
    }

    /// Пересобрать RGBA-кэш запечки (tilemap + objects), пометить текстуру.
    fn rebake(&mut self) {
        let project = self.state.project().clone();
        let sprites = editor_core::render::tile_sprites(ASSETS_TERRAIN);
        let baked = editor_core::render::bake_tilemap(&project.map.tilemap, &sprites);
        let mut cache = editor_core::render::SpriteCache::new();
        let objects_layer = editor_core::render::bake_objects(
            &project.map,
            &self.registry_objects(),
            &mut cache,
            ASSETS_OBJECTS,
        );
        // Слой объектов поверх тайлов (альфа-блит уже в bake_objects).
        let mut rgba = baked.rgba;
        overlay(&mut rgba, &objects_layer.rgba);
        self.bake = Some(BakedMap {
            rgba,
            width: baked.width,
            height: baked.height,
        });
        self.bake_dirty = false;
    }

    /// Реестр объектов: лениво парсится Objects.ini; при ошибке — пустой.
    fn registry_objects(&self) -> Vec<dt_lib::map::object::ObjectInfo> {
        let ini = std::fs::read_to_string("Objects.ini").unwrap_or_default();
        parse_objects_ini(&ini)
    }

    fn top_menu(&mut self, ui: &mut egui::Ui) {
        ui.menu_button("File", |ui| {
            if ui.button("New (50x50)").clicked() {
                // Чистая инициализация state (гайд 9.1: без кэша прошлой карты).
                self.state = EditorState::new(MapProject::new(50, 0));
                self.history = CommandHistory::new();
                self.project_path = None;
                self.issues.clear();
                self.dirty = false;
                ui.close();
            }
            if ui.button("Resize 100x100").clicked() {
                self.history.execute(Box::new(ResizeMap::new(100, 0)), &mut self.state);
                self.dirty = true;
                ui.close();
            }
            if ui.button("Open…").clicked() {
                if let Some(path) = pick_open_path() {
                    self.open_project(path);
                }
                ui.close();
            }
            if ui.button("Save").clicked() {
                self.save_project();
                ui.close();
            }
            if ui.button("Save As…").clicked() {
                if let Some(path) = pick_save_path() {
                    self.project_path = Some(path);
                    self.save_project();
                }
                ui.close();
            }
            if ui.button("Quit").clicked() {
                ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
            }
        });
        ui.menu_button("Edit", |ui| {
            if ui
                .add_enabled(self.history.can_undo(), egui::Button::new("Undo\tCtrl+Z"))
                .clicked()
            {
                self.history.undo(&mut self.state);
                self.dirty = true;
                ui.close();
            }
            if ui
                .add_enabled(self.history.can_redo(), egui::Button::new("Redo\tCtrl+Y"))
                .clicked()
            {
                self.history.redo(&mut self.state);
                self.dirty = true;
                ui.close();
            }
        });
        ui.menu_button("Validate", |ui| {
            if ui.button("Lint").clicked() {
                self.issues = run_all(self.state.project());
                ui.close();
            }
        });
    }

    fn open_project(&mut self, path: PathBuf) {
        let attempted = std::fs::read_to_string(&path)
            .map_err(|e| e.to_string())
            .and_then(|json| MapProject::from_json(&json).map_err(|e| e.to_string()));
        match attempted {
            Ok(project) => {
                self.state = EditorState::new(project);
                self.history = CommandHistory::new();
                self.project_path = Some(path);
                self.issues.clear();
                self.dirty = false;
            }
            Err(err) => eprintln!("open failed: {err}"),
        }
    }

    fn save_project(&mut self) {
        let Some(path) = self.project_path.clone() else {
            return;
        };
        // Валидаторы перед сохранением (ТЗ §5); Error блокируют сохранение.
        self.issues = run_all(self.state.project());
        if self
            .issues
            .iter()
            .any(|i| i.severity == editor_validators::Severity::Error)
        {
            eprintln!("save blocked: errors found");
            return;
        }
        let saved = self
            .state
            .project()
            .to_json()
            .map_err(|e| e.to_string())
            .and_then(|json| std::fs::write(&path, json).map_err(|e| e.to_string()));
        match saved {
            Ok(()) => self.dirty = false,
            Err(err) => eprintln!("save failed: {err}"),
        }
    }
    /// Канвас: запечённая карта текстурой + объектные маркеры + сетка +
    /// выделение клетки. Запечка — по флагу bake_dirty (не в кадре).
    fn canvas(&mut self, ui: &mut egui::Ui) {
        if self.bake_dirty || self.bake.is_none() {
            self.rebake();
        }
        let Some(baked) = self.bake.clone() else {
            return;
        };
        let project = self.state.project().clone();
        let size = project.size();
        let canvas_size = egui::vec2(size as f32 * TILE_SIZE, size as f32 * TILE_SIZE);
        let (response, painter) = ui.allocate_painter(canvas_size, egui::Sense::click_and_drag());

        let tile_under = |pos: egui::Pos2| -> Option<editor_core::project::Pos> {
            let tx = ((pos.x - response.rect.min.x) / TILE_SIZE).floor() as i64;
            let ty = ((pos.y - response.rect.min.y) / TILE_SIZE).floor() as i64;
            (tx >= 0 && ty >= 0 && (tx as usize) < size && (ty as usize) < size)
                .then_some((tx as usize, ty as usize))
        };

        // Позиция курсора в тайлах — для статус-бара.
        if let Some(pos) = response.interact_pointer_pos() {
            if let Some(tile_pos) = tile_under(pos) {
                self.state.set_cursor(tile_pos);
            }
        }

        // Клик -> выделение клетки + команда активного инструмента.
        if response.clicked() {
            if let Some(pos) = response.interact_pointer_pos() {
                if let Some(tile_pos) = tile_under(pos) {
                    self.selected = Some(tile_pos);
                    self.apply_tool(tile_pos);
                }
            }
        }

        // Текстура карты: ColorImage из запечённого RGBA.
        let texture = self.bake_texture.get_or_insert_with(|| {
            ui.ctx().load_texture(
                "map_baked",
                egui::ColorImage::from_rgba_unmultiplied(
                    [baked.width as usize, baked.height as usize],
                    &baked.rgba,
                ),
                egui::TextureOptions::NEAREST,
            )
        });
        if texture.size() != [baked.width as usize, baked.height as usize] {
            texture.set(
                egui::ColorImage::from_rgba_unmultiplied(
                    [baked.width as usize, baked.height as usize],
                    &baked.rgba,
                ),
                egui::TextureOptions::NEAREST,
            );
        }
        // Содержимое могло измениться без смены размера — перезаливка по флагу.
        if self.bake_dirty {
            texture.set(
                egui::ColorImage::from_rgba_unmultiplied(
                    [baked.width as usize, baked.height as usize],
                    &baked.rgba,
                ),
                egui::TextureOptions::NEAREST,
            );
            self.bake_dirty = false;
        }
        painter.image(
            texture.id(),
            response.rect,
            egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
            egui::Color32::WHITE,
        );

        // Маркеры динамических объектов поверх запечённого слоя.
        for light in &project.lights {
            let center = object_center(response.rect.min, light.x, light.y);
            painter.circle_filled(center, TILE_SIZE * 0.33, egui::Color32::YELLOW);
        }
        for army in &project.map.armys {
            let center = object_center(response.rect.min, army.pos.0, army.pos.1);
            painter.circle_filled(center, TILE_SIZE * 0.4, egui::Color32::RED);
        }

        // Сетка клеток.
        if self.show_grid {
            let stroke = egui::Stroke::new(1.0f32, egui::Color32::from_black_alpha(60));
            for i in 0..=size {
                let p = response.rect.min + egui::vec2(i as f32 * TILE_SIZE, i as f32 * TILE_SIZE);
                painter.line_segment(
                    [egui::pos2(p.x, response.rect.min.y), egui::pos2(p.x, response.rect.max.y)],
                    stroke,
                );
                painter.line_segment(
                    [egui::pos2(response.rect.min.x, p.y), egui::pos2(response.rect.max.x, p.y)],
                    stroke,
                );
            }
        }

        // Выделение клетки.
        if let Some((sx, sy)) = self.selected {
            let rect = egui::Rect::from_min_size(
                response.rect.min + egui::vec2(sx as f32 * TILE_SIZE, sy as f32 * TILE_SIZE),
                egui::vec2(TILE_SIZE, TILE_SIZE),
            );
            painter.rect_stroke(rect, 0.0f32, egui::Stroke::new(2.0f32, egui::Color32::YELLOW), egui::StrokeKind::Inside);
        }
    }

    fn apply_tool(&mut self, tile_pos: editor_core::project::Pos) {
        let command: Box<dyn Command> = match self.tool {
            Tool::Brush => Box::new(PaintTile::new(tile_pos, self.state.active_tile())),
            Tool::Deco => {
                self.counter += 1;
                Box::new(PlaceDeco::new(tile_pos, self.counter))
            }
            Tool::Building => {
                self.counter += 1;
                Box::new(PlaceBuilding::new(tile_pos, self.counter % 14))
            }
            Tool::Army => {
                self.counter += 1;
                Box::new(PlaceArmy::new(tile_pos, format!("Army {}", self.counter)))
            }
        };
        if self.history.execute(command, &mut self.state) == editor_core::CommandResult::Applied {
            self.dirty = true;
            self.bake_dirty = true;
        }
    }


    /// Правая панель: палитра инструментов и тайлов (ТЗ §3, упрощённо).
    fn side_panel(&mut self, ui: &mut egui::Ui) {
        ui.heading("Инструменты");
        for tool in Tool::ALL {
            if ui.selectable_label(self.tool == tool, tool.label()).clicked() {
                self.tool = tool;
            }
        }
        ui.separator();
        ui.heading("Тайлы");
        ui.horizontal_wrapped(|ui| {
            for tile in 0..editor_core::project::TILE_COUNT {
                if ui
                    .selectable_label(self.state.active_tile() == tile, format!("{tile}"))
                    .clicked()
                {
                    self.state.set_active_tile(tile);
                }
            }
        });
        ui.separator();
        if ui.button("Lint").clicked() {
            self.issues = run_all(self.state.project());
        }
    }

    /// Статус-бар: координаты курсора, инструмент, ошибки валидаторов (ТЗ §2).
    fn status_bar(&self, ui: &mut egui::Ui) {
        let (cx, cy) = self.state.cursor();
        let errors = self
            .issues
            .iter()
            .filter(|i| i.severity == editor_validators::Severity::Error)
            .count();
        let warnings = self
            .issues
            .iter()
            .filter(|i| i.severity == editor_validators::Severity::Warning)
            .count();
        let path = self
            .project_path
            .as_ref()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|| "без имени".into());
        let dirty_mark = if self.dirty { " *" } else { "" };
        ui.horizontal(|ui| {
            ui.label(format!("x: {cx}  y: {cy}"));
            ui.separator();
            ui.label(self.tool.label());
            ui.separator();
            ui.label(format!("ошибок: {errors}  предупреждений: {warnings}"));
            ui.separator();
            ui.label(format!("{path}{dirty_mark}"));
            ui.separator();
            ui.label(format!(
                "undo: {}  redo: {}",
                self.history.can_undo(),
                self.history.can_redo()
            ));
        });
    }
}

impl Default for MapEditorApp {
    fn default() -> Self {
        Self::new()
    }
}

impl eframe::App for MapEditorApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Хоткеи (ТЗ §2): Ctrl+Z undo, Ctrl+Y redo.
        let (undo_hotkey, redo_hotkey) = ctx.input(|input| {
            (
                input.modifiers.ctrl && input.key_pressed(egui::Key::Z),
                input.modifiers.ctrl && input.key_pressed(egui::Key::Y),
            )
        });
        if undo_hotkey {
            self.history.undo(&mut self.state);
            self.dirty = true;
            self.bake_dirty = true;
        }
        if redo_hotkey {
            self.history.redo(&mut self.state);
            self.dirty = true;
            self.bake_dirty = true;
        }

        egui::TopBottomPanel::top("top_menu").show(ctx, |ui| {
            egui::MenuBar::new().ui(ui, |ui| self.top_menu(ui));
        });
        egui::TopBottomPanel::bottom("status_bar").show(ctx, |ui| self.status_bar(ui));
        egui::SidePanel::right("tool_panel").show(ctx, |ui| self.side_panel(ui));
        egui::CentralPanel::default().show(ctx, |ui| self.canvas(ui));
    }
}

fn object_center(canvas_min: egui::Pos2, x: usize, y: usize) -> egui::Pos2 {
    canvas_min
        + egui::vec2(
            x as f32 * TILE_SIZE + TILE_HALF,
            y as f32 * TILE_SIZE + TILE_HALF,
        )
}

/// Наложить слой объектов поверх запечённого тайлами слоя (альфа-блит).
fn overlay(dst: &mut Vec<u8>, src: &[u8]) {
    for (d, s) in dst.chunks_exact_mut(4).zip(src.chunks_exact(4)) {
        if s[3] >= 128 {
            d.copy_from_slice(s);
        }
    }
}

/// Минимальный парсер Objects.ini: секция -> (index, path, obj_type-id).
/// Полный парсер (advini-derive) — в dt_lib; здесь нужен только путь спрайта
/// и id/индекс для декора и строений. Формат секции:
/// [Tree000] index=1 type=MapDeco id=3 ...
/// Путь спрайта: имя секции до первой цифры + %03d + ".png" (parse_objects).
fn parse_objects_ini(ini: &str) -> Vec<dt_lib::map::object::ObjectInfo> {
    use dt_lib::map::object::{ObjectInfo, ObjectType};
    let mut objects = Vec::new();
    let mut section: Option<String> = None;
    let mut index = 0usize;
    let mut deco_id = None;
    let mut obj_kind = "";
    let flush = |section: &Option<String>, index: usize, deco_id: Option<usize>, kind: &str, out: &mut Vec<ObjectInfo>| {
        if let Some(name) = section {
            let path = match name.find(|c: char| c.is_ascii_digit()) {
                Some(at) => {
                    let (a, b) = name.split_at(at);
                    match b.parse::<usize>() {
                        Ok(num) => format!("{a}{num:03}.png"),
                        Err(_) => format!("{name}.png"),
                    }
                }
                None => format!("{name}.png"),
            };
            let obj_type = match kind {
                "MapDeco" => ObjectType::MapDeco {
                    id: deco_id.unwrap_or(0),
                },
                _ => ObjectType::Building { group: 0, variant: 0 },
            };
            out.push(ObjectInfo {
                name: name.clone(),
                path,
                category: String::new(),
                obj_type,
                index,
                size: (1, 1),
            });
        }
    };
    for line in ini.lines() {
        let line = line.trim();
        if let Some(rest) = line.strip_prefix('[') {
            let Some(name) = rest.strip_suffix(']') else { continue };
            flush(&section, index, deco_id, obj_kind, &mut objects);
            section = Some(name.to_string());
            index = 0;
            deco_id = None;
            obj_kind = "";
        } else if section.is_some() {
            let Some((key, value)) = line.split_once('=') else { continue };
            let key = key.trim();
            let value = value.trim();
            match key {
                "index" => index = value.parse().unwrap_or(0),
                "id" => deco_id = value.parse().ok(),
                "type" => obj_kind = value,
                _ => {}
            }
        }
    }
    flush(&section, index, deco_id, obj_kind, &mut objects);
    objects
}
// Нативные диалоги файлов (rfd). Блокирующий вызов допустим: диалог модален
// по смыслу, пользователь не работает с канвасом, пока он открыт.
fn pick_open_path() -> Option<PathBuf> {
    rfd::FileDialog::new()
        .set_title("Открыть проект карты")
        .add_filter("Проект редактора (JSON)", &["json"])
        .pick_file()
}

fn pick_save_path() -> Option<PathBuf> {
    rfd::FileDialog::new()
        .set_title("Сохранить проект карты")
        .add_filter("Проект редактора (JSON)", &["json"])
        .set_file_name("map.json")
        .pick_file()
}
