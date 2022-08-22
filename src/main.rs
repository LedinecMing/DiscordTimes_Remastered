mod lib;

use
{
    std::
    {
        collections::HashMap,
        cell::RefCell,
        rc::Rc
    },
    lib::
    {
        units::units::*,
        units::unit::Unit,
        battle::battlefield::BattleField,
        battle::army::{Army, ArmyStats},
        battle::troop::Troop,
        items::item::Item
    }
};
use std::io;
use std::io::prelude::*;
use std::io::Stdout;
use crate::lib::effects::effects::DisableMagic;

fn pause() {
    let mut stdin = io::stdin();
    let mut stdout = io::stdout();

    // We want the cursor to stay at the end of the line, so we print without a newline and flush manually.
    write!(stdout, "Press any key to continue...").unwrap();
    stdout.flush().unwrap();

    // Read a single byte and discard
    let _ = stdin.read(&mut [0u8]).unwrap();
}
fn input(your_io: &mut Stdout, input_string: &str) -> String
{
    const START: usize = 0;
    your_io.write(input_string.as_ref()).unwrap();
    your_io.flush().unwrap();
    let mut input = "".to_string();
    io::stdin().read_line(&mut input).unwrap();
    let end = input.chars().count() - 1;
    input.chars().take(end).skip(START).collect()
}
pub fn insert_unit(hashmap: &mut HashMap<String, Box<dyn Unit>>, unit: Box<dyn Unit>)
{
    hashmap.insert(unit.get_data().info.name.clone(), unit);
}
type MutRc<T> = Rc<RefCell<T>>;
fn main() {
    let mut mio = io::stdout();
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
    let mut army = Army {
        troops: vec![Troop {
            is_free: true,
            is_main: true,
            custom_name: Some(ch_name),
            unit: character,
            ..Troop::empty()
        }.into()],
        stats: ArmyStats {
            gold: 0,
            mana: 0,
            army_name
        },
        inventory: vec![]
    };
    println!("Информация по вашей армии:");
    dbg!(&army);
    println!("Хочешь добавить больше юнитов? Легко, напиши его название, а если захочешь остановиться пиши СТОП");
    characters.keys().for_each(|key| print!("{} ", key));
    println!();
    loop {
        let ch_string = &*input(&mut mio, "Хочешь себе юнита?(а не хочешь - пиши СТОП) введи один из списка,");
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
    dbg!(&army);
    pause();
}