use std::sync::Arc;
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
use futures_util::StreamExt;
use tokio::sync::oneshot;
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};
use tungstenite::client::IntoClientRequest;


pub struct IncomingEvent((Vec<Army>, BattleInfo));
pub struct OutcomingEvent((usize, usize));

pub struct Connection {
    pub incoming_events: futures_channel::mpsc::Receiver<IncomingEvent>,
    pub events_sender: futures_channel::mpsc::Sender<OutcomingEvent>,
    // tasks_handle: JoinAll<JoinHandle<()>>,
}

pub async fn connect(room: &str, id: &str) -> Connection {
    let mut request = "ws://localhost:3000/ws".into_client_request().unwrap();
    {
        let headers = request.headers_mut();
        headers.insert("room-code", room.parse().unwrap());
        headers.insert("player-id", id.parse().unwrap());

        // welp you're gonna need to serialize it to a safe string
        // let initial_data = Vec::new();
        // alkahest::serialize_to_vec((0, 0), &mut initial_data);
        // headers.insert("init-data", initial_data);
    }

    // Connect to an echo server
    let (ws_stream, _) = connect_async(request)
        .await
        .expect("failed to connect to the server");

    let (write, read) = ws_stream.split();

    // ie = incoming events, oe = outcoming events
    let (ie_tx, ie_rx) = futures_channel::mpsc::channel::<IncomingEvent>(256);
    let (oe_tx, oe_rx) = futures_channel::mpsc::channel::<OutcomingEvent>(256);

    let i = read.filter_map(|msg| async {
        let Message::Binary(e) = msg.ok()? else {return None};
        let data =
            alkahest::deserialize::<(Vec<Army>, BattleInfo), (Vec<Army>, BattleInfo)>(&e).ok()?;
        Some(Ok(IncomingEvent(data)))
    }).forward(ie_tx);

    let o = oe_rx.map(|OutcomingEvent(msg)| {
        let mut result = Vec::new();
        alkahest::serialize_to_vec::<(usize, usize), (usize, usize)>(msg, &mut result);
        Ok(Message::binary(result))
    }).forward(write);

    tokio::spawn(async move {
        tokio::select! {
            _ = i => (),
            _ = o => (),
        }
    });

    Connection {
        incoming_events: ie_rx,
        events_sender: oe_tx,
    }
}
#[cfg(test)]
mod tests {
    use futures_util::StreamExt;
    use tokio::sync::oneshot;
    use crate::connect;

    #[tokio::test]
    async fn test() {
        let mut server_proc = tokio::process::Command::new(tokio::fs::canonicalize("../dt/dt_server").await.unwrap())
            .current_dir("../dt/")
            .spawn()
            .unwrap();

        tokio::time::sleep(std::time::Duration::from_secs(1)).await;

        let mut conn = connect("000000", "1").await;

        conn.events_sender.try_send(crate::OutcomingEvent((0,0))).unwrap();
        tokio::spawn(conn.incoming_events.for_each(|msg| async {
            // dbg!(msg);
            //
            // HEEYAWYEYYAYSDYAYS READ THIS
            //  SO BASICALLY
            //  YOU SHOULD CHANGE THE TYPE OF INCOMINGEVENT TO YOUR NEED
            // THE SERVER CAN RETURN AN ERROR OR SOME SHIT
            //     HANDLE THAT
            ()
        }));

        // stop_signal.0.send(()).unwrap();
        server_proc.kill().await.unwrap();
    }
}
