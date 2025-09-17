use log::error;
/*
[GlobalIndex Name]

GlobalIndex=[1-250; standard = 1-129] — индекс

Name=[{символы}] — название

Descript=[{символы}] — описание

Cost=[{число}] — стоимость найма

CostMultipler=[{число}; standard = 100] — коррекция силы

CostGoldDiv=[1-9] — делитель стоимости найма

Nature=[{отсутствие строки}/Undead/Elemental/Rogue/Animal/Hero/People/Mecha] | Нормальный/Нежить/Элементаль/Разбойники/Животные/Герой/Люди/Mech — тип персонажа

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
use num::{integer::Roots, Num};

use super::{
    battle::{control::Control, troop::Troop},
    bonuses::*,
    items::item::{ItemInfo, *},
    locale::*,
    map::{
        event::*,
        map::{GameMap, Tilemap, MAP_SIZE},
        object::{MapBuildingdata, Market, ObjectInfo, ObjectType, RecruitUnit, Recruitment},
    },
    mutrc::SendMut,
    time::time::{Data::*, Time},
    units::{
        unit::{MagicDirection::*, MagicType::*, *},
        unitstats::ModifyUnitStats,
    },
};
use crate::{
    battle::{
        army::{Army, ArmyStats},
        control::Relations,
    },
    items,
    map::{deco::MapDeco, map::TileMap, object::BuildingVariant},
};
use advini::*;
use ini_core::{Item as IniItem, Parser};
use math_thingies::Percent;
use once_cell::sync::Lazy;
use std::{
    any::type_name,
    collections::HashMap,
    default,
    fmt::{Debug, Display},
    io::Read,
    net::{IpAddr, Ipv4Addr},
    ops::Add,
    str::FromStr,
    sync::RwLock,
};
use tracing_mutex::stdsync::TracingMutex as Mutex;

//#[cfg(target_arch = "wasm32")]
//use wasm_bindgen_futures::spawn_local;
pub fn read_file(path: &str) -> Vec<u8> {
    std::fs::read(path.clone()).unwrap()
}

trait CollectInplace {
    type Output;
    fn collect(self) -> Self::Output;
}
impl<T, E> CollectInplace for Vec<Result<T, E>> {
    type Output = Result<Vec<T>, E>;
    fn collect(self) -> Result<Vec<T>, E> {
        self.into_iter().collect()
    }
}
impl<T> CollectInplace for Vec<Option<T>> {
    type Output = Option<Vec<T>>;
    fn collect(self) -> Option<Vec<T>> {
        self.into_iter().collect()
    }
}

pub fn collect_errors<T, K: Display>(
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
pub fn handle_parse<T: FromStr + Display>(
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
fn parse_duo_tuple<T: FromStr>(v: &str) -> Result<(T, T), &'static str> {
    let mut points = v.split(|ch: char| !ch.is_ascii_digit()).map(|string| {
        string
            .parse()
            .or_else(|_| Err("Couldn't parse given string"))
    });
    (|(r1, r2)| Ok((r1?, r2?)))((
        points.next().ok_or("").and_then(|v| v),
        points.next().ok_or("").and_then(|v| v),
    ))
}
const MATCH_ERR: Result<(), &str> = Err("parse.rs: cant match field;");
pub fn match_magictype(
    error_collector: &mut Vec<String>,
    magic_type: &str,
    direction: MagicDirection,
) -> Option<MagicType> {
    match magic_type {
        "LifeMagic" => Some(Life),
        "ElementalMagic" => Some(Elemental),
        "DeathMagic" => Some(Death),
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

pub async fn parse_units<Reader: FileAccess>(
    path: Option<&str>,
) -> Result<(Vec<Unit>, (&'static str, Vec<String>)), String> {
    let mut units = vec![];
    let sections = parse_for_sections::<Reader>(path.unwrap_or("Units.ini")).await;
    let mut error_collector: Vec<String> = Vec::new();
    let mut req_assets = Vec::new();
    let mut upgrades: HashMap<usize, Vec<String>> = HashMap::new();

    for (_, prop) in sections.iter() {
        let mut counter = None;
        let mut bonus_name = "";
        let mut magic_type = "";
        let mut magic_direction = "";
        let mut nature = "";
        let mut name = "";
        let mut description = "";

        let mut cost_hire = None;
        let mut size = None;
        let mut surrender = None;
        let mut _icon_index = None;

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

        let mut next_unit: Vec<String> = Vec::new();
        for (k, value) in prop.iter() {
            let v = &**value;
            match &**k {
                "name" => name = v,
                "descript" => description = v,
                "nature" => nature = v,
                "iconindex" => {
                    _icon_index = handle_parse::<usize>(v, &mut error_collector, "iconindex")
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
                }
                "size" => {
                    size = collect_errors(
                        parse_duo_tuple(v),
                        &mut error_collector,
                        "Field size is incorrect",
                    );
                }
                "bonus" => {
                    bonus_name = v;
                }
                "globalindex" => {
                    counter = handle_parse::<usize>(v, &mut error_collector, "globalindex");
                }
                _ => (),
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
        let hp = hp.unwrap_or(1);
        let xp_up = xp_up.unwrap_or(1);
        let max_xp = max_xp.unwrap_or(1);

        let cost_hire = cost_hire.unwrap_or(1);
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
                magic: damage_magic.unwrap_or(0),
                ranged: damage_ranged.unwrap_or(0),
                hand: damage_hand.unwrap_or(1),
            },
            defence: Defence {
                death_magic: Percent::new(defence_death_magic.unwrap_or(0)),
                elemental_magic: Percent::new(defence_elemental_magic.unwrap_or(0)),
                life_magic: Percent::new(defence_life_magic.unwrap_or(0)),
                hand_percent: Percent::new(defence_hand_percent.unwrap_or(0)),
                ranged_percent: Percent::new(defence_ranged_percent.unwrap_or(0)),
                magic_units: defence_magic.unwrap_or(0),
                hand_units: defence_hand.unwrap_or(0),
                ranged_units: defence_ranged.unwrap_or(0),
            },
            moves: moves.unwrap_or(1),
            max_moves: moves.unwrap_or(1),
            speed: speed.unwrap_or(1),
            vamp: Percent::new(vamp.unwrap_or(0)),
            regen: Percent::new(regen.unwrap_or(0)),
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
                magic_info: magic_type.and_then(|x| Some((x, magic_direction))),
                size: size.unwrap_or((1, 1)),
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
        req_assets.push(format!("unit_{}.png", counter.unwrap() - 1));
        units.push((counter.unwrap(), unit));
        for (index, up) in upgrades.iter() {
            let upgrade = up
                .iter()
                .map(|name| {
                    units
                        .iter()
                        .filter(|unit| unit.1.info.name == *name)
                        .map(|unit| unit.0)
                        .next()
                        .unwrap()
                })
                .collect();
            units[*index].1.info.next_unit = upgrade;
        }
    }
    units.sort_by_key(|v| v.0);
    let units = units.into_iter().map(|v| v.1).collect::<Vec<Unit>>();
    if let Ok(mut units_write) = UNITS.write() {
        units_write.append(&mut units.clone());
    };
    if error_collector.is_empty() {
        Ok((units, ("assets/Icons", req_assets)))
    } else {
        Err(error_collector.join("\n"))
    }
}

#[derive(Clone, Debug)]
pub struct Settings {
    pub max_troops: usize,
    pub locale: String,
    pub additional_locale: String,
    pub fullscreen: bool,
    pub init_size: (u32, u32),
    pub ip: IpAddr,
    pub port: u64,
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            max_troops: 12,
            locale: String::new(),
            additional_locale: String::new(),
            fullscreen: true,
            init_size: (1600, 1200),
            ip: IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
            port: 0,
        }
    }
}

pub static mut SETTINGS: Settings = Settings {
    max_troops: 12,
    locale: String::new(),
    additional_locale: String::new(),
    fullscreen: true,
    init_size: (1600, 1200),
    ip: IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
    port: 0,
};
pub static LOCALE: Lazy<RwLock<Locale>> =
    Lazy::new(|| RwLock::new(Locale::new("Rus".into(), "Eng".into())));

pub async fn parse_settings<Reader: FileAccess>() -> Settings {
    let sections = parse_for_sections::<Reader>("Settings.ini").await;
    let mut max_troops: usize = 0;
    let mut locale = String::new();
    let mut additional_locale = String::new();
    let mut fullscreen = false;
    let mut init_size = None;
    let mut port = 0;
    let mut ip = IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1));
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
                    init_size = Some((
                        parsed.next().unwrap().parse().unwrap(),
                        parsed.next().unwrap().parse().unwrap(),
                    ));
                }
                "port" => {
                    port = value.parse::<u64>().unwrap();
                }
                "ip" => ip = value.parse().unwrap(),
                _ => {}
            }
        }
    }
    let settings = Settings {
        max_troops,
        locale,
        additional_locale,
        fullscreen,
        init_size: init_size.unwrap(),
        ip,
        port,
    };
    unsafe {
        SETTINGS = settings.clone();
    }
    settings
}

pub trait FileAccess {
    async fn read(path: &str) -> Vec<u8>;
    async fn read_as_string(path: &str) -> String {
        String::from_utf8(Self::read(path).await).unwrap()
    }
}
pub struct StupidReader;
impl FileAccess for StupidReader {
    async fn read(path: &str) -> Vec<u8> {
        read_file(path)
    }
}
async fn parse_for_props<Reader: FileAccess>(path: &str) -> HashMap<String, String> {
    let mut props = HashMap::new();
    let ini_doc = Reader::read_as_string(path).await;
    let parser = Parser::new(&*ini_doc).auto_trim(true);
    for item in parser {
        match item {
            IniItem::Section(_) => {}
            IniItem::Property(k, v) => {
                props.insert(k.to_lowercase().into(), v.into());
            }
            IniItem::Blank | IniItem::Action(_) | IniItem::Comment(_) => {}
            IniItem::Error(err) => panic!("{}", err),
        }
    }
    props
}
async fn parse_for_sections<Reader: FileAccess>(
    path: &str,
) -> Vec<(String, HashMap<String, String>)> {
    let ini_doc = Reader::read_as_string(path).await;
    advini::parse_for_sections(&ini_doc)
}
pub type Objects = Vec<ObjectInfo>;
pub async fn parse_objects<Reader: FileAccess>() -> (Objects, (&'static str, Vec<String>)) {
    let mut objects = Vec::new();
    let mut req_assets = Vec::new();
    let sections = parse_for_sections::<Reader>("Objects.ini").await;
    for (sec, prop) in sections.iter() {
        let mut category = "".to_string();
        let mut obj_type = None;
        let mut index = None;
        let name = sec.clone();
        let mut size = (Some(1), Some(1));
        let mut error_collector: Vec<String> = Vec::new();
        let path = if let Some(at) = sec.find(|x: char| x.is_ascii_digit()) {
            sec.clone()
                .split_at_checked(at)
                .and_then(|(a, b)| {
                    if let Some(num) = b.parse::<usize>().ok() {
                        Some(format!("{a}{num:0>3}"))
                    } else {
                        None
                    }
                })
                .unwrap_or_else(|| sec.clone())
                .add(".png")
        } else {
            sec.clone().add(".png")
        };
        req_assets.push(path.clone());
        if let Some(res) = prop.get("index") {
            index = collect_errors(
                res.parse::<usize>(),
                &mut error_collector,
                "Value of field Index ommited as non-usize",
            );
        }
        if let Some(res) = prop.get("size") {
            let mut sizes = res
                .split(|ch: char| !ch.is_ascii_digit())
                .map(|string| Some(string.parse().unwrap()));
            size.0 = sizes.next().unwrap();
            size.1 = sizes.next().unwrap();
        }
        if let Some(res) = prop.get("type") {
            obj_type = Some(match &**res {
                "MapDeco" => ObjectType::MapDeco {
					id: {
						prop.get("id").and_then(|res| {
							collect_errors(
								res.parse::<usize>(),
								&mut error_collector,
								"Value of field Id ommited as non-usize",
							)
						}).unwrap_or_else(|| {
							error!("MapDeco lacks id field");
							0
						})
					}
				},
                "Bridge" | "Building" => {
					let group = prop.get("group").and_then(|res| {
						collect_errors(
							res.parse::<u8>(),
							&mut error_collector,
							"Value of field group ommited as non-u8",
						)
					}).unwrap_or_else(|| {
						error!("Lacking group field");
						0
					});
					let variant = prop.get("variant").and_then(|res| {
						collect_errors(
							res.parse::<u8>(),
							&mut error_collector,
							"Value of field variant ommited as non-u8",
						)
					}).unwrap_or_else(|| {
						error!("Lacking variant field");
						0
					});
					if res == "Building" {
						ObjectType::Building {
							group,
							variant
						}
					} else {
						ObjectType::Bridge {
							group,
							variant
						}
					}
				}
                _ => panic!(
                    "{}",
                    format!("Wrong Object Type - '{}' at section {}", res, sec)
                ),
            })
        }
        category = prop.get("category").cloned().unwrap_or_default();

        if !error_collector.is_empty() {
            panic!("{}", error_collector.join("\n"));
        }

        objects.push((
            index.unwrap(),
            ObjectInfo {
                category,
                name,
                obj_type: obj_type.expect("Cant find Type key!"),
                index: index.expect("Cant find Index key!"),
                size: (
                    size.0.expect("Cant find SizeW key!"),
                    size.1.expect("Cant find SizeH key!"),
                ),
                path: {
                    if let Some(at) = sec.find(|x: char| x.is_ascii_digit()) {
                        sec.clone()
                            .split_at_checked(at)
                            .and_then(|(a, b)| {
                                if let Some(num) = b.parse::<usize>().ok() {
                                    Some(format!("{a}{num:0>3}"))
                                } else {
                                    None
                                }
                            })
                            .unwrap_or_else(|| sec.clone())
                            .add(".png")
                    } else {
                        sec.clone().add(".png")
                    }
                },
            },
        ));
    }
    objects.sort_by(|(id, _), (oth_id, _)| id.cmp(oth_id));
    (
        objects.into_iter().map(|(_, object)| object).collect(),
        ("assets/Objects", req_assets),
    )
}
fn match_magic_variants(magic_type: String) -> MagicVariants {
    match &*magic_type {
        "LifeMagic" => MagicVariants::Life,
        "ElementalMagic" => MagicVariants::Elemental,
        "DeathMagic" => MagicVariants::Death,
        "NoMagic" | "" => MagicVariants::Any,
        _ => MagicVariants::Any,
    }
}
/*
 * d-{stat} - добавление
 * p-{stat} - добавление процента
 * f-{stat} - установить
 */
pub async fn parse_items<Reader: FileAccess>(
    path: Option<&str>,
    lang: &String,
) -> (&'static str, Vec<String>) {
    let mut error_collector: Vec<String> = Vec::new();
    let mut items = vec![];
    let mut req_assets = Vec::new();

    let secs = parse_for_sections::<Reader>(path.unwrap_or("Rus_Artefacts.ini")).await;
    for (sec, props) in secs {
        let mut cost: Option<i64> = None;
        let mut description = None;
        let mut name = None;
        let mut itemtype = None;
        let mut modify = ModifyUnitStats::default();
        let direction = MagicDirection::ToAll;
        let mut icon = None;
        let mut magic = MagicVariants::Any;
        let mut index: Option<u64> = None;
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
                    req_assets.push(value.to_string());
                }
                "cost" => cost = handle_parse(value, &mut error_collector, "cost"),
                "magic" => {
                    magic = match_magic_variants(value.into());
                }
                "type" => {
                    let itemtype_name = value;
                    itemtype = match value {
                        "Staff" => ArtifactType::Weapon(WeaponType::Magic),
                        "ShotWeapon" => ArtifactType::Weapon(WeaponType::Ranged),
                        "BlowWeapon" => ArtifactType::Weapon(WeaponType::Hand),
                        "Ring" => ArtifactType::Ring,
                        "Armor" => ArtifactType::Armor,
                        "Helm" | "Helmet" => ArtifactType::Helmet,
                        "Shield" => ArtifactType::Shield,
                        "Amulet" => ArtifactType::Amulet,
                        "Item" => ArtifactType::Item,
                        "Potion" => ArtifactType::Potion,
                        _ => panic!("Wrong Item Type - {}!", value),
                    }
                    .into()
                }
                "d-hits" => {
                    modify.max_hp.add = add_opt(modify.max_hp.add, value.parse::<i64>().ok());
                    //modify.hp.add = add_opt(modify.hp.add, value.parse::<i64>().ok());
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
                    //modify.moves.add = add_opt(modify.moves.add, value.parse().ok());
                }
                "d-initiative" => modify.speed.add = add_opt(modify.speed.add, value.parse().ok()),
                "d-vampirizm" => modify.vamp.add = add_opt(modify.vamp.add, value.parse().ok()),
                "d-regen" => modify.regen.add = add_opt(modify.regen.add, value.parse().ok()),

                "p-hits" => {
                    modify.max_hp.percent_add = add_opt(
                        modify.max_hp.percent_add,
                        Percent::new(value.parse().unwrap()).into(),
                    );
                    // modify.hp.percent_add = add_opt(
                    //     modify.hp.percent_add,
                    //     Percent::new(value.parse().unwrap()).into(),
                    // );
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
                }
                "f-attackblow" => modify.damage.hand.set = value.parse().ok(),
                "f-attackshot" => modify.damage.ranged.set = value.parse().ok(),
                "f-magicpower" => modify.damage.magic.set = value.parse().ok(),
                "f-defenceblow" => modify.defence.hand_units.set = value.parse().ok(),
                "f-defenceshot" => modify.defence.ranged_units.set = value.parse().ok(),
                "f-defencemagic" => modify.defence.magic_units.set = value.parse().ok(),
                "f-manevres" => {
                    modify.max_moves.set = value.parse().ok();
                }
                "f-initiative" => modify.speed.set = value.parse().ok(),
                "f-vampirizm" => modify.vamp.set = value.parse().ok(),
                "f-regen" => modify.regen.add = value.parse().ok(),
                "bonus" => bonus = Some(Bonus::from(value)),
                _ => {}
            }
        }
        items.push((
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
                magic_req: magic,
                icon: icon.expect("No icon key").into(),
                sells: cost.unwrap() > 0,
                bonus,
                itemtype: itemtype.expect(&*format!("{name}", name = name.unwrap())),
                modify,
            },
        ));
    }
    items.sort_by_key(|x| x.0);
    ITEMS
        .write()
        .unwrap()
        .extend(items.iter().map(|(_, v)| v.clone()));
    assert!(ITEMS.read().unwrap().len() > 0);
    ("assets/Items", req_assets)
}

trait IsRus {
    fn is_rus_alphabet(&self) -> bool;
}
impl IsRus for char {
    fn is_rus_alphabet(&self) -> bool {
        matches!(*self, 'А'..='Я' | 'а'..='я' | 'ё' | 'Ё')
    }
}

fn split_and_parse<N: Num>(string: String) -> Vec<N> {
    string
        .split(|ch: char| !ch.is_ascii_digit())
        .map(|string| N::from_str_radix(string, 10).unwrap_or(N::zero()))
        .collect()
}

fn parse_cmp<V: Ord + FromStr>(v: String) -> Cmp<V>
where
    <V as FromStr>::Err: Debug,
{
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
        _ => false,
    }
}

async fn parse_events<Reader: FileAccess>(path: String, locale: &mut Locale) -> Vec<Event> {
    let mut events = Vec::new();
    for (sec, props) in parse_for_sections_localised::<Reader>(&*path, locale).await {
        let event = <Event as Sections>::from_section(props).unwrap();
        events.push(event.0);
    }
    events
}

async fn parse_mapdata<Reader: FileAccess>(
    path: String,
    units: &Vec<Unit>,
    locale: &mut Locale,
    objects: &Objects,
) -> (
    TileMap<usize>,
    Vec<MapDeco>,
    Vec<MapBuildingdata>,
    Vec<Army>,
) {
    let mut tilemap: Option<TileMap<usize>> = None;
    let mut decomap = Vec::default();

    let mut armys = Vec::new();
    let mut buildings = Vec::new();

    for (sec, props) in parse_for_sections::<Reader>(&*path).await {
        match &*sec {
            "Tilemaps" => {
                for prop in props {
                    match &*prop.0 {
                        "tilemap" => {
                            tilemap = Some({
                                let tilemap = prop
                                    .1
                                    .split(|ch: char| !ch.is_ascii_digit())
                                    .map(|ch| ch.parse::<usize>().unwrap());
                                TileMap::new(tilemap)
                            });
                        }
                        "decomap" => decomap = { Vec::<MapDeco>::eat(prop.1.chars()).unwrap().0 },
                        _ => {}
                    }
                }
            }
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
                    let prop = (prop.0, process_locale(prop.1, locale));
                    match &*prop.0 {
                        "id" => id = prop.1.parse().ok(),
                        "name" => stats.army_name = prop.1,
                        "mana" => stats.mana = prop.1.parse().unwrap(),
                        "gold" => stats.gold = prop.1.parse().unwrap(),
                        "inventory" => {
                            inv = split_and_parse(prop.1)
                                .iter()
                                .map(|num| Some(items::item::Item { index: *num }))
                                .collect()
                        }
                        "pos" => {
                            let things =
                                prop.1.split_once(|ch: char| !ch.is_ascii_digit()).unwrap();
                            pos = (things.0.parse().unwrap(), things.1.parse().unwrap());
                        }
                        "active" => active = str_bool(prop.1),
                        "troops" => {
                            in_troops = prop
                                .1
                                .split(",")
                                .map(|string| string.split_once(";").unwrap())
                                .map(|(num, lvl)| {
                                    (num.parse::<usize>().unwrap(), lvl.parse::<i64>().unwrap())
                                })
                                .map(|(num, _)| {
                                    let mut troop = Troop::empty();
                                    troop.unit = units[num].clone();
                                    troop.unit.army = armys.len();
                                    SendMut::new(troop)
                                })
                                .collect()
                        }
                        "player" => control = Control::Player(prop.1.parse().unwrap()),
                        "main" => {
                            let things =
                                prop.1.split_once(|ch: char| !ch.is_ascii_digit()).unwrap();
                            let troop = Troop {
                                unit: {
                                    let mut unit =
                                        units[things.0.parse::<usize>().unwrap()].clone();
                                    unit.army = armys.len();
                                    unit
                                },
                                is_main: true,
                                is_free: true,
                                was_payed: true,
                                pos: UnitPos::from_index(0),
                                custom_name: Some(things.1.into()),
                            };
                            main = Some(SendMut::new(troop));
                        }
                        _ => {}
                    }
                }
                let mut troops = vec![main.unwrap()];
                troops.append(&mut in_troops);

                armys.push((
                    id.unwrap(),
                    Army::new(troops, stats, inv, pos, active, control),
                ));
            }
            x if x.starts_with("Building") => {
                let id = props.get("id").and_then(|x| x.parse().ok()).unwrap_or(0);
                let name = props.get("name").cloned().unwrap_or_else(|| String::new());
                let object_name = props
                    .get("object")
                    .cloned()
                    .unwrap_or_else(|| String::new());
                let desc = props.get("desc").cloned().unwrap_or_else(|| String::new());
                let building_type = props.get("type").cloned().unwrap_or_else(|| String::new());
                let mut event = Vec::new();
                let mut units = Vec::new();
                let mut recruitment = None;
                let cost_modify = 1.;
                let mut market = None;
                let items = props
                    .get("items")
                    .and_then(|x| {
                        Some(
                            split_and_parse::<usize>(x.to_string())
                                .iter()
                                .map(|index| Item { index: *index })
                                .collect(),
                        )
                    })
                    .unwrap_or(vec![]);
                let itemcost_range = props
                    .get("itemcost_range")
                    .and_then(|x| parse_duo_tuple::<u64>(&x).ok())
                    .unwrap_or((0u64, 1000u64));
                let max_items = props.get("id").and_then(|x| x.parse().ok()).unwrap_or(10);
                let control = Control::PC;
                let pos = props
                    .get("pos")
                    .and_then(|x| parse_duo_tuple::<usize>(&x).ok())
                    .unwrap_or((0, 0));
                let defense = props
                    .get("defense")
                    .and_then(|x| x.parse().ok())
                    .unwrap_or(0);
                let gold_income = props
                    .get("income")
                    .and_then(|x| x.parse().ok())
                    .unwrap_or(0);
                let owner = props.get("owner").and_then(|x| x.parse().ok());
                for prop in props {
                    let prop = (prop.0, process_locale(prop.1, locale));
                    match &*prop.0 {
                        "recruit" => {
                            units = prop
                                .1
                                .split(",")
                                .map(|string| string.split_once(";").unwrap())
                                .map(|(id, num)| {
                                    (id.parse().unwrap(), num.parse::<usize>().unwrap())
                                })
                                .map(|(id, num)| RecruitUnit {
                                    unit: id,
                                    count: num,
                                })
                                .collect();
                        }
                        "events" => event = split_and_parse::<usize>(prop.1),
                        _ => {}
                    }
                }
                if !items.is_empty() {
                    market = Market {
                        itemcost_range,
                        items,
                        max_items,
                    }
                    .into();
                }
                if !units.is_empty() {
                    recruitment = Recruitment { cost_modify, units }.into();
                }
                buildings.push((
                    id,
                    MapBuildingdata {
                        owner_name: String::new(),
                        spells_to_learn: Vec::new(),
                        variant: BuildingVariant::Castle,
                        garrison: Vec::new(),
                        group: 0,
                        mana_income: 0,
                        relations: Relations::default(),
                        id: objects
                            .into_iter()
                            .position(|obj| obj.name == object_name)
                            .unwrap(),
                        name,
                        desc,
                        events: event,
                        market,
                        recruitment,
                        pos,
                        additional_defense: defense,
                        gold_income,
                        owner,
                    },
                ));
            }
            _ => {}
        }
    }
    (
        tilemap.unwrap(),
        decomap,
        {
            buildings.sort_by(|(id, _), (oth_id, _)| id.cmp(oth_id));
            buildings
                .into_iter()
                .map(|(_, building)| building)
                .collect()
        },
        {
            armys.sort_by(|(id, _), (oth_id, _)| id.cmp(oth_id));
            armys.into_iter().map(|(_, army)| army).collect()
        },
    )
}

pub async fn parse_story<Reader: FileAccess>(
    units: &Vec<Unit>,
    objects: &Objects,
    lang: &String,
    additional_lang: &String,
) -> (GameMap, Vec<Event>) {
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

    for (sec, props) in parse_for_sections::<Reader>(&format!("{map_path}")).await {
        for prop in props {
            let prop = (prop.0, process_locale(prop.1, &mut locale));
            match &*prop.0 {
                "filepath" => match &*sec {
                    "Locale" => {
                        parse_map_locale::<Reader>(
                            &*format!("{}/{}", map_dir, prop.1),
                            &[&locale.main_lang.clone(), &locale.additional_lang.clone()],
                            &mut locale,
                        );
                    }
                    "MapData" => mapdata_path = prop.1.into(),
                    "EventsAndLights" => events_path = prop.1.into(),
                    _ => {}
                },
                "name" => name = Some(prop.1),
                "desc" => description = Some(prop.1),
                "start_time" => start_time = Time::from_data(prop.1, [YEAR, MONTH, DAY, HOUR]),
                "start_gold" => start_gold = handle_parse(prop.1, &mut err_coll, "start_gold"),
                "start_mana" => start_mana = handle_parse(prop.1, &mut err_coll, "start_mana"),
                "start_items" => {
                    start_items = prop
                        .1
                        .split(|ch: char| !ch.is_ascii_digit())
                        .map(|string| handle_parse::<usize>(string, &mut err_coll, "start_items"))
                        .collect();
                }
                _ => {}
            }
        }
    }
    if !err_coll.is_empty() {
        panic!("{}", err_coll.join("\n"));
    }
    let mapdata = parse_mapdata::<Reader>(
        format!("{map_dir}{}", mapdata_path.unwrap()),
        units,
        &mut locale,
        objects,
    )
    .await;
    let events =
        parse_events::<Reader>(format!("{map_dir}{}", events_path.unwrap()), &mut locale).await;

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
