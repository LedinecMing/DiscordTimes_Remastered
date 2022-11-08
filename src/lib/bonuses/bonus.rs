use {
    std::fmt::Debug,
    dyn_clone::DynClone,
    crate::lib::{
        time::time::Time,
        units::unit::{
            Unit1,
            Power
}   }   };


dyn_clone::clone_trait_object!(Bonus);
pub trait Bonus : DynClone + Debug {
    fn on_attacked(&self, damage: Power, receiver: &mut Unit1, sender: &mut Unit1) -> Power { damage }
    fn on_attacking(&self, damage: Power, receiver: &mut Unit1, sender: &mut Unit1) -> Power { damage }
    fn on_kill(&self,  receiver: &mut Unit1, sender: &mut Unit1) -> bool { false }
    fn on_tick(&self, unit: &mut Unit1) -> bool { false }
    fn on_hour(&self, unit: &mut Unit1, time: Time) -> bool { false }
    fn on_battle_start(&self, unit: &mut Unit1) -> bool { false }
    fn on_move_skip(&self, unit: &mut Unit1) -> bool { false }
}