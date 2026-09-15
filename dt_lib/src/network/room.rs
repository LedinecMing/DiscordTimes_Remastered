//! Комнатный ПВП-слой (Этапы 1-2 плана notes/PVP_TECH_PLAN.md):
//! serde-DTO (RoomConfig/PvpRoomSummary), RoomManager (HashMap<RoomId, Room>),
//! Транспорт: ws-кадры Text = serde_json(ClientRoomMsg/ServerRoomMsg);
//! Binary = alkahest(Incoming/Outcoming) боя. renet (net.rs) — легаси.
//! Комнатные сообщения (§1.1): serde_json внутри транспортных вариантов
//! ClientMessage::Room/ServerMessage::Room (ренет-обёртка, легаси).

use crate::{
    battle::army::Army,
    registry::{GameInfo, UnitId},
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Идентификатор комнаты (u64 — серверный счётчик; uuid в фазе аккаунтов).
pub type RoomId = u64;

// ---------------------------------------------------------------------------
// DTO
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum RoomMode {
    /// Чистая битва (2 игрока).
    Battle,
    /// Полноценная игра на карте (executor).
    MapGame,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum RoomStatus {
    Lobby,
    InGame,
    Finished,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum SpectatorPolicy {
    Allowed,
    Forbidden,
}

/// Краткая публичная информация об аккаунте (фаза 4 добавит поля).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AccountBrief {
    pub display_name: String,
}

impl AccountBrief {
    pub fn new(name: &str) -> Self {
        Self {
            display_name: name.to_owned(),
        }
    }
}

/// Строка таблицы комнат (ServerRoomMsg::RoomList).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PvpRoomSummary {
    pub id: RoomId,
    pub title: String,
    pub mode: RoomMode,
    /// Для MapGame (например «Проклятое озеро»).
    pub map: Option<String>,
    pub rules_preset: Option<String>,
    pub rated: bool,
    pub players: u8,
    pub players_max: u8,
    pub spectators: u8,
    pub spectators_allowed: bool,
    pub status: RoomStatus,
    pub host: AccountBrief,
}

/// Тип ограничения состава армии.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum LimitKind {
    /// Бюджет: сумма cost_hire <= gold_per_side.
    #[default]
    Gold,
    /// Поинты: цена юнита из points_table (дефолт = cost_hire).
    Points,
    /// Без лимитов.
    None,
}

/// «Выбор модификации игровой» (§5, идеи 6.1).
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum GameModification {
    #[default]
    None,
    /// Армии противника скрыты до старта.
    FogOfWarDraft,
    /// Случайная +1 инициатива одной из армий (= coin_flip_initiative).
    RandomIniBonus,
    /// Оба играют одинаковыми наборами.
    MirrorMatch,
    /// Бюджет скрыт до старта.
    BlindGold,
    Custom(String),
}

/// Правила заклинаний: покупка в сетапе и применение в бою — раздельно.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpellRules {
    pub purchase: bool,
    pub use_in_battle: bool,
}

impl Default for SpellRules {
    fn default() -> Self {
        Self {
            purchase: true,
            use_in_battle: true,
        }
    }
}

/// Совместный драфт-бан (порядок ABBA фиксируется позднее, bans уже здесь).
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DraftRules {
    /// Баны стороны 0 и стороны 1; юнит в любом списке запрещён обоим.
    pub bans: [Vec<UnitId>; 2],
}

/// Подкрепления: покупка в процессе (появляются в Field::Reserve).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReinforcementRules {
    #[default]
    Disabled,
    BuyOnly,
    FreeWave,
}

/// Полный конфиг комнаты (сохраняется как пресет правил, §1.3/§1.4).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RoomConfig {
    pub title: String,
    pub rated: bool,
    pub mode: RoomMode,
    pub spectators: SpectatorPolicy,
    pub rules_preset_id: Option<String>,
    /// Карта для MapGame.
    pub map: Option<String>,
    /// Бюджет на сторону (LimitKind::Gold).
    pub gold_per_side: u64,
    /// Покупка артефактов в сетапе.
    pub allow_items: bool,
    pub allow_spells: SpellRules,
    pub game_modification: GameModification,
    pub draft: DraftRules,
    /// Случайная +1 инициатива одной из армий на старте боя.
    pub coin_flip_initiative: bool,
    /// Лимит юнитов на армию.
    pub max_units: Option<u8>,
    /// Лимит магов (UnitInfo.magic_type.is_some()).
    pub max_mages: Option<u8>,
    pub reinforcements: ReinforcementRules,
    /// Полный запрет юнитов (сверх драфт-банов).
    pub banlist_units: Vec<UnitId>,
    /// Полный запрет предметов (Item.index).
    pub banlist_items: Vec<usize>,
    pub limit_kind: LimitKind,
    /// Цены поинтовой системы; отсутствующий юнит = cost_hire.
    pub points_table: Option<HashMap<UnitId, u64>>,
}

impl Default for RoomConfig {
    fn default() -> Self {
        Self {
            title: "Новая комната".into(),
            rated: false,
            mode: RoomMode::Battle,
            spectators: SpectatorPolicy::Allowed,
            rules_preset_id: None,
            map: None,
            gold_per_side: 1000,
            allow_items: true,
            allow_spells: SpellRules::default(),
            game_modification: GameModification::None,
            draft: DraftRules::default(),
            coin_flip_initiative: false,
            max_units: None,
            max_mages: None,
            reinforcements: ReinforcementRules::Disabled,
            banlist_units: vec![],
            banlist_items: vec![],
            limit_kind: LimitKind::Gold,
            points_table: None,
        }
    }
}

/// Нарушение правила армией: индекс юнита (для подсветки карточки) + текст.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RuleViolation {
    /// Индекс в army.troops, если нарушение локально у юнита.
    pub unit: Option<usize>,
    pub message: String,
}

impl RoomConfig {
    /// Максимум игроков-участников (Battle = 2; MapGame N>2 — фаза 6).
    pub fn players_max(&self) -> u8 {
        match self.mode {
            RoomMode::Battle => 2,
            RoomMode::MapGame => 2,
        }
    }

    /// Множество запрещённых юнитов: banlist + оба драфт-бана.
    pub fn banned_units(&self) -> Vec<UnitId> {
        let mut banned = self.banlist_units.clone();
        banned.extend(self.draft.bans[0].iter().copied());
        banned.extend(self.draft.bans[1].iter().copied());
        banned.sort_unstable();
        banned.dedup();
        banned
    }

    /// Валидация конфига (экран создания подсвечивает ошибки on-change).
    pub fn validate(&self) -> Result<(), String> {
        if self.title.trim().is_empty() {
            return Err("Название комнаты не может быть пустым".into());
        }
        match self.limit_kind {
            LimitKind::Gold if self.gold_per_side == 0 => {
                return Err("Бюджет (золото на сторону) должен быть больше нуля".into())
            }
            LimitKind::Points if self.points_table.is_none() => {
                return Err("Поинтовая система требует таблицу цен (points_table)".into())
            }
            _ => {}
        }
        if self.max_units == Some(0) {
            return Err("Лимит юнитов должен быть больше нуля".into());
        }
        // Пересечение банов сторон: один и тот же юнит не может быть забанен
        // дважды в драфте ABBA.
        let (a, b) = (&self.draft.bans[0], &self.draft.bans[1]);
        if a.iter().any(|id| b.contains(id)) {
            return Err("Один и тот же юнит не может быть забанен обеими сторонами".into());
        }
        Ok(())
    }

    /// Единая точка валидации армии против правил комнаты (§1.3): её же
    /// использует валидатор UI сетапа. Guard'ы SendMut — короткие, снапшоты.
    pub fn check_army(&self, army: &Army, registry: &GameInfo) -> Vec<RuleViolation> {
        let mut violations = Vec::new();
        let banned = self.banned_units();
        let mut spent: u64 = 0;
        let mut mages: usize = 0;
        for (i, troop) in army.troops.iter().enumerate() {
            let snapshot = {
                let t = troop.get();
                let info = t.unit.get_info(&registry.units);
                (t.unit.id, info.magic_type.is_some(), info.cost_hire)
            };
            let (unit_id, is_mage, cost_hire) = snapshot;
            if banned.contains(&unit_id) {
                violations.push(RuleViolation {
                    unit: Some(i),
                    message: format!("Юнит {} запрещён правилами комнаты", unit_id),
                });
            }
            if is_mage {
                mages += 1;
            }
            spent += match self.limit_kind {
                LimitKind::Gold => cost_hire,
                LimitKind::Points => self
                    .points_table
                    .as_ref()
                    .and_then(|t| t.get(&unit_id).copied())
                    .unwrap_or(cost_hire),
                LimitKind::None => 0,
            };
        }
        if let Some(max) = self.max_units {
            if army.troops.len() > max as usize {
                violations.push(RuleViolation {
                    unit: None,
                    message: format!(
                        "Слишком много юнитов: {} (лимит {})",
                        army.troops.len(),
                        max
                    ),
                });
            }
        }
        if let Some(max) = self.max_mages {
            if mages > max as usize {
                violations.push(RuleViolation {
                    unit: None,
                    message: format!("Слишком много магов: {} (лимит {})", mages, max),
                });
            }
        }
        match self.limit_kind {
            LimitKind::Gold if spent > self.gold_per_side => violations.push(RuleViolation {
                unit: None,
                message: format!(
                    "Превышен бюджет: {} из {} золота",
                    spent, self.gold_per_side
                ),
            }),
            LimitKind::Points if spent > self.gold_per_side => violations.push(RuleViolation {
                unit: None,
                message: format!(
                    "Превышен лимит поинтов: {} из {}",
                    spent, self.gold_per_side
                ),
            }),
            _ => {}
        }
        violations
    }
}

/// Вид комнаты целиком (ServerRoomMsg::JoinedRoom).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RoomView {
    pub summary: PvpRoomSummary,
    pub config: RoomConfig,
    pub players: Vec<String>,
    pub spectators: Vec<String>,
}

// ---------------------------------------------------------------------------
// RoomManager
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RoomError {
    RoomNotFound,
    /// Нет места для игрока.
    Full,
    SpectatorsForbidden,
    AlreadyInRoom,
    NotInRoom,
    /// Действие требует прав хоста.
    NotHost,
    DuplicateName,
    EmptyName,
    InvalidConfig(String),
    /// Битву нельзя стартовать/покинуть в текущем статусе.
    WrongStatus,
    /// Для старта битвы недостаточно игроков.
    NotEnoughPlayers,
}

impl std::fmt::Display for RoomError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let text = match self {
            RoomError::RoomNotFound => "Комната не найдена",
            RoomError::Full => "В комнате нет мест",
            RoomError::SpectatorsForbidden => "Наблюдатели запрещены в этой комнате",
            RoomError::AlreadyInRoom => "Вы уже в этой комнате",
            RoomError::NotInRoom => "Вы не состоите в этой комнате",
            RoomError::NotHost => "Только хост может это сделать",
            RoomError::DuplicateName => "Это имя уже занято в комнате",
            RoomError::EmptyName => "Введите имя игрока",
            RoomError::InvalidConfig(msg) => return write!(f, "Некорректный конфиг: {msg}"),
            RoomError::WrongStatus => "Комната в другом статусе",
            RoomError::NotEnoughPlayers => "Недостаточно игроков для старта",
        };
        f.write_str(text)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Room {
    pub id: RoomId,
    pub config: RoomConfig,
    /// Имя хоста (всегда присутствует среди players, пока комната жива).
    pub host: String,
    pub players: Vec<String>,
    pub spectators: Vec<String>,
    pub status: RoomStatus,
}

impl Room {
    /// Игрок или наблюдатель с таким именем уже в комнате.
    pub fn is_member(&self, name: &str) -> bool {
        self.players.iter().any(|p| p == name) || self.spectators.iter().any(|p| p == name)
    }

    pub fn summary_pub(&self) -> PvpRoomSummary {
        PvpRoomSummary {
            id: self.id,
            title: self.config.title.clone(),
            mode: self.config.mode,
            map: self.config.map.clone(),
            rules_preset: self.config.rules_preset_id.clone(),
            rated: self.config.rated,
            players: self.players.len() as u8,
            players_max: self.config.players_max(),
            spectators: self.spectators.len() as u8,
            spectators_allowed: self.config.spectators == SpectatorPolicy::Allowed,
            status: self.status,
            host: AccountBrief::new(&self.host),
        }
    }

    fn view(&self) -> RoomView {
        RoomView {
            summary: self.summary_pub(),
            config: self.config.clone(),
            players: self.players.clone(),
            spectators: self.spectators.clone(),
        }
    }
}

/// Серверный реестр комнат. Один инстанс на процесс (в моке — на клиент).
#[derive(Debug, Default)]
pub struct RoomManager {
    next_id: RoomId,
    rooms: HashMap<RoomId, Room>,
}

impl RoomManager {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn create(&mut self, config: RoomConfig, host: &str) -> Result<RoomId, RoomError> {
        let host = host.trim();
        if host.is_empty() {
            return Err(RoomError::EmptyName);
        }
        config.validate().map_err(RoomError::InvalidConfig)?;
        let id = self.next_id;
        self.next_id += 1;
        self.rooms.insert(
            id,
            Room {
                id,
                config,
                host: host.to_owned(),
                players: vec![host.to_owned()],
                spectators: vec![],
                status: RoomStatus::Lobby,
            },
        );
        Ok(id)
    }

    /// Список комнат, отсортированный по id (стабильный порядок для UI).
    pub fn list(&self) -> Vec<PvpRoomSummary> {
        let mut ids: Vec<_> = self.rooms.keys().copied().collect();
        ids.sort_unstable();
        ids.into_iter()
            .filter_map(|id| self.rooms.get(&id).map(Room::summary_pub))
            .collect()
    }

    pub fn get(&self, id: RoomId) -> Option<&Room> {
        self.rooms.get(&id)
    }

    pub fn view(&self, id: RoomId) -> Result<RoomView, RoomError> {
        self.rooms.get(&id).map(Room::view).ok_or(RoomError::RoomNotFound)
    }

    pub fn join(
        &mut self,
        id: RoomId,
        name: &str,
        as_spectator: bool,
    ) -> Result<RoomView, RoomError> {
        let name = name.trim();
        if name.is_empty() {
            return Err(RoomError::EmptyName);
        }
        let room = self.rooms.get_mut(&id).ok_or(RoomError::RoomNotFound)?;
        if room.status == RoomStatus::Finished {
            return Err(RoomError::WrongStatus);
        }
        if room.is_member(name) {
            return Err(RoomError::AlreadyInRoom);
        }
        if as_spectator {
            if room.config.spectators == SpectatorPolicy::Forbidden {
                return Err(RoomError::SpectatorsForbidden);
            }
            room.spectators.push(name.to_owned());
        } else {
            // Игроки входят только в лобби (досад в бою — фаза матчмейкинга).
            if room.status != RoomStatus::Lobby {
                return Err(RoomError::WrongStatus);
            }
            if room.players.len() >= room.config.players_max() as usize {
                return Err(RoomError::Full);
            }
            room.players.push(name.to_owned());
        }
        Ok(room.view())
    }

    /// Выход из комнаты. Если комнату покинул последний участник — она
    /// закрывается (Ok(None)); если хост — хостом становится следующий игрок.
    pub fn leave(&mut self, id: RoomId, name: &str) -> Result<Option<PvpRoomSummary>, RoomError> {
        let room = self.rooms.get_mut(&id).ok_or(RoomError::RoomNotFound)?;
        if let Some(pos) = room.players.iter().position(|p| p == name) {
            room.players.remove(pos);
            if room.players.is_empty() {
                self.rooms.remove(&id);
                return Ok(None);
            }
            if room.host == name {
                room.host = room.players[0].clone();
            }
        } else if let Some(pos) = room.spectators.iter().position(|p| p == name) {
            room.spectators.remove(pos);
        } else {
            return Err(RoomError::NotInRoom);
        }
        Ok(Some(room.summary_pub()))
    }

    /// Кик игрока/наблюдателя хостом.
    pub fn kick(
        &mut self,
        id: RoomId,
        by: &str,
        member: &str,
    ) -> Result<PvpRoomSummary, RoomError> {
        let room = self.rooms.get_mut(&id).ok_or(RoomError::RoomNotFound)?;
        if room.host != by {
            return Err(RoomError::NotHost);
        }
        if member == by {
            return Err(RoomError::NotInRoom);
        }
        if let Some(pos) = room.players.iter().position(|p| p == member) {
            room.players.remove(pos);
        } else if let Some(pos) = room.spectators.iter().position(|p| p == member) {
            room.spectators.remove(pos);
        } else {
            return Err(RoomError::NotInRoom);
        }
        Ok(room.summary_pub())
    }

    /// Старт битвы хостом: Lobby -> InGame.
    pub fn start(&mut self, id: RoomId, by: &str) -> Result<(), RoomError> {
        let room = self.rooms.get_mut(&id).ok_or(RoomError::RoomNotFound)?;
        if room.host != by {
            return Err(RoomError::NotHost);
        }
        if room.status != RoomStatus::Lobby {
            return Err(RoomError::WrongStatus);
        }
        let needed = match room.config.mode {
            RoomMode::Battle => room.config.players_max() as usize,
            RoomMode::MapGame => 2,
        };
        if room.players.len() < needed {
            return Err(RoomError::NotEnoughPlayers);
        }
        room.status = RoomStatus::InGame;
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Комнатные сообщения протокола (§1.1)
// ---------------------------------------------------------------------------

/// Комнатная половина клиентского протокола. Транспортируется как
/// ClientMessage::Room(serde_json) до появления websocket-роутинга комнат.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ClientRoomMsg {
    /// Первое сообщение после коннекта: логин сессии.
    Hello { name: String },
    ListRooms { open_only: bool },
    CreateRoom(RoomConfig),
    JoinRoom { room: RoomId, as_spectator: bool },
    LeaveRoom,
    /// Только хост.
    StartBattle,
    Kick { member: String },
    /// Чат комнаты (§1.7): «{from}» проставляет сервер.
    Chat { text: String },
}

/// Комнатная половина серверного протокола (push в лобби + ответы).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ServerRoomMsg {
    /// Ответ на Hello: принятое имя (уникализировано сервером).
    Welcome { name: String },
    RoomList(Vec<PvpRoomSummary>),
    RoomCreated(RoomId),
    JoinedRoom(RoomView),
    JoinRejected { reason: String },
    /// Ok(None) — комната закрылась.
    LeftRoom(Option<PvpRoomSummary>),
    RoomUpdate(PvpRoomSummary),
    RoomClosed(RoomId),
    /// Старт боя: ваша логическая сторона (0 = army1, 1 = army2) + сид
    /// монетки инициативы (§1.3) — у обоих игроков одинаковый.
    Started { your_army: usize, ini_seed: u64 },
    Kicked(String),
    /// История чата при входе в комнату (§1.7, до 200 строк).
    ChatHistory(Vec<ChatMsg>),
    /// Сообщение чата комнаты (игроки + зрители).
    ChatMsg(ChatMsg),
    Error { message: String },
}

/// Строка чата комнаты (§1.7). at — unix-секунды; system — серым в UI.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChatMsg {
    pub from: String,
    pub text: String,
    pub at: u64,
    pub system: bool,
}

impl ChatMsg {
    pub fn user(from: &str, text: &str) -> Self {
        Self {
            from: from.to_owned(),
            text: text.to_owned(),
            at: now_unix(),
            system: false,
        }
    }

    pub fn system(text: &str) -> Self {
        Self {
            from: String::new(),
            text: text.to_owned(),
            at: now_unix(),
            system: true,
        }
    }
}

pub fn now_unix() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

impl crate::network::server::ClientMessage {
    /// Обернуть комнатное сообщение в транспортный вариант.
    pub fn room(msg: &ClientRoomMsg) -> Self {
        Self::Room(serde_json::to_string(msg).expect("client room msg serialize"))
    }
}

impl crate::network::server::ServerMessage {
    pub fn room(msg: &ServerRoomMsg) -> Self {
        Self::Room(serde_json::to_string(msg).expect("server room msg serialize"))
    }

    /// Разобрать комнатное сообщение, если это оно.
    pub fn as_room(&self) -> Option<ServerRoomMsg> {
        match self {
            Self::Room(raw) => serde_json::from_str(raw).ok(),
            _ => None,
        }
    }
}

// ---------------------------------------------------------------------------
// Тесты
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::battle::tests::{make_army_from_ids, make_test_registry};

    fn battle_config() -> RoomConfig {
        RoomConfig {
            title: "Тестовая битва".into(),
            ..Default::default()
        }
    }

    #[test]
    fn create_and_list_sorted() {
        let mut mgr = RoomManager::new();
        let id1 = mgr.create(battle_config(), "Аня").unwrap();
        let id2 = mgr
            .create(
                RoomConfig {
                    title: "Вторая".into(),
                    rated: true,
                    ..Default::default()
                },
                "Боря",
            )
            .unwrap();
        assert_ne!(id1, id2);
        let list = mgr.list();
        assert_eq!(list.len(), 2);
        // Стабильный порядок по id.
        assert!(list[0].id < list[1].id);
        let first = &list[0];
        assert_eq!(first.title, "Тестовая битва");
        assert_eq!(first.players, 1);
        assert_eq!(first.players_max, 2);
        assert_eq!(first.status, RoomStatus::Lobby);
        assert_eq!(first.host.display_name, "Аня");
        assert!(first.spectators_allowed);
    }

    #[test]
    fn create_rejects_bad_config_and_name() {
        let mut mgr = RoomManager::new();
        assert_eq!(mgr.create(battle_config(), "  "), Err(RoomError::EmptyName));
        let bad = RoomConfig {
            gold_per_side: 0,
            ..Default::default()
        };
        assert!(matches!(
            mgr.create(bad, "Аня"),
            Err(RoomError::InvalidConfig(_))
        ));
    }

    #[test]
    fn join_battle_room_full() {
        let mut mgr = RoomManager::new();
        let id = mgr.create(battle_config(), "host").unwrap();
        // host уже занимает слот 1 из 2.
        mgr.join(id, "второй", false).unwrap();
        assert_eq!(mgr.join(id, "третий", false), Err(RoomError::Full));
    }

    #[test]
    fn join_spectator_policy() {
        let mut mgr = RoomManager::new();
        let open = mgr.create(battle_config(), "host").unwrap();
        mgr.join(open, "зритель", true).unwrap();
        let closed_cfg = RoomConfig {
            spectators: SpectatorPolicy::Forbidden,
            ..Default::default()
        };
        let closed = mgr.create(closed_cfg, "host2").unwrap();
        assert_eq!(
            mgr.join(closed, "зритель", true),
            Err(RoomError::SpectatorsForbidden)
        );
    }

    #[test]
    fn join_duplicate_and_unknown() {
        let mut mgr = RoomManager::new();
        let id = mgr.create(battle_config(), "host").unwrap();
        assert_eq!(mgr.join(id, "host", false), Err(RoomError::AlreadyInRoom));
        assert_eq!(
            mgr.join(RoomId::MAX, "никем", false),
            Err(RoomError::RoomNotFound)
        );
        assert_eq!(mgr.join(id, "", false), Err(RoomError::EmptyName));
    }

    #[test]
    fn join_player_forbidden_after_start() {
        let mut mgr = RoomManager::new();
        let id = mgr.create(battle_config(), "host").unwrap();
        mgr.join(id, "второй", false).unwrap();
        mgr.start(id, "host").unwrap();
        assert_eq!(mgr.join(id, "третий", false), Err(RoomError::WrongStatus));
        // Наблюдатель в идущей битве — можно.
        mgr.join(id, "зритель", true).unwrap();
    }

    #[test]
    fn leave_host_shift_and_close() {
        let mut mgr = RoomManager::new();
        let id = mgr.create(battle_config(), "хост").unwrap();
        mgr.join(id, "второй", false).unwrap();
        // Не участник выйти не может.
        assert_eq!(mgr.leave(id, "левый"), Err(RoomError::NotInRoom));
        // Хост выходит — хостом становится следующий.
        let summary = mgr.leave(id, "хост").unwrap().unwrap();
        assert_eq!(summary.host.display_name, "второй");
        assert_eq!(summary.players, 1);
        // Последний выходит — комната закрывается.
        assert!(mgr.leave(id, "второй").unwrap().is_none());
        assert_eq!(mgr.get(id), None);
    }

    #[test]
    fn kick_requires_host() {
        let mut mgr = RoomManager::new();
        let id = mgr.create(battle_config(), "host").unwrap();
        mgr.join(id, "второй", false).unwrap();
        mgr.join(id, "зритель", true).unwrap();
        assert_eq!(mgr.kick(id, "второй", "зритель"), Err(RoomError::NotHost));
        assert_eq!(mgr.kick(id, "host", "host"), Err(RoomError::NotInRoom));
        assert_eq!(mgr.kick(id, "host", "никем"), Err(RoomError::NotInRoom));
        let summary = mgr.kick(id, "host", "зритель").unwrap();
        assert_eq!(summary.spectators, 0);
        assert_eq!(summary.players, 2);
    }

    #[test]
    fn start_rules() {
        let mut mgr = RoomManager::new();
        let id = mgr.create(battle_config(), "host").unwrap();
        assert_eq!(mgr.start(id, "host"), Err(RoomError::NotEnoughPlayers));
        mgr.join(id, "второй", false).unwrap();
        assert_eq!(mgr.start(id, "второй"), Err(RoomError::NotHost));
        mgr.start(id, "host").unwrap();
        assert_eq!(mgr.get(id).unwrap().status, RoomStatus::InGame);
        assert_eq!(mgr.start(id, "host"), Err(RoomError::WrongStatus));
    }

    #[test]
    fn config_validate_cases() {
        assert!(battle_config().validate().is_ok());
        let no_title = RoomConfig {
            title: "   ".into(),
            ..Default::default()
        };
        assert!(no_title.validate().is_err());
        let no_gold = RoomConfig {
            gold_per_side: 0,
            ..Default::default()
        };
        assert!(no_gold.validate().is_err());
        let points_no_table = RoomConfig {
            limit_kind: LimitKind::Points,
            ..Default::default()
        };
        assert!(points_no_table.validate().is_err());
        let points_with_table = RoomConfig {
            limit_kind: LimitKind::Points,
            points_table: Some(HashMap::from([(20usize, 100u64)])),
            ..Default::default()
        };
        assert!(points_with_table.validate().is_ok());
        let ban_overlap = RoomConfig {
            draft: DraftRules {
                bans: [vec![3], vec![3]],
            },
            ..Default::default()
        };
        assert!(ban_overlap.validate().is_err());
        let zero_units = RoomConfig {
            max_units: Some(0),
            ..Default::default()
        };
        assert!(zero_units.validate().is_err());
    }

    #[test]
    fn config_json_roundtrip() {
        let config = battle_config();
        let json = serde_json::to_string(&config).unwrap();
        let back: RoomConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(back, config);
    }

    #[test]
    fn room_protocol_json_roundtrip() {
        use crate::network::server::{ClientMessage, ServerMessage};
        // Клиент -> сервер.
        let client = ClientMessage::room(&ClientRoomMsg::JoinRoom {
            room: 7,
            as_spectator: true,
        });
        assert!(matches!(client, ClientMessage::Room(_)));
        // Сервер -> клиент: your_army при старте.
        let server = ServerMessage::room(&ServerRoomMsg::Started { your_army: 1, ini_seed: 7 });
        assert_eq!(
            server.as_room(),
            Some(ServerRoomMsg::Started { your_army: 1, ini_seed: 7 })
        );
        assert_eq!(ServerMessage::ChangeMenu(0).as_room(), None);
        // Список комнат возит DTO целиком.
        let mut mgr = RoomManager::new();
        let id = mgr.create(battle_config(), "host").unwrap();
        let list = ServerMessage::room(&ServerRoomMsg::RoomList(mgr.list()));
        match list.as_room().unwrap() {
            ServerRoomMsg::RoomList(rooms) => {
                assert_eq!(rooms.len(), 1);
                assert_eq!(rooms[0].id, id);
            }
            other => panic!("unexpected {other:?}"),
        }
    }

    /// Сервер -> клиент: YourArmy (алкэhest-транспорт) + монетка инициативы
    /// применяется к BattleInfo по сиду одинаково у обеих сторон.
    #[test]
    fn your_army_and_coin_flip_over_protocol() {
        use crate::battle::tests::{make_army_from_ids, make_test_registry};
        use crate::battle::BattleInfo;
        use crate::network::server::ServerMessage;
        let registry = make_test_registry();
        let mut armies = vec![
            make_army_from_ids(&registry, &[(20, 7)]),
            make_army_from_ids(&registry, &[(21, 1)]),
        ];
        // Серверная сторона: YourArmy уходит каждому игроку со своим your_army
        // и общим сидом монетки.
        let msg = ServerMessage::YourArmy {
            your_army: 1,
            ini_seed: 42,
        };
        let ServerMessage::YourArmy { your_army, ini_seed } = msg else {
            panic!("variant must roundtrip")
        };
        // Оба клиента применяют монетку по одному сиду — одинаковый исход.
        let mut battle = BattleInfo::new(&armies, 0, 1);
        let flip_here = battle.coin_flip_initiative(&mut armies, &registry, ini_seed);
        let mut armies2 = vec![
            make_army_from_ids(&registry, &[(20, 7)]),
            make_army_from_ids(&registry, &[(21, 1)]),
        ];
        let mut battle2 = BattleInfo::new(&armies2, 0, 1);
        let flip_there = battle2.coin_flip_initiative(&mut armies2, &registry, ini_seed);
        assert_eq!(flip_here, flip_there);
        assert_eq!(your_army, 1);
        // your_army попадает в BattleInfo для рендера (§1.8).
        battle.your_army = Some(your_army);
        assert_eq!(battle.your_army, Some(1));
    }

    #[test]
    fn check_army_limit_gold() {
        let registry = make_test_registry();
        // 3 юнита-мясника (id 20, cost_hire 0 в тестовом реестре — бюджет
        // через points_table).
        let army = make_army_from_ids(&registry, &[(20, 7), (20, 8), (20, 9)]);
        let config = RoomConfig {
            limit_kind: LimitKind::Points,
            points_table: Some(HashMap::from([(20usize, 100u64)])),
            gold_per_side: 250,
            ..Default::default()
        };
        let v = config.check_army(&army, &registry);
        assert_eq!(v.len(), 1);
        assert!(v[0].unit.is_none());
        assert!(v[0].message.contains("300"));
        // Увеличиваем бюджет — нарушений нет.
        let config = RoomConfig {
            gold_per_side: 300,
            ..config
        };
        assert!(config.check_army(&army, &registry).is_empty());
    }

    #[test]
    fn check_army_banlist_and_draft() {
        let registry = make_test_registry();
        let army = make_army_from_ids(&registry, &[(20, 7), (21, 8)]);
        // Полный банлист.
        let config = RoomConfig {
            banlist_units: vec![20],
            ..Default::default()
        };
        let v = config.check_army(&army, &registry);
        assert_eq!(v.len(), 1);
        assert_eq!(v[0].unit, Some(0));
        // Драфт-бан стороны 1 тоже действует на обе армии.
        let config = RoomConfig {
            draft: DraftRules {
                bans: [vec![], vec![21]],
            },
            ..Default::default()
        };
        let v = config.check_army(&army, &registry);
        assert_eq!(v.len(), 1);
        assert_eq!(v[0].unit, Some(1));
    }

    #[test]
    fn check_army_units_and_mages() {
        let registry = make_test_registry();
        // id 22 — маг (Death), id 20 — мясник.
        let army = make_army_from_ids(&registry, &[(22, 1), (20, 7)]);
        let config = RoomConfig {
            max_mages: Some(0),
            ..Default::default()
        };
        let v = config.check_army(&army, &registry);
        assert!(v.iter().any(|x| x.message.contains("магов")));
        let config = RoomConfig {
            max_units: Some(1),
            ..Default::default()
        };
        let v = config.check_army(&army, &registry);
        assert!(v.iter().any(|x| x.message.contains("юнитов")));
        // Без лимитов — чисто.
        let config = RoomConfig {
            limit_kind: LimitKind::None,
            ..Default::default()
        };
        assert!(config.check_army(&army, &registry).is_empty());
    }
}
