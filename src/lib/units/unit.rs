use {
    std::fmt::Debug,
    dyn_clone::DynClone,
    crate::lib::{
        effects::effect::{Effect, EffectKind},
        items::item::Item,
        bonuses::bonus::Bonus
    },
    derive_more::{Add, Sub}
};

#[derive(Copy, Clone, Debug, Add, Sub)]
pub struct Defence {
    pub magic_percent: i32,
    pub hand_percent: i32,
    pub ranged_percent: i32,
    pub magic_units: u64,
    pub hand_units: u64,
    pub ranged_units: u64
}
impl Defence {
    pub fn empty() -> Self {
        Self {
            magic_percent: 0,
            hand_percent: 0,
            ranged_percent: 0,
            magic_units: 0,
            hand_units: 0,
            ranged_units: 0
}   }   }
#[derive(Copy, Clone, Debug, Add, Sub)]
pub struct Power {
    pub magic: u64,
    pub ranged: u64,
    pub hand: u64
}
impl Power {
    pub fn empty() -> Self {
        Self {
            magic: 0,
            ranged: 0,
            hand: 0
}   }   }

#[derive(Copy, Clone, Debug, Add, Sub)]
pub struct UnitStats {
    pub hp: u64,
    pub max_hp: u64,
    pub damage: Power,
    pub defence: Defence,
    pub moves: u64,
    pub max_moves: u64,
    pub speed: u64,
    pub vamp: u32,
    pub regen: u32
}
impl UnitStats {
    pub fn empty() -> Self {
        Self {
            hp: 0,
            max_hp: 0,
            damage: Power {
                magic: 0,
                ranged: 0,
                hand: 0
            },
            defence: Defence {
                magic_percent: 0,
                hand_percent: 0,
                ranged_percent: 0,
                magic_units: 0,
                hand_units: 0,
                ranged_units: 0
            },
            moves: 0,
            max_moves: 0,
            speed: 0,
            vamp: 0,
            regen: 0
}   }   }
#[derive(Clone, Debug)]
pub struct UnitInfo {
    pub name: String,
    pub cost: u64,
    pub unit_type: UnitType
}
impl UnitInfo {
    pub fn empty() -> Self {
        Self {
            name: "".into(),
            cost: 0,
            unit_type: UnitType::Unidentified
}   }   }
#[derive(Clone, Debug)]
pub struct UnitInventory {
    pub items: Vec<Item>
}
impl UnitInventory {
    pub fn empty() -> Self {
        Self {
            items: vec![]
}   }   }

#[derive(Clone, Debug)]
pub struct UnitData {
    pub stats: UnitStats,
    pub info: UnitInfo,
    pub inventory: UnitInventory,
    pub bonus: Box<dyn Bonus>,
    pub effects: Vec<Box<dyn Effect>>
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum UnitType {
    Alive,
    Undead,
    Unidentified
}

dyn_clone::clone_trait_object!(Unit);
pub trait Unit : DynClone + Debug {
    fn attack(&mut self, target: &mut dyn Unit) -> bool;
    fn heal(&mut self, amount: u64) -> bool {
        if self.get_effected_stats().max_hp < self.get_effected_stats().hp + amount {
            self.get_mut_data().stats.hp = self.get_effected_stats().max_hp;
            return true;
        }
        self.get_mut_data().stats.hp += amount;
        return false;
    }
    fn get_effected_stats(&self) -> UnitStats {
        let mut previous: UnitStats = self.get_data().stats;
        let effects = &self.get_data().effects;
        effects.iter().for_each(|effect| {
                previous = effect.update_stats(previous);
            });
        let inventory = &self.get_data().inventory;
        inventory.items.iter().for_each(|item| {
                previous = item.effect.update_stats(previous);
            });
        previous
    }
    fn get_mut_data(&mut self) -> &mut UnitData;
    fn get_data(&self) -> &UnitData;
    fn get_info(&self) -> &UnitInfo { &self.get_data().info }
    fn get_bonus(&self) -> Box<dyn Bonus>;
    fn get_unittype(&self) -> UnitType {
        self.get_info().unit_type
    }
    fn is_dead(&self) -> bool { self.get_effected_stats().hp < 1 }
    fn has_effect_kind(&self, kind: EffectKind) -> bool {
        self.get_data().effects.iter().map(|effect| effect.get_kind()).collect::<Vec<EffectKind>>().contains(&kind)
    }
    fn kill(&mut self) { self.get_mut_data().stats.hp = 0;}
    fn add_effect(&mut self, effect: Box<dyn Effect>) -> bool {
        self.get_mut_data().effects.push(effect);
        true
    }
    fn add_item(&mut self, item: Item) -> bool {
        self.get_mut_data().inventory.items.push(item);
        true
    }
    fn being_attacked(&mut self, damage: &Power, sender: &mut dyn Unit) -> u64;
    fn correct_damage(&self, damage: &Power) -> Power {
        let defence: Defence = self.get_effected_stats().defence;
        println!("Использую защиту {:?}", defence);
        Power {
            ranged: (damage.ranged.saturating_sub(defence.ranged_units) as f32 * (1.0 - defence.ranged_percent as f32 / 100.0)) as u64,
            magic: (damage.magic.saturating_sub(defence.magic_units) as f32 * (1.0 - defence.magic_percent as f32 / 100.0)) as u64,
            hand: (damage.hand.saturating_sub(defence.hand_units) as f32 * (1.0 - defence.hand_percent as f32 / 100.0)) as u64
    }   }
    fn tick(&mut self) -> bool;
}
