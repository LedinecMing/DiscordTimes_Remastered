use crate::{
    battle::{
        Army, HitMap, army::{MAX_LINES, MAX_TROOPS}, battlefield::{BattleInfo, Field, field_type}
    }, registry::{BonusId, Bonuses, Effects, GameInfo, Registry, UnitId, Units}, units::unitstats::Modify
};
use advini::Sections;
use num::{abs, Signed};
use once_cell::sync::Lazy;
use schemars::JsonSchema;

use super::unitstats::{ModifyUnitStats, is_default};
use crate::{
    bonuses::*,
    effects::effect::*,
    items::item::Item,
    units::unit::{MagicDirection::*, MagicType::*},
};
use advini::*;
use alkahest::alkahest;
use derive_more::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Sub, SubAssign};
use math_thingies::Percent;
use std::{
    fmt::{Debug, Display, Formatter},
    ops::{Div, Mul},
    sync::RwLock,
};

#[derive(Copy, Clone, Debug, Add, Sub, Default, Sections, PartialEq, serde::Deserialize, serde::Serialize)]
#[alkahest(Deserialize, Serialize, SerializeRef, Formula)]
pub struct Defence {
	#[serde(default, skip_serializing_if = "is_default")]
	#[default_value = "Percent::new(0)"]
	#[alias = "protectdeath"]
    pub death_magic: Percent,
	#[serde(default, skip_serializing_if = "is_default")]
	#[default_value = "Percent::new(0)"]
	#[alias = "protectelemental"]
    pub elemental_magic: Percent,
	#[serde(default, skip_serializing_if = "is_default")]
	#[alias = "protectlife"]
	#[default_value = "Percent::new(0)"]
	pub life_magic: Percent,
	#[serde(default, skip_serializing_if = "is_default")]
	#[alias = "protectblow"]
	#[default_value = "Percent::new(0)"]
    pub hand_percent: Percent,
	#[serde(default, skip_serializing_if = "is_default")]
	#[alias = "protectshot"]
	#[default_value = "Percent::new(0)"]
    pub ranged_percent: Percent,
	#[serde(default, skip_serializing_if = "is_default")]
	#[alias = "defencemagic"]
	#[default_value = "0u64"]
    pub magic_units: u64,
	#[alias = "defenceblow"]
	#[default_value = "0u64"]
    pub hand_units: u64,
	#[alias = "defenceshot"]
	#[default_value = "0u64"]
    pub ranged_units: u64,
}
impl Defence {
    pub fn new(
        death_magic: Percent,
        elemental_magic: Percent,
        life_magic: Percent,
        hand_percent: Percent,
        ranged_percent: Percent,
        magic_units: u64,
        hand_units: u64,
        ranged_units: u64,
    ) -> Self {
        Self {
            death_magic,
            elemental_magic,
            life_magic,
            hand_percent,
            hand_units,
            ranged_percent,
            ranged_units,
            magic_units,
        }
    }
}

#[derive(Copy, Clone, Debug, Add, Sub, Default, Sections, PartialEq, serde::Deserialize, serde::Serialize)]
#[alkahest(Deserialize, Serialize, SerializeRef, Formula)]
pub struct Power {
	#[serde(default, skip_serializing_if = "is_default")]
	#[alias = "magicpower"]
	#[default_value = "0u64"]
    pub magic: u64,
	#[serde(default, skip_serializing_if = "is_default")]
	#[default_value = "0u64"]
	#[alias = "attackshot"]
    pub ranged: u64,
	#[serde(default, skip_serializing_if = "is_default")]
	#[default_value = "0u64"]
	#[alias = "attackblow"]
    pub hand: u64,
}
impl Power {
    pub fn new(magic: u64, ranged: u64, hand: u64) -> Self {
        Self {
            magic,
            ranged,
            hand,
        }
    }
}
#[derive(Copy, Clone, Debug, Add, Sub, Default, PartialEq, Sections)]
#[alkahest(Deserialize, Serialize, SerializeRef, Formula)]
pub struct UnitStats {
	#[default_value = "1i64"]
	#[alias = "hits"]
    pub hp: i64,
	#[default_value = "1i64"]
	#[alias = "hits"]
    pub max_hp: i64,
    #[inline_parsing]
    pub damage: Power,
    #[inline_parsing]
    pub defence: Defence,
	#[default_value = "1i64"]
	#[alias = "manevres"]
    pub moves: i64,
	#[default_value = "1i64"]
	#[alias = "manevres"]
    pub max_moves: i64,
	#[default_value = "1i64"]
	#[alias = "initiative"]
    pub speed: i64,

	#[default_value = "Modify::<i64>::default().with_set(0i64)"]
	#[alias = "flank"]
	pub flank_mod: Modify<i64>, // * damage
	#[default_value = "Modify::<i64>::default().with_set(0i64)"]
	#[alias = "pierce"]
	pub pierce: Modify<i64>, // * damage
	#[default_value = "Modify::<i64>::default().with_set(0i64)"]
	#[alias = "truedamage"]
	pub true_damage: Modify<i64>, // * damage
	#[default_value = "Percent::new(0)"]
	#[alias = "vampirizm"]
    pub vamp: Percent, // * damage dealt
	#[default_value = "Percent::new(0)"]
    pub regen: Percent, // * max_hp
}
impl UnitStats {
	pub fn get_stat(&self, stat: UnitStat) -> i64 {
		use UnitStat::*;
		match stat {
			Hp => self.hp,
			Speed => self.speed,
			MaxHp => self.max_hp,
			Moves => self.moves,
			MaxMoves => self.max_moves,
			HandAttack => self.damage.hand as i64,
			HandDefence => self.defence.hand_units as i64,
			RangedAttack => self.damage.ranged as i64,
			RangedDefence => self.defence.ranged_units as i64,
			MagicPower => self.damage.magic as i64,
			ElementalDefence => self.defence.elemental_magic.get() as i64,
			DeathDefence => self.defence.death_magic.get() as i64,
			LifeDefence => self.defence.life_magic.get() as i64,
			Vamp => self.vamp.get() as i64,
			Regen => self.vamp.get() as i64,
		}
	}
}
#[derive(Copy, Clone, Debug, PartialEq, Ini, Default)]
#[alkahest(Deserialize, Serialize, SerializeRef, Formula)]
pub enum MagicDirection {
    ToAlly,
	#[default]
    ToAll,
    ToEnemy,
    CurseOnly,
    StrikeOnly,
    BlessOnly,
    CureOnly,
}
#[derive(Copy, Clone, Debug, PartialEq, Ini)]
#[alkahest(Deserialize, Serialize, SerializeRef, Formula)]
pub enum MagicType {
    Life,
    Death,
    Elemental,
}

#[derive(Clone, Debug, PartialEq, Default, Sections)]
#[alkahest(Deserialize, Serialize, SerializeRef, Formula)]
pub struct LevelUpInfo {
	#[unused]
	#[inline_parsing]
	#[default_value = "ModifyUnitStats::default()"]
    pub stats: ModifyUnitStats,
	#[alias = "startexpirience"]
    pub xp_up: i16,
	#[alias= "levelmultipler"]
    pub max_xp: i16,
}

#[derive(Clone, Debug, PartialEq, Default, Sections)]
#[alkahest(Deserialize, Serialize, SerializeRef, Formula)]
pub struct UnitInfo {
	#[unused]
	#[default_value = "0usize"]
	pub id: usize,
	#[default_value = "String::new()"]
	pub str_id: String,
    pub name: String,
    pub descript: String,
	#[default_value = "0u64"]
    pub cost: u64,
	#[default_value = "1u64"]
    pub cost_hire: u64,
	#[alias = "globalindex"]
    pub icon_index: usize,
	#[default_value = "(1usize, 1usize)"]
    pub size: (usize, usize),
	#[alias = "nature"]
	#[default_value = "UnitType::People"]
    pub unit_type: UnitType,
	#[default_value = "Vec::<usize>::new()"]
    pub next_unit: Vec<usize>,
	#[alias = "magic"]
	#[default_value = "None"]
    pub magic_type: Option<MagicType>,
	#[alias = "magicdirection"]
	#[default_value = "MagicDirection::ToAll"]
	pub magic_direction: MagicDirection,
	#[default_value = "None"]
    pub surrender: Option<u64>,
	#[default_value = "None"]
	pub bonus: Option<String>,
	#[default_value = "AttackSettings::default()"]
	#[inline_parsing]
	pub settings: AttackSettings,
	#[inline_parsing]
	pub stats: UnitStats,
	#[inline_parsing]
    pub lvl: LevelUpInfo,
}
#[derive(Clone, Debug, PartialEq, Default)]
#[alkahest(Deserialize, Serialize, SerializeRef, Formula)]
pub struct UnitInventory {
    pub items: Vec<Option<Item>>,
}

#[derive(Clone, Debug, PartialEq, Default)]
#[alkahest(Deserialize, Serialize, SerializeRef, Formula)]
pub struct UnitLvl {
    pub lvl: u64,
    pub max_xp: u64,
    pub xp: u64,
}

#[derive(
    Debug,
    Copy,
    Clone,
    Add,
    AddAssign,
    Sub,
    SubAssign,
    Mul,
    MulAssign,
    Div,
    DivAssign,
    PartialEq,
    Eq,
)]
#[alkahest(Deserialize, Serialize, SerializeRef, Formula)]
pub struct UnitPos(pub usize, pub usize);
impl UnitPos {
    pub fn from_index(index: usize) -> Self {
        let max_troops = MAX_TROOPS / MAX_LINES;
        Self(index % max_troops, index / max_troops)
    }
    pub fn whole(&self) -> usize {
        self.0 + self.1 * MAX_TROOPS / MAX_LINES
    }
}
impl Into<(usize, usize)> for UnitPos {
    fn into(self) -> (usize, usize) {
        (self.0, self.1)
    }
}
impl Into<usize> for UnitPos {
    fn into(self) -> usize {
        self.0 + self.1 * MAX_TROOPS / MAX_LINES
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Default, Ini, serde::Serialize, serde::Deserialize, JsonSchema)]
#[alkahest(Deserialize, Serialize, SerializeRef, Formula)]
pub enum UnitType {
	#[default]
    People,
    Hero,
    Animal,
    Mecha,
    Undead,
    Rogue,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Sections, serde::Deserialize, serde::Serialize, JsonSchema)]
#[alkahest(Deserialize, Serialize, SerializeRef, Formula)]
pub struct AttackSettings {
	#[alias = "attacksize"]
	#[default_value = "(1, 1)"]
	pub attack_size: (usize, usize),
	#[alias = "distributeattack"]
	#[default_value = "false"]
	pub distribute_attack: bool,
	#[alias = "attacksfromreserve"]
	#[default_value = "false"]
	pub attacks_from_reserve: bool,
}

#[derive(Clone, Debug, PartialEq, Default)]
#[alkahest(Deserialize, Serialize, SerializeRef, Formula)]
pub struct Unit {
	pub id: UnitId,
	pub hp: i64,
	pub moves: i64,
    pub modified: UnitStats,
    pub modify: ModifyUnitStats,
	pub settings: AttackSettings,
    pub lvl: UnitLvl,
    pub inventory: UnitInventory,
    pub bonus: Option<BonusId>,
    pub effects: Vec<StatusEffect>,
}
impl From<(UnitInfo, &Bonuses)> for Unit {
	fn from((value, bonuses): (UnitInfo, &Bonuses)) -> Self {
		Self {
			id: value.icon_index - 1,
			hp: value.stats.hp,
			moves: value.stats.moves,
			modified: value.stats.clone(),
			bonus: value.bonus.and_then(|x| bonuses.str_to_id(&x)),
			..Default::default()
		}
	}
}
impl Ini<'_> for Unit {
	type Arg = Units;
	fn eat<'a>(input: &'a str, _additional: Self::Arg) -> Result<(&'a str, Self), IniParseError> {
		todo!()
	}
	fn vomit(&self, additional: Self::Arg) -> String {
		todo!()
	}
}
pub fn heal_unit(
    unit: &mut Unit,
    mut damage: Power,
    magic_type: MagicType,
	target_unit_type: UnitType,
) -> Option<ActionResult> {
    return match (target_unit_type, magic_type) {
        (UnitType::Rogue | UnitType::Hero | UnitType::People, Death) => None,
        (UnitType::Undead, Life) => None,
        _ => {
            if let MagicType::Elemental = magic_type {
                damage.magic /= 2;
            }
            if unit.modified.max_hp <= unit.hp {
                None
            } else {
                if unit.heal(damage.magic as i64) {
                    Some(ActionResult::Buff)
                } else {
                    Some(ActionResult::Buff)
                }
            }
        }
    };
}
fn bless_unit(target: &mut Unit, damage: Power, magic_type: MagicType, target_unit_type: UnitType, registry: &GameInfo) -> Option<ActionResult> {
    match (target_unit_type, magic_type) {
        (UnitType::Rogue | UnitType::Hero | UnitType::People, Death) => None,
        (UnitType::Undead, Life) => None,
        (UnitType::Mecha, _) => None,
        _ => {
			let mut res = ModifyUnitStats::default();
			let magic = damage.magic as i64;
			let (add_damage, add_defence) = match magic_type {
				MagicType::Death => {
					(1 + magic  / 6, 1 + magic / 12)
				},
				MagicType::Life => {
					(1 + magic / 8, 1 + magic / 4)
				},
				_ => { (1, 1) }
			};
			res.damage.hand = Modify { add: Some(add_damage), ..Default::default() };
			res.damage.ranged = Modify { add: Some(add_damage), ..Default::default() };
			res.defence.hand_units = Modify { add: Some(add_defence), ..Default::default() };
			res.defence.ranged_units = Modify { add: Some(add_defence), ..Default::default() };
			if add_modify_effect(target, res, 1, registry) {
				return Some(ActionResult::Buff);
			} else {
				None
			}
        }
    }
}
pub fn heal_bless(target: &mut Unit, damage: Power, magic_type: MagicType, target_unit_type: UnitType, registry: &GameInfo) -> Option<ActionResult> {
    match (target_unit_type, magic_type) {
        (UnitType::Rogue | UnitType::Hero | UnitType::People, Death) => None,
        (UnitType::Undead, Life) => None,
        (UnitType::Mecha, Death | Life) => None,
        _ => {
            if heal_unit(target, damage, magic_type, target_unit_type).is_none() {
                return bless_unit(target, damage, magic_type, target_unit_type, registry);
            }
            Some(ActionResult::Buff)
        }
    }
}
pub fn elemental_bless(target: &mut Unit, damage: Power, target_unit_type: UnitType, target_magic_type: Option<MagicType>, registry: &GameInfo) -> Option<ActionResult> {
    if heal_unit(target, damage, Elemental, target_unit_type).is_none() {
        return if !target.has_effect_kind("mage_support", &registry.effects)
            && !matches!(target_magic_type, Some(MagicType::Elemental))
        {
			let mut res = ModifyUnitStats::default();
			let magic = damage.magic as i64;
			let add_moves = match magic {
				0..=19 => 0,
				20..=44 => 1,
				45..=99 => 2,
				100..=255 => 3,
				_ => magic / 64
			};
			let add_ini = 1 + magic /  7;
			target.moves += add_moves;
			res.speed += Modify { add: Some(add_ini), ..Default::default() };
			if add_modify_effect(target, res, 1, registry) {
				Some(ActionResult::Buff)
			} else {
				None
			}
        } else {
            None
        };
    }
    Some(ActionResult::Buff)
}

fn magic_curse(
    me: &mut Unit,
    target: &mut Unit,
    mut damage: Power,
    magic_type: MagicType,
	target_unit_type: UnitType,
	registry: &GameInfo,
) -> Option<ActionResult> {
    match (target_unit_type, magic_type) {
        (UnitType::Undead, Life) => {
            damage.magic *= 2;
        }
        _ => {}
    }
    if !target.has_effect_kind("mage_curse", &registry.effects) {
		let mut res = ModifyUnitStats::default();
		let magic = damage.magic as i64;
		let (add_damage, add_defence) = match magic_type {
			MagicType::Death => {
				(1 + magic  / 5, 1 + magic / 10)
			},
			MagicType::Life => {
				(1 + magic / 10, 1 + magic / 3)
			},
			_ => { (1, 1) }
		};
		res.damage.hand = Modify { add: Some(-add_damage), ..Default::default() };
		res.damage.ranged = Modify { add: Some(-add_damage), ..Default::default() };
		res.defence.hand_units = Modify { add: Some(-add_defence), ..Default::default() };
		res.defence.ranged_units = Modify { add: Some(-add_defence), ..Default::default() };
		if add_modify_effect(target, res, 2, registry) {
			return Some(ActionResult::Debuff);
		} else {
			None
		}
    } else {
        None
    }
}

fn elemental_curse(
    me: &mut Unit,
    target: &mut Unit,
    damage: Power,
    magic_type: MagicType,
	registry: &GameInfo,
) -> Option<ActionResult> {
    if !target.has_effect_kind("elemental_curse", &registry.effects) {
        let mut res = ModifyUnitStats::default();
		let magic = damage.magic as i64;
		let add_moves = match magic {
			0..=19 => 0,
			20..=44 => 1,
			45..=99 => 2,
			100..=255 => 3,
			_ => magic / 64
		};
		let add_ini = 1 + magic /  7;
		target.moves -= add_moves;
		res.speed = Modify { add: Some(-add_ini), ..Default::default() };
		if add_modify_effect(target, res, 2, registry) {
			return Some(ActionResult::Debuff);
		} else { None }
    } else {
        None
    }
}

fn magic_attack(
    me: &mut Unit,
    target: &mut Unit,
    mut damage: Power,
    magic_type: MagicType,
    target_pos: UnitPos,
    my_pos: UnitPos,
    battle: &BattleInfo,
    hitmap: &HitMap,
	target_unit_type: UnitType,
	registry: &GameInfo
) -> Option<ActionResult> {
    match (target_unit_type, magic_type) {
        (UnitType::Undead, Life) => {
            damage.magic *= 2;
        }
        _ => {}
    }
    if let Some(res) = magic_curse(me, target, damage, magic_type, target_unit_type, registry) {
        Some(res)
    } else {
        damage.hand = 0;
        damage.ranged = 0;
        let res = target.being_attacked(&damage, me, target_pos, hitmap, my_pos, battle, registry);
        Some(ActionResult::Melee)
    }
}

fn elemental_attack(
    me: &mut Unit,
    target: &mut Unit,
    damage: Power,
    target_pos: UnitPos,
    my_pos: UnitPos,
    battle: &BattleInfo,
    magic_type: MagicType,
    hitmap: &HitMap,
	registry: &GameInfo,
) -> Option<ActionResult> {
    let mut damage = damage;
    if !elemental_curse(me, target, damage, magic_type, registry).is_some() {
        damage.hand = 0;
        damage.ranged = 0;
        target.being_attacked(&damage, me, target_pos, hitmap, my_pos, battle, registry);
    }
    Some(ActionResult::Debuff)
}

fn is_front_empty(hitmap: &HitMap, my_pos: UnitPos) -> bool {
    let me = my_pos.whole() as i64;
    let other = me + 2;
    let sign = if me > other { -1 } else { 1 };
    (1..=(me - sign - other).abs())
        .map(|pos| (other + (pos * -sign)).max(0).min((MAX_TROOPS - 1) as i64) as usize)
        .all(|x| hitmap[x].is_none() || field_type(x, MAX_TROOPS) == Field::Reserve)
}
fn is_path_empty(hitmap: &HitMap, my_pos: UnitPos, target_pos: UnitPos) -> bool {
    let me = my_pos.whole() as i64;
    let other = target_pos.whole() as i64;
    if me == other {
        return true;
    };
    let sign = if me > other { -1 } else { 1 };
    (1..=(me - sign - other).abs())
        .map(|pos| (other + (pos * -sign)).max(0).min((MAX_TROOPS - 1) as i64) as usize)
        .all(|x| hitmap[x].is_none() || field_type(x, MAX_TROOPS) == Field::Reserve)
}

#[derive(Debug, PartialEq)]
pub enum ActionResult {
    Buff,
    Debuff,
    Melee,
    Ranged,
    Move,
}
impl Unit {
    pub fn recalc(&mut self, registry: &GameInfo) {
		let base = self.get_base_stats(&registry.units);
		let info = self.get_info(&registry.units);
        self.modified = self
            .modify
            .apply(base, info.magic_type);
        self.hp = self.hp.min(self.modified.max_hp);
        self.moves = self.moves.min(self.modified.max_moves);
    }
    pub fn new(
		id: UnitId,
        info: UnitInfo,
        lvl: UnitLvl,
        inventory: UnitInventory,
        bonus: Option<BonusId>,
        effects: Vec<StatusEffect>,
    ) -> Self {
        let mut unit = Self {
			id,
            lvl,
            inventory,
            effects,
            modify: ModifyUnitStats::default(),
            bonus,
			..Default::default()
        };
        unit
    }
    pub fn can_attack(
        &self,
        target: &Unit,
        target_pos: UnitPos,
        my_pos: UnitPos,
        is_enemy: bool,
        target_hitmap: &Vec<Option<usize>>,
		registry: &GameInfo,
    ) -> bool {
        let effected = self.modified;
        let my_field = field_type(my_pos.into(), MAX_TROOPS);
        let is_in_back = my_field == Field::Back;
        let enemy_field = field_type(target_pos.into(), MAX_TROOPS);
        let mut damage = effected.damage;
        let enemy_in_reserve = enemy_field == Field::Reserve;
        let me_in_reserve = my_field == Field::Reserve;
        let both_in_reserve = me_in_reserve && enemy_in_reserve;
        let both_not_in_reserve = !me_in_reserve && !enemy_in_reserve;

        let can_reserve = me_in_reserve && self.settings.attacks_from_reserve;
        let allies_together = (both_in_reserve || both_not_in_reserve) && !is_enemy;
        let field_checks = allies_together || !enemy_in_reserve && (!me_in_reserve || can_reserve);

        let is_path_empty = is_path_empty(target_hitmap, my_pos, target_pos);
        let is_front_empty = is_front_empty(target_hitmap, my_pos);
        let dist = target_pos.0.abs_diff(my_pos.0);

		let my_info = self.get_info(&registry.units);
		let target_info = target.get_info(&registry.units);
		
		if !field_checks {
            return false;
        }
        if damage.ranged > 0
            && ((enemy_field == Field::Front && (dist < 2 || is_path_empty))
                || enemy_field == Field::Back && is_front_empty
                || is_in_back
                || can_reserve)
            && is_enemy
        {
            true
        } else if (damage.hand > 0 && !is_in_back && is_enemy && enemy_field == Field::Front)
            && (is_path_empty || dist < 2)
        {
            // This checks that receiver is in front and there are no obstacles between me and him.
            true
        } else {
            if !((enemy_field == Field::Front && is_path_empty && dist > 1)
                || (enemy_field == Field::Back && is_front_empty)
                || is_in_back
                || can_reserve
                || (!is_enemy
                    && (allies_together || can_reserve)
                    && matches!(
                        my_info.magic_direction,
						ToAll | ToAlly | CureOnly | BlessOnly)
					&& my_info.magic_type.is_some()
                    ))
            {
                return false;
            }
            match my_info.magic_type {
                None => false,
                Some(magic_type) => {
                    match (my_info.magic_direction, magic_type, is_enemy) {
                        (ToAlly, _, false) => match magic_type {
                            Death | Life => match (target_info.unit_type, magic_type) {
                                (UnitType::Rogue | UnitType::Hero | UnitType::People, Death) => {
                                    false
                                }
                                (UnitType::Undead, Life) => false,
                                _ => !target.has_effect_kind("mage_support", &registry.effects),
                            },
                            Elemental => !target.has_effect_kind("elemental_support", &registry.effects),
                        },
                        (ToAll, _, _) => match (magic_type, is_enemy) {
                            (Death | Life, true) => true,
                            (Death | Life, false) => match (target_info.unit_type, magic_type) {
                                (UnitType::Rogue | UnitType::Hero | UnitType::People, Death) => {
                                    false
                                }
                                (UnitType::Undead, Life) => false,
                                (UnitType::Mecha, Death | Life) => false,
                                _ => !target.has_effect_kind("mage_support", &registry.effects),
                            },
                            (Elemental, true) => true,
                            (Elemental, false) => !target.has_effect_kind("elemental_support", &registry.effects),
                        },
                        (ToEnemy, _, true) => match magic_type {
                            Death | Life => true,
                            Elemental => true,
                        },
                        (CurseOnly, _, true) => match magic_type {
                            Death | Life => true,
                            Elemental => true,
                        },
                        (StrikeOnly, _, true) => true,
                        (BlessOnly, _, false) => match magic_type {
                            Life | Death => match (target_info.unit_type, magic_type) {
                                (UnitType::Rogue | UnitType::Hero | UnitType::People, Death) => {
                                    false
                                }
                                (UnitType::Undead, Life) => false,
                                (UnitType::Mecha, Death | Life) => false,
                                _ => !target.has_effect_kind("mage_support", &registry.effects),
                            },
                            Elemental => !target.has_effect_kind("elemental_support", &registry.effects),
                        },
                        (CureOnly, Life | Death, false) => {
                            match (target_info.unit_type, magic_type) {
                                (UnitType::Rogue | UnitType::Hero | UnitType::People, Death) => {
                                    false
                                }
                                (UnitType::Undead, Life) => false,
                                (UnitType::Mecha, Death | Life) => false,
                                _ => !target.hp >= target.modified.max_hp,
                            }
                        }
                        _ => false,
                    }
                }
            }
        }
    }
    pub fn attack(
        &mut self,
        target: &mut Unit,
        target_pos: UnitPos,
        my_pos: UnitPos,
        battle: &BattleInfo,
        target_hitmap: &Vec<Option<usize>>,
        is_enemy: bool,
		registry: &GameInfo,
    ) -> Option<ActionResult> {
        let effected = self.modified;
        let my_field = field_type(my_pos.into(), MAX_TROOPS);
        let is_in_back = my_field == Field::Back;
        let enemy_field = field_type(target_pos.into(), MAX_TROOPS);
        let mut damage = effected.damage;
        let enemy_in_reserve = enemy_field == Field::Reserve;
        let me_in_reserve = my_field == Field::Reserve;
        let both_in_reserve = me_in_reserve && enemy_in_reserve;
        let both_not_in_reserve = !me_in_reserve && !enemy_in_reserve;

		let my_info = self.get_info(&registry.units);
        let bonus = self.get_bonus(registry);

        let can_reserve = me_in_reserve && self.settings.attacks_from_reserve;
        let allies_together = (both_in_reserve || both_not_in_reserve) && !is_enemy;
        let field_checks = allies_together || !enemy_in_reserve && (!me_in_reserve || can_reserve);

        let is_path_empty = is_path_empty(target_hitmap, my_pos, target_pos);
        let is_front_empty = is_front_empty(target_hitmap, my_pos);
        let dist = target_pos.0.abs_diff(my_pos.0);

		let target_info = target.get_info(&registry.units);
		let target_unit_type = target_info.unit_type;
		
        return if !field_checks {
            None
        } else if (damage.hand > 0 && !is_in_back && target_pos.1 == 1 && is_enemy)
            && (is_path_empty || dist < 2)
        {
            damage.ranged = 0;
            damage.magic = 0;
            let _ = target.being_attacked(&damage, self, target_pos, target_hitmap, my_pos, battle, registry);
            Some(ActionResult::Melee)
        } else if damage.ranged > 0
            && ((enemy_field == Field::Front && (dist < 2 || is_path_empty))
                || enemy_field == Field::Back && is_front_empty
                || is_in_back
                || can_reserve)
            && is_enemy
        {
            damage.hand = 0;
            damage.magic = 0;
            let _ = target.being_attacked(&damage, self, target_pos, target_hitmap, my_pos, battle, registry);
            Some(ActionResult::Ranged)
        } else {
            if !((enemy_field == Field::Front && is_path_empty && dist > 1)
                || (enemy_field == Field::Back && is_front_empty)
                || is_in_back
                || can_reserve
                || (!is_enemy
                    && (allies_together || can_reserve)
                    && matches!(
                        my_info.magic_direction,
						ToAll | ToAlly | CureOnly | BlessOnly)
					&& my_info.magic_type.is_some()
                    ))
            {
                return None;
            }
            if effected.damage.magic < 1 {
                return None;
            }
            let Some(magic_type) = my_info.magic_type else {
                return None;
            };
            match (my_info.magic_direction, magic_type, is_enemy) {
                (ToAlly, _, false) => match magic_type {
                    Death | Life => heal_bless(target, damage, magic_type, target_unit_type, registry),
                    Elemental => elemental_bless(target, damage, target_unit_type, target_info.magic_type, registry),
                },
                (ToAll, _, _) => match (magic_type, is_enemy) {
                    (Death | Life, true) => magic_attack(
                        self,
                        target,
                        damage,
                        magic_type,
                        my_pos,
                        target_pos,
                        battle,
                        target_hitmap,
						target_unit_type,
						registry
                    ),
                    (Death | Life, false) => heal_bless(target, damage, magic_type, target_unit_type, registry),
                    (Elemental, true) => elemental_attack(
                        self,
                        target,
                        damage,
                        target_pos,
                        my_pos,
                        battle,
                        magic_type,
                        target_hitmap,
						registry
                    ),
                    (Elemental, false) => elemental_bless(target, damage, target_unit_type, target_info.magic_type, registry),
                },
                (ToEnemy, _, true) => match magic_type {
                    Death | Life => magic_attack(
                        self,
                        target,
                        damage,
                        magic_type,
                        my_pos,
                        target_pos,
                        battle,
                        target_hitmap,
						target_unit_type,
						registry
                    ),
                    Elemental => elemental_attack(
                        self,
                        target,
                        damage,
                        target_pos,
                        my_pos,
                        battle,
                        magic_type,
                        target_hitmap,
						registry
                    ),
                },
                (CurseOnly, _, true) => match magic_type {
                    Death | Life => magic_curse(self, target, damage, magic_type, target_unit_type, registry),
                    Elemental => elemental_curse(self, target, damage, magic_type, registry),
                },
                (StrikeOnly, _, true) => {
                    damage.hand = 0;
                    damage.ranged = 0;
                    target.being_attacked(&damage, self, my_pos, target_hitmap, target_pos, battle, registry);
                    Some(ActionResult::Debuff)
                }
                (BlessOnly, _, false) => match magic_type {
                    Life | Death => bless_unit(target, damage, magic_type, target_unit_type, registry),
                    Elemental => elemental_bless(target, damage, target_unit_type, target_info.magic_type, registry),
                },
                (CureOnly, Life | Death, false) => heal_unit(target, damage, magic_type, target_unit_type),
                _ => None,
            }
        };
    }
    pub fn restore(&mut self) {
        self.hp = self.modified.max_hp;
        self.moves = self.modified.max_moves;
    }
    pub fn heal(&mut self, amount: i64) -> bool {
        let effected = self.modified;
        if effected.max_hp < self.hp + amount {
            self.hp = effected.max_hp;
            return true;
        }
        self.hp += amount;
        return false;
    }
	pub fn get_info<'a>(&self, registry: &'a Units) -> &'a UnitInfo {
		&registry[self.id]
	}
	pub fn get_base_stats<'a>(&self, registry: &'a Units) -> &'a UnitStats {
		&registry[self.id].stats
	}
    pub fn get_stats(&self) -> &UnitStats {
        &self.modified
    }
    pub fn is_dead(&self) -> bool {
        self.hp < 1
    }
    pub fn has_effect_kind(&self, kind: impl Into<String>, registry: &Effects) -> bool {
		let kind = kind.into();
		self.effects.iter().any(|x| &registry[self.id].kind == &kind)
    }
	pub fn has_effect_id(&self, id: EffectId) -> bool {
		self.effects.iter().any(|x| x.id == id)
    }
	pub fn get_effect_by_id(&mut self, id: EffectId) -> Option<&mut StatusEffect> {
		self.effects.iter_mut().find(|x| x.id == id)
    }
	
    pub fn kill(&mut self, registry: &GameInfo) {
        self.hp = 0;
        self.recalc(registry);
    }
	
    pub fn add_effect(&mut self, add: StatusEffect, registry: &GameInfo) -> bool {
		let mut effect = self.get_effect_by_id(add.id);
		if effect.is_none() {
			self.effects.push(add.clone());
			effect = self.effects.last_mut();
		}
		let Some(effect) = effect else { return false; }; 
		let effect_info = &registry.effects[effect.id];
		if !effect_info.stacks {
			return false;
		}
		let old_power = effect.get_power(effect_info);
		if effect_info.power_scales {
			effect.power += add.power;
		}
		if let Some(lifetime) = &mut effect.lifetime.lifetime {
			*lifetime += add.lifetime.lifetime.unwrap_or(0);
		}
		let new_power = effect.get_power(effect_info);
		if old_power != new_power {
			if let Some(added_modify) = effect.added_modify {
				self.modify -= added_modify;
				self.modify += added_modify / old_power * new_power;
			}
		}
		true
    }
    pub fn remove_effect(&mut self, remove: RemoveEffect, registry: &GameInfo) -> bool {
		let Some(effect) = registry.effects.str_to_id(&remove.id).and_then(|x| self.get_effect_by_id(x)) else { return false; };
		let effect_info = &registry.effects[effect.id];
		let old_power = effect.get_power(effect_info);
		let modify = effect.added_modify;
		if let Some(lifetime) = &mut effect.lifetime.lifetime && *lifetime > remove.amount && effect_info.stacks {
			*lifetime -= remove.amount;
			drop(lifetime);
			if effect_info.power_scales {
				effect.power = (effect.power - remove.amount).max(1);
			}
			let new_power = effect.get_power(effect_info);
			if old_power != new_power {
				if let Some(added_modify) = effect.added_modify {
					drop(effect);
					self.modify -= added_modify;
					self.modify += added_modify / old_power * new_power;
					self.recalc(registry);
				}
			}
			false
		} else {
			if let Some(modify) = modify {
				self.modify -= modify;
			}
			self.recalc(registry);
			true
		}
	}	
	pub fn add_item(&mut self, item: Option<Item>, index: usize, registry: &GameInfo) -> bool {
		let Some(item) = item else {
			self.inventory.items[index] = None;
			return true;
		};
		if item.can_equip(&*self, registry) {
			self.modify += item.get_info(&registry.items).modify;
			self.inventory.items[index] = Some(item);
			self.recalc(registry);
			true
		} else {
			false
		}
	}
	pub fn remove_item(&mut self, index: usize, registry: &GameInfo) {
		let item = self
			.inventory
			.items
			.remove(index)
			.expect("No such index for items");
		self.inventory.items.insert(index, None);
		let item_info = item.get_info(&registry.items);
		self.modify -= item_info.modify;
		self.recalc(registry);
	}
	pub fn swap_item(&mut self, index: usize, add_item: Option<Item>, registry: &GameInfo) -> Option<Item> {
		let item = self.inventory.items.remove(index);
		if let Some(item) = item {
			let item_info = item.get_info(&registry.items);
			self.modify -= item_info.modify;
			self.recalc(registry);
		}
		if let Some(item) = add_item {
			if item.can_equip(&self, registry) {
				self.inventory.items.insert(index, add_item);
				self.modify += item.get_info(&registry.items).modify;
				self.recalc(registry);
			} else {
				self.inventory.items.insert(index, None);
			}
		} else {
			self.inventory.items.insert(index, None);
		}
		item
	}
	pub fn get_bonus<'a>(&self, registry: &'a GameInfo) -> Option<&'a BonusInfo> {
		let mut bonus = self.bonus;
		if let Some(item) = self
			.inventory
			.items
			.iter()
			.filter(|item| {
				if let Some(item) = item {
					item.get_info(&registry.items).bonus.is_some()
				} else {
					false
				}
			})
			.last()
		{
			if let Some(item_bonus) = (&item.unwrap()).get_info(&registry.items).bonus {
				bonus = Some(item_bonus.clone());
			};
		};
		bonus.and_then(|b| Some(&registry.bonuses[b]))
	}
	pub fn being_attacked(
		&mut self,
		damage: &Power,
		sender: &mut Unit,
		my_pos: UnitPos,
		my_hitmap: &HitMap,
		attacker_pos: UnitPos,
		battle: &BattleInfo,
		registry: &GameInfo,
	) -> u64 {
		let info = self.get_info(&registry.units);
		let is_flank_attack = my_pos.0.abs_diff(attacker_pos.0) > 1;
		let flank_info = if is_flank_attack { Some(1.) } else { None };
		let corrected_damage = self.correct_damage(
			damage,
			info.magic_type,
			flank_info,
		);
		let unit_bonus = sender.get_bonus(registry);
		// TODO: bonus should be applied after the attack
		let corrected_damage_units =
			(corrected_damage.magic + corrected_damage.ranged + corrected_damage.hand).max(1);
		self.hp = self.hp.abs_sub(&(corrected_damage_units as i64));
		if self.is_dead() {
			// TODO: bonus
		}
		match info.unit_type {
			UnitType::Undead | UnitType::Mecha => {}
			_ => {
				sender.heal(sender.modified.vamp.calc(corrected_damage_units) as i64); // apply vampirizm
			}
		}
		corrected_damage_units
	}
	pub fn correct_damage(
		&self,
		damage: &Power,
		magic_type: Option<MagicType>,
		flank_modifier: Option<f32>,
	) -> Power {
		let mut damage = damage.clone();
		let defence: Defence = self.modified.defence;
		let percent_100 = Percent::new(100);
		let mut magic: u64 = 0;

		if let Some(magic_type) = magic_type {
			let magic_def = match magic_type {
				Life => defence.life_magic,
				Death => defence.death_magic,
				Elemental => defence.elemental_magic,
			};
			magic =
				(percent_100 - magic_def).calc(damage.magic.saturating_sub(defence.magic_units));
		}
		let hand_defence = if let Some(flank_mod) = flank_modifier {
			damage.hand = (damage.hand as f32).mul(flank_mod).ceil() as u64;
			(defence.hand_units as f32).div(2.).ceil() as u64
		} else {
			defence.hand_units
		};
		Power {
			ranged: (percent_100 - defence.ranged_percent)
				.calc(damage.ranged.saturating_sub(defence.ranged_units)),
			magic,
			hand: (percent_100 - defence.hand_percent)
				.calc(damage.hand.saturating_sub(hand_defence)),
		}
	}
    pub fn tick(&mut self, battle: &BattleInfo, armies: &Vec<Army>, registry: &GameInfo) -> bool {
        let mut effects = self.effects.clone();
		// TODO: EFFECTS
        
        self.heal(self.modified.regen.calc(self.modified.max_hp)); // apply regeneration
		
		// TODO: APPLY BONUS
        if let Some(bonus) = &self.get_bonus(registry) {
			bonus.apply_rules(AbilityCondition::Turn, )
		};
        self.recalc(registry);
        true
    }
}
pub fn display_unit(unit: &Unit, registry: &GameInfo) -> Vec<String> {
    let mut strings = vec![];
	let info = unit.get_info(&registry.units);
    let stats = &unit.modified;
    let unchanged = &unit.get_base_stats(&registry.units);
    let locale = &registry.locale;
    strings.push(format!(
        "{}: {}/{}",
        locale.get("unitstats_hp"),
        unchanged.hp,
        stats.max_hp
    ));
    if stats.damage.hand > 0 {
        strings.push(if unit.modify.damage.hand != Modify::default() {
            format!(
                "{}: {} + {}",
                locale.get("unitstats_attack_hand"),
                unchanged.damage.hand,
                stats.damage.hand as i64 - unchanged.damage.hand as i64
            )
        } else {
            format!(
                "{}: {}",
                locale.get("unitstats_attack_hand"),
                unchanged.damage.hand
            )
        });
    }
    if stats.damage.ranged > 0 {
        strings.push(if unit.modify.damage.ranged != Modify::default() {
            format!(
                "{}: {} + {}",
                locale.get("unitstats_attack_ranged"),
                unchanged.damage.ranged,
                stats.damage.ranged as i64 - unchanged.damage.ranged as i64
            )
        } else {
            format!(
                "{}: {}",
                locale.get("unitstats_attack_ranged"),
                unchanged.damage.ranged
            )
        })
    }
    if stats.damage.magic > 0 {
        let magic = stats.damage.magic;
        if let Some(magic_type) = info.magic_type {
			let dir = info.magic_direction;
            let heal = matches!(
                dir,
                MagicDirection::CureOnly | MagicDirection::ToAll | MagicDirection::ToAlly
            );
            let bless = matches!(
                dir,
                MagicDirection::BlessOnly | MagicDirection::ToAll | MagicDirection::ToAlly
            );
            let curse = matches!(
                dir,
                MagicDirection::CurseOnly | MagicDirection::ToEnemy | MagicDirection::ToAll
            );
            let strike = matches!(
                dir,
                MagicDirection::StrikeOnly | MagicDirection::ToAll | MagicDirection::ToEnemy
            );

            let mut add_attack = 0;
            let mut add_defence = 0;
            let mut add_moves = 0;
            let mut add_ini = 0;

            let mut minus_attack = 0;
            let mut minus_defence = 0;
            let mut minus_moves = 0;
            let mut minus_ini = 0;

            let add_hp;
            let damage;

            match magic_type {
                MagicType::Death => {
                    add_attack = 1 + magic / 6;
                    add_defence = magic / 12;
                    minus_defence = magic / 10;
                    minus_attack = 1 + magic / 5;
                    add_hp = magic;
                    damage = magic;
                }
                MagicType::Life => {
                    add_attack = magic / 8;
                    add_defence = 1 + magic / 4;
                    minus_defence = 1 + magic / 3;
                    minus_attack = magic / 10;
                    add_hp = magic;
                    damage = magic;
                }
                MagicType::Elemental => {
                    add_moves = match magic {
                        0..=19 => 0,
                        20..=44 => 1,
                        45..=99 => 2,
                        100..=255 => 3,
                        x => x as i64 / 64,
                    };
                    add_ini = 1 + magic / 7;
                    minus_ini = 1 + magic / 7;
                    minus_moves = add_moves;
                    add_hp = magic / 2;
                    damage = magic / 2;
                }
            }
            if bless && add_attack > 0 {
                strings.push(format!(
                    "{}: +{}",
                    locale.get("unitstats_add_attack"),
                    add_attack
                ));
            };
            if bless && add_defence > 0 {
                strings.push(format!(
                    "{}: +{}",
                    locale.get("unitstats_add_defence"),
                    add_defence
                ));
            }
            if bless && add_moves > 0 {
                strings.push(format!(
                    "{}: +{}",
                    locale.get("unitstats_add_moves"),
                    add_moves
                ));
            };
            if bless && add_ini > 0 {
                strings.push(format!("{}: +{}", locale.get("unitstats_add_ini"), add_ini));
            }
            if heal && add_hp > 0 {
                strings.push(format!("{}: {}", locale.get("unitstats_add_hp"), add_hp));
            }
            if curse && minus_attack > 0 {
                strings.push(format!(
                    "{}: -{}",
                    locale.get("unitstats_minus_attack"),
                    minus_attack
                ));
            }
            if curse && minus_defence > 0 {
                strings.push(format!(
                    "{}: -{}",
                    locale.get("unitstats_minus_defence"),
                    minus_defence
                ));
            }
            if curse && minus_moves > 0 {
                strings.push(format!(
                    "{}: -{}",
                    locale.get("unitstats_minus_moves"),
                    minus_moves
                ));
            }
            if curse && minus_ini > 0 {
                strings.push(format!(
                    "{}: -{}",
                    locale.get("unitstats_minus_ini"),
                    minus_ini
                ));
            }
            if strike && damage > 0 {
                strings.push(format!("{}: {}", locale.get("unitstats_minus_hp"), damage));
            }
        }
    }
    let defence = stats.defence;
    if stats.defence.hand_units > 0 {
        strings.push(if unit.modify.defence.hand_units != Modify::default() {
            format!(
                "{}: {} + {}",
                locale.get("unitstats_defence_hand"),
                unchanged.defence.hand_units,
                defence.hand_units as i64 - unchanged.defence.hand_units as i64
            )
        } else {
            format!(
                "{}: {}",
                locale.get("unitstats_defence_hand"),
                unchanged.defence.hand_units
            )
        });
    }
    if stats.defence.ranged_units > 0 {
        strings.push(if unit.modify.defence.ranged_units != Modify::default() {
            format!(
                "{}: {} + {}",
                locale.get("unitstats_defence_ranged"),
                unchanged.defence.ranged_units,
                defence.ranged_units as i64 - unchanged.defence.ranged_units as i64
            )
        } else {
            format!(
                "{}: {}",
                locale.get("unitstats_defence_ranged"),
                unchanged.defence.ranged_units
            )
        });
    }
    if stats.defence.magic_units > 0 {
        strings.push(if unit.modify.defence.magic_units != Modify::default() {
            format!(
                "{}: {} + {}",
                locale.get("unitstats_defence_magic_units"),
                unchanged.defence.magic_units,
                defence.magic_units as i64 - unchanged.defence.magic_units as i64
            )
        } else {
            format!(
                "{}: {}",
                locale.get("unitstats_defence_magic_units"),
                unchanged.defence.magic_units
            )
        });
    }
    if defence.death_magic == defence.life_magic
        && defence.life_magic == defence.elemental_magic
        && defence.elemental_magic != 0
    {
        strings.push(format!(
            "{}: {}%",
            locale.get("unitstats_defence_magic"),
            defence.death_magic.get()
        ));
    } else {
        if defence.death_magic.get() != 0 {
            strings.push(format!(
                "{}: {}%",
                locale.get("unitstats_defence_magic_death"),
                defence.death_magic.get()
            ));
        }
        if defence.life_magic.get() != 0 {
            strings.push(format!(
                "{}: {}%",
                locale.get("unitstats_defence_magic_life"),
                defence.life_magic.get()
            ));
        }
        if defence.elemental_magic.get() != 0 {
            strings.push(format!(
                "{}: {}%",
                locale.get("unitstats_defence_magic_elemental"),
                defence.elemental_magic.get()
            ));
        }
    }
    if stats.vamp.get() != 0 {
        strings.push(format!("{}: {}", locale.get("unitstats_vamp"), stats.vamp));
    }
    if stats.vamp.get() != 0 {
        strings.push(format!(
            "{}: {}",
            locale.get("unitstats_regen"),
            stats.regen
        ));
    }
    strings.push(format!(
        "{}: {}",
        locale.get("unitstats_speed"),
        stats.speed
    ));
    strings.push(format!(
        "{}: {}/{}",
        locale.get("unitstats_moves"),
        unchanged.moves,
        stats.max_moves
    ));
    // let bonus = get_bonus_info(unit.get_bonus());
    // strings.push(bonus.0);
    // strings.push(bonus.1);
    strings
}

pub fn calclate_unit_power(unit: &Unit) -> f32 {
    let stats = unit.modified;
    let health = stats.max_hp as f32;
    let defence = stats.defence;
    let (hand_defence, ranged_defence) = (defence.hand_units as f32, defence.ranged_units as f32);
    let (death_defence, elemental_defence, life_defence) = (
        defence.death_magic.get() as f32,
        defence.elemental_magic.get() as f32,
        defence.life_magic.get() as f32,
    );
    let damage = stats.damage;
    let (hand_damage, ranged_damage, magic_damage) = (
        damage.hand as f32,
        damage.ranged as f32,
        damage.magic as f32,
    );
    let speed = stats.speed as f32;
    let moves = stats.max_moves as f32;
    let regen = stats.regen.get() as f32;
    let vamp = stats.vamp.get() as f32;
    /*
    ((атака/45+инициатива/20)/2*ходы+(защита/10)+(маг смерти/100+маг жизни/100*2+маг стихий/100)*2+реген/100*2+вампиризм/100*3 + health / 50) * множ бонуса
     */
    let health_points = health / 50.;
    let attack_points = hand_damage / 45. + ranged_damage / 45. + magic_damage / 30.;
    let speed_points = speed / 20.;
    let moves_modifier = moves;
    let defence_points = hand_defence / 10. + ranged_defence / 10.;
    let magic_points =
        death_defence / 100. + elemental_defence / 100. * 2. + life_defence / 100. * 2.;

    let regen_points = regen / 100.;
    let vamp_points = vamp / 100.;

    let (bonus_defence_modifier, bonus_attack_modifier, bonus_health_modifier) = (1., 1., 1.);
	//match unit.bonus {
    //     Bonus::SpearDefence => (1.5, 1., 1.),
    //     Bonus::GodAnger => (1., 1.1, 1.),
    //     Bonus::GodStrike => (1., 1.2, 1.),
    //     Bonus::AncientVampiresGist => (1., 1. + attack_points, 1.3),
    //     Bonus::Artillery => (1., 2. + attack_points, 1.),
    //     Bonus::Berserk => (1., 1.5, 1.),
    //     Bonus::Block => (1.3, 1., 1.),
    //     Bonus::DeadDodging | Bonus::Dodging => (1., 1., 1.3),
    //     Bonus::Fast => (1., 2., 1.),
    //     Bonus::DeadRessurect => (1., 1., 1.25),
    //     Bonus::FireAttack | Bonus::PoisonAttack => (1., 1.3, 1.),
    //     Bonus::Ghost => (1., 1.5, 2.),
    //     Bonus::Invulnerable => (1., 1., 2.),
    //     Bonus::DefencePiercing => (1., 1. + attack_points, 1.),
    //     Bonus::FastDead => (1., 2., 1.),
    //     Bonus::FlankStrike => (1., 1.5, 1.),
    //     Bonus::Garrison => (1.5, 1.5, 1.5),
    //     Bonus::Stealth => (1., 1.5, 1.),
    //     _ => (1., 1., 1.),
    // };
    (1.75_f32.powf(attack_points).max(attack_points) * bonus_attack_modifier + speed_points) / 2.
        * moves_modifier
        + (defence_points * bonus_defence_modifier)
        + magic_points * 2.
        + regen_points * 2.
        + vamp_points * 3.
        + health_points * bonus_health_modifier
}
