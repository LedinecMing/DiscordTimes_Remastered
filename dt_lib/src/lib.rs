pub mod battle;
pub mod bonuses;
pub mod console;
pub mod effects;
pub mod items;
pub mod locale;
pub mod map;
pub mod mutrc;
pub mod network;
pub mod new_forms;
pub mod parse;
pub mod time;
pub mod units;

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
