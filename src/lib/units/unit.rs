use dyn_clone::{clone_box, DynClone};
use crate::{DefencePiercing, Dodging};
use crate::lib::army::Army;
use crate::lib::battle::battlefield::BattleField;
use crate::lib::effects::effect::Effect;
use crate::lib::effects::effects::MoreHealth;
use crate::lib::items::item::Item;
use crate::lib::bonuses::bonus::Bonus;
use derive_more::Add;

#[derive(Copy, Clone, Debug, Add)]
pub struct Defence
{
    pub magic_percent: i32,
    pub hand_percent: i32,
    pub ranged_percent: i32,
    pub magic_units: i32,
    pub hand_units: i32,
    pub ranged_units: i32
}
#[derive(Copy, Clone, Debug, Add)]
pub struct Power
{
    pub magic: i32,
    pub ranged: i32,
    pub hand: i32
}

#[derive(Copy, Clone, Debug, Add)]
pub struct UnitStats
{
    pub hp: i32,
    pub max_hp: i32,
    pub damage: Power,
    pub defence: Defence,
    pub moves: i32,
    pub max_moves: i32,
    pub speed: i32,
}
impl UnitStats
{
    pub(crate) fn empty() -> Self
    {
        Self
        {
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
            speed: 0
        }
    }
}
#[derive(Clone, Debug)]
pub struct UnitInfo
{
    pub name: String
}
pub struct UnitInventory
{
    pub items: Vec<Box<dyn Item>>
}

dyn_clone::clone_trait_object!(Unit);
pub trait Unit : DynClone
{
    fn attack(&mut self, target: &mut dyn Unit, battle: &mut BattleField) -> bool;
    fn get_effected_stats(&self) -> UnitStats;
    fn get_bonus(&self) -> Box<dyn Bonus>;
    fn add_effect(&mut self, effect: Box<dyn Effect>) -> bool;
    fn being_attacked(&mut self, damage: &Power, sender: &mut dyn Unit) -> i32;
    fn correct_damage(&self, damage: &Power) -> Power
    {
        let defence: Defence = self.get_effected_stats().defence;
        println!("Использую защиту {:?}", defence);
        Power {
            ranged: (damage.ranged-defence.ranged_units) / 100*(100-defence.ranged_percent),
            magic: (damage.magic-defence.magic_units) / 100*(100-defence.magic_percent),
            hand: (damage.ranged-defence.hand_units) / 100*(100-defence.hand_percent)
        }
    }
    fn tick(&mut self) -> bool;
}

#[derive(Clone)]
pub struct Ranged
{
    pub stats: UnitStats,
    pub info: UnitInfo,
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
        previous
    }
    fn get_bonus(&self) -> Box<dyn Bonus> {
         clone_box(&*self.bonus)
    }

    fn add_effect(&mut self, effect: Box<dyn Effect>) -> bool
    {
        self.effects.push(effect);
        true
    }

    fn being_attacked(&mut self, damage: &Power, sender: &mut dyn Unit) -> i32
    {
        println!("Кто-то атакует {:?}", damage);
        let corrected_damage = self.correct_damage(damage);
        let unit_bonus = &sender.get_bonus().clone();
        println!("{:?}", unit_bonus);
        let corrected_damage = unit_bonus.on_attacking(corrected_damage, self, sender);
        let corrected_damage = unit_bonus.on_attacked(corrected_damage, self, sender);
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
                    magic: 0,
                    ranged: 40,
                    hand: 0
                },
                defence: Defence
                {
                    magic_percent: 0,
                    hand_percent: 0,
                    ranged_percent: 0,
                    magic_units: 0,
                    hand_units: 10,
                    ranged_units: 10
                },
                moves: 2,
                max_moves: 2,
                speed: 10
            },
            effects: vec![],
            bonus: Box::new(DefencePiercing {}),
            info: UnitInfo { name: "Снайпер".into() }
        }
    }
    pub fn Hunter() -> Self
    {
        Self 
        {
            stats: UnitStats
            {
                hp: 65,
                max_hp: 65,
                damage: Power
                {
                    magic: 0,
                    ranged: 40,
                    hand: 0
                },
                defence: Defence
                {
                    magic_percent: 0,
                    hand_percent: 0,
                    ranged_percent: 0,
                    magic_units: 0,
                    hand_units: 10,
                    ranged_units: 10
                },
                moves: 2,
                max_moves: 2,
                speed: 10
            },
            effects: vec![],
            bonus: Box::new(Dodging {}),
            info: UnitInfo { name: "Охотник".into() }
        }
    }
}
