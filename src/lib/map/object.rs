use {
    crate::lib::{
        battle::{army::Army, troop::Troop},
        items::item::Item,
        map::event::Event,
        mutrc::SendMut,
        units::unit::Unit,
    },
    dyn_clone::DynClone,
    rand::seq::SliceRandom,
};

#[derive(Clone)]
pub struct MapObjectData {
    pub event: Option<Vec<Event>>,
    pub market: Option<Market>,
    pub recruitment: Option<Recruitment>,
    pub owner: Option<SendMut<Army>>,
}

const RECRUIT_COST: f64 = 2.0;
#[derive(Clone)]
pub struct Market {
    pub itemcost_range: [u64; 2],
    pub items: Vec<Item>,
    pub max_items: usize,
}
impl Market {
    fn update(&mut self) {
        for _ in self.max_items - self.items.len()..0 {
            self.items.push(
                [Item::CoolSword()]
                    .choose(&mut rand::thread_rng())
                    .expect("Ты че наделал?")
                    .clone(),
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
        if self
            .items
            .get(item_num)
            .expect("Trying to get item at unknown place")
            .info
            .sells
        {
            return buyer.stats.gold >= self.items[item_num].info.cost;
        }
        false
    }
    fn get_item_cost(&self, item_num: usize) -> u64 {
        self.items
            .get(item_num)
            .expect("Trying to get item at unknown place")
            .info
            .cost
    }
}
#[derive(Clone)]
pub struct RecruitUnit {
    pub unit: Unit,
    pub count: usize,
}
#[derive(Clone)]
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
                .cost as f64
                * (RECRUIT_COST * self.cost_modify)) as u64;
    }
}

dyn_clone::clone_trait_object!(MapObject);
pub trait MapObject: DynClone {
    fn on_step(&self) {}
    fn get_data(&self) -> MapObjectData;
}

#[derive(Clone)]
struct Building {
    data: MapObjectData,
}
impl MapObject for Building {
    fn get_data(&self) -> MapObjectData {
        self.data.clone()
    }
}
