// Порт состояния игры из quad_ui main.rs: State/Game/GameVariant/Menu/UiState,
// game_init, load_map, process_event и хелперы текстур юнитов.
use crate::assets::{load_assets, load_fonts, Assets, BENGUIAT};
use crate::camera::Camera;
use crate::files::DesktopFiles;
use crate::gfx::{Gfx, TexId};
use crate::text::{FontId, TextRenderer};
use dt_lib::battle::army::*;
use dt_lib::battle::control::{Control, Player};
use dt_lib::battle::troop::Troop;
use dt_lib::locale::{parse_locale, Locale};
use dt_lib::map::convert::{convert_dtm_map, parse_dtm_vec};
use dt_lib::map::map::GameMap;
use dt_lib::mutrc::SendMut;
use dt_lib::map::event::{Event as GameEvent, Events};
use dt_lib::network::room::{RoomConfig, RoomId, RoomManager, RoomView};
use dt_lib::network::server::Executor;
use dt_lib::parse::{
    parse_bonuses, parse_effects, parse_items, parse_objects, parse_settings, parse_units,
};
use dt_lib::registry::GameInfo;
use dt_lib::units::unit::UnitInfo;
use dt_lib::units::{unit::Unit, unitstats::ModifyUnitStats};
use dt_client::{Connection, IncomingEvent};
use std::collections::HashMap;
use tokio::runtime::Runtime;

pub const SIZE: (f32, f32) = (32., 22.);
pub const CARD_SIZE: f32 = 160.;

#[derive(Debug)]
pub struct Scenario {
    pub events: Vec<GameEvent>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum ConnectionStatus {
    NotFull,
    Full(bool),
}

#[derive(Debug)]
pub struct Online {
    pub conn: Connection,
    pub status: ConnectionStatus,
    pub army: usize,
}

#[derive(Debug)]
pub enum GameVariant {
    Single(Scenario),
    Online(Online),
}

#[derive(Debug)]
pub struct Game {
    pub executor: Executor,
    pub variant: GameVariant,
    pub focus: dt_lib::battle::battlefield::BattleUnitPos,
    /// Победы подряд (для Discord Rich Presence): +1 за победу над чужой
    /// армией, сброс при поражении.
    pub recent_wins: u32,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum BlendMode {
    Off,
    Edges,
    EdgesStrong,
    Rounded,
    RoundedStrong,
    /// Точная копия оригинального редактора (notes/тайлы): наплыв
    /// половиной клетки с атласа соседа, линейный градиент альфы
    /// 68→0 от шва к середине клетки.
    Original,
}
impl BlendMode {
    pub const ALL: [BlendMode; 6] = [
        BlendMode::Off,
        BlendMode::Edges,
        BlendMode::EdgesStrong,
        BlendMode::Rounded,
        BlendMode::RoundedStrong,
        BlendMode::Original,
    ];
    pub fn next(self) -> BlendMode {
        let i = BlendMode::ALL.iter().position(|m| *m == self).unwrap();
        BlendMode::ALL[(i + 1) % BlendMode::ALL.len()]
    }
    pub fn strong(self) -> bool {
        matches!(self, BlendMode::EdgesStrong | BlendMode::RoundedStrong)
    }
    pub fn rounded(self) -> bool {
        matches!(self, BlendMode::Rounded | BlendMode::RoundedStrong)
    }
    /// Подпись для комбобокса настроек рендера.
    pub fn label(self) -> &'static str {
        match self {
            BlendMode::Off => "Off",
            BlendMode::Edges => "Edges",
            BlendMode::EdgesStrong => "Strong",
            BlendMode::Rounded => "Rounded",
            BlendMode::RoundedStrong => "Rounded Strong",
            BlendMode::Original => "Оригинал",
        }
    }
}
#[derive(Debug, Clone)]
pub struct MapRenderSettings {
    pub camera: Camera,
    /// Зум-лимиты карты (вся карта..ZOOM_IN×): false — legacy-кламп [8e-6; 0.02].
    pub zoom_limits: bool,
    pub deco_render: bool,
    pub buildings_render: bool,
    pub event_render: bool,
    pub tiles_render: bool,
    /// Слой принадлежности тайлов строениям (F9): полупрозрачная заливка
    /// hitmap.building, цвет — детерминированный HSV-хеш из индекса строения.
    pub ownership_render: bool,
    pub armies_render: bool,
    pub err_render: bool,
    pub seed: u64,
    pub decos_dirty: bool,
    pub blend_mode: BlendMode,
    pub blend_dirty: bool,
}
impl Default for MapRenderSettings {
    fn default() -> Self {
        Self {
            zoom_limits: true,
            camera: Camera::from_display_rect(0., SIZE.1 * 50., SIZE.0 * 50., -SIZE.1 * 50.),
            deco_render: true,
            buildings_render: true,
            event_render: true,
            tiles_render: true,
            armies_render: true,
            err_render: false,
            ownership_render: false,
            seed: 0,
            decos_dirty: false,
            blend_mode: BlendMode::Rounded,
            blend_dirty: false,
        }
    }
}

#[derive(Debug, Clone)]
pub enum Menu {
    Main,
    Atlas,
    Info,
    Map(MapRenderSettings),
    Message(dt_lib::map::event::Message),
    Battle,
    RoomCreation,
    BattleSetup,
    /// Лобби ПВП: список комнат (браузер, §1.1). Держится в PvpState.
    PvpLobby,
    /// Создание ПВП-комнаты: форма настроек RoomConfig (§1.3).
    PvpRoomSetup,
    /// Комната ПВП после входа (ожидание старта хостом).
    PvpRoom,
    /// Окно строения: индекс в gamemap.buildings + активная вкладка.
    Building(usize, BuildingTab),
    /// Экран редактора карт: состояние живёт в State.editor (EditorUi).
    Editor,
}

/// Состояние вкладки «Редактор карт»: проект, история команд, запечка.
/// По образцу PvpState/BuildingUi — живёт в State, не в Menu.
pub struct EditorUi {
    /// Источник истины — state.project() (снапшот editor.project удалён:
    /// запек/вьюхи читают живой state, иначе правки не отражались).
    pub history: editor_core::CommandHistory,
    pub state: editor_core::EditorState,
    pub tool: editor_core::Tool,
    /// Запечка завершена (RT залит, egui-текстура привязана).
    pub baked: bool,
    /// RT тайлового слоя (первый слой, как в игре).
    pub rt: Option<crate::gfx::Rt>,
    /// Нативная egui-текстура тайлового слоя (zero-copy сэмпл GPU-объекта).
    pub egui_tex: Option<egui::TextureId>,
    /// Последняя world-позиция кисти (непрерывный drag-paint: траектория
    /// сэмплируется шагом в полклетки — без пропусков на кривых движениях).
    pub brush_pos: Option<[f32; 2]>,
    /// RT слоя декораций/строений/армий (второй слой, как в игре).
    pub decos_rt: Option<crate::gfx::Rt>,
    /// Нативная egui-текстура слоя декораций.
    pub decos_tex: Option<egui::TextureId>,
    /// egui-текстуры маркеров (E1..E4): родной egui-пайплайн (premultiplied
    /// alpha — сырой PNG с straight-alpha через нативную регистрацию
    /// рисовался белым фоном).
    pub marker_handles: std::collections::HashMap<String, egui::TextureHandle>,
    pub cam: crate::camera::Camera,
    /// Строка статуса (последняя операция/ошибка).
    pub status: String,
    /// Путь проекта (Save/Save As).
    pub project_path: Option<std::path::PathBuf>,
    /// Счётчик имён новых объектов (декор/строения/армии).
    pub counter: usize,
    /// Кэш результата валидаторов (Lint / перед сохранением).
    pub issues: Vec<editor_validators::Issue>,
    /// Грязный флаг: документ менялся с последнего запека.
    pub bake_dirty: bool,
    /// Активный тайл в палитре (кисть).
    pub active_tile: usize,
    /// Выделенная клетка (рамка egui-пейнтера).
    pub selected: Option<(usize, usize)>,
    /// Слоты результатов rfd-диалогов (async, xdg-портал; zbus требует
    /// tokio-реактор — блокирующий FileDialog паниковал вне runtime).
    pub open_slot: Option<tokio::sync::oneshot::Receiver<std::path::PathBuf>>,
    pub save_slot: Option<tokio::sync::oneshot::Receiver<std::path::PathBuf>>,
    /// Диалог размера новой карты (ввод + слайдер 16..200).
    pub size_dialog: Option<NewMapDialog>,
    /// Выбранная декорация в палитре (индекс в registry.objects.inner).
    pub active_deco: Option<usize>,
    /// Выбранное строение в палитре (индекс в registry.objects.inner).
    pub active_building: Option<usize>,
    /// Выбранный армейский шаблон (id юнита-главы из реестра units).
    pub active_army_template: Option<usize>,
    /// Кэш egui-текстур палитры (ключ = имя ассета; load_texture по надобности).
    pub palette_tex: std::collections::HashMap<String, egui::TextureHandle>,
    pub palette_search: String,
    /// Фильтр категории декораций (первое слово имени; None = все).
    pub deco_category: Option<String>,
    /// Фильтр категории строений (первое слово имени; None = все).
    pub building_category: Option<String>,
    /// Фильтр размера объектов (макс. сторона в клетках; None = все).
    /// Строения и декорации: 1×1, 2×2, 4×3, … — фильтр «до N».
    pub deco_size_max: Option<u8>,
    pub building_size_max: Option<u8>,
    /// Фильтр типа армейских шаблонов (None = все).
    pub army_nature: Option<ArmyNature>,
    /// Выбранный элемент палитры «События»: true — фонарик
    /// (map_model=8, active, radius=3), false — точка локальных событий
    /// (map_model=9, inactive, radius=3; видна как E4/E3).
    pub active_lantern_kind: Option<bool>,
    /// Таймер повтора Ctrl+Z/Y: время последнего повтора (сек),
    /// None — удержание только началось (ждём задержку до первого повтора).
    pub hotkey_repeat_at: Option<f64>,
    /// Настройки рендера редактора (поповер «Рендер» в тулбаре).
    pub render_settings: EditorRenderSettings,
    /// Выделение интеракта (рамка + инфоокно «Свойства объекта»).
    pub selection: Option<Selection>,
    /// Перенос клик-клик: ПКМ на объекте взял, ПКМ в новой клетке положил.
    /// Ghost следует за курсором без зажатой кнопки; Esc — отмена.
    pub carrying: Option<CarriedObject>,
    /// Докинг инфоокон (п.6): egui_tiles-дерево в правой панели,
    /// каждая панель = объект (Selection). None — дерево ещё не создано.
    pub info_tree: Option<egui_tiles::Tree<Selection>>,
    /// Единое дерево экрана (п.3 ТЗ-2): палитра | карта | инфо-панели.
    /// Панель Map — канвас; пользователь может перетащить как вкладку.
    pub screen_tree: Option<egui_tiles::Tree<EditorPane>>,
    pub lantern_drag: Option<(usize, (usize, usize), (usize, usize))>,
    /// Настройки кисти (форма/размер/заливка/мульти-выбор).
    pub brush: BrushConfig,
    /// Закреплять объекты по умолчанию (галочка в интеракте).
    pub pin_by_default: bool,
    /// Закреплённые объекты (пин = панель живёт без выделения).
    pub pinned: Vec<Selection>,
    /// Фильтр цели интеракта (п.7): что object_at считает целью.
    pub interact_filter: InteractFilter,
    /// Тултип объекта (п.6 ТЗ-2): клетка, с которой начат ховер, и
    /// время начала (ctx.time). Показ после 3 с.
    pub hover_cell: Option<(usize, usize)>,
    pub hover_since: Option<f64>,
    pub hover_sel: Option<Selection>,
}

/// Задержка показа тултипа на объекте карты.
pub const TOOLTIP_DELAY: f64 = 3.0;

/// Что выбирает ЛКМ/ПКМ в интеракте.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum InteractFilter {
    /// Всё (первое попавшееся: точки → армии → строения).
    #[default]
    All,
    /// Только строения.
    Buildings,
    /// Только армии.
    Armies,
    /// Только точки событий.
    Lanterns,
}

impl InteractFilter {
    pub const ALL: [InteractFilter; 4] = [
        InteractFilter::All,
        InteractFilter::Buildings,
        InteractFilter::Armies,
        InteractFilter::Lanterns,
    ];
    pub fn label(self) -> &'static str {
        match self {
            InteractFilter::All => "Всё",
            InteractFilter::Buildings => "Строения",
            InteractFilter::Armies => "Армии",
            InteractFilter::Lanterns => "Точки",
        }
    }
    /// Пропускает ли фильтр данный Selection.
    pub fn allows(self, sel: Selection) -> bool {
        match self {
            InteractFilter::All => true,
            InteractFilter::Buildings => matches!(sel, Selection::Building(_)),
            InteractFilter::Armies => matches!(sel, Selection::Army(_)),
            InteractFilter::Lanterns => matches!(sel, Selection::Lantern(_)),
        }
    }
}

/// Панель дерева экрана: карта или инфоокно объекта.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EditorPane {
    /// Карта-канвас (центральная область).
    Map,
    /// Инфоокно объекта (id объекта в тайле).
    Info(Selection),
}

/// Инфоокно по умолчанию закрыто, закреплённое живёт параллельно.
impl EditorUi {
    /// Панель открыта, если объект выбран или закреплён.
    pub fn is_info_open(&self, sel: &Selection) -> bool {
        self.selection == Some(*sel) || self.pinned.contains(sel)
    }
}

/// Настройки кисти (п.3 ТЗ): форма и размер фигуры под курсором,
/// flood-fill для заливки, мульти-выбор палитры.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BrushConfig {
    /// Форма кисти.
    pub shape: BrushShape,
    /// Радиус кисти в клетках (1..=16; 1 = одна клетка).
    pub size: u32,
    /// Заливка: максимальный объём за клик (клеток); 0 = без лимита.
    /// Радиус достижимости: 0 = вся связная область (∞).
    pub fill_max_volume: usize,
    pub fill_max_range: usize,
    /// Мульти-выбор палитры: несколько элементов, применяются случайно
    /// или последовательно (клетки фигуры под курсором).
    pub multi_select: Vec<MultiPick>,
    /// Режим мультивыбора палитры: клик по ячейке тогглит элемент
    /// (не снимая остальные), рисует список мульти-выбора.
    pub multi_mode: bool,
    /// Порядок применения мульти-выбора.
    pub multi_order: MultiOrder,
    /// Зерно рандома мульти-выбора: hash координат клетки (детерминизм
    /// относительно позиции — одно и то же место рисует одно и то же).
    pub multi_seed_salt: u64,
}

impl Default for BrushConfig {
    fn default() -> Self {
        Self {
            shape: BrushShape::Square,
            size: 1,
            fill_max_volume: 0,
            fill_max_range: 0,
            multi_select: Vec::new(),
            multi_mode: false,
            multi_order: MultiOrder::Random,
            multi_seed_salt: 0,
        }
    }
}

/// Форма кисти: круг (диск), кольцо, квадрат, периметр квадрата.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BrushShape {
    /// Заполненный диск: dx² + dy² ≤ r².
    Circle,
    /// Кольцо: внешний радиус = size, толщина 1 клетка.
    Ring,
    /// Заполненный квадрат (2r-1)×(2r-1).
    Square,
    /// Периметр квадрата (рамка толщиной 1).
    Perimeter,
}

impl BrushShape {
    pub const ALL: [BrushShape; 4] = [
        BrushShape::Circle,
        BrushShape::Ring,
        BrushShape::Square,
        BrushShape::Perimeter,
    ];
    pub fn label(self) -> &'static str {
        match self {
            BrushShape::Circle => "Круг",
            BrushShape::Ring => "Кольцо",
            BrushShape::Square => "Квадрат",
            BrushShape::Perimeter => "Периметр",
        }
    }
}

/// Порядок применения мульти-выбора палитры.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MultiOrder {
    /// Случайно (зерно = hash координат клетки).
    Random,
    /// Последовательно (по кругу).
    Sequential,
}

impl MultiOrder {
    pub const ALL: [MultiOrder; 2] = [MultiOrder::Random, MultiOrder::Sequential];
    pub fn label(self) -> &'static str {
        match self {
            MultiOrder::Random => "Случайно",
            MultiOrder::Sequential => "Последовательно",
        }
    }
}

/// Выбранный элемент мульти-палитры: элемент инструмента.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MultiPick {
    /// Тайл (индекс TILES).
    Tile(usize),
    /// Декорация (индекс registry.objects.inner).
    Deco(usize),
    /// Строение (индекс registry.objects.inner).
    Building(usize),
    /// Армия (id юнита-шаблона).
    Army(usize),
}

/// Настройки рендера редактора: слои и режимы вкладки «Рендер».
#[derive(Debug, Clone)]
pub struct EditorRenderSettings {
    /// Маркеры E1 (неактивные армии), E2/E4 (фонарики).
    pub markers: bool,
    /// Сетка канваса.
    pub grid: bool,
    /// Заливка принадлежности тайлов строениям (цвет — HSV-хеш строения).
    pub ownership: bool,
    /// Режим наплывов тайлов (Off/Strong/Rounded и т.д.).
    pub blend_mode: BlendMode,
    /// Рисование армий (модельки/корабли в слое декораций).
    pub armies: bool,
    /// Рисование строений в слое декораций.
    pub buildings: bool,
}

impl Default for EditorRenderSettings {
    fn default() -> Self {
        Self {
            markers: true,
            grid: true,
            ownership: false,
            blend_mode: BlendMode::Rounded,
            armies: true,
            buildings: true,
        }
    }
}

/// Выделенный объект интеракта (рамка на канвасе + инфоокно).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Selection {
    /// Армия (индекс в map.armys).
    Army(usize),
    /// Строение (индекс в map.buildings).
    Building(usize),
    /// Точка событий/фонарик (индекс в lanterns).
    Lantern(usize),
    /// Событие карты (индекс в project.events) — открыто из селектора
    /// (goto-definition); инфо-панель read-only просмотр.
    Event(usize),
}

/// Объект в режиме переноса (клик-клик ПКМ).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CarriedObject {
    pub kind: Selection,
    /// Исходная клетка (для команды Move* при фиксации).
    pub from: (usize, usize),
}

/// Тип армейского шаблона для палитры (4 базовых фракции старого
/// редактора). Это UI-категория, а не UnitType: в Units.ini Крестьянин
/// и Бандит оба Nature=Rogue — различаем по характерному юниту.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArmyNature {
    /// Феодальный (рыцарь).
    Feudal,
    /// Разбойник (бандит).
    Rogue,
    /// Деревенщина (крестьянин).
    Peasant,
    /// Нежить (мертвяк/зомби).
    Undead,
}

impl ArmyNature {
    /// Русская подпись для фильтра палитры.
    pub fn label(self) -> &'static str {
        match self {
            ArmyNature::Feudal => "Феодальный",
            ArmyNature::Rogue => "Разбойник",
            ArmyNature::Peasant => "Деревенщина",
            ArmyNature::Undead => "Нежить",
        }
    }


    /// Имя характерного юнита-шаблона (Units.ini, поле Name): феодал →
    /// рыцарь, разбойник → бандит, деревенщина → крестьянин, нежить →
    /// мертвяк. id разрешается по реестру units в editor_view.
    pub fn template_unit_name(self) -> &'static str {
        match self {
            ArmyNature::Feudal => "Рыцарь",
            ArmyNature::Rogue => "Бандит",
            ArmyNature::Peasant => "Крестьянин",
            ArmyNature::Undead => "Мертвяк",
        }
    }
    /// Все типы в порядке палитры.
    pub const ALL: [ArmyNature; 4] = [
        ArmyNature::Feudal,
        ArmyNature::Rogue,
        ArmyNature::Peasant,
        ArmyNature::Undead,
    ];
}

/// Диалог «Новая карта»: выбранный размер (16..200, по умолчанию 50).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NewMapDialog {
    pub size: usize,
}

impl std::fmt::Debug for EditorUi {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EditorUi")
            .field("size", &self.state.project().size())
            .field("undo", &self.history.undo_len())
            .field("tool", &self.tool)
            .field("path", &self.project_path)
            .finish()
    }
}

impl Default for EditorUi {
    fn default() -> Self {
        Self {
            history: editor_core::CommandHistory::new(),
            brush_pos: None,
            state: editor_core::EditorState::new(editor_core::MapProject::new(50, 0)),
            tool: editor_core::Tool::default(),
            baked: false,
            decos_rt: None,
            decos_tex: None,
            egui_tex: None,
            marker_handles: std::collections::HashMap::new(),
            rt: None,
            cam: crate::camera::Camera::from_display_rect(
                0.,
                SIZE.1 * 50.,
                SIZE.0 * 50.,
                -SIZE.1 * 50.,
            ),
            status: String::new(),
            project_path: None,
            issues: Vec::new(),
            bake_dirty: true,
            counter: 0,
            active_tile: 0,
            selected: None,
            open_slot: None,
            save_slot: None,
            size_dialog: None,
            active_deco: None,
            active_building: None,
            active_army_template: None,
            palette_search: String::new(),
            deco_category: None,
            building_category: None,
            army_nature: None,
            palette_tex: std::collections::HashMap::new(),
            hotkey_repeat_at: None,
            render_settings: EditorRenderSettings::default(),
            active_lantern_kind: None,
            lantern_drag: None,
            selection: None,
            carrying: None,
            brush: BrushConfig::default(),
            info_tree: None,
            screen_tree: None,
            pin_by_default: false,
            pinned: Vec::new(),
            interact_filter: InteractFilter::default(),
            hover_cell: None,
            hover_since: None,
            hover_sel: None,
            deco_size_max: None,
            building_size_max: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuildingTab {
    Main,
    Market,
    Recruits,
    Spells,
}

/// Состояние окна строения: выделения списков рынка, карта статов, лог сделки.
#[derive(Debug, Default)]
pub struct BuildingUi {
    /// Выбранные к покупке (индексы market.items).
    pub market_pick: Vec<usize>,
    /// Выбранные к продаже (позиции army.inventory).
    pub player_pick: Vec<usize>,
    /// Hover/выбранный юнит найма для карты статов (юнит из реестра).
    pub inspect_unit: Option<usize>,
    /// Выбранный юнит в армии для клик-мува (позиция слота).
    pub drag_from: Option<usize>,
    /// Результат последней сделки.
    pub deal_log: Vec<String>,
    /// Последний клик ЛКМ на карте (время, тайл) — дабл-клик по строению.
    pub last_click: Option<(f64, [usize; 2])>,
    /// Дабл-клик по строению вдали: армия идёт туда, окно открыть по прибытии.
    pub pending_open: Option<usize>,
    /// Тайл последнего клика GoTo — подсветка цели на карте.
    pub goto_tile: Option<[usize; 2]>,
    /// Строение под курсором/выбранное ЛКМ — зелёная обводка.
    pub lmb_building: Option<usize>,
    /// ПКМ-окно информации о строении: индекс в gamemap.buildings.
    pub building_info_open: Option<usize>,
    /// Тайл, где открыто ПКМ-окно (привязка позиции попапа).
    pub building_info_tile: [usize; 2],
}

/// Порт struct Ui (quad_ui main.rs:531): текущее меню, UI-камера, стек.
#[derive(Debug)]
pub struct UiState {
    pub main: Menu,
    pub camera: Camera,
    pub stack: Vec<Menu>,
}

#[derive(Debug)]
pub struct RenderTextures {
    pub map: TexId,
    pub decos: TexId,
}

/// Состояние ПВП-лобби (Этапы 1-2: локальный мок RoomManager; реальный
/// websocket-транспорт комнат — следующий спринт).
#[derive(Debug)]
pub struct PvpState {
    /// Локальный ник игрока (аккаунты — фаза 4).
    pub nick: String,
    /// Локальный реестр комнат (мок сервера).
    pub manager: RoomManager,
    /// Таб-фильтр лобби: 0 = все, 1 = битвы, 2 = карты.
    pub lobby_tab: usize,
    /// Чекбокс «только открытые» (есть места и не InGame/Finished).
    pub open_only: bool,
    /// Черновик конфига комнаты на экране создания.
    pub setup_draft: RoomConfig,
    /// Комната, в которой находимся (id), и вид.
    pub joined: Option<(RoomId, RoomView)>,
    /// Последняя ошибка для отображения (сбрасывается при следующем действии).
    pub error: Option<String>,
}

impl Default for PvpState {
    fn default() -> Self {
        Self {
            nick: "Игрок".into(),
            manager: RoomManager::new(),
            lobby_tab: 0,
            open_only: false,
            setup_draft: RoomConfig::default(),
            joined: None,
            error: None,
        }
    }
}

pub struct State {
    pub assets: Assets,
    pub registry: GameInfo,
    pub textures: RenderTextures,
    pub game: Game,
    pub ui: UiState,
    /// Состояние окна строения (выделения рынка, лог сделки, дабл-клик).
    pub building_ui: BuildingUi,
    /// ПВП-лобби: локальный ник, фильтры, мок RoomManager до сетевого слоя.
    pub pvp: PvpState,
    /// Редактор карт: проект, история, запечка (экран Menu::Editor).
    pub editor: EditorUi,
    pub delta: f32,
    /// Discord Rich Presence (клиент None, если Discord не запущен).
    pub rpc: crate::rich_presence::RichPresence,
    pub rt: Runtime,
    /// Пиксельные снапшоты тайлов (TILES), для генерации наплывов.
    pub tile_pixels: Vec<image::RgbaImage>,
}

// ---------------- Хелперы юнитов (quad_ui main.rs:696-715) ----------------

pub fn get_unit_texture(assets: &Assets, unit: &Unit, units: &dt_lib::registry::Units) -> TexId {
    assets.get(&format!("unit_{}.png", unit.get_info(units).icon_index - 1))
}

pub fn get_unit_info_texture(assets: &Assets, unit: &UnitInfo) -> TexId {
    assets.get(&format!("unit_{}.png", unit.icon_index - 1))
}

pub fn get_troop_texture(
    assets: &Assets,
    armies: &Vec<Army>,
    unit: usize,
    army: usize,
    units: &dt_lib::registry::Units,
) -> Option<TexId> {
    armies.get(army).and_then(|x| {
        x.troops.get(unit).and_then(|tr| {
            let troop = tr.get();
            Some(assets.get(&format!(
                "unit_{}.png",
                troop.unit.get_info(units).icon_index - 1
            )))
        })
    })
}

// ---------------- load_map / game_init (quad_ui main.rs:139-384) ----------------

pub fn load_map(map: &str, registry: &GameInfo) -> (GameMap, Events) {
    let bytes = crate::files::read_file_bytes(&format!("Maps_Rus/{map}"));
    let (mut gamemap, events) = convert_dtm_map(parse_dtm_vec(bytes).unwrap(), registry);
    gamemap.calc_hitboxes(&registry.objects.inner);
    if gamemap.armys.is_empty() {
        gamemap.armys.push(Army::new(
            vec![SendMut::new(Troop::new(
                (registry.units.inner[0].clone(), &registry.bonuses).into(),
            ))],
            ArmyStats {
                gold: 0,
                army_name: "".into(),
                mana: 0,
            },
            vec![],
            (1, 1),
            true,
            Control::Player(0),
            registry,
        ));
    }
    (gamemap, events)
}

pub async fn game_init(gfx: &mut Gfx, text: &mut TextRenderer) -> State {
    let mut registry = GameInfo::new();
    let settings = parse_settings::<DesktopFiles>().await;
    let mut locale = Locale::new("Rus".into(), "Eng".into());
    {
        locale.set_lang((&settings.locale, &settings.additional_locale));
        parse_locale::<DesktopFiles>(&[&settings.locale, &settings.additional_locale], &mut locale).await;
    }
    registry.locale = locale;
    parse_bonuses::<DesktopFiles>(None, &mut registry).await;
    parse_effects::<DesktopFiles>(None, &mut registry).await;
    let fonts = load_fonts(text);
    let assets = {
        let req_assets_items =
            parse_items::<DesktopFiles>(None, &settings.locale, &mut registry).await;
        let mut res = parse_units::<DesktopFiles>(Some("Units.ini"), &mut registry).await;
        if let Ok(res) = &mut res {
            res.0 = "assets/Icons";
        }
        if let Err(err) = res {
            panic!("{}", err);
        }
        let Ok(req_assets_units) = res else {
            panic!("Unit parsing error")
        };
        let req_assets_objects = parse_objects::<DesktopFiles>(&mut registry).await;
        let req_assets_windows = (
            "assets/Window",
            [
                "front.png",
                "backyard.png",
                "tent.png",
                "undercell.png",
                "melee.png",
                "ranged.png",
                "buff.png",
                "debuff.png",
                "button.png",
                "buttonblue.png",
                "cursor.png",
                "Menu.png",
                "Paper.png",
                "gold.png",
                "red.png",
                // Рестайл окна строений/боя (ассеты dt/assets/Window).
                "Btn1Up.png",
                "Btn1Down.png",
                "CloseButtonRed-Up.png",
                "CloseButtonRed-Down2.png",
                "CloseButtonGreen-Up.png",
                "Corner_Frame-LU.png",
                "Corner_Frame-RU.png",
                "Corner_Frame-LD.png",
                "Corner_Frame-RD.png",
                "Win-marble.png",
                "Win-red.png",
                "WinLong.png",
                "QuestBorder.png",
                "DownCorner_alpha.png",
                "wb1.png",
                "wb2.png",
                "wb3.png",
                "S_Town.png",
                "S_Village.png",
                "S_Castle.png",
                "S_Market.png",
                "S_Tavern.png",
                "S_Church.png",
                "S_Shipyard.png",
                "S_Ruin.png",
            ]
            .map(|x| x.to_owned())
            .to_vec(),
        );
        let req_assets_stats_list = (
            "assets/StatsIcons",
            [
                "crossed-swords.png",
                "hearts.png",
                "high-shot.png",
                "sprint.png",
                "mounted-knight.png",
                "breastplate.png",
                "wizard-staff.png",
                "heart-bottle.png",
                "spartan.png",
                "small-fire.png",
                "checked-shield.png",
                "raise-zombie.png",
                "poison-bottle.png",
            ]
            .map(|x| x.to_owned())
            .to_vec(),
        );
        let req_assets_terrain_list = (
            "assets/Terrain",
            [
                "Badground.png",
                "DeepSwamp.png",
                "DeepWater.png",
                "Desert.png",
                "Detail.png",
                "Dust.png",
                "FlameLand.png",
                "Land.png",
                "LowLand.png",
                "Plain.png",
                "Road.png",
                "Rock.png",
                "Shallow.png",
                "Snow.png",
                "Swamp.png",
                "Water.png",
            ]
            .map(|x| x.to_owned())
            .to_vec(),
        );
        let req_assets_armies_list = (
            "assets/Armies",
            [
                "феодал",
                "ГГрыцарь",
                "ГГархимаг",
                "ГГследопыт",
                "крестьянин",
                "разбойник",
                "мертвяк",
                "некромант",
                "призрак",
            ]
            .map(|x| {
                [x].repeat(64)
                    .iter()
                    .enumerate()
                    .map(|x| format!("{}/Unit_{}.png", x.1, x.0))
                    .collect::<Vec<_>>()
            })
            .iter_mut()
            .fold(vec![], |mut acc, x| {
                acc.append(x);
                acc
            })
            .to_vec(),
        );
        // Корабли на воде (assets_editor): SHIP1 феодальный, SHIP2 бандиты,
        // SHIP3 торговый — рисуются в RT-слое bake. Маркеры E1-E4 грузятся
        /// egui-пайплайном с диска (editor_view), не как gfx-текстуры.
        let req_assets_editor_list = (
            "assets_editor",
            ["SHIP1.png", "SHIP2.png", "SHIP3.png"]
                .map(|x| x.to_string())
                .to_vec(),
        );
        let req_assets_list = [
            req_assets_objects,
            req_assets_items,
            req_assets_units,
            req_assets_terrain_list,
            req_assets_windows,
            req_assets_stats_list,
            req_assets_armies_list,
            req_assets_editor_list,
        ];
        load_assets(gfx, text, &req_assets_list, fonts).await
    };
    // Снапшоты тайлов: декодируем спрайты TILES напрямую (замена
    // get_texture_data снапшотов до build_textures_atlas — атласа больше нет).
    let tile_pixels = dt_lib::map::tile::TILES
        .iter()
        .map(|tile| {
            image::load_from_memory(&crate::files::read_file_bytes(&format!(
                "assets/Terrain/{}",
                tile.sprite()
            )))
            .expect("tile sprite decode failed")
            .to_rgba8()
        })
        .collect::<Vec<_>>();
    let map = "Stinger-Paramount_War_HARD.dtm";
    let (mut gamemap, events) = load_map(map, &registry);
    let mut executor = Executor {
        gamemap: gamemap.clone(),
        events: events.clone(),
        battle: None,
        execution_queue: vec![],
        players: vec![Player {
            army: gamemap.armys.len() - 1,
            questbook: None,
            execution_queue: vec![],
            wait_until: None,
        }],
        last_day: gamemap.time.get_days(),
    };
    executor.tick(&registry);
    gamemap.calc_hitboxes(&registry.objects.inner);
    let rt = tokio::runtime::Runtime::new().unwrap();
    let camera = Camera::from_display_rect(0., 1080., 1920., -1080.);
    let game = Game {
        executor,
        focus: dt_lib::battle::battlefield::BattleUnitPos { army: 0, pos: 0 },
        variant: GameVariant::Single(Scenario { events }),
        recent_wins: 0,
    };
    let _ = BENGUIAT;
    State {
        registry,
        rt,
        delta: 0.,
        assets,
        textures: RenderTextures {
            map: TexId(0),
            decos: TexId(0),
        },
        ui: UiState {
            main: Menu::Main,
            camera,
            stack: Vec::new(),
        },
        game,
        tile_pixels,
        building_ui: BuildingUi::default(),
        editor: EditorUi::default(),
        pvp: PvpState::default(),
        rpc: crate::rich_presence::RichPresence::new(),
    }
}

/// Порт process_event (quad_ui main.rs:2105).
pub fn process_event(game: &mut Game, event: IncomingEvent) {
    let GameVariant::Online(conn) = &mut game.variant else {
        return;
    };
    match event {
        IncomingEvent::Id(army) => {
            conn.army = army;
        }
        IncomingEvent::Game(g) => {
            game.executor.gamemap.armys = g.0;
            game.executor.battle = Some(g.1);
        }
        IncomingEvent::Info(text) => match &*text {
            "Wait for another player" => {
                conn.status = ConnectionStatus::NotFull;
            }
            "Room full" => {
                conn.status = ConnectionStatus::Full(false);
            }
            _ => {}
        },
        IncomingEvent::Acceptance(a) => {
            if a == [true, true] {
                conn.status = ConnectionStatus::Full(true);
            }
        }
    }
}

/// Порт screen_center (quad_ui main.rs:562).
pub fn screen_center(input: &crate::ui::InputState) -> [f32; 2] {
    [input.screen_width() / 2., input.screen_height() / 2.]
}

/// Ввод строки для editbox (порт macroquad ui.input_text семантики).
pub fn collect_edit_input(input: &crate::ui::InputState, buf: &mut String) {
    for ch in &input.chars {
        if !ch.is_control() {
            buf.push(*ch);
        }
    }
    if input.key_down(winit::keyboard::KeyCode::Backspace) {
        buf.pop();
    }
}


