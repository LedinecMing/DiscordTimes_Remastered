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
use editor_core::project::MapProject;
use editor_core::EditorState;
use editor_validators::run_all;

/// Размер тайла на канвасе в пикселях (Фаза 0: без зума).
const TILE_SIZE: f32 = 16.0;

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
}

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
        }
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
                ui.close_menu();
            }
            if ui.button("Resize 100x100").clicked() {
                self.history.execute(Box::new(ResizeMap::new(100, 0)), &mut self.state);
                self.dirty = true;
                ui.close_menu();
            }
            if ui.button("Open…").clicked() {
                if let Some(path) = pick_open_path() {
                    self.open_project(path);
                }
                ui.close_menu();
            }
            if ui.button("Save").clicked() {
                self.save_project();
                ui.close_menu();
            }
            if ui.button("Save As…").clicked() {
                if let Some(path) = pick_save_path() {
                    self.project_path = Some(path);
                    self.save_project();
                }
                ui.close_menu();
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
                ui.close_menu();
            }
            if ui
                .add_enabled(self.history.can_redo(), egui::Button::new("Redo\tCtrl+Y"))
                .clicked()
            {
                self.history.redo(&mut self.state);
                self.dirty = true;
                ui.close_menu();
            }
        });
        ui.menu_button("Validate", |ui| {
            if ui.button("Lint").clicked() {
                self.issues = run_all(self.state.project());
                ui.close_menu();
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

    /// Канвас-заглушка: tilemap прямоугольниками (Фаза 0; wgpu — позже).
    fn canvas(&mut self, ui: &mut egui::Ui) {
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

        // Клик -> команда активного инструмента.
        if response.clicked() {
            if let Some(pos) = response.interact_pointer_pos() {
                if let Some(tile_pos) = tile_under(pos) {
                    self.apply_tool(tile_pos);
                }
            }
        }

        // Рендер: тайл цветом из палитры (tilemap.inner — построчно).
        for (index, &tile) in project.map.tilemap.inner.iter().enumerate() {
            let x = index % size;
            let y = index / size;
            let rect = egui::Rect::from_min_size(
                response.rect.min + egui::vec2(x as f32 * TILE_SIZE, y as f32 * TILE_SIZE),
                egui::vec2(TILE_SIZE, TILE_SIZE),
            );
            painter.rect_filled(rect, 0.0, tile_color(tile));
        }
        // Фонарики — жёлтые, строения — синие, армии — красные, декор — зелёные.
        for light in &project.lights {
            let center = object_center(response.rect.min, light.x, light.y);
            painter.circle_filled(center, TILE_SIZE / 3.0, egui::Color32::YELLOW);
        }
        for building in &project.map.buildings {
            let center = object_center(response.rect.min, building.pos.0, building.pos.1);
            painter.circle_filled(center, TILE_SIZE / 2.5, egui::Color32::BLUE);
        }
        for army in &project.map.armys {
            let center = object_center(response.rect.min, army.pos.0, army.pos.1);
            painter.circle_filled(center, TILE_SIZE / 2.5, egui::Color32::RED);
        }
        for deco in &project.map.decomap {
            let center = object_center(response.rect.min, deco.x, deco.y);
            painter.circle_filled(center, TILE_SIZE / 3.0, egui::Color32::GREEN);
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
        }
        if redo_hotkey {
            self.history.redo(&mut self.state);
            self.dirty = true;
        }

        egui::TopBottomPanel::top("top_menu").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| self.top_menu(ui));
        });
        egui::TopBottomPanel::bottom("status_bar").show(ctx, |ui| self.status_bar(ui));
        egui::SidePanel::right("tool_panel").show(ctx, |ui| self.side_panel(ui));
        egui::CentralPanel::default().show(ctx, |ui| self.canvas(ui));
    }
}

fn object_center(canvas_min: egui::Pos2, x: usize, y: usize) -> egui::Pos2 {
    canvas_min
        + egui::vec2(
            x as f32 * TILE_SIZE + TILE_SIZE / 2.0,
            y as f32 * TILE_SIZE + TILE_SIZE / 2.0,
        )
}

/// Цвет тайла по id (палитра-заглушка для TILES из dt_lib: вода, земля, ...).
fn tile_color(tile: usize) -> egui::Color32 {
    const COLORS: [egui::Color32; editor_core::project::TILE_COUNT] = [
        egui::Color32::from_rgb(120, 170, 220), // Shallow
        egui::Color32::from_rgb(70, 120, 200),  // Water
        egui::Color32::from_rgb(30, 60, 140),   // DeepWater
        egui::Color32::from_rgb(200, 80, 40),   // FlameLand
        egui::Color32::from_rgb(150, 140, 120), // Road
        egui::Color32::from_rgb(120, 160, 90),  // LowLand
        egui::Color32::from_rgb(90, 150, 70),   // Land
        egui::Color32::from_rgb(110, 170, 80),  // Plain
        egui::Color32::from_rgb(130, 120, 70),  // Swamp
        egui::Color32::from_rgb(90, 85, 50),    // DeepSwamp
        egui::Color32::from_rgb(210, 190, 130), // Desert
        egui::Color32::from_rgb(140, 110, 80),  // Badground
        egui::Color32::from_rgb(120, 120, 120), // Rock
        egui::Color32::from_rgb(180, 160, 120), // Dust
        egui::Color32::from_rgb(230, 230, 235), // Snow
        egui::Color32::from_rgb(220, 220, 230), // Snow (роща)
    ];
    COLORS[tile % COLORS.len()]
}

// Нативные диалоги файлов — Phase 1 (rfd); в каркасе путь вводится в консоль.
fn pick_open_path() -> Option<PathBuf> {
    console_prompt("путь для открытия")
}

fn pick_save_path() -> Option<PathBuf> {
    console_prompt("путь для сохранения")
}

fn console_prompt(what: &str) -> Option<PathBuf> {
    use std::io::Write;
    print!("{what}: ");
    std::io::stdout().flush().ok()?;
    let mut line = String::new();
    std::io::stdin().read_line(&mut line).ok()?;
    let trimmed = line.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(PathBuf::from(trimmed))
    }
}
