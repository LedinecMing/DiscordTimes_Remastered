use crate::{
    battle::troop::Troop,
    map::map::GameMap,
    mutrc::SendMut,
    time::time::Time,
    units::unit::{Unit, UnitPos},
};
use advini::{Ini, IniParseError, Section, SectionError, Sections, SEPARATOR};
use serde;
use std::collections::HashMap;
use struct_field_names_as_array::FieldNamesAsArray;

#[allow(dead_code)]
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
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

#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub enum Location {
    #[default]
    Global,
    Local(usize),          // building id
    Place((usize, usize)), // map xy
    Quest,
    Sub,
    Talks(usize), // building id
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
                Ok(v) => Ok((Self::Local(v.0), v.1)),
                Err(err) => Err(err),
            },
            "Quest" => {
                return Ok((Self::Quest, chars));
            }
            "Talks" => match <usize as Ini>::eat(chars) {
                Ok(v) => Ok((Self::Talks(v.0), v.1)),
                Err(err) => Err(err),
            },
            "Sub" => {
                return Ok((Self::Sub, chars));
            }
            _ => Err(IniParseError::Error("netu")),
        }
    }
    fn vomit(&self) -> String {
        match self {
            Location::Global => "Global".into(),
            Location::Place(p) => "Place".to_string() + "," + &p.vomit(),
            Location::Local(v) => "Local,".to_string() + "," + &v.to_string(),
            Location::Talks(v) => "Talks,".to_string() + "," + &v.to_string(),
            Location::Sub => "Sub".to_string(),
            Location::Quest => "Quest".into(),
        }
    }
}
#[derive(
    Clone, Debug, Default, Sections, serde::Serialize, serde::Deserialize, FieldNamesAsArray,
)]
pub struct Conditions {
    #[default_value = "false"]
    pub relative_time: bool,
    #[default_value = "None"]
    pub repeat: Option<Time>,
    #[default_value = "false"]
    #[unused]
    pub executed: bool,

    #[default_value = "Time::new(0)"]
    pub activation_time: Time,
    #[default_value = "None"]
    pub if_event_executed: Option<usize>,
    #[default_value = "None"]
    pub armys_defeated: Option<Vec<usize>>,
    #[default_value = "None"]
    pub not_executed: Option<Vec<usize>>,
    #[default_value = "String::new()"]
    pub flag_check: String,

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
}
impl Conditions {
    fn new(
        relative_time: bool,
        repeat: Option<Time>,
        activation_time: Time,
        if_event_executed: Option<usize>,
        armys_defeated: Option<Vec<usize>>,
        not_executed: Option<Vec<usize>>,
        flag_check: String,
        xp_req: Option<Cmp<u64>>,
        gold_req: Option<Cmp<u64>>,
        mana_req: Option<Cmp<u64>>,
        army_req: Option<Cmp<u64>>,
        power_req: Option<Cmp<u64>>,
        hero_has_1_hp: bool,
        in_building: Option<usize>,
    ) -> Self {
        Self {
            relative_time,
            executed: false,
            repeat,
            activation_time,
            if_event_executed,
            army_req,
            not_executed,
            flag_check,
            xp_req,
            gold_req,
            mana_req,
            armys_defeated,
            hero_has_1_hp,
            power_req,
            in_building,
        }
    }
}
#[derive(
    Clone, Debug, Default, Sections, serde::Serialize, serde::Deserialize, FieldNamesAsArray,
)]
pub struct EventResult {
    #[default_value = "None"]
    pub lit_lights: Option<Vec<usize>>,
    #[default_value = "(Time::new(0), false)"]
    pub delay: (Time, bool),
    #[default_value = "String::new()"]
    pub flag_change: String,
    #[default_value = "None"]
    pub sub_event: Option<Vec<usize>>,
    #[default_value = "None"]
    pub delayed_event: Option<DelayedEvent>,
    #[default_value = "None"]
    pub minus_items: Option<Vec<usize>>, // index of all game items
    #[default_value = "None"]
    pub plus_items: Option<Vec<usize>>,
    #[default_value = "None"]
    pub question: Option<(String, Vec<String>)>,

    #[default_value = "0"]
    pub change_xp: i64,
    #[default_value = "0"]
    pub change_gold: i64,
    #[default_value = "0"]
    pub change_mana: i64,

    #[default_value = "None"]
    pub add_units: Option<Vec<usize>>, // index of all game units
    #[default_value = "None"]
    pub remove_units: Option<Vec<usize>>,
    #[default_value = "None"]
    pub change_personality: Option<usize>,
}
impl EventResult {
    fn new(
        lit_lights: Option<Vec<usize>>,
        delay: (Time, bool),
        flag_change: String,
        sub_event: Option<Vec<usize>>,
        delayed_event: Option<DelayedEvent>,
        minus_items: Option<Vec<usize>>,
        plus_items: Option<Vec<usize>>,
        question: Option<(String, Vec<String>)>,
        change_xp: i64,
        change_gold: i64,
        change_mana: i64,
        add_units: Option<Vec<usize>>,
        remove_units: Option<Vec<usize>>,
        change_personality: Option<usize>,
    ) -> Self {
        Self {
            lit_lights,
            delay,
            flag_change,
            sub_event,
            delayed_event,
            minus_items,
            plus_items,
            question,
            change_gold,
            change_mana,
            change_personality,
            change_xp,
            add_units,
            remove_units,
        }
    }
}

#[derive(Clone, Debug, Default, Sections, serde::Serialize, serde::Deserialize)]
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
    gamemap: &mut GameMap,
    events: &mut Vec<Event>,
    units: &HashMap<usize, Unit>,
    executed_as_sub: bool,
) -> Option<Vec<Execute>> {
    {
        let Event {
            name,
            player,
            location,
            conditions,
            result,
            message,
        } = &events[event];
        match location {
            Location::Sub => {
                if !executed_as_sub {
                    return None;
                }
            }
            _ => {}
        }
        let time = if conditions.relative_time {
            gamemap.time - gamemap.time
        } else {
            gamemap.time
        };
        if !(((!conditions.executed || conditions.repeat.is_some())
            && conditions.activation_time <= gamemap.time)
            && (conditions
                .if_event_executed
                .is_some_and(|event| events[event].conditions.executed)
                || conditions.if_event_executed.is_none())
            && (conditions
                .armys_defeated
                .as_ref()
                .is_some_and(|armys_index| {
                    armys_index.iter().all(|army| gamemap.armys[*army].defeated)
                })
                || conditions.armys_defeated.is_none())
            && (conditions
                .not_executed
                .as_ref()
                .is_some_and(|events_index| {
                    events_index
                        .iter()
                        .all(|event| !events[*event].conditions.executed)
                })
                || conditions.not_executed.is_none()))
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
            message, result, conditions, location, gamemap, *player, units,
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
    player: usize,
    units: &HashMap<usize, Unit>,
) -> Option<Vec<Execute>> {
    match location {
        Location::Local(building) => {
            if !gamemap.armys[player]
                .building
                .is_some_and(|v| v == *building)
            {
                return None;
            }
        }
        Location::Talks(building) => {
            if !gamemap.armys[player]
                .building
                .is_some_and(|v| v == *building)
            {
                return None;
            }
        }
        Location::Place(pos) => {
            if gamemap.armys[player].pos != *pos {
                return None;
            }
        }
        _ => {}
    }
    let time = if conds.relative_time {
        gamemap.time - gamemap.time
    } else {
        gamemap.time
    };
    if (conds
        .xp_req
        .as_ref()
        .is_some_and(|req| req.check(gamemap.armys[player].troops[0].get().unit.lvl.xp))
        || conds.xp_req.is_none())
        && (conds
            .gold_req
            .as_ref()
            .is_some_and(|req| req.check(gamemap.armys[player].stats.gold))
            || conds.gold_req.is_none())
        && (conds
            .army_req
            .as_ref()
            .is_some_and(|req| req.check(gamemap.armys[player].troops.len() as u64))
            || conds.army_req.is_none())
        && (conds
            .mana_req
            .as_ref()
            .is_some_and(|req| req.check(gamemap.armys[player].stats.mana))
            || conds.mana_req.is_none())
        && (conds.power_req.as_ref().is_some_and(|req| true) || conds.power_req.is_none())
        && (conds.hero_has_1_hp || !conds.hero_has_1_hp)
        && (conds
            .in_building
            .and_then(|building| Some(gamemap.armys[player].building == building.into())))
        .unwrap_or(true)
    {
        let repeat = conds.repeat;
        // Player army items change
        if let Some(remove_items) = &mut result.minus_items {
            remove_items
                .iter()
                .for_each(|item| gamemap.armys[player].remove_item(*item));
        }
        if let Some(add_items) = &mut result.plus_items {
            add_items
                .iter()
                .for_each(|item| gamemap.armys[player].add_item(*item));
        }

        {
            // Player army stats changes
            let army = &mut gamemap.armys[player];
            army.stats.gold = army.stats.gold.saturating_add_signed(result.change_gold);
            army.stats.mana = army.stats.mana.saturating_add_signed(result.change_mana);
            let troop = &mut army.troops[0].get();
            troop.unit.lvl.xp = troop.unit.lvl.xp.saturating_add_signed(result.change_xp);
        }
        {
            let army = &mut gamemap.armys[player];
            if let Some(add_units) = &mut result.add_units {
                add_units.iter().for_each(|unit| {
                    army.add_troop(SendMut::new(Troop {
                        unit: units[unit].clone(),
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
        if let Some(text) = message {
            res.push(Execute::Message(text.clone(), player));
        }
        if result.delay.1 == true {
            res.push(Execute::Wait(result.delay.0.clone(), player));
        }
        if let Some(event) = &result.sub_event {
            for event in event {
                res.push(Execute::Execute(
                    DelayedEvent::new(Time::new(0), *event),
                    player,
                ));
            }
        }
        if let Some(event) = result.delayed_event.clone() {
            res.push(Execute::Execute(event, player));
        }
        if let Some(time) = repeat {
            dbg!(time, gamemap.time);
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

#[derive(Clone, Debug, Default, serde::Deserialize, serde::Serialize)]
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
        player: usize,
        units: &HashMap<usize, Unit>,
    ) -> Option<()> {
        if self.time <= gamemap.time {
            execute_event(self.event, gamemap, events, units, false);
        }
        None
    }
}
impl Ini for DelayedEvent {
    fn eat<'a>(chars: std::str::Chars<'a>) -> Result<(Self, std::str::Chars<'a>), IniParseError> {
        match <(Time, usize) as Ini>::eat(chars) {
            Ok(res) => Ok((
                Self {
                    time: res.0 .0,
                    event: res.0 .1,
                },
                res.1,
            )),
            Err(err) => Err(err),
        }
    }
    fn vomit(&self) -> String {
        <(Time, usize) as Ini>::vomit(&(self.time, self.event))
    }
}
#[derive(Debug, Clone)]
pub struct Message {
    text: String,
}
#[derive(Debug, Clone)]
pub enum Execute {
    Wait(Time, usize),
    Message(String, usize),
    StartBattle(usize, usize),
    Execute(DelayedEvent, usize),
}
