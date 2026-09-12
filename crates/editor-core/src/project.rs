//! Проект карты: обёртка над доменом dt_lib.
//!
//! Источник истины — типы `dt_lib` (ТЗ §6): [`GameMap`] несёт tilemap
//! (`TileMap<usize>`), decomap, buildings ([`MapBuildingdata`]), armys
//! ([`Army`]) и стартовые настройки ([`StartStats`]); события — [`Event`].
//! Свои типы здесь только то, чего в dt_lib нет: [`ProjectMeta`] (данные
//! редактора: формат, автор, локали, словарь строк) и [`Light`] (фонарик —
//! в GameMap отсутствует; в .DTm это `convert::LightOrEvent`).
//!
//! JSON-сериализация (бриф): снапшот редактируемых данных. Армии/строения в
//! снапшот не входят — их сериализация идёт через advini/alkahest в
//! editor-serde (Phase 1, .DTm-совместимость).

use dt_lib::map::event::{Event, Events};
use dt_lib::map::map::{GameMap, TileMap};
use dt_lib::map::object::MapBuildingdata;
use serde::{Deserialize, Serialize};

/// Позиция на карте (колонка, строка).
pub type Pos = (usize, usize);

/// Типы тайлов из `dt_lib::map::tile::TILES` (16 штук, индекс = id).
pub const TILE_COUNT: usize = 16;

/// Метаданные редактора (не игры): формат, автор, локали, словарь строк.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ProjectMeta {
    /// Версия формата проекта редактора.
    pub format_version: u32,
    pub author: String,
    /// Поддерживаемые языки локализации (например, ["RUS", "ENG"]).
    pub locales: Vec<String>,
    /// Ключи локализации: key -> (язык -> текст).
    pub strings: std::collections::BTreeMap<String, std::collections::BTreeMap<String, String>>,
}

impl ProjectMeta {
    pub fn localized(&self, key: &str, lang: &str) -> Option<&str> {
        self.strings
            .get(key)
            .and_then(|by_lang| by_lang.get(lang))
            .map(String::as_str)
    }
}

/// Фонарик карты: позиция и радиус света (0..=24, см. editor-validators).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Light {
    pub x: usize,
    pub y: usize,
    pub radius: u64,
}

/// Проект карты: обёртка над `GameMap` + данные редактора.
#[derive(Clone, Debug, Default)]
pub struct MapProject {
    /// Данные редактора (формат, автор, локали).
    pub editor: ProjectMeta,
    /// Игровая карта — источник истины dt_lib (start, tilemap, decomap,
    /// buildings, armys, relations, ...).
    pub map: GameMap,
    /// События карты (в GameMap не входят: игра хранит их рядом с картой).
    pub events: Events,
    /// Фонарики (радиус 0..=24; валидатор RadiusRangeCheck).
    pub lights: Vec<Light>,
}

impl MapProject {
    /// Новый проект: квадратная карта `size`x`size` (50/100/200/400/800),
    /// залитая тайлом `tile` (как старый редактор: 50x50 воды).
    pub fn new(size: usize, tile: usize) -> Self {
        let size = size.clamp(1, 800);
        let tile = tile.min(TILE_COUNT - 1);
        Self {
            editor: ProjectMeta {
                format_version: 1,
                ..Default::default()
            },
            map: GameMap {
                tilemap: TileMap {
                    inner: vec![tile; size * size],
                    size,
                },
                ..Default::default()
            },
            events: Vec::new(),
            lights: Vec::new(),
        }
    }

    /// Сторона квадратной карты в тайлах.
    pub fn size(&self) -> usize {
        self.map.tilemap.size
    }

    /// Тайл в позиции, если внутри карты.
    pub fn tile(&self, pos: Pos) -> Option<usize> {
        let (x, y) = pos;
        (x < self.size() && y < self.size()).then(|| self.map.tilemap[(x, y)])
    }

    /// Сериализация снапшота проекта в JSON (метаданные и состояние для
    /// тестов; .DTm-экспорт — позже, в editor-serde).
    pub fn to_json(&self) -> serde_json::Result<String> {
        serde_json::to_string_pretty(&ProjectSnapshot::of(self))
    }

    /// Десериализация проекта из JSON-снапшота.
    pub fn from_json(json: &str) -> serde_json::Result<Self> {
        let snapshot: ProjectSnapshot = serde_json::from_str(json)?;
        Ok(Self {
            editor: snapshot.editor,
            map: GameMap {
                start: dt_lib::map::map::StartStats {
                    name: snapshot.name,
                    description: snapshot.description,
                    ..Default::default()
                },
                tilemap: TileMap {
                    inner: snapshot.tiles,
                    size: snapshot.size,
                },
                decomap: snapshot
                    .decos
                    .into_iter()
                    .map(|(index, x, y)| dt_lib::map::deco::MapDeco { index, x, y })
                    .collect(),
                ..Default::default()
            },
            events: snapshot.events,
            lights: snapshot.lights,
        })
    }
}

/// Сериализуемый срез проекта: всё, что не требует alkahest/advini.
#[derive(Serialize, Deserialize)]
struct ProjectSnapshot {
    editor: ProjectMeta,
    name: String,
    description: String,
    size: usize,
    /// Тайлы построчно, len = size*size.
    tiles: Vec<usize>,
    /// Декорации: (index, x, y).
    decos: Vec<(usize, usize, usize)>,
    events: Events,
    lights: Vec<Light>,
}

impl ProjectSnapshot {
    fn of(project: &MapProject) -> Self {
        Self {
            editor: project.editor.clone(),
            name: project.map.start.name.clone(),
            description: project.map.start.description.clone(),
            size: project.size(),
            tiles: project.map.tilemap.inner.clone(),
            decos: project
                .map
                .decomap
                .iter()
                .map(|d| (d.index, d.x, d.y))
                .collect(),
            events: project.events.clone(),
            lights: project.lights.clone(),
        }
    }
}

/// Уникальное имя нового строения/армии (Phase 0: без реестра объектов).
pub fn default_building(id: usize, pos: Pos) -> MapBuildingdata {
    MapBuildingdata {
        name: String::new(),
        desc: String::new(),
        owner_name: String::new(),
        id,
        events: Vec::new(),
        variant: dt_lib::map::object::BuildingVariant::Castle,
        market: None,
        recruitment: None,
        pos,
        owner: None,
        garrison: Vec::new(),
        additional_defense: 0,
        gold_income: 0,
        mana_income: 0,
        spells_to_learn: Vec::new(),
        relations: dt_lib::battle::control::Relations::default(),
        group: 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_project_defaults() {
        let p = MapProject::new(50, 0);
        assert_eq!(p.size(), 50);
        assert!(p.map.tilemap.inner.iter().all(|&t| t == 0));
        assert!(p.map.buildings.is_empty());
        assert!(p.events.is_empty());
    }

    #[test]
    fn new_project_clamps() {
        let p = MapProject::new(5000, 99);
        assert_eq!(p.size(), 800);
        assert!(p.tile((0, 0)).unwrap() < TILE_COUNT);
    }

    #[test]
    fn json_roundtrip_keeps_tiles_events_lights() {
        let mut p = MapProject::new(4, 2);
        p.map.tilemap[(2, 1)] = 5;
        p.map.decomap.push(dt_lib::map::deco::MapDeco::new(3, 1, 1));
        p.lights.push(Light { x: 1, y: 2, radius: 10 });
        p.events.push(Event::default());
        p.map.start.name = "Test map".into();
        let json = p.to_json().expect("serialize");
        let back = MapProject::from_json(&json).expect("deserialize");
        assert_eq!(back.editor, p.editor);
        assert_eq!(back.size(), 4);
        assert_eq!(back.map.tilemap[(2, 1)], 5);
        assert_eq!(back.map.start.name, "Test map");
        assert_eq!(back.map.decomap.len(), 1);
        assert_eq!(back.events.len(), 1);
        assert_eq!(back.lights, p.lights);
    }
}
