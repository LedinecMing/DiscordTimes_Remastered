use super::troop::Troop;

pub struct ArmyStats
{
    pub gold: i32,
    pub mana: i32,
    pub army_name: String
}
pub struct Army
{
    pub troops: Vec<Option<Box<Troop>>>,
    pub stats: ArmyStats
}

pub fn print_army(army: &Army)
{

}