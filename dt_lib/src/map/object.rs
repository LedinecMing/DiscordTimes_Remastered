use std::collections::HashMap;

use crate::{
    battle::{army::Army, control::Relations, troop::Troop},
    items::{item::ITEMS, Item},
    units::unit::Unit,
};
use advini::*;
use alkahest::alkahest;
use num_enum::FromPrimitive;
use rand::{seq::SliceRandom, thread_rng};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ObjectType {
    Building,
    Bridge,
    MapDeco,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ObjectInfo {
    pub name: String,
    pub path: String,
    pub category: String,
    pub obj_type: ObjectType,
    pub index: usize,
    /// DTm id
    pub id: usize,
    pub size: (u8, u8),
}

#[derive(Clone, Debug, PartialEq)]
#[alkahest(Deserialize, Serialize, SerializeRef, Formula)]
pub struct Village {
    pub max_gold: u64,
    pub max_mana: u64,
}
impl Ini for Village {
    fn eat(chars: std::str::Chars) -> Result<(Self, std::str::Chars), IniParseError> {
        let (max_gold, chars) = u64::eat(chars)?;
        let (max_mana, chars) = u64::eat(chars)?;
        Ok((Self { max_gold, max_mana }, chars))
    }
    fn vomit(&self) -> String {
        [self.max_gold.vomit(), self.max_mana.vomit()].join(",")
    }
}
#[derive(Clone, Debug, Ini, PartialEq)]
#[alkahest(Deserialize, Serialize, SerializeRef, Formula)]
pub enum BuildingVariant {
    Town,
    Village(Village),
    Castle,
    Fort,
    Tavern,
    Market,
    Church,
    Forge,
    Verf,
    Altar,
    Mine,
    Ruins(Vec<Item>),
    StoneBridge,
    WoodenBridge,
}
#[derive(Clone, Debug, Sections, PartialEq)]
#[alkahest(Deserialize, Serialize, SerializeRef, Formula)]
pub struct MapBuildingdata {
    pub name: String,
    #[alias([description])]
    #[default_value = "String::new()"]
    pub desc: String,
    #[default_value = "String::new()"]
    pub owner_name: String,

    pub id: usize,

    #[default_value = "vec![]"]
    pub events: Vec<usize>,
    #[default_value = "BuildingVariant::Castle"]
    pub variant: BuildingVariant,
    #[inline_parsing]
    pub market: Option<Market>,
    #[inline_parsing]
    pub recruitment: Option<Recruitment>,

    pub pos: (usize, usize),
    #[default_value = "None"]
    pub owner: Option<usize>,

    #[default_value = "vec![]"]
    pub garrison: Vec<Unit>,
    #[default_value = "0u64"]
    pub additional_defense: u64,

    #[default_value = "0u64"]
    pub gold_income: u64,
    #[default_value = "0u64"]
    pub mana_income: u64,

    #[default_value = "vec![]"]
    pub spells_to_learn: Vec<usize>,

    pub relations: Relations,
    #[default_value = "0usize"]
    pub group: usize,
}
const RECRUIT_COST: f64 = 2.0;
#[derive(Clone, Debug, Sections, PartialEq)]
#[alkahest(Deserialize, Serialize, SerializeRef, Formula)]
pub struct Market {
    pub itemcost_range: (u64, u64),
    pub items: Vec<Item>,
    pub max_items: usize,
}
impl Market {
    fn new(itemcost_range: (u64, u64), items: Vec<Item>, max_items: usize) -> Self {
        Self {
            itemcost_range,
            items,
            max_items,
        }
    }
    fn update(&mut self) {
        for _ in self.max_items - self.items.len()..0 {
            let items = ITEMS.read().unwrap();
            let nice_items = items
                .iter()
                .enumerate()
                .filter(|(_, item)| {
                    self.itemcost_range.0 <= item.cost && item.cost <= self.itemcost_range.1
                })
                .collect::<Vec<_>>();
            self.items.append(
                &mut nice_items
                    .choose_multiple(&mut thread_rng(), self.max_items)
                    .map(|(index, _)| Item { index: *index })
                    .collect(),
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
        if self.items[item_num].get_info().sells {
            return buyer.stats.gold >= self.get_item_cost(item_num);
        }
        false
    }
    fn get_item_cost(&self, item_num: usize) -> u64 {
        self.items[item_num].get_info().cost
    }
}
#[derive(Clone, Debug, PartialEq)]
#[alkahest(Deserialize, Serialize, SerializeRef, Formula)]
pub struct RecruitUnit {
    pub unit: usize,
    pub count: usize,
}
impl Ini for RecruitUnit {
    fn eat(chars: std::str::Chars) -> Result<(Self, std::str::Chars), IniParseError> {
        let (unit, chars) = usize::eat(chars)?;
        let (count, chars) = usize::eat(chars)?;
        Ok((Self { unit, count }, chars))
    }
    fn vomit(&self) -> String {
        [self.unit.vomit(), self.count.vomit()].join(",")
    }
}
impl RecruitUnit {
    fn new(unit: usize, count: usize) -> Self {
        Self { unit, count }
    }
}
#[derive(Clone, Debug, PartialEq, Sections)]
#[alkahest(Deserialize, Serialize, SerializeRef, Formula)]
pub struct Recruitment {
    pub units: Vec<RecruitUnit>,
    pub cost_modify: f64,
}
impl Recruitment {
    pub fn new(units: Vec<RecruitUnit>, cost_modify: f64) -> Self {
        Self { units, cost_modify }
    }
    pub fn buy(&mut self, buyer: &mut Army, unit_num: usize, units: &Vec<Unit>) -> Result<(), ()> {
        if self.can_buy(buyer, unit_num, units) {
            buyer.add_troop(
                Troop {
                    unit: units[self.units[unit_num].unit].clone(),
                    ..Troop::empty()
                }
                .into(),
            )?;
            self.units[unit_num].count -= 1;
            buyer.stats.gold -= units[self.units[unit_num].unit].info.cost_hire;
        }
        Err(())
    }
    pub fn can_buy(&self, buyer: &Army, unit_num: usize, units: &Vec<Unit>) -> bool {
        let info = &self
            .units
            .get(unit_num)
            .expect("Trying to get unit at unknown index");
        buyer.stats.gold >= units[info.unit].info.cost_hire && info.count > 0
        //* (RECRUIT_COST * self.cost_modify)) as u64;
    }
}
