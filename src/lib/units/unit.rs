use {
    std::fmt::Debug,
    dyn_clone::DynClone,
    crate::lib::{
        effects::effect::{Effect, EffectKind},
        items::item::Item,
        bonuses::bonus::Bonus
    },
    derive_more::{Add, Sub, BitOr},
    math::Percent,
};
use crate::lib::math;
use crate::lib::units::unit::MagicType::NoMagic;

#[derive(Copy, Clone, Debug, Add, Sub)]
pub struct Defence {
    pub magic_percent: Percent,
    pub hand_percent: Percent,
    pub ranged_percent: Percent,
    pub magic_units: u64,
    pub hand_units: u64,
    pub ranged_units: u64
}
impl Defence {
    pub fn empty() -> Self {
        Self {
            magic_percent: Percent::new(0),
            hand_percent: Percent::new(0),
            ranged_percent: Percent::new(0),
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
    pub vamp: Percent,
    pub regen: Percent
}
impl UnitStats {
    pub fn empty() -> Self {
        Self {
            hp: 0,
            max_hp: 0,
            damage: Power::empty(),
            defence: Defence::empty(),
            moves: 0,
            max_moves: 0,
            speed: 0,
            vamp: Percent::new(0),
            regen: Percent::new(0)
}   }   }
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum MagicType {
    Life,
    Death,
    Elemental,
    NoMagic
}
#[derive(Clone, Debug)]
pub struct LevelUpInfo {
    pub stats: UnitStats,
    pub xp_up: Percent,
    pub max_xp: u64,
}
impl LevelUpInfo {
    pub fn empty() -> Self {
        Self {
            stats: UnitStats::empty(),
            xp_up: Percent::new(0),
            max_xp: 0
}   }   }
#[derive(Clone, Debug)]
pub struct UnitInfo {
    pub name: String,
    pub descript: String,
    pub cost: u64,
    pub unit_type: UnitType,
    pub magic_type: MagicType,
    pub surrender: Option<u64>,
    pub lvl: LevelUpInfo
}
impl UnitInfo {
    pub fn empty() -> Self {
        Self {
            name: "".into(),
            descript: "".into(),
            cost: 0,
            unit_type: UnitType::Unidentified,
            magic_type: NoMagic,
            surrender: None,
            lvl: LevelUpInfo::empty()
}   }   }
#[derive(Clone, Debug)]
pub struct UnitInventory {
    pub items: Vec<Option<Item>>
}
impl UnitInventory {
    pub fn empty() -> Self {
        Self {
            items: vec![]
}   }   }
#[derive(Clone, Debug)]
pub struct UnitLvl {
    pub lvl: u64,
    pub max_xp: u64,
    pub xp: u64
}
impl UnitLvl {
    pub fn empty() -> Self {
        Self {
            lvl: 0,
            max_xp: 0,
            xp: 0
}   }   }
#[derive(Clone, Debug)]
pub struct UnitData {
    pub stats: UnitStats,
    pub info: UnitInfo,
    pub lvl: UnitLvl,
    pub inventory: UnitInventory,
    pub bonus: Box<dyn Bonus>,
    pub effects: Vec<Box<dyn Effect>>
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum UnitType {
    Alive,
    Undead,
    Rogue,
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
            if let Some(item) = item {
                previous = item.effect.update_stats(previous);
        }   });
        previous
    }
    fn get_mut_data(&mut self) -> &mut UnitData;
    fn get_data(&self) -> &UnitData;
    fn get_info(&self) -> &UnitInfo { &self.get_data().info }
    fn get_bonus(&self) -> Box<dyn Bonus>;
    fn get_unittype(&self) -> UnitType { self.get_info().unit_type }
    fn get_magictype(&self) -> MagicType { self.get_info().magic_type }
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
        self.get_mut_data().inventory.items.push(Some(item));
        true
    }
    fn being_attacked(&mut self, damage: &Power, sender: &mut dyn Unit) -> u64;
    fn correct_damage(&self, damage: &Power) -> Power {
        let defence: Defence = self.get_effected_stats().defence;
        println!("Использую защиту {:?}", defence);
        let percent_100 = Percent::new(100);
        Power {
            ranged: (percent_100 - defence.ranged_percent).calc(damage.ranged.saturating_sub(defence.ranged_units)),
            magic: (percent_100 - defence.magic_percent).calc(damage.magic.saturating_sub(defence.magic_units)),
            hand: (percent_100 - defence.hand_percent).calc(damage.hand.saturating_sub(defence.hand_units))
    }   }
    fn tick(&mut self) -> bool;
}
