use {
    crate::lib::{
        bonuses::bonus::Bonus,
        effects::{
            effect::{EffectInfo, EffectKind},
            effects::*
        },
        units::unit::{Defence, Power, UnitStats, Unit1}
    },
    math_thingies::Percent
};

#[derive(Copy, Clone, Debug)]
pub struct DefencePiercing {}

impl Bonus for DefencePiercing {
    fn on_attacking(&self, damage: Power, receiver: &mut Unit1, sender: &mut Unit1) -> Power {
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
    fn on_attacked(&self, damage: Power, receiver: &mut Unit1, sender: &mut Unit1) -> Power {
        let percent_75 = Percent::new(75);
        Power {
            magic: percent_75.calc(damage.magic),
            ranged: percent_75.calc(damage.ranged),
            hand: percent_75.calc(damage.hand)
}   }   }

#[derive(Copy, Clone, Debug)]
pub struct FastGoing {}
impl Bonus for FastGoing {
    fn on_battle_start(&self, unit: &mut Unit1) -> bool {
        unit.add_effect(Box::new(MoreMoves::default()));
        true
}   }

#[derive(Copy, Clone, Debug)]
pub struct Berserk {}
impl Bonus for Berserk {
    fn on_kill(&self, receiver: &mut Unit1, sender: &mut Unit1) -> bool {
        let receiver_stats = receiver.stats;
        let percent_10 = Percent::new(10);
        sender.add_effect(
            Box::new(ToEndEffect {
                info: EffectInfo { lifetime: i32::MAX },
                additions: UnitStats {
                    damage: Power {
                        hand: percent_10.calc(receiver_stats.damage.hand),
                        ranged: percent_10.calc(receiver_stats.damage.ranged),
                        magic: percent_10.calc(receiver_stats.damage.magic),
                    },
                    ..UnitStats::empty()
            }   }));
        true
}   }

#[derive(Copy, Clone, Debug)]
pub struct Block {}
impl Bonus for Block {
    fn on_move_skip(&self, unit: &mut Unit1) -> bool {
        println!("Бонус: персонаж пропустил ход, увеличиваю защиту в 2 раза");
        unit.add_effect(
            Box::new(ItemEffect {
                info: EffectInfo { lifetime: 1 },
                additions: UnitStats {
                    defence: Defence {
                        ranged_percent: Percent::new(0),
                        hand_percent: Percent::new(0),
                        death_magic: Percent::new(0),
                        life_magic: Percent::new(0),
                        elemental_magic: Percent::new(0),
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
    fn on_attacking(&self, damage: Power, receiver: &mut Unit1, sender: &mut Unit1) -> Power {
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
    fn on_attacking(&self, damage: Power, receiver: &mut Unit1, sender: &mut Unit1) -> Power {
        let receiver_stats = receiver.get_effected_stats();
        if !receiver.has_effect_kind(EffectKind::Fire) {
            if !(receiver_stats.defence.hand_percent.get() >= 99 && receiver.get_effected_stats().defence.ranged_percent.get() >= 99) {
                receiver.add_effect(Box::new(Fire::default()));
        }   }
        damage
}   }

#[derive(Copy, Clone, Debug)]
pub struct NoBonus {}
impl Bonus for NoBonus {}

pub fn match_bonus(bonus: &str) -> Result<Box<dyn Bonus>, ()> {
    Ok(match bonus {
        "Berserk" => Box::new(Berserk {}) as Box<dyn Bonus>,
        "Fire" => Box::new(FireAttack {}),
        "Poison" => Box::new(PoisonAttack {}),
        "Block" => Box::new(Block {}),
        "FastGoing" | "HorseAtack" => Box::new(FastGoing {}),
        "DefencePiercing" | "ArmorIgnore" => Box::new(DefencePiercing {}),
        "Dodging" | "Evasive" => Box::new(Dodging {}),
        "NoBonus" | "" | _ => Box::new( NoBonus {}),
        _ => return Err(())
    })
}
