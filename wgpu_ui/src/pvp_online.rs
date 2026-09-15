//! Онлайн-слой ПВП (пункт 2): websocket-клиент лобби/комнат/чата.
//!
//! Транспорт: ws://ip:port/ws?name=Ник. Текстовые кадры — serde_json
//! ClientRoomMsg/ServerRoomMsg; бинарные — alkahest Incoming/Outcoming
//! боя (dt_server::{Incoming, Outcoming} через dt_client реэкспорт).
//!
//! Владение: `PvpNet` живёт в PvpState; команды UI кладёт в `outgoing`
//! (mpsc), фоновая задача в `state.rt` читает сокет и пишет входящие в
//! `incoming` (RwLock<Vec>), экраны забирают их в начале кадра.

use dt_lib::network::room::{ClientRoomMsg, RoomId, RoomView, ServerRoomMsg};
use dt_server::Incoming;
use futures_util::{SinkExt, StreamExt};
use std::sync::{Arc, Mutex as StdMutex, RwLock};
use tokio::sync::mpsc;
use tokio_tungstenite::{
    connect_async,
    tungstenite::{client::IntoClientRequest, Message as WsMessage},
};

/// Входящее событие для UI (уже разобранное).
#[derive(Debug)]
pub enum PvpEvent {
    /// ServerRoomMsg (контроль + чат).
    Room(ServerRoomMsg),
    /// Полный снапшот битвы: (armies, battle) из Outcoming::Battle.
    Battle((Vec<dt_lib::battle::army::Army>, dt_lib::battle::battlefield::BattleInfo)),
    /// Outcoming::Status (acceptance) сетапа.
    Acceptance([bool; 2]),
    /// Соединение закрылось.
    Closed,
}

/// Сетевая половина PvpState. None = оффлайн (мок работает как раньше).
pub struct PvpNet {
    /// Команды UI → сеть.
    outgoing: Option<mpsc::UnboundedSender<C2S>>,
    /// Накопленные входящие события (UI забирает drain-ом).
    pub inbox: Arc<RwLock<Vec<PvpEvent>>>,
    /// Принятое сервером имя (Welcome).
    pub server_name: Option<String>,
    /// Комната, в которой нас видит сервер (по JoinedRoom/LeftRoom).
    pub room: Option<RoomId>,
    /// Последний вид комнаты (JoinedRoom).
    pub view: Option<RoomView>,
    /// Сид последнего старта (для монетки §1.3).
    pub ini_seed: u64,
}

/// Команда UI → сеть.
pub enum C2S {
    Room(ClientRoomMsg),
    Battle(Incoming),
}

impl PvpNet {
    pub fn offline() -> Self {
        Self {
            outgoing: None,
            inbox: Arc::new(RwLock::new(Vec::new())),
            server_name: None,
            room: None,
            view: None,
            ini_seed: 0,
        }
    }

    pub fn connected(&self) -> bool {
        self.outgoing.is_some()
    }

    /// Отправить комнатное сообщение (если онлайн).
    pub fn send_room(&self, msg: ClientRoomMsg) {
        if let Some(tx) = &self.outgoing {
            let _ = tx.send(C2S::Room(msg));
        }
    }

    /// Отправить кадр боя (если онлайн).
    pub fn send_battle(&self, msg: Incoming) {
        if let Some(tx) = &self.outgoing {
            let _ = tx.send(C2S::Battle(msg));
        }
    }

    /// Забрать накопленные события.
    pub fn drain(&self) -> Vec<PvpEvent> {
        match self.inbox.write() {
            Ok(mut v) => std::mem::take(&mut *v),
            Err(_) => Vec::new(),
        }
    }

    fn push(&self, ev: PvpEvent) {
        if let Ok(mut v) = self.inbox.write() {
            v.push(ev);
        }
    }
}

/// Подключение к серверу: спавнит фоновую задачу в `rt`, возвращает PvpNet.
/// Ник уходит в query (?name=) — сервер сразу отвечает Welcome.
pub fn connect(rt: &tokio::runtime::Runtime, addr: &str, name: &str) -> Result<PvpNet, String> {
    let addr = addr.trim().trim_start_matches("ws://").to_owned();
    let name = name.trim().to_owned();
    let (out_tx, mut out_rx) = mpsc::unbounded_channel::<C2S>();
    let inbox: Arc<RwLock<Vec<PvpEvent>>> = Arc::new(RwLock::new(Vec::new()));
    let inbox2 = Arc::clone(&inbox);

    let join = rt.spawn(async move {
        let url = format!("ws://{addr}/ws?name={name}");
        let request = match url.into_client_request() {
            Ok(r) => r,
            Err(e) => {
                if let Ok(mut v) = inbox2.write() {
                    v.push(PvpEvent::Closed);
                }
                return Err(format!("bad url: {e}"));
            }
        };
        let (ws, _) = match connect_async(request).await {
            Ok(x) => x,
            Err(e) => {
                if let Ok(mut v) = inbox2.write() {
                    v.push(PvpEvent::Closed);
                }
                return Err(format!("connect: {e}"));
            }
        };
        let (mut sink, mut stream) = ws.split();
        let mut closed = false;

        // Один select-цикл: исходящие UI и входящие сервера.
        loop {
            // Отдаём всё, что накопил UI (не блокируясь).
            let mut sent_any = false;
            while let Ok(cmd) = out_rx.try_recv() {
                sent_any = true;
                let frame = match cmd {
                    C2S::Room(msg) => {
                        WsMessage::Text(serde_json::to_string(&msg).unwrap_or_default().into())
                    }
                    C2S::Battle(msg) => {
                        let mut buf = vec![];
                        alkahest::serialize_to_vec::<Incoming, Incoming>(msg, &mut buf);
                        WsMessage::Binary(buf.into())
                    }
                };
                if sink.send(frame).await.is_err() {
                    closed = true;
                    break;
                }
            }
            if closed {
                break;
            }
            let next = if sent_any {
                // Есть отправленное — не ждём новый вход мгновенно, но и
                // не крутим холосто: короткий таймаут недоступен без timer,
                // поэтому просто продолжаем чтение (yield_now).
                tokio::task::yield_now().await;
                stream.next().await
            } else {
                stream.next().await
            };
            match next {
                Some(Ok(WsMessage::Text(raw))) => {
                    match serde_json::from_str::<ServerRoomMsg>(&raw) {
                        Ok(msg) => {
                            if let Ok(mut v) = inbox2.write() {
                                v.push(PvpEvent::Room(msg));
                            }
                        }
                        Err(e) => eprintln!("[pvp] bad server frame: {e}"),
                    }
                }
                Some(Ok(WsMessage::Binary(data))) => {
                    use dt_server::Outcoming;
                    if let Ok(msg) = alkahest::deserialize::<Outcoming, Outcoming>(&data) {
                        let ev = match msg {
                            Outcoming::Battle((armies, battle)) => PvpEvent::Battle((armies, battle)),
                            Outcoming::Status(a) => PvpEvent::Acceptance(a),
                            Outcoming::Id(_) => continue, // v1: роль из Started
                        };
                        if let Ok(mut v) = inbox2.write() {
                            v.push(ev);
                        }
                    }
                }
                Some(Ok(WsMessage::Close(_))) | Some(Err(_)) | None => break,
                Some(Ok(WsMessage::Ping(_)))
                | Some(Ok(WsMessage::Pong(_)))
                | Some(Ok(WsMessage::Frame(_))) => {}
            }
        }
        if let Ok(mut v) = inbox2.write() {
            v.push(PvpEvent::Closed);
        }
        Ok(())
    });

    // Не ждём задачу: connected=true сразу; ошибки придут как PvpEvent::Closed.
    let _ = join;
    Ok(PvpNet {
        outgoing: Some(out_tx),
        inbox,
        server_name: None,
        room: None,
        view: None,
        ini_seed: 0,
    })
}

/// Хелпер: разбор входящих Room-сообщений с обновлением состояния PvpNet
/// (вызывает экран после drain).
pub fn apply_room_event(net: &mut PvpNet, ev: &PvpEvent) -> Option<Action> {
    let PvpEvent::Room(msg) = ev else {
        return None;
    };
    apply_room_msg(net, msg)
}

/// То же для уже разобранного ServerRoomMsg.
pub fn apply_room_msg(net: &mut PvpNet, msg: &ServerRoomMsg) -> Option<Action> {
    match msg {
        ServerRoomMsg::Welcome { name } => {
            net.server_name = Some(name.clone());
            None
        }
        ServerRoomMsg::JoinedRoom(view) => {
            net.room = Some(view.summary.id);
            net.view = Some(view.clone());
            Some(Action::EnteredRoom)
        }
        ServerRoomMsg::LeftRoom(_) => {
            net.room = None;
            net.view = None;
            Some(Action::LeftRoom)
        }
        ServerRoomMsg::RoomClosed(_) => {
            if net.room.is_some() {
                net.room = None;
                net.view = None;
                Some(Action::LeftRoom)
            } else {
                None
            }
        }
        ServerRoomMsg::RoomUpdate(summary) => {
            if let Some(view) = &mut net.view {
                if view.summary.id == summary.id {
                    view.summary = summary.clone();
                }
            }
            None
        }
        ServerRoomMsg::Started { your_army, ini_seed } => {
            net.ini_seed = *ini_seed;
            Some(Action::Started(*your_army))
        }
        ServerRoomMsg::Kicked(_) => {
            net.room = None;
            net.view = None;
            Some(Action::LeftRoom)
        }
        _ => None,
    }
}

/// Действия UI, выведенные из событий.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    EnteredRoom,
    LeftRoom,
    /// Старт боя: your_army.
    Started(usize),
}
