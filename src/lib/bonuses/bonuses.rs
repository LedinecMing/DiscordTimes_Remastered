use {
    crate::lib::{
        bonuses::bonus::Bonus,
        effects::{
            effect::EffectInfo,
            effects::*
        },
        units::unit::{Defence, Power, UnitStats, Unit}
    }
};

#[derive(Copy, Clone, Debug)]
pub struct DefencePiercing {}

impl Bonus for DefencePiercing {
    fn on_attacking(&self, damage: Power, receiver: &mut dyn Unit, sender: &mut dyn Unit) -> Power {
        let sender_damage: Power = sender.get_effected_stats().damage;
        println!("Бонус: Атакую, ручной и дальний урон проходит сквозь броню - {:?}", Power {
            magic: damage.magic,
            ranged: sender_damage.ranged,
            hand: sender_damage.hand
        });
        Power {
            magic: damage.magic,
            ranged: sender_damage.ranged,
            hand: sender_damage.hand
        }
    }
}

#[derive(Copy, Clone, Debug)]
pub struct Dodging {}
impl Bonus for Dodging {
    fn on_attacked(&self, damage: Power, receiver: &mut dyn Unit, sender: &mut dyn Unit) -> Power {
        println!("Бонус: кто-то атакует, пропускаю 75%");
        Power {
            magic: damage.magic / 4 * 3,
            ranged: damage.ranged / 4 * 3,
            hand: damage.hand / 4 * 3
        }
    }
}

#[derive(Copy, Clone, Debug)]
pub struct FastGoing {}
impl Bonus for FastGoing {
    fn on_battle_start(&self, unit: &mut dyn Unit) -> bool {
        unit.add_effect(Box::new(MoreMoves::default()));
        true
    }
}

#[derive(Copy, Clone, Debug)]
pub struct Berserk {}
impl Bonus for Berserk {
    fn on_kill(&self, receiver: &mut dyn Unit, sender: &mut dyn Unit) -> bool {
        sender.add_effect(
            Box::new(ItemEffect {
                info: EffectInfo { lifetime: i32::MAX },
                additions: UnitStats {
                    damage: Power {
                        hand: 10,
                        ranged: 10,
                        magic: 10
                    },
                    ..UnitStats::empty()
                }
            }));
        true
    }
}

#[derive(Copy, Clone, Debug)]
pub struct Block {}
impl Bonus for Block {
    fn on_move_skip(&self, unit: &mut dyn Unit) -> bool {
        println!("Бонус: персонаж пропустил ход, увеличиваю защиту в 2 раза");
        unit.add_effect(
            Box::new(ItemEffect {
                info: EffectInfo { lifetime: 1 },
                additions: UnitStats {
                    defence: Defence {
                        ranged_percent: 0,
                        hand_percent: 0,
                        magic_percent: 0,
                        ..unit.get_data().stats.defence
                    },
                    ..UnitStats::empty()
                }
            }));
        true
    }
}

const POISON_PERCENT: u64 = 15;
#[derive(Copy, Clone, Debug)]
pub struct Poison {}
impl Bonus for Poison {
    fn on_tick(&self, unit: &mut dyn Unit) -> bool {
        let amount = unit.get_data().stats.hp / 100 * POISON_PERCENT;
        unit.get_mut_data().stats.hp -= amount;
        true
    }
}

#[derive(Copy, Clone, Debug)]
pub struct NoBonus {}
impl Bonus for NoBonus {}
