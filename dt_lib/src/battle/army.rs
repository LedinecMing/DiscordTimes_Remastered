use crate::{
    battle::{
        battlefield::{Field, field_type},
        troop::Troop,
    }, items::item::Item, map::{
        map::{GameMap, MAP_SIZE},
        object::{ObjectInfo, ObjectType},
        tile::TILES,
    }, mutrc::SendMut, registry::{self, GameInfo, Units}, units::{self, unit::{Unit, UnitPos}}
};
use advini::{Ini, IniParseError, Section, SectionError, Sections};
use alkahest::{alkahest, private::*};
use num::{integer::sqrt, pow};
use once_cell::sync::Lazy;
use pathfinding::directed::astar::astar;
use zerocopy::Usize;

use super::{
    control::{Control, PC_ControlSetings},
    troop_inactive, BattleUnitPos,
};
#[derive(Clone, Debug, Default, Sections)]
#[alkahest(Deserialize, Serialize, SerializeRef, Formula)]
pub struct ArmyStats {
    #[default_value = "0_u64"]
    pub gold: u64,
    #[default_value = "0_u64"]
    pub mana: u64,
    pub army_name: String,
}
impl ArmyStats {
    fn new(gold: u64, mana: u64, army_name: String) -> Self {
        Self {
            gold,
            mana,
            army_name,
        }
    }
}
fn eq_fields(index: usize, index1: usize, columns: usize, rows: usize) -> bool {
    let start_field = field_type(index, MAX_TROOPS);
    let this_field = field_type(index1, MAX_TROOPS);

    matches!(
        (start_field, this_field),
        (Field::Back | Field::Front, Field::Back | Field::Front)
    ) || matches!((start_field, this_field), (Field::Reserve, Field::Reserve))
}

pub const MAX_LINES: usize = 2;

pub type HitMap = Vec<Option<usize>>;
#[derive(Clone, Debug, Default)]
#[alkahest(Deserialize, Serialize, SerializeRef, Formula)]
pub struct Army {
    pub troops: Vec<TroopType>, // vec of units
    //#[unused]
    pub hitmap: HitMap, // map of maybe units ids
    //#[unused]
    pub building: Option<usize>, // what building army in
    //#[inline_parsing]
    pub stats: ArmyStats,
    //#[default_value="Vec::new()"]
    pub inventory: Vec<Option<Item>>,
    pub pos: (usize, usize),
	pub transport: bool,
    //#[default_value="true"]
    pub active: bool,
    //#[unused]
    pub defeated: bool,
    //#[default_value = "Control::PC"]
    pub control: Control,
    pub pc_settings: Option<PC_ControlSetings>,
    //#[unused]
    pub path: Vec<(usize, usize)>,
}
pub type TroopType = SendMut<Troop>;

pub static MAX_TROOPS: usize = 12;

impl Army {
    pub fn new(
        troops: Vec<TroopType>,
        stats: ArmyStats,
        mut inventory: Vec<Option<Item>>,
        pos: (usize, usize),
        active: bool,
        control: Control,
		registry: &GameInfo
    ) -> Self {
        let hitmap: HitMap = (0..MAX_TROOPS).map(|_| None::<usize>).collect();
        if inventory.len() < 4 {
            inventory.extend([None].iter().cycle().take(4 - inventory.len()));
        }
        let mut army = Army {
            pc_settings: None,
            troops: Vec::new(),
            building: None,
            hitmap,
            defeated: false,
			// TODO FIX
			transport: true,
            stats,
            control,
            inventory,
            pos,
            active,
            path: Vec::new(),
        };
        for troop in troops {
            army.add_troop(troop, &registry.units).ok();
        }
        army
    }
    pub fn recalc_hitmap_solo(
        hitmap: &mut HitMap,
        size: (usize, usize),
        pos: UnitPos,
        num: usize,
        columns: usize,
    ) {
        for j in 0..size.1 {
            for i in 0..size.0 {
                hitmap[(j + pos.1) * columns + (i + pos.0)] = Some(num);
            }
        }
    }
    pub fn recalc_hitmap(troops: &Vec<TroopType>, hitmap: &mut HitMap, columns: usize, registry: &Units) {
        let info = troops.iter().enumerate().filter_map(|(num, troop)| {
            let troop = troop.get();
            if troop.is_dead() {
                return None;
            }
            Some((troop.unit.get_info(registry).size, troop.pos, num))
        });
        for (size, pos, num) in info {
            Army::recalc_hitmap_solo(hitmap, size, pos, num, columns);
        }
    }
    pub fn recalc_army_hitmap(&mut self, registry: &Units) {
        let mut hitmap = Vec::with_capacity(MAX_TROOPS);
        for _ in 0..MAX_TROOPS {
            hitmap.push(None);
        }
        Army::recalc_hitmap(&self.troops, &mut hitmap, MAX_TROOPS / MAX_LINES, registry);
        self.hitmap = hitmap
    }
    pub fn set_unit_at(
        &mut self,
        unit_id: Option<usize>,
        BattleUnitPos { army, pos }: BattleUnitPos,
		registry: &GameInfo,
    ) {
        let index = self.hitmap.get(pos);
        if let Some(Some(index)) = index {
            self.troops.remove(*index);
        }
        if let Some(unit_id) = unit_id {
            let new_unit_info = registry.units[unit_id].clone();
            let dpos = UnitPos::from_index(pos);
            if !Army::fit_to(
                &self.hitmap,
                new_unit_info.size,
                MAX_TROOPS / 2,
                MAX_LINES,
                dpos.1,
                dpos.0,
            ) {
                return;
            }
			let mut new_unit = Unit::from((new_unit_info, &registry.bonuses));
            new_unit.restore();
            let mut new_troop = Troop::new(new_unit);
            new_troop.pos = UnitPos::from_index(pos);
            self.troops.push(new_troop.into());
        } else {
        }
        self.recalc_army_hitmap(&registry.units);
    }
    pub fn set_item_unit_at(
        &mut self,
        item_id: Option<usize>,
        BattleUnitPos { army, pos }: BattleUnitPos,
        index: usize,
		registry: &GameInfo,
    ) -> bool {
        if let Some(troop) = self.get_troop(pos) {
            let unit = &mut troop.get().unit;
            if let Some(item_id) = item_id {
                unit.swap_item(index, Some(Item { index: item_id }), registry);
                unit.restore();
                true
            } else {
                false
            }
        } else {
            false
        }
    }
    pub fn get_army_slice<'a>(troops: &'a mut Vec<Troop>) -> Vec<&'a mut [Troop]> {
        let len = troops.len();
        let mut res = Vec::new();
        fn get_slices<'a>(
            mut res: Vec<&'a mut [Troop]>,
            s: &'a mut [Troop],
        ) -> Vec<&'a mut [Troop]> {
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
    pub fn fit_to(
        hitmap: &[Option<usize>],
        size: (usize, usize),
        columns: usize,
        rows: usize,
        row: usize,
        column: usize,
    ) -> bool {
        let mut fits = true;
        let mut index = row * columns + column;
        'check_rect: for j in 0..size.1 {
            for i in 0..size.0 {
                let my_index = (row + j) * columns + (i + column);
                if hitmap[my_index].is_some() || !eq_fields(my_index, index, columns, rows) {
                    fits = false;
                    break 'check_rect;
                }
                index = my_index;
            }
        }
        fits
    }
    pub fn fit(
        hitmap: &[Option<usize>],
        size: (usize, usize),
        rows: usize,
        columns: usize,
    ) -> Vec<usize> {
        let mut possible = Vec::new();
        for row in 0..=(rows - size.1) {
            for column in 0..=(columns - size.0) {
                if Army::fit_to(hitmap, size, columns, rows, row, column) {
                    possible.push(row * columns + column);
                }
            }
        }
        possible
    }

    pub fn add_troop(&mut self, wrap_troop: TroopType, registry: &Units) -> Result<(), ()> {
        let size = wrap_troop.get().unit.get_info(registry).size;
        let res = Army::fit(&self.hitmap, size, MAX_LINES, MAX_TROOPS / MAX_LINES);
        let pos = res.first().ok_or(())?;
        wrap_troop.get().pos = UnitPos::from_index(*pos);
        self.troops.push(wrap_troop);
        self.recalc_army_hitmap(registry);
        Ok(())
    }

    pub fn add_item(&mut self, item: Item) {
        let first = self.inventory.iter().position(|x| x.is_none());
        if let Some(index) = first {
            self.inventory[index] = Some(item);
        }
    }
    pub fn remove_item(&mut self, rem_item: usize) {
        if let Some(index) = self
            .inventory
            .iter()
            .position(|item| *item == Some(Item { index: rem_item }))
        {
            self.inventory.remove(index);
        }
    }
    pub fn get_troop(&self, pos: usize) -> Option<TroopType> {
        if let Some(index) = self.hitmap[pos] {
            return self.troops.get(index).cloned();
        };
        None
    }
}

fn dist(p1: &(usize, usize), p2: &(usize, usize)) -> u32 {
    sqrt((pow(p1.0 as isize - p2.0 as isize, 2) + pow(p1.1 as isize - p2.1 as isize, 2)) as u32)
}
pub fn find_path(
    gamemap: &GameMap,
    start: (usize, usize),
    goal: (usize, usize),
    on_transport: bool,
	registry: &GameInfo
) -> Option<(Vec<(usize, usize)>, u32)> {
    let path = astar(
        &start,
        |&(x, y)| {
			let size = gamemap.tilemap.size as f32 - 1.;
            let edge = (
                (x as f32 / size),
                (y as f32 / size),
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
			.filter_map(|p: (usize, usize)| {
                let hitbox = &gamemap.hitmap[(p.1, p.0)];
                if hitbox.passable() && (!hitbox.need_transport || on_transport) {
					Some((p, false))
				} else if hitbox.building.is_some_and(|n| registry.objects.inner.iter().find(|x| x.index==gamemap.buildings[n].id).is_some_and(|x| matches!(x.obj_type, ObjectType::Bridge { .. })))  {
					Some((p, true))
				} else { None }
            })
			.map(|(p, is_bridge)| (p, if is_bridge {10} else {10 / TILES[gamemap.tilemap[(p.1, p.0)]].walkspeed}))
        },
        |&p| dist(&p, &goal),
        |&p| p == goal,
    )?;
    Some((path.0[1..].to_vec(), path.1))
}
