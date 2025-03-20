mod hotel;

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{atomic::AtomicU16, RwLock};
use std::fmt::Debug;
use crate::hotel::*;
use alkahest::{deserialize, serialize, serialize_to_vec, serialized_size, Formula};
use axum::body::Bytes;
use axum::{
    extract::{
        ws::{Message as AWsMessage, WebSocket, WebSocketUpgrade},
        State as AState,
    },
    http::HeaderMap,
    Error as AError, Router,
};
use futures::{
    future::{self, ready}, stream::{self, SplitSink, SplitStream}, SinkExt, StreamExt
};
use serde::{Deserialize, Serialize};
use tokio::task::JoinSet;
use tokio::time::{interval, sleep};
use std::{
    cell::LazyCell, collections::HashSet, error::Error, net::SocketAddr, ops::Bound, sync::{Arc, LazyLock}, task::Context, time::Duration
};
use tokio::sync::{mpsc, Mutex};
use tokio_tungstenite::{
    connect_async,
    tungstenite::{client::IntoClientRequest, protocol::Message as TWsMessage},
};

struct WsSocket {
    stream: SplitStream<WebSocket>,
    sink: SplitSink<WebSocket, AWsMessage>,
}
impl WsSocket {
	async fn send(&mut self, message: AWsMessage) -> Result<(), AError> {
		self.sink.send(message).await
	}
	async fn req_one(&mut self) -> Option<Result<AWsMessage, AError>> {
		self.stream.next().await
	}
}
impl From<WebSocket> for WsSocket {
    fn from(val: WebSocket) -> Self {
        let (sink, stream) = val.split();
        WsSocket { stream, sink }
    }
}
use std::{collections::HashMap, time::Instant};

use dt_lib::{
    battle::{army::*, battlefield::*, troop::Troop},
    items::item::*,
    locale::{parse_locale, Locale},
    map::{
        event::{execute_event, Event, Execute},
        map::*,
        object::ObjectInfo,
        tile::*,
    },
    network::net::*,
    parse::{parse_items, parse_objects, parse_settings, parse_story, parse_units, StupidReader},
    time::time::Data as TimeData,
    units::{
        unit::{ActionResult, Unit, UnitPos},
        unitstats::ModifyUnitStats,
    },
};
type MutRc<T> = Arc<Mutex<T>>;
enum ServerService {
    Matchmaking,
}
enum InstanceError {
	Str(&'static str),
	Fatal
}
#[derive(alkahest::Deserialize, alkahest::Serialize, Formula)]
pub enum Outcoming {
	Id(usize),
	Battle((Vec<Army>, BattleInfo)),
	Status([bool; 2]),
}
#[derive(alkahest::Deserialize, alkahest::Serialize, Formula)]
pub enum Incoming {
	Disconnect,
	GetState,
	Action(BattleUnitPos),
	SetItem((BattleUnitPos, (usize, Option<usize>))),
	SetUnit((BattleUnitPos, Option<usize>)),
	Status(bool),
}
#[derive(Clone)]
pub struct State {
    pub hotel: MutRc<Hotel>,
}
pub enum RoomStatus {
	Preparing,
	Battle,
	End
}
struct RoomInstance {
    pub armies: Vec<Army>,
    pub battle: BattleInfo,
	pub status: RoomStatus,
	pub acceptance: [bool;2]
}
impl RoomInstance {
    pub fn new(units: &Vec<Unit>) -> Self {
        let mut armies = Vec::new();
        armies.push(gen_army(0, units));
        armies.push(gen_army(1, units));
        let battle = BattleInfo::new(&mut armies, 0, 1);
        Self { armies, battle, acceptance: [false, false], status: RoomStatus::Preparing }
    }
}
fn gen_army(army_num: usize, units: &Vec<Unit>) -> Army {
    let mut army = Army::new(
        vec![],
        ArmyStats {
            gold: 0,
            mana: 0,
            army_name: String::new(),
        },
        vec![None, None, None, None],
        (0, 0),
        true,
        dt_lib::battle::control::Control::PC,
    );
    // for _ in 0..12 {
	// 	let mut unit = units.get(4).unwrap().clone();
	// 	unit.inventory.items = vec![None; 4];
    //     army.add_troop(Troop::new(units.get(4).unwrap().clone()).into())
    //         .ok();
    //}
    army
}
static UNITS: LazyLock<RwLock<Vec<Unit>>> = LazyLock::new(|| RwLock::new(vec![]));
macro_rules! units {
    () => {
		&UNITS.read().unwrap()
    };
}
macro_rules! units_mut {
    () => {
        &mut UNITS.write().unwrap()
    };
}
static PORT: AtomicU64 = AtomicU64::new(3000);
async fn setup() -> State {
	units_mut!().append(&mut parse_units::<StupidReader>(None).await.unwrap().0);
    let settings = parse_settings::<StupidReader>().await;
	PORT.store(settings.port, Ordering::Relaxed);
    let _ = parse_items::<StupidReader>(None, &settings.locale).await;
    State {
        hotel: Arc::new(Mutex::new(Hotel::new())),
    }
}
fn serialize_gamemap(armies: &Vec<Army>, battle: &BattleInfo) -> AWsMessage {
	let msg = (armies.clone(), battle.clone());
    let mut buf = vec![];
    serialize_to_vec::<Outcoming, Outcoming>(
        Outcoming::Battle(msg.clone()),
        &mut buf,
    );
	assert!(deserialize::<Outcoming, Outcoming>(&buf).is_ok());
	AWsMessage::Binary(buf.into())
}
fn process_message(message: Result<AWsMessage, AError>, instance: &mut RoomInstance, army: usize) -> Result<AWsMessage, InstanceError> {
	let message = match message {
		Ok(m) => m,
		Err(err) => {
			dbg!(err);
			return Err(InstanceError::Fatal);
		}
	};
	if let AWsMessage::Close(_) = message {
		dbg!("I close");
		return Err(InstanceError::Fatal);
	}
	let action = deserialize::<Incoming, Incoming>(&message.into_data());
	match action {
		Ok(Incoming::Disconnect) => {
			return Err(InstanceError::Fatal);
		}
		Ok(Incoming::GetState) => {
			Ok(serialize_gamemap(&instance.armies, &instance.battle))
		}
		Ok(Incoming::Action(action)) => {
			process_move(action, instance, army)
		},
		Ok(Incoming::SetItem((pos, (index, item_id)))) => {
			if instance.acceptance == [true, true] {
				return Err(InstanceError::Str("pashalka"));
			}
			let army = pos.army;
			let send = instance.armies[army].set_item_unit_at(item_id, pos, index);
			if send {
				Ok(serialize_gamemap(&instance.armies, &instance.battle))
			} else  { Err(InstanceError::Str("pashalka")) }
		},
		Ok(Incoming::SetUnit((pos, unit_id))) => {
			let army = pos.army;
			let RoomInstance { armies, battle, .. } = instance;
			if instance.acceptance == [true, true] {
				return Err(InstanceError::Str("pashalka"));
			}
			armies[army].set_unit_at(unit_id, pos);
			Ok(serialize_gamemap(&armies, &battle))
		},
		Ok(Incoming::Status(status)) => {
			if instance.acceptance == [true, true] {
				return Err(InstanceError::Str("pashalka"));
			}
			instance.acceptance[army] = status;
			if instance.acceptance == [true, true] {
				instance.battle.start(&mut instance.armies);
			}
			let mut buf = vec![];
			serialize_to_vec::<Outcoming, Outcoming>(Outcoming::Status(instance.acceptance), &mut buf);
			Ok(AWsMessage::Binary(buf.into()))
		},
		Err(_) => { Err(InstanceError::Str(r#"shit happens ¯\_(ツ)_/¯"#)) }
	}
}
fn process_move(
    action: BattleUnitPos,
    instance: &mut RoomInstance,
    army: usize,
) -> Result<AWsMessage, InstanceError> {
    let RoomInstance { armies, battle, .. } = instance;
    if battle.active_unit.is_some_and(|x| x.army != army) {
		return Err(InstanceError::Str("Not your turn"));
    }
    let (target_army, target_pos) = (action.army, action.pos);
    handle_action((target_pos, target_army), battle, armies);
	Ok(serialize_gamemap(&armies, &battle))
}
#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Create a hotel with a room
    let state = setup().await;
    let hotel = state.hotel;

    // Start the server
    let app = Router::new()
        .route("/ws", axum::routing::any(ws_handler))
        .with_state(Arc::clone(&hotel));
	let port= dbg!(PORT.load(Ordering::Acquire));
    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", port)).await?;
    let server = tokio::spawn(async {
        axum::serve(
            listener,
            app.into_make_service_with_connect_info::<SocketAddr>(),
        )
			.await
			.unwrap();
    });
	let hotel: Arc<_> = Arc::clone(&hotel);
	tokio::spawn(async move {
		loop {
			sleep(Duration::from_secs(5)).await;
			let mut hotel = hotel.lock().await;
			let values = hotel.0.iter_mut();
			let mut futures = unicycle::FuturesUnordered::new();
			for (key, value) in values {
				futures.push( async move {
					(key.clone(), match value {
						Some((Some(ws), None)) => {
							let sent = ws.send(AWsMessage::Ping(Bytes::new())).await;
							sent.is_err() || ws.req_one().await.is_none_or(|x| x.is_err())
						},
						_ => { false }
					})
				});
			}
			let to_remove: Vec<_> = futures.filter_map(|x| async move {
				x.1.then(|| x.0)
			}).collect().await;
			if to_remove.len() > 0 {
				println!("Removed dead rooms: {}", &to_remove.join(" ||| "));
			}
			for key in to_remove {
				hotel.0.remove(&key);
			}
		}
	});
    server.await.unwrap();
    Ok(())
}

async fn ws_handler(
    ws: WebSocketUpgrade,
    headers: HeaderMap,
    AState(hotel): AState<MutRc<Hotel>>,
) -> impl axum::response::IntoResponse {
    let room_code = headers
        .get("room-code")
        .unwrap()
        .to_str()
        .unwrap()
        .to_owned();
    ws.on_upgrade(move |socket| {
        handle_socket(socket, room_code, (hotel, RoomInstance::new(units!())))
    })
}

async fn handle_socket(
    mut socket: WebSocket,
    room_code: String,
    (hotel, mut instance): (Arc<Mutex<Hotel>>, RoomInstance),
) {
	{
		let buf = serialize_gamemap(&instance.armies, &instance.battle);
		socket.send(buf).await;
		if let Some(Some((Some(sock), None))) = hotel.lock().await.0.get_mut(&room_code) {
			dbg!("Second player");
			socket.send(AWsMessage::Text("Room full".into())).await.expect("Wtf?");
			if sock.send(AWsMessage::Text("Room full".into())).await.is_ok() {
				let mut buf = vec![];
				serialize_to_vec::<Outcoming, Outcoming>(Outcoming::Id(1), &mut buf);
				socket.send(AWsMessage::Binary(buf.into())).await;
			} else {
				return;
			}
		} else {
			dbg!("First player");
			let mut buf = vec![];
			serialize_to_vec::<Outcoming, Outcoming>(Outcoming::Id(0), &mut buf);
			if socket.send(AWsMessage::Binary(buf.into())).await.is_ok() {
				socket.send(AWsMessage::Text("Wait for another player".into())).await.expect("Wtf?");
			} else {
				return;
			}
		}
	}
	println!("Creating room {}", room_code);
    let mut room = {
        let mut hotel = hotel.lock().await;
        match hotel.put_socket(room_code.clone(), socket).await {
            Ok(None) => {
				return;
			},
            Ok(Some(room)) => room,
            Err((mut socket, e)) => {
                socket.send(AWsMessage::Text(e.into())).await.unwrap();
                socket.close().await.unwrap();
                return;
            }
        }
    };
	{
		let buf = serialize_gamemap(&instance.armies, &instance.battle);
		room.0.send(buf.clone()).await;
		room.1.send(buf).await;
	}
	println!("Room established {}", &room_code);
    fn stream_with_id<T: Debug, E: Debug>(
        stream: impl StreamExt<Item = Result<T, E>>,
        id: usize,
    ) -> impl StreamExt<Item = (Result<T, E>, usize)> {
        stream
            .map(move |x| (x, id))
    }
    futures::stream::select(
        stream_with_id(room.0.stream, 0),
        stream_with_id(room.1.stream, 1),
    )
		.map(|(x, id)| process_message(x, &mut instance, id))
		.take_while(|x| future::ready(!matches!(x, Err(InstanceError::Fatal))))
		.filter_map(|x| {
			ready(if let Ok(x) = x {
				Some(Ok(x))
			} else { None })
		})
		.forward(room.1.sink.fanout(room.0.sink))
    .await
		.ok();
	hotel.lock().await.0.remove(&room_code);
	println!("Room destructed: {room_code}");
}
