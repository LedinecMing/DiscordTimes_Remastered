mod hotel;

use crate::hotel::*;
use alkahest::{deserialize, serialize, serialize_to_vec, serialized_size, Formula};
use axum::{
    extract::{
        ws::{Message as AWsMessage, WebSocket, WebSocketUpgrade},
        State as AState,
    },
    http::HeaderMap,
    Error as AError, Router,
};
use futures::{
    future, stream::{self, SplitSink, SplitStream}, SinkExt, StreamExt
};
use serde::{Deserialize, Serialize};
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
	async fn send(&mut self, message: AWsMessage) {
		self.sink.send(message).await;
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
    parse::{parse_items, parse_objects, parse_settings, parse_story, parse_units},
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
#[derive(alkahest::Deserialize, alkahest::Serialize, Formula)]
pub enum Outcoming {
	Battle((Vec<Army>, BattleInfo)),
	Status([bool; 2]),
}
#[derive(alkahest::Deserialize, alkahest::Serialize, Formula)]
pub enum Incoming {
	Action(BattleUnit),
	SetItem((BattleUnit, (usize, Option<usize>))),
	SetUnit((BattleUnit, Option<usize>)),
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
    for _ in 0..10 {
        army.add_troop(Troop::new(units.get(4).unwrap().clone()).into())
            .ok();
    }
    army
}
static UNITS: LazyLock<Vec<Unit>> = LazyLock::new(|| parse_units(None).unwrap().0);
fn setup() -> State {
    let settings = parse_settings();
    let _ = parse_items(None, &settings.locale);
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
	AWsMessage::Binary(buf)
}
fn process_message(message: AWsMessage, instance: &mut RoomInstance, army: usize) -> Result<AWsMessage, AError> {
	let action = deserialize::<Incoming, Incoming>(&message.into_data());
	match action {
		Ok(Incoming::Action(action)) => {
			process_move(action, instance, army)
		},
		Ok(Incoming::SetItem((pos, (index, item_id)))) => {
			let army = pos.army;
			if let Some(troop) = instance.armies[army].get_troop(pos.index) {
				let unit = &mut troop.get().unit;
				unit.add_item(item_id.and_then(|index| Some(Item { index })), index);
			};
			Ok(serialize_gamemap(&instance.armies, &instance.battle))
		},
		Ok(Incoming::SetUnit((pos, unit_id))) => {
			let army = pos.army;
			let RoomInstance { armies, battle, .. } = instance;
			let pos: usize = pos.index;
			let index = armies[army].hitmap[pos];
			if let Some(index) = index {
				armies[army].troops.remove(index);
			}
			if let Some(unit_id) = unit_id {
				let new_unit = UNITS.get(unit_id).cloned();
				let Some(new_unit) = new_unit else { return Err(AError::new("fuck")) };
				let mut new_troop = Troop::new(new_unit);
				new_troop.pos = UnitPos::from_index(pos);
				armies[army].troops.push(new_troop.into());
			} else {
				
			}
			armies[army].recalc_army_hitmap();
			Ok(serialize_gamemap(&armies, &battle))
		},
		Ok(Incoming::Status(status)) => {
			instance.acceptance[army] = status;
			let mut buf = vec![];
			serialize_to_vec::<Outcoming, Outcoming>(Outcoming::Status(instance.acceptance), &mut buf);
			Ok(AWsMessage::Binary(buf))
		},
		Err(_) => { Err(AError::new(r#"shit happens ¯\_(ツ)_/¯"#)) }
	}
}
fn process_move(
    action: BattleUnit,
    instance: &mut RoomInstance,
    army: usize,
) -> Result<AWsMessage, AError> {
    let RoomInstance { armies, battle, .. } = instance;
    if battle.active_unit.is_some_and(|x| x.army != army) {
		return Err(AError::new("Not your turn".to_owned()));
    }
    let (target_army, target_index) = (action.army, action.index);
    
    handle_action((target_army, target_index), battle, armies);
	Ok(serialize_gamemap(&armies, &battle))
}
#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Create a hotel with a room
    let state = setup();
    let hotel = state.hotel;

    // Start the server
    let app = Router::new()
        .route("/ws", axum::routing::any(ws_handler))
        .with_state(hotel);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000").await?;
    let server = tokio::spawn(async {
        axum::serve(
            listener,
            app.into_make_service_with_connect_info::<SocketAddr>(),
        )
        .await
        .unwrap();
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
        handle_socket(socket, room_code, (hotel, RoomInstance::new(&UNITS)))
    })
}

async fn handle_socket(
    mut socket: WebSocket,
    room_code: String,
    (hotel, mut instance): (Arc<Mutex<Hotel>>, RoomInstance),
) {
	{
		let buf = serialize_gamemap(&instance.armies, &instance.battle);
		socket.send(buf);
		if let Some(Some((Some(sock), None))) = hotel.lock().await.0.get_mut(&room_code) {
			dbg!("Second player");
			socket.send(AWsMessage::Text("Room full".to_owned())).await.expect("Wtf?");
			sock.send(AWsMessage::Text("Room full".to_owned())).await;
		} else {
			dbg!("First player");
			socket.send(AWsMessage::Text("Wait for another player".to_owned())).await.expect("Wtf?");
		}
	}
	dbg!("Creating room");
    let mut room = {
        let mut hotel = hotel.lock().await;
        match hotel.put_socket(room_code, socket).await {
            Ok(None) => {
				return;
			},
            Ok(Some(room)) => room,
            Err((mut socket, e)) => {
                socket.send(AWsMessage::Text(e.to_string())).await.unwrap();
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
	dbg!("Room established");
    fn stream_with_id<T, E>(
        stream: impl StreamExt<Item = Result<T, E>>,
        id: usize,
    ) -> impl StreamExt<Item = (T, usize)> {
        stream
            .filter_map(|x| futures::future::ready(x.ok()))
            .map(move |x| (x, id))
    }
    futures::stream::select(
        stream_with_id(room.0.stream, 0),
        stream_with_id(room.1.stream, 1),
    )
		.map(|(x, id)| process_message(x, &mut instance, id))
		.filter(|x| future::ready(x.is_ok()))
		.forward(room.1.sink.fanout(room.0.sink))
    .await
		.ok();
}
