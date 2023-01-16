use {
    super::{deco::MapDeco, object::MapObject, tile::Tile},
    crate::lib::{battle::army::Army, time::time::Time},
};

pub const MAP_SIZE: usize = 50;
#[derive(Clone, Debug)]
pub struct GameMap {
    pub time: Time,
    pub tilemap: [[usize; MAP_SIZE]; MAP_SIZE],
    pub decomap: [[Option<MapDeco>; MAP_SIZE]; MAP_SIZE],
    pub armys: Vec<Army>,
}
