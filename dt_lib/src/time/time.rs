use std::ops::{Sub, SubAssign};

use advini::{Ini, IniParseError, SEPARATOR};
use alkahest::alkahest;
use derive_more::{Add, AddAssign};
use nom::{Parser, bytes::complete::take, character::complete::digit1, combinator::map_res, multi::many0, sequence::terminated};
use serde::{self, de::value};

const HOUR: u64 = 60;
const DAY: u64 = 24;
const MONTH: u64 = 31;
const YEAR: u64 = 12;
const AGE: u64 = 100;

#[repr(u64)]
pub enum Data {
    YEAR = 12 * Data::MONTH as u64,
    MONTH = 31 * Data::DAY as u64,
    DAY = 24 * Data::HOUR as u64,
    HOUR = 60u64,
    MINUTES = 1u64,
}

#[derive(
    Add,
    AddAssign,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Clone,
    Debug,
    Default,
    Copy,
    serde::Deserialize,
    serde::Serialize,
)]
#[alkahest(Deserialize, Serialize, SerializeRef, Formula)]
pub struct Time {
    pub minutes: u64,
}
impl Ini<'_> for Time {
    fn eat<'a>(input: &'a str, _additional: Self::Arg) -> Result<(&'a str, Self), IniParseError> {
		let (rest, times) = many0(
			terminated(map_res(digit1, |x: &str| x.parse::<u64>()), take(1usize))
		).parse(input)?;
        let len = times.len().min(5);
        let value = times
            .iter()
            .enumerate()
            .map(|(i, v)| match i {
                x if x == len - 1 => v * Data::HOUR as u64,
                x if x == len - 2 => v * Data::DAY as u64,
                x if x == len - 3 => v * Data::MONTH as u64,
                x if x == len - 4 => v * Data::YEAR as u64,
                _ => 0,
            })
            .sum();
        Ok((rest, Time::new(value)))
    }
	fn vomit(&self, additional: Self::Arg) -> String {
        if self.get_years() > 0 {
            self.to_data([Data::YEAR, Data::MONTH, Data::DAY, Data::HOUR], ":")
        } else if self.get_months() > 0 {
            self.to_data([Data::MONTH, Data::DAY, Data::HOUR], ":")
        } else if self.get_day() > 0 {
            self.to_data([Data::DAY, Data::HOUR], ":")
        } else {
            self.to_data([Data::HOUR], ":")
        }
    }
}
impl Time {
    pub fn get_minute(&self) -> u64 {
        self.minutes % 60
    }
    pub fn get_hours(&self) -> u64 {
        self.minutes / HOUR
    }
    pub fn get_hour(&self) -> u64 {
        self.get_hours() % DAY
    }
    pub fn get_days(&self) -> u64 {
        self.get_hours() / DAY
    }
    pub fn get_day(&self) -> u64 {
        self.get_days() % MONTH
    }
    pub fn get_months(&self) -> u64 {
        self.get_days() / MONTH
    }
    pub fn get_month(&self) -> u64 {
        self.get_months() % YEAR
    }
    pub fn get_years(&self) -> u64 {
        self.get_months() / YEAR
    }
    pub fn get_year(&self) -> u64 {
        self.get_years() % AGE
    }

    pub fn new(minutes: u64) -> Self {
        Time { minutes }
    }
    pub fn from_data<const DATA: usize>(data: impl Into<String>, data_repr: [Data; DATA]) -> Self {
        Time::new(
            data.into()
                .split(|ch: char| !ch.is_ascii_digit())
                .zip(data_repr)
                .map(|(num, data)| num.parse::<u64>().unwrap() * data as u64)
                .sum(),
        )
    }
    pub fn to_data<const DATA: usize>(&self, data_repr: [Data; DATA], split: &str) -> String {
        let data = data_repr
            .iter()
            .map(|data| {
                match data {
                    Data::YEAR => self.get_years(),
                    Data::MONTH => self.get_month(),
                    Data::DAY => self.get_day(),
                    Data::HOUR => self.get_hour(),
                    Data::MINUTES => self.minutes % 60,
                }
                .to_string()
                    + split
            })
            .collect::<String>();
        data[..data.len() - 1].into()
    }
}
impl Sub for Time {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self::Output {
        Self {
            minutes: rhs.minutes.saturating_sub(rhs.minutes),
        }
    }
}
impl SubAssign for Time {
    fn sub_assign(&mut self, rhs: Self) {
        self.minutes = self.minutes.saturating_sub(rhs.minutes);
    }
}
