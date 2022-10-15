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
use crate::lib::units::unit::MagicType;


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
    pub fn new(data: UnitData) -> Self {
        Self { data }
}   }

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
    pub fn new(data: UnitData) -> Self {
        Self { data }
}   }


#[derive(Clone, Debug)]
pub struct HealMage {
    data: UnitData
}
impl Unit for HealMage {
    fn attack(&mut self, target: &mut dyn Unit) -> bool {
        let target_name = target.get_info().name.clone();
        let stats = self.get_effected_stats();
        if target.get_unittype() == UnitType::Undead && self.get_magictype() == MagicType::Life ||
            [UnitType::Alive, UnitType::Rogue].contains(&target.get_unittype()) && self.get_magictype() == MagicType::Death
            { return false }

        if target.heal(stats.damage.magic) {
            if !target.has_effect_kind(EffectKind::MageSupport) {
                target.add_effect(Box::new(HealMagic {
                    info: EffectInfo { lifetime: 2 }, magic_power: stats.damage.magic }));
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
        let corrected_damage = self.correct_damage(damage);
        let unit_bonus = sender.get_bonus().clone();
        let corrected_damage = unit_bonus.on_attacking(corrected_damage, self, sender);
        let corrected_damage = self.get_bonus().on_attacked(corrected_damage, self, sender);
        let corrected_damage_units = corrected_damage.magic + corrected_damage.ranged + corrected_damage.hand;
        self.get_mut_data().stats.hp -= corrected_damage_units;
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
    pub fn new(data: UnitData) -> Self { Self { data } }
}

#[derive(Clone, Debug)]
pub struct AttackMage {
    data: UnitData
}
impl Unit for AttackMage {
    fn attack(&mut self, target: &mut dyn Unit) -> bool {
        let stats = self.get_effected_stats();
        let mut magic_power_mult: u64 = 1;
        if target.get_unittype() == UnitType::Undead && self.get_magictype() == MagicType::Life {
             magic_power_mult = 2;
        }
        let mut damage = stats.damage;
        damage.magic *= magic_power_mult;
        if !target.has_effect_kind(EffectKind::MageCurse) {
            target.add_effect(Box::new(AttackMagic { info: EffectInfo { lifetime: 1 },
                magic_power: target.correct_damage(&damage).magic}));
            return false;
        }
        let value = target.being_attacked(&(damage), self);
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
        let corrected_damage = self.correct_damage(damage);
        let unit_bonus = sender.get_bonus().clone();
        let corrected_damage = unit_bonus.on_attacking(corrected_damage, self, sender);
        let corrected_damage = self.get_bonus().on_attacked(corrected_damage, self, sender);
        let corrected_damage_units = corrected_damage.magic + corrected_damage.ranged + corrected_damage.hand;
        self.get_mut_data().stats.hp -= corrected_damage_units;
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
        let corrected_damage = self.correct_damage(damage);
        let unit_bonus = sender.get_bonus().clone();
        let corrected_damage = unit_bonus.on_attacking(corrected_damage, self, sender);
        let corrected_damage = self.get_bonus().on_attacked(corrected_damage, self, sender);
        let corrected_damage_units = corrected_damage.magic + corrected_damage.ranged + corrected_damage.hand;
        self.get_mut_data().stats.hp -= corrected_damage_units;
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
    pub fn new(data: UnitData) -> Self { Self { data } }
}
