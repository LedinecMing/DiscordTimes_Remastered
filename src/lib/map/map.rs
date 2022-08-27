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

pub const MAP_SIZE: usize = 1;
pub struct GameMap {
    pub time: Time,
    pub tilemap: [[Tile; MAP_SIZE]; MAP_SIZE],
    pub objectmap: [[Option<Box<dyn MapObject>>; MAP_SIZE]; MAP_SIZE],
    pub decomap: [[Vec<MapDeco>; MAP_SIZE]; MAP_SIZE],
    pub armys: Vec<Army>
}

