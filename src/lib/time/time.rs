use {
    derive_more::{Add, Sub}
};


const HOUR: u64 = 60;
const DAY: u64 = 24;
const MONTH: u64 = 31;
const YEAR: u64 = 12;
const AGE: u64 = 100;

#[derive(Add, Sub, Clone)]
pub struct Time {
    pub minutes: u64
}
impl Time {
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

    pub fn new(_minutes: u64) -> Self {
        todo!("This module is boilerplate shit and should be replaced when actually used\n\
    Variants are: https://crates.io/crates/ticktime")
    }
}