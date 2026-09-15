//! Мультикомнатный ПВП-сервер (этап 1 §3 техплана).
//!
//! Архитектура: AppState { hub: Arc<Mutex<LobbyHub>>, battles:
//! Arc<Mutex<HashMap<RoomId, RoomBattle>>>, mods }. Комнаты живут в
//! LobbyHub (dt_lib::network::lobby), подключение = регистрация сессии,
//! НЕ создание игры. Битва (армии + BattleInfo) принадлежит комнате
//! (RoomBattle) и создаётся при старте хостом.
//!
//! Транспорт: ws /ws?name=Ник. Кадры Text = serde_json
//! ClientRoomMsg/ServerRoomMsg (комнатный контроль + чат); Binary =
//! alkahest Incoming/Outcoming (бой, как раньше). Входящее от сокета →
//! по сессии в комнату; исходящее → адресно или broadcast через writers.

mod battle_room;

use crate::battle_room::{incoming_from_bytes, Incoming, RoomBattle};
use axum::{
    extract::{
        ws::{Message as AWsMessage, WebSocket, WebSocketUpgrade},
        Query, State as AState,
    },
    response::IntoResponse,
    Router,
};
use dt_lib::network::lobby::{LobbyHub, Outgoing, Session, SocketId};
use dt_lib::network::room::{ClientRoomMsg, RoomId, RoomStatus, ServerRoomMsg};
use dt_lib::registry::GameInfo;
use futures::{SinkExt, StreamExt};
use serde::Deserialize;
use std::{
    collections::HashMap,
    error::Error,
    net::SocketAddr,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
};
use tokio::sync::{mpsc, Mutex};

/// Писатель сокета: исходящая очередь (кадры в формате axum).
type Tx = mpsc::UnboundedSender<AWsMessage>;

#[derive(Clone)]
pub struct AppState {
    pub hub: Arc<Mutex<LobbyHub>>,
    pub battles: Arc<Mutex<HashMap<RoomId, RoomBattle>>>,
    pub writers: Arc<Mutex<HashMap<SocketId, Tx>>>,
    pub mods: Vec<Arc<GameInfo>>,
}

static PORT: AtomicU64 = AtomicU64::new(3000);
static NEXT_SOCKET: AtomicU64 = AtomicU64::new(0);

#[derive(Deserialize)]
struct WsQuery {
    #[serde(default)]
    name: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let state = setup().await;
    let app = Router::new()
        .route("/ws", axum::routing::any(ws_handler))
        .with_state(state);
    let port = PORT.load(Ordering::Acquire);
    println!("dt_server: listening on 0.0.0.0:{port}");
    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{port}")).await?;
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await?;
    Ok(())
}

async fn setup() -> AppState {
    let mut registry = GameInfo::new();
    dt_lib::parse::parse_units::<dt_lib::parse::StupidReader>(None, &mut registry).await;
    let settings = dt_lib::parse::parse_settings::<dt_lib::parse::StupidReader>().await;
    PORT.store(settings.port, Ordering::Relaxed);
    let _ = dt_lib::parse::parse_items::<dt_lib::parse::StupidReader>(
        None,
        &settings.locale,
        &mut registry,
    )
    .await;
    AppState {
        hub: Arc::new(Mutex::new(LobbyHub::new())),
        battles: Arc::new(Mutex::new(HashMap::new())),
        writers: Arc::new(Mutex::new(HashMap::new())),
        mods: vec![Arc::new(registry)],
    }
}

async fn ws_handler(
    ws: WebSocketUpgrade,
    Query(query): Query<WsQuery>,
    AState(state): AState<AppState>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, state, query.name))
}

/// Полный цикл одного соединения: регистрация сессии, цикл чтения кадров,
/// разбор Text/Binary, адресная доставка исходящих, уборка при обрыве.
async fn handle_socket(socket: WebSocket, state: AppState, name_hint: String) {
    let (mut sink, mut stream_in) = socket.split();
    let (tx, mut rx) = mpsc::unbounded_channel::<AWsMessage>();

    // Регистрация сессии. Ник — из query (?name=) либо первым кадром Hello.
    let socket_id = NEXT_SOCKET.fetch_add(1, Ordering::Relaxed);
    let session = if name_hint.trim().is_empty() {
        Session::new()
    } else {
        Session::named(name_hint.trim())
    };
    state.writers.lock().await.insert(socket_id, tx.clone());
    {
        let mut hub = state.hub.lock().await;
        hub.connect(session);
        if let Some(s) = hub.sessions.get(&socket_id) {
            if s.is_named() {
                let _ = tx.send(text_msg(&ServerRoomMsg::Welcome {
                    name: s.name.clone(),
                }));
            }
        }
    }
    println!("[ws] socket #{socket_id} connected");

    // Задача-писатель: единственный владелец sink.
    let writer = tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            if sink.send(msg).await.is_err() {
                break;
            }
        }
    });

    // Цикл чтения.
    while let Some(frame) = stream_in.next().await {
        let Ok(frame) = frame else { break };
        match frame {
            AWsMessage::Close(_) => break,
            AWsMessage::Text(raw) => {
                let msg: ClientRoomMsg = match serde_json::from_str(&raw) {
                    Ok(m) => m,
                    Err(e) => {
                        let _ = tx.send(text_msg(&ServerRoomMsg::Error {
                            message: format!("bad frame: {e}"),
                        }));
                        continue;
                    }
                };
                let outputs = on_room_msg(&state, socket_id, msg).await;
                deliver(&state, outputs).await;
            }
            AWsMessage::Binary(data) => {
                on_battle_frame(&state, socket_id, &data).await;
            }
            AWsMessage::Ping(_) | AWsMessage::Pong(_) => {}
        }
    }

    // Уборка: сессия, комната, писатель.
    let outputs = state.hub.lock().await.disconnect(socket_id);
    deliver(&state, outputs).await;
    state.writers.lock().await.remove(&socket_id);
    drop(tx);
    let _ = writer.await;
    println!("[ws] socket #{socket_id} disconnected");
}

/// Комнатное сообщение: через LobbyHub; старт боя дополнительно создаёт
/// RoomBattle и рассылает армии (Outcoming::Battle) всей комнате.
async fn on_room_msg(state: &AppState, socket_id: SocketId, msg: ClientRoomMsg) -> Vec<Outgoing> {
    let start_room = matches!(msg, ClientRoomMsg::StartBattle);
    let mut outputs = {
        let mut hub = state.hub.lock().await;
        hub.handle(socket_id, msg)
    };
    if !start_room {
        return outputs;
    }
    // Хост стартовал: если комната теперь InGame — создаём игру.
    let (room, players): (RoomId, Vec<String>) = {
        let hub = state.hub.lock().await;
        match hub.sessions.get(&socket_id).and_then(|s| s.room) {
            Some(room)
                if hub.rooms.get(room).map(|r| r.status) == Some(RoomStatus::InGame) =>
            {
                let players = hub
                    .rooms
                    .get(room)
                    .map(|r| r.players.clone())
                    .unwrap_or_default();
                (room, players)
            }
            _ => return outputs,
        }
    };
    let registry = state.mods[0].clone();
    let battle = RoomBattle::new(&players, &registry);
    // Снапшот битвы всем (до Started-обработки клиентом — порядок не важен:
    // кадры независимы).
    broadcast_frame(&state, room, &battle.state_frame()).await;
    // Started{your_army, ini_seed} каждому игроку.
    for (idx, out) in hub_started_messages(&state, room).await.into_iter().enumerate() {
        outputs.push(out);
        let _ = idx;
    }
    state.battles.lock().await.insert(room, battle);
    println!("[room {room}] battle started");
    outputs
}

/// Started-сообщения: каждому игроку комнаты с его your_army (общий сид
/// уже в ini_seed сессии старта — берём из LobbyHub: монетка решается
/// детерминированно по нему).
async fn hub_started_messages(state: &AppState, room: RoomId) -> Vec<Outgoing> {
    let hub = state.hub.lock().await;
    let Some(room_data) = hub.rooms.get(room) else {
        return vec![];
    };
    let seed = room_data
        .players
        .first()
        .map(|_| dt_lib::network::room::now_unix())
        .unwrap_or(0);
    room_data
        .players
        .iter()
        .enumerate()
        .map(|(idx, player)| {
            let sock = hub
                .room_sockets(room)
                .into_iter()
                .find(|s| hub.sessions.get(s).map(|x| x.name.as_str()) == Some(player.as_str()));
            Outgoing {
                to: sock,
                msg: ServerRoomMsg::Started {
                    your_army: idx,
                    ini_seed: seed,
                },
            }
        })
        .collect()
}

/// Бинарный кадр боя: строго для сессии в комнате с битвой. Номер армии —
/// по имени в room.players; логика — порт process_message старого сервера.
async fn on_battle_frame(state: &AppState, socket_id: SocketId, data: &[u8]) {
    let Some(incoming) = incoming_from_bytes(data) else {
        return;
    };
    if matches!(incoming, Incoming::Disconnect) {
        return;
    }
    let (room, army) = {
        let hub = state.hub.lock().await;
        let Some(session) = hub.sessions.get(&socket_id) else {
            return;
        };
        match session.room {
            Some(room) => {
                let army = hub
                    .rooms
                    .get(room)
                    .and_then(|r| r.players.iter().position(|p| *p == session.name))
                    .unwrap_or(usize::MAX);
                (room, army)
            }
            None => return,
        }
    };
    let registry = state.mods[0].clone();
    let (reject, broadcast) = {
        let mut battles = state.battles.lock().await;
        let Some(battle) = battles.get_mut(&room) else {
            return;
        };
        if army == usize::MAX && !matches!(incoming, Incoming::GetState) {
            (Some("Наблюдатель: только просмотр".into()), false)
        } else {
            battle.process(incoming, army, &registry)
        }
    };
    if let Some(text) = reject {
        if let Some(tx) = state.writers.lock().await.get(&socket_id) {
            let _ = tx.send(text_msg(&ServerRoomMsg::Error { message: text }));
        }
        return;
    }
    if broadcast {
        let battles = state.battles.lock().await;
        if let Some(battle) = battles.get(&room) {
            broadcast_frame(state, room, &battle.state_frame()).await;
        }
    }
}

/// Binary-кадр всей комнате через writers.
async fn broadcast_frame(state: &AppState, room: RoomId, frame: &AWsMessage) {
    let targets = state.hub.lock().await.room_sockets(room);
    let writers = state.writers.lock().await;
    for sock in targets {
        if let Some(tx) = writers.get(&sock) {
            let _ = tx.send(frame.clone());
        }
    }
}

/// Доставка Outgoing-пакетов: адресно или в общий броадкаст (Text-кадры).
async fn deliver(state: &AppState, outputs: Vec<Outgoing>) {
    if outputs.is_empty() {
        return;
    }
    let writers = state.writers.lock().await;
    for out in outputs {
        let frame = text_msg(&out.msg);
        match out.to {
            Some(sock) => {
                if let Some(tx) = writers.get(&sock) {
                    let _ = tx.send(frame.clone());
                }
            }
            None => {
                for tx in writers.values() {
                    let _ = tx.send(frame.clone());
                }
            }
        }
    }
}

fn text_msg(msg: &ServerRoomMsg) -> AWsMessage {
    AWsMessage::Text(serde_json::to_string(msg).expect("server room msg serialize").into())
}
