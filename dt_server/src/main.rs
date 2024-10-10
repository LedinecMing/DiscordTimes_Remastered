mod hotel;

use crate::hotel::*;
use axum::extract::ws::Message as AWsMessage;
use axum::extract::ws::{WebSocket, WebSocketUpgrade};
use axum::extract::State as AState;
use axum::http::HeaderMap;
use axum::Router;
use futures::stream::{SplitSink, SplitStream};
use futures::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::error::Error;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, Mutex};
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::tungstenite::protocol::Message as TWsMessage;

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Hash)]
enum Message {
    Event1(String),
    Event2(isize),
}

struct WsSocket {
    stream: SplitStream<WebSocket>,
    sink: SplitSink<WebSocket, AWsMessage>,
}

impl From<WebSocket> for WsSocket {
    fn from(val: WebSocket) -> Self {
        let (sink, stream) = val.split();
        WsSocket { stream, sink }
    }
}
use std::{
    collections::HashMap,
    time::Instant,
};

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

enum ServerService {
    Matchmaking,
}
#[derive(Clone, Debug)]
pub struct State {
    pub gamemap: GameMap,
    pub battle: Option<BattleInfo>,
    pub connection: Option<ConnectionManager>,
    pub gameevents: Vec<Event>,
    pub gameloop_time: Duration,
    pub units: HashMap<usize, Unit>,
    pub objects: Vec<ObjectInfo>,
    pub pause: bool,
}
fn setup_connection(state: &mut State) {
    state.connection = Some(ConnectionManager {
        con: Connection::Host(GameServer::new(false)),
        gamemap: state.gamemap.clone(),
        battle: state.battle.clone(),
        events: state.gameevents.clone(),
        last_updated: Instant::now(),
    });
}
fn setup() {
    let settings = parse_settings();
    parse_items(None, &settings.locale);
    let res = parse_units(None);
    if let Err(err) = res {
        panic!("{}", err);
    }
    let Ok((units, req_assets)) = res else {
        panic!("Unit parsing error")
    };
    let objects = parse_objects().0;

    let (mut gamemap, gameevents) = parse_story(
        &units,
        &objects,
        &settings.locale,
        &settings.additional_locale,
    );
    gamemap.calc_hitboxes(&objects);
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Create a hotel with a room
    let hotel = Arc::new(Mutex::new(Hotel::new()));

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
    AState(hotel): AState<Arc<Mutex<Hotel>>>,
) -> impl axum::response::IntoResponse {
    let room_code = headers
        .get("room-code")
        .unwrap()
        .to_str()
        .unwrap()
        .to_owned();
    ws.on_upgrade(move |socket| handle_socket(socket, room_code, hotel))
}

async fn handle_socket(socket: WebSocket, room_code: String, hotel: Arc<Mutex<Hotel>>) {
    let room = {
        let mut hotel = hotel.lock().await;
        match hotel.put_socket(&room_code, socket) {
            Ok(None) => return,
            Ok(Some(room)) => room,
            Err((mut socket, e)) => {
                socket.send(AWsMessage::Text(e.to_string())).await.unwrap();
                socket.close().await.unwrap();
                return;
            }
        }
    };

    // A way to use `forward` with multiple clients is folding their sinks with fanout

    #[rustfmt::skip]
    tokio::try_join!(
        room.0.stream.forward(room.1.sink),
        room.1.stream.forward(room.0.sink),
    )
    .unwrap();
}
