use std::fmt::Debug;
use dyn_clone::{clone_box, DynClone};
use crate::{DefencePiercing, Dodging};
use crate::lib::battle::army::Army;
use crate::lib::battle::battlefield::BattleField;
use crate::lib::effects::effect::Effect;
use crate::lib::effects::effects::MoreHealth;
use crate::lib::items::item::Item;
use crate::lib::bonuses::bonus::Bonus;
use derive_more::Add;

fn nat(a: i32) -> i32
{
    if a >= 0
    {
        return a;
    }
    0
}

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
impl Defence
{
    pub fn empty() -> Self
    {
        Self
        {
            magic_percent: 0,
            hand_percent: 0,
            ranged_percent: 0,
            magic_units: 0,
            hand_units: 0,
            ranged_units: 0
        }
    }
}
#[derive(Copy, Clone, Debug, Add)]
pub struct Power
{
    pub magic: i32,
    pub ranged: i32,
    pub hand: i32
}
impl Power
{
    pub fn empty() -> Self
    {
        Self
        {
            magic: 0,
            ranged: 0,
            hand: 0
        }
    }

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
    pub fn empty() -> Self
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
    pub name: String,
    pub cost: i32
}
impl UnitInfo
{
    pub fn empty() -> Self
    {
        Self
        {
            name: "".to_string(),
            cost: 0
        }
    }
}
#[derive(Clone, Debug)]
pub struct UnitInventory
{
    pub items: Vec<Item>
}

dyn_clone::clone_trait_object!(Unit);
pub trait Unit : DynClone + Debug
{
    fn attack(&mut self, target: &mut dyn Unit, battle: &mut BattleField) -> bool;
    fn get_effected_stats(&self) -> UnitStats;
    fn get_info(&self) -> UnitInfo;
    fn get_bonus(&self) -> Box<dyn Bonus>;
    fn add_effect(&mut self, effect: Box<dyn Effect>) -> bool;
    fn add_item(&mut self, item: Item) -> bool;
    fn being_attacked(&mut self, damage: &Power, sender: &mut dyn Unit) -> i32;
    fn correct_damage(&self, damage: &Power) -> Power
    {
        let defence: Defence = self.get_effected_stats().defence;
        println!("Использую защиту {:?}", defence);
        let new_pow = Power {
            ranged: (nat(damage.ranged - defence.ranged_units) as f32 * (1.0 - defence.ranged_percent as f32 / 100.0)) as i32,
            magic: (nat(damage.magic-defence.magic_units) as f32 * (1.0 - defence.magic_percent as f32 / 100.0)) as i32,
            hand: (nat(damage.hand-defence.hand_units) as f32 * (1.0 - defence.hand_percent as f32 / 100.0)) as i32
        };
        new_pow
    }
    fn tick(&mut self) -> bool;
}
