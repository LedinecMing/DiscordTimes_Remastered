use crate::WsSocket;
use axum::extract::ws::WebSocket as AWsSocket;
use std::collections::HashMap;

pub type RoomCode = String;

// None => Room full
// Some(None, None) => Room empty
// Some(Some(..), None) => Room half-full (or half-empty smh)
// The rest is unreachable
pub struct Hotel(pub HashMap<RoomCode, Option<(Option<WsSocket>, Option<WsSocket>)>>);

impl Hotel {
    pub fn new() -> Self {
        Hotel(HashMap::new())
    }

    pub fn create_room(&mut self, room_code: RoomCode) {
        self.0.insert(room_code, Some((None, None)));
    }

    // If the second player joined, returns both sockets and sets the room full
    pub fn put_socket(
        &mut self,
        room_code: &RoomCode,
        socket: AWsSocket,
    ) -> Result<Option<(WsSocket, WsSocket)>, (AWsSocket, &'static str)> {
        let Some(room_ext) = self.0.get_mut(room_code) else {
            return Err((socket, "Room doesn't exist"));
        };
        let Some(room) = room_ext else {
            return Err((socket, "Room is full"));
        };
        match (&room.0, &room.1) {
            (None, None) => {
                room.0 = Some(socket.into());
                Ok(None)
            }
            (Some(_), None) => {
                room.1 = Some(socket.into());
                let Some((Some(s1), Some(s2))) = room_ext.take() else {
                    unreachable!()
                };
                Ok(Some((s1, s2)))
            }
            _ => unreachable!(),
        }
    }
}
