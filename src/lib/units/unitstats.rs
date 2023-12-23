use super::unit::{Defence, Power, UnitStats};
use derive_more::{Add, AddAssign, Sub, SubAssign};
use math_thingies::{add_opt, sub_opt, Percent};
use num::{Num, NumCast};
use std::{
    fmt::Debug,
    ops::{Add, AddAssign, Neg, Sub, SubAssign},
};

#[derive(Copy, Clone, Debug)]
pub struct Modify<V: Num + NumCast> {
    pub set: Option<V>,
    pub add: Option<V>,
    pub percent_add: Option<Percent>,
    pub percent_set: Option<Percent>,
}
impl<K: Num + NumCast + Add<Percent, Output = K> + Copy> Modify<K> {
    pub fn apply<V: Num + NumCast + Add<Percent, Output = V> + Copy>(&self, v: V) -> V {
        let mut v: K =
            <K as NumCast>::from(v).unwrap() + self.add.unwrap_or(K::zero());
        if let Some(percent_add) = &self.percent_add {
            v = v + *percent_add;
        }
        if let Some(percent_set) = &self.percent_set {
            v = percent_set.calc(v);
        }
        if let Some(set) = &self.set {
            if !set.is_zero() {
                v = *set;
            }
        }
        NumCast::from(v).unwrap_or(NumCast::from(0).unwrap())
    }
    pub fn set(&mut self, v: impl Into<Option<K>>) -> &mut Self {
        self.set = v.into();
        self
    }
    pub fn add(&mut self, v: impl Into<Option<K>>) -> &mut Self {
        self.add = v.into();
        self
    }
    pub fn percent_add(&mut self, v: impl Into<Option<Percent>>) -> &mut Self {
        self.percent_add = v.into();
        self
    }
    pub fn percent_set(&mut self, v: impl Into<Option<Percent>>) -> &mut Self {
        self.percent_set = v.into();
        self
    }
}
impl<V: Num + NumCast + Copy> Add<Modify<V>> for Modify<V> {

    type Output = Self;
    fn add(self, _rhs: Self) -> Self::Output {
        Self {
            set: add_opt(self.set, _rhs.set),
            add: add_opt(self.add, _rhs.add),
            percent_add: add_opt(self.percent_add, _rhs.percent_add),
            percent_set: add_opt(self.percent_set, _rhs.percent_set),
        }
    }
}
impl<V: Num + NumCast + Copy + Neg<Output = V>> Sub<Modify<V>> for Modify<V> {
    type Output = Self;
    fn sub(self, _rhs: Self) -> Self::Output {
        Self {
            set: sub_opt(self.set, _rhs.set),
            add: sub_opt(self.add, _rhs.add),
            percent_add: sub_opt(self.percent_add, _rhs.percent_add),
            percent_set: sub_opt(self.percent_set, _rhs.percent_set),
        }
    }
}
impl<V: Num + NumCast + Copy> AddAssign<Modify<V>> for Modify<V> {
    fn add_assign(&mut self, _rhs: Self) {
        self.set = add_opt(self.set, _rhs.set);
        self.add = add_opt(self.add, _rhs.add);
        self.percent_add = add_opt(self.percent_add, _rhs.percent_add);
        self.percent_set = add_opt(self.percent_set, _rhs.percent_set);
    }
}
impl<V: Num + NumCast + Copy + Neg<Output = V>> SubAssign<Modify<V>> for Modify<V> {
    fn sub_assign(&mut self, _rhs: Self) {
        self.set = sub_opt(self.set, _rhs.set);
        self.add = sub_opt(self.add, _rhs.add);
        self.percent_add = sub_opt(self.percent_add, _rhs.percent_add);
        self.percent_set = sub_opt(self.percent_set, _rhs.percent_set);
    }
}
impl<V: Num + NumCast> Default for Modify<V> {
    fn default() -> Modify<V> {
        Self {
            set: None,
            add: None,
            percent_add: None,
            percent_set: None,
        }
    }
}

#[derive(Copy, Clone, Debug, Add, Sub, AddAssign, SubAssign)]
pub struct ModifyDefence {
    pub death_magic: Modify<i16>,
    pub elemental_magic: Modify<i16>,
    pub life_magic: Modify<i16>,
    pub hand_percent: Modify<i16>,
    pub ranged_percent: Modify<i16>,
    pub magic_units: Modify<i64>,
    pub hand_units: Modify<i64>,
    pub ranged_units: Modify<i64>,
}
impl ModifyDefence {
    pub fn apply(&self, defence: &Defence) -> Defence {
        let mut n_defence = Defence::empty();
        n_defence.death_magic = self.death_magic.apply(defence.death_magic);
        n_defence.elemental_magic = self.elemental_magic.apply(defence.elemental_magic);
        n_defence.life_magic = self.life_magic.apply(defence.life_magic);
        n_defence.hand_percent = self.hand_percent.apply(defence.hand_percent);
        n_defence.ranged_percent = self.ranged_percent.apply(defence.ranged_percent);
        n_defence.magic_units = self.magic_units.apply(defence.magic_units);
        n_defence.ranged_units = self.magic_units.apply(defence.magic_units);
        n_defence.hand_units = self.hand_units.apply(defence.hand_units);
        n_defence
    }
}
impl Default for ModifyDefence {
    fn default() -> Self {
        Self {
            death_magic: Modify::default(),
            elemental_magic: Modify::default(),
            life_magic: Modify::default(),
            hand_percent: Modify::default(),
            ranged_percent: Modify::default(),
            magic_units: Modify::default(),
            hand_units: Modify::default(),
            ranged_units: Modify::default(),
        }
    }
}

#[derive(Copy, Clone, Debug, Add, Sub, AddAssign, SubAssign)]
pub struct ModifyPower {
    pub magic: Modify<i64>,
    pub ranged: Modify<i64>,
    pub hand: Modify<i64>,
}
impl Default for ModifyPower {
    fn default() -> Self {
        Self {
            magic: Modify::default(),
            ranged: Modify::default(),
            hand: Modify::default(),
        }
    }
}
impl ModifyPower {
    pub fn apply(&self, power: &Power) -> Power {
        let mut n_power = Power::empty();
        n_power.magic = self.magic.apply(power.magic);
        n_power.ranged = self.ranged.apply(power.ranged);
        n_power.hand = self.hand.apply(power.hand);
        n_power
    }
}

#[derive(Copy, Clone, Debug, Add, Sub, AddAssign, SubAssign)]
pub struct ModifyUnitStats {
    pub hp: Modify<i64>,
    pub max_hp: Modify<i64>,
    pub damage: ModifyPower,
    pub defence: ModifyDefence,
    pub moves: Modify<i64>,
    pub max_moves: Modify<i64>,
    pub speed: Modify<i64>,
    pub vamp: Modify<i16>,
    pub regen: Modify<i16>,
}
impl ModifyUnitStats {
    pub fn apply(&self, stats: &UnitStats) -> UnitStats {
        let mut n_stats = UnitStats::empty();
        n_stats.hp = self.hp.apply(stats.hp);
        n_stats.max_hp = self.max_hp.apply(stats.max_hp);
        n_stats.damage = self.damage.apply(&stats.damage);
        n_stats.defence = self.defence.apply(&stats.defence);
        n_stats.moves = self.moves.apply(stats.moves);
        n_stats.max_moves = self.max_moves.apply(stats.max_moves);
        n_stats.speed = self.speed.apply(stats.speed);
        n_stats.vamp = self.vamp.apply(stats.vamp);
        n_stats.regen = self.regen.apply(stats.regen);

        n_stats
    }
}
impl Default for ModifyUnitStats {
    fn default() -> Self {
        Self {
            hp: Modify::default(),
            max_hp: Modify::default(),
            damage: ModifyPower::default(),
            defence: ModifyDefence::default(),
            moves: Modify::default(),
            max_moves: Modify::default(),
            speed: Modify::default(),
            vamp: Modify::default(),
            regen: Modify::default(),
        }
    }
}
