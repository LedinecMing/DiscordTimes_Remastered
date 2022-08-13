#![allow(non_snake_case)]

use
{
    crate::lib::
    {
        bonuses::bonus::Bonus,
        effects::
        {
            effect::EffectInfo,
            effects::ItemEffect,
        },
        items::item::{Item, ItemInfo},
        units::unit::{Defence, Power, UnitStats, Unit}
    }
};


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
            info: ItemInfo {
                name: "Cool Sword".to_string(),
                description: "really cool".to_string(),
                cost: 100,
                sells: true
            },
            ..Item::empty()
        }
    }
    fn empty() -> Self
    {
        Self
        {
            bonus: None,
            effect: Box::new(ItemEffect { info: EffectInfo { lifetime: -1 }, additions: UnitStats::empty() }),
            info: ItemInfo {
                name: "".to_string(),
                description: "".to_string(),
                cost: 0,
                sells: false
            }
        }
    }
}
