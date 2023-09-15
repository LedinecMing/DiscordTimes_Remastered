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


#[derive(Clone, Debug, Default)]
pub struct Relations {
    player: u8,
    ally: u8,
    neighbour: u8,
    enemy: u8
}

#[derive(Clone, Debug, Default)]
pub struct ArmyStats {
    pub gold: u64,
    pub mana: u64,
    pub army_name: String,
}

fn eq_fields(index: usize, index1: usize, columns: usize, rows: usize) -> bool {
    let start_field = field_type(index, *MAX_TROOPS);
    let this_field = field_type(index1, *MAX_TROOPS);

	matches!((start_field, this_field), (Field::Back | Field::Front, Field::Back | Field::Front)) ||
		matches!((start_field, this_field), (Field::Reserve, Field::Reserve))
}

pub const MAX_LINES : usize = 2;

#[derive(Clone, Debug, Default)]
pub enum Control {
	#[default]
	PC,
	Player(usize)
}
#[derive(Clone, Debug, Default)]
pub struct Army {
	pub troops: Vec<SendMut<Troop>>, // vec of units
	pub hitmap: Vec<Option<usize>>, // map of maybe units ids
	pub building: Option<usize>, // what building army inи
    pub stats: ArmyStats,
    pub inventory: Vec<Item>,
    pub pos: (usize, usize),
	pub active: bool,
	pub defeated: bool,
	pub control: Control,
    pub path: Vec<(usize, usize)>,
}
pub type TroopType = SendMut<Troop>;

pub static MAX_TROOPS: Lazy<usize> = Lazy::new(|| unsafe{&SETTINGS}.max_troops);

impl Army {
	pub fn recalc_hitmap(troops: &Vec<SendMut<Troop>>, hitmap: &mut Vec<Option<usize>>, columns: usize) {
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
	/*
	rffffr - 1;2;3
    rbbbbr - 7;8;9
	
 	rfTffr - 3; - possible positions to fit with size
  	rbTbTr
	where f - front; r - reserve; T - troop; b - back
	 */
	fn fit_to(hitmap: &[Option<usize>], size: (usize, usize), columns: usize, rows: usize, row: usize, column: usize) -> bool {
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
	fn fit(hitmap: &[Option<usize>], size: (usize, usize), rows: usize, columns: usize) -> Vec<usize> {
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
    pub fn new(
        troops: Vec<TroopType>,
        stats: ArmyStats,
        inventory: Vec<Item>,
		control: Control,
        pos: (usize, usize),
		active: bool
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
