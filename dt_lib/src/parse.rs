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
use indexmap::IndexMap;
use log::error;
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
    }, effects::{EffectInfo, EffectLifetime, StatusEffect}, items, map::{deco::MapDeco, map::TileMap, object::BuildingVariant}, registry::{GameInfo, Items, Objects, Settings, Units}, units::unitstats::Modify
};
use advini::*;
use ini_core::{Item as IniItem, Parser};
use math_thingies::Percent;
use once_cell::sync::Lazy;
use std::{
    any::type_name,
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
    std::fs::read(path).unwrap_or_else(|x| panic!("{}/{}", x.to_string(), path))
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
        "LifeMagic" => Some(LifeMagic),
        "ElementalMagic" => Some(ElementalMagic),
        "DeathMagic" => Some(DeathMagic),
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

pub async fn parse_bonuses<Reader: FileAccess>(
    path: Option<&str>,
	registry: &mut GameInfo
) -> Result<(&'static str, Vec<String>), String> {
	let mut bonus = BonusInfo::default();
	
	let reg = json5::from_str::<Vec<BonusInfo>>(&Reader::read_as_string("bonuses.json5").await).unwrap();
	std::fs::write("bonuses.json5", json5::to_string(&reg).unwrap());
	let schema = schemars::schema_for!(BonusInfo);
	for bonus in reg {
		let id = bonus.id.clone();
		registry.bonuses.register(bonus, id);
	}
	std::fs::write("schema.json", json5::to_string(&schema).unwrap());
	Ok(("", vec![]))
}

pub async fn parse_effects<Reader: FileAccess>(
    path: Option<&str>,
	registry: &mut GameInfo
) -> Result<(&'static str, Vec<String>), String> {

	let reg = &mut registry.effects;
	let reg = json5::from_str::<Vec<EffectInfo>>(&Reader::read_as_string(path.unwrap_or("effects.json5")).await).unwrap();
	std::fs::write(path.unwrap_or("effects.json5"), json5::to_string(&reg).unwrap());
	Ok(("", vec![]))
}
pub async fn parse_units<Reader: FileAccess>(
    path: Option<&str>,
	registry: &mut GameInfo
) -> Result<(&'static str, Vec<String>), String> {

    let sections = parse_for_sections::<Reader>(path.unwrap_or("Units.ini")).await;
    let mut error_collector: Vec<String> = Vec::new();
    let mut req_assets = Vec::new();
    let mut upgrades: IndexMap<usize, Vec<String>> = IndexMap::new();

    for (_, prop) in sections.iter() {
		let unit = UnitInfo::from_section(prop.clone(), Default::default()).unwrap_or_else(|err| {
			panic!("{err}: {:?}", prop.clone());
		});
		let name = unit.0.name.clone();
		req_assets.push(format!("unit_{}.png", unit.0.icon_index - 1));
		registry.units.register(unit.0, name);
	}
    if error_collector.is_empty() {
        Ok(("assets/Icons", req_assets))
    } else {
        Err(error_collector.join("\n"))
    }
}

pub async fn parse_settings<Reader: FileAccess>() -> Settings {
    let sections = parse_for_sections::<Reader>("Settings.ini").await;
    let mut max_troops: usize = 0;
    let mut locale = String::new();
    let mut additional_locale = String::new();
    let mut fullscreen = false;
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
                "port" => {
                    port = value.parse::<u64>().unwrap();
                }
                "ip" => ip = value.parse().unwrap(),
                _ => {}
            }
        }
    }
    let settings = Settings {
        locale,
        additional_locale,
        fullscreen,
        ip,
        port,
    };
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
async fn parse_for_props<Reader: FileAccess>(path: &str) -> IndexMap<String, String> {
    let mut props = IndexMap::new();
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
) -> Vec<(String, IndexMap<String, String>)> {
    let ini_doc = Reader::read_as_string(path).await;
    advini::parse_for_sections(&ini_doc)
}
pub async fn parse_objects<Reader: FileAccess>(registry: &mut GameInfo) -> (&'static str, Vec<String>) {
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

        registry.objects.register(
            ObjectInfo {
                category,
                name: name.clone(),
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
			name
        );
    }
    ("assets/Objects", req_assets)
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
	registry: &mut GameInfo
) -> (&'static str, Vec<String>) {
    let mut error_collector: Vec<String> = Vec::new();
    let mut req_assets = Vec::new();

    let secs = parse_for_sections::<Reader>(path.unwrap_or("Rus_Artefacts.ini")).await;
    for (sec, props) in secs {
        let Some((item, rest)) = ItemInfo::from_section(props.clone(), ()).ok() else { continue; };
		let name = item.name.clone();
		registry.items.register(item, name);
    }
    ("assets/Items", req_assets)
}

fn split_and_parse<N: Num>(string: String) -> Vec<N> {
    string
        .split(|ch: char| !ch.is_ascii_digit())
        .map(|string| N::from_str_radix(string, 10).unwrap_or(N::zero()))
        .collect()
}

fn str_bool(v: String) -> bool {
    match &*v.to_lowercase() {
        "true" | "1" | "t" | "y" => true,
        _ => false,
    }
}

async fn parse_events<Reader: FileAccess>(path: String, locale: &mut Locale, registry: &GameInfo) -> Vec<Event> {
    let mut events = Vec::new();
    for (sec, props) in parse_for_sections_localised::<Reader>(&*path, locale).await {
        let event = <Event as Sections>::from_section(props, ()).unwrap();
        events.push(event.0);
    }
    events
}

async fn parse_mapdata<Reader: FileAccess>(
    path: String,
    locale: &mut Locale,
	registry: &GameInfo,
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
                        "decomap" => decomap = { Vec::<MapDeco>::eat(&prop.1, ()).unwrap().1 },
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
                                    let mut troop = Troop::new((registry.units[num].clone(), &registry.bonuses).into());
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
                                        (registry.units[things.0.parse::<usize>().unwrap()].clone(), &registry.bonuses).into();
                                    unit
                                },
                                is_main: true,
                                is_free: true,
                                was_payed: true,
                                pos: UnitPos::from_index(0, 6),
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
                    Army::new(troops, stats, inv, pos, active, control, registry),
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
                        id: registry.objects.inner
                            .iter()
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
    lang: &String,
    additional_lang: &String,
	registry: &GameInfo,
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
        &mut locale,
        registry
    )
    .await;
    let events =
        parse_events::<Reader>(format!("{map_dir}{}", events_path.unwrap()), &mut locale, registry).await;

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
