use crate::{
    lib::{
        battle::{troop::Troop, battlefield::{field_type, Field}},
        items::item::Item,
        map::{map::{GameMap, MAP_SIZE}, object::{ObjectInfo, ObjectType}},
        mutrc::SendMut,
		parse::SETTINGS, units::unit::UnitPos
    },
    TILES,
};
use num::{integer::sqrt, pow};
use once_cell::sync::Lazy;
use pathfinding::directed::astar::astar;
use advini::{Sections, Ini, Section, SectionError, IniParseError};
use alkahest::{alkahest, private::*};

#[derive(Clone, Debug, Default)]
#[alkahest(Deserialize, Serialize, SerializeRef, Formula)]
pub struct Relations {
    player: u8,
    ally: u8,
    neighbour: u8,
    enemy: u8
}
impl Ini for Relations {
	fn eat<'a>(chars: std::str::Chars<'a>) -> Result<(Self, std::str::Chars<'a>), IniParseError> {
		match <(u8, u8, u8, u8) as Ini>::eat(chars) {
			Ok(v) => Ok({
				let rels = v.0;
				(Self { player: rels.0, ally: rels.1, neighbour: rels.2, enemy: rels.3 }, v.1)
			}),
			Err(err) => Err(err)
		}
	}
	fn vomit(&self) -> String {
		(self.player, self.ally, self.neighbour, self.enemy).vomit()
	}
}
#[derive(Clone, Debug, Default, Sections)]
#[alkahest(Deserialize, Serialize, SerializeRef, Formula)]
pub struct ArmyStats {
	#[default_value="0_u64"]
    pub gold: u64,
	#[default_value="0_u64"]
    pub mana: u64,
    pub army_name: String,
}
impl ArmyStats {
	fn new(gold: u64, mana: u64, army_name: String) -> Self {
		Self { gold, mana, army_name }
	}
}
fn eq_fields(index: usize, index1: usize, columns: usize, rows: usize) -> bool {
    let start_field = field_type(index, *MAX_TROOPS);
    let this_field = field_type(index1, *MAX_TROOPS);

	matches!((start_field, this_field), (Field::Back | Field::Front, Field::Back | Field::Front)) ||
		matches!((start_field, this_field), (Field::Reserve, Field::Reserve))
}

pub const MAX_LINES : usize = 2;

#[derive(Clone, Debug, Default)]
#[alkahest(Deserialize, Serialize, SerializeRef, Formula)]
pub enum Control {
	#[default]
	PC,
	Player(usize)
}
impl Ini for Control {
	fn eat<'a>(chars: std::str::Chars<'a>) -> Result<(Self, std::str::Chars<'a>), IniParseError> {
		let (tag, chars) = match <u8 as Ini>::eat(chars) {
			Ok(v) => Ok(v),
			Err(err) => Err(err)
		}?;
		match tag {
			0 => Ok((Control::PC, chars)),
			_ => {
				let (v, chars) = match <usize as Ini>::eat(chars) {
					Ok(v) => Ok(v),
					Err(err) => Err(err)
				}?;
				Ok((Control::Player(v), chars))
			}
		}
	}
	fn vomit(&self) -> String {
		match self {
			Control::PC => 0_u8.vomit(),
			Control::Player(n) => (0_u8, *n).vomit()
		}
	}
}
#[derive(Clone, Debug, Default)]
//#[alkahest(Formula)]
#[alkahest(Deserialize, Serialize, SerializeRef, Formula)]
pub struct Army {
	pub troops: Vec<TroopType>, // vec of units
	//#[unused]
	pub hitmap: Vec<Option<usize>>, // map of maybe units ids
	//#[unused]
	pub building: Option<usize>, // what building army in
	//#[inline_parsing]
    pub stats: ArmyStats,
	//#[default_value="Vec::new()"]
    pub inventory: Vec<Item>,
    pub pos: (usize, usize),
	//#[default_value="true"]
	pub active: bool,
	//#[unused]
	pub defeated: bool,
	//#[default_value = "Control::PC"]
	pub control: Control,
	//#[unused]
    pub path: Vec<(usize, usize)>,
}
// [TODO REMOVE NAHUJ]
// impl<'de> Deserialize<'de, Self> for Army {
// 	fn deserialize(mut de: Deserializer<'de>) -> Result<Self, DeserializeError>
//     where
//         Self: Sized {
// 		let formula = with_formula(|s: &Self| match *s {
// 			Self { ref troops, ref hitmap, ref building, ref stats, ref inventory, ref pos, ref active, ref defeated, ref control, ref path } => hitmap,
// 			_ => unreachable!()
// 		});
// 		let hitmap = formula.read_field(&mut de, false)?;
// 		Ok(Self { hitmap, ..Default::default() })
// 	}
// }
pub type TroopType = SendMut<Troop>;

pub static MAX_TROOPS: Lazy<usize> = Lazy::new(|| unsafe{&SETTINGS}.max_troops);

impl Army {
	pub fn new(
        troops: Vec<TroopType>,
        stats: ArmyStats,
        inventory: Vec<Item>,
        pos: (usize, usize),
		active: bool,
		control: Control,
    ) -> Self {
		let hitmap: Vec<Option<usize>> = (0..*MAX_TROOPS).map(|_| None::<usize>).collect();
		let mut army = Army {
			troops: Vec::new(),
			building: None,
			hitmap,
			defeated: false,
			stats,
			control,
			inventory,
			pos,
			active,
			path: Vec::new()
		};
		for troop in troops {
			army.add_troop(troop).ok();
		}
		army
    }
	pub fn recalc_hitmap(troops: &Vec<TroopType>, hitmap: &mut Vec<Option<usize>>, columns: usize) {
		let info = troops.iter().enumerate().map(|(num, troop)| {
			let troop = troop.get();
			(troop.unit.info.size, troop.pos, num)
		});
		for (size, pos, num) in info {
			for j in 0..size.1 {
				for i in 0..size.0 {
					hitmap[(j + pos.1) * columns + (i + pos.0)] = Some(num);
				}
			}
		}
	}
	pub fn recalc_army_hitmap(&mut self) {
		let mut hitmap = Vec::with_capacity(*MAX_TROOPS);
		for _ in 0..*MAX_TROOPS { hitmap.push(None); }
		Army::recalc_hitmap(&self.troops, &mut hitmap, *MAX_TROOPS / MAX_LINES);
		self.hitmap = hitmap
	}
	pub fn get_army_slice<'a>(troops: &'a mut Vec<Troop>) -> Vec<&'a mut [Troop]>
	{
		let len = troops.len();
		let mut res = Vec::new();
		fn get_slices<'a>(mut res: Vec<&'a mut [Troop]>, s: &'a mut [Troop]) -> Vec<&'a mut [Troop]> {
			let len = s.len();
			let (troop1, troop2) = s.split_at_mut(len / 2);
			
			if troop1.len() == 1 {
			    res.push(troop1);
			} else {
			    res = get_slices(res, troop1);
			}
			if troop2.len() == 1 {
			    res.push(troop2);
			} else {
			    res = get_slices(res, troop2);
			}   
			
			res
		}
		res = get_slices(res, troops.as_mut_slice());
		res
	}
	/*
	rffffr - 1;2;3
    rbbbbr - 7;8;9
	
 	rfTffr - 3; - possible positions to fit with size
  	rbTbTr
	where f - front; r - reserve; T - troop; b - back
	 */
	pub fn fit_to(hitmap: &[Option<usize>], size: (usize, usize), columns: usize, rows: usize, row: usize, column: usize) -> bool {
		let mut fits = true;
		let mut index = row*columns+column;
		'check_rect: for j in 0..size.1 {
			for i in 0..size.0 {
				let my_index = (row+j)*columns+(i+column);
				if hitmap[my_index].is_some() || !eq_fields(my_index, index, columns, rows) {
					fits = false;
					break 'check_rect;
				}
				index = my_index;
			}
		}
		fits
	}
	pub fn fit(hitmap: &[Option<usize>], size: (usize, usize), rows: usize, columns: usize) -> Vec<usize> {
		let mut possible = Vec::new();
		for row in 0..=(rows - size.1) {
			for column in 0..=(columns - size.0) {
				if Army::fit_to(hitmap, size, columns, rows, row, column) {
					possible.push(row*columns+column);
				}
			}
		}
		possible
	}
		
	pub fn add_troop(&mut self, wrap_troop: TroopType) -> Result<(), ()> {
		let size = wrap_troop.get().unit.info.size;
		let res = Army::fit(&self.hitmap, size, MAX_LINES, *MAX_TROOPS / MAX_LINES);
		let pos = res.first().ok_or(())?;
		wrap_troop.get().pos = UnitPos::from_index(*pos);
		self.troops.push(wrap_troop);
		self.recalc_army_hitmap();
		Ok(())
	}
	
    pub fn add_item(&mut self, item: usize) {
        self.inventory.push(Item { index:item })
    }
	pub fn remove_item(&mut self, rem_item: usize) {
		if let Some(index) = self.inventory.iter().position(|item| item.index==rem_item) {
			self.inventory.remove(index);
		}
	}
	pub fn get_troop(&self, pos: usize) -> Option<TroopType> {
		if let Some(index) = self.hitmap[pos] {
			return self.troops.get(index).cloned()
		};
		None
    }

}

fn dist(p1: &(usize, usize), p2: &(usize, usize)) -> u32 {
    sqrt((pow(p1.0 as isize - p2.0 as isize, 2) + pow(p1.1 as isize - p2.1 as isize, 2)) as u32)
}
pub fn find_path(
    gamemap: &GameMap,
	objects: &[ObjectInfo],
    start: (usize, usize),
    goal: (usize, usize),
    on_transport: bool,
) -> Option<(Vec<(usize, usize)>, u32)> {
    let path = astar(
        &start,
        |&(x, y)| {
            let edge = (
                (x as f32 / (MAP_SIZE as f32 - 1.)),
                (y as f32 / (MAP_SIZE as f32 - 1.)),
            );
            match edge {
                (0., 0.) => vec![(x + 1, y), (x + 1, y + 1), (x, y + 1)],
                (0., 1.) => vec![(x + 1, y), (x + 1, y - 1), (x, y - 1)],
                (1., 1.) => vec![(x - 1, y), (x - 1, y - 1), (x, y - 1)],
                (1., 0.) => vec![(x - 1, y), (x - 1, y + 1), (x, y + 1)],
                (0., _) => vec![
                    (x + 1, y),
                    (x + 1, y + 1),
                    (x, y + 1),
                    (x + 1, y - 1),
                    (x, y - 1),
                ],
                (1., _) => vec![
                    (x - 1, y),
                    (x - 1, y + 1),
                    (x, y + 1),
                    (x - 1, y - 1),
                    (x, y - 1),
                ],
                (_, 0.) => vec![
                    (x - 1, y),
                    (x - 1, y + 1),
                    (x, y + 1),
                    (x + 1, y + 1),
                    (x + 1, y),
                ],
                (_, 1.) => vec![
                    (x - 1, y),
                    (x - 1, y - 1),
                    (x, y - 1),
                    (x + 1, y - 1),
                    (x + 1, y),
                ],
                (_, _) => vec![
                    (x - 1, y - 1),
                    (x, y - 1),
                    (x + 1, y - 1),
                    (x + 1, y),
                    (x + 1, y + 1),
                    (x, y + 1),
                    (x - 1, y + 1),
                    (x - 1, y),
                ],
            }
            .into_iter()
            .filter(|p: &(usize, usize)| {
                let hitbox = &gamemap.hitmap[p.0][p.1];
                hitbox.passable() && (!(hitbox.need_transport ^ on_transport) || hitbox.building.is_some_and(|n| objects[gamemap.buildings[n].id].obj_type == ObjectType::Bridge))
            })
            .map(|p| (p, 10 / TILES[gamemap.tilemap[p.0][p.1]].walkspeed))
        },
        |&p| dist(&p, &goal),
        |&p| p == goal,
    )?;
    Some((path.0[1..].to_vec(), path.1))
}

