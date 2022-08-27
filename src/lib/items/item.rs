use {
    crate::lib::{
        bonuses::bonus::Bonus,
        effects::{
            effect::Effect
        }
    }
};

#[derive(Clone, Debug)]
pub struct ItemInfo {
    pub name: String,
    pub description: String,
    pub cost: u64,
    pub sells: bool
}
#[derive(Clone, Debug)]
pub struct Item {
    pub bonus: Option<Box<dyn Bonus>>,
    pub effect: Box<dyn Effect>,
    pub info: ItemInfo
}
