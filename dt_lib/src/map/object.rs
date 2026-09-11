use std::collections::HashMap;

use crate::{
    battle::{army::Army, control::Relations, troop::Troop}, items::Item, registry::{GameInfo, Items, UnitId, Units}, units::unit::{Unit, UnitPos}
};
use advini::*;
use alkahest::alkahest;
use num_enum::FromPrimitive;
use rand::{seq::SliceRandom, thread_rng};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ObjectType {
    Building {
		/// DTm info
		group: u8,
		variant: u8
	},
    Bridge {
		group: u8,
		variant: u8
	},
    MapDeco {
		/// DTm id
		id: usize
	},
}

#[derive(Clone, Debug, PartialEq)]
pub struct ObjectInfo {
    pub name: String,
    pub path: String,
    pub category: String,
    pub obj_type: ObjectType,
    pub index: usize,
    pub size: (u8, u8),
}

#[derive(Clone, Debug, PartialEq)]
#[alkahest(Deserialize, Serialize, SerializeRef, Formula)]
pub struct Village {
    pub max_gold: u64,
    pub max_mana: u64,
}
impl Ini<'_> for Village {
	type Arg = ();
	fn eat<'a>(input: &'a str, _additional: Self::Arg) -> Result<(&'a str, Self), IniParseError> {
        let (input, max_gold) = u64::eat(input, _additional)?;
        let (input, max_mana) = u64::eat(input, _additional)?;
        Ok((input, Self { max_gold, max_mana }))
    }
    fn vomit(&self, _additional: Self::Arg) -> String {
        [self.max_gold.vomit(_additional), self.max_mana.vomit(_additional)].join(",")
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
    Port,
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
	
    #[default_value = "Vec::<usize>::new()"]
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

    #[default_value = "Vec::<usize>::new()"]
    pub garrison: Vec<UnitId>,
    #[default_value = "0u64"]
    pub additional_defense: u64,

    #[default_value = "0u64"]
    pub gold_income: u64,
    #[default_value = "0u64"]
    pub mana_income: u64,

    #[default_value = "Vec::<usize>::new()"]
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
    pub fn new(itemcost_range: (u64, u64), items: Vec<Item>, max_items: usize) -> Self {
        Self {
            itemcost_range,
            items,
            max_items,
        }
    }
    pub fn update(&mut self, items: &Items) {
        for _ in self.max_items - self.items.len()..0 {
            let nice_items = items
                .inner
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
    pub fn buy(&mut self, buyer: &mut Army, item_num: usize, registry: &Items) {
        if self.can_buy(buyer, item_num, registry) {
            buyer.stats.gold = buyer
                .stats
                .gold
                .saturating_sub(self.get_item_cost(item_num, registry));
            buyer.add_item(self.items.remove(item_num));
        }
    }
    pub fn can_buy(&self, buyer: &Army, item_num: usize, registry: &Items) -> bool {
        if self.items[item_num].get_info(&registry).sells {
            return buyer.stats.gold >= self.get_item_cost(item_num, registry);
        }
        false
    }
    pub fn get_item_cost(&self, item_num: usize, registry: &Items) -> u64 {
        self.items[item_num].get_info(&registry).cost
    }

    /// Продажа артефакта из инвентаря армии на рынок: 50% стоимости.
    /// Ok(выручка) при успехе; Err если предмета нет в инвентаре.
    pub fn sell(&mut self, seller: &mut Army, inventory_pos: usize, registry: &Items) -> Result<u64, ()> {
        let Some(item) = seller.inventory.get(inventory_pos).copied().flatten() else {
            return Err(());
        };
        let revenue = item.get_info(registry).cost / 2;
        seller.inventory[inventory_pos] = None;
        seller.stats.gold = seller.stats.gold.saturating_add(revenue);
        // Рынок принимает предмет, если есть место (Иначе предмет просто исчезает).
        if self.items.len() < self.max_items {
            self.items.push(item);
        }
        Ok(revenue)
    }
}
#[derive(Clone, Debug, PartialEq)]
#[alkahest(Deserialize, Serialize, SerializeRef, Formula)]
pub struct RecruitUnit {
    pub unit: usize,
    pub count: usize,
}
impl Ini<'_> for RecruitUnit {
	type Arg = ();
    fn eat<'a>(input: &'a str, _additional: Self::Arg) -> Result<(&'a str, Self), IniParseError> {
        let (input, unit) = usize::eat(input, _additional)?;
        let (input, count) = usize::eat(input, _additional)?;
        Ok((input, Self { unit, count }))
    }
	fn vomit(&self, additional: Self::Arg) -> String {
        [self.unit.vomit(additional), self.count.vomit(additional)].join(",")
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
    /// Найм юнита в армию: списывает золото, уменьшает доступный счётчик.
    /// Ok(новая позиция в армии) при успехе.
    pub fn buy(&mut self, buyer: &mut Army, unit_num: usize, registry: &GameInfo) -> Result<usize, ()> {
        if !self.can_buy(buyer, unit_num, &registry.units) {
            return Err(());
        }
        let info = registry.units[self.units[unit_num].unit].clone();
        buyer.stats.gold = buyer.stats.gold.saturating_sub(info.cost_hire);
        self.units[unit_num].count -= 1;
        let troop = Troop {
            unit: (info, &registry.bonuses).into(),
            was_payed: true,
            is_free: false,
            is_main: false,
            pos: UnitPos::from_index(0, 6),
            custom_name: None,
        };
        buyer.add_troop(troop.into(), &registry.units)?;
        Ok(buyer.troops.len() - 1)
    }
    pub fn can_buy(&self, buyer: &Army, unit_num: usize, registry: &Units) -> bool {
		let recruit = &self.units[unit_num];
        let info = &registry[recruit.unit];
        buyer.stats.gold >= info.cost_hire && recruit.count > 0
        //* (RECRUIT_COST * self.cost_modify)) as u64;
    }
}

// ------------------- Услуги строения (лечение/воскрешение) -------------------

/// Цена лечения юнита: 1 золото за 1 HP недостающего.
pub fn heal_cost(troop: &Troop) -> u64 {
    (troop.unit.modified.max_hp - troop.unit.hp).max(0) as u64
}
/// Лечение за указанное золото (до конца, если хватает). Возвращает
/// фактическую потраченную сумму. Мёртвых не лечит (это resurrect).
pub fn heal_for_gold(troop: &mut Troop, gold: u64, _registry: &GameInfo) -> Result<u64, ()> {
    if troop.is_dead() || gold == 0 {
        return Err(());
    }
    let missing = (troop.unit.modified.max_hp - troop.unit.hp).max(0) as u64;
    if missing == 0 {
        return Err(());
    }
    let spent = gold.min(missing);
    troop.unit.heal(spent as i64);
    Ok(spent)
}

/// Воскрешение мёртвого юнита армии за cost_hire из реестра.
/// Юнит возвращается с полным здоровьем. `gold` — доступные деньги армии,
/// возвращается потраченная сумма. Troop берётся отдельно от Army: troop
/// живёт в `army.troops`, caller снимает guard и сам списывает золото
/// (см. resurrect: Caller subtracts `spent` from army gold).
pub fn resurrect_cost(troop: &Troop, registry: &GameInfo) -> u64 {
    registry.units[troop.unit.id].cost_hire
}

pub fn resurrect(troop: &mut Troop, gold: u64, registry: &GameInfo) -> Result<u64, ()> {
    if !troop.is_dead() {
        return Err(());
    }
    let cost = resurrect_cost(troop, registry);
    if gold < cost {
        return Err(());
    }
    troop.unit.restore();
    Ok(cost)
}

