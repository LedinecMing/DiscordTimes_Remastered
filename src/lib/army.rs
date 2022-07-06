use crate::Unit as Unit;

pub struct Army
{
    pub troops: [Option<Box<dyn Unit>>;12],

}
