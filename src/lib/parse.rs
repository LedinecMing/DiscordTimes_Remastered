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
use {
    ini::ini,
    std::{
        fmt::Display,
        collections::HashMap
    },
    math_thingies::Percent,
    super::{
        bonuses::bonuses::*,
        units::{
            unit::{*, MagicDirection::*, MagicType::*},
        }
    },
};

fn collect_errors<T, K: Display>(for_check: Result<T, K>, collector: &mut Vec<String>, additional: &str) -> Option<T> {
    match for_check {
        Ok(value) => Some(value),
        Err(info) => {
            collector.push(format!("Error: {}; additional: {}", info.to_string(), additional));
            None
}   }   }
pub fn parse_units() -> HashMap<usize, Unit1> {
    let mut units = HashMap::new();
    let sections = ini!("Units.ini");
    let mut counter : usize = 0;
    for (sec, prop) in sections.iter() {
        println!("Section: {:?}", sec);
        let mut name = "";
        let mut description = "";
        let mut magic_type = "";
        let mut magic_direction = "";
        let mut nature = "";
        let mut bonus_name = "";

        let mut cost = None;
        let mut surrender = None;

        let mut hp = None;

        let mut max_xp = None;
        let mut xp_up = None;

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

        let mut error_collector: Vec<String> = Vec::new();
        for (k, value) in prop.iter() {
            println!("{}: {:?}", k.clone(), value.clone());
            if let Some(v) = value.as_ref() {
                let v = &**v;
                match &**k {
                    "name" => { name = v; },
                    "descript" => { description = v; },
                    "nature" => { nature = v; },
                    "cost" => {
                        cost = collect_errors(v.parse::<u64>(), &mut error_collector,
                                              "Value of field cost omitted as non-u64");
                    },
                    "surrender" => {
                        surrender = collect_errors(v.parse::<u64>(), &mut error_collector,
                                                   "Value of field surrender omitted as non-u64");
                    },
                    "hits" => {
                        hp = collect_errors(v.parse::<i64>(), &mut error_collector,
                                            "Value of field hp omitted as non-i64");
                    },
                    "attackblow" | "attackhand" => {
                        damage_hand = collect_errors(v.parse::<u64>(), &mut error_collector,
                                                     "Value of field damage_hand omitted as non-u64");
                    }
                    "attackshot" | "attackranged" => {
                        damage_ranged = collect_errors(v.parse::<u64>(), &mut error_collector,
                                                       "Value of field damage_ranged omitted as non-u64");
                    }
                    "magic" | "attackmagic" => {
                        magic_type = v;
                    }
                    "defenceblow" | "defencehand" => {
                        defence_hand = collect_errors(v.parse::<u64>(), &mut error_collector,
                                                     "Value of field defence_hand omitted as non-u64");
                    }
                    "defenceshot" | "defenceranged" => {
                        defence_ranged = collect_errors(v.parse::<u64>(), &mut error_collector,
                                                     "Value of field defence_ranged omitted as non-u64");
                    }
                    "defencemagic" => {
                        defence_magic = collect_errors(v.parse::<u64>(), &mut error_collector,
                                                     "Value of field defence_magic omitted as non-u64");
                    }
                    "protectdeath" => {
                        defence_death_magic = collect_errors(v.parse::<i16>(), &mut error_collector,
                                                           "Value of field defence_death_magic omitted as non-i16");
                    }
                    "protectlife" => {
                        defence_life_magic = collect_errors(v.parse::<i16>(), &mut error_collector,
                                                           "Value of field defence_life_magic omitted as non-i16");
                    }
                    "protectelemental" => {
                        defence_elemental_magic = collect_errors(v.parse::<i16>(), &mut error_collector,
                                                           "Value of field defence_elemental_magic omitted as non-i16");
                    }
                    "protectblow" | "protecthand" => {
                        defence_hand_percent = collect_errors(v.parse::<i16>(), &mut error_collector,
                                                           "Value of field defence_hand_percent omitted as non-i16");
                    }
                    "protectshot" | "protectranged" => {
                        defence_ranged_percent = collect_errors(v.parse::<i16>(), &mut error_collector,
                                                           "Value of field defence_ranged_percent omitted as non-i16");
                    }
                    "magicdirection" => {
                        magic_direction = v;
                    },
                    "manevres" | "moves" => {
                        moves = collect_errors(v.parse::<u64>(), &mut error_collector,
                                               "Value of field defence_elemental_magic omitted as non-u64");
                    },
                    "intiative" | "speed" => {
                        speed = collect_errors(v.parse::<u64>(), &mut error_collector,
                                               "Value of field speed omitted as non-u64");
                    }
                    "vampirism" => {
                        vamp = collect_errors(v.parse::<i16>(), &mut error_collector,
                                              "Value of field vamp omitted as non-i16");
                    },
                    "regen" => {
                        regen = collect_errors(v.parse::<i16>(), &mut error_collector,
                                               "Value of field vamp omitted as non-i16");
                    },
                    "levelmultipler" => {
                        xp_up = collect_errors(v.parse::<i16>(), &mut error_collector,
                                               "Value of field xp_up omitted as non-i16");
                    },
                    "startexperience" => {
                        max_xp = collect_errors(v.parse::<u64>(), &mut error_collector,
                                               "Value of field max_xp omitted as non-u64");
                    },
                    "bonus" => { bonus_name = v; }
                    _ => {}
        }   }   }

        let match_err: Result<(), &str> = Err("parse.rs: cant match field;");

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
                collect_errors(match_err, &mut error_collector,
                               &*format!("Field MagicDirection is invalid: {}", magic_direction));
                ToAll
        }   };
        let magic_type = match magic_type {
            "LifeMagic" => Some(Life(magic_direction)),
            "ElementalMagic" => Some(Elemental(magic_direction)),
            "DeathMagic" => Some(Death(magic_direction)),
            "NoMagic" | "" => None,
            _ => {
                collect_errors(match_err, &mut error_collector,
                               &*format!("Field MagicType is invalid: {}", magic_type));
                None
        }   };
        let bonus = match match_bonus(bonus_name) {
            Ok(bonus) => bonus,
            Err(_) => {
                collect_errors(match_err, &mut error_collector,
                               &*format!("Field Bonus is invalid: {}", bonus_name));
                Box::new(NoBonus {})
        }   };
        let unit_type = match nature {
            "Alive" | "" => UnitType::Alive,
            "Rogue" => UnitType::Rogue,
            "Undead" => UnitType::Undead,
            "Hero" => UnitType::Alive,
            _ => {
                collect_errors(match_err, &mut error_collector,
                               &*format!("Field Nature is invalid: {}", nature));
                UnitType::Alive
        }   };

        assert!(error_collector.is_empty(), "{}", error_collector.join("\n"));
        let hp = hp.unwrap();
        let xp_up = Percent::new(xp_up.unwrap());
        let max_xp = max_xp.unwrap();

        let unit = Unit1 {
            stats: UnitStats {
                hp,
                max_hp: hp,
                damage: Power {
                    magic: damage_hand.unwrap(),
                    ranged: damage_ranged.unwrap(),
                    hand: damage_magic.unwrap()
                },
                defence: Defence {
                    death_magic: Percent::new(defence_death_magic.unwrap()),
                    elemental_magic: Percent::new(defence_elemental_magic.unwrap()),
                    life_magic: Percent::new(defence_life_magic.unwrap()),
                    hand_percent: Percent::new(defence_hand_percent.unwrap()),
                    ranged_percent: Percent::new(defence_ranged_percent.unwrap()),
                    magic_units: defence_magic.unwrap(),
                    hand_units: defence_hand.unwrap(),
                    ranged_units: defence_ranged.unwrap()
                },
                moves: moves.unwrap(),
                max_moves: moves.unwrap(),
                speed: speed.unwrap(),
                vamp: Percent::new(vamp.unwrap()),
                regen: Percent::new(regen.unwrap())
            },
            info: UnitInfo {
                name: name.into(),
                descript: description.into(),
                cost: cost.unwrap(),
                unit_type,
                magic_type,
                surrender,
                lvl: LevelUpInfo {
                    stats: UnitStats::empty(),
                    xp_up,
                    max_xp
            }   },
            lvl: UnitLvl {
                lvl: 0,
                max_xp,
                xp: 0
            },
            inventory: UnitInventory::empty(),
            army: 0,
            bonus,
            effects: vec![]
        };
        units.insert(counter,
            unit
        );
        counter += 1;
    }
    units
}
