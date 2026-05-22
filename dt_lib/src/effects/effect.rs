use crate::{battle::{Army, BattleInfo, BattleUnit}, bonuses::{AbilityCondition, AbilityTowardsTroop, ListenTo, MAX_ABILITY, Mechanic, RemoveEffect, Rules}, registry::{Effects, GameInfo}, units::{
    unit::{Unit, *},
    unitstats::*,
}};
use indexmap::IndexMap;
use schemars::JsonSchema;
use serde_with::{FromInto, serde_as};
use advini::{Sections, Ini};
use alkahest::alkahest;
use dyn_clone::DynClone;
use enum_dispatch::enum_dispatch;
use math_thingies::{Percent, add_opt};
use std::fmt::Debug;

pub type EffectId = usize;

#[derive(Clone, Copy, Debug, PartialEq, Default, serde::Deserialize, serde::Serialize, JsonSchema)]
#[alkahest(Deserialize, Serialize, SerializeRef, Formula)]
pub struct EffectLifetime {
	#[serde(default, skip_serializing_if = "is_default")]
	pub lifetime: Option<usize>,
	pub decay: usize,
	pub remove_on_battle_end: bool,
}
impl EffectLifetime {
	pub fn tick(&mut self) {
		if let Some(lifetime) = &mut self.lifetime {
			*lifetime -= self.decay;
		}
	}
}

#[derive(Clone, Debug, PartialEq, Default, serde::Deserialize, serde::Serialize)]
pub struct EffectInfo {
	pub id: String,
	pub name: String,
	pub desc: String,

	pub kind: String,
	
	pub lifetime: EffectLifetime,
	#[serde(default, skip_serializing_if = "is_default")]
	pub stacks: bool,

	#[serde(default, skip_serializing_if = "is_default")]
	pub added_modify: ModifyUnitStats,

	#[serde(default, skip_serializing_if = "is_default")]
	pub rules: IndexMap<AbilityCondition, (ListenTo, Vec<Mechanic>)>,

	#[serde(default)]
	pub power_scales: bool,
	#[serde(default)]
	pub power_scales_with_lifetime: bool,
}

#[derive(Clone, Debug, PartialEq, Default, serde::Deserialize, serde::Serialize, JsonSchema)]
#[alkahest(Deserialize, Serialize, SerializeRef, Formula)]
pub struct StatusEffect {
	pub id: EffectId,

	pub internal: bool,
	pub lifetime: EffectLifetime,
	pub power: usize,

	pub added_modify: Option<ModifyUnitStats>,
}
impl StatusEffect {
	pub fn new(id: EffectId, lifetime: Option<EffectLifetime>, power: usize, registry: &Effects) -> Self {
		Self { id, lifetime: lifetime.unwrap_or(registry[id].lifetime.clone()), power, ..Default::default() }
	}
	pub fn with_times(self, power: usize, registry: &GameInfo) -> Self {
		let mut new = self.clone();
		let effect_info = &registry.effects[self.id];
		if !effect_info.stacks {
			return new;
		}
		let old_power = self.get_power(effect_info);
		if effect_info.power_scales {
			new.power += power;
		}
		if let Some(lifetime) = &mut new.lifetime.lifetime {
			*lifetime += power;
		}
		new
	}
	pub fn get_power(&self, info: &EffectInfo) -> usize {
		if info.power_scales_with_lifetime {
			self.power * self.lifetime.lifetime.unwrap_or(1)
		} else {
			self.power
		}
	}
	pub fn is_dead(&self) -> bool {
		self.lifetime.lifetime.is_some_and(|x| x==0) && !self.lifetime.remove_on_battle_end
	}
	pub fn removal(&self, unit: &mut Unit, registry: &GameInfo) {
		if let Some(modify) = self.added_modify {
			unit.modify -= modify;
		}
	}
	fn add_modify(&mut self, modify: Option<ModifyUnitStats>) {
		self.added_modify = add_opt(self.added_modify, modify);
	}
	// TODO: probably should add animations here
	fn apply_ability(&mut self, abilities: &Vec<Mechanic>, unit_army: usize, unit_index: usize, unit_pos: usize, armies: &Vec<Army>, ability_units: &Vec<BattleUnit>, battle: &BattleInfo, registry: &GameInfo) {
		let info = &registry.effects[self.id];
		let (army1, army2) = if unit_army == battle.army1 { (battle.army1, battle.army2) } else { (battle.army1, battle.army2) };
		let armies = [&armies[army1], &armies[army2]];
		let troops = armies.map(|x| &x.troops);
		let hitmaps = armies.map(|x| &x.hitmap);
		for mechanic in abilities {
			let to_add = mechanic.apply(self.get_power(info), unit_index, unit_pos, &hitmaps, &troops, ability_units, battle, registry);
			self.add_modify(Some(to_add));
		}
	}
	pub fn apply_rule(&mut self, rule: AbilityCondition, BattleUnit { army, index }: BattleUnit, unit_pos: usize, armies: &Vec<Army>, ability_units: &Vec<BattleUnit>, battle: &BattleInfo, registry: &GameInfo) {
		let info = &registry.effects[self.id];
		let abilities = &info.rules[&rule];
		self.apply_ability(&abilities.1, army, index, unit_pos, armies, ability_units, battle, registry);
	}
	pub fn on_tick(&mut self, unit_info: BattleUnit, unit_pos: usize, armies: &Vec<Army>, ability_units: &Vec<BattleUnit>, battle: &BattleInfo, registry: &GameInfo) {
		let info = &registry.effects[self.id];
		if let Some(lifetime) = &mut self.lifetime.lifetime {
			*lifetime = lifetime.saturating_sub(self.lifetime.decay);
		}
		self.apply_rule(AbilityCondition::Turn, unit_info, unit_pos, armies, ability_units, battle, registry);
	}
}

pub fn add_modify_effect(unit: &mut Unit, modify: ModifyUnitStats, id: usize, registry: &GameInfo) -> bool{
	unit.modify += modify;
	unit.add_effect(StatusEffect {
		id,
		internal: true,
		lifetime: EffectLifetime {
			lifetime: Some(1),
			decay: 1,
			remove_on_battle_end: true
		},
		power: 1,
		added_modify: Some(modify)
	}, registry)
}

pub fn add_modify_effect_ex(unit: &mut Unit, modify: ModifyUnitStats, id: usize, lifetime: EffectLifetime, registry: &GameInfo) -> bool{
	unit.modify += modify;
	unit.add_effect(StatusEffect {
		id,
		internal: true,
		lifetime,
		power: 1,
		added_modify: Some(modify)
	}, registry)
}


#[derive(Clone, Debug, PartialEq, Default, serde::Deserialize, serde::Serialize)]
#[alkahest(Deserialize, Serialize, SerializeRef, Formula)]
pub enum EffectKind {
    MageCurse,
    MageSupport,
	#[default]
    Bonus,
    Item,
    Potion,
    Poison,
    Fire,
}
