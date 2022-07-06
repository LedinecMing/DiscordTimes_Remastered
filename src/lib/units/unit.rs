use crate::Dodging;
use crate::lib::army::Army;
use crate::lib::battle::battlefield::BattleField;
use crate::lib::effects::effect::{Effect, MoreHealth};
use crate::lib::items::item::Item;
use crate::lib::bonuses::bonus::Bonus;


#[derive(Copy, Clone, Debug)]
pub struct Defence
{
    pub magic_percent: i32,
    pub hand_percent: i32,
    pub ranged_percent: i32,
    pub magic_units: i32,
    pub hand_units: i32,
    pub ranged_units: i32
}
#[derive(Copy, Clone, Debug)]
pub struct Power
{
    pub magic: i32,
    pub ranged: i32,
    pub hand: i32
}

#[derive(Copy, Clone, Debug)]
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

#[derive(Copy, Clone, Debug)]
pub struct UnitInfo
{
}

pub trait Unit
{
    fn attack(&self, target: &mut dyn Unit, battle: &mut BattleField) -> bool;
    fn get_effected_stats(&self) -> UnitStats;
    fn get_bonus(&self) -> &Box<dyn Bonus>;
    fn being_attacked(&mut self, damage: &Power, sender: &dyn Unit) -> i32;
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
    fn tick(&self) -> bool;
}

pub struct Ranged
{
    pub stats: UnitStats,
    pub info: UnitInfo,
    pub bonus: Box<dyn Bonus>,
    pub effects: Vec<Box<dyn Effect>>
}

impl Unit for Ranged
{
    fn attack(&self, target: &mut dyn Unit, battle: &mut BattleField) -> bool
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
    fn get_bonus(&self) -> &Box<dyn Bonus> {
         &self.bonus
    }
    fn being_attacked(&mut self, damage: &Power, sender: &dyn Unit) -> i32
    {
        println!("Кто-то атакует {:?}", damage);
        let corrected_damage = self.bonus.on_attacked(sender.get_bonus().on_attacking(self.correct_damage(damage), self, sender), self, sender);
        let corrected_damage_units = corrected_damage.magic + corrected_damage.ranged + corrected_damage.hand;
        self.stats.hp -= corrected_damage_units;
        corrected_damage_units
    }
    fn tick(&self) -> bool
    {
        self.effects.iter().for_each(|effect|
            {
                effect.tick(self);
            });
        self.get_bonus().on_tick(self);
        true
    }
}
impl Ranged
{
    pub fn new() -> Self
    {
        Self
        {
            stats: UnitStats {hp: 30, max_hp:30, damage: Power {
                magic: 0,
                ranged: 10,
                hand: 0
            },
                defence: Defence {
                    magic_percent: 0,
                    hand_percent: 0,
                    ranged_percent: 0,
                    magic_units: 0,
                    hand_units: 5,
                    ranged_units: 10
                },
                moves: 1, max_moves: 1, speed: 10},
            effects: vec![Box::new(MoreHealth {..Default::default()})],
            bonus: Box::new(Dodging {}),
            info: UnitInfo {  }
        }
    }
}
