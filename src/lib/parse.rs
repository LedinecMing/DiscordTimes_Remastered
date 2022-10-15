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

Bonus=[{отсутствие строки}/SpearDefense/HorseAtack/ArmorIgnore/ArmyMedic/Merchant/DeathCurse/GodAnger/GodStrike/Unvulnerabe/VampirsGist/OldVampirsGist/Evasive/Ghost/Artillery/Garrison/AddPayment/Poison/Dead/FastDead/Counterblow/FlankStrike] | Длинное Оружие/Быстрая Атака/Проникающий Удар/Лекарское Умение/Торговец-Эксперт/Проклятие Смерти/Кара Господня/Гнев Господен/Неуязвимость/Тёмный Дар/Тёмное Искусство/Увертливость/Яростный Дух/Шквальная Атака/Гарнизон/Тыловая Служба/Отравленное Оружие/Мертвец/Быстрый Мертвец/Контрудар/Фланговый удар — бонус

// развитие отряда

NextUnit1=[Name] — развитие в персонажа

NextUnit1Level=[{число}; standard = 1] — необходимый уровень для развития в отряде ИИ

NextUnit2=[Name] — развитие в персонажа

NextUnit2Level=[{число}; standard = 1] — необходимый уровень для развития в отряде ИИ

NextUnit3=[Name] — развитие в персонажа

NextUnit3Level=[{число}; standard = 1] — необходимый уровень для развития в отряде ИИ

// боевые характеристики

Hits=[{число}] — хиты

AttackBlow=[{отсутствие строки}/{число}] — рукопашная атака

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
    super::{
        bonuses::bonuses::*,
        math::Percent,
        units::{
            unit::{Unit, *},
            units::*
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
fn parse_units() -> HashMap<usize, Box<dyn Unit>> {
    let mut units = HashMap::new();
    let sections = ini!("Units.ini");
    assert!(!sections.is_empty());
    let mut counter : usize = 0;
    for (sec, prop) in sections.iter() {
        println!("Section: {:?}", sec);
        let mut name = "";
        let mut description = "";
        let mut spec = "";
        let mut magic_type = "";
        let mut nature = "";
        let mut cost = None;
        let mut surrender = None;
        let mut start_xp = Some(0);
        let mut hp = Some(0);

        let mut damage_hand = Some(0);
        let mut damage_ranged = Some(0);
        let mut damage_magic = Some(0);
        let mut vamp = Some(0);
        let mut regen = Some(0);

        let mut bonus_name: Option<&str> = None;

        let mut error_collector: Vec<String> = Vec::new();
        for (k, value) in prop.iter() {
            let v = &**value.as_ref().unwrap();
            match &**k {
                "Name" => { name = v; },
                "Descript" => { description = v; },
                "Nature" => { nature = v; },
                "Cost" => {
                    cost = collect_errors(v.parse::<u64>(), &mut error_collector,
                                          "Value of field cost omitted as non-u64");
                },
                "Surrender" => {
                    surrender = collect_errors(v.parse::<u64>(), &mut error_collector,
                                               "Value of field surrender omitted as non-u64");
                },
                "StartExperience" => {
                    start_xp = collect_errors(v.parse::<u64>(), &mut error_collector,
                                              "Value of field start_xp omitted as non-u64");
                },
                "Hits" => {
                    hp = collect_errors(v.parse::<u64>(), &mut error_collector,
                                              "Value of field hp omitted as non-u64");
                },
                "AttackBlow" => {
                    damage_hand = collect_errors(v.parse::<u64>(), &mut error_collector,
                                               "Value of field damage_hand omitted as non-u64");
                }
                "AttackShot" => {
                    damage_ranged = collect_errors(v.parse::<u64>(), &mut error_collector,
                                                   "Value of field damage_ranged omitted as non-u64");
                }
                "Magic" => {
                    magic_type = v;
                }
                "Vampirism" => {
                    vamp = collect_errors(v.parse::<i16>(), &mut error_collector,
                                          "Value of field vamp omitted as non-i16");
                },
                "Regen" => {
                    regen = collect_errors(v.parse::<i16>(), &mut error_collector,
                                          "Value of field vamp omitted as non-i16");
                }
                "Specialization" => { spec = v; }
                "Bonus" => { bonus_name = Some(v); }
                _ => {}
            }
        }
        assert!(!error_collector.is_empty(), "{}", error_collector.join("\n"));
        let bonus = match_bonus(bonus_name);
        let hp = hp.unwrap();
        let data: UnitData = UnitData {
            stats: UnitStats {
                hp,
                max_hp: hp,
                damage: Power {
                    magic: damage_hand.unwrap(),
                    ranged: damage_ranged.unwrap(),
                    hand: damage_magic.unwrap()
                },
                defence: Defence::empty(),
                moves: 0,
                max_moves: 0,
                speed: 0,
                vamp: Percent::new(vamp.unwrap()),
                regen: Percent::new(regen.unwrap())
            },
            info: UnitInfo {
                name: name.to_string(),
                cost: cost.unwrap(),
                unit_type: match nature {
                    "Alive" => UnitType::Alive,
                    "Rogue" => UnitType::Rogue,
                    "Undead" => UnitType::Undead,
                    _ => UnitType::Unidentified,
                },
                magic_type: match magic_type {
                    "LifeMagic" => MagicType::Life,
                    "ElementalMagic" => MagicType::Elemental,
                    "DeathMagic" => MagicType::Death,
                    _ => MagicType::Death
                },
                surrender,

            },
            inventory: UnitInventory::empty(),
            bonus,
            effects: vec![]
        };
        let unit: Box<dyn Unit> = match spec {
                "Hand" => Box::new(Hand::new(data)) as Box<dyn Unit>,
                "Ranged" => Box::new(Ranged::new(data)),
                "HealMage" => Box::new(HealMage::new(data)),
                _ => panic!("Unknown unit type")
            };
        units.insert(counter,
            unit
        );
        counter += 1;
    }
    units
}
