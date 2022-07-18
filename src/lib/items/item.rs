use crate::lib::bonuses::bonus::Bonus;
use crate::lib::effects::effect::{Effect, EffectInfo};
use crate::lib::effects::effects::ItemEffect;
use crate::lib::units::unit::{Power, UnitStats};
use crate::Unit;

#[derive(Clone, Debug)]
pub struct ItemInfo
{

}
#[derive(Clone, Debug)]
pub struct Item
{
    pub bonus: Option<Box<dyn Bonus>>,
    pub effect: Box<dyn Effect>
}
