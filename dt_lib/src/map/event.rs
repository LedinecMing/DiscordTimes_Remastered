use crate::{
    battle::{
        control::{Player, PlayerId, Players},
        troop::Troop,
    },
    items::Item,
    map::map::GameMap,
    mutrc::SendMut,
    time::time::Time,
    units::unit::{Unit, UnitPos},
};
use advini::{Ini, IniParseError, Section, SectionError, Sections, SEPARATOR};
use alkahest::*;
use serde;
use std::{collections::HashMap, path::Path};
use struct_field_names_as_array::FieldNamesAsArray;

#[allow(dead_code)]
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, PartialEq)]
pub enum Cmp<V: Ord> {
    L(V),
    G(V),
    LE(V),
    GE(V),
    E(V),
}
impl<V: Ord> Cmp<V> {
    pub fn check(&self, v: V) -> bool {
        match self {
            Cmp::L(cmp_v) => v < *cmp_v,
            Cmp::G(cmp_v) => v > *cmp_v,
            Cmp::LE(cmp_v) => v <= *cmp_v,
            Cmp::GE(cmp_v) => v >= *cmp_v,
            Cmp::E(cmp_v) => v == *cmp_v,
        }
    }
}
impl<V: Ord + Ini> Ini for Cmp<V> {
    fn eat<'a>(
        mut chars: std::str::Chars<'a>,
    ) -> Result<(Self, std::str::Chars<'a>), IniParseError> {
        match (chars.next(), chars.next()) {
            (Some(chr), Some(chr1)) => match (chr, chr1) {
                ('=', '<') | ('<', '=') => {
                    let v = match <V as Ini>::eat(chars) {
                        Ok(v) => v,
                        Err(err) => {
                            return Err(err);
                        }
                    };
                    Ok((Self::LE(v.0), v.1))
                }
                ('=', '>') | ('>', '=') => {
                    let v = match <V as Ini>::eat(chars) {
                        Ok(v) => v,
                        Err(err) => {
                            return Err(err);
                        }
                    };
                    Ok((Self::GE(v.0), v.1))
                }
                ('>', value) => {
                    let mut res_string = String::new();
                    res_string.push(value);
                    let v = match <V as Ini>::eat(chars) {
                        Ok(v) => v,
                        Err(err) => {
                            return Err(err);
                        }
                    };
                    Ok((Self::G(v.0), v.1))
                }
                ('<', value) => {
                    let mut res_string = String::new();
                    res_string.push(value);
                    let v = match <V as Ini>::eat(chars) {
                        Ok(v) => v,
                        Err(err) => {
                            return Err(err);
                        }
                    };
                    Ok((Self::L(v.0), v.1))
                }
                ('=', value) => {
                    let mut res_string = String::new();
                    res_string.push(value);
                    while let Some(chr) = chars.next() {
                        if chr == SEPARATOR {
                            break;
                        } else {
                            res_string.push(chr);
                        }
                    }
                    let v = match <V as Ini>::eat(chars) {
                        Ok(v) => v,
                        Err(err) => {
                            return Err(err);
                        }
                    };
                    Ok((Self::E(v.0), v.1))
                }
                (_, _) => Err(IniParseError::Error("oops")),
            },
            (_, _) => Err(IniParseError::Empty(chars)),
        }
    }
    fn vomit(&self) -> String {
        match self {
            Self::E(v) => "=".to_string() + &v.vomit(),
            Self::G(v) => ">".to_string() + &v.vomit(),
            Self::GE(v) => ">=".to_string() + &v.vomit(),
            Self::L(v) => "<".to_string() + &v.vomit(),
            Self::LE(v) => "<=".to_string() + &v.vomit(),
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
impl Ini for Location {
    fn eat<'a>(chars: std::str::Chars<'a>) -> Result<(Self, std::str::Chars<'a>), IniParseError> {
        let (res, chars) = match <String as Ini>::eat(chars) {
            Ok(v) => v,
            Err(err) => return Err(err),
        };
        match &*res {
            "Global" => {
                return Ok((Self::Global, chars));
            }
            "Local" => match <usize as Ini>::eat(chars) {
                Ok(v) => Ok((Self::Local, v.1)),
                Err(err) => Err(err),
            },
            "Quest" => {
                return Ok((Self::Quest, chars));
            }
            "Talks" => match <usize as Ini>::eat(chars) {
                Ok(v) => Ok((Self::Talks, v.1)),
                Err(err) => Err(err),
            },
            _ => Err(IniParseError::Error("netu")),
        }
    }
    fn vomit(&self) -> String {
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
    FieldNamesAsArray,
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
    FieldNamesAsArray,
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
        }
    }
}
pub fn execute_event(
    event: usize,
    players: &mut Players,
    gamemap: &mut GameMap,
    events: &mut Vec<Event>,
    units: &Vec<Unit>,
    executed_as_sub: bool,
) -> Option<Executions> {
    {
        let Event {
            name,
            player,
            location,
            conditions,
            result,
            message,
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
            units,
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
    units: &Vec<Unit>,
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
        .is_some_and(|req| req.check(player_army.troops[0].get().unit.lvl.xp))
        || conds.xp_req.is_none())
        && (conds
            .gold_req
            .as_ref()
            .is_some_and(|req| req.check(player_army.stats.gold))
            || conds.gold_req.is_none())
        && conds
        .army_req
        .as_ref()
        .is_none_or(|req| req.check(player_army.troops.len() as u64))
		// TODO archetype check
		&& conds.archetype_req.is_none_or(|arch| arch == 1)
		//TODO
        && conds
            .mana_req
            .as_ref()
            .is_none_or(|req| req.check(player_army.stats.mana))
		&&	conds.army_meet.is_none_or(|army| false)
		// TODO defeated armies check 
		&& conds
		.armies_defeated_by_player
		.as_ref()
		.is_none_or(|req| req.iter().all(|&x| false))
		// TODO power check
        && (conds.power_req.as_ref().and_then(|req| Some(true)).unwrap_or(true))
        && (conds.hero_has_1_hp || !conds.hero_has_1_hp)
		&& (conds
			.items_check
			.as_ref()
			.is_some_and(|items| items.iter().all(|item| {
				match item.0 {
					ItemCheck::Player => player_army.inventory.contains(&Some(Item { index: item.1 })),
					ItemCheck::PlayerHasNo => !player_army.inventory.contains(&Some(Item { index: item.1 })),
					_ => false
				}
			})) || conds.items_check.is_none())
		// TODO
		&& conds.building_ownership.as_ref().is_none_or(|reqs| reqs.iter().all(|x| false))
        && (conds
            .in_building
            .and_then(|building| Some(player_army.building == building.into())))
        .unwrap_or(true)
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
            let troop = &mut player_army.troops[0].get();
            troop.unit.lvl.xp = troop.unit.lvl.xp.saturating_add_signed(result.change_xp);
        }
        {
            if let Some(add_units) = &mut result.add_units {
                add_units.iter().for_each(|unit| {
                    player_army
                        .add_troop(SendMut::new(Troop {
                            unit: units[*unit].clone(),
                            custom_name: None,
                            is_free: true,
                            was_payed: true,
                            is_main: false,
                            pos: UnitPos::from_index(0),
                        }))
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
                }),
                player_id,
            ));
        }
        if let Some(text) = message.clone() {
            res.push((
                Execute::Message(Message {
                    text,
                    variants: vec![],
                }),
                player_id,
            ));
        }
        if result.delay.1 == true {
            res.push((Execute::Wait(result.delay.0.clone()), player_id));
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
    pub event: usize,
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
        units: &Vec<Unit>,
    ) -> Option<()> {
        if self.time <= gamemap.time {
            execute_event(self.event, players, gamemap, events, units, false);
        }
        None
    }
}
impl Ini for DelayedEvent {
    fn eat(chars: std::str::Chars<'_>) -> Result<(Self, std::str::Chars<'_>), IniParseError> {
        <(Time, usize) as Ini>::eat(chars).map(|res| {
            (
                Self {
                    time: res.0 .0,
                    event: res.0 .1,
                },
                res.1,
            )
        })
    }
    fn vomit(&self) -> String {
        <(Time, usize) as Ini>::vomit(&(self.time, self.event))
    }
}
#[derive(Debug, Clone, PartialEq)]
#[alkahest(Deserialize, Serialize, SerializeRef, Formula)]
pub struct Message {
    pub text: String,
    pub variants: Vec<String>,
}
pub type Executions = Vec<(Execute, PlayerId)>;
#[derive(Debug, Clone, PartialEq)]
#[alkahest(Deserialize, Serialize, SerializeRef, Formula)]
pub enum Execute {
    Wait(Time),
    Message(Message),
    StartBattle(usize),
    Execute(DelayedEvent),
    Sub(EventId),
}
