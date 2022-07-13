use std::fmt::Debug;
use dyn_clone::DynClone;
use crate::lib::units::unit::Power;
use crate::Unit;


dyn_clone::clone_trait_object!(Bonus);
pub trait Bonus : DynClone + Debug
{
    fn on_attacked(&self, damage: Power, receiver: &mut dyn Unit, sender: &mut dyn Unit) -> Power
    {
        damage
    }
    fn on_attacking(&self, damage: Power, receiver: &mut dyn Unit, sender: &mut dyn Unit) -> Power
    {
        damage
    }
    fn on_tick(&self, unit: &mut dyn Unit) -> bool
    {
        false
    }
    fn on_hour(&self, unit: &mut dyn Unit, time: i64) -> bool
    {
        false
    }
    fn on_battle_start(&self, unit: &mut dyn Unit) -> bool
    {
        false
    }
}