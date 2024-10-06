pub mod battle;
pub mod bonuses;
pub mod effects;
pub mod items;
pub mod map;
pub mod mutrc;
pub mod network;
pub mod parse;
pub mod time;
pub mod units;
pub mod locale;

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
