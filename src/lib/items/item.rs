use crate::Unit;

pub trait Item
{
    fn add_effects(&self, unit: &mut dyn Unit);
}

pub struct ItemInfo
{

}