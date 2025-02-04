use std::{io::{self, stdin}, time::Duration};

use dt_lib::hwid;
use futures_util::StreamExt;
use tokio::sync::oneshot;
use dt_client::*;

fn parse_duo_tuple<T: std::str::FromStr>(v: &str) -> Result<(T, T), &'static str> {
    let mut points = v.split(|ch: char| !ch.is_ascii_digit()).map(|string| {
        string
            .parse()
            .or_else(|_| Err("Couldn't parse given string"))
    });
    (|(r1, r2)| Ok((r1?, r2?)))((
        points.next().ok_or("").and_then(|v| v),
        points.next().ok_or("").and_then(|v| v),
    ))
}

#[tokio::main]
async fn main() {
    // let mut server_proc = tokio::process::Command::new(tokio::fs::canonicalize("../dt/dt_server").await.unwrap())
    //     .current_dir("../dt/")
    //     .spawn()
    //     .unwrap();

	let mut room = String::new();
	stdin().read_line(&mut room);
	room = room.trim().to_owned();
    let mut conn = connect(room, hwid::get_id().unwrap()).await;

    //conn.events_sender.try_send(OutcomingEvent((0,0))).unwrap();
    // tokio::spawn(conn.incoming_events.for_each(|msg| async {
    //     dbg!(msg);
    //     //
    //     // HEEYAWYEYYAYSDYAYS READ THIS
    //     //  SO BASICALLY
    //     //  YOU SHOULD CHANGE THE TYPE OF INCOMINGEVENT TO YOUR NEED
    //     // THE SERVER CAN RETURN AN ERROR OR SOME SHIT
    //     //     HANDLE THAT
    //     ()
    // }));
	println!("Connection established");
	loop {
		let mut action = String::new();
		stdin().read_line(&mut action);
		action = action.trim().to_owned();
		let Ok(action) = parse_duo_tuple::<usize>(&action) else {
			println!("Wrong action!");
			continue;
		};
		println!("Sent action");
	} 
}
