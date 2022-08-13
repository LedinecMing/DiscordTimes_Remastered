#![allow(non_snake_case)]

use
{
    dyn_clone::clone_box,
    crate::lib::
    {
        bonuses::bonus::Bonus,
        units::unit::{Defence, Power, Unit, UnitData, UnitInfo, UnitInventory, UnitStats},
        bonuses::bonuses::*,
        effects::effect::Effect,
        battle::battlefield::BattleField,
        items::item::Item
    },
};

#[derive(Clone, Debug)]
pub struct Ranged
{
    data: UnitData
}

impl Unit for Ranged
{
    fn attack(&mut self, target: &mut dyn Unit, battle: &mut BattleField) -> bool
    {
        let stats = self.get_effected_stats();
        println!("Атакую цель {:?}", stats.damage);
        target.being_attacked(&(stats.damage), self) > 0
    }


    fn get_mut_data(&mut self) -> &mut UnitData { &mut self.data }

    fn get_data(&self) -> &UnitData { &self.data }

    fn get_bonus(&self) -> Box<dyn Bonus> {
        clone_box(&*self.data.bonus)
    }

    fn being_attacked(&mut self, damage: &Power, sender: &mut dyn Unit) -> i32
    {
        println!("Кто-то атакует {:?}", damage);
        let corrected_damage = self.correct_damage(damage);
        let unit_bonus = dbg!(sender.get_bonus().clone());
        let corrected_damage = unit_bonus.on_attacking(corrected_damage, self, sender);
        let corrected_damage = self.get_bonus().on_attacked(corrected_damage, self, sender);
        let corrected_damage_units = dbg!(corrected_damage.magic + corrected_damage.ranged + corrected_damage.hand);
        println!("Было {:?}", self.get_data().stats.hp);
        self.get_mut_data().stats.hp -= corrected_damage_units;
        println!("Стало {:?}", self.get_data().stats.hp);
        corrected_damage_units
    }

    fn tick(&mut self) -> bool
    {
        let mut effect: Box<dyn Effect>;
        for effect_num in 0..self.data.effects.len()
        {
            effect = self.data.effects[effect_num].clone();
            effect.tick(self);
            self.data.effects[effect_num].on_tick();
            if effect.is_dead()
            {
                self.data.effects.remove(effect_num);
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
            data: UnitData
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
    }
    pub fn Hunter() -> Self
    {
        Self
        {
            data: UnitData
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
}

#[derive(Clone, Debug)]
pub struct Hand
{
    data: UnitData
}
impl Unit for Hand
{
    fn attack(&mut self, target: &mut dyn Unit, battle: &mut BattleField) -> bool
    {
        let stats = self.get_effected_stats();
        println!("Атакую цель {:?}", stats.damage);
        target.being_attacked(&(stats.damage), self) > 0
    }
    fn get_mut_data(&mut self) -> &mut UnitData
    {
        &mut self.data
    }
    fn get_data(&self) -> &UnitData
    {
        &self.data
    }
    fn get_bonus(&self) -> Box<dyn Bonus> {
        clone_box(&*self.data.bonus)
    }

    fn being_attacked(&mut self, damage: &Power, sender: &mut dyn Unit) -> i32
    {
        println!("Кто-то атакует {:?}", damage);
        let corrected_damage = self.correct_damage(damage);
        let unit_bonus = dbg!(sender.get_bonus().clone());
        let corrected_damage = unit_bonus.on_attacking(corrected_damage, self, sender);
        let corrected_damage = self.get_bonus().on_attacked(corrected_damage, self, sender);
        let corrected_damage_units = dbg!(corrected_damage.magic + corrected_damage.ranged + corrected_damage.hand);
        println!("Было {:?}", self.get_data().stats.hp);
        self.get_mut_data().stats.hp -= corrected_damage_units;
        println!("Стало {:?}", self.get_data().stats.hp);
        corrected_damage_units
    }
    fn tick(&mut self) -> bool
    {
        let mut effect: Box<dyn Effect>;
        for effect_num in 0..self.data.effects.len()
        {
            effect = self.data.effects[effect_num].clone();
            effect.tick(self);
            self.data.effects[effect_num].on_tick();
            if effect.is_dead()
            {
                self.data.effects.remove(effect_num);
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
            data: UnitData
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
}