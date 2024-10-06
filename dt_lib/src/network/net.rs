use crate::{
    battle::{
        army::{find_path, Army, TroopType},
        battlefield::{handle_action, Action, BattleInfo},
        troop::Troop,
    },
    map::{
        event::{execute_event, execute_event_as_player, Event, Execute},
        map::GameMap,
        object::ObjectInfo,
    },
    parse::SETTINGS,
    units::unit::{Unit, UnitInfo, UnitInventory, UnitLvl, UnitStats},
    Menu,
};
use alkahest::*;
use renet::{
    transport::{
        ClientAuthentication, ConnectToken, NetcodeClientTransport, NetcodeServerTransport,
        ServerAuthentication, ServerConfig,
    },
    ClientId, ConnectionConfig, DefaultChannel, RenetClient, RenetServer, ServerEvent,
};
use log;
use std::{
    collections::HashMap,
    io,
    mem::size_of,
    net::{IpAddr, Ipv4Addr, SocketAddr, UdpSocket},
    time::{Duration, Instant, SystemTime},
};

#[derive(Clone, Debug)]
#[alkahest(Deserialize, Serialize, SerializeRef, Formula)]
pub enum ClientMessage {
    Action((usize, usize)),
    MapClick((usize, usize)),
}

#[derive(Clone, Debug)]
#[alkahest(Deserialize, Serialize, SerializeRef, Formula)]
pub enum ServerMessage {
    State((Option<BattleInfo>, GameMap)),
    ChangeMenu(usize),
    Message(String),
}
#[derive(Debug)]
pub struct ClientConnection {
    pub client: Box<RenetClient>,
    pub transport: Box<NetcodeClientTransport>,
}
impl Clone for ClientConnection {
    fn clone(&self) -> Self {
        let (a, b) = create_renet_client();
        Self {
            client: Box::new(a),
            transport: Box::new(b),
        }
    }
}
#[derive(Debug)]
pub struct GameServer {
    pub server: Box<RenetServer>,
    pub transport: Box<NetcodeServerTransport>,
    pub auth: HashMap<ClientId, usize>,
}
impl Clone for GameServer {
    fn clone(&self) -> Self {
        GameServer {
            server: Box::new(RenetServer::new(ConnectionConfig::default())),
            transport: {
                let socket = UdpSocket::bind(*SERVER).unwrap();
                let current_time = SystemTime::now()
                    .duration_since(SystemTime::UNIX_EPOCH)
                    .unwrap();
                let server_config = ServerConfig {
                    current_time,
                    max_clients: 2,
                    protocol_id: PROTOCOL_ID,
                    public_addresses: vec![*SERVER],
                    authentication: ServerAuthentication::Unsecure,
                };

                Box::new(NetcodeServerTransport::new(server_config, socket).unwrap())
            },
            auth: self.auth.clone(),
        }
    }
}
pub const HOST_CLIENT_ID: ClientId = ClientId::from_raw(1);
pub const PROTOCOL_ID: u64 = 228;
pub const ADDR: SocketAddr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 0);
pub static SERVER: once_cell::sync::Lazy<SocketAddr> = once_cell::sync::Lazy::new(|| {
    format!("127.0.0.1:{}", unsafe { &SETTINGS }.port)
        .parse()
        .unwrap()
});
impl GameServer {
	/// Consutrct a new GameServer, pass a boolean indicating if server itself is a player or not.
    pub fn new(host_is_player: bool) -> Self {
        let socket = UdpSocket::bind(*SERVER).unwrap();
        let current_time = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap();
        let server_config = ServerConfig {
            current_time,
            max_clients: 64,
            protocol_id: PROTOCOL_ID,
            public_addresses: vec![*SERVER],
            authentication: ServerAuthentication::Unsecure,
        };
        let server = Box::new(RenetServer::new(ConnectionConfig::default()));
        let transport = Box::new(NetcodeServerTransport::new(server_config, socket).unwrap());

        let mut auth = HashMap::new();
		if host_is_player {
			auth.insert(HOST_CLIENT_ID, 0);
		}

        Self {
            server,
            transport,
            auth,
        }
    }
    fn try_to_send_message(
        &mut self,
        gamemap: &mut GameMap,
        army: usize,
        message: ServerMessage,
    ) -> Option<()> {
        if let Some(target_client_id) =
            self.auth
                .iter()
                .find_map(|(key, &val)| if val == army { Some(key) } else { None })
        {
            let size = serialized_size::<ServerMessage, _>(&message);
            let mut output = vec![0u8; size.0];
            serialize::<ServerMessage, ServerMessage>(message, &mut output).ok();
            self.server.send_message(
                *target_client_id,
                DefaultChannel::ReliableOrdered,
                renet::Bytes::copy_from_slice(&output),
            );
            Some(())
        } else {
            None
        }
    }
    fn send_message(
        &mut self,
        gamemap: &mut GameMap,
        client_id: ClientId,
        message: ServerMessage,
    ) -> Option<()> {
        let size = serialized_size::<ServerMessage, _>(&message);
        let mut output = vec![0u8; size.0];
        serialize::<ServerMessage, ServerMessage>(message, &mut output).ok();
        self.server.send_message(
            client_id,
            DefaultChannel::ReliableOrdered,
            renet::Bytes::copy_from_slice(&output),
        );
        Some(())
    }
	/// Used by server to process client's input
    pub fn handle_client_message(
        &mut self,
        client_id: Option<ClientId>,
        message: ClientMessage,
        gamemap: &mut GameMap,
        battle: &mut Option<BattleInfo>,
        gameevents: &mut Vec<Event>,
        units: &Vec<Unit>,
        objects: &Vec<ObjectInfo>,
    ) {
        match message {
            ClientMessage::Action(v) => {
				let Some(battle) = battle.as_mut() else { return; };
                if Some(client_id.and_then(|v| self.auth.get(&v)).unwrap_or(&0usize))
                    == battle.active_unit.and_then(|v| Some(v.0)).as_ref()
                {
                    handle_action(Action::Cell(v.0 as usize, v.1 as usize), battle, &mut gamemap.armys);
                    let message = ServerMessage::State((Some(battle.clone()), gamemap.clone()));
                    let size = serialized_size::<ServerMessage, _>(&message);
                    let mut output = vec![0u8; size.0];
                    serialize::<ServerMessage, ServerMessage>(message, &mut output).ok();

                    if let Some(client_id) = client_id {
                        self.server.send_message(
                            client_id,
                            DefaultChannel::ReliableOrdered,
                            renet::Bytes::copy_from_slice(&output),
                        );
                    } else {
                        self.server.send_message(
                            ClientId::from_raw(255),
                            DefaultChannel::ReliableOrdered,
                            renet::Bytes::copy_from_slice(&output),
                        );
                    }
                }
            }
            ClientMessage::MapClick(goal) => {
                let army_index = client_id
                    .and_then(|v| self.auth.get(&v).cloned())
                    .unwrap_or(0usize);
                if let Some(target_army) = gamemap.hitmap[goal.0][goal.1].army {
                    let Some(army) = gamemap.armys.get(army_index) else {
                        return;
                    };
                    let pos = army.pos;
                    let diff = (pos.0 as i64 - goal.0 as i64, pos.1 as i64 - goal.1 as i64);
                    if -1 <= diff.0 && diff.0 <= 1 && -1 <= diff.1 && diff.1 <= 1 {
                        if battle.is_none() {
                            let battle_new = BattleInfo::new(&mut gamemap.armys, target_army, army_index);
                            *battle = Some(battle_new);
                        }
                        let message = ServerMessage::ChangeMenu(Menu::Connect as usize);
                        self.try_to_send_message(gamemap, target_army, message.clone());
                        self.try_to_send_message(gamemap, army_index, message);
                    }
                } else {
                    let army_pos = {
                        let Some(army) = gamemap.armys.get_mut(army_index) else {
                            return;
                        };
                        army.pos
                    };
                    let path = find_path(&*gamemap, objects, army_pos, goal, false);
                    let Some(army) = gamemap.armys.get_mut(army_index) else {
                        return;
                    };
                    army.path = if let Some(path) = path {
                        path.0
                    } else {
                        Vec::new()
                    };
                };
            }
        }
    }
    pub fn update(
        &mut self,
        duration: Duration,
        gamemap: &mut GameMap,
        battle: &mut Option<BattleInfo>,
        gameevents: &mut Vec<Event>,
        units: &Vec<Unit>,
        objects: &Vec<ObjectInfo>,
    ) -> Result<(), io::Error> {
        self.server.update(duration);
        self.transport.update(duration, &mut self.server).unwrap();

        while let Some(event) = self.server.get_event() {
            match event {
                ServerEvent::ClientConnected { client_id } => {
                    let user_data = self.transport.user_data(client_id).unwrap();
                    log::info!("User is connecting");
                    if self.auth.values().any(|v| *v == 1) {
                        self.auth.insert(client_id, 0);
                    } else {
                        self.auth.insert(client_id, 1);
                    }
                    log::info!("Constructing server-client cross state");
                    let message = ServerMessage::State((battle.clone(), gamemap.clone()));
                    log::info!("Calculate message size");
                    let size = serialized_size::<ServerMessage, _>(&message);
                    log::info!("Message serialized");
                    log::info!("Constructing bytes buffer");
                    let mut output = vec![0u8; size.0];
                    log::info!("Serizalie message into buffer");
                    serialize::<ServerMessage, ServerMessage>(message, &mut output).ok();
                    log::info!("Sending greeting message!");
                    self.server.send_message(
                        client_id,
                        DefaultChannel::ReliableOrdered,
                        renet::Bytes::copy_from_slice(&output),
                    );
                    log::info!("Sent greeting message!");
                }
                ServerEvent::ClientDisconnected {
                    client_id,
                    reason: _,
                } => {
                    self.auth.remove(&client_id);
                }
            }
        }
        if self.server.connected_clients() > 0 {
            log::debug!("connected {} users", self.server.connected_clients());
        }

        for client_id in self.server.clients_id() {
            while let Some(message) = self
                .server
                .receive_message(client_id, DefaultChannel::ReliableOrdered)
            {
                let buf = message;
                let res: Result<ClientMessage, DeserializeError> =
                    deserialize::<ClientMessage, ClientMessage>(&*buf);
                if let Ok(message) = res {
                    log::info!("Received message from client {}: {:?}", client_id, message);
                    match message {
                        ClientMessage::Action(v) => {
							let Some(battle) = battle.as_mut() else { continue; };
                            if self.auth.get(&client_id)
                                == battle.active_unit.and_then(|v| Some(v.0)).as_ref()
                            {
                                handle_action(
                                    Action::Cell(v.0 as usize, v.1 as usize),
                                    battle,
                                    &mut gamemap.armys,
                                );
                                let message =
                                    ServerMessage::State((Some(battle.clone()), gamemap.clone()));
                                let size = serialized_size::<ServerMessage, _>(&message);
                                let mut output = vec![0u8; size.0];
                                serialize::<ServerMessage, ServerMessage>(message, &mut output)
                                    .ok();

                                self.server.send_message(
                                    client_id,
                                    DefaultChannel::ReliableOrdered,
                                    renet::Bytes::copy_from_slice(&output),
                                );
                            }
                        }
                        ClientMessage::MapClick(goal) => {
                            let Some(army_index) = self.auth.get(&client_id).and_then(|v| Some(*v))
                            else {
                                continue;
                            };
                            if let Some(target_army) = gamemap.hitmap[goal.0][goal.1].army {
                                let Some(army) = gamemap.armys.get(army_index) else {
                                    continue;
                                };
                                let pos = army.pos;
                                let diff =
                                    (pos.0 as i64 - goal.0 as i64, pos.1 as i64 - goal.1 as i64);
                                if -1 <= diff.0 && diff.0 <= 1 && -1 <= diff.1 && diff.1 <= 1 {
                                    if battle.is_none() {
                                        let battle_new =
                                            BattleInfo::new(&mut gamemap.armys, target_army, army_index);
                                        *battle = Some(battle_new);
                                    }
                                    let message = ServerMessage::ChangeMenu(Menu::Connect as usize);
                                    self.try_to_send_message(gamemap, army_index, message.clone());
                                    self.try_to_send_message(gamemap, target_army, message);
                                }
                            } else {
                                let army_pos = {
                                    let Some(army) = gamemap.armys.get_mut(army_index) else {
                                        continue;
                                    };
                                    army.pos
                                };
                                let path = find_path(&*gamemap, objects, army_pos, goal, false);
                                let Some(army) = gamemap.armys.get_mut(army_index) else {
                                    continue;
                                };
                                army.path = if let Some(path) = path {
                                    path.0
                                } else {
                                    Vec::new()
                                };
                                self.try_to_send_message(
                                    gamemap,
                                    army_index,
                                    ServerMessage::State((battle.clone(), gamemap.clone())),
                                );
                            };
                        }
                    }
                }
            }
        }
        self.transport.send_packets(&mut self.server);
        let mut pause = false;
        for i in 0..=1 {
            let army = &mut gamemap.armys[i];
            if army.path.len() < 1 {
                if i == 0 {
                    pause = true;
                }
                continue;
            }
        }
        let mut moved = false;
        gamemap.pause = pause;
        if !gamemap.pause {
            for i in 0..gamemap.armys.len() {
                let army = &mut gamemap.armys[i];
                if army.path.len() < 1 {
                    continue;
                }
                moved = true;
                army.pos = army.path.remove(0);
                if let Some(building) = gamemap.hitmap[army.pos.0][army.pos.1].building {
                    army.building = Some(building);
                } else {
                    army.building = None;
                }
                gamemap.recalc_armies_hitboxes();
            }
            gamemap.time.minutes += 10;

            for i in 0..gameevents.len() {
                if let Some(executions) = execute_event(i, gamemap, gameevents, units, false) {
                    for exec in executions {
                        match exec {
                            Execute::Wait(t, player) => {}
                            Execute::Execute(event, player) => {
                                execute_event(event.event, gamemap, gameevents, units, true);
                            }
                            Execute::StartBattle(army, player) => {
                                if battle.is_none() {
                                    let battle_new = BattleInfo::new(&mut gamemap.armys, army, 0);
                                    *battle = Some(battle_new);
									self.try_to_send_message(
										gamemap,
										player,
										ServerMessage::ChangeMenu(Menu::ConnectBattle as usize),
									);
								}
                            }
                            Execute::Message(text, player) => {
                                self.try_to_send_message(
                                    gamemap,
                                    player,
                                    ServerMessage::Message(text),
                                );
                            },
                        }
                    }
                    break;
                };
            }
        }
        if moved {
            self.try_to_send_message(
                gamemap,
                1,
                ServerMessage::State((battle.clone(), gamemap.clone())),
            );
        }

        Ok(())
    }
}

pub fn create_renet_client() -> (RenetClient, NetcodeClientTransport) {
    let connection_config = ConnectionConfig::default();
    let client = RenetClient::new(connection_config);
    let server_addr = *SERVER;
    let socket = UdpSocket::bind(ADDR).unwrap();
    let current_time = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap();
    let client_id = current_time.as_millis() as u64;
    let authentication = ClientAuthentication::Unsecure {
        server_addr,
        client_id,
        user_data: None,
        protocol_id: PROTOCOL_ID,
    };
    let transport = NetcodeClientTransport::new(current_time, authentication, socket).unwrap();
    (client, transport)
}
#[derive(Clone, Debug)]
pub enum Connection {
    Host(GameServer),
    Client(ClientConnection),
}
#[derive(Clone, Debug)]
pub struct ConnectionManager {
    pub con: Connection,
    pub gamemap: GameMap,
    pub battle: Option<BattleInfo>,
    pub events: Vec<Event>,
    pub last_updated: Instant,
}
impl ConnectionManager {
    pub fn updates(
        &mut self,
        units: &Vec<Unit>,
        objects: &Vec<ObjectInfo>,
    ) -> (Option<usize>, Option<String>) {
        let now = Instant::now();
        let duration = now - self.last_updated;
        self.last_updated = now;
        match &mut self.con {
            Connection::Client(client) => {
                let ClientConnection { client, transport } = client;
                client.update(duration);
                if let Err(e) = transport.update(duration, client) {
                    dbg!(e);
                }

                if let Some(e) = client.disconnect_reason() {
                    dbg!(e);
                }
                while let Some(message) = client.receive_message(DefaultChannel::ReliableOrdered) {
                    let message: ServerMessage =
                        deserialize::<ServerMessage, ServerMessage>(&message).unwrap();
                    match message {
                        ServerMessage::State(state) => {
                            log::info!("Received state!");
                            let (battle, gamemap) = state.into();
                            self.battle = battle;
                            self.gamemap = gamemap;
                        }
                        ServerMessage::ChangeMenu(menu) => {
                            return (Some(menu), None);
                        }
                        ServerMessage::Message(text) => return (None, Some(text)),
                    }
                }

                if let Err(e) = transport.send_packets(client) {
                    log::error!("Error sending packets: {}", e);
                }
            }
            Connection::Host(server) => {
                if let Err(e) = server.update(
                    duration,
                    &mut self.gamemap,
                    &mut self.battle,
                    &mut self.events,
                    units,
                    objects,
                ) {
                    dbg!(e);
                };
            }
        }
        return (None, None);
    }
    pub fn send_message_to_server(
        &mut self,
        message: ClientMessage,
        units: &Vec<Unit>,
        objects: &Vec<ObjectInfo>,
    ) {
        match &mut self.con {
            Connection::Client(ref mut conn) => {
                let size = serialized_size::<ClientMessage, _>(&message);
                let mut output = vec![0u8; size.0];
                serialize::<ClientMessage, ClientMessage>(message, &mut output).ok();
                conn.client.send_message(
                    DefaultChannel::ReliableOrdered,
                    renet::Bytes::copy_from_slice(&output),
                );
            }
            Connection::Host(ref mut conn) => {
                conn.handle_client_message(
                    None,
                    message,
                    &mut self.gamemap,
                    &mut self.battle,
                    &mut self.events,
                    units,
                    objects,
                );
            }
        }
    }
}
