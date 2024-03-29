/*
[GlobalIndex Name]

GlobalIndex=[1-250; standard = 1-129] — индекс

Name=[{символы}] — название

Descript=[{символы}] — описание

Cost=[{число}] — стоимость найма

CostMultipler=[{число}; standard = 100] — коррекция силы

CostGoldDiv=[1-9] — делитель стоимости найма

Nature=[{отсутствие строки}/Undead/Elemental/Rogue/Animal/Hero/People] | Нормальный/Нежить/Элементаль/Разбойники/Животные/Герой/Люди — тип персонажа

Magic=[{отсутствие строки}/LifeMagic/ElementalMagic/DeathMagic] | Нет/Жизни/Стихий/Смерти — магия

MagicDirection=[{отсутствие строки}/ToAll/ToEnemy/ToAlly/CurseOnly/StrikeOnly/BlessOnly/CureOnly] | На всех/На чужих/На своих/Проклятие/Атакующая/Благость/Лечащая — направление магии

Surrender=[{число}] — плен

StartExpirience=[{число}] — базовый опыт (2lvl = StartExpirience)

Levpler=[{число}; standard = 140] — множитель уровня (y*lvl → y+1lvl = StartExpirience × (LevelMultipler / 100) ^ y. * – y > 2)

IconIndex=[{число}] — индекс иконки

Bonus=[{отсутствие строки}
Dead, Fire,
Ghost, Block, Poison,
Evasive, Berserk,
Merchant, GodAnger, Garrison, FastDead,
ArmyMedic, GodStrike, Artillery,
DeathCurse, AddPayment,
HorseAttack, ArmorIgnore, Unvulnerabe, VampirsGist, Counterblow, FlankStrike,
SpearDefense,
OldVampirsGist

/Длинное Оружие/Быстрая Атака/Проникающий Удар/Лекарское Умение/Торговец-Эксперт/Проклятие Смерти/Кара Господня/Гнев Господен/Неуязвимость/Тёмный Дар/Тёмное Искусство/Увертливость/Яростный Дух/Шквальная Атака/Гарнизон/Тыловая Служба/Отравленное Оружие/Мертвец/Быстрый Мертвец/Контрудар/Фланговый удар — бонус

// развитие отряда

NextUnit1=[Name] — развитие в персонажа
Option
NextUnit1Level=[{число}; standard = 1] — необходимый уровень для развития в отряде ИИ

NextUnit2=[Name] — развитие в персонажа

NextUnit2Level=[{число}; standard = 1] — необходимый уровень для развития в отряде ИИ

NextUnit3=[Name] — развитие в персонажа

NextUnit3Level=[{число}; standard = 1] — необходимый уровень для развития в отряде ИИ

// боевые характеристики

Hits=[{число}] — хиты

AttackBlow=[{отсутствие строки}/{число}] — рукопашная атака+

AttackShot=[{отсутствие строки}/{число}] — стрелковая атака

DefenceBlow=[{отсутствие строки}/{число}] — рукопашная защита

DefenceShot=[{отсутствие строки}/{число}] — стрелковая защита

MagicPower=[{отсутствие строки}/{число}] — сила магии

ProtectLife=[{отсутствие строки}/1-100] — защита от магии жизни

ProtectDeath=[{отсутствие строки}/1-100] — защита от магии смерти

ProtectElemental=[{отсутствие строки}/1-100] — защита от магии стихий

Initiative=[{число}] — инициатива

Manevres=[{число}] — действия

Regen=[{отсутствие строки}/1-100] — регенерация

Vampirizm=[{отсутствие строки}/1-100] — вампиризм

// поуровневые изменения характеристики

d-Hits=[{отсутствие строки}/{число}] — +хиты

d-AttackBlow=[{отсутствие строки}/{число}] — +рукопашная атака

d-AttackShot=[{отсутствие строки}/{число}] — +стрелковая атака

d-DefenceBlow=[{отсутствие строки}/{число}] — +рукопашная защита

d-DefenceShot=[{отсутствие строки}/{число}] — +стрелковая защита

d-MagicPower=[{отсутствие строки}/{число}] — +сила магии

d-ProtectLife=[{отсутствие строки}/1-100] — +защита от магии жизни

d-ProtectDeath=[{отсутствие строки}/1-100] — +защита от магии смерти

d-ProtectElemental={отсутствие строки}/1-100] — +защита от магии стихий

d-Initiative=[{отсутствие строки}/{число}] — +инициатива

d-Manevres=[{отсутствие строки}/{число}] — +действия

d-Regen=[{отсутствие строки}/1-100] — +регенерация

d-Vampirizm=[{отсутствие строки}/1-100] — +вампиризм

 */
use math_thingies::add_opt;
use num::Num;

use std::{any::type_name, io::Read, ops::Add, fmt::Debug};
use std::{collections::HashMap, fmt::Display, str::FromStr};
use notan::{
	prelude::{Asset, Texture, Graphics, Assets},
	log
};
use super::{
    bonuses::*,
	battle::{troop::Troop, army::Control},
    items::item::{ItemInfo, *},
    map::{
		object::{ObjectInfo, ObjectType, MapBuildingdata, Market, Recruitment, RecruitUnit},
		map::{GameMap, Tilemap, MAP_SIZE},
		event::*
	},
    units::{
        unit::{MagicDirection::*, MagicType::*, *},
        unitstats::ModifyUnitStats,
    },
	time::time::{Time, Data::*},
	mutrc::SendMut,
};
use anyhow::Result as EResult;
use crate::lib::{battle::army::{Army, ArmyStats}, items};
use math_thingies::Percent;
use ini_core::{Item, Parser};
use advini::*;

//#[cfg(target_arch = "wasm32")]
//use wasm_bindgen_futures::spawn_local;
#[cfg(not(target_arch = "wasm32"))]
pub fn read_file_as_string(assets: &mut Assets, path: String) -> String {
	// let asset = assets.load_asset::<Vec<u8>>(&path);
	//let mut fut = load_file(&path);
	log::warn!("Loading {path}");
	let res = String::from_utf8(std::fs::read(path.clone()).unwrap()).unwrap();
	log::warn!("Loaded {path}");
	return res;
}
#[cfg(target_arch = "wasm32")]
pub fn read_file_as_string(assets: &mut Assets, path: String) -> String {
	use std::sync::{Arc, Mutex};
	use std::task;
	// use ehttp::fetch;
	// let asset = assets.load_asset::<Vec<u8>>(&path);
	//let mut fut = load_file(&path);
	// log::warn!("Loading {path}");
	// let request = ehttp::Request::get(&format!("localhost:8080/{path}"));
	// let mut content: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));
	// let mut res = content.clone();
	// ehttp::fetch(request, move |result: ehttp::Result<ehttp::Response>| {
	// 	let text = String::from_utf8(result.unwrap().bytes).unwrap();
	// 	log::warn!("{text}");
	// 	*res.lock().unwrap() = Some(text);
	// });
	// loop {
	// 	if let Some(v) = &*content.lock().unwrap() {
	// 		log::warn!("Loaded {path}");
	// 		let x = v.clone(); return x;
	// 	}
	// }
	String::new()
}

pub fn load_asset(manager: &mut Assets, gfx: &mut Graphics, path: &str) -> Result<Asset<Texture>, String> {
	//manager.load_asset(path)
	log::warn!("Loading {path}");
	Ok(Asset::from_data(&*path,
						gfx.create_texture()
						.from_image(&std::fs::read(path).unwrap())
						.build().unwrap()))
}

fn collect_errors<T, K: Display>(
    for_check: Result<T, K>,
    collector: &mut Vec<String>,
    additional: &str,
) -> Option<T> {
    match for_check {
        Ok(value) => Some(value),
        Err(info) => {
            collector.push(format!(
                "Error: {}; additional: {}",
                info.to_string(),
                 additional
            ));
            None
        }
    }
}
fn handle_parse<T: FromStr + Display>(
    v: impl Into<String>,
    collector: &mut Vec<String>,
    field: &str,
) -> Option<T>
where
    <T as FromStr>::Err: Display,
{
    collect_errors(
        v.into().parse::<T>(),
        collector,
        &*format!("Value of field {field} ommited as non-{}", type_name::<T>()),
    )
}

const MATCH_ERR: Result<(), &str> = Err("parse.rs: cant match field;");
pub fn match_magictype(
    error_collector: &mut Vec<String>,
    magic_type: &str,
    direction: MagicDirection,
) -> Option<MagicType> {
    match magic_type {
        "LifeMagic" => Some(Life(direction)),
        "ElementalMagic" => Some(Elemental(direction)),
        "DeathMagic" => Some(Death(direction)),
        "NoMagic" | "" => None,
        _ => {
            collect_errors(
                MATCH_ERR,
                error_collector,
                &*format!("Field MagicType is invalid: {}", magic_type),
            );
            None
        }
    }
}
pub fn parse_units(manager: &mut Assets, asset_map: &mut HashMap<&str, HashMap<String, Asset<Texture>>>, gfx: &mut Graphics) -> HashMap<usize, Unit> {
    let mut units = HashMap::new();
    let sections = parse_for_sections(manager, "Units.ini");
    let mut counter = None;
	asset_map.insert("assets/Icons", HashMap::new());
	let mut upgrades: HashMap<usize, Vec<String>> = HashMap::new();
	
    for (sec, prop) in sections.iter() {
        let mut name = "";
        let mut description = "";
        let mut magic_type = "";
        let mut magic_direction = "";
        let mut nature = "";
        let mut bonus_name = "";

        let mut cost_hire = None;
		let mut size = (1, 1);
        let cost = None::<u64>;
        let mut surrender = None;
        let mut icon_index = None;

        let mut hp = None;

        let mut max_xp = None;
        let mut xp_up = None;

        let mut damage_hand = Some(0);
        let mut damage_ranged = Some(0);
        let mut damage_magic = Some(0);

        let mut defence_hand = Some(0);
        let mut defence_ranged = Some(0);
        let mut defence_magic = Some(0);

        let mut defence_hand_percent = Some(0);
        let mut defence_ranged_percent = Some(0);
        let mut defence_death_magic = Some(0);
        let mut defence_life_magic = Some(0);
        let mut defence_elemental_magic = Some(0);

        let mut moves = Some(0);
        let mut speed = Some(0);
        let mut vamp = Some(0);
        let mut regen = Some(0);

        let mut next_unit = Vec::new();
        let mut error_collector: Vec<String> = Vec::new();
        for (k, value) in prop.iter() {
            let v = &**value;
            match &**k {
                "name" => name = v,
                "descript" => description = v,
                "nature" => nature = v,
                "iconindex" => {
                    icon_index = handle_parse::<usize>(v, &mut error_collector, "iconindex")
                }
                "cost" => cost_hire = handle_parse::<u64>(v, &mut error_collector, "cost_hire"),
                "surrender" => {
                    surrender = handle_parse::<u64>(v, &mut error_collector, "surrender")
                }
                "hits" => hp = handle_parse::<i64>(v, &mut error_collector, "hp"),
                "attackblow" | "attackhand" => {
                    damage_hand = handle_parse::<u64>(v, &mut error_collector, "damage_hand")
                }
                "attackshot" | "attackranged" => {
                    damage_ranged = handle_parse::<u64>(v, &mut error_collector, "damage_ranged")
                }
                "magicpower" => {
                    damage_magic = handle_parse::<u64>(v, &mut error_collector, "magic_power")
                }
                "magic" | "attackmagic" => {
                    magic_type = v;
                }
                "defenceblow" | "defencehand" => {
                    defence_hand = handle_parse::<u64>(v, &mut error_collector, "defence_hand");
                }
                "defenceshot" | "defenceranged" => {
                    defence_ranged = handle_parse::<u64>(v, &mut error_collector, "defence_ranged");
                }
                "defencemagic" => {
                    defence_magic = handle_parse::<u64>(v, &mut error_collector, "defence_magic");
                }
                "protectdeath" => {
                    defence_death_magic =
                        handle_parse::<i16>(v, &mut error_collector, "defence_death_magic");
                }
                "protectlife" => {
                    defence_life_magic =
                        handle_parse::<i16>(v, &mut error_collector, "defence_life_magic");
                }
                "protectelemental" => {
                    defence_elemental_magic =
                        handle_parse::<i16>(v, &mut error_collector, "defence_elemental_magic");
                }
                "protectblow" | "protecthand" => {
                    defence_hand_percent =
                        handle_parse::<i16>(v, &mut error_collector, "defence_hand_percent");
                }
                "protectshot" | "protectranged" => {
                    defence_ranged_percent =
                        handle_parse::<i16>(v, &mut error_collector, "defence_ranged_percent");
                }
                "magicdirection" => {
                    magic_direction = v;
                }
                "manevres" | "moves" => {
                    moves = handle_parse::<i64>(v, &mut error_collector, "moves");
                }
                "initiative" | "speed" => {
                    speed = handle_parse::<i64>(v, &mut error_collector, "speed");
                }
                "vampirizm" => {
                    vamp = handle_parse::<i16>(v, &mut error_collector, "vamp");
                }
                "regen" => {
                    regen = handle_parse::<i16>(v, &mut error_collector, "regen");
                }
                "levelmultipler" => {
                    xp_up = handle_parse::<i16>(v, &mut error_collector, "levelmultipler");
                }
                "startexpirience" => {
                    max_xp = handle_parse::<u64>(v, &mut error_collector, "max_xp");
                }
                "nextunit1" | "nextunit2" | "nextunit3" => {
					next_unit.push(v.into());
				},
				"size" => size = {
					let mut points = v.split(|ch: char| !ch.is_ascii_digit()).map(|string| string.parse().unwrap());
					(points.next().unwrap(), points.next().unwrap())
				},
                "bonus" => {
                    bonus_name = v;
                }
                "globalindex" => {
                    counter = handle_parse::<usize>(v, &mut error_collector, "globalindex");
					if !upgrades.contains_key(&counter.unwrap()) { upgrades.insert(counter.unwrap(), next_unit.clone()); }
					let path = format!("assets/Icons/img_{}.png", counter.unwrap() - 1);
					asset_map
						.get_mut("assets/Icons").unwrap()
						.insert(format!("img_{}.png", counter.unwrap() - 1),
								load_asset(manager, gfx, &*path.clone())
								// Asset::from_data(&*path.clone(),
								// 				 gfx.create_texture()
								// 				 .from_image(&read_file(path))
								//				 .build()
												 .unwrap());
					
                }
                _ => {}
            }
        }

        let magic_direction = match magic_direction {
            "ToAll" => ToAll,
            "ToAlly" => ToAlly,
            "ToEnemy" => ToEnemy,
            "CurseOnly" => CurseOnly,
            "CureOnly" => CureOnly,
            "BlessOnly" => BlessOnly,
            "StrikeOnly" => StrikeOnly,
            "" => ToAll,
            _ => {
                collect_errors(
                    MATCH_ERR,
                    &mut error_collector,
                    &*format!("Field MagicDirection is invalid: {}", magic_direction),
                );
                ToAll
            }
        };
        let magic_type = match_magictype(&mut error_collector, magic_type, magic_direction);
        let bonus = Bonus::from(bonus_name);
        let unit_type = match nature {
            "People" | "" => UnitType::People,
            "Rogue" => UnitType::Rogue,
            "Undead" => UnitType::Undead,
            "Hero" => UnitType::Hero,
			"Mecha" => UnitType::Mecha,
            _ => {
                collect_errors(
                    MATCH_ERR,
                    &mut error_collector,
                    &*format!("Field Nature is invalid: {}", nature),
                );
                UnitType::People
            }
        };
		if !error_collector.is_empty() {             
		    panic!("{}", error_collector.join("\n"));
		}                                     
  
        let hp = hp.unwrap();
        let xp_up = xp_up.unwrap();
        let max_xp = max_xp.unwrap();

        let cost_hire = cost_hire.unwrap();
        let cost = if cost_hire <= 50 {
            cost_hire / 8
        } else if cost_hire > 50 && cost_hire <= 100 {
            cost_hire / 4
        } else if cost_hire > 100 && cost_hire <= 150 {
            (cost_hire as f64 / 2.65) as u64
        } else {
            cost_hire / 2
        };
        let stats = UnitStats {
            hp,
            max_hp: hp,
            damage: Power {
                magic: damage_magic.unwrap(),
                ranged: damage_ranged.unwrap(),
                hand: damage_hand.unwrap(),
            },
            defence: Defence {
                death_magic: Percent::new(defence_death_magic.unwrap()),
                elemental_magic: Percent::new(defence_elemental_magic.unwrap()),
                life_magic: Percent::new(defence_life_magic.unwrap()),
                hand_percent: Percent::new(defence_hand_percent.unwrap()),
                ranged_percent: Percent::new(defence_ranged_percent.unwrap()),
                magic_units: defence_magic.unwrap(),
                hand_units: defence_hand.unwrap(),
                ranged_units: defence_ranged.unwrap(),
            },
            moves: moves.unwrap(),
            max_moves: moves.unwrap(),
            speed: speed.unwrap(),
            vamp: Percent::new(vamp.unwrap()),
            regen: Percent::new(regen.unwrap()),
        };
        let unit = Unit {
            stats,
            modified: stats,
            modify: ModifyUnitStats::default(),
            info: UnitInfo {
                name: name.into(),
                descript: description.into(),
                cost,
                cost_hire,
                icon_index: counter.unwrap() - 1,
                unit_type,
                next_unit: Vec::new(),
                magic_type,
				size,
                surrender,
                lvl: LevelUpInfo {
                    stats: ModifyUnitStats::default(),
                    xp_up,
                    max_xp,
                },
            },
            lvl: UnitLvl {
                lvl: 0,
                max_xp,
                xp: 0,
            },
            inventory: UnitInventory {
                items: vec![None; 4],
            },
            army: 0,
            bonus,
            effects: vec![],
        };
        units.insert(counter.unwrap(), unit);
    }
	for (index, up) in upgrades {
		let upgrade = up.iter().map(|name| units.iter().filter(|unit| unit.1.info.name == *name).map(|unit| *unit.0).next().unwrap() ).collect();
		units.get_mut(&index).unwrap().info.next_unit = upgrade;
	}
	let powered = units.clone();
	let bycost = units.clone();
	let mut bycost = bycost.into_values()
		.map(|v| {
			let cost = v.info.cost;
			 (v, cost as f32)
		})
		.collect::<Vec<(Unit, f32)>>();
	let mut powered = powered.into_values()
		.map(|v| {
			let power = calclate_unit_power(&v);
			 (v, power)
		})
		.collect::<Vec<(Unit, f32)>>();
	bycost.sort_by(|a, b|
					a.1.total_cmp(&b.1));
	powered.sort_by(|a, b|
					a.1.total_cmp(&b.1));
	for (unit, cost) in powered.iter().zip(bycost) {
		println!("{} is {} costs {} and {} costs {}", &unit.0.info.name, unit.1, unit.0.info.cost, &cost.0.info.name, cost.0.info.cost);
	}
    units
}

#[derive(Clone, Debug, Default)]
pub struct Settings {
    pub max_troops: usize,
    pub locale: String,
	pub additional_locale: String,
	pub fullscreen: bool,
	pub init_size: (u32, u32),
	pub port: u64
}

pub static mut SETTINGS: Settings = Settings {
	max_troops: 0,
	locale: String::new(),
	additional_locale: String::new(),
	fullscreen: true,
	init_size: (1600, 1200),
	port: 0
};

#[derive(Debug)]
pub struct Locale {
	map: HashMap<String, HashMap<String, String>>,
	main_lang: String,
	additional_lang: String
}
impl Locale {
	pub fn switch_lang(&mut self) {
		let main = self.main_lang.clone();
		let add = self.additional_lang.clone();
		self.main_lang = add;
		self.additional_lang = main;
	}
	pub fn set_lang(&mut self, lang: (&String, &String)) {
		self.main_lang = lang.0.clone();
		self.additional_lang = lang.1.clone();
	}
    pub fn get<K: AsRef<str> + ToString>(&self, id: K) -> String {
		let id = id.as_ref();
        self.map
            .get(&self.main_lang)
            .and_then(|lang_map| 
                lang_map.get(id).or_else(||
                    self.map.get(&self.additional_lang)
                        .and_then(|lang_map|
                            lang_map.get(id)
                        )
				)
			)
            .cloned()
            .unwrap_or(id.to_string())
    }
    pub fn insert<V: Into<String>, K: Into<String>>(&mut self, key: K, value: V, lang: &String) {
		let (k, v) = (key.into(), value.into());
		self.map.entry(lang.clone())
			.or_insert_with(HashMap::new)
			.insert(k, v);
    }
    pub fn new(main_lang: String, additional_lang: String) -> Self {
        Locale {
			map: HashMap::new(),
			main_lang,
			additional_lang
		}
    }
}
pub fn parse_settings(assets: &mut Assets) -> Settings {
    let sections = parse_for_sections(assets, "Settings.ini");
    let mut max_troops: usize = 0;
    let mut locale = String::new();
	let mut additional_locale = String::new();
	let mut fullscreen = false;
	let mut init_size = None;
	let mut port = 0;
    for (sec, prop) in sections.iter() {
        for (k, value) in prop.iter() {
            match &**k {
                "max_troops" => {
                    max_troops = value
                        .parse::<usize>()
                        .expect("Field max_troops is not usize type")
                }
                "locale" => locale = value.clone(),
				"additional_locale" => additional_locale = value.clone(),
				"fullscreen" => fullscreen = str_bool(value.to_string()),
				"init_size" => {
					let mut parsed = value.split(",");
					init_size = Some((parsed.next().unwrap().parse().unwrap(), parsed.next().unwrap().parse().unwrap()));
				}
				"port" => { port = value.parse::<u64>().unwrap(); }
                _ => {}
            }
        }
    }
	let settings = Settings { max_troops, locale, additional_locale, fullscreen, init_size: init_size.unwrap(), port };
	unsafe {
		SETTINGS=settings.clone();
	}
	settings
}

pub fn parse_locale(assets: &mut Assets, languages: &[&String], locale: &mut Locale) {
	for language in languages {
		let props = parse_for_props(assets, &*format!("{}_Locale.ini", language));
		log::warn!("{props:?}");
		for (k, value) in props {
			locale.insert(k, value, &language);
		}
	}
}
pub fn parse_map_locale(assets: &mut Assets, path: &str, languages: &[&String], locale: &mut Locale) {
	for (sec, props) in parse_for_sections(assets, path) {
		if !languages.contains(&&sec) { continue; }
		for prop in props {
			register_locale(prop.0, prop.1, sec.clone(), locale).unwrap();
		}
	}
}

fn parse_for_props(assets: &mut Assets, path: &str) -> HashMap<String, String> {
    let mut props = HashMap::new();
	let ini_doc = read_file_as_string(assets, path.into());
    let parser = Parser::new(&*ini_doc).auto_trim(true);
    for item in parser {
        match item {
            Item::Section(_) => {}
            Item::Property(k, v) => {
                props.insert(k.to_lowercase().into(), v.into());
            }
            Item::Blank | Item::Action(_) | Item::Comment(_) => {}
            Item::Error(err) => panic!("{}", err),
        }
    }
    props
}
fn parse_for_sections(assets: &mut Assets, path: &str) -> Vec<(String, HashMap<String, String>)> {
	let ini_doc = read_file_as_string(assets, path.into());
	advini::parse_for_sections(&ini_doc)
}
fn parse_for_sections_localised(assets: &mut Assets, path: &str, locale: &mut Locale) -> Vec<(String, HashMap<String, String>)> {
	let ini_doc = read_file_as_string(assets, path.into());
	advini::parse_for_sections_with(&ini_doc, |(prop, v, s)| (prop.to_lowercase(), process_locale(v, s).unwrap_or(v.to_string())), locale)
}
type Objects = Vec<ObjectInfo>;
pub fn parse_objects(manager: &mut Assets, gfx: &mut Graphics, assets: &mut HashMap<&str, HashMap<String, Asset<Texture>>>) -> Objects {
    let mut objects = Vec::new();
    let sections = parse_for_sections(manager, "Objects.ini");
	assets.insert("assets/Objects", HashMap::new());
    for (sec, prop) in sections.iter() {
        let mut category = "".into();
        let mut obj_type = None;
        let mut index = None;
		let name = sec.clone();
        let mut size = (Some(1), Some(1));
        let mut error_collector: Vec<String> = Vec::new();
        for (k, v) in prop.iter() {
            match &**k {
                "index" => {
                    index = collect_errors(
                        v.parse::<usize>(),
                        &mut error_collector,
                        "Value of field Index ommited as non-usize",
                    );
					let path = format!("{sec}.png");
					let asset = assets
						.get_mut("assets/Objects").unwrap()
						.insert(path.clone(),
								load_asset(manager, gfx, &*format!("assets/Objects/{path}"))
  								// Asset::from_data(&*path.clone(),
								// 				 gfx.create_texture()
								// 				 .from_image(&read_file(format!("assets/Objects/{}", path)))
								// 				 .build()
								.unwrap());
                }
                "size" => {
                    let mut sizes = v.split(|ch: char| !ch.is_ascii_digit()).map(|string| Some(string.parse().unwrap()));
					size.0=sizes.next().unwrap(); size.1=sizes.next().unwrap();
                }
                "type" => {
                    obj_type = Some(match &**v {
                        "MapDeco" => ObjectType::MapDeco,
						"Bridge" => ObjectType::Bridge,
                        "Building" => ObjectType::Building,
                        _ => panic!(
                            "{}",
                            format!("Wrong Object Type - '{}' at section {}", v, sec)
                        ),
                    })
                }
                "category" => category = v.clone(),
                _ => {}
            }
        }
		
		if !error_collector.is_empty() {
			panic!("{}", error_collector.join("\n"));
		}
		
        objects.push(
            (index.unwrap(),
            ObjectInfo {
                category,
				name,
                obj_type: obj_type.expect("Cant find Type key!"),
                index: index.expect("Cant find Index key!"),
                size: (
                    size.0.expect("Cant find SizeW key!"),
                    size.1.expect("Cant find SizeH key!"),
                ),
                path: sec.clone().add(".png"),
            })
        );
    }
	objects.sort_by(|(id, _), (oth_id, _)| id.cmp(oth_id));
	objects.into_iter().map(|(_, object)| object).collect()
}
fn match_magic_variants(
    error_collector: &mut Vec<String>,
    magic_type: String,
) -> Option<MagicVariants> {
    match &*magic_type {
        "LifeMagic" => Some(MagicVariants::Life),
        "ElementalMagic" => Some(MagicVariants::Elemental),
        "DeathMagic" => Some(MagicVariants::Death),
        "NoMagic" | "" => None,
        _ => {
            collect_errors(
                MATCH_ERR,
                error_collector,
                &*format!("Invalid magic variant: {}", magic_type),
            );
            None
        }
    }
}
/*
 * d-{stat} - добавление
 * p-{stat} - добавление процента
 * f-{stat} - установить
 */
pub fn parse_items(lang: &String, manager: &mut Assets, gfx: &mut Graphics, assets: &mut HashMap<&str, HashMap<String, Asset<Texture>>>) {
    let mut error_collector: Vec<String> = Vec::new();
    let mut items = ITEMS.lock().unwrap();
	assets.insert("assets/Items", HashMap::new());
    let secs = parse_for_sections(manager, &*format!("{}_Artefacts.ini", "Rus")); //lang));
    for (sec, props) in secs {
        let mut cost: Option<i64> = None;
        let mut description = None;
        let mut name = None;
        let mut itemtype = None;
        let mut modify = ModifyUnitStats::default();
        let direction = MagicDirection::ToAll;
        let mut icon = None;
        let mut magic = None;
        let mut index = None;
        let mut bonus = None;
        let itemtype_name = "";
        for (k, value) in props.iter() {
            let value = &**value;
            match &**k {
                "globalindex" => index = handle_parse(value, &mut error_collector, "globalindex"),
                "name" => name = Some(value),
                "descript" => description = Some(value),
                "icon" => {
					icon = Some(value);
					let path = format!("assets/Items/{}", icon.unwrap().to_string());
					let asset = assets
						.get_mut("assets/Items").unwrap()
						.insert(value.to_string(),
								load_asset(manager, gfx, &*path.clone())
								// Asset::from_data(&*path.clone(),
								// 				 gfx.create_texture()
								// 				 .from_image(&read_file(format!("assets/items/{}", path)))
								//.build()
												 .unwrap());
				},
                "cost" => cost = handle_parse(value, &mut error_collector, "cost"),
                "magic" => {
                    magic = match_magic_variants(&mut error_collector, value.into());
                    itemtype = match itemtype_name {
                        "Staff" => Some(ArtifactType::Weapon(WeaponType::Magic(
                            magic.expect("Item type is Stuff but no Magic field provided"),
                        ))),
                        _ => itemtype,
                    };
                }
                "type" => {
                    let itemtype_name = value;
                    itemtype = match value {
                        "Staff" => ArtifactType::Weapon(WeaponType::Magic(MagicVariants::Any)),
                        "ShotWeapon" => ArtifactType::Weapon(WeaponType::Ranged),
                        "BlowWeapon" => ArtifactType::Weapon(WeaponType::Hand),
                        "Ring" => ArtifactType::Ring,
                        "Armor" => ArtifactType::Armor,
                        "Helm" | "Helmet" => ArtifactType::Helmet,
                        "Shield" => ArtifactType::Shield,
                        "Amulet" => ArtifactType::Amulet,
                        "Item" => ArtifactType::Item,
                        "Potion" => ArtifactType::Amulet,
                        _ => panic!("Wrong Item Type - {}!", value),
                    }
                    .into()
                }
                "d-hits" => {
                    modify.max_hp.add = add_opt(modify.max_hp.add, value.parse::<i64>().ok());
                    modify.hp.add = add_opt(modify.hp.add, value.parse::<i64>().ok());
                }
                "d-attackblow" => {
                    modify.damage.hand.add = add_opt(modify.damage.hand.add, value.parse().ok())
                }
                "d-attackshot" => {
                    modify.damage.ranged.add = add_opt(modify.damage.ranged.add, value.parse().ok())
                }
                "d-magicpower" => {
                    modify.damage.magic.add = add_opt(modify.damage.magic.add, value.parse().ok())
                }
                "d-defenceblow" => {
                    modify.defence.hand_units.add =
                        add_opt(modify.defence.hand_units.add, value.parse().ok())
                }
                "d-defenceshot" => {
                    modify.defence.ranged_units.add =
                        add_opt(modify.defence.ranged_units.add, value.parse().ok())
                }
                "d-defencemagic" => {
                    modify.defence.magic_units.add =
                        add_opt(modify.defence.magic_units.add, value.parse().ok())
                }
                "d-manevres" => {
                    modify.max_moves.add = add_opt(modify.max_moves.add, value.parse().ok());
                    modify.moves.add = add_opt(modify.moves.add, value.parse().ok());
                }
                "d-initiative" => modify.speed.add = add_opt(modify.speed.add, value.parse().ok()),
                "d-vampirizm" => modify.vamp.add = add_opt(modify.vamp.add, value.parse().ok()),
                "d-regen" => modify.regen.add = add_opt(modify.regen.add, value.parse().ok()),

                "p-hits" => {
                    modify.max_hp.percent_add = add_opt(
                        modify.max_hp.percent_add,
                        Percent::new(value.parse().unwrap()).into(),
                    );
                    modify.hp.percent_add = add_opt(
                        modify.hp.percent_add,
                        Percent::new(value.parse().unwrap()).into(),
                    );
                }
                "p-attackblow" => {
                    modify.damage.hand.percent_add = add_opt(
                        modify.damage.hand.percent_add,
                        Percent::new(value.parse().unwrap()).into(),
                    )
                }
                "p-attackshot" => {
                    modify.damage.ranged.percent_add = add_opt(
                        modify.damage.ranged.percent_add,
                        Percent::new(value.parse().unwrap()).into(),
                    )
                }
                "p-magicpower" => {
                    modify.damage.magic.percent_add = add_opt(
                        modify.damage.magic.percent_add,
                        Percent::new(value.parse().unwrap()).into(),
                    )
                }
                "p-defenceblow" => {
                    modify.defence.hand_units.percent_add = add_opt(
                        modify.defence.hand_units.percent_add,
                        Percent::new(value.parse().unwrap()).into(),
                    )
                }
                "p-defenceshot" => {
                    modify.defence.ranged_units.percent_add = add_opt(
                        modify.defence.ranged_units.percent_add,
                        Percent::new(value.parse().unwrap()).into(),
                    )
                }
                "p-defencemagic" => {
                    modify.defence.magic_units.percent_add = add_opt(
                        modify.defence.magic_units.percent_add,
                        Percent::new(value.parse().unwrap()).into(),
                    )
                }
                "p-protectlife" => {
                    modify.defence.life_magic.percent_add = add_opt(
                        modify.defence.life_magic.percent_add,
                        Percent::new(value.parse().unwrap()).into(),
                    )
                }
                "p-protectdeath" => {
                    modify.defence.death_magic.percent_add = add_opt(
                        modify.defence.death_magic.percent_add,
                        Percent::new(value.parse().unwrap()).into(),
                    )
                }
                "p-protectelemental" => {
                    modify.defence.elemental_magic.percent_add = add_opt(
                        modify.defence.elemental_magic.percent_add,
                        Percent::new(value.parse().unwrap()).into(),
                    )
                }
                "p-manevres" => {
                    modify.max_moves.percent_add = add_opt(
                        modify.max_moves.percent_add,
                        Percent::new(value.parse().unwrap()).into(),
                    );
                    modify.moves.percent_add = add_opt(
                        modify.moves.percent_add,
                        Percent::new(value.parse().unwrap()).into(),
                    );
                }
                "p-initiative" => {
                    modify.speed.percent_add = add_opt(
                        modify.speed.percent_add,
                        Percent::new(value.parse().unwrap()).into(),
                    )
                }
                "p-vampirizm" => {
                    modify.vamp.percent_add = add_opt(
                        modify.vamp.percent_add,
                        Percent::new(value.parse().unwrap()).into(),
                    )
                }
                "p-regen" => {
                    modify.regen.percent_add = add_opt(
                        modify.regen.percent_add,
                        Percent::new(value.parse().unwrap()).into(),
                    )
                }

                "f-hits" => {
                    modify.max_hp.set = value.parse::<i64>().ok();
                    modify.hp.set = value.parse::<i64>().ok();
                }
                "f-attackblow" => modify.damage.hand.set = value.parse().ok(),
                "f-attackshot" => modify.damage.ranged.set = value.parse().ok(),
                "f-magicpower" => modify.damage.magic.set = value.parse().ok(),
                "f-defenceblow" => modify.defence.hand_units.set = value.parse().ok(),
                "f-defenceshot" => modify.defence.ranged_units.set = value.parse().ok(),
                "f-defencemagic" => modify.defence.magic_units.set = value.parse().ok(),
                "f-manevres" => {
                    modify.max_moves.set = value.parse().ok();
                    modify.moves.set = value.parse().ok();
                }
                "f-initiative" => modify.speed.set = value.parse().ok(),
                "f-vampirizm" => modify.vamp.set = value.parse().ok(),
                "f-regen" => modify.regen.add = value.parse().ok(),
                "bonus" => bonus = Some(Bonus::from(value)),
                _ => {}
            }
        }
        items.insert(
            index.unwrap(),
            ItemInfo {
                name: name.expect("No name field").into(),
                description: description.expect("No description field").into(),
                cost: {
                    let cost = cost.expect("No cost field");
                    if cost > 0 {
                        cost as u64
                    } else {
                        0
                    }
                },
                icon: icon.expect("No icon key").into(),
                sells: cost.unwrap() > 0,
                bonus,
                itemtype: itemtype.expect(&*format!("{name}", name = name.unwrap())),
                modify,
            },
        );
    }
}

trait IsRus {
	fn is_rus_alphabet(&self) -> bool;
}
impl IsRus for char {
	fn is_rus_alphabet(&self) -> bool {
		matches!(*self, 'А'..='Я' | 'а'..='я' | 'ё' | 'Ё')
	}
}

fn process_locale(locale: impl Into<String>, map_locale: &mut Locale) -> EResult<String> {
	const LOCALE_START: char = '$';
	let locale = locale.into();
	let mut end_string = locale.clone();
	let mut locale_chars = locale.chars();
	for i in 0..locale.len() {
		if locale_chars.nth(i) == Some(LOCALE_START) {
			let end = locale.chars().skip(i+1).position(|ch| {
				!(ch.is_ascii_alphabetic() || ch.is_ascii_digit() || ch.is_rus_alphabet() || ch == '_')
			}).unwrap_or(locale.len() - 1);
			if !(i+1 == end) {
				let identificator = locale.chars().skip(i+1).take(end-i+1).collect::<String>();
				end_string = end_string.replace(&("$".to_owned() + &identificator), &map_locale.get(&identificator));
			}
		}
	}
	Ok(end_string)
}

fn split_and_parse<N: Num>(string: String) -> Vec<N> {
	string
		.split(|ch: char| !ch.is_ascii_digit())
		.map(|string| N::from_str_radix(string, 10).unwrap_or(N::zero()))
		.collect()
}

fn register_locale(locale_name: impl Into<String>, locale: impl Into<String>, lang: String, map_locale: &mut Locale) -> EResult<(), > {
	let end_string = process_locale(locale, map_locale)?;
	map_locale.insert(locale_name.into(), end_string, &lang);
	Ok(())
}

fn parse_cmp<V: Ord + FromStr>(v: String) -> Cmp<V> where <V as FromStr>::Err: Debug {
	match v {
		v if v.starts_with("<=") => Cmp::LE(v.split_at(2).1.parse().unwrap()),
		v if v.starts_with(">=") => Cmp::GE(v.split_at(2).1.parse().unwrap()),
		v if v.starts_with("<") => Cmp::L(v.split_at(1).1.parse().unwrap()),
        v if v.starts_with(">") => Cmp::G(v.split_at(1).1.parse().unwrap()),
		v if v.starts_with("=") => Cmp::E(v.split_at(1).1.parse().unwrap()),
		_ => Cmp::E(v.parse().unwrap()),
	}
}

fn str_bool(v: String) -> bool {         
    match &*v.to_lowercase() {           
        "true" | "1" | "t" | "y" => true,
        _ => false                       
    }                                    
}

fn parse_events(assets: &mut Assets, path: String, locale: &mut Locale) -> Vec<Event> {
	let mut events = Vec::new();
	for (sec, props) in parse_for_sections_localised(assets, &*path, locale) {
		let event = <Event as Sections>::from_section(props).unwrap();
		events.push(event.0);
	}
	events
}

fn parse_mapdata(assets: &mut Assets, path: String, units: &HashMap<usize, Unit>, locale: &mut Locale, objects: &Objects) -> (Tilemap<usize>, Tilemap<Option<usize>>, Vec<MapBuildingdata>, Vec<Army>) {
	let mut tilemap: Option<Tilemap<usize>> = None;
	let mut decomap: Option<Tilemap<Option<usize>>> = None;
	
	let mut armys = Vec::new();
	let mut buildings = Vec::new();
	
	for (sec, props) in parse_for_sections(assets, &*path) {
		match &*sec {
			"Tilemaps" => {
				for prop in props {
					match &*prop.0 {
						"tilemap" => tilemap = Some({
							let mut tiles = prop.1.split(|ch: char| !ch.is_ascii_digit())
								.map(|ch| ch.parse::<usize>().unwrap());
							(0..MAP_SIZE)
							.map(|_|
								 (0..MAP_SIZE)
								 .map(|_|
									  tiles.next().unwrap()
								 )
								 .collect::<Vec<_>>()
								 .try_into().unwrap()
							)
							.collect::<Vec<_>>()
							.try_into().unwrap()
						}),
						"decomap" => decomap = Some({
							let (tiles, _) = <Vec<usize> as Ini>::eat(prop.1.chars()).unwrap();
							// prop.1.split(|ch: char| !ch.is_ascii_digit())
							// 	.map(|ch|
							// 		 if ch=="0" {
							// 			 None
							// 		 } else { Some(ch.parse().unwrap()) }
							// 	);
							(0..MAP_SIZE)
							.map(|_|
								 (0..MAP_SIZE)
								 .zip(tiles.iter())
								 .map(|(_, v)| {
									 if *v == 0 {
										 None
									 } else { Some(*v) }
								 })
								 .collect::<Vec<_>>()
								 .try_into().unwrap()
							)
							.collect::<Vec<_>>()
							.try_into().unwrap()
						}),
						_ => {}
					}
				}
			},
			x if x.starts_with("Army") => {
				let mut inv = Vec::new();
				let mut pos = (0, 0);
				let mut stats = ArmyStats::default();
				let mut in_troops = Vec::new();
				let mut main = None;
				let mut active = true;
				let mut control = Control::PC;
				let mut id: Option<usize> = None;
				
				for prop in props {
					let prop = (prop.0, process_locale(prop.1, locale).unwrap());
					match &*prop.0 {
						"id" => id = prop.1.parse().ok(),
						"name" => stats.army_name = prop.1,
						"mana" => stats.mana = prop.1.parse().unwrap(),
						"gold" => stats.gold = prop.1.parse().unwrap(),
						"inventory" => inv = split_and_parse(prop.1).iter().map(|num| items::item::Item{index:*num}).collect(),
						"pos" => {
							let things = prop.1.split_once(|ch: char| !ch.is_ascii_digit()).unwrap();
							pos = (things.0.parse().unwrap(), things.1.parse().unwrap());
						},
						"active" => active = str_bool(prop.1),
						"troops" => in_troops =
							prop.1.split(",")
								.map(|string|
									 string
									 .split_once(";").unwrap())
								.map(|(num, lvl)|
									 (num.parse().unwrap(), lvl.parse::<i64>().unwrap()))
								.map(|(num, _)|
									 {
										 let mut troop = Troop::empty();
										 troop.unit = units[&num].clone();
										 troop.unit.army = armys.len();
										 SendMut::new(troop)
									 }).collect(),
						"player" => control = Control::Player(prop.1.parse().unwrap()),
						"main" => {
							let things = prop.1.split_once(|ch: char| !ch.is_ascii_digit()).unwrap();
							let troop = Troop {
								unit: {
									let mut unit = units[&things.0.parse().unwrap()].clone();
									unit.army = armys.len();
									unit
								},
								is_main: true,
								is_free: true,
								was_payed: true,
								pos: UnitPos::from_index(0),
								custom_name: Some(things.1.into())
							};
							main = Some(SendMut::new(troop));
							
						},
						_ => {}
					}
				}
				let mut troops = vec![main.unwrap()];
				troops.append(&mut in_troops);
				
				armys.push((id.unwrap(), Army::new(troops, stats, inv, pos, active, control)));
			},
			x if x.starts_with("Building") => {
				let mut id : Option<usize> = None;
				let mut name = String::new();
				let mut object_name = None;
				let mut desc = String::new();
				let mut building_type = None;
				let mut event = Vec::new();
				let mut units = Vec::new();
				let mut recruitment = None;
				let cost_modify = 1.;
				let mut market = None;
				let mut items = Vec::new();
				let mut itemcost_range = Some((0u64, 1000u64));
				let max_items = 10;
				let control = Control::PC;
				let mut pos = None;
				let mut defense = Some(0);
				let mut income = 0;
				let mut owner = None;
				for prop in props {
					let prop = (prop.0, process_locale(prop.1, locale).unwrap());
					match &*prop.0 {
						"name" => name = prop.1,
						"desc" => desc = prop.1,
						"id" => id = Some(prop.1.parse().unwrap()),
						"type" => building_type = Some(prop.1),
						"owner" => owner = Some(prop.1.parse().unwrap()),
						"items" => items = split_and_parse(prop.1),
						"defense" => defense = prop.1.parse().ok(),
						"object" => object_name = prop.1.into(),
						"itemcost_range" => itemcost_range = {
							let mut points = prop.1.split(|ch: char| !ch.is_ascii_digit()).map(|string| string.parse().unwrap());
							Some((points.next().unwrap(), points.next().unwrap()))
						},
						"income" => income = prop.1.parse().unwrap(),
						"recruit" => {
							units = prop.1.split(",")
								.map(|string|
									 string
									 .split_once(";").unwrap())
								.map(|(id, num)|
									 (id.parse().unwrap(), num.parse::<usize>().unwrap()))
								.map(|(id, num)|
									 RecruitUnit { unit: id, count: num }
								).collect();
						},
						"pos" => pos = {
							let mut points = prop.1.split(|ch: char| !ch.is_ascii_digit()).map(|string| string.parse().unwrap());
							Some((points.next().unwrap(), points.next().unwrap()))
						},
						"events" => event = split_and_parse::<usize>(prop.1),
						_ => {}
					}
				}
				if !items.is_empty() {
					market = Market {
						itemcost_range: itemcost_range.unwrap(),
						items,
						max_items
					}.into();
				}
				if !units.is_empty() {
					recruitment = Recruitment {
						cost_modify,
						units
					}.into();
				}
				buildings.push((id.unwrap(),
					MapBuildingdata {
						id: objects.into_iter().position(|obj| {
							&obj.name == object_name.as_ref().unwrap()
						}).unwrap(),
						name,
						desc,
					 	event,
						market,
						recruitment,
						pos: pos.unwrap(),
						defense: defense.unwrap(),
						income,
						owner
					}
				));
			}
			_ => {}
		}
	}
	(tilemap.unwrap(), decomap.unwrap(), {
		buildings.sort_by(|(id, _), (oth_id, _)| id.cmp(oth_id));
		buildings.into_iter().map(|(_, building)| building).collect()
	}, {
		armys.sort_by(|(id, _), (oth_id, _)| id.cmp(oth_id));
		armys.into_iter().map(|(_, army)| army).collect()
	})
}

pub fn parse_story(assets: &mut Assets, units: &HashMap<usize, Unit>, objects: &Objects, lang: &String, additional_lang: &String) -> (GameMap, Vec<Event>) {
	let mut err_coll = Vec::new();
	let map_dir = "map/";
	let map_path = "MapExample.ini";
	// Locale
	let mut locale = Locale::new(lang.clone(), additional_lang.clone());

	// Info
	let mut name = None;
	let mut description = None;

	// Settings
	let mut start_gold = Some(0u64);
	let mut start_mana = Some(0u64);
	let mut start_items = vec![];
	let mut start_time = Time::from_data("1540:1:1:12:0", [YEAR, MONTH, DAY, HOUR, MINUTES]);

	// MapData
	let mut mapdata_path = None;

	// Eventsandlights
	let mut events_path = None;
	
	for (sec, props) in parse_for_sections(assets, &format!("{map_path}")) {
		for prop in props {
			let prop = (prop.0, process_locale(prop.1, &mut locale).unwrap());
			match &*prop.0 {
				"filepath" => {					
					match &*sec {
						"Locale" => {
							parse_map_locale(assets, &*format!("{}/{}", map_dir, prop.1), &[&locale.main_lang.clone(), &locale.additional_lang.clone()], &mut locale);
						},
						"MapData" => mapdata_path = prop.1.into(),
						"EventsAndLights" => events_path = prop.1.into(),
						_ => {}
					}
				},
				"name" => name = Some(prop.1),
				"desc" => description = Some(prop.1),
				"start_time" => start_time = Time::from_data(prop.1, [YEAR, MONTH, DAY, HOUR]),
				"start_gold" => start_gold = handle_parse(prop.1, &mut err_coll, "start_gold"),
				"start_mana" => start_mana = handle_parse(prop.1, &mut err_coll, "start_mana"),
				"start_items" => {
					start_items = prop.1.split(|ch: char| !ch.is_ascii_digit())
						.map(|string|
							handle_parse::<usize>(string, &mut err_coll, "start_items") 
						)
						.collect();
				},
				_ => {}
			}
		}
	}
	if !err_coll.is_empty() {
		panic!("{}", err_coll.join("\n"));
	}
	let mapdata = parse_mapdata(assets, format!("{map_dir}{}", mapdata_path.unwrap()), units, &mut locale, objects);
	let events = parse_events(assets, format!("{map_dir}{}", events_path.unwrap()), &mut locale);

	let gamemap = GameMap {
		armys: mapdata.3,
		decomap: mapdata.1,
		tilemap: mapdata.0,
		buildings: mapdata.2,
		time: start_time,
		..Default::default()
	};
	(gamemap, events)
}
