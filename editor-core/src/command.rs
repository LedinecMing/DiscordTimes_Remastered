//! Команды редактора (ТЗ §6: поток изменений через команды).
//!
//! [`Command`] — трейт с `apply`/`undo`. [`CommandHistory`] хранит undo/redo
//! стеки с лимитом (1000 по ТЗ §5) и правилом: новая команда сбрасывает redo.
//! Команды мутируют `GameMap`/события из dt_lib напрямую — без копий домена.

use crate::project::{default_building, Pos, TILE_COUNT};
use crate::state::EditorState;

/// Результат применения команды.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CommandResult {
    /// Применена (или отменена) — попала в историю.
    Applied,
    /// No-op: не изменила состояние и не попала в историю
    /// (например, PaintTile тем же тайлом).
    Noop,
}

/// Команда редактора: атомарное изменение state с обратимой стороной.
pub trait Command: std::fmt::Debug {
    /// Применить команду. `Noop`-возврат означает «ничего не изменилось».
    fn apply(&mut self, state: &mut EditorState) -> CommandResult;
    /// Отменить ранее применённую команду.
    fn undo(&mut self, state: &mut EditorState);
    /// Человекочитаемое имя (статус-бар, история, будущий command palette).
    fn name(&self) -> &'static str;
}

/// Лимит undo-стека (ТЗ §5: 1000 команд).
pub const HISTORY_LIMIT: usize = 1000;

/// История команд: undo/redo стеки.
#[derive(Debug, Default)]
pub struct CommandHistory {
    undo_stack: Vec<Box<dyn Command>>,
    redo_stack: Vec<Box<dyn Command>>,
}

impl CommandHistory {
    pub fn new() -> Self {
        Self::default()
    }

    /// Выполнить команду: применить, при успехе положить в undo и очистить redo.
    pub fn execute(
        &mut self,
        mut command: Box<dyn Command>,
        state: &mut EditorState,
    ) -> CommandResult {
        match command.apply(state) {
            CommandResult::Noop => CommandResult::Noop,
            CommandResult::Applied => {
                self.undo_stack.push(command);
                if self.undo_stack.len() > HISTORY_LIMIT {
                    self.undo_stack.remove(0);
                }
                self.redo_stack.clear();
                CommandResult::Applied
            }
        }
    }

    /// Отменить последнюю команду.
    pub fn undo(&mut self, state: &mut EditorState) -> Option<&'static str> {
        let mut command = self.undo_stack.pop()?;
        command.undo(state);
        let name = command.name();
        self.redo_stack.push(command);
        Some(name)
    }

    /// Повторить последнюю отменённую команду.
    pub fn redo(&mut self, state: &mut EditorState) -> Option<&'static str> {
        let mut command = self.redo_stack.pop()?;
        let result = command.apply(state);
        debug_assert_eq!(result, CommandResult::Applied, "redo of applied command");
        let name = command.name();
        self.undo_stack.push(command);
        Some(name)
    }

    pub fn can_undo(&self) -> bool {
        !self.undo_stack.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo_stack.is_empty()
    }

    pub fn undo_len(&self) -> usize {
        self.undo_stack.len()
    }
}

/// Рисование тайла кистью.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PaintTile {
    pub pos: Pos,
    pub tile: usize,
    previous: Option<usize>,
}

impl PaintTile {
    pub fn new(pos: Pos, tile: usize) -> Self {
        Self {
            pos,
            tile: tile.min(TILE_COUNT - 1),
            previous: None,
        }
    }
}

impl Command for PaintTile {
    fn apply(&mut self, state: &mut EditorState) -> CommandResult {
        let tilemap = &mut state.project_mut().map.tilemap;
        let (x, y) = self.pos;
        if x >= tilemap.size || y >= tilemap.size {
            return CommandResult::Noop;
        }
        // Конвенция экрана: pos=(x=колонка, y=строка), хранение tilemap[(строка,
        // колонка)] — как draw_tiles. Прежний прямой (x,y) транспонировал клетки.
        let cell = &mut tilemap[(y, x)];
        if *cell == self.tile {
            return CommandResult::Noop; // идемпотентность: тот же тайл — no-op
        }
        self.previous = Some(*cell);
        *cell = self.tile;
        CommandResult::Applied
    }

    fn undo(&mut self, state: &mut EditorState) {
        if let Some(prev) = self.previous {
            let tilemap = &mut state.project_mut().map.tilemap;
            let (x, y) = self.pos;
            if x < tilemap.size && y < tilemap.size {
                tilemap[(y, x)] = prev;
            }
        }
    }

    fn name(&self) -> &'static str {
        "Paint Tile"
    }
}

/// Постановка декорации (index в реестре декора, позиция).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlaceDeco {
    pub pos: Pos,
    pub deco_index: usize,
    /// Индекс в `map.decomap`, заполняется при apply (для undo).
    placed_at: Option<usize>,
}

impl PlaceDeco {
    pub fn new(pos: Pos, deco_index: usize) -> Self {
        Self {
            pos,
            deco_index,
            placed_at: None,
        }
    }
}

impl Command for PlaceDeco {
    fn apply(&mut self, state: &mut EditorState) -> CommandResult {
        let project = state.project_mut();
        // Не больше одной декорации на клетку: замена — отдельной командой.
        if project
            .map
            .decomap
            .iter()
            .any(|d| (d.x, d.y) == (self.pos.0, self.pos.1))
        {
            return CommandResult::Noop;
        }
        project.map.decomap.push(dt_lib::map::deco::MapDeco::new(
            self.deco_index,
            self.pos.0,
            self.pos.1,
        ));
        self.placed_at = Some(project.map.decomap.len() - 1);
        CommandResult::Applied
    }

    fn undo(&mut self, state: &mut EditorState) {
        if let Some(at) = self.placed_at.take() {
            state.project_mut().map.decomap.remove(at);
        }
    }

    fn name(&self) -> &'static str {
        "Place Deco"
    }
}

/// Постановка строения (id в реестре объектов, позиция).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlaceBuilding {
    pub pos: Pos,
    pub building_id: usize,
    placed_at: Option<usize>,
}

impl PlaceBuilding {
    pub fn new(pos: Pos, building_id: usize) -> Self {
        Self {
            pos,
            building_id,
            placed_at: None,
        }
    }
}

impl Command for PlaceBuilding {
    fn apply(&mut self, state: &mut EditorState) -> CommandResult {
        let project = state.project_mut();
        project
            .map
            .buildings
            .push(default_building(self.building_id, self.pos));
        self.placed_at = Some(project.map.buildings.len() - 1);
        CommandResult::Applied
    }

    fn undo(&mut self, state: &mut EditorState) {
        if let Some(at) = self.placed_at.take() {
            state.project_mut().map.buildings.remove(at);
        }
    }

    fn name(&self) -> &'static str {
        "Place Building"
    }
}

/// Постановка армии (позиция + имя отряда + юниты-шаблон).
///
/// `template_units` — id юнитов из реестра units (Units.ini): армия
/// заполняется палеточным шаблоном (рыцарь/бандит/крестьянин/зомби).
/// Команда самодостаточна: армия строится при создании (реестр нужен
/// только там) через `default_army_with_registry` — hitmap/max_troops/
/// инвентарь корректны, как при загрузке .dtm.
pub struct PlaceArmy {
    pub pos: Pos,
    pub army_name: String,
    /// Готовая армия (юниты-шаблоны уже внутри).
    army: dt_lib::battle::army::Army,
    placed_at: Option<usize>,
}

impl std::fmt::Debug for PlaceArmy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PlaceArmy")
            .field("pos", &self.pos)
            .field("army_name", &self.army_name)
            .finish()
    }
}

impl PlaceArmy {
    /// Пустая армия (без юнитов) — как в тестах/минимальной постановке.
    pub fn new(pos: Pos, army_name: impl Into<String>) -> Self {
        Self::with_units(pos, army_name, Vec::new(), &dt_lib::registry::GameInfo::new())
    }

    /// Армия с юнитами-шаблонами: `template_units` — id из реестра units
    /// (первый — главный), `registry` — реестр игры (уже загружен).
    pub fn with_units(
        pos: Pos,
        army_name: impl Into<String>,
        template_units: Vec<usize>,
        registry: &dt_lib::registry::GameInfo,
    ) -> Self {
        let army_name = army_name.into();
        let army = dt_lib::map::object::default_army_with_registry(
            &template_units,
            army_name.clone(),
            pos,
            registry,
        );
        Self {
            pos,
            army_name,
            army,
            placed_at: None,
        }
    }
}

impl Command for PlaceArmy {
    fn apply(&mut self, state: &mut EditorState) -> CommandResult {
        let project = state.project_mut();
        let mut army = self.army.clone();
        army.pos = self.pos;
        project.map.armys.push(army);
        self.placed_at = Some(project.map.armys.len() - 1);
        CommandResult::Applied
    }

    fn undo(&mut self, state: &mut EditorState) {
        if let Some(at) = self.placed_at.take() {
            state.project_mut().map.armys.remove(at);
        }
    }

    fn name(&self) -> &'static str {
        "Place Army"
    }
}

/// Изменение размера квадратной карты (якорь — левый верхний угол).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResizeMap {
    pub size: usize,
    /// Заливка для новых клеток при расширении.
    pub fill_tile: usize,
    previous: Vec<usize>,
    previous_size: usize,
}

impl ResizeMap {
    pub fn new(size: usize, fill_tile: usize) -> Self {
        Self {
            size: size.clamp(1, 800),
            fill_tile: fill_tile.min(TILE_COUNT - 1),
            previous: Vec::new(),
            previous_size: 0,
        }
    }
}

impl Command for ResizeMap {
    fn apply(&mut self, state: &mut EditorState) -> CommandResult {
        let tilemap = &mut state.project_mut().map.tilemap;
        if tilemap.size == self.size {
            return CommandResult::Noop;
        }
        self.previous = tilemap.inner.clone();
        self.previous_size = tilemap.size;
        let old = std::mem::take(&mut tilemap.inner);
        let old_size = std::mem::replace(&mut tilemap.size, self.size);
        tilemap.inner = vec![self.fill_tile; self.size * self.size];
        for y in 0..old_size.min(self.size) {
            for x in 0..old_size.min(self.size) {
                tilemap.inner[y * self.size + x] = old[y * old_size + x];
            }
        }
        CommandResult::Applied
    }

    fn undo(&mut self, state: &mut EditorState) {
        if !self.previous.is_empty() {
            let tilemap = &mut state.project_mut().map.tilemap;
            tilemap.inner = std::mem::take(&mut self.previous);
            tilemap.size = self.previous_size;
        }
    }

    fn name(&self) -> &'static str {
        "Resize Map"
    }
}

/// Установка свойства проекта (имя сценария / описание).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ProjectProperty {
    Name(String),
    Description(String),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SetProperty {
    pub property: ProjectProperty,
    previous: Option<ProjectProperty>,
}

impl SetProperty {
    pub fn new(property: ProjectProperty) -> Self {
        Self {
            property,
            previous: None,
        }
    }
}

impl Command for SetProperty {
    fn apply(&mut self, state: &mut EditorState) -> CommandResult {
        let start = &mut state.project_mut().map.start;
        let old = match &self.property {
            ProjectProperty::Name(_) => ProjectProperty::Name(start.name.clone()),
            ProjectProperty::Description(_) => {
                ProjectProperty::Description(start.description.clone())
            }
        };
        let changed = match (&self.property, &old) {
            (ProjectProperty::Name(new), ProjectProperty::Name(prev)) => new != prev,
            (ProjectProperty::Description(new), ProjectProperty::Description(prev)) => {
                new != prev
            }
            _ => true,
        };
        if !changed {
            return CommandResult::Noop;
        }
        self.previous = Some(old);
        match &self.property {
            ProjectProperty::Name(v) => start.name = v.clone(),
            ProjectProperty::Description(v) => start.description = v.clone(),
        }
        CommandResult::Applied
    }

    fn undo(&mut self, state: &mut EditorState) {
        let start = &mut state.project_mut().map.start;
        match self.previous.take() {
            Some(ProjectProperty::Name(v)) => start.name = v,
            Some(ProjectProperty::Description(v)) => start.description = v,
            None => {}
        }
    }

    fn name(&self) -> &'static str {
        "Set Property"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::project::MapProject;

    fn state() -> EditorState {
        EditorState::new(MapProject::new(8, 0))
    }

    #[test]
    fn paint_tile_apply_undo_roundtrip() {
        let mut state = state();
        let mut history = CommandHistory::new();
        let applied = history.execute(Box::new(PaintTile::new((2, 3), 7)), &mut state);
        assert_eq!(applied, CommandResult::Applied);
        assert_eq!(state.project().tile((2, 3)), Some(7));

        history.undo(&mut state);
        assert_eq!(state.project().tile((2, 3)), Some(0));

        history.redo(&mut state);
        assert_eq!(state.project().tile((2, 3)), Some(7));
    }

    #[test]
    fn paint_tile_is_idempotent() {
        let mut state = state();
        let mut history = CommandHistory::new();
        let again = history.execute(Box::new(PaintTile::new((2, 3), 0)), &mut state);
        assert_eq!(again, CommandResult::Noop);
        assert!(!history.can_undo());
    }

    #[test]
    fn paint_tile_out_of_bounds_is_noop() {
        let mut state = state();
        let mut history = CommandHistory::new();
        let res = history.execute(Box::new(PaintTile::new((100, 100), 3)), &mut state);
        assert_eq!(res, CommandResult::Noop);
    }

    #[test]
    fn resize_map_grows_and_shrinks() {
        let mut state = state();
        let mut history = CommandHistory::new();
        history.execute(Box::new(ResizeMap::new(4, 1)), &mut state);
        assert_eq!(state.project().size(), 4);
        // (3,3) внутри старого квадрата 8x8 — сохранён старый тайл 0.
        assert_eq!(state.project().tile((3, 3)), Some(0));
        // Клеток за пределами старого квадрата в 4x4 нет (сжатие), поэтому
        // заливку проверяем на расширении в resize_map_keeps_content.

        history.undo(&mut state);
        assert_eq!(state.project().size(), 8);
        assert_eq!(state.project().tile((7, 7)), Some(0));
    }

    #[test]
    fn resize_map_keeps_content() {
        let mut state = state();
        state.project_mut().map.tilemap[(1, 1)] = 9;
        let mut history = CommandHistory::new();
        history.execute(Box::new(ResizeMap::new(10, 5)), &mut state);
        // Старое содержимое сохранено.
        assert_eq!(state.project().tile((1, 1)), Some(9));
        // Новые клетки залиты fill_tile.
        assert_eq!(state.project().tile((9, 9)), Some(5));
        assert_eq!(state.project().tile((8, 2)), Some(5));
    }

    #[test]
    fn set_property_undo_redo() {
        let mut state = state();
        let mut history = CommandHistory::new();
        history.execute(
            Box::new(SetProperty::new(ProjectProperty::Name("Test".into()))),
            &mut state,
        );
        assert_eq!(state.project().map.start.name, "Test");
        history.undo(&mut state);
        assert_eq!(state.project().map.start.name, "");
        history.redo(&mut state);
        assert_eq!(state.project().map.start.name, "Test");
    }

    #[test]
    fn set_property_noop_when_same_value() {
        let mut state = state();
        let mut history = CommandHistory::new();
        let res = history.execute(
            Box::new(SetProperty::new(ProjectProperty::Name(String::new()))),
            &mut state,
        );
        assert_eq!(res, CommandResult::Noop);
    }

    #[test]
    fn place_deco_and_army_roundtrip() {
        let mut state = state();
        let mut history = CommandHistory::new();

        history.execute(Box::new(PlaceDeco::new((1, 1), 3)), &mut state);
        assert_eq!(state.project().map.decomap.len(), 1);
        assert_eq!(
            (state.project().map.decomap[0].x, state.project().map.decomap[0].y),
            (1, 1)
        );

        history.execute(Box::new(PlaceArmy::new((2, 2), "Recon")), &mut state);
        assert_eq!(state.project().map.armys.len(), 1);
        assert_eq!(state.project().map.armys[0].pos, (2, 2));

        history.execute(Box::new(PlaceBuilding::new((3, 3), 2)), &mut state);
        assert_eq!(state.project().map.buildings.len(), 1);
        assert_eq!(state.project().map.buildings[0].pos, (3, 3));

        history.undo(&mut state);
        history.undo(&mut state);
        history.undo(&mut state);
        assert!(state.project().map.decomap.is_empty());
        assert!(state.project().map.armys.is_empty());
        assert!(state.project().map.buildings.is_empty());
    }

    #[test]
    fn place_deco_rejects_occupied_cell() {
        let mut state = state();
        let mut history = CommandHistory::new();
        history.execute(Box::new(PlaceDeco::new((1, 1), 3)), &mut state);
        let again = history.execute(Box::new(PlaceDeco::new((1, 1), 4)), &mut state);
        assert_eq!(again, CommandResult::Noop);
        assert_eq!(state.project().map.decomap.len(), 1);
    }

    #[test]
    fn new_command_clears_redo_stack() {
        let mut state = state();
        let mut history = CommandHistory::new();
        history.execute(Box::new(PaintTile::new((0, 0), 1)), &mut state);
        history.undo(&mut state);
        assert!(history.can_redo());
        history.execute(Box::new(PaintTile::new((0, 0), 2)), &mut state);
        assert!(!history.can_redo());
    }

    #[test]
    fn history_limit_1000() {
        let mut state = state();
        let mut history = CommandHistory::new();
        for i in 0..(HISTORY_LIMIT + 50) {
            history.execute(
                Box::new(SetProperty::new(ProjectProperty::Name(format!("n{i}")))),
                &mut state,
            );
        }
        assert_eq!(history.undo_len(), HISTORY_LIMIT);
        // Самая старая команда вытеснена: первый name — "n50".
        assert_eq!(
            state.project().map.start.name,
            format!("n{}", HISTORY_LIMIT + 49)
        );
        for _ in 0..HISTORY_LIMIT {
            history.undo(&mut state);
        }
        assert!(!history.can_undo());
        // Вытесненная команда не восстановима.
        assert_ne!(state.project().map.start.name, "n0");
    }
}
