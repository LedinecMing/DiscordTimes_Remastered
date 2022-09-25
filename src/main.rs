#![feature(slice_flatten)]
mod lib;


use {
    std:: {
        collections::HashMap,
        num::ParseIntError,
        fmt::Write,
        io:: {
            stdout,
            Stdout,
    }   },
    lib:: {
        units::units::*,
        units::unit::{Unit},
        battle::army::{Army, ArmyStats},
        battle::troop::Troop,
        time::time::Time,
        console::*,
        mutrc::MutRc,
        map::{
            map::{GameMap, MAP_SIZE},
            tile::Tile,
            object::MapObject
}   }   };


fn register_units() -> HashMap<String, Box<dyn Unit>>) {
    let units: [Box<dyn Unit>;8] = [
        Box::new(Hand::Knight()),
        Box::new(Hand::Recruit()),
        Box::new(Ranged::Sniper()),
        Box::new(Ranged::Hunter()),
        Box::new(Ranged::Pathfinder()),
        Box::new(HealMage::Maidservant()),
        Box::new(HealMage::Nun()),
        Box::new(DisablerMage::Archimage()),
    ];
    units.into_iter().for_each(|unit| {
        hashmap.insert(unit.get_info().name.clone(), unit;
    })
}

fn add_character(your_io: &mut Stdout, army: &mut Army, characters: &HashMap<String, Box<dyn Unit>>) {
    println!("Хочешь добавить больше юнитов? Легко, напиши его название, а если захочешь остановиться пиши СТОП");
    characters.keys().for_each(|key| print!("{} ", key));
    println!();
    loop {
        let ch_string = &*input(your_io, "Хочешь себе юнита?(а не хочешь - пиши СТОП) введи один из списка,");
        if ch_string == "СТОП" {
            println!("Остановка");
            break
        }
        match characters.get(ch_string) {
            Some(character) => match army.add_troop(Troop { unit: character.clone(), ..Troop::empty() }) {
                Ok(_) => println!("Добавлен персонаж {} в вашу армию!", ch_string),
                Err(_) => {
                    println!("Достигнут предел по количеству юнитов в отряде");
                    break
            }   }
            None => println!("Это какая-то несуразица, попробуйте ещё разок!")
}   }   }

fn main() {
    let mut gamemap = GameMap {
        time: Time {minutes: 0},
        tilemap: [[Tile::new(1, '#'); MAP_SIZE]; MAP_SIZE],
        decomap: [[None; MAP_SIZE]; MAP_SIZE],
        armys: vec![]
    };
    let mut mio = stdout();
    let army_name = input(&mut mio, "Название вашей армии: ");
    let mut characters: HashMap<String, Box<dyn Unit>> = register_units(&mut characters);

    let character: Box<dyn Unit>;
    {
        let mut playable_characters: HashMap<String, Box<dyn Unit>> = HashMap::new();
        playable_characters.insert("Рыцарь".into(), Box::new(Hand::Knight()));
        playable_characters.insert("Егерь".into(), Box::new(Ranged::Pathfinder()));
        playable_characters.insert("Архимаг".into(), Box::new(DisablerMage::Archimage()));
        let mut ch_string: &str = &*input(&mut mio, "Тип персонажа(Рыцарь, Егерь, Архимаг): ");

        match playable_characters.contains_key(ch_string) {
            true => character = playable_characters.get(ch_string).unwrap().clone(),
            _ => {
                character = Box::new(Hand::Knight());
                println!("Игрок ошибся: {:?}", ch_string);
    }   }   }
    let ch_name = input(&mut mio, "Имя персонажа: ");

    let mut army = Army::new(
        vec![Troop {
            is_free: true,
            is_main: true,
            custom_name: Some(ch_name),
            unit: character,
            ..Troop::empty()
        }.into()],
        ArmyStats { gold: 0, mana: 0, army_name },
        vec![],
        [0, 0]
    );
    gamemap.armys.push(army);
    println!("Вы можете написать Армия => [Статистика, Бойцы, Добавить персонажа, Статистика персонажа, Атаковать, Справка], Справка, СТОП");
    loop {
        let user_choice = input(&mut mio, "Что делать? ");
        match &*user_choice {
            "Армия" => {
                loop {
                    let army = &mut gamemap.armys[0];
                    match &*input(&mut mio, "Что желаете делать?") {
                        "Статистика" => {
                            println!("Ваша армия \"{}\"| Золото⛀⛁⛃⛂: {}⛃ | Мана✧: {}✧ |", army.stats.army_name, army.stats.gold, army.stats.mana);
                        }
                        "Бойцы" => {
                            army.troops.iter().for_each(
                                |troop| {
                                    print!(" {} |",
                                           match troop.borrow().as_ref() {
                                               Some(troop) => troop.unit.get_info().name.clone(),
                                               None => "Пусто".into()
                                           })
                                });
                            println!();
                        }
                        "Добавить персонажа" => {
                            add_character(&mut mio, army, &characters)
                        }
                        "Статистика персонажа" => {
                            match input(&mut mio, "Введите номер ячейки вашего персонажа:").parse::<usize>() {
                                Ok(value) => {
                                    match army.get_troop(value - 1) {
                                        Some(troop) => {
                                            let borrowed = troop.borrow();
                                            let troop = borrowed.as_ref().unwrap();
                                            println!("{}", troop);
                                        },
                                        _ => break
                                    }   }
                                Err(_) => println!("Милорд, это не число!")
                        }   }
                        "Атаковать" => {
                            let inputed = &*input(&mut mio, "Напишите номер ячейки персонажа, который ударит и номер персонажа, которого будут атаковать через пробел:");
                            let probably_values : Vec<Result<usize, ParseIntError>> = inputed.split(" ").take(2).map(|el| el.parse::<usize>()).collect();
                            match probably_values.iter().all(|el| el.is_ok()) {
                                true => {
                                    let numbers: Vec<usize> = probably_values.iter().map(|el| *el.as_ref().unwrap() - 1).collect();
                                    match numbers.iter().map(|&el| army.troops.get(el)).all(|el| el.is_some()) {
                                        true => {
                                            if numbers[0] == numbers[1] {
                                                println!("Суицид не выход");
                                                break;
                                            }
                                            match (army.get_troop(numbers[0]), army.get_troop(numbers[1])) {
                                                (Some(troop0), Some(troop1)) => {
                                                    troop0.borrow_mut().as_mut().unwrap().unit.attack(
                                                        &mut *troop1.borrow_mut().as_mut().unwrap().unit);
                                                },
                                                _ => println!("Вы ошиблись, попробуйте снова!")
                                        }   },
                                        false => println!("Ошибка!"),
                                }   },
                                false => print!("Это не целые числа!")
                        }   }
                        "НАЗАД" => {
                            break
                        }
                        "Справка" => {
                            println!("Вы можете написать Статистика, Бойцы, Добавить персонажа, Статистика персонажа, Атаковать, СТОП");
                        }
                        T => println!("Ничего не понял в этих ваших \"{}\"!", T)
            }   }   }
            "Карта" => {
                match &*input(&mut mio, "") {
                    "Показать" => {
                        let tilemap_ref = gamemap.tilemap.flatten();
                        let decomap_ref = gamemap.decomap.flatten();
                        let mut out = String::new();
                        for i in 0..MAP_SIZE.pow(2) {
                            if i % MAP_SIZE == 0 { write!(&mut out, "\n").unwrap(); }
                            if let Some(deco) = decomap_ref[i] {
                                write!(&mut out, "{}", deco.get_symbol()).unwrap();
                                continue;
                            };
                            write!(&mut out, "{}", tilemap_ref[i].get_symbol()).unwrap();
                        }
                        println!("{}", out);
                    }
                    _ => {}
            }   }
            "Справка" => println!("Вы можете написать Армия => [Статистика, Бойцы, Добавить персонажа, Статистика персонажа, Атаковать, Справка], Справка, СТОП"),
            "СТОП" => break,
            _ => println!("Ничего не понял!")
        }   }
    pause();
}
