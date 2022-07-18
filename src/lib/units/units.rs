#![allow(non_snake_case)]

use dyn_clone::clone_box;
use crate::lib::bonuses::bonus::Bonus;
use crate::lib::units::unit::{Defence, Power, UnitInfo, UnitInventory, UnitStats};
use crate::lib::bonuses::bonuses::*;
use crate::lib::effects::effect::Effect;
use crate::{BattleField, Item, Unit};

#[derive(Clone, Debug)]
pub struct Ranged
{
    pub stats: UnitStats,
    pub info: UnitInfo,
    pub inventory: UnitInventory,
    pub bonus: Box<dyn Bonus>,
    pub effects: Vec<Box<dyn Effect>>
}

impl Unit for Ranged
{
    fn attack(&mut self, target: &mut dyn Unit, battle: &mut BattleField) -> bool
    {
        let stats = self.get_effected_stats();
        println!("Атакую цель {:?}", stats.damage);
        target.being_attacked(&(stats.damage), self) > 0
    }
    fn get_effected_stats(&self) -> UnitStats
    {
        let mut previous: UnitStats = self.stats.clone();
        self.effects.iter().for_each(|effect|
            {
                previous = effect.update_stats(previous);
            });
        self.inventory.items.iter().for_each(|item|
            {
                previous = item.effect.update_stats(previous);
            });
        previous
    }
    fn get_info(&self) -> UnitInfo
    {
        self.info.clone()
    }
    fn get_bonus(&self) -> Box<dyn Bonus> {
        clone_box(&*self.bonus)
    }

    fn add_effect(&mut self, effect: Box<dyn Effect>) -> bool
    {
        self.effects.push(effect);
        true
    }

    fn add_item(&mut self, item: Item) -> bool
    {
        self.inventory.items.push(item);
        true
    }

    fn being_attacked(&mut self, damage: &Power, sender: &mut dyn Unit) -> i32
    {
        println!("Кто-то атакует {:?}", damage);
        let corrected_damage = self.correct_damage(damage);
        let unit_bonus = &sender.get_bonus().clone();
        let corrected_damage = unit_bonus.on_attacking(corrected_damage, self, sender);
        let corrected_damage = self.get_bonus().on_attacked(corrected_damage, self, sender);
        let corrected_damage_units = corrected_damage.magic + corrected_damage.ranged + corrected_damage.hand;
        self.stats.hp -= corrected_damage_units;
        corrected_damage_units
    }

    fn tick(&mut self) -> bool
    {
        let mut effect: Box<dyn Effect>;
        for effect_num in 0..self.effects.len()
        {
            effect = self.effects[effect_num].clone();
            effect.tick(self);
            self.effects[effect_num].on_tick();
            if effect.is_dead()
            {
                self.effects.remove(effect_num);
            }
        }
        self.get_bonus().on_tick(self);
        true
    }
}
impl Ranged
{
    pub fn Sniper() -> Self
    {
        Self
        {
            stats: UnitStats
            {
                hp: 65,
                max_hp: 65,
                damage: Power
                {
                    ranged: 40,
                    ..Power::empty()
                },
                defence: Defence
                {
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
            info: UnitInfo { name: "Снайпер".into(), cost: 100, ..UnitInfo::empty() },
            inventory: UnitInventory { items: vec![] }
        }
    }
    pub fn Hunter() -> Self
    {
        Self
        {
            stats: UnitStats
            {
                hp: 50,
                max_hp: 50,
                damage: Power
                {
                    ranged: 20,
                    ..Power::empty()
                },
                defence: Defence
                {
                    ..Defence::empty()
                },
                moves: 2,
                max_moves: 2,
                speed: 16,
                ..UnitStats::empty()
            },
            effects: vec![],
            bonus: Box::new(NoBonus {}),
            info: UnitInfo { name: "Охотник".into(), cost: 22, ..UnitInfo::empty() },

            inventory: UnitInventory { items: vec![] }
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) struct Hand
{
    pub stats: UnitStats,
    pub info: UnitInfo,
    pub bonus: Box<dyn Bonus>,
    pub effects: Vec<Box<dyn Effect>>,
    pub inventory: UnitInventory,
}
impl Unit for Hand
{
    fn attack(&mut self, target: &mut dyn Unit, battle: &mut BattleField) -> bool
    {
        let stats = self.get_effected_stats();
        println!("Атакую цель {:?}", stats.damage);
        target.being_attacked(&(stats.damage), self) > 0
    }
    fn get_effected_stats(&self) -> UnitStats
    {
        let mut previous: UnitStats = self.stats.clone();
        self.effects.iter().for_each(|effect|
            {
                previous = effect.update_stats(previous);
            });
        self.inventory.items.iter().for_each(|item|
            {
                previous = item.effect.update_stats(previous);
            });
        previous
    }
    fn get_info(&self) -> UnitInfo
    {
        self.info.clone()
    }
    fn get_bonus(&self) -> Box<dyn Bonus> {
        clone_box(&*self.bonus)
    }

    fn add_effect(&mut self, effect: Box<dyn Effect>) -> bool
    {
        self.effects.push(effect);
        true
    }

    fn add_item(&mut self, item: Item) -> bool
    {
        self.inventory.items.push(item);
        true
    }

    fn being_attacked(&mut self, damage: &Power, sender: &mut dyn Unit) -> i32
    {
        println!("Кто-то атакует {:?}", damage);
        let corrected_damage = self.correct_damage(damage);
        let unit_bonus = &sender.get_bonus().clone();
        println!("{:?}", unit_bonus);
        let corrected_damage = unit_bonus.on_attacking(corrected_damage, self, sender);
        let corrected_damage = self.get_bonus().on_attacked(corrected_damage, self, sender);
        let corrected_damage_units = corrected_damage.magic + corrected_damage.ranged + corrected_damage.hand;
        self.stats.hp -= corrected_damage_units;
        println!("{}", corrected_damage_units);
        corrected_damage_units
    }

    fn tick(&mut self) -> bool
    {
        let mut effect: Box<dyn Effect>;
        for effect_num in 0..self.effects.len()
        {
            effect = self.effects[effect_num].clone();
            effect.tick(self);
            self.effects[effect_num].on_tick();
            if effect.is_dead()
            {
                self.effects.remove(effect_num);
            }
        }
        self.get_bonus().on_tick(self);
        true
    }
}

impl Hand
{
    pub fn Recruit () -> Self
    {
        Self
        {
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
                name: "".to_string(),
                cost: 0,
                ..UnitInfo::empty()
            },
            bonus: Box::new(NoBonus {}),
            effects: vec![],
            inventory: UnitInventory { items: vec![] }
        }
    }
}