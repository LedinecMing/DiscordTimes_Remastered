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

/// Точка событий/фонарик карты — единая структура, как в оригинале игры
/// (convert::LightOrEvent): одна сущность «фонарик + опции», а не два
/// списка. Из гайда старого редактора (old_editor_guide.md §5):
/// - обычный фонарик (Lamp) — точка без событий;
/// - точка локального события (map_model==9) — может быть И фонариком,
///   И содержать локальные события одновременно; чекбокс «активен в
///   начале игры» делает её обычным фонариком (active_from_start).
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
#[alkahest(Deserialize, Serialize, SerializeRef, Formula)]
pub struct MapLantern {
    pub x: usize,
    pub y: usize,
    /// Id точки (LightOrEvent.id).
    pub id: usize,
    /// Модель на карте: 9 — точка локального события, прочие (8) —
    /// обычный фонарик.
    pub map_model: u8,
    /// Радиус света (0..=24; валидатор RadiusRangeCheck).
    pub light_radius: u8,
    /// Обычный фонарик, активен с начала игры (map_model != 9).
    pub active_from_start: bool,
    /// Id локальных событий точки (конвертируются из LightOrEvent.events).
    pub events: Vec<usize>,
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
    /// Точки событий/фонарики (заменили eventmap — единая структура,
    /// см. MapLantern; события клетки — events_at(x, y)).
    pub lanterns: Vec<MapLantern>,
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
            lanterns: Vec::new(),
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

    /// События клетки (x, y): события точек в этой клетке (замена
    /// eventmap[(x, y)] для исполнителя и рендера событий).
    pub fn events_at(&self, x: usize, y: usize) -> &[usize] {
        static EMPTY: [usize; 0] = [];
        self.lanterns
            .iter()
            .find(|l| l.x == x && l.y == y)
            .map(|l| l.events.as_slice())
            .unwrap_or(&EMPTY)
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
            // building.id — декларативный ObjectInfo.index из Objects.ini
            // (сопоставление (group,variant) в convert.rs), а не позиция в
            // векторе реестра. Позиционный lookup давал всем строениям размер
            // заглушки objects[0] (1×1) — хитбоксы схлопывались в тайл якоря.
            let Some(size) = objects.iter().find(|o| o.index == building.id).map(|o| o.size)
            else {
                // Грязный id из карты: строение без спрайта/хитбокса, не паникуем.
                log::warn!("calc_hitboxes: building id {} не найден в Objects", building.id);
                continue;
            };
            for x in 0..size.0 {
                for y in 0..size.1 {
                    // Хитбокс уходит ВЛЕВО-ВВЕРХ от якоря и покрывает ровно
                    // спан рендера bake: [pos-w+1..pos] × [pos-h+1..pos]
                    // (спрайт draw от (pos-w+1, pos-h+1), правый-нижний угол
                    // = якорь). Прежние +1/+1 сдвигали хитбокс на клетку
                    // вправо-вниз от нарисованного спрайта.
                    let tx = building.pos.0 as isize - x as isize;
                    let ty = building.pos.1 as isize - y as isize;
                    if tx < 0 || ty < 0 || tx as usize >= self.tilemap.size
                        || ty as usize >= self.tilemap.size
                    {
                        continue;
                    }
                    let hitbox = &mut self.hitmap[(tx as usize, ty as usize)];
                    hitbox.building = Some(i);
                    hitbox.passable = TILES
                        [self.tilemap[(tx as usize, ty as usize)]]
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

#[cfg(test)]
mod tests {
    use super::*;

    /// Объекты реестра: 0 — «заглушка» 1×1 (как Tree0 в Objects.ini),
    /// 500 — строение 2×2, 515 — строение 3×2. Позиции в векторе НЕ
    /// совпадают с index — реестр отражает Objects.ini (index задан явно).
    fn test_objects() -> Vec<ObjectInfo> {
        vec![
            ObjectInfo {
                name: "Tree0".into(),
                path: "Tree000.png".into(),
                category: String::new(),
                obj_type: crate::map::object::ObjectType::MapDeco { id: 0 },
                index: 0,
                size: (1, 1),
            },
            ObjectInfo {
                name: "Town0".into(),
                path: "Town000.png".into(),
                category: String::new(),
                obj_type: crate::map::object::ObjectType::Building { group: 6, variant: 0 },
                index: 500,
                size: (2, 2),
            },
            ObjectInfo {
                name: "Village4".into(),
                path: "Village004.png".into(),
                category: String::new(),
                obj_type: crate::map::object::ObjectType::Building { group: 6, variant: 4 },
                index: 515,
                size: (3, 2),
            },
        ]
    }

    fn test_map() -> GameMap {
        let size = 8usize;
        let mut m = GameMap::default();
        m.tilemap = TileMap::new(vec![4usize; size * size].into_iter());
        m.hitmap = TileMap::new(vec![HitboxTile::default(); size * size].into_iter());
        m
    }

    /// Формула рендера (bake.rs): спрайт строения с якорем pos рисуется
    /// от (pos-w+1, pos-h+1) до pos включительно. Хитбокс обязан совпадать.
    fn render_span(pos: (usize, usize), size: (u8, u8)) -> Vec<(usize, usize)> {
        let (w, h) = (size.0 as isize, size.1 as isize);
        let mut tiles = vec![];
        for tx in (pos.0 as isize - w + 1)..=(pos.0 as isize) {
            for ty in (pos.1 as isize - h + 1)..=(pos.1 as isize) {
                tiles.push((tx as usize, ty as usize));
            }
        }
        tiles
    }

    fn building(id: usize, pos: (usize, usize)) -> crate::map::object::MapBuildingdata {
        crate::map::object::MapBuildingdata {
            name: String::new(),
            desc: String::new(),
            owner_name: String::new(),
            id,
            events: vec![],
            variant: crate::map::object::BuildingVariant::Town,
            market: None,
            recruitment: None,
            pos,
            owner: None,
            garrison: vec![],
            additional_defense: 0,
            gold_income: 0,
            mana_income: 0,
            spells_to_learn: vec![],
            relations: Default::default(),
            group: 0,
        }
    }

    #[test]
    fn hitbox_matches_render_span_and_object_index() {
        let objects = test_objects();
        let mut m = test_map();
        // building.id — ДЕКЛАРАТИВНЫЙ ObjectInfo.index (сопоставление
        // (group,variant) в convert.rs), а не позиция в векторе реестра.
        m.buildings = vec![building(500, (4, 4)), building(515, (7, 7))];
        m.calc_hitboxes(&objects);

        for (bi, b) in m.buildings.iter().enumerate() {
            let size = objects
                .iter()
                .find(|o| o.index == b.id)
                .map(|o| o.size)
                .expect("building id должен попадать на ObjectInfo.index");
            for tile in render_span(b.pos, size) {
                assert_eq!(
                    m.hitmap[tile].building,
                    Some(bi),
                    "тайл {:?} строения {} (id={}, size={:?}) не в хитбоксе",
                    tile,
                    bi,
                    b.id,
                    size
                );
            }
        }
        // Вне спанов строений hitmap.building пуст.
        assert_eq!(m.hitmap[(0, 0)].building, None);
        assert_eq!(m.hitmap[(7, 0)].building, None);
    }

    #[test]
    fn hitbox_of_missing_id_falls_back_to_nothing() {
        // Битый id (нет в реестре) — не паникует (правило AGENTS.md п.10
        // про грязные id из карт) и не рисует хитбокс 1×1 от заглушки:
        // у строения просто нет тайлов.
        let objects = test_objects();
        let mut m = test_map();
        m.buildings = vec![building(9999, (4, 4))];
        m.calc_hitboxes(&objects);
        for (tx, ty) in render_span((4, 4), (1, 1)) {
            assert_eq!(
                m.hitmap[(tx, ty)].building, None,
                "битый id не должен помечать тайлы"
            );
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
