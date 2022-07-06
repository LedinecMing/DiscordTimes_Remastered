use crate::lib::bonuses::bonus::Bonus;
use crate::lib::units::unit::Power;
use crate::Unit;


pub struct DefencePiercing {}

impl Bonus for DefencePiercing
{
    fn on_attacking(&self, damage: Power, receiver: &dyn Unit, sender: &dyn Unit) -> Power
    {
        let sender_damage: Power = sender.get_effected_stats().damage;
        println!("Бонус: Атакую, ручной и дальний урон проходит сквозь броню - {:?}", Power
        {
            magic: damage.magic,
            ranged: sender_damage.ranged,
            hand: sender_damage.hand
        });
        Power
        {
            magic: damage.magic,
            ranged: sender_damage.ranged,
            hand: sender_damage.hand
        }
    }
}

pub struct Dodging {}

impl Bonus for Dodging
{
    fn on_attacked(&self, damage: Power, receiver: &dyn Unit, sender: &dyn Unit) -> Power
    {
        println!("Бонус: кто-то атакует, пропускаю 75%");
        Power
        {
            magic: damage.magic / 4 * 3,
            ranged: damage.ranged / 4 * 3,
            hand: damage.hand / 4 * 3
        }
    }
}