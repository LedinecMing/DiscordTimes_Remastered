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

LevelMultipler=[{число}; standard = 140] — множитель уровня (y*lvl → y+1lvl = StartExpirience × (LevelMultipler / 100) ^ y. * – y > 2)

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
use std::{any::type_name, fs::File, io::Read, ops::Add};

use super::{
    bonuses::bonuses::*,
    items::item::{Item as GameItem, ItemInfo, ItemType, *},
    map::object::{ObjectInfo, ObjectType},
    units::{
        unit::{MagicDirection::*, MagicType::*, *},
        unitstats::{Modify, ModifyDefence, ModifyPower, ModifyUnitStats},
    },
};
use crate::{lib::mutrc::SendMut, LOCALE};
use ini_core::{Item, Parser};
use math_thingies::Percent;
use std::{collections::HashMap, fmt::Display, str::FromStr};

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
pub fn parse_units() -> HashMap<usize, Unit> {
    let mut units = HashMap::new();
    let sections = parse_for_sections("Units.ini");
    let mut counter = None;
    for (sec, prop) in sections.iter() {
        let mut name = "";
        let mut description = "";
        let mut magic_type = "";
        let mut magic_direction = "";
        let mut nature = "";
        let mut bonus_name = "";

        let mut cost_hire = None;
        let mut cost = None::<u64>;
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
                    moves = handle_parse::<u64>(v, &mut error_collector, "moves");
                }
                "initiative" | "speed" => {
                    speed = handle_parse::<u64>(v, &mut error_collector, "speed");
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
                "nextunit1" | "nextunit2" | "nextunit3" => next_unit.push(v.into()),
                "bonus" => {
                    bonus_name = v;
                }
                "globalindex" => {
                    counter = handle_parse::<usize>(v, &mut error_collector, "globalindex");
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
        let bonus = match match_bonus(bonus_name) {
            Ok(bonus) => bonus,
            Err(_) => {
                collect_errors(
                    MATCH_ERR,
                    &mut error_collector,
                    &*format!("Field Bonus is invalid: {}", bonus_name),
                );
                Box::new(NoBonus {})
            }
        };
        let unit_type = match nature {
            "People" | "" => UnitType::People,
            "Rogue" => UnitType::Rogue,
            "Undead" => UnitType::Undead,
            "Hero" => UnitType::Hero,
            _ => {
                collect_errors(
                    MATCH_ERR,
                    &mut error_collector,
                    &*format!("Field Nature is invalid: {}", nature),
                );
                UnitType::People
            }
        };

        assert!(error_collector.is_empty(), "{}", error_collector.join("\n"));
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
                next_unit,
                magic_type,
                surrender,
                lvl: LevelUpInfo {
                    stats: UnitStats::empty(),
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
    units
}

#[derive(Debug)]
pub struct Settings {
    pub(crate) max_troops: usize,
    pub locale: String,
}
#[derive(Debug)]
pub struct Locale(HashMap<String, String>);
impl Locale {
    pub fn get<K: Into<String> + Copy>(&self, id: K) -> String {
        self.0
            .get(&id.into())
            .expect(&*format!("Cant find locale key {}", id.into()))
            .clone()
    }
    pub fn insert<V: Into<String>, K: Into<String>>(&mut self, key: K, value: V) {
        self.0.insert(key.into(), value.into());
    }
    pub fn new() -> Self {
        Locale(HashMap::new())
    }
}
pub fn parse_settings() -> Settings {
    let sections = parse_for_sections("Settings.ini");
    let mut max_troops: usize = 0;
    let mut locale = String::new();
    for (sec, prop) in sections.iter() {
        for (k, value) in prop.iter() {
            match &**k {
                "max_troops" => {
                    max_troops = value
                        .parse::<usize>()
                        .expect("Field max_troops is not usize type")
                }
                "locale" => locale = value.clone(),
                _ => {}
            }
        }
    }
    Settings { max_troops, locale }
}
pub fn parse_locale(language: String) {
    let mut locale = LOCALE.lock().unwrap();
    let props = parse_for_props(&*format!("{}_Locale.ini", language));
    for (k, value) in props {
        locale.insert(k, value);
    }
}
fn parse_for_props(path: &str) -> HashMap<String, String> {
    let mut props = HashMap::new();
    let mut ini_doc = String::new();
    File::open(path)
        .unwrap()
        .read_to_string(&mut ini_doc)
        .unwrap();
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
fn parse_for_sections(path: &str) -> HashMap<String, HashMap<String, String>> {
    let mut hashmap = HashMap::new();
    let mut old_sec = "";
    let mut props = HashMap::new();
    let mut ini_doc = String::new();
    File::open(path)
        .unwrap()
        .read_to_string(&mut ini_doc)
        .unwrap();
    let parser = Parser::new(&*ini_doc).auto_trim(true);
    for item in parser {
        match item {
            Item::Section(sec) => {
                if !old_sec.is_empty() {
                    hashmap.insert(old_sec.into(), props);
                    props = HashMap::new();
                    old_sec = sec;
                } else {
                    old_sec = sec
                }
            }
            Item::Property(k, v) => {
                props.insert(k.to_lowercase().into(), v.into());
            }
            Item::Blank | Item::Action(_) | Item::Comment(_) => {}
            Item::Error(err) => panic!("{}", err),
        }
    }
    hashmap.insert(old_sec.into(), props);
    props = HashMap::new();
    hashmap
}
pub fn parse_objects() -> HashMap<usize, ObjectInfo> {
    let mut objects = HashMap::new();
    let sections = parse_for_sections("Objects.ini");
    for (sec, prop) in sections.iter() {
        let mut category = "".into();
        let mut obj_type = None;
        let mut index = None;
        let mut size = (Some(1), Some(1));
        let mut error_collector: Vec<String> = Vec::new();
        for (k, v) in prop.iter() {
            match &**k {
                "index" => {
                    index = collect_errors(
                        v.parse::<usize>(),
                        &mut error_collector,
                        "Value of field Index ommited as non-usize",
                    )
                }
                "sizew" => {
                    size.0 = collect_errors(
                        v.parse::<u8>(),
                        &mut error_collector,
                        "Value of field SizeW ommited as non-u8",
                    )
                }
                "sizeh" => {
                    size.1 = collect_errors(
                        v.parse::<u8>(),
                        &mut error_collector,
                        "Value of field SizeH ommited as non-u8",
                    )
                }
                "type" => {
                    obj_type = Some(match &**v {
                        "MapDeco" => ObjectType::MapDeco,
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
        assert!(error_collector.is_empty(), "{}", error_collector.join("\n"));
        objects.insert(
            index.unwrap(),
            ObjectInfo {
                category,
                obj_type: obj_type.expect("Cant find Type key!"),
                index: index.expect("Cant find Index key!"),
                size: (
                    size.0.expect("Cant find SizeW key!"),
                    size.1.expect("Cant find SizeH key!"),
                ),
                path: sec.clone().add(".png"),
            },
        );
    }
    objects
}
fn match_magic_variants(
    error_collector: &mut Vec<String>,
    magic_type: String,
    direction: MagicDirection,
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
pub fn parse_items(lang: &String) {
    let mut error_collector: Vec<String> = Vec::new();
    let mut items = ITEMS.lock().unwrap();
    let secs = parse_for_sections(&*format!("{}_Artefacts.ini", lang));
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
        let mut itemtype_name = "";
        for (k, value) in props.iter() {
            let value = &**value;
            match &**k {
                "globalindex" => index = handle_parse(value, &mut error_collector, "globalindex"),
                "name" => name = Some(value.clone()),
                "descript" => description = Some(value.clone()),
                "icon" => icon = Some(value.clone()),
                "cost" => cost = handle_parse(value, &mut error_collector, "cost"),
                "magic" => {
                    magic = match_magic_variants(&mut error_collector, value.into(), direction);
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
                "bonus" => bonus = match_bonus(&*value).ok(),
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
