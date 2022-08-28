use {
    crate::lib::{
        effects::effect::{Effect, EffectInfo},
        units::{
            unit::UnitStats,
            unit::Unit
        }
    }
};

#[derive(Copy, Clone, Debug)]
pub struct MoreHealth {
    pub info: EffectInfo
}
impl Default for MoreHealth {
    fn default() -> Self {
        Self {
            info: EffectInfo { lifetime: -1 }
        }
    }
}
impl Effect for MoreHealth {
    fn update_stats(&self, mut unitstats: UnitStats) -> UnitStats {
        unitstats.hp += 10;
        unitstats.max_hp += 10;
        unitstats
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
        }
    }
}
impl Effect for MoreHandAttack {
    fn update_stats(&self, mut unitstats: UnitStats) -> UnitStats {
        unitstats.damage.hand += 5;
        unitstats
    }
    fn tick(&mut self, unit: &mut dyn Unit) -> bool { false }
}

#[derive(Copy, Clone, Debug)]
pub struct MoreMoves {
    pub info: EffectInfo
}
impl Default for MoreMoves {
    fn default() -> Self {
        Self {
            info: EffectInfo {lifetime: 1}
        }
    }
}
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
    }
}

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
        }
    }
}
impl Effect for HealMagic {
    fn update_stats(&self, mut unitstats: UnitStats) -> UnitStats {
        unitstats.defence.ranged_units += self.magic_power / 5;
        unitstats.defence.hand_units += self.magic_power / 5;
        unitstats.damage.ranged += self.magic_power / 10;
        unitstats.damage.hand += self.magic_power / 10;
        unitstats
    }
    fn on_tick(&mut self) -> bool {
        self.info.lifetime -= 1;
        true
    }
    fn is_dead(&self) -> bool {
        self.info.lifetime < 1
    }
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
        }
    }
}
impl Effect for DisableMagic {
    fn update_stats(&self, mut unitstats: UnitStats) -> UnitStats {
        if self.magic_power < 20 {
            return unitstats
        }
        unitstats.moves = 0;
        unitstats
    }
    fn on_tick(&mut self) -> bool {
        self.info.lifetime -= 1;
        true
    }
    fn is_dead(&self) -> bool {
        self.info.lifetime < 1
    }
}

const POISON_PERCENT: u64 = 15;
#[derive(Copy, Clone, Debug)]
pub struct Poison {
    pub info: EffectInfo,
}
impl Effect for Poison {
    fn update_stats(&self, unitstats: UnitStats) -> UnitStats {
        let mut unitstats = unitstats;
        unitstats.hp -= unitstats.hp / 100 * POISON_PERCENT;
        unitstats
    }
    fn on_battle_end(&mut self) -> bool {
        self.info.lifetime = 0;
        true
    }
    fn is_dead(&self) -> bool {
        self.info.lifetime < 1
    }
}

const FIRE_PERCENT: u64 = 10;
const FIRE_SLOWNESS_PERCENT: u64 = 50;
#[derive(Copy, Clone, Debug)]
pub struct Fire {
    pub info: EffectInfo,
}
impl Effect for Fire {
    fn update_stats(&self, unitstats: UnitStats) -> UnitStats {
        let mut unitstats = unitstats;
        unitstats.hp -= unitstats.hp / 100 * FIRE_PERCENT;
        unitstats.speed -= unitstats.speed / 100 * FIRE_SLOWNESS_PERCENT;
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
    fn is_dead(&self) -> bool {
        self.info.lifetime < 1
    }
}
impl Default for Fire {
    fn default() -> Self {
        Self {
            info: EffectInfo { lifetime: 5 }
        }
    }
}


#[derive(Copy, Clone, Debug)]
pub struct ItemEffect {
    pub info: EffectInfo,
    pub additions: UnitStats
}
impl Effect for ItemEffect {
    fn update_stats(&self, unitstats: UnitStats) -> UnitStats {
        unitstats + self.additions
    }
    fn on_battle_end(&mut self) -> bool {
        self.info.lifetime = 0;
        true
    }
}