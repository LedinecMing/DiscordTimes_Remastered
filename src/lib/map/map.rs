use super::{deco::MapDeco, object::MapBuildingdata, tile::Tile};
use crate::lib::{battle::army::Army, time::time::Time};

pub const MAP_SIZE: usize = 50;
#[derive(Clone, Debug)]
pub struct GameMap {
    pub time: Time,
    pub tilemap: [[usize; MAP_SIZE]; MAP_SIZE],
    pub decomap: [[Option<usize>; MAP_SIZE]; MAP_SIZE],
    pub buildings: Vec<MapBuildingdata>,
    pub armys: Vec<Army>,
}
