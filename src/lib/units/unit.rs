use {
    std::{
        fmt::Debug,
        cmp::Ordering
    },
    dyn_clone::DynClone,
    derive_more::{AddAssign, DivAssign, MulAssign, SubAssign},
    num::{
        integer::sqrt,
        pow
    },
    crate::lib::{
        effects::{
            effect::{Effect, EffectKind, EffectInfo},
            effects::{AttackMagic, HealMagic, DisableMagic, ElementalSupport}
        },
        items::item::Item,
        bonuses::bonus::Bonus,
        battle::troop::Troop,
        units::unit::{
            MagicType::*,
            MagicDirection::*
    }   },
    derive_more::{Add, Sub, Mul, Div, Neg},
    math_thingies::Percent,
};
#[derive(Copy, Clone, Debug, Add, Sub)]
pub struct Defence {
    pub death_magic: Percent,
    pub elemental_magic: Percent,
    pub life_magic: Percent,
    pub hand_percent: Percent,
    pub ranged_percent: Percent,
    pub magic_units: u64,
    pub hand_units: u64,
    pub ranged_units: u64
}
impl Defence {
    pub fn empty() -> Self {
        Self {
            death_magic: Percent::new(0),
            elemental_magic: Percent::new(0),
            life_magic: Percent::new(0),
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
    pub hp: i64,
    pub max_hp: i64,
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
pub enum MagicDirection {
    ToAlly,
    ToAll,
    ToEnemy,
    CurseOnly,
    StrikeOnly,
    BlessOnly,
    CureOnly
}
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum MagicType {
    Life(MagicDirection),
    Death(MagicDirection),
    Elemental(MagicDirection),
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
    pub magic_type: Option<MagicType>,
    pub surrender: Option<u64>,
    pub lvl: LevelUpInfo
}
impl UnitInfo {
    pub fn empty() -> Self {
        Self {
            name: "".into(),
            descript: "".into(),
            cost: 0,
            unit_type: UnitType::Alive,
            magic_type: None,
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

#[derive(Copy, Clone, Add, AddAssign, Sub, SubAssign, Mul, MulAssign, Div, DivAssign)]
pub struct UnitPos(pub usize, pub usize);
impl Into<(usize, usize)> for UnitPos {
    fn into(self) -> (usize, usize) {
        (self.0, self.1)
}   }

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum UnitType {
    Alive,
    Undead,
    Rogue
}

dyn_clone::clone_trait_object!(Unit);
pub trait Unit : DynClone + Debug {
    fn attack(&mut self, target: &mut dyn Unit) -> bool;
    fn heal(&mut self, amount: u64) -> bool {
        if self.get_effected_stats().max_hp < self.get_effected_stats().hp + amount as i64 {
            self.get_mut_data().stats.hp = self.get_effected_stats().max_hp;
            return true;
        }
        self.get_mut_data().stats.hp += amount as i64;
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
    fn get_magictype(&self) -> Option<MagicType> { self.get_info().magic_type }
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
            magic: (percent_100 - percent_100).calc(damage.magic.saturating_sub(defence.magic_units)),
            hand: (percent_100 - defence.hand_percent).calc(damage.hand.saturating_sub(defence.hand_units))
    }   }
    fn tick(&mut self) -> bool;
}

#[derive(Clone, Debug)]
pub struct Unit1 {
    pub stats: UnitStats,
    pub info: UnitInfo,
    pub lvl: UnitLvl,
    pub inventory: UnitInventory,
    pub army: usize,
    pub bonus: Box<dyn Bonus>,
    pub effects: Vec<Box<dyn Effect>>
}

fn heal_unit(me: &mut Unit1, unit: &mut Unit1, damage: Power, magic_type: MagicType) -> bool {
    match (unit.info.unit_type, magic_type) {
        (UnitType::Rogue | UnitType::Alive, Death(_)) => {
            return false
        }
        (UnitType::Undead, Life(_)) => {
            return false
        }
        _ => {}
    }
    return unit.heal(damage.magic)
}
fn bless_unit(me: &mut Unit1, target: &mut Unit1, damage: Power) -> bool {
    if !target.has_effect_kind(EffectKind::MageSupport) {
        target.add_effect(Box::new(HealMagic {
            info: EffectInfo { lifetime: 1 },
            magic_power: damage.magic
        }));
        return true
    }
    false
}
fn heal_bless(me: &mut Unit1, target: &mut Unit1, damage: Power, magic_type: MagicType) -> bool {
    if heal_unit(me, target, damage, magic_type) {
        bless_unit(me, target, damage);
        return true
    }
    false
}

fn elemental_bless(me: &mut Unit1, target: &mut Unit1, damage: Power) -> bool {
    if !target.has_effect_kind(EffectKind::MageSupport) {
        target.add_effect(Box::new(ElementalSupport {
            info: EffectInfo { lifetime: 1 },
            magic_power: damage.magic
        }));
        return true
    };
    false
}

fn magic_curse(me: &mut Unit1, target: &mut Unit1, mut damage: Power, magic_type: MagicType) -> bool {
    match (target.info.unit_type, magic_type) {
        (UnitType::Undead, Life(_)) => {
            damage.magic *= 2;
        },
        _ => {}
    }
    if !target.has_effect_kind(EffectKind::MageCurse) {
        target.add_effect(Box::new(AttackMagic {
            info: EffectInfo { lifetime: 1 },
            magic_power: target.correct_damage(&damage, me.info.magic_type).magic
        }));
        true
    } else {
        false
}   }

fn elemental_curse(me: &mut Unit1, target: &mut Unit1, damage: Power) -> bool {
    if !target.has_effect_kind(EffectKind::MageCurse) {
        target.add_effect(Box::new(DisableMagic {
            info: EffectInfo { lifetime: 1 },
            magic_power: target.correct_damage(&damage, me.info.magic_type).magic
        }));
        true
    } else {
        false
}   }

fn magic_attack(me: &mut Unit1, target: &mut Unit1, mut damage: Power, magic_type: MagicType) -> bool {
    match (target.info.unit_type, magic_type) {
        (UnitType::Undead, Life(_)) => {
            damage.magic *= 2;
        },
        _ => {}
    }
    if !magic_curse(me, target, damage, magic_type) {
        target.being_attacked(&damage, me);
    }
    true
}

fn elemental_attack(me: &mut Unit1, target: &mut Unit1, damage: Power) -> bool {
    if !elemental_curse(me, target, damage) {
        target.being_attacked(&damage, me);
    }
    true
}

fn get_magic_direction(magic_type: MagicType) -> MagicDirection {
    match magic_type {
        Death(direction) => direction,
        Life(direction) => direction,
        Elemental(direction) => direction
}   }

impl Unit1 {
    pub fn attack(&mut self, target: &mut Unit1, target_pos: UnitPos, my_pos: UnitPos) -> bool {
        let effected = self.get_effected_stats();
        let distance_vec: UnitPos = my_pos - target_pos;
        let distance: usize = distance_vec.0.max(distance_vec.1);
        let is_enemy = self.army != target.army;
        let mut damage = effected.damage;

        if distance == 1 {
            damage.magic = 0;
            if damage.hand > 0 {
                damage.ranged = 0;
            } else if damage.ranged > 0 {
                damage.hand = 0;
            } else {
                return false
            }
        } else if distance < 5 {
            damage.hand = 0;
            if damage.magic > 0 {
                damage.ranged = 0;
            } else if damage.ranged > 0 {
                damage.magic = 0;
            } else {
                return false
            }
        } else {
            return false
        }

        return if damage.hand > 0 || damage.ranged > 0{
            target.being_attacked(&damage, self);
            true
        } else {
            match self.info.magic_type {
                None => {
                    false
                },
                Some(magic_type) => {
                    let direction = get_magic_direction(magic_type);
                    match (direction, magic_type, is_enemy) {
                        (ToAlly, _, false) => {
                            match magic_type {
                                Death(_) | Life(_) => heal_bless(self, target, damage, magic_type),
                                Elemental(_) => elemental_bless(self, target, damage)
                        }   },
                        (ToAll, _, _) => {
                            match (magic_type, is_enemy) {
                                (Death(_) | Life(_), true) => magic_attack(self, target, damage, magic_type),
                                (Death(_) | Life(_), false) => heal_bless(self, target, damage, magic_type),
                                (Elemental(_), true) => elemental_attack(self, target, damage),
                                (Elemental(_), false) => elemental_bless(self, target, damage)
                        }   },
                        (ToEnemy, _, true) => {
                            match magic_type {
                                Death(_) | Life(_) => magic_attack(self, target, damage, magic_type),
                                Elemental(_) => elemental_attack(self, target, damage)
                        }   }
                        (CurseOnly, _, true) => {
                            match magic_type {
                                Death(_) | Life(_) => magic_curse(self, target, damage, magic_type),
                                Elemental(_) => elemental_curse(self, target, damage)
                        }   }
                        (StrikeOnly, _, true) => {
                            target.being_attacked(&damage, self);
                            true
                        },
                        (BlessOnly, _, false) => {
                            match magic_type {
                                Life(_) | Death(_) => bless_unit(self, target, damage),
                                Elemental(_) => elemental_bless(self, target, damage)
                        }   }
                        (CureOnly, Life(_) | Death(_), false) => heal_unit(self, target, damage, magic_type),
                        _ => false
    }   }   }   }   }

    pub fn heal(&mut self, amount: u64) -> bool {
        let effected = self.get_effected_stats();
        let amount = amount as i64;
        if effected.max_hp < effected.hp + amount {
            self.stats.hp = effected.max_hp;
            return true;
        }
        self.stats.hp += amount;
        return false;
    }

    pub fn get_effected_stats(&self) -> UnitStats {
        let mut previous: UnitStats = self.stats;
        let effects = &self.effects;
        effects.iter().for_each(|effect| {
            previous = effect.update_stats(previous);
        });
        &self.inventory.items.iter().for_each(|item| {
            if let Some(item) = item {
                previous = item.effect.update_stats(previous);
        }   });
        previous
    }
    pub fn is_dead(&self) -> bool {
        self.get_effected_stats().hp < 1
    }
    pub fn has_effect_kind(&self, kind: EffectKind) -> bool {
        for effect in &self.effects {
            if effect.get_kind() == kind {
               return true;
            };
        }
        return false;
    }
    pub fn kill(&mut self) { self.stats.hp -= self.get_effected_stats().hp - self.stats.hp;}
    pub fn add_effect(&mut self, effect: Box<dyn Effect>) -> bool {
        self.effects.push(effect);
        true
    }
    pub fn add_item(&mut self, item: Item) -> bool {
        self.inventory.items.push(Some(item));
        true
    }
    pub fn being_attacked(&mut self, damage: &Power, sender: &mut Unit1) -> u64 {
        let corrected_damage = self.correct_damage(damage, sender.info.magic_type);
        let unit_bonus = sender.bonus.clone();
        let corrected_damage = unit_bonus.on_attacking(corrected_damage, self, sender);
        let corrected_damage = self.bonus.clone().on_attacked(corrected_damage, self, sender);
        let corrected_damage_units = corrected_damage.magic + corrected_damage.ranged + corrected_damage.hand;
        self.stats.hp -= corrected_damage_units as i64;
        corrected_damage_units
    }
    pub fn correct_damage(&self, damage: &Power, magic_type: Option<MagicType>) -> Power {
        let defence: Defence = self.get_effected_stats().defence;
        let percent_100 = Percent::new(100);
        let mut magic: u64 = 0;

        if let Some(magic_type) = magic_type {
            let magic_def = match magic_type {
                Life(_) => defence.life_magic,
                Death(_) => defence.death_magic,
                Elemental(_) => defence.elemental_magic
            };
            magic = (percent_100 - magic_def).calc(damage.magic.saturating_sub(defence.magic_units));
        }

        Power {
            ranged: (percent_100 - defence.ranged_percent).calc(damage.ranged.saturating_sub(defence.ranged_units)),
            magic,
            hand: (percent_100 - defence.hand_percent).calc(damage.hand.saturating_sub(defence.hand_units))
    }   }
    pub fn tick(&mut self) -> bool {
        let mut effect: Box<dyn Effect>;
        for effect_num in 0..self.effects.len() {
            effect = self.effects[effect_num].clone();
            effect.tick(self);
            self.effects[effect_num].on_tick();
            if effect.is_dead() {
                self.effects.remove(effect_num);
            }   }
        self.bonus.clone().on_tick(self);
        true
    }
}