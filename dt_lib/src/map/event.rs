use crate::{
    battle::{
        control::{Player, PlayerId, Players},
        troop::Troop,
    }, items::Item, map::map::GameMap, mutrc::SendMut, registry::{self, GameInfo}, time::time::Time, units::unit::{Unit, UnitPos}
};
use advini::{Ini, IniParseError, Section, SectionError, Sections, SEPARATOR};
use alkahest::*;
use num::{Num, Zero};
use num_enum;
use schemars::JsonSchema;
use serde;
use std::{collections::HashMap, path::Path};
use nom::{Parser, branch::alt, bytes::complete::tag, character::complete::digit1, combinator::{consumed, value}, sequence::pair};

#[allow(dead_code)]
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, PartialEq, JsonSchema)]
pub enum Cmp<V: Ord> {
    L(V),
    G(V),
    E(V),
}
impl<V: Ord> Cmp<V> {
    pub fn check(&self, v: V) -> bool {
        match self {
            Cmp::L(cmp_v) => v < *cmp_v,
            Cmp::G(cmp_v) => v > *cmp_v,
            Cmp::E(cmp_v) => v == *cmp_v,
        }
    }
}
impl<'z, V: Ord + Num + Ini<'z, Arg = ()>> Ini<'z> for Cmp<V> {
	type Arg = <V as Ini<'z>>::Arg;
	fn eat<'a>(input: &'a str, _additional: Self::Arg) -> Result<(&'a str, Self), IniParseError> {
		let (input, res) = pair(
			consumed(alt([
				tag("="),
				tag(">"),
				tag("<"),
			])),
			digit1).parse(input)?;
		let num = Num::from_str_radix(res.0.0, 10).ok().unwrap_or(<V as Zero>::zero());
		let res = match res.0.0 {
			v if v == "=" => {
				Cmp::E(num)
			},
			v if v == ">" => {
				Cmp::G(num)
			}
			v if v == "<" => {
				Cmp::L(num)
			},
			_ => { Cmp::E(num) }
		};
		Ok((input, res))
	}
	fn vomit(&self, additional: Self::Arg) -> String {
		match self {
			Self::E(v) => "=".to_string() + &v.vomit(additional),
			Self::G(v) => ">".to_string() + &v.vomit(additional),
			Self::L(v) => "<".to_string() + &v.vomit(additional),
		}
	}
}

#[derive(Clone, Debug, PartialEq, Default, serde::Serialize, serde::Deserialize)]
pub enum Location {
    #[default]
    Global,
    Local, // building id
    Place, // map xy
    Quest,
    Talks, // building id
}
impl Ini<'_> for Location {
	fn eat<'a>(input: &'a str, _additional: Self::Arg) -> Result<(&'a str, Self), IniParseError> {
        let (input, res) = match <String as Ini>::eat(input, ()) {
            Ok(v) => v,
            Err(err) => return Err(err),
        };
        match &*res {
            "Global" => {
                return Ok((input, Self::Global));
            }
            "Local" => match <usize as Ini>::eat(input, ()) {
                Ok(v) => Ok((v.0, Self::Local)),
                Err(err) => Err(err),
            },
            "Quest" => {
                return Ok((input, Self::Quest));
            }
            "Talks" => match <usize as Ini>::eat(input, ()) {
                Ok(v) => Ok((v.0, Self::Talks)),
                Err(err) => Err(err),
            },
            _ => Err(IniParseError::Error("netu".to_owned())),
        }
    }
	fn vomit(&self, _additional: Self::Arg) -> String {
        match self {
            Location::Global => "Global".into(),
            Location::Place => "Place".to_string(),
            Location::Local => "Local".to_string(),
            Location::Talks => "Talks".to_string(),
            Location::Quest => "Quest".into(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Default, serde::Serialize, serde::Deserialize, Ini)]
pub enum ItemCheck {
    #[default]
    Player,
    PlayerHasNo,
    FactionHas(usize),
}

pub type EventId = usize;

#[derive(
    Clone,
    Debug,
    PartialEq,
    Default,
    Sections,
    serde::Serialize,
    serde::Deserialize,
)]
pub struct Conditions {
   #[default_value = "false"]
    pub relative_time: bool,
    #[default_value = "None"]
    pub repeat: Option<Time>,
    #[default_value = "false"]
    #[unused]
    pub executed: bool,
    #[default_value = "false"]
    pub sub: bool,
    #[default_value = "Time::new(0)"]
    pub activation_time: Time,
    #[default_value = "None"]
    pub if_event_executed: Option<Vec<usize>>,
    #[default_value = "None"]
    pub army_meet: Option<usize>,
    #[default_value = "None"]
    pub armies_defeated: Option<Vec<usize>>,
    #[default_value = "None"]
    pub armies_defeated_by_player: Option<Vec<usize>>,
    #[default_value = "None"]
    pub armies_active: Option<Vec<usize>>,
    #[default_value = "None"]
    pub armies_inactive: Option<Vec<usize>>,

    #[default_value = "None"]
    pub building_ownership: Option<Vec<(usize, usize)>>,

    #[default_value = "None"]
    pub items_check: Option<Vec<(ItemCheck, usize)>>,
	
    #[default_value = "None"]
    pub if_event_not_executed: Option<Vec<usize>>,
    #[default_value = "None"]
    pub if_event_answ: Option<Vec<(usize, usize)>>,
    #[default_value = "None"]
    #[default_value = "None"]
    pub flag_check: Option<String>,

    #[default_value = "None"]
    pub xp_req: Option<Cmp<u64>>,
    #[default_value = "None"]
    pub gold_req: Option<Cmp<u64>>,
    #[default_value = "None"]
    pub mana_req: Option<Cmp<u64>>,
    #[default_value = "None"]
    pub army_req: Option<Cmp<u64>>,
    #[default_value = "None"]
    pub power_req: Option<Cmp<u64>>,

    #[default_value = "false"]
    pub hero_has_1_hp: bool,
    #[default_value = "None"]
    pub in_building: Option<usize>,
    #[default_value = "None"]
    pub archetype_req: Option<usize>,
}
#[derive(
    Clone,
    Debug,
    PartialEq,
    Default,
    serde::Serialize,
    serde::Deserialize,
    Sections,
)]
pub struct EventResult {
    #[default_value = "None"]
    /// Возможный список активации фонарей в следующем формате
    /// lit_lights=2,3,1
    pub lit_lights: Option<Vec<usize>>,
    #[default_value = "(Time::new(0), false)"]
    /// Задержка перед выполнением
    pub delay: (Time, bool),
    /// Операция смены флага
    #[default_value = "None"]
    pub flag_change: Option<String>,
    /// Возможный список подчинённых событий в следующем формате
    /// sub_event=2,1,3
    #[default_value = "None"]
    pub sub_event: Option<Vec<usize>>,
    #[default_value = "None"]
    /// Возможное отложенное событие
    pub delayed_event: Option<DelayedEvent>,
    #[default_value = "None"]
    /// Список индексов предметов, которые будут отобраны
    pub minus_items: Option<Vec<usize>>, // index of all game items
    #[default_value = "None"]
    /// Убрать предметыы
    pub plus_items: Option<Vec<usize>>,
    #[default_value = "None"]
    pub question: Option<(String, Vec<String>)>,

    #[default_value = "None"]
    pub activate_armies: Option<Vec<usize>>,
    #[default_value = "None"]
    pub deactivate_armies: Option<Vec<usize>>,
    #[default_value = "None"]
    pub start_battle_with: Option<usize>,

    #[default_value = "None"]
    pub complete_quest: Option<usize>,
    #[default_value = "None"]
    pub learn_spells: Option<Vec<usize>>,

    #[default_value = "0"]
    /// Добавить опыт
    pub change_xp: i64,
    #[default_value = "0"]
    /// Добавить золото
    pub change_gold: i64,
    #[default_value = "0"]
    /// Добавить ману
    pub change_mana: i64,

    #[default_value = "None"]
    pub add_units: Option<Vec<usize>>, // index of all game units
    #[default_value = "None"]
    pub remove_units: Option<Vec<usize>>,
    #[default_value = "None"]
    pub change_personality: Option<usize>, // Changes player-controlled army
}
pub type Events = Vec<Event>;
#[derive(Clone, Debug, PartialEq, Default, Sections, serde::Serialize, serde::Deserialize)]
pub struct Event {
	// FIX?
	#[default_value = "0usize"]
	#[unused]
	pub id: EventId,	
    #[default_value = "String::new()"]
    pub name: String,
    #[default_value = "vec![0]"]
    pub player: Vec<usize>,
    #[default_value = "Location::Global"]
    pub location: Location,
    #[inline_parsing]
    pub conditions: Conditions,
    #[inline_parsing]
    pub result: EventResult,
    #[default_value = "None"]
    pub message: Option<String>,
}
impl Event {
    fn new(
        name: String,
        player: Vec<usize>,
        location: Location,
        conditions: Conditions,
        result: EventResult,
        message: Option<String>,
    ) -> Self {
        Self {
            name,
            player,
            location,
            conditions,
            result,
            message,
			id: 0
        }
    }
}
pub fn execute_event(
    event: usize,
    players: &mut Players,
    gamemap: &mut GameMap,
    events: &mut Vec<Event>,
    executed_as_sub: bool,
	registry: &GameInfo,
) -> Option<Executions> {
    {
        let Event {
            name,
            player,
            location,
            conditions,
            result,
            message,
			..
        } = &events[event];
        let mut sub = false;
        if conditions.sub || *location == Location::Quest {
            if !executed_as_sub {
                return None;
            } else {
                sub = true;
            }
        };
        match location {
            Location::Talks | Location::Local => {
                return None;
            }
            _ => {}
        }
        let time = if conditions.relative_time {
            gamemap.time - gamemap.start.time
        } else {
            gamemap.time
        };
        if !sub
            && !(((!conditions.executed || conditions.repeat.is_some())
			&& conditions.activation_time <= gamemap.time)
			 // TODO asnw check
			&& conditions.if_event_answ.is_none()
            && conditions
                .if_event_executed
                .as_ref()
                .is_none_or(|event| {
					event
						.iter()
						.all(|&event| events[event].conditions.executed)
				})
			&& (conditions
                .armies_defeated
                .as_ref()
                .is_some_and(|armys_index| {
                    armys_index.iter().all(|army| gamemap.armys[*army].defeated)
                })
                || conditions.armies_defeated.is_none())
			&& (conditions
				.armies_active
				.as_ref()
				.is_some_and(|armys_index| {
					armys_index.iter().all(|army| gamemap.armys[*army].active)
				})
				|| conditions.armies_active.is_none())
			&& (conditions
				.armies_inactive
				.as_ref()
				.is_some_and(|armys_index| {
					armys_index.iter().all(|army| !gamemap.armys[*army].active)
				})
				|| conditions.armies_inactive.is_none())
			&& (conditions
				.flag_check.is_none()
			)
			&& (conditions
                .if_event_not_executed
                .as_ref()
                .is_some_and(|events_index| {
                    events_index
                        .iter()
                        .all(|event| !events[*event].conditions.executed)
                })
                || conditions.if_event_not_executed.is_none()))
        {
            return None;
        }
    }
    let Event {
		id,
        name,
        player,
        location,
        conditions,
        result,
        message,
    } = &mut events[event];
    let mut res = Vec::new();
    for player in player {
        if let Some(events) = execute_event_as_player(
            message,
            result,
            conditions,
            location,
            gamemap,
            &mut players[*player],
            *player,
			executed_as_sub,
			*id,
			registry
        ) {
            res.extend(events);
        };
    }
    Some(res)
}
pub fn execute_event_as_player(
    message: &Option<String>,
    result: &mut EventResult,
    conds: &mut Conditions,
    location: &Location,
    gamemap: &mut GameMap,
    player: &mut Player,
    player_id: usize,
	executed_as_sub: bool,
	event_id: EventId,
	registry: &GameInfo,
) -> Option<Executions> {
    let time = if conds.relative_time {
        gamemap.time - gamemap.time
    } else {
        gamemap.time
    };
    let player_army = &mut gamemap.armys[player.army];
    if (conds
        .xp_req
        .as_ref()
        .is_none_or(|req| req.check(player_army.troops[0].get().unit.lvl.xp)))
        && (conds
            .gold_req
            .as_ref()
            .is_none_or(|req| req.check(player_army.stats.gold)))
        && (conds
        .army_req
        .as_ref()
        .is_none_or(|req| req.check(player_army.troops.len() as u64)))
		// TODO archetype check
		&& (conds.archetype_req.is_none_or(|arch| arch == 1))
		//TODO
        && (conds
            .mana_req
            .as_ref()
            .is_none_or(|req| req.check(player_army.stats.mana)))
		&&	(conds.army_meet.is_none_or(|army| false))
		// TODO defeated armies check 
		&& (conds
		.armies_defeated_by_player
		.as_ref()
		.is_none_or(|req| req.iter().all(|&x| false)))
		// TODO power check
        && (conds.power_req.as_ref().is_none_or(|req| true))
        && (!conds.hero_has_1_hp || player_army.troops[0].get().unit.modified.hp == 1)
		&& (conds
			.items_check
			.as_ref()
			.is_none_or(|items| items.iter().all(|item| {
				match item.0 {
					ItemCheck::Player => player_army.inventory.contains(&Some(Item { index: item.1 })),
					ItemCheck::PlayerHasNo => !player_army.inventory.contains(&Some(Item { index: item.1 })),
					_ => false
				}
			})))
		// TODO
		&& (conds.building_ownership.as_ref().is_none_or(|reqs| reqs.iter().all(|x| false)))
        && (conds
            .in_building
            .is_none_or(|building| player_army.building == building.into()))
		|| executed_as_sub
    {
        let repeat = conds.repeat;
        // Player army items change
        if let Some(remove_items) = &mut result.minus_items {
            remove_items
                .iter()
                .for_each(|item| player_army.remove_item(*item));
        }
        if let Some(add_items) = &mut result.plus_items {
            add_items
                .iter()
                .for_each(|item| player_army.add_item(Item { index: *item }));
        }

        {
            // Player army stats changes
            player_army.stats.gold = player_army
                .stats
                .gold
                .saturating_add_signed(result.change_gold);
            player_army.stats.mana = player_army
                .stats
                .mana
                .saturating_add_signed(result.change_mana);
            let troop = &mut player_army.troops.get(0);
			if let Some(troop) = *troop {
				let mut troop = troop.get();
				troop.unit.lvl.xp = troop.unit.lvl.xp.saturating_add_signed(result.change_xp);
			}
        }
        {
            if let Some(add_units) = &mut result.add_units {
                add_units.iter().for_each(|unit| {
                    player_army
                        .add_troop(SendMut::new(Troop {
                            unit: (registry.units[*unit].clone(), &registry.bonuses).into(),
                            custom_name: None,
                            is_free: true,
                            was_payed: true,
                            is_main: false,
                            pos: UnitPos::from_index(0, 6),
                        }), &registry.units)
                        .ok();
                });
            }
        }

        let mut res = Vec::new();
        if let Some(q) = &result.question {
            res.push((
                Execute::Message(Message {
                    text: q.0.clone(),
                    variants: q.1.clone(),
					corresponding_event_id: Some(event_id),					
					..Default::default()
                }),
                player_id,
            ));
        }
        if let Some(text) = message.clone() {
            res.push((
                Execute::Message(Message {
                    text,
                    variants: vec![],
					corresponding_event_id: Some(event_id),
					..Default::default()
                }),
                player_id,
            ));
        }
        if result.delay.1 == true {
			player.wait_until = Some(result.delay.0.clone() + gamemap.time);
        }
        if let Some(event) = &result.sub_event {
            for event in event {
                res.push((Execute::Sub(*event), player_id));
            }
        }
        if let Some(event) = result.delayed_event.clone() {
            res.push((Execute::Execute(event), player_id));
        }
        if let Some(time) = repeat {
            conds.activation_time = gamemap.time + time;
        }
        conds.executed = true;
        if !res.is_empty() {
            return Some(res);
        }
        return None;
    }
    None
}

#[derive(Clone, Debug, PartialEq, Default, serde::Deserialize, serde::Serialize)]
#[alkahest(Deserialize, Serialize, SerializeRef, Formula)]
pub struct DelayedEvent {
    pub time: Time,
    pub event: EventId,
}
impl DelayedEvent {
    pub fn new(time: Time, event: usize) -> Self {
        DelayedEvent { time, event }
    }
    pub fn execute(
        &self,
        gamemap: &mut GameMap,
        events: &mut Vec<Event>,
        players: &mut Players,
        player: usize,
		registry: &GameInfo,
    ) -> Option<()> {
        if self.time <= gamemap.time {
            execute_event(self.event, players, gamemap, events, false, registry);
        }
        None
    }
}
impl Ini<'_> for DelayedEvent {
    fn eat<'a>(input: &'a str, _additional: Self::Arg) -> Result<(&'a str, Self), IniParseError> {
        <(Time, usize) as Ini>::eat(input, _additional).map(|res| {
            (
                res.0,
				Self {
                    time: res.1.0,
                    event: res.1.1,
                },
            )
        })
    }
	fn vomit(&self, additional: Self::Arg) -> String {
        <(Time, usize) as Ini>::vomit(&(self.time, self.event), additional)
    }
}
#[derive(Default, Debug, Clone, PartialEq)]
#[alkahest(Deserialize, Serialize, SerializeRef, Formula)]
pub struct Message {
    pub text: String,
    pub variants: Vec<String>,
	pub corresponding_event_id: Option<EventId>,
}
impl Message {
	pub fn from_event(event: &Event, event_id: EventId) -> Self {
		Self {
			text: event.message.clone().unwrap_or_default(),
			variants: event.result.question.as_ref().and_then(|x| Some(x.1.clone())).unwrap_or_default(),
			corresponding_event_id: Some(event_id)
		}
	}
}
pub type Executions = Vec<(Execute, PlayerId)>;
#[derive(Debug, Clone, PartialEq)]
#[alkahest(Deserialize, Serialize, SerializeRef, Formula)]
pub enum Execute {
    Message(Message),
    StartBattle(usize),
    Execute(DelayedEvent),
    Sub(EventId),
}
