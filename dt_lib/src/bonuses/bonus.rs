use crate::{
    battle::{army::Army, battlefield::BattleInfo},
    effects::effect::*,
    time::time::Time,
    units::{
        unit::{MagicType, Power, Unit, UnitPos, UnitType},
        unitstats::{Modify, ModifyDefence, *},
    },
};
use alkahest::alkahest;
use dyn_clone::DynClone;
use math_thingies::Percent;
use std::{cmp::min, fmt::Debug};

pub struct BonusInfo {
	pub piercing: Option<Percent>,
	pub true_damage: Option<usize>,
	pub attacks_from_reserve: bool,
	pub attacks_back: bool,
	pub stats_on_first_move: Option<ModifyUnitStats>,
	pub stats_in_garrison: Option<ModifyUnitStats>,
	pub stats_on_block: Option<ModifyUnitStats>,
	pub stats_on_kill: Option<ModifyUnitStats>,
	pub kills_on_death: bool,
	pub hand_defence: Option<Percent>,
	pub ranged_defence: Option<Percent>,
	pub magic_defence: Option<Percent>,
	pub multiple_targets: bool,
	pub flank_multiplier: f32,
	pub trading_multiplier: f32,
	pub poison: bool,
	pub global_healing: Option<Percent>
}
#[derive(Copy, Debug, Clone, PartialEq, Eq)]
#[repr(u32)]
#[alkahest(Deserialize, Serialize, SerializeRef, Formula)]
pub enum Bonus {
    DefencePiercing,
    Dodging,
    Fast,
    DeadDodging,
    FastDead,
    VampiresGist,
    AncientVampiresGist,
    Berserk,
    Block,
    PoisonAttack,
    FireAttack,
    Invulnerable,
    GodAnger,
    GodStrike,
    Ghost,
    DeathCurse,
    Artillery,
    Counterblow,
    Garrison,
    Stealth,
    DeadRessurect,
    SpearDefence,
    ManyTargets,
    FlankStrike,
    Merchant,
    ArmyMedic,
    Custom = { u32::MAX - 1 },
    NoBonus = u32::MAX,
}
impl Bonus {
    pub fn on_attacked(
        &self,
        damage: Power,
        receiver: &mut Unit,
		receiver_hitmap: &Vec<Option<usize>>,
        sender: &mut Unit,
        receiver_pos: UnitPos,
        sender_pos: UnitPos,
        battle: &BattleInfo,
    ) -> Power {
        match self {
            Self::AncientVampiresGist
            | Self::VampiresGist
            | Self::DeadDodging
            | Self::Dodging
            | Self::Garrison => {
                let percent_70 = Percent::new(70);
                Power {
                    magic: percent_70.calc(damage.magic),
                    ranged: percent_70.calc(damage.ranged),
                    hand: percent_70.calc(damage.hand),
                }
            }
            Self::Invulnerable => Power {
                hand: min(1, damage.hand),
                ranged: min(1, damage.ranged),
                magic: damage.magic,
            },
            Self::Ghost => {
                let mut corrected_damage_units = damage.magic + damage.ranged + damage.hand;
                if corrected_damage_units == 0 {
                    corrected_damage_units = 1;
                }
                if (receiver.modified.hp - corrected_damage_units as i64) < 1 {
                    if sender.modified.defence.death_magic.get()
                        <= 30 * (sender.modified.max_moves as i16)
                    {
                        sender.kill();
                    }
                }
                Power {
                    magic: damage.magic,
                    ..Power::empty()
                }
            }
            Self::DeathCurse => {
                let mut corrected_damage_units = damage.magic + damage.ranged + damage.hand;
                if corrected_damage_units == 0 {
                    corrected_damage_units = 1;
                }
                if (receiver.modified.hp as i64 - corrected_damage_units as i64) < 1 {
                    sender.kill();
                }
                damage
            }
            Self::DeadRessurect => {
                let mut corrected_damage_units = damage.magic + damage.hand + damage.ranged;
                if corrected_damage_units == 0 {
                    corrected_damage_units = 1;
                }
                if (receiver.modified.hp - corrected_damage_units as i64) < 1
                    && !receiver.has_effect_kind(EffectKind::Fire)
                {
                    let hp = receiver.modified.hp;
                    receiver.add_effect(RessurectedEffect::new());
                    Power {
                        hand: hp as u64,
                        ..Power::empty()
                    }
                } else {
                    damage
                }
            }
            Self::Counterblow => {
                if sender.get_bonus() != Bonus::Counterblow {
					receiver.attack(sender, sender_pos, receiver_pos, battle, receiver_hitmap, true);
				}
                damage
            }
            Self::Stealth => {
                if receiver.modified.moves == receiver.modified.max_moves {
                    Power::empty()
                } else {
                    damage
                }
            }
            _ => damage,
        }
    }
    pub fn on_attacking(
        &self,
        damage: Power,
        receiver: &mut Unit,
        sender: &mut Unit,
        receiver_pos: UnitPos,
        sender_pos: UnitPos,
    ) -> Power {
        match self {
            Self::DefencePiercing
            | Self::VampiresGist
            | Self::AncientVampiresGist
            | Self::Artillery => pierce(sender),
            Self::PoisonAttack => {
                if !receiver.has_effect_kind(EffectKind::Poison)
                    && receiver.info.unit_type != UnitType::Undead
                {
                    if damage.ranged > 1 || damage.hand > 1 {
                        receiver.add_effect(Poison::default());
                    }
                }
                damage
            }
            Self::FireAttack => {
                if !receiver.has_effect_kind(EffectKind::Fire) {
                    if damage.ranged > 1 || damage.hand > 1 {
                        receiver.add_effect(Fire::default());
                    } else if damage.magic > 1
                        && matches!(sender.info.magic_info.and_then(|x| Some(x.0)), Some(MagicType::Elemental))
                    {
                        receiver.add_effect(Fire::new(sender.modified.damage.magic as i64));
                    } else if damage.magic > 1 {
						receiver.add_effect(Fire::new(sender.modified.damage.magic as i64 / 2));
					}
                }
                damage
            }
            Self::GodAnger => {
                damage
                    + Power {
                        hand: 10,
                        ..Power::empty()
                    }
            }
            Self::GodStrike => {
                damage
                    + Power {
                        hand: 20,
                        ..Power::empty()
                    }
            }
            Self::FlankStrike => {
                if damage.hand > 0 && receiver_pos.0.abs_diff(sender_pos.0) > 1 {
                    Power {
                        hand: {
                            damage.hand * 2
                        },
                        ..Power::empty()
                    }
                } else {
                    damage
                }
            }
            _ => damage,
        }
    }
    pub fn on_kill(&self, receiver: &mut Unit, sender: &mut Unit) -> bool {
        match self {
            Self::Berserk => {
                let percent_10 = Percent::new(10);
                sender.add_effect(ToEndEffect {
                    info: EffectInfo { lifetime: i32::MAX },
                    modify: ModifyUnitStats {
                        damage: ModifyPower {
                            hand: *Modify::default().percent_add(percent_10),
                            ranged: *Modify::default().percent_add(percent_10),
                            magic: *Modify::default().percent_add(percent_10),
                        },
                        ..Default::default()
                    },
                });
                true
            }
            _ => false,
        }
    }
    pub fn on_tick(&self, unit: &mut Unit) -> bool {
        match self {
            _ => false,
        }
    }
    pub fn on_12_hour(&self, army: &Army) -> bool {
        match self {
            Self::ArmyMedic => true,
            _ => false,
        }
    }
    pub fn on_battle_start(&self, unit: &mut Unit, battle: &BattleInfo) -> bool {
        match self {
            Self::Fast | Self::FastDead | Self::AncientVampiresGist => {
                unit.add_effect(MoreMoves::default());
                true
            }
            Self::Artillery => {
                unit.add_effect(ArtilleryEffect {
                    info: EffectInfo { lifetime: 1 },
                });
                true
            }
            Self::Garrison => unit.add_effect(ToEndEffect {
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
            }),
			Self::SpearDefence => unit.add_effect(SpearEffect { info: EffectInfo { lifetime: 1 }}),
            _ => false,
        }
    }
    pub fn on_move_skip(&self, unit: &mut Unit) -> bool {
        match self {
            Self::Block => {
                unit.add_effect(BlockEffect { info: EffectInfo { lifetime: 1 } });
                true
            }
            _ => false,
        }
    }
    pub fn can_trade(&self) -> bool {
        match self {
            Self::Merchant => true,
            _ => false,
        }
    }
    pub fn can_attack_from_reserve(&self) -> bool {
        match self {
            Self::Ghost => true,
            _ => false,
        }
    }
    pub fn locale_id(&self) -> (&'static str, &'static str) {
        match self {
            Self::Artillery => ("bonus_artillery", "bonus_artillery_desc"),
            Self::AncientVampiresGist => ("bonus_oldvampiresgist", "bonus_oldvampiresgist_desc"),
			Self::VampiresGist => ("bonus_vampiresgist", "bonus_vampiresgist_desc"),
			Self::SpearDefence => ("bonus_speardefense", "bonus_speardefense_desc"),
			Self::ArmyMedic => ("bonus_armymedic", "bonus_armymedic_desc"),
            Self::Berserk => ("bonus_berserk", "bonus_berserk_desc"),
            Self::Block => ("bonus_block", "bonus_block_desc"),
            Self::Counterblow => ("bonus_counterblow", "bonus_counterblow_desc"),
            Self::DeadDodging => ("bonus_deaddodging", "bonus_deaddodging_desc"),
            Self::DeadRessurect => ("bonus_deadressurect", "bonus_deadressurect_desc"),
            Self::DeathCurse => ("bonus_deathcurse", "bonus_deathcurse"),
            Self::DefencePiercing => ("bonus_defencepiercing", "bonus_defencepiercing_desc"),
            Self::Dodging => ("bonus_dodging", "bonus_dodging_desc"),
            Self::Fast => ("bonus_fastgoing", "bonus_fastgoing_desc"),
            Self::FastDead => ("bonus_fastdead", "bonus_fastdead_desc"),
            Self::FireAttack => ("bonus_fire", "bonus_fire_desc"),
            Self::Garrison => ("bonus_garrison", "bonus_garrison_desc"),
            Self::Ghost => ("bonus_ghost", "bonus_ghost_desc"),
            Self::GodAnger => ("bonus_godanger", "bonus_godanger_desc"),
            Self::GodStrike => ("bonus_godstrike", "bonus_godstrike_desc"),
            Self::Invulnerable => ("bonus_invulrenable", "bonus_invulrenable_desc"),
            Self::ManyTargets => ("bonus_manytargets", "bonus_manytargets"),
            Self::Merchant => ("bonus_merchant", "bonus_merchant"),
            Self::PoisonAttack => ("bonus_poison", "bonus_poison_desc"),
			Self::Stealth => ("bonus_stealth", "bonus_stealth_desc"),
			Self::FlankStrike => ("bonus_flankstrike", "bonus_flankstrike_desc"),
			Self::NoBonus | Self::Custom => ("", ""),
        }
    }
}

impl From<&str> for Bonus {
    fn from(value: &str) -> Self {
        // Bonus=[{отсутствие строки}
        // Dead, Fire,
        // Ghost, Block, Poison,
        // Evasive, Berserk,
        // Merchant, GodAnger, Garrison, FastDead,
        // ArmyMedic, GodStrike, Artillery,
        // DeathCurse, AddPayment,
        // HorseAttack, ArmorIgnore, Unvulnerabe, VampirsGist, Counterblow, FlankStrike,
        // Stealth, DeadRessurect,
        // SpearDefense,
        // OldVampirsGist
        let bonus = match value {
            "DefencePiercing" | "ArmorIgnore" => Self::DefencePiercing,
            "Dodging" | "Evasive" => Self::Dodging,
            "Fast" | "FastGoing" | "HorseAttack" | "HorseAtack" => Self::Fast,
            "DeadDodging" | "Dead" => Self::DeadDodging,
            "FastDead" => Self::FastDead,
            "VampiresGist" | "VampirsGist" => Self::VampiresGist,
            "AncientVampiresGist" | "OldVampirsGist" | "OldVampiresGist" => {
                Self::AncientVampiresGist
            }
            "Berserk" => Self::Berserk,
            "Block" => Self::Block,
            "PoisonAttack" | "Poison" => Self::PoisonAttack,
            "FireAttack" | "Fire" => Self::FireAttack,
            "Invulnerable" | "Unvulnerabe" => Self::Invulnerable,
            "GodAnger" => Self::GodAnger,
            "GodStrike" => Self::GodStrike,
            "Ghost" => Self::Ghost,
            "DeathCurse" => Self::DeathCurse,
            "Artillery" => Self::Artillery,
            "Counterblow" => Self::Counterblow,
            "Garrison" => Self::Garrison,
            "Stealth" => Self::Stealth,
            "DeadRessurect" => Self::DeadRessurect,
            "SpearDefence" | "SpearDefense" => Self::SpearDefence,
            "ManyTargets" => Self::ManyTargets,
            "Merchant" => Self::Merchant,
            "ArmyMedic" => Self::ArmyMedic,
            "FlankStrike" => Self::FlankStrike,
            "Custom" => Self::Custom,
            "NoBonus" | "" => Self::NoBonus,
			_ => {
				dbg!(value);
				Self::NoBonus
			}
        };
		if value.contains("Flank") {
			dbg!(value, bonus);
		}
		bonus
    }
}

fn pierce(sender: &mut Unit) -> Power {
    let sender_damage: Power = sender.modified.damage;
    if sender_damage.magic > sender_damage.ranged && sender_damage.magic > sender_damage.hand {
        Power {
            magic: sender_damage.magic,
            ranged: 0,
            hand: 0,
        }
    } else if sender_damage.hand > sender_damage.ranged && sender_damage.hand > sender_damage.magic
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
