use futures_util::StreamExt;
use tokio::sync::oneshot;
use dt_client::*;

#[tokio::main]
async fn main() {
    // let mut server_proc = tokio::process::Command::new(tokio::fs::canonicalize("../dt/dt_server").await.unwrap())
    //     .current_dir("../dt/")
    //     .spawn()
    //     .unwrap();

    tokio::time::sleep(std::time::Duration::from_secs(1)).await;

    let mut conn = connect("000000".to_owned(), "1".to_owned()).await;

    conn.events_sender.try_send(OutcomingEvent((0,0))).unwrap();
    tokio::spawn(conn.incoming_events.for_each(|msg| async {
        dbg!(msg);
        //
        // HEEYAWYEYYAYSDYAYS READ THIS
        //  SO BASICALLY
        //  YOU SHOULD CHANGE THE TYPE OF INCOMINGEVENT TO YOUR NEED
        // THE SERVER CAN RETURN AN ERROR OR SOME SHIT
        //     HANDLE THAT
        ()
    }));
}
