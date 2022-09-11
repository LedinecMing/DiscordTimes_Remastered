
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
use crate::lib::effects::effect::EffectKind;
use crate::Time;

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
}   }

#[derive(Copy, Clone, Debug)]
pub struct Berserk {}
impl Bonus for Berserk {
    fn on_kill(&self, receiver: &mut dyn Unit, sender: &mut dyn Unit) -> bool {
        let receiver_stats = receiver.get_data().stats;
        sender.add_effect(
            Box::new(ItemEffect {
                info: EffectInfo { lifetime: i32::MAX },
                additions: UnitStats {
                    damage: Power {
                        hand: receiver_stats.damage.hand / 100 * 10,
                        ranged: receiver_stats.damage.ranged / 100 * 10,
                        magic: receiver_stats.damage.magic / 100 * 10,
                    },
                    ..UnitStats::empty()
            }   }));
        true
}   }

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
                        ..unit.get_effected_stats().defence
                    },
                    ..UnitStats::empty()
                }
            }));
        true
}   }

#[derive(Copy, Clone, Debug)]
pub struct PoisonAttack {}
impl Bonus for PoisonAttack {
    fn on_attacking(&self, damage: Power, receiver: &mut dyn Unit, sender: &mut dyn Unit) -> Power {
        let receiver_stats = receiver.get_effected_stats();
        if !receiver.has_effect_kind(EffectKind::Poison) {
            if !(receiver_stats.defence.hand_percent > 99 && receiver.get_effected_stats().defence.ranged_percent > 99) {
                receiver.add_effect(Box::new(Poison::default()));
        }   }
        damage
}   }

#[derive(Copy, Clone, Debug)]
pub struct FireAttack {}
impl Bonus for FireAttack {
    fn on_attacking(&self, damage: Power, receiver: &mut dyn Unit, sender: &mut dyn Unit) -> Power {
        let receiver_stats = receiver.get_effected_stats();
        if !receiver.has_effect_kind(EffectKind::Fire) {
            if !(receiver_stats.defence.hand_percent > 99 && receiver.get_effected_stats().defence.ranged_percent > 99) {
                receiver.add_effect(Box::new(Fire::default()));
            }   }
        damage
    }   }

#[derive(Copy, Clone, Debug)]
pub struct NoBonus {}
impl Bonus for NoBonus {}
