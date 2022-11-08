use {
    crate::lib::{
        effects::effect::{Effect, EffectInfo, EffectKind},
        units::{
            unit::UnitStats,
            unit::Unit1
        },
    },
    math_thingies::{nat, Percent, add_if_nat}
};


#[derive(Copy, Clone, Debug)]
pub struct MoreHealth {
    pub info: EffectInfo
}
impl Default for MoreHealth {
    fn default() -> Self {
        Self {
            info: EffectInfo { lifetime: -1 }
}   }   }
impl Effect for MoreHealth {
    fn update_stats(&self, mut unitstats: UnitStats) -> UnitStats {
        unitstats.hp += 10;
        unitstats.max_hp += 10;
        unitstats
}   }

#[derive(Copy, Clone, Debug)]
pub struct JustAdd<const F: fn(&mut Unit1) -> UnitStats> {
    pub info: EffectInfo
}
impl<const F: fn(&mut Unit1) -> UnitStats> Effect for JustAdd<F> {
    fn update_stats(&self, unit: &mut Unit1) {
        unit.stats+=F(unit);
    }
}

#[derive(Copy, Clone, Debug)]
pub struct MoreHandAttack {
    pub info: EffectInfo
}
impl Default for MoreHandAttack {
    fn default() -> Self {
        Self {
            info: EffectInfo { lifetime: -1 }
}   }   }
impl Effect for MoreHandAttack {
    fn update_stats(&self, mut unitstats: UnitStats) -> UnitStats {
        add_if_nat(&mut unitstats.damage.hand, 5u64);
        unitstats
    }
    fn tick(&mut self, unit: &mut Unit1) -> bool { false }
}

#[derive(Copy, Clone, Debug)]
pub struct MoreMoves {
    pub info: EffectInfo
}
impl Default for MoreMoves {
    fn default() -> Self {
        Self {
            info: EffectInfo {lifetime: 1}
}   }   }
impl Effect for MoreMoves
{
    fn update_stats(&self, mut unitstats: UnitStats) -> UnitStats {
        unitstats.max_moves+=1;
        unitstats
    }
    fn on_tick(&mut self) -> bool {
        self.info.lifetime -= 1;
        true
    }
    fn is_dead(&self) -> bool {
        self.info.lifetime < 1
}   }

#[derive(Copy, Clone, Debug)]
pub struct HealMagic {
    pub info: EffectInfo,
    pub magic_power: u64
}
impl Default for HealMagic {
    fn default() -> Self {
        Self {
            info: EffectInfo {lifetime: 1},
            magic_power: 15
}   }   }
impl Effect for HealMagic {
    fn update_stats(&self, mut unitstats: UnitStats) -> UnitStats {
        add_if_nat(&mut unitstats.defence.ranged_units, self.magic_power / 5);
        add_if_nat(&mut unitstats.defence.hand_units, self.magic_power / 5);
        add_if_nat(&mut unitstats.damage.ranged, self.magic_power / 10);
        add_if_nat(&mut unitstats.damage.hand, self.magic_power / 10);
        unitstats
    }
    fn on_tick(&mut self) -> bool {
        self.info.lifetime -= 1;
        true
    }
    fn kill(&mut self, unit: &mut Unit1) {
        let mut unitstats = unit.stats;
        add_if_nat(&mut unitstats.defence.ranged_units, -self.magic_power / 5);
        add_if_nat(&mut unitstats.defence.hand_units, -self.magic_power / 5);
        add_if_nat(&mut unitstats.damage.ranged, -self.magic_power / 10);
        add_if_nat(&mut unitstats.damage.hand, -self.magic_power / 10);
    } 
    fn is_dead(&self) -> bool { self.info.lifetime < 1 }
    fn get_kind(&self) -> EffectKind { EffectKind::MageSupport }
}

#[derive(Copy, Clone, Debug)]
pub struct DisableMagic {
    pub info: EffectInfo,
    pub magic_power: u64
}
impl Default for DisableMagic {
    fn default() -> Self {
        Self {
            info: EffectInfo {lifetime: 1},
            magic_power: 15
}   }   }
impl Effect for DisableMagic {
    fn update_stats(&self, mut unitstats: UnitStats) -> UnitStats {
        if self.magic_power < 20 {
            return unitstats
        }
        unitstats.moves = unitstats.moves.saturating_sub(1 + self.magic_power / 50);
        unitstats
    }
    fn on_tick(&mut self) -> bool {
        self.info.lifetime -= 1;
        true
    }
    fn on_battle_end(&mut self) -> bool {
        self.info.lifetime = 0;
        true
    }
    fn is_dead(&self) -> bool { self.info.lifetime < 1 }
    fn get_kind(&self) -> EffectKind { EffectKind::MageCurse }
}

#[derive(Copy, Clone, Debug)]
pub struct ElementalSupport {
    pub info: EffectInfo,
    pub magic_power: u64
}
impl Default for ElementalSupport {
    fn default() -> Self {
        Self {
            info: EffectInfo {lifetime: 1},
            magic_power: 15
}   }   }
impl Effect for ElementalSupport {
    fn update_stats(&self, mut unitstats: UnitStats) -> UnitStats {
        if self.magic_power < 20 {
            return unitstats
        }
        unitstats.moves = unitstats.moves.saturating_add(1 + self.magic_power / 50);
        unitstats
    }
    fn on_tick(&mut self) -> bool {
        self.info.lifetime -= 1;
        true
    }
    fn on_battle_end(&mut self) -> bool {
        self.info.lifetime = 0;
        true
    }
    fn is_dead(&self) -> bool { self.info.lifetime < 1 }
    fn get_kind(&self) -> EffectKind { EffectKind::MageSupport }
}

#[derive(Copy, Clone, Debug)]
pub struct AttackMagic {
    pub info: EffectInfo,
    pub magic_power: u64
}
impl Default for AttackMagic {
    fn default() -> Self {
        Self {
            info: EffectInfo {lifetime: 1},
            magic_power: 15
}   }   }
impl Effect for AttackMagic {
    fn update_stats(&self, mut unitstats: UnitStats) -> UnitStats {
        unitstats.damage.hand = unitstats.damage.hand.saturating_sub(1 + self.magic_power / 10);
        unitstats.damage.ranged = unitstats.damage.ranged.saturating_sub(1 + self.magic_power / 10);
        unitstats.defence.hand_units = unitstats.defence.hand_units.saturating_sub(1 + self.magic_power / 5);
        unitstats.defence.ranged_units = unitstats.defence.ranged_units.saturating_sub(1 + self.magic_power / 5);
        unitstats
    }
    fn on_tick(&mut self) -> bool {
        self.info.lifetime -= 1;
        true
    }
    fn on_battle_end(&mut self) -> bool {
        self.info.lifetime = 0;
        true
    }
    fn is_dead(&self) -> bool { self.info.lifetime < 1 }
    fn get_kind(&self) -> EffectKind { EffectKind::MageCurse }
}

const POISON_PERCENT: Percent = Percent::const_new(15);
#[derive(Copy, Clone, Debug)]
pub struct Poison {
    pub info: EffectInfo,
}
impl Effect for Poison {
    fn update_stats(&self, unitstats: UnitStats) -> UnitStats {
        let mut unitstats = unitstats;
        unitstats.hp -= POISON_PERCENT.calc(unitstats.hp);
        unitstats
    }
    fn on_battle_end(&mut self) -> bool {
        self.info.lifetime = 0;
        true
    }
    fn is_dead(&self) -> bool { self.info.lifetime < 1 }
    fn get_kind(&self) -> EffectKind { EffectKind::Poison }
}
impl Default for Poison {
    fn default() -> Self {
        Self {
            info: EffectInfo { lifetime: -1 }
}   }   }

const FIRE_PERCENT: Percent = Percent::const_new(10);
const FIRE_SLOWNESS_PERCENT: Percent = Percent::const_new(10);
#[derive(Copy, Clone, Debug)]
pub struct Fire {
    pub info: EffectInfo,
}
impl Effect for Fire {
    fn update_stats(&self, unitstats: UnitStats) -> UnitStats {
        let mut unitstats = unitstats;
        unitstats.hp -= FIRE_PERCENT.calc(unitstats.hp);
        unitstats.speed -= FIRE_SLOWNESS_PERCENT.calc(unitstats.speed);
        unitstats
    }
    fn on_tick(&mut self) -> bool {
        self.info.lifetime -= 1;
        true
    }
    fn on_battle_end(&mut self) -> bool {
        self.info.lifetime = 0;
        true
    }
    fn is_dead(&self) -> bool { self.info.lifetime < 1 }
    fn get_kind(&self) -> EffectKind { EffectKind::Fire }
}
impl Default for Fire {
    fn default() -> Self {
        Self {
            info: EffectInfo { lifetime: 5 }
}   }   }


#[derive(Copy, Clone, Debug)]
pub struct ItemEffect {
    pub info: EffectInfo,
    pub additions: UnitStats
}
impl Effect for ItemEffect {
    fn update_stats(&self, unitstats: UnitStats) -> UnitStats {
        unitstats + self.additions
    }
    fn get_kind(&self) -> EffectKind { EffectKind::Item }
}


#[derive(Copy, Clone, Debug)]
pub struct ToEndEffect {
    pub info: EffectInfo,
    pub additions: UnitStats
}
impl Effect for ToEndEffect {
    fn update_stats(&self, unitstats: UnitStats) -> UnitStats {
        unitstats + self.additions
    }
    fn on_battle_end(&mut self) -> bool {
        self.info.lifetime = 0;
        true
    }
    fn get_kind(&self) -> EffectKind { EffectKind::Bonus }
}