use super::{
    object::{MapBuildingdata, ObjectInfo},
    tile::*,
};
use crate::{
    battle::army::{Army, Relations},
    time::time::Time,
};
use advini::{Ini, IniParseError, Section, SectionError, Sections};
use alkahest::alkahest;

pub type Tilemap<T> = [[T; MAP_SIZE]; MAP_SIZE];
#[derive(Copy, Clone, Debug)]
#[alkahest(Deserialize, Serialize, SerializeRef, Formula)]
pub struct HitboxTile {
    pub passable: bool,
    pub need_transport: bool,
    pub building: Option<usize>,
    pub army: Option<usize>,
}
impl HitboxTile {
    pub fn passable(&self) -> bool {
        self.army.is_none() && self.passable
    }
}
impl Default for HitboxTile {
    fn default() -> Self {
        HitboxTile {
            passable: true,
            need_transport: false,
            building: None,
            army: None,
        }
    }
}

#[derive(Clone, Debug, Default, Sections)]
#[alkahest(Deserialize, Serialize, SerializeRef, Formula)]
pub struct FractionsRelations {
    #[default_value = "Relations::default()"]
    pub ally: Relations,
    #[default_value = "Relations::default()"]
    pub neighbour: Relations,
    #[default_value = "Relations::default()"]
    pub enemy: Relations,
}
impl FractionsRelations {
    fn new(ally: Relations, neighbour: Relations, enemy: Relations) -> Self {
        Self {
            ally,
            neighbour,
            enemy,
        }
    }
}
pub const MAP_SIZE: usize = 50;

#[derive(Clone, Debug, Default, Sections)]
#[alkahest(Deserialize, Serialize, SerializeRef, Formula)]
pub struct StartStats {
    #[alias([start_time])]
    pub time: Time,
    #[alias([start_money])]
    pub money: u64,
    #[alias([start_mana])]
    pub mana: u64,
}
impl StartStats {
    pub fn new(time: Time, money: u64, mana: u64) -> Self {
        Self { time, money, mana }
    }
}
#[derive(Clone, Debug, Sections)]
#[alkahest(Deserialize, Serialize, SerializeRef, Formula)]
pub struct GameMap {
    #[inline_parsing]
    pub start: StartStats,
    #[unused]
    pub time: Time,
    #[unused]
    pub tilemap: Tilemap<usize>,
    #[unused]
    pub decomap: Tilemap<Option<usize>>,
    #[unused]
    pub hitmap: Tilemap<HitboxTile>,
    #[unused]
    pub buildings: Vec<MapBuildingdata>,
    #[unused]
    pub armys: Vec<Army>,
    #[inline_parsing]
    pub relations: FractionsRelations,
    #[unused]
    pub pause: bool,
}
impl Default for GameMap {
    fn default() -> Self {
        GameMap {
            start: Default::default(),
            time: Default::default(),
            tilemap: [[0; MAP_SIZE]; MAP_SIZE],
            decomap: [[None; MAP_SIZE]; MAP_SIZE],
            hitmap: [[HitboxTile::default(); MAP_SIZE]; MAP_SIZE],
            buildings: Vec::new(),
            armys: Vec::new(),
            relations: Default::default(),
            pause: false,
        }
    }
}
impl GameMap {
    pub fn new(start: StartStats, relations: FractionsRelations) -> Self {
        let time = start.time;
        Self {
            start,
            relations,
            time,
            ..Default::default()
        }
    }
    pub fn calc_hitboxes(&mut self, objects: &[ObjectInfo]) {
        for ((tileline, decoline), x) in self
            .tilemap
            .iter()
            .zip(self.decomap.iter())
            .zip(0..MAP_SIZE)
        {
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
                    let hitbox =
                        &mut self.hitmap[building.pos.0 + x as usize][building.pos.1 + y as usize];
                    hitbox.building = Some(i);
                }
            }
        }
    }
    pub fn recalc_armies_hitboxes(&mut self) {
        self.hitmap.iter_mut().for_each(|arr| {
            arr.iter_mut().for_each(|el| {
                el.army = None;
            })
        });
        for (i, army) in self.armys.iter().enumerate() {
            if !army.active || army.defeated {
                continue;
            }
            let (x, y) = army.pos;
            self.hitmap[x][y].army = Some(i);
        }
    }
}
