#![allow(non_snake_case)]

use crate::lib::bonuses::bonus::Bonus;
use crate::lib::effects::effect::{Effect, EffectInfo};
use crate::lib::effects::effects::ItemEffect;
use crate::lib::items::item::Item;
use crate::lib::units::unit::{Defence, Power, UnitStats};
use crate::Unit;

impl Item
{
    pub fn CoolSword() -> Self
    {
        Self
        {
            effect: Box::new(ItemEffect { info: EffectInfo { lifetime: -1 }, additions: UnitStats {
                damage: Power {
                    magic: 0,
                    ranged: 0,
                    hand: 20,
                },
                max_moves: 1,
                speed: 10,
                ..UnitStats::empty()
            } }),
            ..Item::empty()
        }
    }
    fn empty() -> Self
    {
        Self
        {
            bonus: None,
            effect: Box::new(ItemEffect { info: EffectInfo { lifetime: -1 }, additions: UnitStats::empty() })
        }
    }
}
