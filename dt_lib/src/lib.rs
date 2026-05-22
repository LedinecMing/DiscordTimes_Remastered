#![allow(dead_code)]
#![allow(unused_imports)]
#![allow(unused_variables)]
#![allow(unused)]

#![feature(associated_type_defaults)]

use std::collections::HashMap;

pub mod battle;
pub mod bonuses;
pub mod effects;
pub mod hwid;
pub mod items;
pub mod locale;
pub mod map;
pub mod mutrc;
pub mod network;
pub mod parse;
pub mod time;
pub mod units;
pub mod registry;

#[repr(u32)]
pub enum Menu {
    Main,
    Start,
    Load,
    Settings,
    Authors,
    UnitView,
    Battle,
    Items,
    Connect,
    ConnectBattle,
}
