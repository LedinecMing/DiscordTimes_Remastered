use std::{
	time::{Duration, Instant},
	collections::HashMap
};	  

use dt_lib::{
    battle::{army::*, battlefield::*, troop::Troop},
    items::item::*,
    map::{
        event::{execute_event, Event, Execute},
        map::*,
        object::ObjectInfo,
        tile::*,
    },
    network::net::*,
    parse::{
        parse_items, parse_objects, parse_settings, parse_story,
        parse_units,
    },
	locale::{parse_locale, Locale},
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
	state.connection = Some(
		ConnectionManager {
			con: Connection::Host(
				GameServer::new(false)
			),
			gamemap: state.gamemap.clone(),
			battle: state.battle.clone(),
			events: state.gameevents.clone(),
			last_updated: Instant::now()
		}
	);
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
fn main() {
    println!("Hello, world!");
}
