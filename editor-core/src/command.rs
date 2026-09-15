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
///
/// В клетке остаётся ОДНА декорация на категорию (категория = первое
/// слово имени объекта, Tree/GreenHills/Mountains/...): новая декорация
/// той же категории ЗАМЕНЯЕТ старую (нет слоёв деревьев в клетке);
/// декорация другой категории соседствует. `same_category` — индексы
/// декораций той же категории (вычисляет вызывающий по реестру; команда
/// домена реестра не знает). Заменённая пара сохраняется для undo:
/// удаление старой и добавление новой откатываются вместе.
#[derive(Clone, Debug, PartialEq)]
pub struct PlaceDeco {
    pub pos: Pos,
    pub deco_index: usize,
    /// Индексы декораций той же категории (заменялись бы в клетке).
    pub same_category: Vec<usize>,
    /// Индекс добавленной в `map.decomap`, заполняется при apply.
    placed_at: Option<usize>,
    /// Заменённая декорация (позиция вставки + значение) для undo.
    replaced: Option<(usize, dt_lib::map::deco::MapDeco)>,
}

impl PlaceDeco {
    pub fn new(pos: Pos, deco_index: usize) -> Self {
        Self {
            pos,
            deco_index,
            same_category: Vec::new(),
            placed_at: None,
            replaced: None,
        }
    }

    /// Индексы декораций той же категории (замена в клетке).
    pub fn with_category(mut self, same_category: Vec<usize>) -> Self {
        self.same_category = same_category;
        self
    }
}

impl Command for PlaceDeco {
    fn apply(&mut self, state: &mut EditorState) -> CommandResult {
        let project = state.project_mut();
        // Полный дубль (тот же id в клетке) — идемпотентный no-op.
        if project
            .map
            .decomap
            .iter()
            .any(|d| (d.x, d.y, d.index) == (self.pos.0, self.pos.1, self.deco_index))
        {
            return CommandResult::Noop;
        }
        // Замена существующей декорации той же категории в этой клетке.
        if let Some(at) = project.map.decomap.iter().position(|d| {
            (d.x, d.y) == (self.pos.0, self.pos.1)
                && self.same_category.contains(&d.index)
        }) {
            self.replaced = Some((at, project.map.decomap.remove(at)));
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
        let project = state.project_mut();
        if let Some(at) = self.placed_at.take() {
            if at < project.map.decomap.len() {
                project.map.decomap.remove(at);
            }
        }
        if let Some((at, deco)) = self.replaced.take() {
            let at = at.min(project.map.decomap.len());
            project.map.decomap.insert(at, deco);
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

/// Перемещение точки событий/фонарика (MapLantern) ПКМ-драгом на канвасе.
///
/// Меняет x/y в ОБОИХ источниках проекта (lanterns и map.lanterns —
/// редактор держит их синхронными); undo возвращает старую клетку.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MoveLantern {
    /// Индекс в Vec<MapLantern> (project.lanterns и map.lanterns).
    pub index: usize,
    pub from: Pos,
    pub to: Pos,
}

impl MoveLantern {
    pub fn new(index: usize, from: Pos, to: Pos) -> Self {
        Self { index, from, to }
    }
}

impl Command for MoveLantern {
    fn apply(&mut self, state: &mut EditorState) -> CommandResult {
        if self.from == self.to {
            return CommandResult::Noop;
        }
        let project = state.project_mut();
        let Some(lantern) = project.lanterns.get_mut(self.index) else {
            return CommandResult::Noop;
        };
        if (lantern.x, lantern.y) != self.from {
            return CommandResult::Noop; // индекс протух
        }
        lantern.x = self.to.0;
        lantern.y = self.to.1;
        if let Some(map_lantern) = project.map.lanterns.get_mut(self.index) {
            map_lantern.x = self.to.0;
            map_lantern.y = self.to.1;
        }
        CommandResult::Applied
    }

    fn undo(&mut self, state: &mut EditorState) {
        let project = state.project_mut();
        if let Some(lantern) = project.lanterns.get_mut(self.index) {
            lantern.x = self.from.0;
            lantern.y = self.from.1;
        }
        if let Some(map_lantern) = project.map.lanterns.get_mut(self.index) {
            map_lantern.x = self.from.0;
            map_lantern.y = self.from.1;
        }
    }

    fn name(&self) -> &'static str {
        "Move Lantern"
    }
}

/// Изменение радиуса света точки событий/фонарика (MapLantern).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SetLanternRadius {
    /// Индекс в Vec<MapLantern> (project.lanterns и map.lanterns).
    pub index: usize,
    pub radius: u8,
    previous: u8,
}

impl SetLanternRadius {
    pub fn new(index: usize, radius: u8) -> Self {
        Self {
            index,
            radius: radius.min(24),
            previous: 0,
        }
    }
}

impl Command for SetLanternRadius {
    fn apply(&mut self, state: &mut EditorState) -> CommandResult {
        let project = state.project_mut();
        let Some(lantern) = project.lanterns.get_mut(self.index) else {
            return CommandResult::Noop;
        };
        if lantern.light_radius == self.radius {
            return CommandResult::Noop;
        }
        self.previous = lantern.light_radius;
        lantern.light_radius = self.radius;
        if let Some(l) = project.map.lanterns.get_mut(self.index) {
            l.light_radius = self.radius;
        }
        CommandResult::Applied
    }

    fn undo(&mut self, state: &mut EditorState) {
        let project = state.project_mut();
        if let Some(lantern) = project.lanterns.get_mut(self.index) {
            lantern.light_radius = self.previous;
        }
        if let Some(l) = project.map.lanterns.get_mut(self.index) {
            l.light_radius = self.previous;
        }
    }

    fn name(&self) -> &'static str {
        "Set Lantern Radius"
    }
}

/// Постановка точки событий/фонарика (MapLantern) инструментом палитры.
///
/// Дубли в клетке запрещены (как декорации): существующая точка в (x, y)
/// — no-op. Точка добавляется в ОБОИХ источника (project.lanterns и
/// map.lanterns — редактор держит их синхронными).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlaceLantern {
    pub x: usize,
    pub y: usize,
    pub map_model: u8,
    pub radius: u8,
    pub active: bool,
    placed_at: Option<usize>,
}

impl PlaceLantern {
    pub fn new(x: usize, y: usize, map_model: u8, radius: u8, active: bool) -> Self {
        Self {
            x,
            y,
            map_model,
            radius,
            active,
            placed_at: None,
        }
    }
}

impl Command for PlaceLantern {
    fn apply(&mut self, state: &mut EditorState) -> CommandResult {
        let project = state.project_mut();
        // Одна точка на клетку.
        if project.lanterns.iter().any(|l| l.x == self.x && l.y == self.y) {
            return CommandResult::Noop;
        }
        let lantern = dt_lib::map::map::MapLantern {
            x: self.x,
            y: self.y,
            id: project.lanterns.len(),
            map_model: self.map_model,
            light_radius: self.radius,
            active_from_start: self.active,
            events: Vec::new(),
        };
        project.lanterns.push(lantern.clone());
        project.map.lanterns.push(lantern);
        self.placed_at = Some(project.lanterns.len() - 1);
        CommandResult::Applied
    }

    fn undo(&mut self, state: &mut EditorState) {
        let project = state.project_mut();
        if let Some(at) = self.placed_at.take() {
            if at < project.lanterns.len() {
                project.lanterns.remove(at);
            }
            if at < project.map.lanterns.len() {
                project.map.lanterns.remove(at);
            }
        }
    }

    fn name(&self) -> &'static str {
        "Place Lantern"
    }
}

/// Удаление точки событий/фонарика (снятая точка хранится для undo).
#[derive(Clone, Debug, PartialEq)]
pub struct RemoveLantern {
    /// Индекс в project.lanterns / map.lanterns.
    pub index: usize,
    /// Снятая точка (восстанавливается undo).
    taken: Option<dt_lib::map::map::MapLantern>,
}

impl RemoveLantern {
    pub fn new(index: usize) -> Self {
        Self { index, taken: None }
    }
}

impl Command for RemoveLantern {
    fn apply(&mut self, state: &mut EditorState) -> CommandResult {
        let project = state.project_mut();
        if self.index >= project.lanterns.len() {
            return CommandResult::Noop;
        }
        self.taken = Some(project.lanterns.remove(self.index));
        project.map.lanterns.remove(self.index);
        CommandResult::Applied
    }

    fn undo(&mut self, state: &mut EditorState) {
        let project = state.project_mut();
        if let Some(lantern) = self.taken.take() {
            let at = self.index.min(project.lanterns.len());
            project.lanterns.insert(at, lantern.clone());
            let at = self.index.min(project.map.lanterns.len());
            project.map.lanterns.insert(at, lantern);
        }
    }

    fn name(&self) -> &'static str {
        "Remove Lantern"
    }
}

/// Привязка события к точке: ссылка event_id в lantern.events
/// (индексный доступ по lantern_index; определение события не трогаем).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AddLanternEvent {
    pub lantern_index: usize,
    pub event_id: usize,
    added_at: Option<usize>,
}

impl AddLanternEvent {
    pub fn new(lantern_index: usize, event_id: usize) -> Self {
        Self {
            lantern_index,
            event_id,
            added_at: None,
        }
    }
}

impl Command for AddLanternEvent {
    fn apply(&mut self, state: &mut EditorState) -> CommandResult {
        let project = state.project_mut();
        let Some(lantern) = project.lanterns.get_mut(self.lantern_index) else {
            return CommandResult::Noop;
        };
        if lantern.events.contains(&self.event_id) {
            return CommandResult::Noop;
        }
        lantern.events.push(self.event_id);
        self.added_at = Some(lantern.events.len() - 1);
        if let Some(l) = project.map.lanterns.get_mut(self.lantern_index) {
            l.events.push(self.event_id);
        }
        CommandResult::Applied
    }

    fn undo(&mut self, state: &mut EditorState) {
        let project = state.project_mut();
        if let Some(at) = self.added_at.take() {
            if let Some(lantern) = project.lanterns.get_mut(self.lantern_index) {
                if at < lantern.events.len() {
                    lantern.events.remove(at);
                }
            }
            if let Some(l) = project.map.lanterns.get_mut(self.lantern_index) {
                if at < l.events.len() {
                    l.events.remove(at);
                }
            }
        }
    }

    fn name(&self) -> &'static str {
        "Add Lantern Event"
    }
}

/// Отвязка события от точки: удаляется ТОЛЬКО ссылка (index в
/// lantern.events); определение события остаётся в project.events —
/// на него могут ссылаться другие точки.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RemoveLanternEvent {
    pub lantern_index: usize,
    /// Позиция в lantern.events (не id события).
    pub index: usize,
    removed_id: Option<usize>,
}

impl RemoveLanternEvent {
    pub fn new(lantern_index: usize, index: usize) -> Self {
        Self {
            lantern_index,
            index,
            removed_id: None,
        }
    }
}

impl Command for RemoveLanternEvent {
    fn apply(&mut self, state: &mut EditorState) -> CommandResult {
        let project = state.project_mut();
        let Some(lantern) = project.lanterns.get_mut(self.lantern_index) else {
            return CommandResult::Noop;
        };
        if self.index >= lantern.events.len() {
            return CommandResult::Noop;
        }
        self.removed_id = Some(lantern.events.remove(self.index));
        if let Some(l) = project.map.lanterns.get_mut(self.lantern_index) {
            if self.index < l.events.len() {
                l.events.remove(self.index);
            }
        }
        CommandResult::Applied
    }

    fn undo(&mut self, state: &mut EditorState) {
        let project = state.project_mut();
        if let Some(id) = self.removed_id.take() {
            if let Some(lantern) = project.lanterns.get_mut(self.lantern_index) {
                let at = self.index.min(lantern.events.len());
                lantern.events.insert(at, id);
            }
            if let Some(l) = project.map.lanterns.get_mut(self.lantern_index) {
                let at = self.index.min(l.events.len());
                l.events.insert(at, id);
            }
        }
    }

    fn name(&self) -> &'static str {
        "Remove Lantern Event"
    }
}

/// Перенос строения (клик-клик/драг переносом в интеракте).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MoveBuilding {
    pub index: usize,
    pub from: Pos,
    pub to: Pos,
}

impl MoveBuilding {
    pub fn new(index: usize, from: Pos, to: Pos) -> Self {
        Self { index, from, to }
    }
}

impl Command for MoveBuilding {
    fn apply(&mut self, state: &mut EditorState) -> CommandResult {
        if self.from == self.to {
            return CommandResult::Noop;
        }
        let project = state.project_mut();
        let Some(building) = project.map.buildings.get_mut(self.index) else {
            return CommandResult::Noop;
        };
        if building.pos != self.from {
            return CommandResult::Noop;
        }
        building.pos = self.to;
        CommandResult::Applied
    }

    fn undo(&mut self, state: &mut EditorState) {
        if let Some(building) = state.project_mut().map.buildings.get_mut(self.index) {
            building.pos = self.from;
        }
    }

    fn name(&self) -> &'static str {
        "Move Building"
    }
}

/// Перенос армии (клик-клик/драг переносом в интеракте).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MoveArmy {
    pub index: usize,
    pub from: Pos,
    pub to: Pos,
}

impl MoveArmy {
    pub fn new(index: usize, from: Pos, to: Pos) -> Self {
        Self { index, from, to }
    }
}

impl Command for MoveArmy {
    fn apply(&mut self, state: &mut EditorState) -> CommandResult {
        if self.from == self.to {
            return CommandResult::Noop;
        }
        let project = state.project_mut();
        let Some(army) = project.map.armys.get_mut(self.index) else {
            return CommandResult::Noop;
        };
        if army.pos != self.from {
            return CommandResult::Noop;
        }
        army.pos = self.to;
        CommandResult::Applied
    }

    fn undo(&mut self, state: &mut EditorState) {
        if let Some(army) = state.project_mut().map.armys.get_mut(self.index) {
            army.pos = self.from;
        }
    }

    fn name(&self) -> &'static str {
        "Move Army"
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
    fn place_deco_same_id_in_cell_is_noop() {
        let mut state = state();
        let mut history = CommandHistory::new();
        history.execute(Box::new(PlaceDeco::new((1, 1), 3)), &mut state);
        let again = history.execute(Box::new(PlaceDeco::new((1, 1), 3)), &mut state);
        assert_eq!(again, CommandResult::Noop);
        assert_eq!(state.project().map.decomap.len(), 1);
    }

    #[test]
    fn place_deco_replaces_same_category_in_cell() {
        let mut state = state();
        let mut history = CommandHistory::new();
        // Дерево (3) в клетке; новое дерево (4) той же категории —
        // замена: старое удалено, новое стоит, len = 1.
        history.execute(Box::new(PlaceDeco::new((1, 1), 3)), &mut state);
        let res = history.execute(
            Box::new(PlaceDeco::new((1, 1), 4).with_category(vec![3, 4])),
            &mut state,
        );
        assert_eq!(res, CommandResult::Applied);
        assert_eq!(state.project().map.decomap.len(), 1);
        assert_eq!(state.project().map.decomap[0].index, 4);

        // Undo откатывает замену целиком: возвращено исходное дерево.
        history.undo(&mut state);
        assert_eq!(state.project().map.decomap.len(), 1);
        assert_eq!(state.project().map.decomap[0].index, 3);
    }

    #[test]
    fn place_deco_other_category_coexists() {
        let mut state = state();
        let mut history = CommandHistory::new();
        // Дерево (3) и холм (7) — разные категории: соседствуют.
        history.execute(Box::new(PlaceDeco::new((1, 1), 3)), &mut state);
        history.execute(
            Box::new(PlaceDeco::new((1, 1), 7).with_category(vec![7])),
            &mut state,
        );
        assert_eq!(state.project().map.decomap.len(), 2);
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
    fn place_remove_lantern_roundtrip() {
        let mut state = state();
        let mut history = CommandHistory::new();
        history.execute(
            Box::new(PlaceLantern::new(2, 3, 9, 3, false)),
            &mut state,
        );
        assert_eq!(state.project().lanterns.len(), 1);
        assert_eq!(state.project().map.lanterns.len(), 1);
        assert_eq!(state.project().lanterns[0].map_model, 9);
        assert!(!state.project().lanterns[0].active_from_start);

        // Дубль в клетке — no-op.
        let dup = history.execute(
            Box::new(PlaceLantern::new(2, 3, 8, 3, true)),
            &mut state,
        );
        assert_eq!(dup, CommandResult::Noop);

        history.undo(&mut state);
        assert!(state.project().lanterns.is_empty());
        assert!(state.project().map.lanterns.is_empty());

        // RemoveLantern с undo.
        history.redo(&mut state);
        history.execute(Box::new(RemoveLantern::new(0)), &mut state);
        assert!(state.project().lanterns.is_empty());
        history.undo(&mut state);
        assert_eq!(state.project().lanterns.len(), 1);
    }

    #[test]
    fn lantern_events_add_remove_undo() {
        let mut state = state();
        let mut history = CommandHistory::new();
        history.execute(
            Box::new(PlaceLantern::new(0, 0, 9, 3, false)),
            &mut state,
        );
        history.execute(Box::new(AddLanternEvent::new(0, 5)), &mut state);
        history.execute(Box::new(AddLanternEvent::new(0, 9)), &mut state);
        assert_eq!(state.project().lanterns[0].events, vec![5, 9]);
        assert_eq!(state.project().map.lanterns[0].events, vec![5, 9]);

        // Дубль ссылки — no-op.
        let dup = history.execute(Box::new(AddLanternEvent::new(0, 5)), &mut state);
        assert_eq!(dup, CommandResult::Noop);

        // Удаление ссылки (только ссылка, id остаётся в undo-данных).
        history.execute(Box::new(RemoveLanternEvent::new(0, 0)), &mut state);
        assert_eq!(state.project().lanterns[0].events, vec![9]);
        history.undo(&mut state);
        assert_eq!(state.project().lanterns[0].events, vec![5, 9]);
        history.undo(&mut state); // undo AddLanternEvent(9)
        history.undo(&mut state); // undo AddLanternEvent(5)
        assert!(state.project().lanterns[0].events.is_empty());
    }

    #[test]
    fn move_building_and_army_roundtrip() {
        let mut state = state();
        let mut history = CommandHistory::new();
        history.execute(Box::new(PlaceBuilding::new((1, 1), 2)), &mut state);
        history.execute(Box::new(PlaceArmy::new((2, 2), "A")), &mut state);

        history.execute(Box::new(MoveBuilding::new(0, (1, 1), (5, 5))), &mut state);
        assert_eq!(state.project().map.buildings[0].pos, (5, 5));
        history.undo(&mut state);
        assert_eq!(state.project().map.buildings[0].pos, (1, 1));

        history.execute(Box::new(MoveArmy::new(0, (2, 2), (3, 3))), &mut state);
        assert_eq!(state.project().map.armys[0].pos, (3, 3));
        history.undo(&mut state);
        assert_eq!(state.project().map.armys[0].pos, (2, 2));

        // Тот же from/to — no-op.
        let noop = history.execute(Box::new(MoveArmy::new(0, (2, 2), (2, 2))), &mut state);
        assert_eq!(noop, CommandResult::Noop);
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
