use crate::{locale::process_locale, parse::LOCALE};

use super::unit::{Defence, MagicType, Power, UnitStats};
use alkahest::alkahest;
use derive_more::{Add, AddAssign, Sub, SubAssign};
use math_thingies::{add_opt, sub_opt, Percent};
use num::{Num, NumCast, Zero};
use std::{
    fmt::{Debug, Display},
    ops::{Add, AddAssign, Neg, Sub, SubAssign},
};

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[alkahest(Deserialize, Serialize, SerializeRef, Formula)]
pub struct Modify<V: Num + NumCast> {
    pub set: Option<V>,
    pub add: Option<V>,
    pub percent_add: Option<Percent>,
}
impl<K: Num + NumCast + Add<Percent, Output = K> + Copy> Modify<K> {
    pub fn apply<V: Num + NumCast + Add<Percent, Output = V> + Ord + Copy>(&self, v: V) -> V {
		let mut v = <K as NumCast>::from(v).unwrap();
		if let Some(set) = &self.set {
            if !set.is_zero() {
                v = *set;
            }
        }
        let mut v: K = v + self.add.unwrap_or(K::zero());
        if let Some(percent_add) = &self.percent_add {
            v = v + *percent_add;
        }
        NumCast::from(v).unwrap_or(<V as Zero>::zero()).max(<V as Zero>::zero())
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
}
impl<V: Num + NumCast + Copy + Display + PartialOrd> Display for Modify<V> {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		let mut stats = vec![];
		if let Some(add) = self.add {
			let sign = if add >= V::zero() {
				"+"
			} else { "-" };
			stats.push(format!("{sign}{add}"));
		}
		if let Some(add) = self.percent_add {
			let r = add.get();
			let sign = if add >= 0 {
				"+"
			} else { "-" };
			stats.push(format!("{sign}{}%", add.get()));
		}
		if let Some(add) = self.set {
			stats.push(format!("={}", add));
		}
		f.write_str(&stats.join("/"));
		std::fmt::Result::Ok(())
	}
}
impl<V: Num + NumCast + Copy> Add<Modify<V>> for Modify<V> {
    type Output = Self;
    fn add(self, _rhs: Self) -> Self::Output {
        Self {
            set: _rhs.set,
            add: add_opt(self.add, _rhs.add),
            percent_add: add_opt(self.percent_add, _rhs.percent_add),
        }
    }
}
impl<V: Num + NumCast + Copy + Neg<Output = V>> Sub<Modify<V>> for Modify<V> {
    type Output = Self;
    fn sub(self, _rhs: Self) -> Self::Output {
        Self {
            set: Some(V::zero()),
            add: sub_opt(self.add, _rhs.add),
            percent_add: sub_opt(self.percent_add, _rhs.percent_add),
        }
    }
}
impl<V: Num + NumCast + Copy> AddAssign<Modify<V>> for Modify<V> {
    fn add_assign(&mut self, _rhs: Self) {
        self.set = _rhs.set;
        self.add = add_opt(self.add, _rhs.add);
        self.percent_add = add_opt(self.percent_add, _rhs.percent_add);
    }
}
impl<V: Num + NumCast + Copy + Neg<Output = V>> SubAssign<Modify<V>> for Modify<V> {
    fn sub_assign(&mut self, _rhs: Self) {
        self.set = Some(V::zero());
        self.add = sub_opt(self.add, _rhs.add);
        self.percent_add = sub_opt(self.percent_add, _rhs.percent_add);
    }
}
impl<V: Num + NumCast> Default for Modify<V> {
    fn default() -> Modify<V> {
        Self {
            set: None,
            add: None,
            percent_add: None,
        }
    }
}

#[derive(Copy, Clone, Debug, Add, Sub, AddAssign, SubAssign, PartialEq)]
#[alkahest(Deserialize, Serialize, SerializeRef, Formula)]
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
        n_defence.ranged_units = self.ranged_units.apply(defence.ranged_units);
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

#[derive(Copy, Clone, Debug, Add, Sub, AddAssign, SubAssign, PartialEq)]
#[alkahest(Deserialize, Serialize, SerializeRef, Formula)]
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
    pub fn apply(&self, power: &Power, magic_type: Option<MagicType>) -> Power {
        let mut n_power = Power::empty();
		let expected_min = match magic_type {
			Some(MagicType::Death) => 25,
			Some(_) => 15,
			_ => 0

		};
		let min = if power.magic >= expected_min {
			expected_min
		} else { power.magic };
        n_power.magic = self.magic.apply(power.magic).max(min);
		if power.ranged > 0 {
			n_power.ranged = self.ranged.apply(power.ranged);
		}
		if power.hand > 0 {
			n_power.hand = self.hand.apply(power.hand);
		}
        n_power
    }
}

#[derive(Copy, Clone, Debug, Add, Sub, AddAssign, SubAssign, PartialEq)]
#[alkahest(Deserialize, Serialize, SerializeRef, Formula)]
pub struct ModifyUnitStats {
    pub max_hp: Modify<i64>,
    pub damage: ModifyPower,
    pub defence: ModifyDefence,
    pub max_moves: Modify<i64>,
    pub speed: Modify<i64>,
    pub vamp: Modify<i16>,
    pub regen: Modify<i16>,
}
impl ModifyUnitStats {
	pub fn display_string(&self) -> Vec<String> {
		let locale = LOCALE.read().unwrap();
		let mut strings = vec![];
		if self.max_hp != Modify::default() {
			strings.push(format!("{}: {}", locale.get("unitstats_hp"), self.max_hp.to_string()));
		}
		
		if self.damage.hand != Modify::default() {
			strings.push(format!("{}: {}", locale.get("unitstats_attack_hand"), self.damage.hand.to_string()));
		}
		if self.damage.ranged != Modify::default() {
			strings.push(format!("{}: {}", locale.get("unitstats_attack_ranged"), self.damage.ranged.to_string()));
		}
		if self.damage.magic != Modify::default() {
			strings.push(format!("{}: {}", locale.get("unitstats_attack_magic"), self.damage.magic.to_string()));
		}
		
		if self.defence.hand_units != Modify::default() {
			strings.push(format!("{}: {}", locale.get("unitstats_defence_hand"), self.defence.hand_units.to_string()));
		}
		if self.defence.ranged_units != Modify::default() {
			strings.push(format!("{}: {}", locale.get("unitstats_defence_ranged"), self.defence.ranged_units.to_string()));
		}
		if self.defence.magic_units != Modify::default() {
			strings.push(format!("{}: {}", locale.get("unitstats_defence_magic"), self.defence.magic_units.to_string()));
		}
		if self.defence.death_magic != Modify::default() {
			strings.push(format!("{}: {}", process_locale("$unitstats_defence_magic_death", &locale), self.defence.death_magic.to_string()));
		}
		if self.defence.life_magic != Modify::default() {
			strings.push(format!("{}: {}", process_locale("$unitstats_defence_magic_life", &locale), self.defence.life_magic.to_string()));
		}
		if self.defence.elemental_magic != Modify::default() {
			strings.push(format!("{}: {}", process_locale("$unitstats_defence_magic_elemental", &locale), self.defence.elemental_magic.to_string()));
		}
		if self.max_moves != Modify::default() {
			strings.push(format!("{}: {}", locale.get("unitstats_moves"), self.max_moves.to_string()));
		}
		if self.speed != Modify::default() {
			strings.push(format!("{}: {}", locale.get("unitstats_speed"), self.speed.to_string()));
		}
		if self.vamp != Modify::default() {
			strings.push(format!("{}: {}", locale.get("unitstats_vamp"), self.vamp.to_string()));
		}
		if self.regen != Modify::default() {
			strings.push(format!("{}: {}", locale.get("unitstats_regen"), self.regen.to_string()));
		}
		strings
	}
    pub fn apply(&self, stats: &UnitStats, magic_type: Option<MagicType>) -> UnitStats {
        let mut n_stats = UnitStats::empty();
        //n_stats.hp = self.hp.apply(stats.hp)
        n_stats.max_hp = self.max_hp.apply(stats.max_hp);
        n_stats.damage = self.damage.apply(&stats.damage, magic_type);
        n_stats.defence = self.defence.apply(&stats.defence);
        //n_stats.moves = self.moves.apply(stats.moves);
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
            max_hp: Modify::default(),
            damage: ModifyPower::default(),
            defence: ModifyDefence::default(),
            max_moves: Modify::default(),
            speed: Modify::default(),
            vamp: Modify::default(),
            regen: Modify::default(),
        }
    }
}
