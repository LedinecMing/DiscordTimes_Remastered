mod lib;
use lib::units::unit::Ranged;
use lib::units::unit::Unit;
use lib::battle::battlefield::BattleField;
use lib::bonuses::bonuses::Dodging;

fn main() {
    println!("Приветц!");
    let mut x1 = Ranged::new();
    let mut x2 = Ranged::new();
    let mut troops: [Option<Ranged>; 2];
    // x[0].attack(&mut *x[1], &mut BattleField{ troops: [Some(Box::new(Ranged::new()))] });
    println!("Хиты {:?}", x1.get_effected_stats().hp);

    x1.bonus = Box::new(Dodging {});
    x2.attack(&mut x1, &mut BattleField{ troops: [Some(Box::new(Ranged::new()))] });
    println!("Хиты {:?}", x1.get_effected_stats().hp)
}
