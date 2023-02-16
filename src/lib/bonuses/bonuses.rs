use crate::{
    lib::{
        bonuses::bonus::Bonus,
        effects::{
            effect::{EffectInfo, EffectKind},
            effects::*,
        },
        units::{
            unit::{Defence, Power, Unit, UnitPos, UnitStats},
            unitstats::{Modify, ModifyDefence, *},
        },
    },
    LOCALE,
};
use math_thingies::Percent;
use std::cmp::min;

#[derive(Copy, Clone, Debug)]
pub struct DefencePiercing {}
impl Bonus for DefencePiercing {
    fn on_attacking(&self, damage: Power, receiver: &mut Unit, sender: &mut Unit) -> Power {
        let sender_damage: Power = sender.stats.damage;
        if sender_damage.magic > sender_damage.ranged && sender_damage.magic > sender_damage.hand {
            Power {
                magic: sender_damage.magic,
                ranged: 0,
                hand: 0,
            }
        } else if sender_damage.hand > sender_damage.ranged
            && sender_damage.hand > sender_damage.magic
        {
            Power {
                magic: 0,
                ranged: 0,
                hand: sender_damage.hand,
            }
        } else {
            Power {
                magic: 0,
                ranged: sender_damage.ranged,
                hand: 0,
            }
        }
    }

    fn id(&self) -> &'static str {
        "DefencePiercing"
    }
}

#[derive(Copy, Clone, Debug)]
pub struct DeadDodging {}
impl Bonus for DeadDodging {
    fn on_attacked(
        &self,
        damage: Power,
        receiver: &mut Unit,
        sender: &mut Unit,
        receiver_pos: UnitPos,
        sender_pos: UnitPos,
    ) -> Power {
        Power {
            ranged: damage.ranged - Percent::new(70),
            ..damage
        }
    }
    fn id(&self) -> &'static str {
        "DeadDodging"
    }
}

#[derive(Copy, Clone, Debug)]
pub struct FastDead {}
impl Bonus for FastDead {
    fn on_attacked(
        &self,
        damage: Power,
        receiver: &mut Unit,
        sender: &mut Unit,
        receiver_pos: UnitPos,
        sender_pos: UnitPos,
    ) -> Power {
        Power {
            ranged: damage.ranged - Percent::new(70),
            ..damage
        }
    }
    fn on_battle_start(&self, unit: &mut Unit) -> bool {
        unit.add_effect(Box::new(MoreMoves::default()));
        true
    }
    fn id(&self) -> &'static str {
        "FastDead"
    }
}

#[derive(Copy, Clone, Debug)]
pub struct Dodging {}
impl Bonus for Dodging {
    fn on_attacked(
        &self,
        damage: Power,
        receiver: &mut Unit,
        sender: &mut Unit,
        receiver_pos: UnitPos,
        sender_pos: UnitPos,
    ) -> Power {
        let percent_70 = Percent::new(70);
        Power {
            magic: percent_70.calc(damage.magic),
            ranged: percent_70.calc(damage.ranged),
            hand: percent_70.calc(damage.hand),
        }
    }
    fn id(&self) -> &'static str {
        "Dodging"
    }
}

#[derive(Copy, Clone, Debug)]
pub struct VampiresGist {}
impl Bonus for VampiresGist {
    fn on_attacked(
        &self,
        damage: Power,
        receiver: &mut Unit,
        sender: &mut Unit,
        receiver_pos: UnitPos,
        sender_pos: UnitPos,
    ) -> Power {
        let percent_70 = Percent::new(75);
        Power {
            magic: percent_70.calc(damage.magic),
            ranged: percent_70.calc(damage.ranged),
            hand: percent_70.calc(damage.hand),
        }
    }
    fn on_attacking(&self, damage: Power, receiver: &mut Unit, sender: &mut Unit) -> Power {
        let sender_damage: Power = sender.stats.damage;
        if sender_damage.magic > sender_damage.ranged && sender_damage.magic > sender_damage.hand {
            Power {
                magic: sender_damage.magic,
                ranged: 0,
                hand: 0,
            }
        } else if sender_damage.hand > sender_damage.ranged
            && sender_damage.hand > sender_damage.magic
        {
            Power {
                magic: 0,
                ranged: 0,
                hand: sender_damage.hand,
            }
        } else {
            Power {
                magic: 0,
                ranged: sender_damage.ranged,
                hand: 0,
            }
        }
    }
    fn id(&self) -> &'static str {
        "VampiresGist"
    }
}

#[derive(Copy, Clone, Debug)]
pub struct OldVampiresGist {}
impl Bonus for OldVampiresGist {
    fn on_attacked(
        &self,
        damage: Power,
        receiver: &mut Unit,
        sender: &mut Unit,
        receiver_pos: UnitPos,
        sender_pos: UnitPos,
    ) -> Power {
        let percent_70 = Percent::new(75);
        Power {
            magic: percent_70.calc(damage.magic),
            ranged: percent_70.calc(damage.ranged),
            hand: percent_70.calc(damage.hand),
        }
    }
    fn on_attacking(&self, damage: Power, receiver: &mut Unit, sender: &mut Unit) -> Power {
        let sender_damage: Power = sender.stats.damage;
        if sender_damage.magic > sender_damage.ranged && sender_damage.magic > sender_damage.hand {
            Power {
                magic: sender_damage.magic,
                ranged: 0,
                hand: 0,
            }
        } else if sender_damage.hand > sender_damage.ranged
            && sender_damage.hand > sender_damage.magic
        {
            Power {
                magic: 0,
                ranged: 0,
                hand: sender_damage.hand,
            }
        } else {
            Power {
                magic: 0,
                ranged: sender_damage.ranged,
                hand: 0,
            }
        }
    }
    fn on_battle_start(&self, unit: &mut Unit) -> bool {
        unit.add_effect(Box::new(MoreMoves::default()));
        true
    }
    fn id(&self) -> &'static str {
        "VampiresGist"
    }
}

#[derive(Copy, Clone, Debug)]
pub struct FastGoing {}
impl Bonus for FastGoing {
    fn on_battle_start(&self, unit: &mut Unit) -> bool {
        unit.add_effect(Box::new(MoreMoves::default()));
        true
    }
    fn id(&self) -> &'static str {
        "FastGoing"
    }
}

#[derive(Copy, Clone, Debug)]
pub struct Berserk {}
impl Bonus for Berserk {
    fn on_kill(&self, receiver: &mut Unit, sender: &mut Unit) -> bool {
        let receiver_stats = receiver.stats;
        let percent_10 = Percent::new(10);
        sender.add_effect(Box::new(ToEndEffect {
            info: EffectInfo { lifetime: i32::MAX },
            modify: ModifyUnitStats {
                damage: ModifyPower {
                    hand: *Modify::default().percent_add(percent_10),
                    ranged: *Modify::default().percent_add(percent_10),
                    magic: *Modify::default().percent_add(percent_10),
                },
                ..Default::default()
            },
        }));
        true
    }
    fn id(&self) -> &'static str {
        "Berserk"
    }
}
const PERCENT0: Percent = Percent::const_new(0);
#[derive(Copy, Clone, Debug)]
pub struct Block {}
impl Bonus for Block {
    fn on_move_skip(&self, unit: &mut Unit) -> bool {
        unit.add_effect(Box::new(ItemEffect {
            info: EffectInfo { lifetime: 1 },
            modify: ModifyUnitStats {
                defence: ModifyDefence {
                    hand_units: *Modify::default().percent_add(Percent::new(100)),
                    ranged_units: *Modify::default().percent_add(Percent::new(100)),
                    ..ModifyDefence::default()
                },
                ..ModifyUnitStats::default()
            },
        }));
        true
    }
    fn id(&self) -> &'static str {
        "Block"
    }
}

#[derive(Copy, Clone, Debug)]
pub struct PoisonAttack {}
impl Bonus for PoisonAttack {
    fn on_attacking(&self, damage: Power, receiver: &mut Unit, sender: &mut Unit) -> Power {
        if !receiver.has_effect_kind(EffectKind::Poison) {
            if damage.ranged > 1 || damage.hand > 1 {
                receiver.add_effect(Box::new(Poison::default()));
            }
        }
        damage
    }
    fn id(&self) -> &'static str {
        "Poison"
    }
}

#[derive(Copy, Clone, Debug)]
pub struct FireAttack {}
impl Bonus for FireAttack {
    fn on_attacking(&self, damage: Power, receiver: &mut Unit, sender: &mut Unit) -> Power {
        if !receiver.has_effect_kind(EffectKind::Fire) {
            if damage.ranged > 1 || damage.hand > 1 {
                receiver.add_effect(Box::new(Fire::default()));
            }
        }
        damage
    }
    fn id(&self) -> &'static str {
        "Fire"
    }
}

#[derive(Copy, Clone, Debug)]
pub struct Invulnerable {}
impl Bonus for Invulnerable {
    fn on_attacked(
        &self,
        damage: Power,
        receiver: &mut Unit,
        sender: &mut Unit,
        receiver_pos: UnitPos,
        sender_pos: UnitPos,
    ) -> Power {
        Power {
            hand: min(1, damage.hand),
            ranged: min(1, damage.ranged),
            magic: damage.magic,
        }
    }
    fn id(&self) -> &'static str {
        "Invulnerable"
    }
}

#[derive(Copy, Clone, Debug)]
pub struct GodAnger {}
impl Bonus for GodAnger {
    fn on_attacking(&self, damage: Power, receiver: &mut Unit, sender: &mut Unit) -> Power {
        damage
            + Power {
                hand: 10,
                ..Power::empty()
            }
    }
    fn id(&self) -> &'static str {
        "GodAnger"
    }
}

#[derive(Copy, Clone, Debug)]
pub struct GodStrike {}
impl Bonus for GodStrike {
    fn on_attacking(&self, damage: Power, receiver: &mut Unit, sender: &mut Unit) -> Power {
        damage
            + Power {
                hand: 20,
                ..Power::empty()
            }
    }
    fn id(&self) -> &'static str {
        "GodStrike"
    }
}

#[derive(Copy, Clone, Debug)]
pub struct Ghost {}
impl Bonus for Ghost {
    fn on_attacked(
        &self,
        damage: Power,
        receiver: &mut Unit,
        sender: &mut Unit,
        receiver_pos: UnitPos,
        sender_pos: UnitPos,
    ) -> Power {
        let mut corrected_damage_units = damage.magic;
        if corrected_damage_units == 0 {
            corrected_damage_units = 1;
        }
        if (receiver.stats.hp - corrected_damage_units as i64) < 1 {
            if sender.stats.defence.death_magic.get() <= 30 * (sender.stats.max_moves as i16) {
                sender.kill();
            }
        }
        Power {
            magic: damage.magic,
            ..Power::empty()
        }
    }
    fn id(&self) -> &'static str {
        "Ghost"
    }
}

#[derive(Copy, Clone, Debug)]
pub struct DeathCurse {}
impl Bonus for DeathCurse {
    fn on_attacked(
        &self,
        damage: Power,
        receiver: &mut Unit,
        sender: &mut Unit,
        receiver_pos: UnitPos,
        sender_pos: UnitPos,
    ) -> Power {
        let mut corrected_damage_units = damage.magic + damage.ranged + damage.hand;
        if corrected_damage_units == 0 {
            corrected_damage_units = 1;
        }
        if (receiver.stats.hp as i64 - corrected_damage_units as i64) < 1 {
            sender.kill();
        }
        damage
    }
    fn id(&self) -> &'static str {
        "DeathCurse"
    }
}

#[derive(Copy, Clone, Debug)]
pub struct Artillery {}
impl Bonus for Artillery {
    fn on_attacking(&self, damage: Power, receiver: &mut Unit, sender: &mut Unit) -> Power {
        let sender_damage: Power = sender.stats.damage;
        if sender_damage.magic > sender_damage.ranged && sender_damage.magic > sender_damage.hand {
            Power {
                magic: sender_damage.magic,
                ranged: 0,
                hand: 0,
            }
        } else if sender_damage.hand > sender_damage.ranged
            && sender_damage.hand > sender_damage.magic
        {
            Power {
                magic: 0,
                ranged: 0,
                hand: sender_damage.hand,
            }
        } else {
            Power {
                magic: 0,
                ranged: sender_damage.ranged,
                hand: 0,
            }
        }
    }
    fn on_battle_start(&self, unit: &mut Unit) -> bool {
        unit.add_effect(Box::new(ArtilleryEffect {
            info: EffectInfo { lifetime: 1 },
        }));
        true
    }
    fn id(&self) -> &'static str {
        "Artillery"
    }
}

#[derive(Copy, Clone, Debug)]
pub struct Counterblow {}
impl Bonus for Counterblow {
    fn on_attacked(
        &self,
        damage: Power,
        receiver: &mut Unit,
        sender: &mut Unit,
        receiver_pos: UnitPos,
        sender_pos: UnitPos,
    ) -> Power {
        sender.attack(receiver, receiver_pos, sender_pos);
        damage
    }
    fn id(&self) -> &'static str {
        "Counterblow"
    }
}

#[derive(Copy, Clone, Debug)]
pub struct Garrison {}
impl Bonus for Garrison {
    fn on_attacked(
        &self,
        damage: Power,
        receiver: &mut Unit,
        sender: &mut Unit,
        receiver_pos: UnitPos,
        sender_pos: UnitPos,
    ) -> Power {
        let percent_70 = Percent::new(70);
        Power {
            magic: percent_70.calc(damage.magic),
            ranged: percent_70.calc(damage.ranged),
            hand: percent_70.calc(damage.hand),
        }
    }
    fn on_battle_start(&self, unit: &mut Unit) -> bool {
        unit.add_effect(Box::new(ToEndEffect {
            info: EffectInfo { lifetime: i32::MAX },
            modify: ModifyUnitStats {
                damage: ModifyPower {
                    ranged: *Modify::default().percent_add(Percent::new(100)),
                    hand: *Modify::default().percent_add(Percent::new(100)),
                    ..Default::default()
                },
                defence: ModifyDefence {
                    ranged_units: *Modify::default().percent_add(Percent::new(100)),
                    hand_units: *Modify::default().percent_add(Percent::new(100)),
                    ..Default::default()
                },
                ..Default::default()
            },
        }))
    }

    fn id(&self) -> &'static str {
        "Garrison"
    }
}

#[derive(Copy, Clone, Debug)]
pub struct SpearDefence {}
impl Bonus for SpearDefence {
    fn on_battle_start(&self, unit: &mut Unit) -> bool {
        unit.add_effect(Box::new(SpearEffect {
            info: EffectInfo { lifetime: 1 },
        }));
        true
    }
    fn id(&self) -> &'static str {
        "SpearDefence"
    }
}

#[derive(Copy, Clone, Debug)]
pub struct NoBonus {}
impl Bonus for NoBonus {
    fn id(&self) -> &'static str {
        "NoBonus"
    }
}

// Bonus=[{отсутствие строки}
// Dead, Fire,
// Ghost, Block, Poison,
// Evasive, Berserk,
// Merchant, GodAnger, Garrison, FastDead,
// ArmyMedic, GodStrike, Artillery,
// DeathCurse, AddPayment,
// HorseAttack, ArmorIgnore, Unvulnerabe, VampirsGist, Counterblow, FlankStrike,
// SpearDefense,
// OldVampirsGist
pub fn match_bonus(bonus: &str) -> Result<Box<dyn Bonus>, ()> {
    Ok(match bonus {
        "Berserk" => Box::new(Berserk {}) as Box<dyn Bonus>,
        "Fire" => Box::new(FireAttack {}),
        "Poison" => Box::new(PoisonAttack {}),
        "Block" => Box::new(Block {}),
        "FastGoing" | "HorseAttack" | "HorseAtack" => Box::new(FastGoing {}),
        "DefencePiercing" | "ArmorIgnore" => Box::new(DefencePiercing {}),
        "Dodging" | "Evasive" => Box::new(Dodging {}),
        "Dead" | "DeadDodging" => Box::new(DeadDodging {}),
        "Invulnerable" | "Unvulnerabe" => Box::new(Invulnerable {}),
        "Ghost" => Box::new(Ghost {}),
        "GodAnger" => Box::new(GodAnger {}),
        "GodStrike" => Box::new(GodStrike {}),
        "Counterblow" => Box::new(Counterblow {}),
        "Artillery" => Box::new(Artillery {}),
        "FastDead" => Box::new(FastDead {}),
        "Garrison" => Box::new(Garrison {}),
        "DeathCurse" => Box::new(DeathCurse {}),
        "SpearDefence" | "SpearDefense" => Box::new(SpearDefence {}),
        "VampirsGist" | "VampiresGist" => Box::new(VampiresGist {}),
        "OldVampirsGist" | "OldVampiresGist" => Box::new(OldVampiresGist {}),
        "NoBonus" | "" | _ => Box::new(NoBonus {}),
    })
}

pub fn bonus_info(bonus: Box<dyn Bonus>) -> (String, String) {
    let locale = LOCALE.lock().unwrap();
    match bonus.id() {
        "Berserk" => (
            locale.get("bonus_berserk"),
            locale.get("bonus_berserk_desc"),
        ),
        "Fire" => (locale.get("bonus_fire"), locale.get("bonus_fire_desc")),
        "Poison" => (locale.get("bonus_poison"), locale.get("bonus_poison_desc")),
        "Block" => (locale.get("bonus_block"), locale.get("bonus_block_desc")),
        "FastGoing" => (
            locale.get("bonus_fastgoing"),
            locale.get("bonus_fastgoing_desc"),
        ),
        "DefencePiercing" => (
            locale.get("bonus_defencepiercing"),
            locale.get("bonus_defencepiercing_desc"),
        ),
        "Dodging" => (
            locale.get("bonus_dodging"),
            locale.get("bonus_dodging_desc"),
        ),
        "DeadDodging" => (
            locale.get("bonus_deaddodging"),
            locale.get("bonus_deaddodging_desc"),
        ),
        "Invulnerable" => (
            locale.get("bonus_invulnerable"),
            locale.get("bonus_invulnerable_desc"),
        ),
        "Ghost" => (locale.get("bonus_ghost"), locale.get("bonus_ghost_desc")),
        "GodAnger" => (
            locale.get("bonus_godanger"),
            locale.get("bonus_godanger_desc"),
        ),
        "GodStrike" => (
            locale.get("bonus_godstrike"),
            locale.get("bonus_godstrike_desc"),
        ),
        "Counterblow" => (
            locale.get("bonus_counterblow"),
            locale.get("bonus_counterblow_desc"),
        ),
        "Artillery" => (
            locale.get("bonus_artillery"),
            locale.get("bonus_artillery_desc"),
        ),
        "FastDead" => (
            locale.get("bonus_fastdead"),
            locale.get("bonus_fastdead_desc"),
        ),
        "Garrison" => (
            locale.get("bonus_garrison"),
            locale.get("bonus_garrison_desc"),
        ),
        "DeathCurse" => (
            locale.get("bonus_deathcurse"),
            locale.get("bonus_deathcurse_desc"),
        ),
        "VampiresGist" => (
            locale.get("bonus_vampiresgist"),
            locale.get("bonus_vampiresgist_desc"),
        ),
        "OldVampiresGist" => (
            locale.get("bonus_oldvampiresgist"),
            locale.get("bonus_oldvampiresgist_desc"),
        ),
        "SpearDefence" | "SpearDefense" => (
            locale.get("bonus_speardefense"),
            locale.get("bonus_speardefense_desc"),
        ),
        _ => (locale.get("unitstats_empty"), "".into()),
    }
}
