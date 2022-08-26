use
{
    std::
    {
        cell::RefCell,
        rc::Rc
    },
    crate::lib::
    {
        battle::army::Army,
        units::
        {
            unit::Unit,
            units::Hand
        },
    },
    crate::MutRc
};

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
    pub fn on_pay(&self, army: &mut Army) -> u64
    {
        if self.is_free
        {
            return 0;
        }
        self.unit.get_data().info.cost
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

impl From<Troop> for Option<MutRc<Troop>>
{
    fn from(troop: Troop) -> Self
    {
        Some(Rc::new(RefCell::new(troop)))
    }
}
