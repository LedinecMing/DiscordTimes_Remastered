use crate::{
    battle::{
        army::{MAX_LINES, MAX_TROOPS},
        battlefield::{field_type, BattleInfo, Field}, HitMap,
    },
    parse::LOCALE, units::unitstats::Modify,
};
use advini::Sections;
use num::{abs, Signed};
use once_cell::sync::Lazy;

use super::unitstats::ModifyUnitStats;
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
    fmt::{Debug, Display, Formatter}, ops::{Div, Mul}, sync::RwLock
};

#[derive(Copy, Clone, Debug, Add, Sub, Default, Sections)]
#[alkahest(Deserialize, Serialize, SerializeRef, Formula)]
pub struct Defence {
    pub death_magic: Percent,
    pub elemental_magic: Percent,
    pub life_magic: Percent,
    pub hand_percent: Percent,
    pub ranged_percent: Percent,
    pub magic_units: u64,
    pub hand_units: u64,
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
    pub fn empty() -> Self {
        Self {
            death_magic: Percent::new(0),
            elemental_magic: Percent::new(0),
            life_magic: Percent::new(0),
            hand_percent: Percent::new(0),
            ranged_percent: Percent::new(0),
            magic_units: 0,
            hand_units: 0,
            ranged_units: 0,
        }
    }
}

#[derive(Copy, Clone, Debug, Add, Sub, Default, Sections)]
#[alkahest(Deserialize, Serialize, SerializeRef, Formula)]
pub struct Power {
    pub magic: u64,
    pub ranged: u64,
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
    pub fn empty() -> Self {
        Self {
            magic: 0,
            ranged: 0,
            hand: 0,
        }
    }
}

#[derive(Copy, Clone, Debug, Add, Sub, Default, Sections)]
#[alkahest(Deserialize, Serialize, SerializeRef, Formula)]
pub struct UnitStats {
    pub hp: i64,
    pub max_hp: i64,
    #[inline_parsing]
    pub damage: Power,
    #[inline_parsing]
    pub defence: Defence,
    pub moves: i64,
    pub max_moves: i64,
    pub speed: i64,
    pub vamp: Percent,
    pub regen: Percent,
}
impl UnitStats {
    pub fn empty() -> Self {
        Self {
            hp: 0,
            max_hp: 0,
            damage: Power::empty(),
            defence: Defence::empty(),
            moves: 0,
            max_moves: 0,
            speed: 0,
            vamp: Percent::new(0),
            regen: Percent::new(0),
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Ini)]
#[alkahest(Deserialize, Serialize, SerializeRef, Formula)]
pub enum MagicDirection {
    ToAlly,
    ToAll,
    ToEnemy,
    CurseOnly,
    StrikeOnly,
    BlessOnly,
    CureOnly,
}
#[derive(Copy, Clone, Debug, PartialEq)]
#[alkahest(Deserialize, Serialize, SerializeRef, Formula)]
pub enum MagicType {
    Life,
    Death,
    Elemental,
}

#[derive(Clone, Debug)]
#[alkahest(Deserialize, Serialize, SerializeRef, Formula)]
pub struct LevelUpInfo {
    pub stats: ModifyUnitStats,
    pub xp_up: i16,
    pub max_xp: u64,
}
impl LevelUpInfo {
    pub fn empty() -> Self {
        Self {
            stats: ModifyUnitStats::default(),
            xp_up: 0,
            max_xp: 0,
        }
    }
}

#[derive(Clone, Debug)]
#[alkahest(Deserialize, Serialize, SerializeRef, Formula)]
pub struct UnitInfo {
    pub name: String,
    pub descript: String,
    pub cost: u64,
    pub cost_hire: u64,
    pub icon_index: usize,
    pub size: (usize, usize),
    pub unit_type: UnitType,
    pub next_unit: Vec<usize>,
    pub magic_info: Option<(MagicType, MagicDirection)>,
    pub surrender: Option<u64>,
    pub lvl: LevelUpInfo,
}
impl UnitInfo {
    pub fn empty() -> Self {
        Self {
            name: "".into(),
            descript: "".into(),
            cost: 0,
            cost_hire: 0,
            icon_index: 0,
            size: (1, 1),
            unit_type: UnitType::People,
            next_unit: Vec::new(),
            magic_info: None,
            surrender: None,
            lvl: LevelUpInfo::empty(),
        }
    }
}

#[derive(Clone, Debug)]
#[alkahest(Deserialize, Serialize, SerializeRef, Formula)]
pub struct UnitInventory {
    pub items: Vec<Option<Item>>,
}
impl UnitInventory {
    pub fn empty() -> Self {
        Self { items: vec![] }
    }
}

#[derive(Clone, Debug)]
#[alkahest(Deserialize, Serialize, SerializeRef, Formula)]
pub struct UnitLvl {
    pub lvl: u64,
    pub max_xp: u64,
    pub xp: u64,
}
impl UnitLvl {
    pub fn empty() -> Self {
        Self {
            lvl: 0,
            max_xp: 0,
            xp: 0,
        }
    }
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
        let max_troops = *MAX_TROOPS / MAX_LINES;
        Self(index % max_troops, index / max_troops)
    }
	pub fn whole(&self) -> usize {
		self.0 + self.1 * *MAX_TROOPS / MAX_LINES
	}
}
impl Into<(usize, usize)> for UnitPos {
    fn into(self) -> (usize, usize) {
        (self.0, self.1)
    }
}
impl Into<usize> for UnitPos {
    fn into(self) -> usize {
        self.0 + self.1 * *MAX_TROOPS / MAX_LINES
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
#[alkahest(Deserialize, Serialize, SerializeRef, Formula)]
pub enum UnitType {
    People,
    Hero,
    Animal,
    Mecha,
    Undead,
    Rogue,
}
/// Used for parsing
pub static UNITS: Lazy<RwLock<Vec<Unit>>> = Lazy::new(|| RwLock::new(vec![]));
#[derive(Clone, Debug)]
#[alkahest(Deserialize, Serialize, SerializeRef, Formula)]
pub struct Unit {
    pub stats: UnitStats,
    //#[unused]
    pub modified: UnitStats,
    //#[unused]
    pub modify: ModifyUnitStats,
    pub info: UnitInfo,
    pub lvl: UnitLvl,
    pub inventory: UnitInventory,
    pub army: usize,
    pub bonus: Bonus,
    //#[unused]
    pub effects: Vec<Effect>,
}
impl Ini for Unit {
    fn eat(chars: std::str::Chars) -> Result<(Self, std::str::Chars), IniParseError> {
        let units = &UNITS.read().unwrap();
        let (unit_id, chars) = String::eat(chars)?;
        let Some(unit) = (match unit_id.parse::<usize>() {
            Ok(id) => units.get(id),
            Err(_) => units.iter().find(|x| x.info.name == unit_id),
        }) else {
            return Err(IniParseError::Error("No such unit"));
        };
        Ok((unit.clone(), chars))
    }
    fn vomit(&self) -> String {
        UNITS
            .read()
            .unwrap()
            .iter()
            .enumerate()
            .find(|x| x.1.info.name == self.info.name)
            .unwrap()
            .0
            .vomit()
    }
}
fn heal_unit(
    me: &mut Unit,
    unit: &mut Unit,
    damage: Power,
    magic_type: MagicType,
) -> Option<ActionResult> {
    return match (unit.info.unit_type, magic_type) {
        (UnitType::Rogue | UnitType::Hero | UnitType::People, Death) => None,
        (UnitType::Undead, Life) => None,
        _ => {
            if unit.heal(damage.magic) {
                Some(ActionResult::Buff)
            } else {
                None
            }
        }
    };
}
fn bless_unit(
    me: &mut Unit,
    target: &mut Unit,
    damage: Power,
    magic_type: MagicType,
) -> Option<ActionResult> {
    match (target.info.unit_type, magic_type) {
        (UnitType::Rogue | UnitType::Hero | UnitType::People, Death) => None,
        (UnitType::Undead, Life) => None,
        (UnitType::Mecha, _) => None,
        _ => {
            if !target.has_effect_kind(EffectKind::MageSupport) {
                target.add_effect(HealMagic::new(damage.magic));
                return Some(ActionResult::Buff);
            }
            None
        }
    }
}
fn heal_bless(
    me: &mut Unit,
    target: &mut Unit,
    damage: Power,
    magic_type: MagicType,
) -> Option<ActionResult> {
    match (target.info.unit_type, magic_type) {
        (UnitType::Rogue | UnitType::Hero | UnitType::People, Death) => None,
        (UnitType::Undead, Life) => None,
        (UnitType::Mecha, Death | Life) => None,
        _ => {
            if heal_unit(me, target, damage, magic_type).is_some() {
                return bless_unit(me, target, damage, magic_type);
            }
            None
        }
    }
}

fn elemental_bless(me: &mut Unit, target: &mut Unit, damage: Power) -> Option<ActionResult> {
    if !target.has_effect_kind(EffectKind::MageSupport) {
        target.add_effect(ElementalSupport::new(damage.magic));
        return Some(ActionResult::Debuff);
    };
    None
}

fn magic_curse(
    me: &mut Unit,
    target: &mut Unit,
    mut damage: Power,
    magic_type: MagicType,
) -> Option<ActionResult> {
    match (target.info.unit_type, magic_type) {
        (UnitType::Undead, Life) => {
            damage.magic *= 2;
        }
        _ => {}
    }
    if !target.has_effect_kind(EffectKind::MageCurse) {
        target.add_effect(AttackMagic::new(
            target.correct_damage(&damage, Some(magic_type), None).magic,
        ));
        Some(ActionResult::Debuff)
    } else {
        None
    }
}

fn elemental_curse(me: &mut Unit, target: &mut Unit, damage: Power, magic_type: MagicType) -> Option<ActionResult> {
    if !target.has_effect_kind(EffectKind::MageCurse) {
        target.add_effect(DisableMagic::new(
            target.correct_damage(&damage, Some(magic_type), None).magic,
        ));
        Some(ActionResult::Debuff)
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
) -> Option<ActionResult> {
    match (target.info.unit_type, magic_type) {
        (UnitType::Undead, Life) => {
            damage.magic *= 2;
        }
        _ => {}
    }
    if let Some(res) = magic_curse(me, target, damage, magic_type) {
		Some(res)
	} else {
        damage.hand = 0;
        damage.ranged = 0;
        let res = target.being_attacked(&damage, me, target_pos, hitmap, my_pos, battle);
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
) -> Option<ActionResult> {
    let mut damage = damage;
    if !elemental_curse(me, target, damage, magic_type).is_some() {
        damage.hand = 0;
        damage.ranged = 0;
        target.being_attacked(&damage, me, target_pos, hitmap, my_pos, battle);
    }
    Some(ActionResult::Debuff)
}

#[derive(Debug)]
pub enum ActionResult {
    Buff,
    Debuff,
    Melee,
    Ranged,
    Move,
}
impl Unit {
    pub fn recalc(&mut self) {
        self.modified = self.modify.apply(&self.stats);
    }
    pub fn new(
        stats: UnitStats,
        info: UnitInfo,
        lvl: UnitLvl,
        inventory: UnitInventory,
        army: usize,
        bonus: Bonus,
        effects: Vec<Effect>,
    ) -> Self {
        let mut army = Self {
            stats,
            modified: stats,
            info,
            lvl,
            inventory,
            effects,
            army,
            modify: ModifyUnitStats::default(),
            bonus,
        };
        army.recalc();
        army
    }
    pub fn can_attack(&self, target: &Unit, target_pos: UnitPos, my_pos: UnitPos, is_enemy: bool, target_hitmap: &Vec<Option<usize>>) -> bool {
        let effected = self.modified;
        let my_field = field_type(my_pos.into(), *MAX_TROOPS);
        let is_in_back = my_field == Field::Back;
        let enemy_field = field_type(target_pos.into(), *MAX_TROOPS);
        let damage = effected.damage;
        let enemy_in_reserve = enemy_field == Field::Reserve;
        let me_in_reserve = my_field == Field::Reserve;
        let both_in_reserve = me_in_reserve && enemy_in_reserve;
        let both_not_in_reserve = !me_in_reserve && !enemy_in_reserve;
        /*
         * C(`B`(`A`|D)) | `C`(`A^B`)
         */
        let can_attack = (is_enemy
            && (!me_in_reserve || self.get_bonus().can_attack_from_reserve())
            && !enemy_in_reserve)
            || (!is_enemy && (both_not_in_reserve || both_in_reserve));
		if !can_attack { return false; }
		if damage.ranged > 0
            && (target_pos.1 == my_pos.1 && abs(target_pos.0 as i64 - my_pos.0 as i64) < 2
                || my_field == Field::Back)
            && is_enemy
        {
            true
        } else if damage.hand > 0 && !is_in_back && target_pos.1 == 1 && is_enemy && (target_pos.0.abs_diff(my_pos.0) < 2 || ((my_pos.whole() - 1)..(target_pos.whole())).all(|pos| target_hitmap[pos].is_none() || field_type(pos, *MAX_TROOPS) != Field::Front)) {
			// This checks that receiver is in front and there are no obstacles between me and him.
            true
        } else {
            match self.info.magic_info {
                None => false,
                Some((magic_type, magic_direction)) => {
                    match (magic_direction, magic_type, is_enemy) {
                        (ToAlly, _, false) => match magic_type {
                            Death | Life => match (target.info.unit_type, magic_type) {
                                (UnitType::Rogue | UnitType::Hero | UnitType::People, Death) => {
                                    false
                                }
                                (UnitType::Undead, Life) => false,
                                _ => !target.has_effect_kind(EffectKind::MageSupport),
                            },
                            Elemental => !target.has_effect_kind(EffectKind::MageSupport),
                        },
                        (ToAll, _, _) => match (magic_type, is_enemy) {
                            (Death | Life, true) => is_in_back,
                            (Death | Life, false) => {
                                match (target.info.unit_type, magic_type) {
                                    (
                                        UnitType::Rogue | UnitType::Hero | UnitType::People,
                                        Death,
                                    ) => false,
                                    (UnitType::Undead, Life) => false,
                                    (UnitType::Mecha, Death | Life) => false,
                                    _ => !target.has_effect_kind(EffectKind::MageSupport),
                                }
                            }
                            (Elemental, true) => is_in_back,
                            (Elemental, false) => {
                                !target.has_effect_kind(EffectKind::MageSupport)
                            }
                        },
                        (ToEnemy, _, true) => match magic_type {
                            Death | Life => is_in_back,
                            Elemental => is_in_back,
                        },
                        (CurseOnly, _, true) => match magic_type {
                            Death | Life => is_in_back,
                            Elemental => is_in_back,
                        },
                        (StrikeOnly, _, true) => is_in_back,
                        (BlessOnly, _, false) => match magic_type {
                            Life | Death => match (target.info.unit_type, magic_type) {
                                (UnitType::Rogue | UnitType::Hero | UnitType::People, Death) => {
                                    false
                                }
                                (UnitType::Undead, Life) => false,
                                (UnitType::Mecha, Death | Life) => false,
                                _ => !target.has_effect_kind(EffectKind::MageSupport),
                            },
                            Elemental => !target.has_effect_kind(EffectKind::MageSupport),
                        },
                        (CureOnly, Life | Death, false) => {
                            match (target.info.unit_type, magic_type) {
                                (UnitType::Rogue | UnitType::Hero | UnitType::People, Death) => {
                                    false
                                }
                                (UnitType::Undead, Life) => false,
                                (UnitType::Mecha, Death | Life) => false,
                                _ => !target.stats.hp == target.stats.max_hp,
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
		is_enemy: bool
    ) -> Option<ActionResult> {
		
        let effected = self.modified;
        let my_field = field_type(my_pos.into(), *MAX_TROOPS);
        let is_in_back = my_field == Field::Back;
        let enemy_field = field_type(target_pos.into(), *MAX_TROOPS);
        let mut damage = effected.damage;
        let enemy_in_reserve = enemy_field == Field::Reserve;
        let me_in_reserve = my_field == Field::Reserve;
        let both_in_reserve = me_in_reserve && enemy_in_reserve;
        let both_not_in_reserve = !me_in_reserve && !enemy_in_reserve;
        let a = (is_enemy
            && (!me_in_reserve || self.bonus.can_attack_from_reserve())
            && !enemy_in_reserve)
            || (!is_enemy && (both_not_in_reserve || both_in_reserve));
        return if !a {
            None
		} else if damage.hand > 0 && !is_in_back && target_pos.1 == 1 && is_enemy && (target_pos.0.abs_diff(my_pos.0) < 2 || ((my_pos.whole() - 1)..(target_pos.whole())).all(|pos| target_hitmap[pos].is_none() || field_type(pos, *MAX_TROOPS) != Field::Front)) {
            damage.ranged = 0;
            damage.magic = 0;
            let _ = target.being_attacked(&damage, self, target_pos, target_hitmap, my_pos, battle);
            Some(ActionResult::Melee)
        }  else if damage.ranged > 0
            && (target_pos.1 == my_pos.1 && abs(target_pos.0 as i64 - my_pos.0 as i64) < 2
                || my_field == Field::Back)
            && is_enemy
        {
            damage.hand = 0;
            damage.magic = 0;
            let _ = target.being_attacked(&damage, self, target_pos, target_hitmap, my_pos, battle);
            Some(ActionResult::Ranged)
        } else {
			let Some((magic_type, direction)) = self.info.magic_info else { return None; };
            match (direction, magic_type, is_enemy) {
                (ToAlly, _, false) => match magic_type {
                    Death | Life => heal_bless(self, target, damage, magic_type),
                    Elemental => elemental_bless(self, target, damage),
                },
                (ToAll, _, _) => match (magic_type, is_enemy) {
                    (Death | Life, true) => {
                        if is_in_back {
                            magic_attack(
                                self, target, damage, magic_type, my_pos, target_pos,
                                battle, target_hitmap
                            )
                        } else {
                            None
                        }
                    }
                    (Death | Life, false) => {
                        heal_bless(self, target, damage, magic_type)
                    }
                    (Elemental, true) => {
                        if is_in_back {
                            elemental_attack(
                                self, target, damage, target_pos, my_pos, battle, magic_type, target_hitmap
                            )
                        } else {
                            None
                        }
                    }
                    (Elemental, false) => elemental_bless(self, target, damage),
                },
                (ToEnemy, _, true) => {
                    if is_in_back {
                        match magic_type {
                            Death | Life => magic_attack(
                                self, target, damage, magic_type, my_pos, target_pos,
                                battle, target_hitmap
                            ),
                            Elemental => elemental_attack(
                                self, target, damage, target_pos, my_pos, battle, magic_type, target_hitmap
                            ),
                        }
                    } else {
                        None
                    }
                }
                (CurseOnly, _, true) => match magic_type {
                    Death | Life => magic_curse(self, target, damage, magic_type),
                    Elemental => elemental_curse(self, target, damage, magic_type),
                },
                (StrikeOnly, _, true) => {
                    damage.hand = 0;
                    damage.ranged = 0;
                    target.being_attacked(&damage, self, my_pos, target_hitmap, target_pos, battle);
                    Some(ActionResult::Debuff)
                }
                (BlessOnly, _, false) => match magic_type {
                    Life | Death => bless_unit(self, target, damage, magic_type),
                    Elemental => elemental_bless(self, target, damage),
                },
                (CureOnly, Life | Death, false) => {
                    heal_unit(self, target, damage, magic_type)
                }
                _ => None,
            }
        }
	}
	pub fn restore(&mut self) {
		self.stats.hp = self.modified.max_hp;
		self.stats.moves = self.modified.max_moves;
	}
    pub fn heal(&mut self, amount: u64) -> bool {
        let effected = self.modified;
        let amount = amount as i64;
        if effected.max_hp < effected.hp + amount {
            self.stats.hp = effected.max_hp;
            self.recalc();
            return true;
        }
        self.stats.hp += amount;
        self.recalc();
        return false;
    }

    pub fn get_effected_stats(&self) -> UnitStats {
        self.modified
    }
    pub fn is_dead(&self) -> bool {
        self.modified.hp < 1
    }
    pub fn has_effect_kind(&self, kind: EffectKind) -> bool {
        for effect in &self.effects {
            if effect.get_kind() == kind {
                return true;
            };
        }
        return false;
    }
    pub fn kill(&mut self) {
        self.stats.hp = -self.modified.hp;
        self.recalc();
    }
    pub fn add_effect(&mut self, effect: impl Into<Effect>) -> bool {
        self.effects.push(effect.into());
        self.effects.last_mut().unwrap().clone().update_stats(self);
        self.recalc();
        true
    }
    pub fn add_item(&mut self, item: Option<Item>, index: usize) -> bool {
        // if let Some(Some(_)) = self.inventory.items.get(index) {
        //     return false;
        //}
		let Some(item) = item else {
			self.inventory.items[index] = None;
			return true;
		};
        if item.can_equip(&*self) {
            self.modify += item.get_info().modify;
            self.inventory.items[index] = Some(item);
            self.recalc();
            true
        } else {
            false
        }
    }
    pub fn remove_item(&mut self, index: usize) {
        let item = self
            .inventory
            .items
            .remove(index)
            .expect("No such index for items");
        self.inventory.items.insert(index, None);
        let item_info = item.get_info();
        self.modify -= item_info.modify;
        self.recalc();
    }
	pub fn swap_item(&mut self, index: usize, add_item: Option<Item>) -> Option<Item> {
        let item = self
            .inventory
            .items
            .remove(index);
		if let Some(item) = item {
			let item_info = item.get_info();
			self.modify -= item_info.modify;
			self.recalc();
		}
		if let Some(item) = add_item {
			if item.can_equip(&self) {
				self.inventory.items.insert(index, add_item);
				self.modify += item.get_info().modify;
				self.recalc();
			} else {
				self.inventory.items.insert(index, None);
			}
		} else { self.inventory.items.insert(index, None); }
		item
    }
    pub fn get_bonus(&self) -> Bonus {
        let mut bonus = self.bonus;
        if let Some(item) = self
            .inventory
            .items
            .iter()
            .filter(|item| {
                if let Some(item) = item {
                    item.get_info().bonus.is_some()
                } else {
                    false
                }
            })
            .last()
        {
            if let Some(item_bonus) = (&item.unwrap()).get_info().bonus {
                bonus = item_bonus.clone();
            };
        };
        bonus.clone()
    }
    pub fn being_attacked(
        &mut self,
        damage: &Power,
        sender: &mut Unit,
        my_pos: UnitPos,
		my_hitmap: &HitMap,
        attacker_pos: UnitPos,
        battle: &BattleInfo,
    ) -> u64 {
		let is_flank_attack = my_pos.0.abs_diff(attacker_pos.0) > 1;
		let flank_info = if is_flank_attack {
			Some(1.)
		} else { None };
        let corrected_damage = self.correct_damage(damage, sender.info.magic_info.and_then(|x| Some(x.0)), flank_info);
        let unit_bonus = sender.get_bonus();
        let corrected_damage =
            unit_bonus.on_attacking(corrected_damage, self, sender, my_pos, attacker_pos);
        let corrected_damage = self.get_bonus().on_attacked(
            corrected_damage,
            self,
			my_hitmap,
            sender,
            my_pos,
            attacker_pos,
            battle,
        );
        let corrected_damage_units = 
            (corrected_damage.magic + corrected_damage.ranged + corrected_damage.hand).max(1);
		self.stats.hp = self.stats.hp.abs_sub(&(corrected_damage_units as i64));
        self.modified.hp = self.modify.hp.apply(self.stats.hp);
		match self.info.unit_type {
			UnitType::Undead | UnitType::Mecha => {},
			_ => {
				sender.heal(sender.modified.vamp.calc(corrected_damage_units)); // apply vampirizm
			}
		}
        corrected_damage_units
    }
    pub fn correct_damage(&self, damage: &Power, magic_type: Option<MagicType>, flank_modifier: Option<f32>) -> Power {
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
		} else { defence.hand_units };
        Power {
            ranged: (percent_100 - defence.ranged_percent)
                .calc(damage.ranged.saturating_sub(defence.ranged_units)),
            magic,
            hand: (percent_100 - defence.hand_percent)
                .calc(damage.hand.saturating_sub(hand_defence)),
        }
    }
    pub fn tick(&mut self) -> bool {
		let mut effects = self.effects.clone();
		let removed: Vec<_> = effects.extract_if(|ef| ef.on_tick(self) && ef.is_dead()).collect();
		self.effects = effects;
        for mut effect in removed {
			effect.kill(self);
		}
		self.heal(self.modified.regen.calc(self.modified.max_hp as u64)); // apply regeneration
        self.get_bonus().on_tick(self);
        self.recalc();
        true
    }
}
fn get_bonus_info(bonus: Bonus) -> (String, String) {
    let locale_ids = bonus.locale_id();
    let locale = LOCALE.read().unwrap();
    (locale.get(locale_ids.0), locale.get(locale_ids.1))
}
pub fn display_unit(unit: &Unit) -> Vec<String>{
	let mut strings = vec![];
	let stats = &unit.modified;
	let unchanged = &unit.stats;
	let locale = LOCALE.read().unwrap();
	strings.push(format!("{}: {}/{}", locale.get("unitstats_hp"), stats.hp, stats.max_hp));
	if stats.damage.hand > 0 {
		strings.push(if unit.modify.damage.hand != Modify::default() {
			format!("{}: {} + {}", locale.get("unitstats_attack_hand"), unchanged.damage.hand, stats.damage.hand - unchanged.damage.hand)
		} else {
			format!("{}: {}", locale.get("unitstats_attack_hand"), unchanged.damage.hand)
		});
	}
	if stats.damage.ranged > 0 {
		strings.push(if unit.modify.damage.ranged != Modify::default() {
			format!("{}: {} + {}", locale.get("unitstats_attack_ranged"), unchanged.damage.ranged, stats.damage.ranged - unchanged.damage.ranged)
		} else {
			format!("{}: {}", locale.get("unitstats_attack_ranged"), unchanged.damage.ranged)
		})
	}
	if stats.damage.magic > 0 {
		let magic = stats.damage.magic;
		if let Some((magic_type, dir)) = unit.info.magic_info {
			let heal = matches!(dir, MagicDirection::CureOnly | MagicDirection::ToAll | MagicDirection::ToAlly);
			let bless = matches!(dir, MagicDirection::BlessOnly | MagicDirection::ToAll | MagicDirection::ToAlly);
			let curse = matches!(dir, MagicDirection::CurseOnly | MagicDirection::ToEnemy | MagicDirection::ToAll);
			let strike = matches!(dir, MagicDirection::StrikeOnly | MagicDirection::ToAll | MagicDirection::ToEnemy);

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
					damage = magic * 2;
				},
				MagicType::Life => {
					add_attack = magic / 8;
					add_defence = 1 + magic / 4;
					minus_defence = 1 + magic / 3;
					minus_attack = magic / 10;
					add_hp = magic;
					damage = magic;
				},
				MagicType::Elemental => {
					add_moves = match magic {
						0..=19 => 0,
						20..=44 => 1,
						45..=99 => 2,
						100..=255 => 3,
						x => x as i64 / 64
					};
					add_ini = 1 + magic / 7;
					minus_ini = 1 + magic / 7;
					minus_moves = add_moves;
					add_hp = magic / 2;
					damage = magic / 2;
				}
			}
			if bless && add_attack > 0 {
				strings.push(format!("{}: +{}", locale.get("unitstats_add_attack"), add_attack));
			};
			if bless && add_defence > 0 {
				strings.push(format!("{}: +{}", locale.get("unitstats_add_defence"), add_defence));
			}
			if bless && add_moves > 0 {
				strings.push(format!("{}: +{}", locale.get("unitstats_add_moves"), add_moves));
			};
			if bless && add_ini > 0 {
				strings.push(format!("{}: +{}", locale.get("unitstats_add_ini"), add_ini));
			}
			if heal && add_hp > 0 {
				strings.push(format!("{}: {}", locale.get("unitstats_add_hp"), add_hp));
			}
			if curse && minus_attack > 0 {
				strings.push(format!("{}: -{}", locale.get("unitstats_minus_attack"), minus_attack));
			}
			if curse && minus_defence > 0 {
				strings.push(format!("{}: -{}", locale.get("unitstats_minus_defence"), minus_defence));
			}
			if curse && minus_moves > 0 {
				strings.push(format!("{}: -{}", locale.get("unitstats_minus_moves"), minus_moves));
			}
			if curse && minus_ini > 0 {
				strings.push(format!("{}: -{}", locale.get("unitstats_minus_ini"), minus_ini));
			}
			if strike && damage > 0 {
				strings.push(format!("{}: {}", locale.get("unitstats_minus_hp"), damage));
			}
		}
	}
	let defence = stats.defence;
	if stats.defence.hand_units > 0 {
		strings.push(if unit.modify.defence.hand_units != Modify::default() {
			format!("{}: {} + {}", locale.get("unitstats_defence_hand"), unchanged.defence.hand_units, defence.hand_units - unchanged.defence.hand_units)
		} else {
			format!("{}: {}", locale.get("unitstats_defence_hand"), unchanged.defence.hand_units)
		});
	}
	if stats.defence.ranged_units > 0 {
		strings.push(if unit.modify.defence.ranged_units != Modify::default() {
			format!("{}: {} + {}", locale.get("unitstats_defence_ranged"), unchanged.defence.ranged_units, defence.ranged_units - unchanged.defence.ranged_units)
		} else {
			format!("{}: {}", locale.get("unitstats_defence_ranged"), unchanged.defence.ranged_units)
		});
	}
	if stats.defence.magic_units > 0 {
		strings.push(if unit.modify.defence.magic_units != Modify::default() {
			format!("{}: {} + {}", locale.get("unitstats_defence_magic_units"), unchanged.defence.magic_units, defence.magic_units - unchanged.defence.magic_units)
		} else {
			format!("{}: {}", locale.get("unitstats_defence_magic_units"), unchanged.defence.magic_units)
		});
	}
	if defence.death_magic == defence.life_magic && defence.life_magic == defence.elemental_magic && defence.elemental_magic != 0 {
		strings.push(format!("{}: {}%", locale.get("unitstats_defence_magic"), defence.death_magic.get()));
	}
	if stats.vamp.get() != 0 {
		strings.push(format!("{}: {}", locale.get("unitstats_vamp"), stats.vamp));
	}
	if stats.vamp.get() != 0 {
		strings.push(format!("{}: {}", locale.get("unitstats_regen"), stats.regen));
	}
	strings.push(format!("{}: {}", locale.get("unitstats_speed"), stats.speed));
	strings.push(format!("{}: {}/{}", locale.get("unitstats_moves"), stats.moves, stats.max_moves));
	let bonus = get_bonus_info(unit.get_bonus());
	strings.push(bonus.0);
	strings.push(bonus.1);
	strings
}
impl Display for Unit {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let bonus_info = get_bonus_info(self.bonus);
        let locale = LOCALE.read().unwrap();
        let stats = self.modified;
        let damage = stats.damage;
        let attack = format!(
            "{}: {}\n{}: {}\n{}: {}",
            locale.get("unitstats_attack_melee"),
            damage.hand,
            locale.get("unitstats_attack_ranged"),
            damage.ranged,
            locale.get("unitstats_attack_magic"),
            damage.magic
        );
        let defence = format!(
            "{}: {}\n{}: {}\n{} {}: {}, {}: {}, {}: {}",
            locale.get("unitstats_defence_melee"),
            stats.defence.hand_units,
            locale.get("unitstats_defence_ranged"),
            stats.defence.ranged_units,
            locale.get("unitstats_defence_magic"),
            locale.get("unitstats_defence_magic_death"),
            stats.defence.death_magic,
            locale.get("unitstats_defence_magic_life"),
            stats.defence.life_magic,
            locale.get("unitstats_defence_magic_elemental"),
            stats.defence.elemental_magic
        );
        let (magic_dir, magic_type) = match self.info.magic_info {
			Some((magic_type, magic_dir)) => {
				let magic_dir = match magic_dir {
					ToAll => locale.get("unitstats_magic_toall"),
					ToAlly => locale.get("unitstats_magic_toally"),
					ToEnemy => locale.get("unitstats_magic_toenemy"),
					StrikeOnly => locale.get("unitstats_magic_strikeonly"),
					BlessOnly => locale.get("unitstats_magic_blessonly"),
					CureOnly => locale.get("unitstats_magic_cureonly"),
					CurseOnly => locale.get("unitstats_magic_curseonly"),
				};
				let magic_type = match magic_type {
					Life => {
						locale.get("unitstats_magictype_life")
					}
					Death => {
						locale.get("unitstats_magictype_death")
					}
					Elemental => {
						locale.get("unitstats_magictype_elemental")
					}
				};
				(magic_dir, magic_type)
			},
			None => (locale.get("unitstats_empty"), locale.get("unitstats_empty"))
		};
        use UnitType::*;
        let unit_type = match self.info.unit_type {
            Undead => locale.get("unitstats_unittype_undead"),
            People => locale.get("unitstats_unittype_people"),
            Hero => locale.get("unitstats_unittype_hero"),
            Rogue => locale.get("unitstats_unittype_rogue"),
            Animal => locale.get("unitstats_unittype_animal"),
            Mecha => locale.get("unitstats_unittype_mecha"),
        }
        .to_string();
        let surrender = if self.info.surrender > Some(0) {
            locale
                .get("unitstats_giveup")
                .replace("{}", &*self.info.surrender.unwrap().to_string())
        } else {
            locale.get("unitstats_dont_giveup")
        };
        f.write_fmt(format_args!("{} | {} {}\n{}\n {}: {}\n{}: {}|{}: {}\n{}\n{}\n{}: {}\n{}: {}\n{}: {}\n{}: {}\n{}: {}\n{} - {} {}; {} {}\n{} {};{}\n{}: {}",
            self.info.name,
            self.lvl.max_xp,
            locale.get("unitstats_xp"),
            self.info.descript,
            locale.get("unitstats_hp"),
            stats.hp,
            locale.get("unitstats_magic"),
            magic_type,
            locale.get("unitstats_magic_dir"),
            magic_dir,
            attack,
            defence,
            locale.get("unitstats_vamp"),
            stats.vamp,
            locale.get("unitstats_regen"),
            stats.regen,
            locale.get("unitstats_moves"),
            stats.moves,
            locale.get("unitstats_speed"),
            stats.speed,
            locale.get("unitstats_unittype"),
            unit_type,
            locale.get("unitstats_cost"),
            self.info.cost_hire,
            locale.get("unitstats_cost_hire"),
            self.info.cost,
            locale.get("unitstats_cost_per_day"),
            "|",
            surrender,
            locale.get("unitstats_upgrade"),
            locale.get("unitstats_bonus"),
            format!("{} - {}", bonus_info.0, bonus_info.1)))
    }
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

    let (bonus_defence_modifier, bonus_attack_modifier, bonus_health_modifier) = match unit.bonus {
        Bonus::SpearDefence => (1.5, 1., 1.),
        Bonus::GodAnger => (1., 1.1, 1.),
        Bonus::GodStrike => (1., 1.2, 1.),
        Bonus::AncientVampiresGist => (1., 1. + attack_points, 1.3),
        Bonus::Artillery => (1., 2. + attack_points, 1.),
        Bonus::Berserk => (1., 1.5, 1.),
        Bonus::Block => (1.3, 1., 1.),
        Bonus::DeadDodging | Bonus::Dodging => (1., 1., 1.3),
        Bonus::Fast => (1., 2., 1.),
        Bonus::DeadRessurect => (1., 1., 1.25),
        Bonus::FireAttack | Bonus::PoisonAttack => (1., 1.3, 1.),
        Bonus::Ghost => (1., 1.5, 2.),
        Bonus::Invulnerable => (1., 1., 2.),
        Bonus::DefencePiercing => (1., 1. + attack_points, 1.),
        Bonus::FastDead => (1., 2., 1.),
        Bonus::FlankStrike => (1., 1.5, 1.),
        Bonus::Garrison => (1.5, 1.5, 1.5),
        Bonus::Stealth => (1., 1.5, 1.),
        _ => (1., 1., 1.),
    };

    (attack_points * bonus_attack_modifier + speed_points) / 2. * moves_modifier
        + (defence_points * bonus_defence_modifier)
        + magic_points * 2.
        + regen_points * 2.
        + vamp_points * 3.
        + health_points * bonus_health_modifier
}
