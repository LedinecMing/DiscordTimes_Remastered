use super::{deco::MapDeco, object::{MapBuildingdata, ObjectInfo}, tile::*};
use crate::lib::{battle::army::{Army, Relations}, time::time::Time};

pub type Tilemap<T> = [[T; MAP_SIZE]; MAP_SIZE];
#[derive(Copy, Clone, Debug)]
pub struct HitboxTile {
    pub passable: bool,
    pub need_transport: bool,
    pub building: Option<usize>,
    pub army: Option<usize>
}
impl HitboxTile {
	pub fn passable(&self) -> bool {
		self.army.is_none() && self.passable
	}
}
impl Default for HitboxTile {
	fn default() -> Self {
		HitboxTile { passable: true, need_transport: false, building: None, army: None }
	}
}

#[derive(Clone, Debug, Default)]
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
impl Default for GameMap {
	fn default() -> Self {
		GameMap {
			time: Default::default(),
			tilemap: [[0; MAP_SIZE]; MAP_SIZE],
			decomap: [[None; MAP_SIZE]; MAP_SIZE],
			hitmap: [[HitboxTile::default(); MAP_SIZE]; MAP_SIZE],
			buildings: Vec::new(),
			armys: Vec::new(),
			relations: Default::default()
		}
	}
}
impl GameMap {
    pub fn calc_hitboxes(&mut self, objects: &[ObjectInfo]) {
        for ((tileline, decoline), x) in self.tilemap.iter().zip(self.decomap.iter()).zip(0..MAP_SIZE) {
            for ((tile, deco), y) in tileline.iter().zip(decoline.iter()).zip(0..MAP_SIZE) {
                let hitbox = &mut self.hitmap[x][y];
                hitbox.need_transport = TILES[*tile].need_transport();
            }
        }
        self.recalc_armies_hitboxes();
        for (i, building) in self.buildings.iter().enumerate() {
			let size = objects[building.id].size;
            for x in 0..size.0 {
                for y in 0..size.1 {
					let hitbox = &mut self.hitmap[building.pos.0 + x as usize][building.pos.1 + y as usize];
					hitbox.building = Some(i);
                }
            }
        }
    }
    pub fn recalc_armies_hitboxes(&mut self) {
        self.hitmap.iter_mut().for_each(|arr| arr.iter_mut().for_each(|mut el| {el.army = None;}));
        for (i, army) in self.armys.iter().enumerate() {
			if !army.active || army.defeated {
				continue
			}
            let (x, y) = army.pos;
            self.hitmap[x][y].army = Some(i);
        }
    }
	
}
