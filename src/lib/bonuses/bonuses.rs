use crate::lib::bonuses::bonus::Bonus;
use crate::lib::effects::effects::MoreMoves;
use crate::lib::units::unit::Power;
use crate::Unit;


#[derive(Copy, Clone, Debug)]
pub struct DefencePiercing {}

impl Bonus for DefencePiercing
{
    fn on_attacking(&self, damage: Power, receiver: &mut dyn Unit, sender: &mut dyn Unit) -> Power
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

#[derive(Copy, Clone, Debug)]
pub struct Dodging {}
impl Bonus for Dodging
{
    fn on_attacked(&self, damage: Power, receiver: &mut dyn Unit, sender: &mut dyn Unit) -> Power
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

#[derive(Copy, Clone, Debug)]
pub struct FastGoing {}
impl Bonus for FastGoing
{
    fn on_battle_start(&self, unit: &mut dyn Unit) -> bool
    {
        unit.add_effect(Box::new(MoreMoves::default()));
        true
    }
}

#[derive(Copy, Clone, Debug)]
pub struct NoBonus {}
impl Bonus for NoBonus {}