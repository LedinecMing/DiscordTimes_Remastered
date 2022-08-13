mod lib;

use
{
    std::
    {
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

type MutRc<T> = Rc<RefCell<T>>;
fn main() {
    println!("Приветц!");
    let troops: Vec<Option<MutRc<Troop>>> =
        vec![
            Troop {
                is_main: true,
                custom_name: Some("Chel".to_string()),
                unit: Box::new(Ranged::Sniper()),
                ..Troop::empty()
            }.into(),
            Troop {
                unit: Box::new(Ranged::Hunter()),
                ..Troop::empty()
            }.into(),
            None];
    let mut army = Army { troops, stats: ArmyStats {
            gold: 0,
            mana: 0,
            army_name: "Армия героя".into()
        },
        inventory: vec![]
        };
    let troops1 : Vec<Option<MutRc<Troop>>> =
        vec![
            Troop {
                unit: Box::new(Hand::Recruit()),
                ..Troop::empty()
            }.into()
        ];
    let mut army1 = Army { troops: troops1, stats: ArmyStats {
            gold: 0,
            mana: 0,
            army_name: "".into()
        },
        inventory: vec![]
    };
    {
        let mut troop0 = army.troops[0].as_ref().unwrap().borrow_mut();
        let mut troop1 = army1.troops[0].as_ref().unwrap().borrow_mut();
        troop1.unit.add_item(Item::CoolSword());
        troop1.unit.attack(&mut *troop0.unit, &mut BattleField { troops: [None] });
        troop0.unit.attack(&mut *troop1.unit, &mut BattleField { troops: [None] });
    }
    dbg!(army.troops[0].as_ref().unwrap().borrow_mut().unit.get_data().stats.hp);
}
