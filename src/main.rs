mod lib;

use dyn_clone::clone_box;
use lib::units::units::*;
use lib::units::unit::Unit;
use lib::battle::battlefield::BattleField;
use lib::bonuses::bonuses::Dodging;
use crate::lib::battle::army::{Army, ArmyStats};
use crate::lib::battle::troop::Troop;
use crate::lib::bonuses::bonuses::DefencePiercing;
use crate::lib::items::item::Item;
use crate::lib::units::unit::Power;
use crate::lib::units::units::Hand;


fn main() {
    println!("Приветц!");
    let troops: Vec<Option<Box<Troop>>> =
        vec![
            Some(Box::new(Troop {
                was_payed: true,
                is_dead: false,
                is_free: true,
                is_main: true,
                custom_name: Some("Chel".to_string()),
                unit: Box::new(Ranged::Sniper())
            })),
            Some(Box::new(Troop {
                was_payed: false,
                is_dead: false,
                is_free: false,
                is_main: false,
                custom_name: None,
                unit: Box::new(Ranged::Hunter())
            })),
            None];

        let mut army = Army { troops, stats: ArmyStats {
        gold: 0,
        mana: 0,
        army_name: "Армия героя".to_string()
    } };
    let troops1 : Vec<Option<Box<Troop>>> =
        vec![
            Some(Box::new(Troop {
                unit: Box::new(Hand::Recruit()),
                ..Troop::empty()
            }))
        ];
    let mut army1 = Army { troops: troops1, stats: ArmyStats {
        gold: 0,
        mana: 0,
        army_name: "".to_string()
    } };
    let mut troop0 = clone_box(&*army.troops[0].as_ref().unwrap());
    let mut unit0 = troop0.unit;
    let mut troop1 = clone_box(&*army1.troops[0].as_ref().unwrap());
    let mut unit1 : Box<dyn Unit> = troop1.unit;
    unit1.add_item(Item::CoolSword());
    println!("Хиты {:?}", unit1.get_effected_stats().hp);
    unit0.attack(&mut *unit1, &mut BattleField{ troops: [Some(Box::new(Ranged::Sniper()))] });
    println!("Хиты {:?}", unit1.get_effected_stats().hp);
    println!("Хиты {:?}", unit0.get_effected_stats().hp);
    unit1.attack(&mut *unit0, &mut BattleField{ troops: [Some(Box::new(Ranged::Sniper()))] });
    println!("Хиты {:?}", unit0.get_effected_stats().hp);
    troop0.unit = unit0;
    troop1.unit = unit1;
    army.troops[0] = Some(*troop0);
    army.troops[1] = Some(*troop1);
}
