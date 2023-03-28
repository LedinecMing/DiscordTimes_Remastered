use crate::{
    lib::battle::{
        army::MAX_TROOPS,
        battlefield::{field_type, Field},
    },
    LOCALE,
};
use num::abs;

use super::unitstats::ModifyUnitStats;
use crate::lib::{
    bonuses::{bonus::Bonus, bonuses::*},
    effects::{
        effect::{Effect, EffectKind},
        effects::{AttackMagic, DisableMagic, ElementalSupport, HealMagic},
    },
    items::item::Item,
    units::unit::{MagicDirection::*, MagicType::*},
};
use derive_more::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Sub, SubAssign};
use math_thingies::Percent;
use std::fmt::{Debug, Display, Formatter};

#[derive(Copy, Clone, Debug, Add, Sub)]
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

#[derive(Copy, Clone, Debug, Add, Sub)]
pub struct Power {
    pub magic: u64,
    pub ranged: u64,
    pub hand: u64,
}
impl Power {
    pub fn empty() -> Self {
        Self {
            magic: 0,
            ranged: 0,
            hand: 0,
        }
    }
}

#[derive(Copy, Clone, Debug, Add, Sub)]
pub struct UnitStats {
    pub hp: i64,
    pub max_hp: i64,
    pub damage: Power,
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

#[derive(Copy, Clone, Debug, PartialEq)]
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
pub enum MagicType {
    Life(MagicDirection),
    Death(MagicDirection),
    Elemental(MagicDirection),
}

#[derive(Clone, Debug)]
pub struct LevelUpInfo {
    pub stats: UnitStats,
    pub xp_up: i16,
    pub max_xp: u64,
}
impl LevelUpInfo {
    pub fn empty() -> Self {
        Self {
            stats: UnitStats::empty(),
            xp_up: 0,
            max_xp: 0,
        }
    }
}

#[derive(Clone, Debug)]
pub struct UnitInfo {
    pub name: String,
    pub descript: String,
    pub cost: u64,
    pub cost_hire: u64,
    pub icon_index: usize,
    pub unit_type: UnitType,
    pub next_unit: Vec<String>,
    pub magic_type: Option<MagicType>,
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
            unit_type: UnitType::People,
            next_unit: Vec::new(),
            magic_type: None,
            surrender: None,
            lvl: LevelUpInfo::empty(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct UnitInventory {
    pub items: Vec<Option<Item>>,
}
impl UnitInventory {
    pub fn empty() -> Self {
        Self { items: vec![] }
    }
}

#[derive(Clone, Debug)]
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

#[derive(Clone, Debug)]
pub struct UnitData {
    pub stats: UnitStats,
    pub modify: ModifyUnitStats,
    pub info: UnitInfo,
    pub lvl: UnitLvl,
    pub inventory: UnitInventory,
    pub bonus: Box<dyn Bonus>,
    pub effects: Vec<Box<dyn Effect>>,
}

#[derive(Copy, Clone, Add, AddAssign, Sub, SubAssign, Mul, MulAssign, Div, DivAssign)]
pub struct UnitPos(pub usize, pub usize);
impl UnitPos {
    pub fn from_index(index: usize) -> Self {
        let max_troops = *MAX_TROOPS.lock().unwrap() / 2;
        Self(index % max_troops, index / max_troops)
    }
}
impl Into<(usize, usize)> for UnitPos {
    fn into(self) -> (usize, usize) {
        (self.0, self.1)
    }
}
impl Into<usize> for UnitPos {
    fn into(self) -> usize {
        self.0 + self.1 * *MAX_TROOPS.lock().unwrap() / 2
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum UnitType {
    People,
    Hero,
    Animal,
    Undead,
    Rogue,
}

#[derive(Clone, Debug)]
pub struct Unit {
    pub stats: UnitStats,
    pub modified: UnitStats,
    pub modify: ModifyUnitStats,
    pub info: UnitInfo,
    pub lvl: UnitLvl,
    pub inventory: UnitInventory,
    pub army: usize,
    pub bonus: Box<dyn Bonus>,
    pub effects: Vec<Box<dyn Effect>>,
}

fn heal_unit(me: &mut Unit, unit: &mut Unit, damage: Power, magic_type: MagicType) -> bool {
    return match (unit.info.unit_type, magic_type) {
        (UnitType::Rogue | UnitType::Hero | UnitType::People, Death(_)) => false,
        (UnitType::Undead, Life(_)) => false,
        _ => unit.heal(damage.magic),
    };
}
fn bless_unit(me: &mut Unit, target: &mut Unit, damage: Power) -> bool {
    if !target.has_effect_kind(EffectKind::MageSupport) {
        target.add_effect(Box::new(HealMagic::new(damage.magic)));
        return true;
    }
    false
}
fn heal_bless(me: &mut Unit, target: &mut Unit, damage: Power, magic_type: MagicType) -> bool {
    if heal_unit(me, target, damage, magic_type) {
        return bless_unit(me, target, damage);
    }
    true
}

fn elemental_bless(me: &mut Unit, target: &mut Unit, damage: Power) -> bool {
    if !target.has_effect_kind(EffectKind::MageSupport) {
        target.add_effect(Box::new(ElementalSupport::new(damage.magic)));
        return true;
    };
    false
}

fn magic_curse(me: &mut Unit, target: &mut Unit, mut damage: Power, magic_type: MagicType) -> bool {
    match (target.info.unit_type, magic_type) {
        (UnitType::Undead, Life(_)) => {
            damage.magic *= 2;
        }
        _ => {}
    }
    if !target.has_effect_kind(EffectKind::MageCurse) {
        target.add_effect(Box::new(AttackMagic::new(
            target.correct_damage(&damage, me.info.magic_type).magic,
        )));
        true
    } else {
        false
    }
}

fn elemental_curse(me: &mut Unit, target: &mut Unit, damage: Power) -> bool {
    if !target.has_effect_kind(EffectKind::MageCurse) {
        target.add_effect(Box::new(DisableMagic::new(
            target.correct_damage(&damage, me.info.magic_type).magic,
        )));
        true
    } else {
        false
    }
}

fn magic_attack(
    me: &mut Unit,
    target: &mut Unit,
    mut damage: Power,
    magic_type: MagicType,
    target_pos: UnitPos,
    my_pos: UnitPos,
) -> bool {
    match (target.info.unit_type, magic_type) {
        (UnitType::Undead, Life(_)) => {
            damage.magic *= 2;
        }
        _ => {}
    }
    if !magic_curse(me, target, damage, magic_type) {
        damage.hand = 0;
        damage.ranged = 0;
        target.being_attacked(&damage, me, target_pos, my_pos);
    }
    true
}

fn elemental_attack(
    me: &mut Unit,
    target: &mut Unit,
    damage: Power,
    target_pos: UnitPos,
    my_pos: UnitPos,
) -> bool {
    let mut damage = damage;
    if !elemental_curse(me, target, damage) {
        damage.hand = 0;
        damage.ranged = 0;
        target.being_attacked(&damage, me, target_pos, my_pos);
    }
    true
}

fn get_magic_direction(magic_type: MagicType) -> MagicDirection {
    match magic_type {
        Death(direction) => direction,
        Life(direction) => direction,
        Elemental(direction) => direction,
    }
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
        bonus: Box<dyn Bonus>,
        effects: Vec<Box<dyn Effect>>,
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
    pub fn can_attack(&self, target: &Unit, target_pos: UnitPos, my_pos: UnitPos) -> bool {
        let effected = self.modified;
        let is_enemy = self.army != target.army;
        let is_in_back = my_pos.1 == 0;
        let my_field = field_type(my_pos.0.into(), *MAX_TROOPS.lock().unwrap());
        let enemy_field = field_type(target_pos.0.into(), *MAX_TROOPS.lock().unwrap());
        let mut damage = effected.damage;
        /*
         * C(`B`(`A`|D)) | `C`(`A^B`)
         */
        let enemy_in_reserve = enemy_field == Field::Reserve;
        let me_in_reserve = my_field == Field::Reserve;
        let both_in_reserve = me_in_reserve && enemy_in_reserve;
        let both_not_in_reserve = !me_in_reserve && !enemy_in_reserve;
        let a = (is_enemy && (!me_in_reserve || self.bonus.id() == "Ghost") && !enemy_in_reserve)
            || (!is_enemy && (both_not_in_reserve || both_in_reserve));
        return if !a {
            false
        } else if damage.ranged > 0
            && (target_pos.1 == my_pos.1 && abs(target_pos.0 as i64 - my_pos.0 as i64) < 2
                || my_field == Field::Back)
            && is_enemy
        {
            true
        } else if damage.hand > 0 && !is_in_back && target_pos.1 == 1 && is_enemy {
            true
        } else {
            match self.info.magic_type {
                None => false,
                Some(magic_type) => {
                    let direction = get_magic_direction(magic_type);
                    match (direction, magic_type, is_enemy) {
                        (ToAlly, _, false) => match magic_type {
                            Death(_) | Life(_) => !target.has_effect_kind(EffectKind::MageSupport),
                            Elemental(_) => !target.has_effect_kind(EffectKind::MageSupport),
                        },
                        (ToAll, _, _) => match (magic_type, is_enemy) {
                            (Death(_) | Life(_), true) => is_in_back,
                            (Death(_) | Life(_), false) => {
                                !target.has_effect_kind(EffectKind::MageSupport)
                            }
                            (Elemental(_), true) => is_in_back,
                            (Elemental(_), false) => {
                                !target.has_effect_kind(EffectKind::MageSupport)
                            }
                        },
                        (ToEnemy, _, true) => match magic_type {
                            Death(_) | Life(_) => is_in_back,
                            Elemental(_) => is_in_back,
                        },
                        (CurseOnly, _, true) => match magic_type {
                            Death(_) | Life(_) => is_in_back,
                            Elemental(_) => is_in_back,
                        },
                        (StrikeOnly, _, true) => is_in_back,
                        (BlessOnly, _, false) => match magic_type {
                            Life(_) | Death(_) => !target.has_effect_kind(EffectKind::MageSupport),
                            Elemental(_) => !target.has_effect_kind(EffectKind::MageSupport),
                        },
                        (CureOnly, Life(_) | Death(_), false) => {
                            !target.stats.hp == target.stats.max_hp
                        }
                        _ => false,
                    }
                }
            }
        };
    }
    pub fn attack(&mut self, target: &mut Unit, target_pos: UnitPos, my_pos: UnitPos) -> bool {
        let effected = self.modified;
        let is_enemy = self.army != target.army;
        let is_in_back = my_pos.1 == 0;
        let my_field = field_type(my_pos.0.into(), *MAX_TROOPS.lock().unwrap());
        let enemy_field = field_type(target_pos.0.into(), *MAX_TROOPS.lock().unwrap());
        let mut damage = effected.damage;
        let enemy_in_reserve = enemy_field == Field::Reserve;
        let me_in_reserve = my_field == Field::Reserve;
        let both_in_reserve = me_in_reserve && enemy_in_reserve;
        let both_not_in_reserve = !me_in_reserve && !enemy_in_reserve;
        let a = (is_enemy && (!me_in_reserve || self.bonus.id() == "Ghost") && !enemy_in_reserve)
            || (!is_enemy && (both_not_in_reserve || both_in_reserve));
        return if !a {
            false
        } else if damage.ranged > 0
            && (target_pos.1 == my_pos.1 && abs(target_pos.0 as i64 - my_pos.0 as i64) < 2
            || my_field == Field::Back)
            && is_enemy
        {
            damage.hand = 0;
            damage.magic = 0;
            target.being_attacked(&damage, self, target_pos, my_pos);
            true
        } else if damage.hand > 0 && !is_in_back && target_pos.1 == 1 && is_enemy {
            damage.ranged = 0;
            damage.magic = 0;
            target.being_attacked(&damage, self, target_pos, my_pos);
            true
        } else {
            match self.info.magic_type {
                None => false,
                Some(magic_type) => {
                    let direction = get_magic_direction(magic_type);
                    match (direction, magic_type, is_enemy) {
                        (ToAlly, _, false) => match magic_type {
                            Death(_) | Life(_) => heal_bless(self, target, damage, magic_type),
                            Elemental(_) => elemental_bless(self, target, damage),
                        },
                        (ToAll, _, _) => match (magic_type, is_enemy) {
                            (Death(_) | Life(_), true) => {
                                if is_in_back {
                                    magic_attack(
                                        self, target, damage, magic_type, my_pos, target_pos,
                                    )
                                } else {
                                    false
                                }
                            }
                            (Death(_) | Life(_), false) => {
                                heal_bless(self, target, damage, magic_type)
                            }
                            (Elemental(_), true) => {
                                if is_in_back {
                                    elemental_attack(self, target, damage, target_pos, my_pos)
                                } else {
                                    false
                                }
                            }
                            (Elemental(_), false) => elemental_bless(self, target, damage),
                        },
                        (ToEnemy, _, true) => {
                            if is_in_back {
                                match magic_type {
                                    Death(_) | Life(_) => magic_attack(
                                        self, target, damage, magic_type, my_pos, target_pos,
                                    ),
                                    Elemental(_) => {
                                        elemental_attack(self, target, damage, target_pos, my_pos)
                                    }
                                }
                            } else {
                                false
                            }
                        }
                        (CurseOnly, _, true) => match magic_type {
                            Death(_) | Life(_) => magic_curse(self, target, damage, magic_type),
                            Elemental(_) => elemental_curse(self, target, damage),
                        },
                        (StrikeOnly, _, true) => {
                            damage.hand = 0;
                            damage.ranged = 0;
                            target.being_attacked(&damage, self, my_pos, target_pos);
                            true
                        }
                        (BlessOnly, _, false) => match magic_type {
                            Life(_) | Death(_) => bless_unit(self, target, damage),
                            Elemental(_) => elemental_bless(self, target, damage),
                        },
                        (CureOnly, Life(_) | Death(_), false) => {
                            heal_unit(self, target, damage, magic_type)
                        }
                        _ => false,
                    }
                }
            }
        };
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
    pub fn add_effect(&mut self, effect: Box<dyn Effect>) -> bool {
        self.effects.push(effect);
        self.effects.last_mut().unwrap().clone().update_stats(self);
        self.recalc();
        true
    }
    pub fn add_item(&mut self, item: Item, index: usize) -> bool {
        if let Some(Some(_)) = self.inventory.items.get(index) {
            return false;
        }
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
    pub fn get_bonus(&self) -> Box<dyn Bonus> {
        let mut bonus = self.bonus.clone();
        if let Some(item) = self
            .inventory
            .items
            .iter()
            .filter(|item|
                if let Some(item) = item {
                    item.get_info().bonus.is_some()
                } else { false }
            )
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
        target_pos: UnitPos,
    ) -> u64 {
        let corrected_damage = self.correct_damage(damage, sender.info.magic_type);
        let unit_bonus = sender.get_bonus();
        let corrected_damage = unit_bonus.on_attacking(corrected_damage, self, sender);
        let corrected_damage =
            self.get_bonus()
                .on_attacked(corrected_damage, self, sender, my_pos, target_pos);
        let mut corrected_damage_units =
            corrected_damage.magic + corrected_damage.ranged + corrected_damage.hand;
        if corrected_damage_units == 0 {
            corrected_damage_units = 1;
        }
        if corrected_damage_units as i64 > self.modified.hp {
            self.stats.hp = -self.modified.hp;
        } else {
            self.stats.hp -= corrected_damage_units as i64;
        }
        self.recalc();
        corrected_damage_units
    }
    pub fn correct_damage(&self, damage: &Power, magic_type: Option<MagicType>) -> Power {
        let defence: Defence = self.modified.defence;
        let percent_100 = Percent::new(100);
        let mut magic: u64 = 0;

        if let Some(magic_type) = magic_type {
            let magic_def = match magic_type {
                Life(_) => defence.life_magic,
                Death(_) => defence.death_magic,
                Elemental(_) => defence.elemental_magic,
            };
            magic =
                (percent_100 - magic_def).calc(damage.magic.saturating_sub(defence.magic_units));
        }

        Power {
            ranged: (percent_100 - defence.ranged_percent)
                .calc(damage.ranged.saturating_sub(defence.ranged_units)),
            magic,
            hand: (percent_100 - defence.hand_percent)
                .calc(damage.hand.saturating_sub(defence.hand_units)),
        }
    }
    pub fn tick(&mut self) -> bool {
        let mut effect: Box<dyn Effect>;
        let mut removed = 0;
        for effect_num in 0..self.effects.len() {
            effect = self.effects[effect_num - removed].clone();
            effect.tick(self);
            self.effects[effect_num - removed].on_tick();
            if effect.is_dead() {
                self.effects.remove(effect_num - removed).kill(self);
                removed += 1;
                self.recalc();
            }
        }
        self.get_bonus().on_tick(self);
        true
    }
}

impl Display for Unit {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let bonus_info = bonus_info(self.get_bonus());
        let locale = LOCALE.lock().unwrap();
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
        let mut magic_dir = None;
        let magic_type = match &self.info.magic_type {
            Some(magic_type) => match magic_type {
                Life(dir) => {
                    magic_dir = Some(dir);
                    locale.get("unitstats_magictype_life")
                }
                Death(dir) => {
                    magic_dir = Some(dir);
                    locale.get("unitstats_magictype_death")
                }
                Elemental(dir) => {
                    magic_dir = Some(dir);
                    locale.get("unitstats_magictype_elemental")
                }
                _ => locale.get("unitstats_empty"),
            },
            None => locale.get("unitstats_empty"),
        }
        .to_string();
        let magic_dir = match magic_dir {
            Some(dir) => match dir {
                ToAll => locale.get("unitstats_magic_toall"),
                ToAlly => locale.get("unitstats_magic_toally"),
                ToEnemy => locale.get("unitstats_magic_toenemy"),
                StrikeOnly => locale.get("unitstats_magic_strikeonly"),
                BlessOnly => locale.get("unitstats_magic_blessonly"),
                CureOnly => locale.get("unitstats_magic_cureonly"),
                CurseOnly => locale.get("unitstats_magic_curseonly"),
            },
            None => locale.get("unitstats_empty"),
        }
        .to_string();
        let unit_type = match self.info.unit_type {
            UnitType::Undead => locale.get("unitstats_unittype_undead"),
            UnitType::People => locale.get("unitstats_unittype_people"),
            UnitType::Hero => locale.get("unitstats_unittype_hero"),
            UnitType::Rogue => locale.get("unitstats_unittype_rogue"),
            UnitType::Animal => locale.get("unitstats_unittype_animal"),
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
            self.info.next_unit.join("|"),
            surrender,
            locale.get("unitstats_upgrade"),
            locale.get("unitstats_bonus"),
            format!("{} - {}", bonus_info.0, bonus_info.1)))
    }
}
