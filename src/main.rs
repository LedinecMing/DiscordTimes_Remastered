extern crate core;

mod lib;


use {
    std:: {
        collections::HashMap,
        cell::RefCell,
        rc::Rc,
        num::ParseIntError,
        io:: {
            prelude::*,
            stdin, stdout,
            Stdout, Stdin
        }
    },
    lib:: {
        units::units::*,
        units::unit::{Unit, UnitData, UnitStats},
        battle::battlefield::BattleField,
        battle::army::{Army, ArmyStats},
        battle::troop::Troop,
        items::item::Item,
        time::time::Time,
        map::{
            map::{GameMap, MAP_SIZE},
            tile::Tile
        }
    }
};


fn pause() {
    let mut stdin = stdin();
    let mut stdout = stdout();

    // We want the cursor to stay at the end of the line, so we print without a newline and flush manually.
    write!(stdout, "Press any key to continue...").unwrap();
    stdout.flush().unwrap();

    // Read a single byte and discard
    let _ = stdin.read(&mut [0u8]).unwrap();
}

fn input(your_io: &mut Stdout, input_string: &str) -> String {
    const START: usize = 0;
    your_io.write(input_string.as_ref()).unwrap();
    your_io.flush().unwrap();
    let mut input = "".to_string();
    stdin().read_line(&mut input).unwrap();
    let end = input.chars().count() - 1;
    input.chars().take(end).skip(START).collect()
}

fn insert_unit(hashmap: &mut HashMap<String, Box<dyn Unit>>, unit: Box<dyn Unit>)
{
    hashmap.insert(unit.get_data().info.name.clone(), unit);
}

fn add_character(your_io: &mut Stdout, army: &mut Army, characters: &HashMap<String, Box<dyn Unit>>)
{
    println!("Хочешь добавить больше юнитов? Легко, напиши его название, а если захочешь остановиться пиши СТОП");
    characters.keys().for_each(|key| print!("{} ", key));
    println!();
    loop {
        let ch_string = &*input(your_io, "Хочешь себе юнита?(а не хочешь - пиши СТОП) введи один из списка,");
        if ch_string == "СТОП" {
            println!("Остановка");
            break
        }
        if characters.contains_key(ch_string) {
            army.add_troop(Troop {
                unit: characters.get(ch_string).unwrap().clone(),
                ..Troop::empty()
            });
            println!("Добавлен персонаж {} в вашу армию!", ch_string);
        }
        else {
            println!("Это какая-то несуразица, попробуйте ещё разок!");
        }
    }
}

type MutRc<T> = Rc<RefCell<T>>;
fn main() {
    let gamemap = GameMap {
        time: Time{minutes: 0},
        tilemap: [[Tile {walkspeed: 1}; MAP_SIZE]; MAP_SIZE],
        objectmap: [[None; MAP_SIZE]; MAP_SIZE],
        decomap: [[vec![]; MAP_SIZE]; MAP_SIZE],
        armys: vec![]
    };
    let mut mio = stdout();
    let army_name = input(&mut mio, "Название вашей армии: ");
    let mut characters: HashMap<String, Box<dyn Unit>> = HashMap::new();
    insert_unit(&mut characters, Box::new(Hand::Knight()));
    insert_unit(&mut characters, Box::new(Hand::Recruit()));
    insert_unit(&mut characters, Box::new(Ranged::Sniper()));
    insert_unit(&mut characters, Box::new(Ranged::Hunter()));
    insert_unit(&mut characters, Box::new(Ranged::Pathfinder()));
    insert_unit(&mut characters, Box::new(HealMage::Maidservant()));
    insert_unit(&mut characters, Box::new(HealMage::Nun()));
    insert_unit(&mut characters, Box::new(DisablerMage::Archimage()));

    let character: Box<dyn Unit>;
    {
        let mut playable_characters: HashMap<String, Box<dyn Unit>> = HashMap::new();
        playable_characters.insert("Рыцарь".into(), Box::new(Hand::Knight()));
        playable_characters.insert("Егерь".into(), Box::new(Ranged::Pathfinder()));
        playable_characters.insert("Архимаг".into(), Box::new(DisablerMage::Archimage()));
        let mut ch_string: &str = &*input(&mut mio, "Тип персонажа(Рыцарь, Егерь, Архимаг): ");

        if playable_characters.contains_key(ch_string)
        {
            character = playable_characters.get(ch_string).unwrap().clone();
        } else {
            character = Box::new(Hand::Knight());
            println!("Игрок ошибся: {:?}", ch_string);
        }
    }
    let ch_name = input(&mut mio, "Имя персонажа: ");

    let mut army = Army::new(
        vec![Troop {
            is_free: true,
            is_main: true,
            custom_name: Some(ch_name),
            unit: character,
            ..Troop::empty()
        }.into()],
        ArmyStats {
            gold: 0,
            mana: 0,
            army_name
        },
        vec![]
    );
    println!("Вы можете написать Армия => [Статистика, Бойцы, Добавить персонажа, Статистика персонажа, Атаковать, Справка], Справка, СТОП");
    loop {
        let user_choice = input(&mut mio, "Что делать? ");
        match &*user_choice {
            "Армия" => {
                loop {
                    match &*input(&mut mio, "Что желаете делать?") {
                        "Статистика" => {
                            println!("Ваша армия \"{}\"| Золото⛀⛁⛃⛂: {}⛃ | Мана✧: {}✧ |", army.stats.army_name, army.stats.gold, army.stats.mana);
                        }
                        "Бойцы" => {
                            army.troops.iter().for_each(
                                |troop| {
                                    print!(" {} |",
                                           match troop.borrow().as_ref()
                                           {
                                               Some(troop) => troop.unit.get_data().info.name.clone(),
                                               None => "Пусто".into()
                                           })
                                });
                            println!();
                        }
                        "Добавить персонажа" => {
                            add_character(&mut mio, &mut army, &characters)
                        }
                        "Статистика персонажа" => {
                            match input(&mut mio, "Введите номер ячейки вашего персонажа:").parse::<usize>() {
                                Ok(value) => {
                                    match army.get_troop(value - 1) {
                                        Some(ref_troop) => {
                                            let my_borrowed = ref_troop.borrow();
                                            let troop = my_borrowed.as_ref().unwrap();
                                            let custom_name = match &troop.custom_name {
                                                Some(name) => name.clone(),
                                                None => "".into()
                                            };
                                            let unitdata: &UnitData = troop.unit.get_data();
                                            let unitstats: &UnitStats = &unitdata.stats;
                                            let unit_name = unitdata.info.name.clone();
                                            let name = format!("{}-{}", custom_name, unit_name);
                                            println!("| {name} |\n| {hp}/{maxhp}ХП ({is_dead}) |\n| {hand_attack} ближнего урона; {ranged_attack} дальнего урона; {magic_attack} магии |\n| {hand_def} ближней защиты.-{ranged_def} дальней защиты. |\n| Защита от магии: {magic_def_percent}%|\n|Регенерация {regen_percent}% |\n| Вампиризм {vamp_percent}% |\n| {speed} инициативы |\n| {moves}/{max_moves} ходов |", name=name,
                                                     hp = unitstats.hp,
                                                     maxhp = unitstats.max_hp,
                                                     is_dead = match troop.unit.is_dead() {
                                                         true => "мёртв.",
                                                         false => "живой"
                                                     },
                                                     hand_attack = unitstats.damage.hand,
                                                     ranged_attack = unitstats.damage.ranged,
                                                     magic_attack = unitstats.damage.magic,
                                                     hand_def = unitstats.defence.hand_units,
                                                     ranged_def = unitstats.defence.ranged_units,
                                                     magic_def_percent = unitstats.defence.magic_percent,
                                                     regen_percent = unitstats.regen,
                                                     vamp_percent = unitstats.vamp,
                                                     speed = unitstats.speed,
                                                     moves = unitstats.moves,
                                                     max_moves = unitstats.max_moves
                                            );
                                        }
                                        None => {}
                                    }
                                }
                                Err(error) => println!("Милорд, это не число!")
                            }
                        }
                        "Атаковать" => {
                            let inputed = &*input(&mut mio, "Напишите номер ячейки персонажа, который и ударит и номер персонажа, которого будут атаковать через пробел:");
                            let probably_values : Vec<Result<usize, ParseIntError>> = inputed.split(" ").take(2).collect::<Vec<&str>>().iter().map(|&el| el.parse::<usize>()).collect();
                            match probably_values.iter().all(|el| el.is_ok()) {
                                true => {
                                    let numbers: Vec<usize> = probably_values.iter().map(|el| *el.as_ref().unwrap() - 1).collect();
                                    match numbers.iter().map(|&el| army.troops.get(el)).all(|el| el.is_some()) {
                                        true => {
                                            if numbers[0] == numbers[1] {
                                                println!("Нельзя ударить самого себя, глупышка, это статья!");
                                                break
                                            }
                                            let mut troop0 = match army.get_troop(numbers[0]) {
                                                Some(troop) => troop,
                                                None => {
                                                    break
                                                }
                                            };
                                            let mut troop1 = match army.get_troop(numbers[1]) {
                                                Some(troop) => troop,
                                                None => {
                                                    break
                                                }
                                            };
                                            troop0.borrow_mut().as_mut().unwrap().unit.attack(&mut *troop1.borrow_mut().as_mut().unwrap().unit);
                                        }
                                        false => println!("Неправильные номера!")
                                    }
                                }
                                false => {
                                    println!("Это не цифры!");
                                }
                            }
                        }
                        "НАЗАД" => {
                            break
                        }
                        "Справка" => {
                            println!("Вы можете написать Статистика, Бойцы, Добавить персонажа, Статистика персонажа, Атаковать, СТОП");
                        }
                        T => println!("Ничего не понял в этих ваших \"{}\"!", T)
                    }
                }
            }
            "Справка" => {
                println!("Вы можете написать Армия => [Статистика, Бойцы, Добавить персонажа, Статистика персонажа, Атаковать, Справка], Справка, СТОП");
            }
            "СТОП" => {
                break
            }
            _ => println!("Ничего не понял!")
        }
    }
    pause();
}
