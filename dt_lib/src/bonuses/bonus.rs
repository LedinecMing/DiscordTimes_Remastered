use crate::{
    battle::{BattleUnit, HitMap, MAX_LINES, Troop, TroopType, army::Army, battlefield::BattleInfo}, effects::effect::*, map::event::Cmp, registry::{self, GameInfo, Units}, time::time::Time, units::{
        unit::{AttackSettings, MagicType, Power, Unit, UnitPos, UnitType},
        unitstats::{Modify, ModifyDefence, *},
    }
};
use alkahest::alkahest;
use dyn_clone::DynClone;
use indexmap::IndexMap;
use itertools::Itertools;
use math_thingies::Percent;
use schemars::JsonSchema;
use serde::Deserialize;
use serde_with::{serde_as, FromInto};
use std::{cmp::min, collections::HashMap, default, fmt::Debug, ops::{Index, IndexMut}};

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, serde::Deserialize, serde::Serialize, JsonSchema)]
#[alkahest(Deserialize, Serialize, SerializeRef, Formula)]
#[repr(u8)]
pub enum AbilityCondition {
	BattleStart = 0,
	Turn = 1,
	BattleEnd = 2,
	Kill = 3,
	Attacked = 4,
	Attacking = 5,
	Moves = 6,
	Skips = 7,
}
pub const MAX_ABILITY: usize = AbilityCondition::Skips as u8 as usize + 1;

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, JsonSchema)]
#[alkahest(Deserialize, Serialize, SerializeRef, Formula)]
pub enum RelativeUnit {
	ByAbilityUnit(AbilityUnit),
	Absolute {
		index: usize,
		my_army: bool,
	},
	ByRelativePosition {
		pos: isize,
		my_army: bool,
	},
	ByRow {
		my_army: bool,
		row: usize
	},
	ByArmy {
		my_army: bool
	}
}
impl RelativeUnit {
	fn get_army_index(my_army: bool) -> usize {
		if my_army {
			0
		} else {
			1
		}
	}
	pub fn get<'a>(&self, my_pos: usize, hitmap: &'a [&HitMap; 2], troops: &'a [&Vec<TroopType>; 2], ability_units: &Vec<BattleUnit>) -> Vec<&'a TroopType> {
		use RelativeUnit::*;
		match self {
			ByAbilityUnit(ability_unit) => {
				if let Some(res) = ability_units.get(*ability_unit as usize).and_then(|unit| troops[unit.army].get(unit.index)) {
					vec![res]
				} else {
					vec![]
				}
			},
			Absolute { index, my_army } => {
				if let Some(res) = troops[Self::get_army_index(*my_army)].get(*index) {
					vec![res]
				} else {
					vec![]
				}
			},
			ByRelativePosition {
				pos,
				my_army,
			} => {
				let army = Self::get_army_index(*my_army);
				if let Some(res) = hitmap[army].get((my_pos as isize + *pos + hitmap.len() as isize) as usize % hitmap.len()).and_then(|hit| hit.and_then(|hit| troops[army].get(hit))) {
					vec![res]
				} else {
					vec![]
				}
			},
			ByRow { my_army, row } => {
				let army = Self::get_army_index(*my_army);
				hitmap[army].iter().skip(row * 6 ).take(6).filter_map(|x| x.and_then(|x| troops[army].get(x))).collect()
			},
			ByArmy { my_army } => {
				let army = Self::get_army_index(*my_army);
				hitmap[army].iter().filter_map(|x| x.and_then(|x| troops[army].get(x))).collect()
			},
		}
	}
}

#[derive(Debug, Clone, PartialEq, serde::Deserialize, serde::Serialize, JsonSchema)]
#[alkahest(Deserialize, Serialize, SerializeRef, Formula)]
pub struct RemoveEffect {
	#[serde(default, skip_serializing_if = "is_default")]
	pub force: bool,
	pub amount: usize,
	pub id: String,
}
#[derive(Debug, Clone, PartialEq, serde::Deserialize, serde::Serialize, JsonSchema)]
#[alkahest(Deserialize, Serialize, SerializeRef, Formula)]
pub struct AddEffect {
	pub id: String,
	#[serde(default, skip_serializing_if = "is_default")]
	pub force: bool,
	#[serde(default, skip_serializing_if = "is_default")]
	pub internal: bool,
	pub lifetime: EffectLifetime,
	#[serde(default, skip_serializing_if = "is_default")]
	pub power: usize,
	#[serde(default, skip_serializing_if = "is_default")]
	pub added_modify: Option<ModifyUnitStats>,
}
pub type PowerScale = usize;
#[derive(Clone, Debug, PartialEq, serde::Deserialize, serde::Serialize, JsonSchema)]
#[alkahest(Deserialize, Serialize, SerializeRef, Formula)]
pub enum AbilityTowardsTroop {
	Kill,
	Heal(Modify<i64>), // * max_hp
	Damage(Modify<i64>), // * max_hp
	AddEffects(Vec<AddEffect>),
  	AddEffectModify {
		modify: ModifyUnitStats,
		lifetime: EffectLifetime
	},
	// Only for effects
	ModifyStats {
		modify: ModifyUnitStats,
	},
	RemoveEffects(Vec<RemoveEffect>),
	Attack { times: usize},
	SetRules {
		#[serde(default, skip_serializing_if = "is_default")]
	 	attack_size: Option<(usize, usize)>,
		#[serde(default, skip_serializing_if = "is_default")]
	 	attacks_from_reserve: Option<bool>,
	}
}
impl AbilityTowardsTroop {
	// TODO: probably should add animations here
	pub fn apply(
		&self,
		unit: &mut Unit,
		power: usize,
		registry: &GameInfo,
	) -> (Option<ModifyUnitStats>, usize) {
		use AbilityTowardsTroop::*;
		let stats = unit.modified;
		match self {
			Kill => unit.kill(registry),
			Heal(heal) => {
				unit.heal(heal.apply(stats.max_hp) * power as i64);
			},
			Damage(damage) => {
				unit.hp -= damage.apply(stats.max_hp) * power as i64;
				unit.recalc(registry);
			},
			AddEffects(effects) => {
				for effect in effects.iter() {
					if let Some(id) = registry.effects.str_to_id(&effect.id) {
						unit.add_effect(StatusEffect {
							id,
							added_modify: effect.added_modify,
							internal: effect.internal,
							power: 1,
							lifetime: effect.lifetime,
						}.with_times(effect.power, registry), registry);
					}
				}
			},
			RemoveEffects(remove) => {
				for effect in remove.iter() {
					let mut effect = effect.clone();
					effect.amount *= power;
					unit.remove_effect(effect, registry);
				}
			},
			AddEffectModify { modify, lifetime } => {
				add_modify_effect_ex(unit, *modify * power, 0, *lifetime, registry);
				unit.recalc(registry);
				return (Some(*modify), 0);
			},
			ModifyStats { modify } => {
				unit.modify += *modify;
				unit.recalc(registry);
				return (Some(*modify), 0);
			}
			Attack { times } => {
				return (None, power * times);
			},
			SetRules { attack_size, attacks_from_reserve } => {
				if let Some(attack_size) = attack_size {
					unit.settings.attack_size = *attack_size;
				}
				if let Some(attacks_from_reserve) = attacks_from_reserve {
					unit.settings.attacks_from_reserve = *attacks_from_reserve;
				}
			}
		}
		(None, 0)
	}
}

#[derive(Debug, Clone, PartialEq, serde::Deserialize, serde::Serialize, JsonSchema)]
#[alkahest(Deserialize, Serialize, SerializeRef, Formula)]
pub enum UnitStat {
	Hp,
	MaxHp,
	Speed,
	Moves,
	MaxMoves,
	HandAttack,
	HandDefence,
	RangedAttack,
	RangedDefence,
	MagicPower,
	ElementalDefence,
	DeathDefence,
	LifeDefence,
	Vamp,
	Regen
}
#[derive(Debug, Clone, Copy, PartialEq, serde::Deserialize, serde::Serialize, JsonSchema)]
#[alkahest(Deserialize, Serialize, SerializeRef, Formula)]
#[repr(u8)]
pub enum AbilityUnit {
	Myself = 0,
	Actor = 1,
	Affected = 2
}
#[derive(Default, Debug, Clone, PartialEq, serde::Deserialize, serde::Serialize, JsonSchema)]
pub enum MechanicCondition {
	HasEffect {
		id: String,
		ability_unit: AbilityUnit
	},
	HasEffectKind {
		kind: String,
		ability_unit: AbilityUnit
	},
	IsUnitType {
		unit_type: UnitType,
		ability_unit: AbilityUnit
	},
	IsEnemy {
		ability_unit: AbilityUnit
	},
	IsInGarrison,
	Cmp {
		cmp: Cmp<i64>,
		stat: UnitStat,
		ability_unit: AbilityUnit
	},
	All(Vec<MechanicCondition>),
	Any(Vec<MechanicCondition>),
	None(Vec<MechanicCondition>),
	#[default]
	True,
	False
}
impl MechanicCondition {
	// ability_list - 1. me 2. who hit 2. who got hit
	pub fn calc(&self, battle_info: &Option<BattleInfo>, ability_list: &Vec<&(Unit, BattleUnit)>, registry: &GameInfo) -> bool {
		use MechanicCondition::*;
		match self {
			True => true,
			False => false,
			// TODO
			IsInGarrison => {
				if let Some(b) = battle_info {
					b.battle_ter == 0
				} else { false }
			},
		 	HasEffect { id, ability_unit } => {
				registry.effects.str_to_id(id).is_some_and(|x| ability_list[*ability_unit as usize].0.has_effect_id(x))
			},
			HasEffectKind { kind, ability_unit } => {
				ability_list[*ability_unit as usize].0.has_effect_kind(kind, &registry.effects)
			}
			IsUnitType { unit_type, ability_unit } => {
				ability_list[*ability_unit as usize].0.get_info(&registry.units).unit_type == *unit_type
			},
			IsEnemy { ability_unit } => {
				ability_list[*ability_unit as usize].1.army != ability_list[0].1.army
			}
			Cmp { cmp, stat, ability_unit } => {
				let unit = &ability_list[*ability_unit as usize].0;
				use UnitStat::*;
				match stat {        
					Hp => cmp.check(unit.hp),
					Moves => cmp.check(unit.moves),
					_ => cmp.check(unit.modified.get_stat(stat.clone()))
				}
			},
			All(all) => {
				all.iter().all(|x| x.calc(battle_info, ability_list, registry))
			},
			Any(any) => {
				any.iter().any(|x| x.calc(battle_info, ability_list, registry))
			},
			None(none) => {
				!none.iter().any(|x| x.calc(battle_info, ability_list, registry))
			}
		}
	}
}

#[derive(Debug, Clone, PartialEq, Default, serde::Deserialize, serde::Serialize, JsonSchema)]
pub enum ListenTo {
	#[default]
	Myself,
	MyArmy(bool),
 	Everyone,
}

/// When a condition is met applies Abilities towards all RelativeUnit
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, JsonSchema)]
pub struct Mechanic {
	#[serde(default, skip_serializing_if = "is_default")]
	pub affects: Vec<RelativeUnit>,
	
	pub affects_self: bool,

	#[serde(default)]
	pub conditions: MechanicCondition,
	#[serde(default, skip_serializing_if = "is_default")]
	pub ability: Vec<AbilityTowardsTroop>,
	#[serde(default, skip_serializing_if = "is_false")]
	pub works_after_death: bool,
}
impl Mechanic {
	pub fn check_conditions(&self, battle_info: &Option<BattleInfo>, ability_units: &Vec<BattleUnit>, troops: &[Vec<TroopType>; 2], registry: &GameInfo) -> bool {
		let mut units = ability_units.clone();
		units.dedup();
		let units = units.into_iter().map(|x| (troops[x.army][x.index].get().unit.clone(), x)).collect_vec();
		let ability = ability_units.iter().filter_map(|x| units.iter().find(|v| v.1==*x)).collect_vec();
		self.conditions.calc(battle_info, &ability, &registry)
	}
	pub fn apply(&self, power: usize, my_index: usize, my_pos: usize, hitmaps: &[&HitMap; 2], troops: &[&Vec<TroopType>; 2], ability_units: &Vec<BattleUnit>, battle_info: &BattleInfo, registry: &GameInfo) -> ModifyUnitStats {
		
		let res = self.affects.iter().map(|x| x.get(my_pos, hitmaps, troops, ability_units));
		let mut attacks = vec![];
		let mut res_modify = ModifyUnitStats::default();
		for troops in res {
			for troop in troops {
				let unit = &mut troop.get().unit;
				for ability in &self.ability {
					let (modify, attack_times) = ability.apply(unit, power, registry);
					
					if let Some(modify) = modify {
						res_modify += modify;
					}
					if attack_times > 0 {
						attacks.push((troop, attack_times));
					}
				}
			}
		}
		for attack in attacks {
			let target = &mut attack.0.get();
			let target_pos = target.pos;
			let target_unit = &mut target.unit;
			let me = &mut troops[0][my_index].get().unit;
			me.attack(target_unit, target_pos, UnitPos::from_index(my_pos), battle_info, &hitmaps[1], true, registry);
		}
		res_modify
	}
}

#[serde_as]
#[derive(Debug, Clone, PartialEq, Default, serde::Deserialize, serde::Serialize, JsonSchema)]
pub struct BonusInfo {
	pub id: String,
	pub name: String,
	pub desc: String,
	#[serde(default, skip_serializing_if = "is_default")]
	pub attack_settings: AttackSettings,
	#[serde(default, skip_serializing_if = "is_default")]
	pub add_modify: ModifyUnitStats,

	#[serde(default, skip_serializing_if = "is_default")]
	pub rules: IndexMap<AbilityCondition, (ListenTo, Vec<Mechanic>)>
}
impl BonusInfo {
	pub fn added(&self, unit: &mut Unit) {
		unit.modify += self.add_modify;
		unit.settings = self.attack_settings;
	}
	pub fn removed(&self, unit: &mut Unit, registry: &GameInfo) {
		unit.modify -= self.add_modify;
		unit.settings = unit.get_info(&registry.units).settings;
	}
	pub fn apply_rules(&self, rule: AbilityCondition, BattleUnit { army, index }: BattleUnit, my_pos: usize, armies: &Vec<Army>, ability_units: &Vec<BattleUnit>, battle: &BattleInfo, registry: &GameInfo) {
		let (army1, army2) = if army == battle.army1 { (battle.army1, battle.army2) } else { (battle.army1, battle.army2) };
		let armies = [&armies[army1], &armies[army2]];
		let troops = armies.map(|x| &x.troops);
		let hitmaps = armies.map(|x| &x.hitmap);
		for mechanic in &self.rules[&rule].1 {
			mechanic.apply(1, index, my_pos, &hitmaps, &troops, ability_units, battle, registry);
		}
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
