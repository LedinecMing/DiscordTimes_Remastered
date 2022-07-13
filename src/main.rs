mod lib;

use dyn_clone::clone_box;
use lib::units::unit::Ranged;
use lib::units::unit::Unit;
use lib::battle::battlefield::BattleField;
use lib::bonuses::bonuses::Dodging;
use crate::lib::bonuses::bonuses::DefencePiercing;

fn main() {
    println!("Приветц!");
    let mut troops: Vec<Box<dyn Unit>> = vec![Box::new(Ranged::Hunter()), Box::new(Ranged::Sniper())];
    let mut troop0 = *clone_box(&troops[0]);
    let mut troop1 = *clone_box(&troops[1]);
    println!("Хиты {:?}", troop0.get_effected_stats().hp);
    troop1.attack(&mut *troop0, &mut BattleField{ troops: [Some(Box::new(Ranged::Sniper()))] });
    println!("Хиты {:?}", troop0.get_effected_stats().hp);
    troops[0] = troop0;
    troops[1] = troop1;
}
