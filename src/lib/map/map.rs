use {
    crate::lib::{
        time::time::Time,
        battle::army::Army
    },
    super::{
        deco::MapDeco,
        object::MapObject,
        tile::Tile
    }
};

pub const MAP_SIZE: usize = 50;
#[derive(Clone)]
pub struct GameMap {
    pub time: Time,
    pub tilemap: [[usize; MAP_SIZE]; MAP_SIZE],
    pub decomap: [[Option<MapDeco>; MAP_SIZE]; MAP_SIZE],
    pub armys: Vec<Army>
}

