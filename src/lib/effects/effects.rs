use crate::lib::effects::effect::{Effect, EffectInfo};
use crate::lib::units::unit::UnitStats;
use crate::Unit;

#[derive(Copy, Clone)]
pub struct MoreHealth
{
    pub info: EffectInfo
}
impl Default for MoreHealth
{
    fn default() -> Self
    {
        Self
        {
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
}

#[derive(Copy, Clone)]
pub struct MoreHandAttack
{
    pub info: EffectInfo
}
impl Default for MoreHandAttack
{
    fn default() -> Self
    {
        Self
        {
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
    fn tick(&mut self, unit: &mut dyn Unit) -> bool { false }
}

#[derive(Copy, Clone)]
pub struct MoreMoves
{
    pub info: EffectInfo
}
impl Default for MoreMoves
{
    fn default() -> Self
    {
        Self
        {
            info: EffectInfo {lifetime: 2}
        }
    }
}
impl Effect for MoreMoves
{
    fn update_stats(&self, mut unitstats: UnitStats) -> UnitStats
    {
        unitstats.max_moves+=1;
        unitstats
    }
    fn on_tick(&mut self) -> bool
    {
        self.info.lifetime -= 1;
        true
    }
    fn is_dead(&self) -> bool {
        self.info.lifetime < 1
    }
}

#[derive(Copy, Clone)]
pub struct ItemEffect
{
    pub info: EffectInfo,
    pub additions: UnitStats
}
impl Effect for ItemEffect
{
    fn update_stats(&self, unitstats: UnitStats) -> UnitStats
    {
        let mut new_stats = unitstats.clone();
        new_stats
    }
}