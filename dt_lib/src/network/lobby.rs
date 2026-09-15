//! Серверный хаб лобби (этап 1 §3 техплана): сессии поверх RoomManager.
//!
//! Чистая логика без tokio/сокетов — экземпляр `LobbyHub` живёт под
//! `Arc<Mutex<_>>` в `AppState` dt_server, а обработчик websocket просто
//! пересылает кадры. Возвращает исходящие сообщения адресно: `to: None` =
//! всем (лобби-броадкаст), `Some(id)` — конкретной сессии.
//!
//! Сессия: подключение = `Session::new` + `hello` (ник из Hello или
//! query-параметра). Комнатная логика (места, хост, старт) — в
//! `RoomManager` (network::room); хаб мапит сессии на комнаты и добавляет
//! чат §1.7: история 200 строк в комнате, лимит 300 символов,
//! rate-limit 5 сообщений / 10 секунд.

use crate::network::room::{
    now_unix, ChatMsg, ClientRoomMsg, PvpRoomSummary, RoomConfig, RoomError, RoomId,
    RoomManager, RoomStatus, RoomView, ServerRoomMsg,
};
use std::collections::{HashMap, VecDeque};

/// Идентификатор websocket-соединения (серверный счётчик).
pub type SocketId = u64;

/// Лимиты чата комнаты (§1.7).
pub const CHAT_MAX_LEN: usize = 300;
pub const CHAT_HISTORY_CAP: usize = 200;
pub const CHAT_RATE_MESSAGES: usize = 5;
pub const CHAT_RATE_WINDOW_SECS: u64 = 10;

/// Состояние одного websocket-подключения.
#[derive(Debug, Clone)]
pub struct Session {
    /// Имя игрока (после Hello; до — пусто, сообщения отбрасываются).
    pub name: String,
    /// Текущая комната, если вошёл (игрок или зритель).
    pub room: Option<RoomId>,
    /// Окошки отправки чата (unix-секунды) для rate-limit.
    chat_times: VecDeque<u64>,
}

impl Session {
    pub fn new() -> Self {
        Self {
            name: String::new(),
            room: None,
            chat_times: VecDeque::new(),
        }
    }

    pub fn named(name: &str) -> Self {
        let mut s = Self::new();
        s.name = name.to_owned();
        s
    }

    /// Зарегистрирована ли сессия (прошла Hello).
    pub fn is_named(&self) -> bool {
        !self.name.is_empty()
    }
}

/// Исходящее сообщение: адресат + кадр.
#[derive(Debug, Clone)]
pub struct Outgoing {
    pub to: Option<SocketId>,
    pub msg: ServerRoomMsg,
}

impl Outgoing {
    pub fn one(to: SocketId, msg: ServerRoomMsg) -> Self {
        Self { to: Some(to), msg }
    }

    pub fn all(msg: ServerRoomMsg) -> Self {
        Self { to: None, msg }
    }
}

/// Реестр комнат + сессии. Один инстанс на процесс сервера.
#[derive(Debug, Default)]
pub struct LobbyHub {
    pub rooms: RoomManager,
    pub sessions: HashMap<SocketId, Session>,
    next_socket: SocketId,
    /// История чата per-room (§1.7); комната закрыта — история умерла с ней.
    pub chat_history: HashMap<RoomId, VecDeque<ChatMsg>>,
}

impl LobbyHub {
    pub fn new() -> Self {
        Self::default()
    }

    /// Новый сокет подключился: выдать id. Имя приходит сообщением Hello
    /// (или из query-параметра — тогда сразу `Session::named`).
    pub fn connect(&mut self, session: Session) -> SocketId {
        let id = self.next_socket;
        self.next_socket += 1;
        self.sessions.insert(id, session);
        id
    }

    /// Сокет отключился: покинуть комнату (broadcast её участникам),
    /// забыть сессию. Возвращает (RoomClosed для лобби?, коррекция списка).
    pub fn disconnect(&mut self, id: SocketId) -> Vec<Outgoing> {
        let name = self.sessions.get(&id).and_then(|s| {
            if s.is_named() {
                Some(s.name.clone())
            } else {
                None
            }
        });
        let Some(name) = name else {
            self.sessions.remove(&id);
            return vec![];
        };
        let room_id = self.sessions.get(&id).and_then(|s| s.room);
        self.sessions.remove(&id);
        let Some(room_id) = room_id else {
            return vec![];
        };
        self.leave_room_impl(id, room_id, &name)
    }

    /// Обработка комнатного сообщения от сокета. `name_hint` — ник из
    /// query-параметра (Hello можно не слать).
    pub fn handle(
        &mut self,
        id: SocketId,
        msg: ClientRoomMsg,
    ) -> Vec<Outgoing> {
        // Всё, кроме Hello, требует регистрации сессии (после Hello).
        if !matches!(msg, ClientRoomMsg::Hello { .. })
            && !self.sessions.get(&id).is_some_and(|s| s.is_named())
        {
            return vec![self.unregistered(id)];
        }
        match msg {
            ClientRoomMsg::Hello { name } => self.hello(id, &name),
            ClientRoomMsg::ListRooms { open_only } => {
                let list: Vec<PvpRoomSummary> = self
                    .rooms
                    .list()
                    .into_iter()
                    .filter(|r| {
                        !open_only
                            || (r.status == RoomStatus::Lobby && r.players < r.players_max)
                    })
                    .collect();
                vec![Outgoing::one(id, ServerRoomMsg::RoomList(list))]
            }
            ClientRoomMsg::CreateRoom(config) => self.create_room(id, config),
            ClientRoomMsg::JoinRoom { room, as_spectator } => {
                self.join_room(id, room, as_spectator)
            }
            ClientRoomMsg::LeaveRoom => self.leave_room(id),
            ClientRoomMsg::StartBattle => self.start_battle(id),
            ClientRoomMsg::Kick { member } => self.kick(id, &member),
            ClientRoomMsg::Chat { text } => self.chat(id, text),
        }
    }

    /// ID сокетов — получатели комнатного broadcast (игроки + зрители).
    pub fn room_sockets(&self, room: RoomId) -> Vec<SocketId> {
        let mut ids: Vec<SocketId> = self
            .sessions
            .iter()
            .filter(|(_, s)| s.room == Some(room))
            .map(|(id, _)| *id)
            .collect();
        ids.sort_unstable();
        ids
    }

    /// Все зарегистрированные сокеты (лобби-броадкаст).
    pub fn all_sockets(&self) -> Vec<SocketId> {
        let mut ids: Vec<SocketId> = self
            .sessions
            .iter()
            .filter(|(_, s)| s.is_named())
            .map(|(id, _)| *id)
            .collect();
        ids.sort_unstable();
        ids
    }

    fn hello(&mut self, id: SocketId, name: &str) -> Vec<Outgoing> {
        let name = name.trim();
        if name.is_empty() {
            return vec![Outgoing::one(
                id,
                ServerRoomMsg::Error {
                    message: RoomError::EmptyName.to_string(),
                },
            )];
        }
        // Уникализация ников по всему серверу (валидация комнат требует
        // уникальность внутри комнаты — делаем уникальным глобально).
        let mut unique = name.to_owned();
        let mut n = 1;
        while self.sessions.values().any(|s| s.name == unique) {
            n += 1;
            unique = format!("{name}#{n}");
        }
        let Some(session) = self.sessions.get_mut(&id) else {
            return vec![];
        };
        session.name = unique.clone();
        vec![Outgoing::one(
            id,
            ServerRoomMsg::Welcome { name: unique },
        )]
    }

    fn create_room(&mut self, id: SocketId, config: RoomConfig) -> Vec<Outgoing> {
        let Some(name) = self.name_of(id) else {
            return vec![self.unregistered(id)];
        };
        match self.rooms.create(config, &name) {
            Ok(room_id) => {
                // Создатель сразу входит в свою комнату.
                let mut out = vec![Outgoing::one(
                    id,
                    ServerRoomMsg::RoomCreated(room_id),
                )];
                out.extend(self.join_room_inner(id, room_id, false));
                out.push(Outgoing::all(ServerRoomMsg::RoomUpdate(
                    self.rooms.get(room_id).expect("just created").summary_pub(),
                )));
                out
            }
            Err(e) => vec![Outgoing::one(
                id,
                ServerRoomMsg::Error { message: e.to_string() },
            )],
        }
    }

    fn join_room(&mut self, id: SocketId, room: RoomId, as_spectator: bool) -> Vec<Outgoing> {
        let Some(name) = self.name_of(id) else {
            return vec![self.unregistered(id)];
        };
        if self.sessions.get(&id).and_then(|s| s.room) == Some(room) {
            return vec![Outgoing::one(
                id,
                ServerRoomMsg::Error {
                    message: RoomError::AlreadyInRoom.to_string(),
                },
            )];
        }
        match self.rooms.join(room, &name, as_spectator) {
            Ok(_) => self.join_room_inner(id, room, as_spectator),
            Err(e) => vec![Outgoing::one(
                id,
                ServerRoomMsg::JoinRejected { reason: e.to_string() },
            )],
        }
    }

    /// Постобработка успешного join: сессия, системный чат, история,
    /// RoomUpdate в лобби.
    fn join_room_inner(
        &mut self,
        id: SocketId,
        room: RoomId,
        as_spectator: bool,
    ) -> Vec<Outgoing> {
        let name = self.sessions.get(&id).expect("checked by caller").name.clone();
        if let Some(session) = self.sessions.get_mut(&id) {
            session.room = Some(room);
        }
        let view = self.rooms.view(room).expect("join succeeded");
        let mut out = vec![Outgoing::one(
            id,
            ServerRoomMsg::JoinedRoom(view.clone()),
        )];
        // Системное сообщение + RoomUpdate остальным (участникам и лобби).
        let system = ChatMsg::system(&format!(
            "{} присоединился как {}",
            name,
            if as_spectator { "зритель" } else { "игрок" }
        ));
        out.extend(self.push_chat(room, system));
        out.extend(self.room_update(room));
        // История чата входящему (§1.7).
        let history: Vec<ChatMsg> = self
            .chat_history
            .get(&room)
            .map(|h| h.iter().cloned().collect())
            .unwrap_or_default();
        out.push(Outgoing::one(
            id,
            ServerRoomMsg::ChatHistory(history),
        ));
        out
    }

    fn leave_room(&mut self, id: SocketId) -> Vec<Outgoing> {
        let Some((name, room)) = self
            .sessions
            .get(&id)
            .map(|s| (s.name.clone(), s.room))
        else {
            return vec![];
        };
        let Some(room) = room else {
            return vec![Outgoing::one(
                id,
                ServerRoomMsg::Error {
                    message: RoomError::NotInRoom.to_string(),
                },
            )];
        };
        self.leave_room_impl(id, room, &name)
    }

    fn leave_room_impl(&mut self, id: SocketId, room: RoomId, name: &str) -> Vec<Outgoing> {
        let mut out = vec![];
        match self.rooms.leave(room, name) {
            Ok(None) => {
                // Комната закрылась: RoomClosed лобби + участникам.
                self.chat_history.remove(&room);
                out.push(Outgoing::all(ServerRoomMsg::RoomClosed(room)));
            }
            Ok(Some(summary)) => {
                out.push(Outgoing::one(id, ServerRoomMsg::LeftRoom(Some(summary))));
                let system = ChatMsg::system(&format!("{name} покинул комнату"));
                out.extend(self.push_chat(room, system));
                out.extend(self.room_update(room));
            }
            Err(_) => {}
        }
        if let Some(session) = self.sessions.get_mut(&id) {
            session.room = None;
        }
        out
    }

    fn start_battle(&mut self, id: SocketId) -> Vec<Outgoing> {
        let Some((name, Some(room))) = self
            .sessions
            .get(&id)
            .map(|s| (s.name.clone(), s.room))
        else {
            return vec![Outgoing::one(
                id,
                ServerRoomMsg::Error {
                    message: RoomError::NotInRoom.to_string(),
                },
            )];
        };
        if let Err(e) = self.rooms.start(room, &name) {
            return vec![Outgoing::one(
                id,
                ServerRoomMsg::Error { message: e.to_string() },
            )];
        }
        // Армии рассылает транспорт (генерация/Started через Outcoming);
        // здесь — только сигналы протокола комнат.
        let mut out = vec![];
        let system = ChatMsg::system("Батл начался");
        out.extend(self.push_chat(room, system));
        // your_army для каждого игрока: позиция в room.players.
        let view: RoomView = self.rooms.view(room).expect("started room");
        for (idx, player) in view.players.iter().enumerate() {
            for sock in self.room_sockets(room) {
                if self.sessions.get(&sock).map(|s| s.name.as_str()) == Some(player.as_str()) {
                    out.push(Outgoing::one(
                        sock,
                        ServerRoomMsg::Started {
                            your_army: idx,
                            ini_seed: now_unix(),
                        },
                    ));
                }
            }
        }
        out.extend(self.room_update(room));
        out
    }

    fn kick(&mut self, id: SocketId, member: &str) -> Vec<Outgoing> {
        let Some((name, Some(room))) = self
            .sessions
            .get(&id)
            .map(|s| (s.name.clone(), s.room))
        else {
            return vec![Outgoing::one(
                id,
                ServerRoomMsg::Error {
                    message: RoomError::NotInRoom.to_string(),
                },
            )];
        };
        match self.rooms.kick(room, &name, member) {
            Ok(_) => {
                let mut out = vec![];
                // Кикнутому — Kicked + сброс сессии; сокет транспорт закроет.
                for sock in self.room_sockets(room) {
                    if self.sessions.get(&sock).map(|s| s.name.as_str()) == Some(member) {
                        out.push(Outgoing::one(sock, ServerRoomMsg::Kicked(member.into())));
                        if let Some(session) = self.sessions.get_mut(&sock) {
                            session.room = None;
                        }
                    }
                }
                let system = ChatMsg::system(&format!("{member} кикнут хостом"));
                out.extend(self.push_chat(room, system));
                out.extend(self.room_update(room));
                out
            }
            Err(e) => vec![Outgoing::one(
                id,
                ServerRoomMsg::Error { message: e.to_string() },
            )],
        }
    }

    fn chat(&mut self, id: SocketId, text: String) -> Vec<Outgoing> {
        let Some((name, Some(room))) = self
            .sessions
            .get_mut(&id)
            .map(|s| (s.name.clone(), s.room))
        else {
            return vec![Outgoing::one(
                id,
                ServerRoomMsg::Error {
                    message: RoomError::NotInRoom.to_string(),
                },
            )];
        };
        let text = text.trim().to_owned();
        if text.is_empty() {
            return vec![];
        }
        // Лимит длины (§1.7): молча обрезаем.
        let text: String = text.chars().take(CHAT_MAX_LEN).collect();
        // Rate-limit: не более CHAT_RATE_MESSAGES за CHAT_RATE_WINDOW_SECS.
        let now = now_unix();
        let session = self.sessions.get_mut(&id).expect("checked above");
        while session
            .chat_times
            .front()
            .is_some_and(|t| now.saturating_sub(*t) > CHAT_RATE_WINDOW_SECS)
        {
            session.chat_times.pop_front();
        }
        if session.chat_times.len() >= CHAT_RATE_MESSAGES {
            return vec![Outgoing::one(
                id,
                ServerRoomMsg::Error {
                    message: "Слишком много сообщений, подождите".into(),
                },
            )];
        }
        session.chat_times.push_back(now);
        let msg = ChatMsg::user(&name, &text);
        self.push_chat(room, msg)
    }

    /// Добавить сообщение в историю и разослать всем в комнате.
    fn push_chat(&mut self, room: RoomId, msg: ChatMsg) -> Vec<Outgoing> {
        let history = self.chat_history.entry(room).or_default();
        history.push_back(msg.clone());
        while history.len() > CHAT_HISTORY_CAP {
            history.pop_front();
        }
        self.room_sockets(room)
            .into_iter()
            .map(|sock| Outgoing::one(sock, ServerRoomMsg::ChatMsg(msg.clone())))
            .collect()
    }

    /// RoomUpdate всем зарегистрированным (лобби в фоне слушает).
    fn room_update(&self, room: RoomId) -> Vec<Outgoing> {
        let Some(room_data) = self.rooms.get(room) else {
            return vec![];
        };
        vec![Outgoing::all(ServerRoomMsg::RoomUpdate(
            room_data.summary_pub(),
        ))]
    }

    fn name_of(&self, id: SocketId) -> Option<String> {
        self.sessions
            .get(&id)
            .filter(|s| s.is_named())
            .map(|s| s.name.clone())
    }

    fn unregistered(&self, id: SocketId) -> Outgoing {
        Outgoing::one(
            id,
            ServerRoomMsg::Error {
                message: "Сначала Hello { name }".into(),
            },
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::network::room::RoomConfig;

    fn hub() -> LobbyHub {
        LobbyHub::new()
    }

    /// Подключить сессию с именем (Hello уже отработал).
    fn connect_named(hub: &mut LobbyHub, name: &str) -> SocketId {
        let id = hub.connect(Session::new());
        let out = hub.handle(id, ClientRoomMsg::Hello { name: name.into() });
        assert!(matches!(out[0].msg, ServerRoomMsg::Welcome { .. }));
        id
    }

    fn battle_config() -> RoomConfig {
        RoomConfig {
            title: "Тест".into(),
            ..Default::default()
        }
    }

    #[test]
    fn hello_welcome_and_unique_names() {
        let mut h = hub();
        let a = connect_named(&mut h, "Аня");
        let out = h.handle(a, ClientRoomMsg::Hello { name: "Аня".into() });
        // Повторный Hello — имя уникализируется.
        match &out[0].msg {
            ServerRoomMsg::Welcome { name } => assert_eq!(name, "Аня#2"),
            other => panic!("unexpected {other:?}"),
        }
        // Пустое имя — ошибка.
        let b = h.connect(Session::new());
        let out = h.handle(b, ClientRoomMsg::Hello { name: "  ".into() });
        assert!(matches!(out[0].msg, ServerRoomMsg::Error { .. }));
    }

    #[test]
    fn unregistered_rejected() {
        let mut h = hub();
        let id = h.connect(Session::new());
        let out = h.handle(id, ClientRoomMsg::ListRooms { open_only: false });
        assert!(matches!(out[0].msg, ServerRoomMsg::Error { .. }));
    }

    #[test]
    fn list_rooms_and_open_filter() {
        let mut h = hub();
        let a = connect_named(&mut h, "host");
        let room = match &h.handle(a, ClientRoomMsg::CreateRoom(battle_config()))[0].msg {
            ServerRoomMsg::RoomCreated(id) => *id,
            other => panic!("unexpected {other:?}"),
        };
        let b = connect_named(&mut h, "второй");
        h.handle(b, ClientRoomMsg::JoinRoom { room, as_spectator: false });
        // Комната полная: в open_only её нет.
        let out = h.handle(a, ClientRoomMsg::ListRooms { open_only: true });
        let ServerRoomMsg::RoomList(list) = &out[0].msg else {
            panic!("not a list")
        };
        assert!(list.is_empty());
        let out = h.handle(a, ClientRoomMsg::ListRooms { open_only: false });
        let ServerRoomMsg::RoomList(list) = &out[0].msg else {
            panic!("not a list")
        };
        assert_eq!(list.len(), 1);
    }

    #[test]
    fn create_joins_creator_and_broadcasts_update() {
        let mut h = hub();
        let a = connect_named(&mut h, "host");
        let viewer = connect_named(&mut h, "наблюдатель");
        let out = h.handle(a, ClientRoomMsg::CreateRoom(battle_config()));
        // Создателю: RoomCreated + JoinedRoom + системный чат; всем: RoomUpdate.
        assert!(matches!(out[0].msg, ServerRoomMsg::RoomCreated(_)));
        assert!(matches!(out[1].msg, ServerRoomMsg::JoinedRoom(_)));
        assert!(out
            .iter()
            .any(|o| matches!(o.msg, ServerRoomMsg::RoomUpdate(_))));
        // RoomUpdate — broadcast (to: None), придёт и наблюдателю.
        assert!(out
            .iter()
            .any(|o| o.to.is_none() && matches!(o.msg, ServerRoomMsg::RoomUpdate(_))));
        let _ = viewer;
        let session = h.sessions.get(&a).unwrap();
        assert!(session.room.is_some());
    }

    #[test]
    fn join_rejected_when_full() {
        let mut h = hub();
        let a = connect_named(&mut h, "host");
        let room = match &h.handle(a, ClientRoomMsg::CreateRoom(battle_config()))[0].msg {
            ServerRoomMsg::RoomCreated(id) => *id,
            other => panic!("unexpected {other:?}"),
        };
        let b = connect_named(&mut h, "второй");
        h.handle(b, ClientRoomMsg::JoinRoom { room, as_spectator: false });
        let c = connect_named(&mut h, "третий");
        let out = h.handle(c, ClientRoomMsg::JoinRoom { room, as_spectator: false });
        assert!(matches!(out[0].msg, ServerRoomMsg::JoinRejected { .. }));
        // Зрителем — можно.
        let out = h.handle(c, ClientRoomMsg::JoinRoom { room, as_spectator: true });
        assert!(matches!(out[0].msg, ServerRoomMsg::JoinedRoom(_)));
    }

    #[test]
    fn chat_broadcast_history_and_system_messages() {
        let mut h = hub();
        let a = connect_named(&mut h, "host");
        let room = match &h.handle(a, ClientRoomMsg::CreateRoom(battle_config()))[0].msg {
            ServerRoomMsg::RoomCreated(id) => *id,
            other => panic!("unexpected {other:?}"),
        };
        let b = connect_named(&mut h, "гость");
        let out = h.handle(b, ClientRoomMsg::JoinRoom { room, as_spectator: true });
        // Системное «присоединился» разослано обоим + история входящему.
        assert!(out
            .iter()
            .any(|o| matches!(&o.msg, ServerRoomMsg::ChatMsg(m) if m.system)));
        assert!(matches!(out.last().unwrap().msg, ServerRoomMsg::ChatHistory(_)));
        let out = h.handle(a, ClientRoomMsg::Chat { text: "привет".into() });
        let sent: Vec<_> = out
            .iter()
            .filter(|o| matches!(&o.msg, ServerRoomMsg::ChatMsg(m) if m.text == "привет" && m.from == "host"))
            .collect();
        assert_eq!(sent.len(), 2, "both room members get the chat msg");
        // История: system(join host) + system(join гость) + привет.
        let history = h.chat_history.get(&room).unwrap();
        assert_eq!(history.len(), 3);
        assert_eq!(history.back().unwrap().text, "привет");
    }

    #[test]
    fn chat_rate_limit_and_length_cap() {
        let mut h = hub();
        let a = connect_named(&mut h, "host");
        let room = match &h.handle(a, ClientRoomMsg::CreateRoom(battle_config()))[0].msg {
            ServerRoomMsg::RoomCreated(id) => *id,
            other => panic!("unexpected {other:?}"),
        };
        let _ = &room;
        for i in 0..5 {
            let out = h.handle(a, ClientRoomMsg::Chat { text: format!("м{i}") });
            assert!(!out.is_empty(), "msg {i} must pass");
        }
        // 6-е подряд — отказ.
        let out = h.handle(a, ClientRoomMsg::Chat { text: "лишний".into() });
        assert!(matches!(out[0].msg, ServerRoomMsg::Error { .. }));
        // Лимит длины: 400 символов -> 300 в истории.
        h.sessions.get_mut(&a).unwrap().chat_times.clear();
        let long = "ж".repeat(400);
        h.handle(a, ClientRoomMsg::Chat { text: long });
        let last = h.chat_history.get(&room).unwrap().back().unwrap();
        assert_eq!(last.text.chars().count(), CHAT_MAX_LEN);
    }

    #[test]
    fn chat_requires_room() {
        let mut h = hub();
        let a = connect_named(&mut h, "одинокий");
        let out = h.handle(a, ClientRoomMsg::Chat { text: "эх".into() });
        assert!(matches!(out[0].msg, ServerRoomMsg::Error { .. }));
    }

    #[test]
    fn start_only_host_and_started_addresses_players() {
        let mut h = hub();
        let a = connect_named(&mut h, "host");
        let room = match &h.handle(a, ClientRoomMsg::CreateRoom(battle_config()))[0].msg {
            ServerRoomMsg::RoomCreated(id) => *id,
            other => panic!("unexpected {other:?}"),
        };
        let b = connect_named(&mut h, "гость");
        h.handle(b, ClientRoomMsg::JoinRoom { room, as_spectator: false });
        let c = connect_named(&mut h, "зритель");
        h.handle(c, ClientRoomMsg::JoinRoom { room, as_spectator: true });
        // Не-хост стартовать не может.
        let out = h.handle(b, ClientRoomMsg::StartBattle);
        assert!(matches!(out[0].msg, ServerRoomMsg::Error { .. }));
        // Хост стартует: Started с your_army 0/1 именно игрокам.
        let out = h.handle(a, ClientRoomMsg::StartBattle);
        let mut started = vec![];
        for o in &out {
            if let ServerRoomMsg::Started { your_army, ini_seed } = o.msg {
                started.push((o.to, your_army, ini_seed));
            }
        }
        assert_eq!(started.len(), 2);
        let sock_a = started.iter().find(|s| s.0 == Some(a)).unwrap();
        let sock_b = started.iter().find(|s| s.0 == Some(b)).unwrap();
        assert_eq!(sock_a.1, 0);
        assert_eq!(sock_b.1, 1);
        assert_eq!(sock_a.2, sock_b.2, "ini_seed is shared");
        // Зрителю Started не приходил.
        assert!(!started.iter().any(|s| s.0 == Some(c)));
    }

    #[test]
    fn leave_updates_and_closes_room() {
        let mut h = hub();
        let a = connect_named(&mut h, "host");
        let room = match &h.handle(a, ClientRoomMsg::CreateRoom(battle_config()))[0].msg {
            ServerRoomMsg::RoomCreated(id) => *id,
            other => panic!("unexpected {other:?}"),
        };
        let out = h.handle(a, ClientRoomMsg::LeaveRoom);
        // Комната с одним участником закрылась: RoomClosed всем.
        assert!(out
            .iter()
            .any(|o| matches!(o.msg, ServerRoomMsg::RoomClosed(id) if id == room)));
        assert!(h.rooms.get(room).is_none());
        assert!(h.chat_history.get(&room).is_none());
        assert!(h.sessions.get(&a).unwrap().room.is_none());
    }

    #[test]
    fn disconnect_leaves_room_with_broadcast() {
        let mut h = hub();
        let a = connect_named(&mut h, "host");
        let room = match &h.handle(a, ClientRoomMsg::CreateRoom(battle_config()))[0].msg {
            ServerRoomMsg::RoomCreated(id) => *id,
            other => panic!("unexpected {other:?}"),
        };
        let b = connect_named(&mut h, "гость");
        h.handle(b, ClientRoomMsg::JoinRoom { room, as_spectator: false });
        // Хост отвалился: хостом становится b, комната жива.
        let out = h.disconnect(a);
        assert!(out.iter().any(|o| matches!(&o.msg, ServerRoomMsg::ChatMsg(m)
            if m.system && m.text.contains("host покинул"))));
        let room_data = h.rooms.get(room).unwrap();
        assert_eq!(room_data.host, "гость");
        assert!(h.sessions.get(&a).is_none());
    }

    #[test]
    fn kick_removes_session_room() {
        let mut h = hub();
        let a = connect_named(&mut h, "host");
        let room = match &h.handle(a, ClientRoomMsg::CreateRoom(battle_config()))[0].msg {
            ServerRoomMsg::RoomCreated(id) => *id,
            other => panic!("unexpected {other:?}"),
        };
        let b = connect_named(&mut h, "гость");
        h.handle(b, ClientRoomMsg::JoinRoom { room, as_spectator: false });
        let out = h.handle(a, ClientRoomMsg::Kick { member: "гость".into() });
        assert!(out
            .iter()
            .any(|o| o.to == Some(b) && matches!(o.msg, ServerRoomMsg::Kicked(_))));
        assert!(h.sessions.get(&b).unwrap().room.is_none());
        // Не-хост кикать не может.
        let out = h.handle(b, ClientRoomMsg::Kick { member: "host".into() });
        assert!(matches!(out[0].msg, ServerRoomMsg::Error { .. }));
    }

    #[test]
    fn room_sockets_sorted_broadcast() {
        let mut h = hub();
        let a = connect_named(&mut h, "host");
        let room = match &h.handle(a, ClientRoomMsg::CreateRoom(battle_config()))[0].msg {
            ServerRoomMsg::RoomCreated(id) => *id,
            other => panic!("unexpected {other:?}"),
        };
        let b = connect_named(&mut h, "гость");
        h.handle(b, ClientRoomMsg::JoinRoom { room, as_spectator: true });
        let sockets = h.room_sockets(room);
        assert_eq!(sockets, vec![a, b]);
        // Чужой сокет не в списке.
        let outsider = connect_named(&mut h, "мимо");
        assert!(!sockets.contains(&outsider));
    }
}
