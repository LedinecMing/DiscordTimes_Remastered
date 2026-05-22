use advini::Sections;

use crate::{bonuses::{self, BonusInfo}, effects::{EffectInfo, EffectKind, EffectLifetime}, items::{self, ItemInfo}, locale::Locale, map::object::ObjectInfo, units::{self, unit::UnitInfo}};
use std::{collections::HashMap, net::{IpAddr, Ipv4Addr}, ops::{Index, IndexMut}};

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct Registry<T> {
	pub inner: Vec<T>,
	map_ids_to_str: HashMap<usize, String>,
	map_str_to_ids: HashMap<String, usize>,
}
impl<T> Default for Registry<T> {
	fn default() -> Self {
		Self {
			inner: vec![],
			map_ids_to_str: HashMap::new(),
			map_str_to_ids: HashMap::new()
		}
	}
}
impl<T: PartialEq> Registry<T> {
	pub fn new() -> Self {
		Self {
			inner: vec![],
			map_ids_to_str: HashMap::new(),
			map_str_to_ids: HashMap::new()
		}
	}
	pub fn str_to_id(&self, str_id: &String) -> Option<usize> {
		self.map_str_to_ids.get(str_id).copied()
	}
	pub fn register(&mut self, item: T, name: impl Into<String>) -> Option<usize> {
		let name = name.into();
		if self.inner.contains(&item) {
			return None;
		}
		let id = self.inner.len();
		self.inner.push(item);
		self.map_ids_to_str.insert(id, name.clone());
		self.map_str_to_ids.insert(name, id);
		Some(id)
	}
	// pub fn str_to_id(&self, str_id: &String) -> Option<usize> {
	// 	self.map_ids_to_str.iter().find(|x| x.1 == str_id).
	// }
	// pub fn id_to_str(&self, id: usize) -> Option<String> {
		
	// }
}
impl<T> Index<usize> for Registry<T> {
	type Output = T;
	fn index(&self, index: usize) -> &Self::Output {
		&self.inner[index]
	}
}
impl<T> IndexMut<usize> for Registry<T> {
	fn index_mut(&mut self, index: usize) -> &mut T {
		&mut self.inner[index]
	}
}

pub type UnitId = usize;
pub type BonusId = usize;

pub type Bonuses = Registry<BonusInfo>;
pub type Items = Registry<ItemInfo>;
pub type Units = Registry<UnitInfo>;
pub type Effects = Registry<EffectInfo>;
pub type Objects = Registry<ObjectInfo>;

#[derive(Clone, Debug)]
pub struct Settings {
    pub locale: String,
    pub additional_locale: String,
    pub fullscreen: bool,
    pub ip: IpAddr,
    pub port: u64,	
}
#[derive(Clone, Debug)]
pub struct GameSettings {
    pub max_troops: usize,
}
impl Default for GameSettings {
	fn default() -> Self {
		Self {
			max_troops: 12
		}
	}
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            locale: "Rus".into(),
            additional_locale: "Eng".into(),
            fullscreen: true,
            ip: IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
            port: 0,
        }
    }
}

#[derive(Debug, Clone)]
pub struct GameInfo {
	pub units: Units,
	pub items: Items,
	pub bonuses: Bonuses,
	pub effects: Effects,
	pub objects: Objects,

	pub locale: Locale,
	pub game_settings: GameSettings,
	pub settings: Settings
}
impl Default for GameInfo {
	fn default() -> Self {
		Self {
			units: Default::default(),
			items: Default::default(),
			bonuses: Default::default(),
			effects: Default::default(),
			objects: Default::default(),

			locale: Locale::new("Rus".into(), "Eng".into()),
			game_settings: Default::default(),
			settings: Default::default()
		}
	}
}
impl GameInfo {
	pub fn new() -> Self {
		let mut reg = Self::default();
		
		
		reg
	}
}
