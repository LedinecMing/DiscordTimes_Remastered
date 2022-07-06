use crate::lib::units::unit::Power;
use crate::Unit;



pub trait Bonus
{
    fn on_attacked(&self, damage: Power, receiver: &dyn Unit, sender: &dyn Unit) -> Power
    {
        damage
    }
    fn on_attacking(&self, damage: Power, receiver: &dyn Unit, sender: &dyn Unit) -> Power
    {
        damage
    }
    fn on_tick(&self, unit: &dyn Unit) -> bool
    {
        false
    }
    fn on_hour(&self, time: i64) -> bool
    {
        false
    }
}
