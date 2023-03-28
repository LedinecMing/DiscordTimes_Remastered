use super::{deco::MapDeco, object::MapBuildingdata, tile::*};
use crate::lib::{battle::army::{Army, Relations}, time::time::Time};

pub type Tilemap<T> = [[T; MAP_SIZE]; MAP_SIZE];
#[derive(Copy, Clone, Debug)]
pub struct HitboxTile {
    pub passable: bool,
    pub need_transport: bool,
    pub building: Option<usize>,
    pub army: Option<usize>
}

#[derive(Clone, Debug)]
pub struct FractionsRelations {
    pub ally: Relations,
    pub neighbour: Relations,
    pub enemy: Relations
}

pub const MAP_SIZE: usize = 50;
#[derive(Clone, Debug)]
pub struct GameMap {
    pub time: Time,
    pub tilemap: Tilemap<usize>,
    pub decomap: Tilemap<Option<usize>>,
    pub hitmap: Tilemap<HitboxTile>,
    pub buildings: Vec<MapBuildingdata>,
    pub armys: Vec<Army>,
    pub relations: FractionsRelations
}
impl GameMap {
    pub fn calc_hitboxes(&mut self) {

        for ((tileline, decoline), x) in self.tilemap.iter().zip(self.decomap.iter()).zip(0..MAP_SIZE) {
            for ((tile, deco), y) in tileline.iter().zip(decoline.iter()).zip(0..MAP_SIZE) {
                let hitbox = &mut self.hitmap[x][y];
                hitbox.need_transport = TILES[*tile].need_transport();
            }
        }
        self.recalc_armies_hitboxes();
        for (building, i) in self.buildings.iter().zip(0..self.buildings.len()) {
            for x in 0..building.size.0 {
                for y in 0..building.size.1 {
                    self.hitmap[building.pos.0 + x as usize][building.pos.1 + y as usize].building = Some(i);
                }
            }
        }
    }
    pub fn recalc_armies_hitboxes(&mut self) {
        self.hitmap.iter_mut().for_each(|arr| arr.iter_mut().for_each(|mut el| {el.army = None;}));
        for (army, i) in self.armys.iter().zip(0..self.armys.len()) {
            let (x, y) = army.pos;
            self.hitmap[x][y].army = Some(i);
        }
    }
}
