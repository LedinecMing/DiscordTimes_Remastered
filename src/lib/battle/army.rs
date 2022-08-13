use crate::{Item, MutRc};
use super::troop::Troop;

pub struct ArmyStats
{
    pub gold: u64,
    pub mana: u64,
    pub army_name: String
}
pub struct Army
{
    pub troops: Vec<Option<MutRc<Troop>>>,
    pub stats: ArmyStats,
    pub inventory: Vec<Item>
}
impl Army
{
    pub fn add_troop(&mut self, troop: Troop)
    {

    }
    pub fn add_item(&mut self, item: Item)
    {
        self.inventory.push(item)
    }
}