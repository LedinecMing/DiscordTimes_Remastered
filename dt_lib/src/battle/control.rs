use advini::{Ini, IniParseError};
use alkahest::*;
#[derive(Clone, Debug)]
#[alkahest(Deserialize, Serialize, SerializeRef, Formula)]
pub struct Relations {
    player: u8,
    ally: u8,
    neighbour: u8,
    enemy: u8,
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
		Self { player: 0, ally: 0, neighbour: 128, enemy: 255 }
	}
}
#[derive(Clone, Debug, Default)]
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
                Ok((Control::Player(v), chars))
            }
        }
    }
    fn vomit(&self) -> String {
        match self {
            Control::PC => 0_u8.vomit(),
            Control::Player(n) => (0_u8, *n).vomit(),
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct PC_ControlSetings {
	xp_like_player: bool,
	xp_add: u64,
	units_dont_have_money: bool,
	ignores_ai_armys: bool,
	targets_player: bool,
	forbid_random_targets: bool,
	forbid_random_talks: bool,
	not_interested_in_buildings: bool,
	patrol_radius: Option<u64>,
	relations: Relations,
}
#[derive(Clone, Debug)]
pub enum Target {
	Army(usize),
	Building(usize)
}
#[derive(Clone, Debug)]
pub enum Plan {
	ToTax,
	ToMarket,
	ToTalk
}
#[derive(Clone, Debug, Default)]
pub struct PC_ControlState {
	current_target: Option<Target>,
	plan: Option<Plan>
}
