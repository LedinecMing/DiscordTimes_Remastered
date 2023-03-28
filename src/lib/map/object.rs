use crate::lib::{
    battle::{army::Army, troop::Troop},
    items::item::{Item, ITEMS},
    map::event::Event,
    units::unit::Unit,
};
use rand::seq::SliceRandom;

#[derive(Clone, Copy, Debug)]
pub enum ObjectType {
    Building,
    MapDeco,
}

#[derive(Clone, Debug)]
pub struct ObjectInfo {
    pub path: String,
    pub category: String,
    pub obj_type: ObjectType,
    pub index: usize,
    pub size: (u8, u8),
}

#[derive(Clone, Debug)]
pub struct MapBuildingdata {
    pub event: Vec<Event>,
    pub market: Option<Market>,
    pub recruitment: Option<Recruitment>,
    pub pos: (usize, usize),
    pub size: (u8, u8),
    pub defense: u64,
    pub income: u64,
    pub owner: usize,
}

const RECRUIT_COST: f64 = 2.0;
#[derive(Clone, Debug)]
pub struct Market {
    pub itemcost_range: [u64; 2],
    pub items: Vec<Item>,
    pub max_items: usize,
}
impl Market {
    fn update(&mut self) {
        for _ in self.max_items - self.items.len()..0 {}
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
        if self
            .items
            .get(item_num)
            .expect("Trying to get item at unknown place")
            .get_info()
            .sells
        {
            return buyer.stats.gold >= self.items[item_num].get_info().cost as u64;
        }
        false
    }
    fn get_item_cost(&self, item_num: usize) -> u64 {
        self.items
            .get(item_num)
            .expect("Trying to get item at unknown place")
            .get_info()
            .cost
    }
}
#[derive(Clone, Debug)]
pub struct RecruitUnit {
    pub unit: Unit,
    pub count: usize,
}
#[derive(Clone, Debug)]
pub struct Recruitment {
    pub units: Vec<RecruitUnit>,
    pub cost_modify: f64,
}
impl Recruitment {
    pub fn buy(&self, buyer: &mut Army, unit_num: usize) {
        if self.can_buy(buyer, unit_num) {
            match buyer.add_troop(Troop {
                unit: self.units[unit_num].clone().unit,
                ..Troop::empty()
            }) {
                Ok(()) => println!("Успешно приобретён юнит"),
                Err(()) => println!("Произошла ошибка"),
            };
        }
    }
    pub fn can_buy(&self, buyer: &Army, unit_num: usize) -> bool {
        return buyer.stats.gold
            >= (self
                .units
                .get(unit_num)
                .expect("Trying to get unit at unknown index")
                .unit
                .info
                .cost_hire as f64
                * (RECRUIT_COST * self.cost_modify)) as u64;
    }
}
