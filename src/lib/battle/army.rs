use std::cell::RefCell;
use std::rc::Rc;
use crate::{Item, MutRc};
use super::troop::Troop;

#[derive(Debug)]
pub struct ArmyStats
{
    pub gold: u64,
    pub mana: u64,
    pub army_name: String
}
#[derive(Debug)]
pub struct Army
{
    pub troops: Vec<Option<MutRc<Troop>>>,
    pub stats: ArmyStats,
    pub inventory: Vec<Item>
}
const MAX_TROOPS: usize = 12;
impl Army
{
    pub fn add_troop(&mut self, troop: Troop)
    {
        let index = self.troops.iter().position(|el| el.is_none() );
        match index
        {
            Some(index) => self.troops[index] = troop.into(),
            None => if self.troops.len() + 1 < MAX_TROOPS {
                self.troops.push(troop.into())
            }
        }
    }
    pub fn add_item(&mut self, item: Item)
    {
        self.inventory.push(item)
    }
}
impl From<Army> for MutRc<Army>
{
    fn from(army: Army) -> Self
    {
        Rc::new(RefCell::new(army))
    }
}