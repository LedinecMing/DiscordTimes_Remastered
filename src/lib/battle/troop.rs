use crate::lib::battle::army::Army;
use crate::Unit;
use crate::lib::units::units::Hand;

#[derive(Clone, Debug)]
pub struct Troop
{
    pub was_payed: bool,
    pub is_dead: bool,
    pub is_free: bool,
    pub is_main: bool,
    pub custom_name: Option<String>,
    pub unit: Box<dyn Unit>
}

impl Troop
{
    pub fn on_pay(&self, army: &mut Army) -> i32
    {
        if self.is_free
        {
            return 0;
        }
        self.unit.get_info().cost
    }
    pub fn on_hour(&self, army: &mut Army) -> bool
    {
        true
    }
    pub fn empty() -> Self
    {
        Self
        {
            was_payed: true,
            is_dead: false,
            is_free: false,
            is_main: false,
            custom_name: None,
            unit: Box::new(Hand::Recruit())
        }
    }
}