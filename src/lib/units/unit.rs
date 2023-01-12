use {
    std::{
        fmt::{Debug, Display, Formatter},
        cmp::Ordering
    },
    dyn_clone::DynClone,
    derive_more::{AddAssign, DivAssign, MulAssign, SubAssign},
    crate::lib::{
        effects::{
            effect::{Effect, EffectKind, EffectInfo},
            effects::{AttackMagic, HealMagic, DisableMagic, ElementalSupport}
        },
        items::item::Item,
        bonuses::{bonuses::*, bonus::Bonus},
        battle::troop::Troop,
        units::unit::{
            MagicType::*,
            MagicDirection::*
    }   },
    derive_more::{Add, Sub, Mul, Div, Neg},
    math_thingies::Percent,
};
#[derive(Copy, Clone, Debug, Add, Sub)]
pub struct Defence {
    pub death_magic: Percent,
    pub elemental_magic: Percent,
    pub life_magic: Percent,
    pub hand_percent: Percent,
    pub ranged_percent: Percent,
    pub magic_units: u64,
    pub hand_units: u64,
    pub ranged_units: u64
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
            ranged_units: 0
}   }   }

#[derive(Copy, Clone, Debug, Add, Sub)]
pub struct Power {
    pub magic: u64,
    pub ranged: u64,
    pub hand: u64
}
impl Power {
    pub fn empty() -> Self {
        Self {
            magic: 0,
            ranged: 0,
            hand: 0
}   }   }

#[derive(Copy, Clone, Debug, Add, Sub)]
pub struct UnitStats {
    pub hp: i64,
    pub max_hp: i64,
    pub damage: Power,
    pub defence: Defence,
    pub moves: u64,
    pub max_moves: u64,
    pub speed: u64,
    pub vamp: Percent,
    pub regen: Percent
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
            regen: Percent::new(0)
}   }   }

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum MagicDirection {
    ToAlly,
    ToAll,
    ToEnemy,
    CurseOnly,
    StrikeOnly,
    BlessOnly,
    CureOnly
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
            max_xp: 0
}   }   }

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
    pub lvl: LevelUpInfo
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
            lvl: LevelUpInfo::empty()
}   }   }

#[derive(Clone, Debug)]
pub struct UnitInventory {
    pub items: Vec<Option<Item>>
}
impl UnitInventory {
    pub fn empty() -> Self {
        Self {
            items: vec![]
}   }   }

#[derive(Clone, Debug)]
pub struct UnitLvl {
    pub lvl: u64,
    pub max_xp: u64,
    pub xp: u64
}
impl UnitLvl {
    pub fn empty() -> Self {
        Self {
            lvl: 0,
            max_xp: 0,
            xp: 0
}   }   }

#[derive(Clone, Debug)]
pub struct UnitData {
    pub stats: UnitStats,
    pub info: UnitInfo,
    pub lvl: UnitLvl,
    pub inventory: UnitInventory,
    pub bonus: Box<dyn Bonus>,
    pub effects: Vec<Box<dyn Effect>>
}

#[derive(Copy, Clone, Add, AddAssign, Sub, SubAssign, Mul, MulAssign, Div, DivAssign)]
pub struct UnitPos(pub usize, pub usize);
impl Into<(usize, usize)> for UnitPos {
    fn into(self) -> (usize, usize) {
        (self.0, self.1)
}   }

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum UnitType {
    People,
    Hero,
    Animal,
    Undead,
    Rogue
}

#[derive(Clone, Debug)]
pub struct Unit {
    pub stats: UnitStats,
    pub info: UnitInfo,
    pub lvl: UnitLvl,
    pub inventory: UnitInventory,
    pub army: usize,
    pub bonus: Box<dyn Bonus>,
    pub effects: Vec<Box<dyn Effect>>
}

fn heal_unit(me: &mut Unit, unit: &mut Unit, damage: Power, magic_type: MagicType) -> bool {
    return match (unit.info.unit_type, magic_type) {
        (UnitType::Rogue | UnitType::Hero | UnitType::People, Death(_)) => {
            false
        }
        (UnitType::Undead, Life(_)) => {
            false
        }
        _ => { unit.heal(damage.magic) }
}   }
fn bless_unit(me: &mut Unit, target: &mut Unit, damage: Power) -> bool {
    if !target.has_effect_kind(EffectKind::MageSupport) {
        target.add_effect(Box::new(HealMagic::new(damage.magic)));
        return true
    }
    false
}
fn heal_bless(me: &mut Unit, target: &mut Unit, damage: Power, magic_type: MagicType) -> bool {
    if heal_unit(me, target, damage, magic_type) {
        bless_unit(me, target, damage);
        return true
    }
    false
}

fn elemental_bless(me: &mut Unit, target: &mut Unit, damage: Power) -> bool {
    if !target.has_effect_kind(EffectKind::MageSupport) {
        target.add_effect(Box::new(ElementalSupport::new(damage.magic)));
        return true
    };
    false
}

fn magic_curse(me: &mut Unit, target: &mut Unit, mut damage: Power, magic_type: MagicType) -> bool {
    match (target.info.unit_type, magic_type) {
        (UnitType::Undead, Life(_)) => {
            damage.magic *= 2;
        },
        _ => {}
    }
    if !target.has_effect_kind(EffectKind::MageCurse) {
        target.add_effect(Box::new(AttackMagic::new(target.correct_damage(&damage, me.info.magic_type).magic)));
        true
    } else {
        false
}   }

fn elemental_curse(me: &mut Unit, target: &mut Unit, damage: Power) -> bool {
    if !target.has_effect_kind(EffectKind::MageCurse) {
        target.add_effect(Box::new(DisableMagic::new(target.correct_damage(&damage, me.info.magic_type).magic)));
        true
    } else {
        false
}   }

fn magic_attack(me: &mut Unit, target: &mut Unit, mut damage: Power, magic_type: MagicType) -> bool {
    match (target.info.unit_type, magic_type) {
        (UnitType::Undead, Life(_)) => {
            damage.magic *= 2;
        },
        _ => {}
    }
    if !magic_curse(me, target, damage, magic_type) {
        target.being_attacked(&damage, me);
    }
    true
}

fn elemental_attack(me: &mut Unit, target: &mut Unit, damage: Power) -> bool {
    if !elemental_curse(me, target, damage) {
        target.being_attacked(&damage, me);
    }
    true
}

fn get_magic_direction(magic_type: MagicType) -> MagicDirection {
    match magic_type {
        Death(direction) => direction,
        Life(direction) => direction,
        Elemental(direction) => direction
}   }

impl Unit {
    pub fn attack(&mut self, target: &mut Unit, target_pos: UnitPos, my_pos: UnitPos) -> bool {
        let effected = self.get_effected_stats();
        let distance_vec: UnitPos = my_pos - target_pos;
        let distance: usize = distance_vec.0.max(distance_vec.1);
        let is_enemy = self.army != target.army;
        let mut damage = effected.damage;

        if distance == 1 {
            damage.magic = 0;
            if damage.hand > 0 {
                damage.ranged = 0;
            } else if damage.ranged > 0 {
                damage.hand = 0;
            } else {
                return false
            }
        } else if distance < 5 {
            damage.hand = 0;
            if damage.magic > 0 {
                damage.ranged = 0;
            } else if damage.ranged > 0 {
                damage.magic = 0;
            } else {
                return false
            }
        } else {
            return false
        }

        return if damage.hand > 0 || damage.ranged > 0{
            target.being_attacked(&damage, self);
            true
        } else {
            match self.info.magic_type {
                None => {
                    false
                },
                Some(magic_type) => {
                    let direction = get_magic_direction(magic_type);
                    match (direction, magic_type, is_enemy) {
                        (ToAlly, _, false) => {
                            match magic_type {
                                Death(_) | Life(_) => heal_bless(self, target, damage, magic_type),
                                Elemental(_) => elemental_bless(self, target, damage)
                        }   },
                        (ToAll, _, _) => {
                            match (magic_type, is_enemy) {
                                (Death(_) | Life(_), true) => magic_attack(self, target, damage, magic_type),
                                (Death(_) | Life(_), false) => heal_bless(self, target, damage, magic_type),
                                (Elemental(_), true) => elemental_attack(self, target, damage),
                                (Elemental(_), false) => elemental_bless(self, target, damage)
                        }   },
                        (ToEnemy, _, true) => {
                            match magic_type {
                                Death(_) | Life(_) => magic_attack(self, target, damage, magic_type),
                                Elemental(_) => elemental_attack(self, target, damage)
                        }   }
                        (CurseOnly, _, true) => {
                            match magic_type {
                                Death(_) | Life(_) => magic_curse(self, target, damage, magic_type),
                                Elemental(_) => elemental_curse(self, target, damage)
                        }   }
                        (StrikeOnly, _, true) => {
                            target.being_attacked(&damage, self);
                            true
                        },
                        (BlessOnly, _, false) => {
                            match magic_type {
                                Life(_) | Death(_) => bless_unit(self, target, damage),
                                Elemental(_) => elemental_bless(self, target, damage)
                        }   }
                        (CureOnly, Life(_) | Death(_), false) => heal_unit(self, target, damage, magic_type),
                        _ => false
    }   }   }   }   }

    pub fn heal(&mut self, amount: u64) -> bool {
        let effected = self.get_effected_stats();
        let amount = amount as i64;
        if effected.max_hp < effected.hp + amount {
            self.stats.hp = effected.max_hp;
            return true;
        }
        self.stats.hp += amount;
        return false;
    }

    pub fn get_effected_stats(&self) -> UnitStats {
        let mut previous: UnitStats = self.stats;
        previous
    }
    pub fn is_dead(&self) -> bool {
        self.get_effected_stats().hp < 1
    }
    pub fn has_effect_kind(&self, kind: EffectKind) -> bool {
        for effect in &self.effects {
            if effect.get_kind() == kind {
               return true;
            };
        }
        return false;
    }
    pub fn kill(&mut self) { self.stats.hp -= self.get_effected_stats().hp - self.stats.hp;}
    pub fn add_effect(&mut self, effect: Box<dyn Effect>) -> bool {
        self.effects.push(effect);
        true
    }
    pub fn add_item(&mut self, item: Item) -> bool {
        self.inventory.items.push(Some(item));
        true
    }
    pub fn being_attacked(&mut self, damage: &Power, sender: &mut Unit) -> u64 {
        let corrected_damage = self.correct_damage(damage, sender.info.magic_type);
        let unit_bonus = sender.bonus.clone();
        let corrected_damage = unit_bonus.on_attacking(corrected_damage, self, sender);
        let corrected_damage = self.bonus.clone().on_attacked(corrected_damage, self, sender);
        let corrected_damage_units = corrected_damage.magic + corrected_damage.ranged + corrected_damage.hand;
        self.stats.hp -= corrected_damage_units as i64;
        corrected_damage_units
    }
    pub fn correct_damage(&self, damage: &Power, magic_type: Option<MagicType>) -> Power {
        let defence: Defence = self.get_effected_stats().defence;
        let percent_100 = Percent::new(100);
        let mut magic: u64 = 0;

        if let Some(magic_type) = magic_type {
            let magic_def = match magic_type {
                Life(_) => defence.life_magic,
                Death(_) => defence.death_magic,
                Elemental(_) => defence.elemental_magic
            };
            magic = (percent_100 - magic_def).calc(damage.magic.saturating_sub(defence.magic_units));
        }

        Power {
            ranged: (percent_100 - defence.ranged_percent).calc(damage.ranged.saturating_sub(defence.ranged_units)),
            magic,
            hand: (percent_100 - defence.hand_percent).calc(damage.hand.saturating_sub(defence.hand_units))
    }   }
    pub fn tick(&mut self) -> bool {
        let mut effect: Box<dyn Effect>;
        for effect_num in 0..self.effects.len() {
            effect = self.effects[effect_num].clone();
            effect.tick(self);
            self.effects[effect_num].on_tick();
            if effect.is_dead() {
                self.effects.remove(effect_num);
            }   }
        self.bonus.clone().on_tick(self);
        true
    }
}

impl Display for Unit {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let damage = self.stats.damage;
        let attack = format!("Рукопашной атаки: {}\nДальней атаки: {}\nМагической атаки: {}", damage.hand, damage.ranged, damage.magic);
        let defence = format!("Рукопашной защиты: {}\nДальней защиты: {}\nПроцент защиты от магии смерти: {}, жизни: {}, стихий: {}", self.stats.defence.hand_units, self.stats.defence.ranged_units, self.stats.defence.death_magic, self.stats.defence.life_magic, self.stats.defence.elemental_magic);
        let mut magic_dir = None;
        let magic_type = match &self.info.magic_type{
            Some(magic_type) => {
                match magic_type {
                    Life(dir) => {
                        magic_dir = Some(dir);
                        "Магия жизни"
                    }
                    Death(dir) => {
                        magic_dir = Some(dir);
                        "Магия смерти"
                    }
                    Elemental(dir) => {
                        magic_dir = Some(dir);
                        "Магия стихий"
                    }
                    _ => "Отсутствует"
                }
            },
            None => "Отсутствует"
        }.to_string();
        let magic_dir = match magic_dir {
            Some(dir) => {
                match dir {
                    ToAll => "На всех",
                    ToAlly => "На своих",
                    ToEnemy => "На врагов",
                    StrikeOnly => "Только атаковать врагов",
                    BlessOnly => "Только благославлять союзников",
                    CureOnly => "Только лечить союзников",
                    CurseOnly => "ТОлько проклинать врагов",
                }.to_string()
            }
            None => "Без магии.".to_string()
        }.to_string();
        let unit_type = match self.info.unit_type {
            UnitType::Undead => "Нежить",
            UnitType::People => "Человек",
            UnitType::Hero => "Герой",
            UnitType::Rogue => "Разбойник",
            UnitType::Animal => "Животное"
        }.to_string();
        let surrender = if self.info.surrender > Some(0) {
            format!("Сдаётся, даёт {} маны", self.info.surrender.unwrap())
        } else {"Не сдаётся".to_string()};
        f.write_fmt(format_args!("{} | {} опыта для развития\n{}\n Жизни: {}\nТип магии: {}|Направление магии: {}\n{}\n{}\nВампиризм: {}\nРеген: {}\nХоды: {}\nИнициатива: {}\nКто: {}\nСтоимость - {} для найма; {} золотых в день\nЭволюционирует в {};{}\nБонус: {}",
                                 self.info.name, self.lvl.max_xp, self.info.descript, self.stats.hp, magic_type, magic_dir, attack, defence, self.stats.vamp, self.stats.regen, self.stats.moves, self.stats.speed, unit_type, self.info.cost_hire, self.info.cost, self.info.next_unit.join("|"), surrender, "aboba"))
    }
}