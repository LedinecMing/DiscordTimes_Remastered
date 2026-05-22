use crate::{locale::{Locale, process_locale}};

use super::unit::{Defence, MagicType, Power, UnitStats};
use advini::{Ini, IniParseError, Sections, trim_separator};
use alkahest::alkahest;
use derive_more::{Add, AddAssign, Sub, SubAssign, Mul, Div};
use math_thingies::{add_opt, sub_opt, Percent};
use nom::{Parser, bytes::complete::tag, character::complete::digit1, combinator::opt, multi::{many, many0}, sequence::{preceded, terminated}};
use num::{Num, NumCast, Zero};
use schemars::JsonSchema;
use std::{
    fmt::{Debug, Display},
    ops::{Add, AddAssign, Div, Mul, Neg, Sub, SubAssign},
};

#[derive(Copy, Clone, Debug, PartialEq, Eq, serde::Deserialize, serde::Serialize, JsonSchema)]
#[alkahest(Deserialize, Serialize, SerializeRef, Formula)]
pub struct Modify<V: Num + NumCast> {
	#[serde(default, skip_serializing_if = "is_default")]
    pub set: Option<V>,
	#[serde(default, skip_serializing_if = "is_default")]
    pub add: Option<V>,
	#[serde(default, skip_serializing_if = "is_default")]
    pub percent_add: Option<Percent>,
}
impl<'z, V: Num + NumCast + Ini<'z> + Display> Ini<'z> for Modify<V> {
	type Arg = ();
	fn eat<'a>(input: &'a str, _additional: Self::Arg) -> Result<(&'a str, Self), IniParseError> {
		if let Ok((rest, _)) = trim_separator(input) {
			return Ok((rest, Self::default()));
		};
		let data = preceded(
			tag("Modify("),
			terminated(
				(
					opt(preceded(tag("="), digit1)),
					opt(preceded(tag("+"), digit1)),
					opt(preceded(tag("*"), digit1)),
				),
				tag(")")
			)
		).parse(input)?;
		let modify = Self {
			set: data.1.0.and_then(|x| Num::from_str_radix(x, 10).ok()),
			add: data.1.0.and_then(|x| Num::from_str_radix(x, 10).ok()),
			percent_add: data.1.0.and_then(|x| Num::from_str_radix(x, 10).ok()),
		};
		Ok((data.0, modify))
	}
	fn vomit(&self, additional: Self::Arg) -> String {
		if *self == Self::default() {
			"".to_owned()
		} else {
			let mut res = String::new();
			if let Some(s) = &self.set {
				res.push_str(&format!("={s}"));
			}
			if let Some(s) = &self.add {
				res.push_str(&format!("+{s}"));
			}
			if let Some(s) = &self.percent_add {
				res.push_str(&format!("*{s}"));
			}
			res
		}
	}
}
impl<V: Num + NumCast> Default for Modify<V> {
	fn default() -> Self {
		Self {
			set: None,
			add: None,
			percent_add: None
		}
	}
	
}
impl<K: Num + NumCast + Add<Percent, Output = K> + Copy> Modify<K> {
    pub fn apply<V: Num + NumCast + Add<Percent, Output = V> + Ord + Copy>(&self, v: V) -> V {
        let mut v = <K as NumCast>::from(v).unwrap();
        if let Some(set) = &self.set {
            //if !set.is_zero() {
                v = *set;
            //}
        }
        let mut v: K = v + self.add.unwrap_or(K::zero());
        if let Some(percent_add) = &self.percent_add {
            v = v + *percent_add;
        }
        NumCast::from(v)
            .unwrap_or(<V as Zero>::zero())
            .max(<V as Zero>::zero())
    }
    pub fn set(mut self, v: impl Into<Option<K>>) -> Self {
        self.set = v.into();
        self
    }
    pub fn add_val(mut self, v: impl Into<Option<K>>) -> Self {
        self.add = v.into();
        self
    }
    pub fn percent_add(mut self, v: impl Into<Option<Percent>>) -> Self {
        self.percent_add = v.into();
        self
    }
	pub fn with_set(mut self, v: impl Into<Option<K>>) -> Self {
        self.set = v.into();
        self
    }
    pub fn with_add_val(mut self, v: impl Into<Option<K>>) -> Self {
        self.add = v.into();
        self
    }
    pub fn with_percent_add(mut self, v: impl Into<Option<Percent>>) -> Self {
        self.percent_add = v.into();
        self
    }
}
impl<V: Num + NumCast + Copy + Display + PartialOrd> Display for Modify<V> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut stats = vec![];
		if let Some(add) = self.set {
            stats.push(format!("={}", add));
        }
        if let Some(add) = self.add {
            let sign = if add >= V::zero() { "+" } else { "-" };
            stats.push(format!("{sign}{add}"));
        }
        if let Some(add) = self.percent_add {
            let r = add.get();
            let sign = if add >= 0 { "+" } else { "-" };
            stats.push(format!("{sign}{}%", add.get()));
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
            set: self.set,
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
impl<V: Num + NumCast> Mul<usize> for Modify<V> {
	type Output = Self;
	fn mul(self, rhs: usize) -> Self::Output {
		Self {
			set: self.set,
			add: self.add.and_then(|x| Some(x * V::from(rhs).unwrap_or(V::zero()))),
			percent_add: self.percent_add.and_then(|x| Some(Percent::new(x.get() * rhs as i16))),
		}
	}
}
impl<V: Num + NumCast> Div<usize> for Modify<V> {
	type Output = Self;
	fn div(self, mut rhs: usize) -> Self::Output {
		if rhs == 0 {
			rhs = 1;
		}
		Self {
			set: self.set,
			add: self.add.and_then(|x| Some(x / V::from(rhs).unwrap_or(V::one()))),
			percent_add: self.percent_add.and_then(|x| Some(Percent::new(x.get() / rhs as i16))),
		}
	}
}


#[derive(Copy, Clone, Debug, Add, Sub, Mul, Div, AddAssign, SubAssign, PartialEq, Default, Sections, serde::Deserialize, serde::Serialize, JsonSchema)]
#[alkahest(Deserialize, Serialize, SerializeRef, Formula)]
pub struct ModifyDefence {
	#[default_value = "Modify::default()"]
    #[serde(default, skip_serializing_if = "is_default")]
	pub death_magic: Modify<i16>,
	#[default_value = "Modify::default()"]
	#[serde(default, skip_serializing_if = "is_default")]
    pub elemental_magic: Modify<i16>,
	#[default_value = "Modify::default()"]
	#[serde(default, skip_serializing_if = "is_default")]
    pub life_magic: Modify<i16>,
	#[default_value = "Modify::default()"]
	#[serde(default, skip_serializing_if = "is_default")]
    pub hand_percent: Modify<i16>,
	#[default_value = "Modify::default()"]
	#[serde(default, skip_serializing_if = "is_default")]
    pub ranged_percent: Modify<i16>,
	#[default_value = "Modify::default()"]
	#[serde(default, skip_serializing_if = "is_default")]
    pub magic_units: Modify<i64>,
	#[default_value = "Modify::default()"]
	#[serde(default, skip_serializing_if = "is_default")]
    pub hand_units: Modify<i64>,
	#[default_value = "Modify::default()"]
	#[serde(default, skip_serializing_if = "is_default")]
    pub ranged_units: Modify<i64>,
}
impl ModifyDefence {
    pub fn apply(&self, defence: &Defence) -> Defence {
        let mut n_defence = Defence::default();
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

#[derive(Copy, Clone, Debug, Add, Sub, Mul, Div, AddAssign, SubAssign, PartialEq, Default, Sections, serde::Deserialize, serde::Serialize, JsonSchema)]
#[alkahest(Deserialize, Serialize, SerializeRef, Formula)]
pub struct ModifyPower {
	#[serde(default, skip_serializing_if = "is_default")]
    pub magic: Modify<i64>,
	#[serde(default, skip_serializing_if = "is_default")]
    pub ranged: Modify<i64>,
	#[serde(default, skip_serializing_if = "is_default")]
    pub hand: Modify<i64>,
}
impl ModifyPower {
    pub fn apply(&self, power: &Power, magic_type: Option<MagicType>) -> Power {
        let mut n_power = Power::default();
        let expected_min = match magic_type {
            Some(MagicType::Death) => 25,
            Some(_) => 15,
            _ => 0,
        };
        let min = if power.magic >= expected_min {
            expected_min
        } else {
            power.magic
        };
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

pub fn is_default<T: Default + PartialEq>(t: &T) -> bool {
    *t == T::default()
}
pub fn is_true(t: &bool) -> bool { *t }
pub fn is_false(t: &bool) -> bool { !t }

#[derive(Copy, Clone, Debug, Add, Sub, Mul, Div, AddAssign, SubAssign, PartialEq, Default, Sections, serde::Deserialize, serde::Serialize, JsonSchema)]
#[alkahest(Deserialize, Serialize, SerializeRef, Formula)]
pub struct ModifyUnitStats {
    #[serde(default, skip_serializing_if = "is_default")]	
    pub max_hp: Modify<i64>,
	#[serde(default, skip_serializing_if = "is_default")]
	#[inline_parsing]
    pub damage: ModifyPower,
	#[inline_parsing]
	#[serde(default, skip_serializing_if = "is_default")]
    pub defence: ModifyDefence,
	#[serde(default, skip_serializing_if = "is_default")]
    pub max_moves: Modify<i64>,
	#[serde(default, skip_serializing_if = "is_default")]
	pub speed: Modify<i64>,
	#[serde(default, skip_serializing_if = "is_default")]
    pub vamp: Modify<i16>,
	#[serde(default, skip_serializing_if = "is_default")]
    pub regen: Modify<i16>,

	#[serde(default, skip_serializing_if = "is_default")]
	pub pierce: Modify<i64>,
	#[serde(default, skip_serializing_if = "is_default")]
	pub true_damage: Modify<i64>,
	#[serde(default, skip_serializing_if = "is_default")]
	pub flank: Modify<i64>,
}
impl ModifyUnitStats {
    pub fn display_string(&self, locale: &Locale) -> Vec<String> {
        let mut strings = vec![];
        if self.max_hp != Modify::default() {
            strings.push(format!(
                "{}: {}",
                locale.get("unitstats_hp"),
                self.max_hp.to_string()
            ));
        }

        if self.damage.hand != Modify::default() {
            strings.push(format!(
                "{}: {}",
                locale.get("unitstats_attack_hand"),
                self.damage.hand.to_string()
            ));
        }
        if self.damage.ranged != Modify::default() {
            strings.push(format!(
                "{}: {}",
                locale.get("unitstats_attack_ranged"),
                self.damage.ranged.to_string()
            ));
        }
        if self.damage.magic != Modify::default() {
            strings.push(format!(
                "{}: {}",
                locale.get("unitstats_attack_magic"),
                self.damage.magic.to_string()
            ));
        }

        if self.defence.hand_units != Modify::default() {
            strings.push(format!(
                "{}: {}",
                locale.get("unitstats_defence_hand"),
                self.defence.hand_units.to_string()
            ));
        }
        if self.defence.ranged_units != Modify::default() {
            strings.push(format!(
                "{}: {}",
                locale.get("unitstats_defence_ranged"),
                self.defence.ranged_units.to_string()
            ));
        }
        if self.defence.magic_units != Modify::default() {
            strings.push(format!(
                "{}: {}",
                locale.get("unitstats_defence_magic"),
                self.defence.magic_units.to_string()
            ));
        }
        if self.defence.death_magic != Modify::default() {
            strings.push(format!(
                "{}: {}",
                process_locale("$unitstats_defence_magic_death", &locale),
                self.defence.death_magic.to_string()
            ));
        }
        if self.defence.life_magic != Modify::default() {
            strings.push(format!(
                "{}: {}",
                process_locale("$unitstats_defence_magic_life", &locale),
                self.defence.life_magic.to_string()
            ));
        }
        if self.defence.elemental_magic != Modify::default() {
            strings.push(format!(
                "{}: {}",
                process_locale("$unitstats_defence_magic_elemental", &locale),
                self.defence.elemental_magic.to_string()
            ));
        }
        if self.max_moves != Modify::default() {
            strings.push(format!(
                "{}: {}",
                locale.get("unitstats_moves"),
                self.max_moves.to_string()
            ));
        }
        if self.speed != Modify::default() {
            strings.push(format!(
                "{}: {}",
                locale.get("unitstats_speed"),
                self.speed.to_string()
            ));
        }
        if self.vamp != Modify::default() {
            strings.push(format!(
                "{}: {}",
                locale.get("unitstats_vamp"),
                self.vamp.to_string()
            ));
        }
        if self.regen != Modify::default() {
            strings.push(format!(
                "{}: {}",
                locale.get("unitstats_regen"),
                self.regen.to_string()
            ));
        }
        strings
    }
    pub fn apply(&self, stats: &UnitStats, magic_type: Option<MagicType>) -> UnitStats {
        let mut n_stats = UnitStats::default();
        //n_stats.hp = self.hp.apply(stats.hp)
        n_stats.max_hp = self.max_hp.apply(stats.max_hp);
        n_stats.damage = self.damage.apply(&stats.damage, magic_type);
        n_stats.defence = self.defence.apply(&stats.defence);
        //n_stats.moves = self.moves.apply(stats.moves);
        n_stats.max_moves = self.max_moves.apply(stats.max_moves);
        n_stats.speed = self.speed.apply(stats.speed);
        n_stats.vamp = self.vamp.apply(stats.vamp);
        n_stats.regen = self.regen.apply(stats.regen);

		n_stats.flank_mod = self.flank;
		n_stats.true_damage = self.true_damage;
		n_stats.pierce = self.pierce;
			
        n_stats
    }
}

#[test]
fn modifiers() {
	for i in 0usize..10 {
		let modify = Modify {
			set: None,
			add: Some(10),
			percent_add: Some(Percent::new(10)),
		};
		let mut mul_modify = Modify::default();
		mul_modify.add_val(10 * i);
		mul_modify.percent_add(Percent::new(10 * i as i16));
		assert_eq!(modify * i, mul_modify);
	}
}
