use crate::Unit;

pub struct BattleField
{
    pub troops: [Option<Box<dyn Unit>>;1]
}
