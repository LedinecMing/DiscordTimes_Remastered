use {
    std::fmt::Debug,
    dyn_clone::DynClone,
    crate::lib::{
        units::unit::{UnitStats, Unit1}
    }
};

#[derive(PartialEq)]
pub enum EffectKind {
    MageCurse,
    MageSupport,
    Bonus,
    Item,
    Poison,
    Fire
}

dyn_clone::clone_trait_object!(Effect);
pub trait Effect : DynClone + Debug {
    fn update_stats(&self, unitstats: UnitStats) -> UnitStats;
    fn on_tick(&mut self) -> bool { false }
    fn on_battle_end(&mut self) -> bool { false }
    fn tick(&mut self, unit: &mut Unit1) -> bool {
        self.on_tick();
        true
    }
    fn kill(&mut self, unit: &mut Unit1) {

    }
    fn is_dead(&self) -> bool { false }
    fn get_kind(&self) -> EffectKind {
        EffectKind::Bonus
}   }


#[derive(PartialEq, Copy, Clone, Debug)]
pub struct EffectInfo {
    pub lifetime: i32
}