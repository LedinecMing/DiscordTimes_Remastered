use std::collections::HashMap;

use crate::lib::{
    battle::{army::Army, troop::Troop},
    items::item::{ITEMS},
    units::unit::Unit,
};
use rand::{seq::SliceRandom, thread_rng};
use alkahest::alkahest;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ObjectType {
    Building,
	Bridge,
    MapDeco,
}

#[derive(Clone, Debug)]
pub struct ObjectInfo {
	pub name: String,
    pub path: String,
    pub category: String,
    pub obj_type: ObjectType,
    pub index: usize,
    pub size: (u8, u8),
}

enum Building {
	Village(u64, u64),
	Castle,
	Ruined
}

#[derive(Clone, Debug)]
#[alkahest(Deserialize, Serialize, SerializeRef, Formula)]
pub struct MapBuildingdata {
	pub name: String,
	pub desc: String,
	pub id: usize,
    pub event: Vec<usize>,
    pub market: Option<Market>,
    pub recruitment: Option<Recruitment>,
    pub pos: (usize, usize),
    pub defense: u64,
    pub income: u64,
    pub owner: Option<usize>,
}

const RECRUIT_COST: f64 = 2.0;
#[derive(Clone, Debug)]
#[alkahest(Deserialize, Serialize, SerializeRef, Formula)]
pub struct Market {
    pub itemcost_range: (u64, u64),
    pub items: Vec<usize>,
    pub max_items: usize,
}
impl Market {
    fn update(&mut self) {
        for _ in self.max_items - self.items.len()..0 {
			let items = ITEMS.lock().unwrap();
			let nice_items = items
				.iter()
				.filter(|(_, item)|
						self.itemcost_range.0
						<= item.cost
						&&
						item.cost <= self.itemcost_range.1 )
				.collect::<Vec<_>>();
			self.items.append(
				&mut nice_items
					.choose_multiple(&mut thread_rng(), self.max_items)
					.map(|(index, _)| **index)
					.collect()
			);
		}
    }
    fn buy(&mut self, buyer: &mut Army, item_num: usize) {
        if self.can_buy(buyer, item_num) {
            buyer.stats.gold = buyer
                .stats
                .gold
                .saturating_sub(self.get_item_cost(item_num));
            buyer.add_item(self.items.remove(item_num));
        }
    }
    fn can_buy(&self, buyer: &Army, item_num: usize) -> bool {
        if ITEMS.lock().unwrap()[
			self
            .items
            .get(item_num)
            .expect("Trying to get item at unknown place")
            ]
            .sells
        {
            return buyer.stats.gold >= self.get_item_cost(item_num);
        }
        false
    }
    fn get_item_cost(&self, item_num: usize) -> u64 {
		ITEMS.lock().unwrap()[
			self
			.items
            .get(item_num)
            .expect("Trying to get item at unknown place")
		]
        .cost
    }
}
#[derive(Clone, Debug)]
#[alkahest(Deserialize, Serialize, SerializeRef, Formula)]
pub struct RecruitUnit {
    pub unit: usize,
    pub count: usize,
}
#[derive(Clone, Debug)]
#[alkahest(Deserialize, Serialize, SerializeRef, Formula)]
pub struct Recruitment {
    pub units: Vec<RecruitUnit>,
    pub cost_modify: f64,
}
impl Recruitment {
    pub fn buy(&mut self, buyer: &mut Army, unit_num: usize, units: &HashMap<usize, Unit>) -> Result<(),()> {
        if self.can_buy(buyer, unit_num, units) {
			buyer.add_troop(Troop {
                unit: units[&self.units[unit_num].unit].clone(),
                ..Troop::empty()
            }.into())?;
			self.units[unit_num].count-=1;
			buyer.stats.gold -= units[&self.units[unit_num].unit].info.cost_hire;
        }
		Err(())
    }
    pub fn can_buy(&self, buyer: &Army, unit_num: usize, units: &HashMap<usize, Unit>) -> bool {
		let info = &self
					  .units
					  .get(unit_num)
			.expect("Trying to get unit at unknown index");
        buyer.stats.gold
            >= units[&info.unit].info.cost_hire && info.count > 0
                //* (RECRUIT_COST * self.cost_modify)) as u64;
    }
}
