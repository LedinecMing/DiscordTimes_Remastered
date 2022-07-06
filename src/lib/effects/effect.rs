use crate::lib::units::unit::UnitStats as UnitStats;
use crate::Unit;


pub trait Effect
{
    fn update_stats(&self, unitstats: UnitStats) -> UnitStats;
    fn tick(&self, unit: &dyn Unit) -> Option<i8>;
}


pub struct EffectInfo
{
    pub lifetime: i32
}


pub struct MoreHealth
{
    pub info: EffectInfo
}
impl Default for MoreHealth {
    fn default() -> Self {
        Self {
            info: EffectInfo { lifetime: -1 }
        }
    }
}
impl Effect for MoreHealth
{
    fn update_stats(&self, mut unitstats: UnitStats) -> UnitStats
    {
        unitstats.hp += 10;
        unitstats.max_hp += 10;
        unitstats
    }
    fn tick(&self, unit: &dyn Unit) -> Option<i8> {None}
}

pub struct MoreHandAttack
{
    pub info: EffectInfo
}
impl Default for MoreHandAttack {
    fn default() -> Self {
        Self {
            info: EffectInfo { lifetime: -1 }
        }
    }
}
impl Effect for MoreHandAttack
{
    fn update_stats(&self, mut unitstats: UnitStats) -> UnitStats
    {
        unitstats.damage.hand += 5;
        unitstats
    }
    fn tick(&self, unit: &dyn Unit) -> Option<i8> { None }
}