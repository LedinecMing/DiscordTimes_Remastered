use advini::{Ini, IniParseError};
use alkahest::*;
use math_thingies::Percent;

use crate::{map::event::{Execute, Executions}, time::time::Time};
#[derive(Clone, Debug, PartialEq)]
#[alkahest(Deserialize, Serialize, SerializeRef, Formula)]
pub struct Relations {
    pub player: u8,
    pub ally: u8,
    pub neighbour: u8,
    pub enemy: u8,
}
impl Ini for Relations {
    fn eat(chars: std::str::Chars) -> Result<(Self, std::str::Chars), IniParseError> {
        match <(u8, u8, u8, u8) as Ini>::eat(chars) {
            Ok(v) => Ok({
                let rels = v.0;
                (
                    Self {
                        player: rels.0,
                        ally: rels.1,
                        neighbour: rels.2,
                        enemy: rels.3,
                    },
                    v.1,
                )
            }),
            Err(err) => Err(err),
        }
    }
    fn vomit(&self) -> String {
        (self.player, self.ally, self.neighbour, self.enemy).vomit()
    }
}
impl Default for Relations {
    fn default() -> Self {
        Self {
            player: 0,
            ally: 0,
            neighbour: 128,
            enemy: 255,
        }
    }
}
#[derive(Clone, Debug, PartialEq, Default)]
#[alkahest(Deserialize, Serialize, SerializeRef, Formula)]
pub enum Control {
    #[default]
    PC,
    Player(usize),
}
impl Ini for Control {
    fn eat<'a>(chars: std::str::Chars<'a>) -> Result<(Self, std::str::Chars<'a>), IniParseError> {
        let (tag, chars) = match <u8 as Ini>::eat(chars) {
            Ok(v) => Ok(v),
            Err(err) => Err(err),
        }?;
        match tag {
            0 => Ok((Control::PC, chars)),
            _ => {
                let (v, chars) = match <usize as Ini>::eat(chars) {
                    Ok(v) => Ok(v),
                    Err(err) => Err(err),
                }?;
                Ok((Control::Player(v - 1), chars))
            }
        }
    }
    fn vomit(&self) -> String {
        match self {
            Control::PC => 0_u8.vomit(),
            Control::Player(n) => (*n + 1).vomit(),
        }
    }
}
pub type Players = Vec<Player>;
#[derive(Clone, Debug, PartialEq, Default)]
#[alkahest(Deserialize, Serialize, SerializeRef, Formula)] 
pub struct Player {
	pub army: usize,
	pub questbook: Option<Vec<(String, String)>>,
	pub execution_queue: Vec<Execute>,
}
#[derive(Clone, Debug, PartialEq, Default)]
#[alkahest(Deserialize, Serialize, SerializeRef, Formula)]
pub struct PC_ControlSetings {
    pub xp_like_player: bool,
    pub xp_add: u64,
	pub xp_correction: Percent,
	pub gold_income: u64,
	pub mana_income: u64,
	pub speed_correction: Percent,
    pub units_dont_have_money: bool,
    pub ignores_ai_armys: bool,
    pub targets_player: bool,
    pub forbid_random_targets: bool,
    pub forbid_random_talks: bool,
    pub not_interested_in_buildings: bool,
	pub aggression: u8,
	pub patrol: u8,
	pub revive_everyone: bool,
	pub revive_time: Time,
	pub garrison_power: u8,
    pub patrol_radius: Option<u64>,
    pub relations: Relations,
}
#[derive(Clone, Debug, PartialEq)]
pub enum Target {
    Army(usize),
    Building(usize),
}
#[derive(Clone, Debug, PartialEq)]
pub enum Plan {
    ToTax,
    ToMarket,
    ToTalk,
}
#[derive(Clone, Debug, PartialEq, Default)]
pub struct PC_ControlState {
    current_target: Option<Target>,
    plan: Option<Plan>,
}

