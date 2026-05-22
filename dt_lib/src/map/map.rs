use std::{
    ops::{Index, IndexMut},
    path::Path,
};

use super::{
    deco::MapDeco,
    event::Events,
    object::{MapBuildingdata, ObjectInfo},
    tile::*,
};
use crate::{
    battle::{army::Army, control::Relations},
    parse::StupidReader,
    time::time::Time,
};
use advini::{Ini, IniParseError, Section, SectionError, Sections};
use alkahest::alkahest;
use itertools::Itertools;
use nom::{Parser, bytes::complete::tag};
use num::integer::Roots;
use std::fs;

pub type Tilemap<T> = [[T; MAP_SIZE]; MAP_SIZE];
#[derive(Clone, Debug, Default)]
#[alkahest(Deserialize, Serialize, SerializeRef, Formula)]
pub struct TileMap<T> {
    pub inner: Vec<T>,
    pub size: usize,
}
impl<'z, T: Ini<'z, Arg = ()>> Ini<'z> for TileMap<T> {
	fn eat<'a>(mut input: &'a str, _additional: Self::Arg) -> Result<(&'a str, Self), IniParseError> {
		let mut tiles = vec![];
		loop {
			let (rest, res ) = T::eat(input, _additional)?;
			input = rest;
			tiles.push(res);
			input = tag(",").parse(input)?.0;
		}
		Ok((input, Self {
			inner: tiles,
			size: tiles.len().sqrt()
		}))
	}
	fn vomit(&self, additional: Self::Arg) -> String {
		let mut res = String::new();
		for v in &self.inner {
			res.push_str(&v.vomit(additional));
		}
		res
	}
	
}
impl<T> TileMap<T> {
    pub fn new(iter: impl Iterator<Item = T>) -> Self {
        let inner = iter.collect::<Vec<_>>();
        let len = inner.len();
        Self {
            inner,
            size: len.sqrt() as usize,
        }
    }
}
impl<T> AsMut<TileMap<T>> for TileMap<T> {
    fn as_mut(&mut self) -> &mut TileMap<T> {
        self
    }
}
impl<T> IntoIterator for TileMap<T> {
    type IntoIter = <Vec<T> as IntoIterator>::IntoIter;
    type Item = T;
    fn into_iter(self) -> Self::IntoIter {
        self.inner.into_iter()
    }
}
impl<T> Index<(usize, usize)> for TileMap<T> {
    type Output = T;
    fn index(&self, index: (usize, usize)) -> &Self::Output {
        &self.inner[index.1 + index.0 * self.size]
    }
}
impl<T> IndexMut<(usize, usize)> for TileMap<T> {
    fn index_mut(&mut self, index: (usize, usize)) -> &mut Self::Output {
        &mut self.inner[index.1 + index.0 * self.size]
    }
}
#[derive(Copy, Clone, Debug, PartialEq)]
#[alkahest(Deserialize, Serialize, SerializeRef, Formula)]
pub struct HitboxTile {
    pub passable: bool,
    pub deco_blocked: bool,
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
            deco_blocked: false,
            passable: false,
            need_transport: false,
            building: None,
            army: None,
        }
    }
}

#[derive(Clone, Debug, Default, Sections, PartialEq)]
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

#[derive(Clone, Debug, Default, advini::Ini, PartialEq)]
#[alkahest(Deserialize, Serialize, SerializeRef, Formula)]
pub enum ScenarioVariant {
    #[default]
    Single,
    Start(String),
    Series(String),
}
#[derive(Clone, Debug, Default, Sections, PartialEq)]
#[alkahest(Deserialize, Serialize, SerializeRef, Formula)]
pub struct NextMapSettings {
    #[default_value = "true"]
    pub save_mana: bool,
    #[default_value = "true"]
    pub save_gold: bool,
    #[default_value = "true"]
    pub save_xp_and_lvl: bool,
    #[default_value = "true"]
    pub save_own_items: bool,
    #[default_value = "true"]
    pub save_all_items: bool,
    #[default_value = "true"]
    pub save_all_troops: bool,
}
#[derive(Clone, Debug, Default, Sections, PartialEq)]
#[alkahest(Deserialize, Serialize, SerializeRef, Formula)]
pub struct StartStats {
    #[default_value = "\"New map\""]
    pub name: String,
    #[default_value = "String::new()"]
    pub description: String,
    #[unused]
    #[default_value = "0usize"]
    pub seed: usize,
    pub winning_event_id: usize,
    pub losing_event_id: usize,
    #[default_value = "ScenarioVariant::Single"]
    pub scenario: ScenarioVariant,
    #[alias([start_time])]
    pub time: Time,
}
#[derive(Clone, Debug)]
#[alkahest(Deserialize, Serialize, SerializeRef, Formula)]
pub struct GameMap {
    //#[inline_parsing]
    pub start: StartStats,
    //#[unused]
    pub time: Time,
    //#[unused]
    pub tilemap: TileMap<usize>,
    //#[unused]
    pub decomap: Vec<MapDeco>,
	//#[unused]
    pub eventmap: TileMap<Vec<usize>>,
    //#[unused]
    pub hitmap: TileMap<HitboxTile>,
    //#[unused]
    pub buildings: Vec<MapBuildingdata>,
    //#[unused]
    pub armys: Vec<Army>,
    //#[inline_parsing]
    pub relations: FractionsRelations,
    //#[unused]
    pub pause: bool,
}
impl Default for GameMap {
    fn default() -> Self {
        GameMap {
            start: Default::default(),
            time: Default::default(),
            tilemap: Default::default(),
            decomap: Default::default(),
            eventmap: Default::default(),
            hitmap: Default::default(),
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
        for (i, _) in &mut self.tilemap.inner.iter().enumerate() {
            self.hitmap.inner[i].need_transport = TILES[self.tilemap.inner[i]].need_transport();
			self.hitmap.inner[i].passable = TILES
                [self.tilemap.inner[i]]
                .walkspeed
                != 0;
        }
        self.recalc_armies_hitboxes();
        for (i, building) in self.buildings.iter().enumerate() {
            let size = objects.get(building.id).unwrap_or(&objects[0]).size;
            for x in 0..size.0 {
                for y in 0..size.1 {
                    if building.pos.0 + x as usize >= self.tilemap.size
                        || building.pos.1 + y as usize >= self.tilemap.size
                    {
                        continue;
                    }
                    let hitbox = &mut self.hitmap
                        [(building.pos.0 + x as usize, building.pos.1 + y as usize)];
                    hitbox.building = Some(i);
                    hitbox.passable = TILES
                        [self.tilemap[(building.pos.0 + x as usize, building.pos.1 + y as usize)]]
                        .walkspeed
                        != 0;
                }
            }
        }
    }
    pub fn recalc_deco_hitboxes(&mut self) {}
    pub fn recalc_armies_hitboxes(&mut self) {
        for hit in self.hitmap.inner.iter_mut() {
            hit.army = None;
        }
        for (i, army) in self.armys.iter().enumerate() {
            /*if !army.active || army.defeated {
                continue;
            }*/
            self.hitmap[army.pos].army = Some(i);
        }
    }
}

pub fn export(events: &Events, to: &'static str) {
    let res = events
        .iter()
        .map(|event| {
            format!(
                "[Event {}]\n{}\n",
                event.name,
                event
                    .to_section(Default::default())
                    .iter()
                    .map(|(k, v)| format!("{k}={v}"))
                    .join("\n")
            )
        })
        .join("\n");
    fs::write(to, res);
}
