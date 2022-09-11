use {
    dyn_clone::DynClone,
    crate::lib::{
        battle::{
            army::Army,
            troop::Troop
        },
        items::item::Item,
        units::unit::Unit,
        map::event::Event,
    },
    rand::seq::SliceRandom,
    crate::MutRc
};


pub struct MapObjectData {
    pub event: Option<Event>,
    pub market: Option<Market>,
    pub recruitment: Option<Recruitment>,
    pub owner: MutRc<Army>
}

const RECRUIT_COST: f64 = 2.0;
pub struct Market {
    pub itemcost_range: [i32; 2],
    pub items: Vec<Item>,
    pub max_items: usize
}
impl Market {
    fn update(&mut self) {
        for _ in self.max_items - self.items.len()..0 {
            self.items.push([Item::CoolSword()].choose(&mut rand::thread_rng()).expect("Ты че наделал?").clone());
        }
    }
    fn buy(&mut self, buyer: &mut Army, item_num: usize) {
        if self.can_buy(buyer, item_num) {
            buyer.stats.gold -= self.get_item_cost(item_num);
            buyer.add_item(self.items.remove(item_num));
        }
    }
    fn can_buy(&self, buyer: &Army, item_num: usize) -> bool {
        if self.items.get(item_num).expect("Trying to get item at unknown place").info.sells {
            return buyer.stats.gold >= self.items[item_num].info.cost
        }
        false
    }
    fn get_item_cost(&self, item_num: usize) -> u64
    {
        self.items.get(item_num).expect("Trying to get item at unknown place").info.cost
    }
}
pub struct RecruitUnit {
    pub unit: Box<dyn Unit>,
    pub count: usize
}
pub struct Recruitment {
    pub units: Vec<RecruitUnit>,
    pub cost_modify: f64,
}
impl Recruitment {
    pub fn buy(&self, buyer: &mut Army, unit_num: usize) {
        if self.can_buy(buyer, unit_num) {
            match buyer.add_troop(Troop { unit: self.units.get(unit_num).unwrap().unit.clone(), ..Troop::empty()}) {
                Ok(()) => println!("Успешно приобретён юнит"),
                Err(()) => println!("Произошла ошибка")
            };
    }   }
    pub fn can_buy(&self, buyer: &Army, unit_num: usize) -> bool {
        return buyer.stats.gold >= (self.units.get(unit_num).expect("Trying to get unit at unknown index").unit.get_data().info.cost as f64 * (RECRUIT_COST * self.cost_modify)) as u64
    }
}

dyn_clone::clone_trait_object!(MapObject);
pub trait MapObject : DynClone {
    fn on_step(&self) {}
    fn get_data(&self) {}
}

