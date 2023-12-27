use std::{
    collections::HashMap,
    io,
    net::{UdpSocket, SocketAddr, Ipv4Addr, IpAddr},
    time::{Duration, SystemTime, Instant},
};
use crate::{lib::{battle::{army::{TroopType, Army}, troop::Troop}, units::unit::{UnitInfo, UnitLvl, UnitInventory}, parse::SETTINGS}, handle_action};
use crate::lib::battle::battlefield::BattleInfo;
use crate::lib::map::map::GameMap;
use crate::lib::units::unit::Unit;
use crate::lib::{
	units::unit::UnitStats
};
use alkahest::*;
use renet::{
    transport::{NetcodeServerTransport, ServerAuthentication, ServerConfig, NetcodeClientTransport, ClientAuthentication, ConnectToken},
    ClientId, ConnectionConfig, DefaultChannel, RenetServer, RenetClient, ServerEvent,
};
use notan::log;

#[derive(Clone, Debug)]
#[alkahest(Deserialize, Serialize, SerializeRef, Formula)]
pub struct NetUnit {
    pub stats: UnitStats,
    pub modified: UnitStats,
	pub icon_index: u64
}

#[derive(Clone, Debug)]
#[alkahest(Deserialize, Serialize, SerializeRef, Formula)]
pub struct NetTroop {
    pub was_payed: bool,
    pub is_free: bool,
    pub is_main: bool,
    pub unit: NetUnit,
}

#[derive(Clone, Debug, Default)]
#[alkahest(Deserialize, Serialize, SerializeRef, Formula)]
pub struct NetArmy {
	pub troops: Vec<NetTroop>,
	pub hitmap: Vec<Option<u64>>,
}

#[derive(Clone, Debug)]
#[alkahest(Deserialize, Serialize, SerializeRef, Formula)]
pub struct NetBattleInfo {
    pub army1: u64,
    pub army2: u64,
    pub active_unit: Option<(u64, u64)>,
    pub can_interact: Option<Vec<(u64, u64)>>,
}

#[derive(Clone, Debug)]
#[alkahest(Deserialize, Serialize, SerializeRef, Formula)]
pub struct NetGameMap {
    pub armys: Vec<NetArmy>
}

#[derive(Clone, Debug)]
#[alkahest(Deserialize, Serialize, SerializeRef, Formula)]
pub struct BattleState {
	pub battle: NetBattleInfo,
	pub gamemap: NetGameMap
}

impl From<BattleState> for (BattleInfo, GameMap) {
	fn from(value: BattleState) -> Self {
		let mut gamemap = GameMap::default();
		gamemap.armys = value.gamemap.armys.iter().enumerate().map(|(army_n, army)| {
			let mut res = Army::default();
			res.hitmap = army.hitmap.iter().map(|hit| hit.and_then(|v| Some(v as usize) )).collect();
			res.troops = army.troops.iter().map(|troop| {
				TroopType::new({
					let mut tr = Troop::empty();
					tr.is_free = troop.is_free;
					tr.is_main = troop.is_main;
					tr.was_payed = troop.was_payed;
					tr.unit = {
						let unit = Unit {
							stats: troop.unit.stats,
							modified: troop.unit.modified,
							modify: Default::default(),
							info: {
								let mut info = UnitInfo::empty();
								info.icon_index = troop.unit.icon_index as usize;
								info
							},
							lvl: UnitLvl::empty(),
							inventory: UnitInventory::empty(),
							army: army_n,
							bonus: crate::lib::bonuses::Bonus::NoBonus,
							effects: Vec::new()
						};
						unit
					};
					tr
				})
			}).collect();
			res
		}).collect();
		let mut battle = BattleInfo::default();
		battle.army1 = value.battle.army1 as usize;
		battle.army2 = value.battle.army2 as usize;
		battle.active_unit = value.battle.active_unit.and_then(|v| Some((v.0 as usize, v.1 as usize)));
		battle.can_interact = value.battle.can_interact.and_then(|v| Some(v.iter().map(|el| (el.0 as usize, el.1 as usize)).collect()));
		(battle, gamemap)
	}
}
impl From<Unit> for NetUnit {
	fn from(value: Unit) -> Self {
		NetUnit {
			stats: value.stats,
			modified: value.modified,
			icon_index: value.info.icon_index as u64
		}
	}
}
impl From<TroopType> for NetTroop {
	fn from(value: TroopType) -> Self {
		let troop = value.get();
		NetTroop {
			was_payed: troop.was_payed,
			is_free: troop.is_free,
			is_main: troop.is_main,
			unit: troop.unit.clone().into()
		}
	}
}
impl From<Army> for NetArmy {
	fn from(value: Army) -> Self {
		NetArmy {
			troops: value.troops.iter().map(|troop| troop.clone().into()).collect(),
			hitmap: value.hitmap.iter().map(|hit| hit.and_then(|v| Some(v as u64))).collect()
		}
	}
}
impl From<BattleInfo> for NetBattleInfo {
	fn from(value: BattleInfo) -> Self {
		NetBattleInfo {
			army1: value.army1 as u64,
			army2: value.army2 as u64,
			active_unit: value.active_unit.and_then(|v| Some((v.0 as u64, v.1 as u64))),
			can_interact: value.can_interact.and_then(|v| Some(v.iter().map(|e| (e.0 as u64, e.1 as u64)).collect()))
		}
	}
}
impl From<GameMap> for NetGameMap {
	fn from(value: GameMap) -> Self {
		NetGameMap {
			armys: value.armys.iter().map(|army| army.clone().into()).collect()
		}
	}
}
impl From<(BattleInfo, GameMap)> for BattleState {
	fn from(value: (BattleInfo, GameMap)) -> Self {
		BattleState {
			battle: value.0.into(),
			gamemap: value.1.into()
		}
	}
}

#[derive(Clone, Debug)]
#[alkahest(Deserialize, Serialize, SerializeRef, Formula)]
pub enum ClientMessage {
	Action((u64, u64))
}

#[derive(Clone, Debug)]
#[alkahest(Deserialize, Serialize, SerializeRef, Formula)]
pub enum ServerMessage {
	State(BattleState)
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
			transport: Box::new(b)
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
				let current_time = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap();
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
pub static SERVER: once_cell::sync::Lazy<SocketAddr> = once_cell::sync::Lazy::new(|| format!("127.0.0.1:{}", unsafe{&SETTINGS}.port).parse().unwrap());
impl GameServer {
    pub fn new() -> Self {
        let socket = UdpSocket::bind(*SERVER).unwrap();
        let current_time = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap();
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
        auth.insert(HOST_CLIENT_ID, 0);

        Self {
            server,
            transport,
            auth,
		}
	}
	fn a() {
		let connection_config = ConnectionConfig::default();
		let mut server: RenetServer = RenetServer::new(connection_config);
		let public_addr = *SERVER;
		let current_time = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap();
		let server_config = ServerConfig {
			current_time,
			max_clients: 64,
			protocol_id: PROTOCOL_ID,
			public_addresses: vec![public_addr],
			authentication: ServerAuthentication::Unsecure,
		};
		let socket: UdpSocket = UdpSocket::bind(public_addr).unwrap();

		let mut transport = NetcodeServerTransport::new(server_config, socket).unwrap();
	}
	pub fn update(&mut self, duration: Duration, gamemap: &mut GameMap, battle: &mut BattleInfo) -> Result<(), io::Error> {
        self.server.update(duration);
        self.transport.update(duration, &mut self.server).unwrap();

        while let Some(event) = self.server.get_event() {
            match event {
                ServerEvent::ClientConnected { client_id } => {
                    let user_data = self.transport.user_data(client_id).unwrap();

					if self.auth.values().any(|v| *v == 1) {
						self.auth.insert(client_id, 0);
					} else { self.auth.insert(client_id, 1); }
					
                    let message = ServerMessage::State(
						BattleState::from((battle.clone(), gamemap.clone()))
					);
					let size = serialized_size::<ServerMessage, _>(&message);
					let mut output = vec![0u8; size.0];
					serialize::<ServerMessage, ServerMessage>(message, &mut output).ok();
					log::info!("Sent greeting message!");
                    self.server.send_message(client_id, DefaultChannel::ReliableOrdered, renet::Bytes::copy_from_slice(&output));
                }
                ServerEvent::ClientDisconnected { client_id, reason: _ } => {
                    self.auth.remove(&client_id);
                }
            }
        }
		if self.server.connected_clients() > 0 {
			log::debug!("connected {} users", self.server.connected_clients());
		}
		//dbg!(self.server.is_connected(ClientId::from_raw(255)));
        for client_id in self.server.clients_id() {
			//dbg!(client_id);
            while let Some(message) = self.server.receive_message(client_id, DefaultChannel::ReliableOrdered) {
				let buf = message;
				let res: Result<ClientMessage, DeserializeError> = deserialize::<ClientMessage, ClientMessage>(&*buf);
				dbg!(&res);
                if let Ok(message) = res {
                    log::info!("Received message from client {}: {:?}", client_id, message);
                    match message {
                        ClientMessage::Action(v) => {
							if self.auth.get(&client_id) == battle.active_unit.and_then(|v| Some(v.0)).as_ref() {
								handle_action(crate::Action::Cell(v.0 as usize, v.1 as usize), battle, gamemap);
								let message = ServerMessage::State(
									BattleState::from((battle.clone(), gamemap.clone()))
								);
								let size = serialized_size::<ServerMessage, _>(&message);
								let mut output = vec![0u8; size.0];
								serialize::<ServerMessage, ServerMessage>(message, &mut output).ok();
								
								self.server.send_message(client_id, DefaultChannel::ReliableOrdered, renet::Bytes::copy_from_slice(&output));
							}
						}
                    }
                }
            }
        }

        self.transport.send_packets(&mut self.server);

        Ok(())
    }
	fn b() {
		
	}
}

pub fn create_renet_client() -> (RenetClient, NetcodeClientTransport) {
    let connection_config = ConnectionConfig::default();
	dbg!(&connection_config.server_channels_config);
    let client = RenetClient::new(connection_config);
	let server_addr = *SERVER;
    let socket = UdpSocket::bind(ADDR).unwrap();
    let current_time = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap();
    let client_id = 255;//current_time.as_millis() as u64;
    let authentication = ClientAuthentication::Unsecure {
        server_addr,
        client_id,
        user_data: None,
        protocol_id: PROTOCOL_ID,
    };
	// let auth = ClientAuthentication::Secure {
	// 	connect_token: ConnectToken::generate(
			
	// 	)
	// };
    let transport = NetcodeClientTransport::new(current_time, authentication, socket).unwrap();
    (client, transport)
}
pub fn a() {
	let connection_config = ConnectionConfig::default();
    let client = RenetClient::new(connection_config);

    let socket = UdpSocket::bind("127.0.0.1:0").unwrap();
    let current_time = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap();
    let client_id = current_time.as_millis() as u64;
	let server_addr = *SERVER;
    let authentication = ClientAuthentication::Unsecure {
        server_addr,
        client_id,
        user_data: None,
        protocol_id: PROTOCOL_ID,
    };

    let transport = NetcodeClientTransport::new(current_time, authentication, socket).unwrap();
}
#[derive(Clone, Debug)]
pub enum Connection {
	Host(GameServer),
	Client(ClientConnection)
}
#[derive(Clone, Debug)]
pub struct ConnectionManager {
	pub con: Connection,
	pub gamemap: GameMap,
	pub battle: BattleInfo,
	pub last_updated: Instant
}
impl ConnectionManager {
	pub fn updates(&mut self) {
		let now = Instant::now();
        let duration = now - self.last_updated;
		self.last_updated = now;
		match &mut self.con {
			Connection::Client(client) => {
				let ClientConnection { client, transport } = client;
				client.update(duration);
                if let Err(e) = transport.update(duration, client) {
					dbg!(e);
                    return;
                }

				if let Some(e) = client.disconnect_reason() {
					dbg!(e);
					return;
				}
                while let Some(message) = client.receive_message(DefaultChannel::ReliableOrdered) {
                    let message: ServerMessage = deserialize::<ServerMessage, ServerMessage>(&message).unwrap();
                    match message {
                        ServerMessage::State(state) => {
							log::info!("Received state!");
							dbg!(&state);
							let (battle, gamemap) = state.into();
							self.battle = battle;
							self.gamemap = gamemap;
						}
                    }
                }

                if let Err(e) = transport.send_packets(client) {
                     log::error!("Error sending packets: {}", e);
				}
			},
			Connection::Host(server) => {
				if let Err(e) = server.update(duration, &mut self.gamemap, &mut self.battle) {
					dbg!(e);
				};
			 }
		}
	}
}
