#![allow(non_snake_case)]

use {
    dyn_clone::clone_box,
    crate::lib::{
        bonuses::bonus::Bonus,
        units::unit::{Defence, Power, Unit, UnitData, UnitInfo, UnitInventory, UnitStats, UnitType},
        bonuses::bonuses::*,
        effects::{
            effect::{Effect, EffectInfo, EffectKind},
            effects::{DisableMagic, HealMagic, AttackMagic}
}   }   };


#[derive(Clone, Debug)]
pub struct Ranged {
    data: UnitData
}

impl Unit for Ranged {
    fn attack(&mut self, target: &mut dyn Unit) -> bool {
        let stats = self.get_effected_stats();
        println!("Атакую цель {:?}", stats.damage);
        let value = target.being_attacked(&(stats.damage), self);
        if target.is_dead() { self.get_bonus().on_kill(target, self); }
        value > 0
    }

    fn get_mut_data(&mut self) -> &mut UnitData { &mut self.data }

    fn get_data(&self) -> &UnitData { &self.data }

    fn get_bonus(&self) -> Box<dyn Bonus> {
        clone_box(&*self.data.bonus)
    }

    fn being_attacked(&mut self, damage: &Power, sender: &mut dyn Unit) -> u64 {
        let name = &self.data.info.name.clone();
        let sender_name = sender.get_info().name.clone();
        println!("{} атакует {} ({:?} силы)", sender_name, name, damage);
        let corrected_damage = self.correct_damage(damage);
        let unit_bonus = sender.get_bonus().clone();
        let corrected_damage = unit_bonus.on_attacking(corrected_damage, self, sender);
        let corrected_damage = self.get_bonus().on_attacked(corrected_damage, self, sender);
        let corrected_damage_units = corrected_damage.magic + corrected_damage.ranged + corrected_damage.hand;
        println!("У {} было {:?} жизней", name, self.get_data().stats.hp);
        self.get_mut_data().stats.hp -= corrected_damage_units;
        println!("Стало {:?}", self.get_data().stats.hp);
        corrected_damage_units
    }

    fn tick(&mut self) -> bool {
        let mut effect: Box<dyn Effect>;
        for effect_num in 0..self.data.effects.len() {
            effect = self.data.effects[effect_num].clone();
            effect.tick(self);
            self.data.effects[effect_num].on_tick();
            if effect.is_dead() {
                self.data.effects.remove(effect_num);
        }   }
        self.get_bonus().on_tick(self);
        true
}   }
impl Ranged {
    pub fn Sniper() -> Self {
        Self {
            data: UnitData {
                stats: UnitStats {
                    hp: 65,
                    max_hp: 65,
                    damage: Power {
                        ranged: 40,
                        ..Power::empty()
                    },
                    defence: Defence {
                        hand_units: 10,
                        ranged_units: 10,
                        ..Defence::empty()
                    },
                    moves: 2,
                    max_moves: 2,
                    speed: 10,
                    ..UnitStats::empty()
                },
                effects: vec![],
                bonus: Box::new(DefencePiercing {}),
                info: UnitInfo {
                    name: "Снайпер".into(),
                    cost: 100,
                    unit_type: UnitType::Alive,
                    ..UnitInfo::empty()
                },
                inventory: UnitInventory::empty()
            }
        }
    }
    pub fn Pathfinder() -> Self {
        Self {
            data: UnitData {
                stats: UnitStats {
                    hp: 75,
                    max_hp: 75,
                    damage: Power {
                        ranged: 45,
                        ..Power::empty()
                    },
                    defence: Defence {
                        hand_units: 5,
                        ranged_units: 5,
                        ..Defence::empty()
                    },
                    moves: 2,
                    max_moves: 2,
                    speed: 22,
                    ..UnitStats::empty()
                },
                effects: vec![],
                bonus: Box::new(Dodging {}),
                info: UnitInfo {
                    name: "Егерь".into(),
                    cost: 180,
                    unit_type: UnitType::Alive,
                    ..UnitInfo::empty()
                },
                inventory: UnitInventory::empty()
    }   }   }
    pub fn Hunter() -> Self {
        Self {
            data: UnitData {
                stats: UnitStats {
                    hp: 50,
                    max_hp: 50,
                    damage: Power {
                        ranged: 20,
                        ..Power::empty()
                    },
                    defence: Defence::empty(),
                    moves: 2,
                    max_moves: 2,
                    speed: 16,
                    ..UnitStats::empty()
                },
                effects: vec![],
                bonus: Box::new(NoBonus {}),
                info: UnitInfo {
                    name: "Охотник".into(),
                    cost: 22,
                    unit_type: UnitType::Alive,
                    ..UnitInfo::empty()
                },

                inventory: UnitInventory::empty()
}   }   }   }

#[derive(Clone, Debug)]
pub struct Hand {
    data: UnitData
}
impl Unit for Hand {
    fn attack(&mut self, target: &mut dyn Unit) -> bool {
        let stats = self.get_effected_stats();
        println!("Атакую цель {:?}", stats.damage);
        let value = target.being_attacked(&(stats.damage), self);
        if target.is_dead() { self.get_bonus().on_kill(target, self); }
        value > 0
    }
    fn get_mut_data(&mut self) -> &mut UnitData {
        &mut self.data
    }
    fn get_data(&self) -> &UnitData { &self.data }
    fn get_bonus(&self) -> Box<dyn Bonus> { clone_box(&*self.data.bonus) }

    fn being_attacked(&mut self, damage: &Power, sender: &mut dyn Unit) -> u64 {
        let name = self.data.info.name.clone();
        let sender_name = sender.get_info().name.clone();
        println!("{} атакует {} ({:?} силы)", sender_name, name, damage);
        let corrected_damage = self.correct_damage(damage);
        let unit_bonus = sender.get_bonus().clone();
        let corrected_damage = unit_bonus.on_attacking(corrected_damage, self, sender);
        let corrected_damage = self.get_bonus().on_attacked(corrected_damage, self, sender);
        let corrected_damage_units = corrected_damage.magic + corrected_damage.ranged + corrected_damage.hand;
        println!("У {} было {:?} жизней", name, self.get_effected_stats().hp);
        self.get_mut_data().stats.hp -= corrected_damage_units;
        println!("Стало {:?}", self.get_data().stats.hp);
        corrected_damage_units
    }
    fn tick(&mut self) -> bool {
        let mut effect: Box<dyn Effect>;
        for effect_num in 0..self.data.effects.len() {
            effect = self.data.effects[effect_num].clone();
            effect.tick(self);
            self.data.effects[effect_num].on_tick();
            if effect.is_dead() {
                self.data.effects.remove(effect_num);
        }   }
        self.get_bonus().on_tick(self);
        true
}   }
impl Hand {
    pub fn Recruit () -> Self {
        Self {
            data: UnitData {
                stats: UnitStats {
                    hp: 50,
                    max_hp: 50,
                    damage: Power {
                        hand: 25,
                        ..Power::empty()
                    },
                    defence: Defence {
                        ..Defence::empty()
                    },
                    moves: 1,
                    max_moves: 1,
                    speed: 1,
                    ..UnitStats::empty()
                },
                info: UnitInfo {
                    name: "Ополченец".into(),
                    cost: 0,
                    unit_type: UnitType::Alive,
                    ..UnitInfo::empty()
                },
                bonus: Box::new(NoBonus {}),
                effects: vec![],
                inventory: UnitInventory::empty(),
    }   }   }
    pub fn Knight () -> Self
    {
        Self {
            data: UnitData {
                stats: UnitStats {
                    hp: 110,
                    max_hp: 110,
                    damage: Power {
                        hand: 75,
                        ..Power::empty()
                    },
                    defence: Defence {
                        hand_units: 20,
                        ranged_units: 15,
                        magic_percent: 20,
                        ..Defence::empty()
                    },
                    moves: 1,
                    max_moves: 1,
                    speed: 13,
                    ..UnitStats::empty()
                },
                info: UnitInfo {
                    name: "Рыцарь".into(),
                    cost: 0,
                    unit_type: UnitType::Alive,
                    ..UnitInfo::empty()
                },
                bonus: Box::new(NoBonus {}),
                effects: vec![],
                inventory: UnitInventory::empty(),
}   }   }   }


#[derive(Clone, Debug)]
pub struct HealMage {
    data: UnitData
}
impl Unit for HealMage {
    fn attack(&mut self, target: &mut dyn Unit) -> bool {
        let name = &self.data.info.name;
        let target_name = target.get_info().name.clone();
        let stats = self.get_effected_stats();
        if target.get_unittype() == UnitType::Undead { return false }
        println!("У {} {:?} жизней", target.get_info().name, target.get_effected_stats().hp);
        println!("{} исцеляет цель {} ({:?} магии)", name, target.get_info().name, stats.damage.magic);
        if target.heal(stats.damage.magic) {
            if !target.has_effect_kind(EffectKind::MageSupport) {
                target.add_effect(Box::new(HealMagic { info: EffectInfo { lifetime: 2 }, magic_power: stats.damage.magic }));
        }   }
        println!("У {} {:?} жизней", target_name, target.get_effected_stats().hp);
        stats.damage.magic > 0
    }

    fn get_mut_data(&mut self) -> &mut UnitData {
        &mut self.data
    }
    fn get_data(&self) -> &UnitData {
        &self.data
    }
    fn get_bonus(&self) -> Box<dyn Bonus> {
        clone_box(&*self.data.bonus)
    }

    fn being_attacked(&mut self, damage: &Power, sender: &mut dyn Unit) -> u64 {
        let name = &self.data.info.name.clone();
        let sender_name = sender.get_info().name.clone();
        println!("{} атакует {} ({:?} силы)", sender_name, name, damage);
        let corrected_damage = self.correct_damage(damage);
        let unit_bonus = sender.get_bonus().clone();
        let corrected_damage = unit_bonus.on_attacking(corrected_damage, self, sender);
        let corrected_damage = self.get_bonus().on_attacked(corrected_damage, self, sender);
        let corrected_damage_units = corrected_damage.magic + corrected_damage.ranged + corrected_damage.hand;
        println!("У {} было {:?} жизней", sender_name, self.get_data().stats.hp);
        self.get_mut_data().stats.hp -= corrected_damage_units;
        println!("Стало {:?}", self.get_data().stats.hp);
        corrected_damage_units
    }

    fn tick(&mut self) -> bool {
        let mut effect: Box<dyn Effect>;
        for effect_num in 0..self.data.effects.len() {
            effect = self.data.effects[effect_num].clone();
            effect.tick(self);
            self.data.effects[effect_num].on_tick();
            if effect.is_dead() {
                self.data.effects.remove(effect_num);
        }   }
        self.get_bonus().on_tick(self);
        true
}   }
impl HealMage {
    pub fn Maidservant() -> Self {
        Self {
            data: UnitData {
                stats: UnitStats {
                    hp: 45,
                    max_hp: 45,
                    damage: Power {
                        magic: 15,
                        ..Power::empty()
                    },
                    defence: Defence {
                        magic_percent: 10,
                        ..Defence::empty()
                    },
                    moves: 2,
                    max_moves: 2,
                    speed: 20,
                    ..UnitStats::empty()
                },
                info: UnitInfo {
                    name: "Послушница".into(),
                    cost: 14,
                    unit_type: UnitType::Alive,
                    ..UnitInfo::empty()
                },
                inventory: UnitInventory::empty(),
                bonus: Box::new(NoBonus {}),
                effects: vec![]
    }   }   }
    pub fn Nun() -> Self {
        Self {
            data: UnitData {
                stats: UnitStats {
                    hp: 55,
                    max_hp: 55,
                    damage: Power {
                        magic: 25,
                        ..Power::empty()
                    },
                    defence: Defence {
                        magic_percent: 20,
                        ..Defence::empty()
                    },
                    moves: 2,
                    max_moves: 2,
                    speed: 24,
                    regen: 20,
                    ..UnitStats::empty()
                },
                info: UnitInfo {
                    name: "Монашка".into(),
                    cost: 14,
                    unit_type: UnitType::Alive,
                    ..UnitInfo::empty()
                },
                inventory: UnitInventory::empty(),
                bonus: Box::new(NoBonus {}),
                effects: vec![]
}   }   }   }

#[derive(Clone, Debug)]
pub struct AttackMage {
    data: UnitData
}
impl Unit for AttackMage {
    fn attack(&mut self, target: &mut dyn Unit) -> bool {
        let stats = self.get_effected_stats();
        println!("Атакую цель {:?}", stats.damage);
        if !target.has_effect_kind(EffectKind::MageCurse) {
            target.add_effect(Box::new(AttackMagic { info: EffectInfo { lifetime: 1 },
                magic_power: target.correct_damage(&self.data.stats.damage).magic}));
            return false;
        }
        let value = target.being_attacked(&(stats.damage), self);
        if target.is_dead() { self.get_bonus().on_kill(target, self); }
        value > 0
    }
    fn get_mut_data(&mut self) -> &mut UnitData {
        &mut self.data
    }
    fn get_data(&self) -> &UnitData {
        &self.data
    }
    fn get_bonus(&self) -> Box<dyn Bonus> {
        clone_box(&*self.data.bonus)
    }

    fn being_attacked(&mut self, damage: &Power, sender: &mut dyn Unit) -> u64 {
        let name = self.data.info.name.clone();
        let sender_name = sender.get_info().name.clone();
        println!("{} атакует {} ({:?} силы)", sender_name, name, damage);
        let corrected_damage = self.correct_damage(damage);
        let unit_bonus = sender.get_bonus().clone();
        let corrected_damage = unit_bonus.on_attacking(corrected_damage, self, sender);
        let corrected_damage = self.get_bonus().on_attacked(corrected_damage, self, sender);
        let corrected_damage_units = corrected_damage.magic + corrected_damage.ranged + corrected_damage.hand;
        println!("У {} было {:?} жизней", name, self.get_effected_stats().hp);
        self.get_mut_data().stats.hp -= corrected_damage_units;
        println!("Стало {:?}", self.get_data().stats.hp);
        corrected_damage_units
    }
    fn tick(&mut self) -> bool {
        let mut effect: Box<dyn Effect>;
        for effect_num in 0..self.data.effects.len() {
            effect = self.data.effects[effect_num].clone();
            effect.tick(self);
            self.data.effects[effect_num].on_tick();
            if effect.is_dead() {
                self.data.effects.remove(effect_num);
        }   }
        self.get_bonus().on_tick(self);
        true
}   }

#[derive(Clone, Debug)]
pub struct DisablerMage {
    data: UnitData
}
impl Unit for DisablerMage {
    fn attack(&mut self, target: &mut dyn Unit) -> bool {
        let stats = self.get_effected_stats();
        if !target.has_effect_kind(EffectKind::MageCurse) {
            target.add_effect(Box::new(DisableMagic { info: EffectInfo { lifetime: 1 },
                magic_power: target.correct_damage(&self.data.stats.damage).magic}));
            return false;
        }
        let value = target.being_attacked(&(stats.damage), self);
        if target.is_dead() { self.get_bonus().on_kill(target, self); }
        value > 0
    }
    fn get_mut_data(&mut self) -> &mut UnitData {
        &mut self.data
    }
    fn get_data(&self) -> &UnitData {
        &self.data
    }
    fn get_bonus(&self) -> Box<dyn Bonus> {
        clone_box(&*self.data.bonus)
    }

    fn being_attacked(&mut self, damage: &Power, sender: &mut dyn Unit) -> u64 {
        let name = self.data.info.name.clone();
        let sender_name = sender.get_info().name.clone();
        println!("{} атакует {} ({:?} силы)", sender_name, name, damage);
        let corrected_damage = self.correct_damage(damage);
        let unit_bonus = sender.get_bonus().clone();
        let corrected_damage = unit_bonus.on_attacking(corrected_damage, self, sender);
        let corrected_damage = self.get_bonus().on_attacked(corrected_damage, self, sender);
        let corrected_damage_units = corrected_damage.magic + corrected_damage.ranged + corrected_damage.hand;
        println!("У {} было {:?} жизней", name, self.get_effected_stats().hp);
        self.get_mut_data().stats.hp -= corrected_damage_units;
        println!("Стало {:?}", self.get_data().stats.hp);
        corrected_damage_units
    }
    fn tick(&mut self) -> bool {
        let mut effect: Box<dyn Effect>;
        for effect_num in 0..self.data.effects.len() {
            effect = self.data.effects[effect_num].clone();
            effect.tick(self);
            self.data.effects[effect_num].on_tick();
            if effect.is_dead() {
                self.data.effects.remove(effect_num);
        }   }
        self.get_bonus().on_tick(self);
        true
}   }
impl DisablerMage {
    pub fn Archimage() -> Self {
        Self {
            data: UnitData {
                stats: UnitStats {
                    hp: 80,
                    max_hp: 80,
                    damage: Power {
                        magic: 45,
                        ..Power::empty()
                    },
                    defence: Defence {
                        magic_percent: 60,
                        ..Defence::empty()
                    },
                    moves: 2,
                    max_moves: 2,
                    speed: 30,
                    regen: 10,
                    ..UnitStats::empty()
                },
                info: UnitInfo {
                    name: "Архимаг".to_string(),
                    cost: 220,
                    unit_type: UnitType::Alive,
                    ..UnitInfo::empty()
                },
                inventory: UnitInventory::empty(),
                bonus: Box::new(NoBonus {}),
                effects: vec![]
}   }   }   }
