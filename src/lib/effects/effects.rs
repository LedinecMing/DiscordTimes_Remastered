use {
    crate::lib::{
        effects::effect::{Effect, EffectInfo, EffectKind},
        units::unit::*,
    },
    math_thingies::{add_if_nat, nat, saturating, Percent},
    std::cmp::{max, min},
};

#[derive(Copy, Clone, Debug)]
pub struct MoreMoves {
    pub info: EffectInfo,
}
impl Default for MoreMoves {
    fn default() -> Self {
        Self {
            info: EffectInfo { lifetime: 1 },
        }
    }
}
impl Effect for MoreMoves {
    fn update_stats(&mut self, unit: &mut Unit) {
        unit.stats.max_moves += 1;
        unit.stats.moves += 1;
    }
    fn on_tick(&mut self) -> bool {
        self.info.lifetime -= 1;
        true
    }
    fn kill(&mut self, unit: &mut Unit) {
        unit.stats.max_moves -= 1;
    }
    fn is_dead(&self) -> bool {
        self.info.lifetime < 1
    }
}

#[derive(Copy, Clone, Debug)]
pub struct HealMagic {
    pub info: EffectInfo,
    pub magic_power: u64,
    pub additional_damage: Power,
    pub additional_defense: Defence,
}
impl HealMagic {
    pub fn new(magic_power: u64) -> Self {
        let mut magic = Self::default();
        magic.magic_power = magic_power;
        magic
    }
}
impl Default for HealMagic {
    fn default() -> Self {
        Self {
            info: EffectInfo { lifetime: 1 },
            magic_power: 15,
            additional_damage: Power::empty(),
            additional_defense: Defence::empty(),
        }
    }
}
impl Effect for HealMagic {
    fn update_stats(&mut self, unit: &mut Unit) {
        let unitstats = unit.stats;
        let damage = unitstats.damage;
        let defence = unitstats.defence;
        let damage_add = self.magic_power / 5;
        let defence_add = self.magic_power / 10;

        if damage.hand > 0 {
            self.additional_damage.hand = damage_add;
        }
        if damage.ranged > 0 {
            self.additional_damage.ranged = damage_add;
        }
        self.additional_defense.hand_units = defence_add;
        self.additional_defense.ranged_units = defence_add;

        unit.stats.damage = unit.stats.damage + self.additional_damage;
        unit.stats.defence = unit.stats.defence + self.additional_defense;
    }
    fn on_tick(&mut self) -> bool {
        self.info.lifetime -= 1;
        true
    }
    fn kill(&mut self, unit: &mut Unit) {
        unit.stats.damage = unit.stats.damage - self.additional_damage;
        unit.stats.defence = unit.stats.defence - self.additional_defense;
    }
    fn is_dead(&self) -> bool {
        self.info.lifetime < 1
    }
    fn get_kind(&self) -> EffectKind {
        EffectKind::MageSupport
    }
}

#[derive(Copy, Clone, Debug)]
pub struct DisableMagic {
    pub info: EffectInfo,
    pub magic_power: u64,
    pub additional_damage: Power,
    pub additional_defense: Defence,
}
impl DisableMagic {
    pub fn new(magic_power: u64) -> Self {
        let mut magic = Self::default();
        magic.magic_power = magic_power;
        magic
    }
}
impl Default for DisableMagic {
    fn default() -> Self {
        Self {
            info: EffectInfo { lifetime: 1 },
            magic_power: 15,
            additional_damage: Power::empty(),
            additional_defense: Defence::empty(),
        }
    }
}
impl Effect for DisableMagic {
    fn update_stats(&mut self, unit: &mut Unit) {
        if self.magic_power < 20 {
            return;
        }
        unit.stats.moves = unit.stats.moves.saturating_sub(1 + self.magic_power / 50);
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
    fn get_kind(&self) -> EffectKind {
        EffectKind::MageCurse
    }
}

#[derive(Copy, Clone, Debug)]
pub struct ElementalSupport {
    pub info: EffectInfo,
    pub magic_power: u64,
}
impl ElementalSupport {
    pub fn new(magic_power: u64) -> Self {
        let mut magic = Self::default();
        magic.magic_power = magic_power;
        magic
    }
}
impl Default for ElementalSupport {
    fn default() -> Self {
        Self {
            info: EffectInfo { lifetime: 1 },
            magic_power: 15,
        }
    }
}
impl Effect for ElementalSupport {
    fn update_stats(&mut self, unit: &mut Unit) {
        if self.magic_power < 20 {
            return;
        }
        unit.stats.moves = unit.stats.moves.saturating_add(1 + self.magic_power / 50);
    }
    fn on_tick(&mut self) -> bool {
        self.info.lifetime -= 1;
        true
    }
    fn on_battle_end(&mut self) -> bool {
        self.info.lifetime = 0;
        true
    }
    fn kill(&mut self, unit: &mut Unit) {}
    fn is_dead(&self) -> bool {
        self.info.lifetime < 1
    }
    fn get_kind(&self) -> EffectKind {
        EffectKind::MageSupport
    }
}

#[derive(Copy, Clone, Debug)]
pub struct AttackMagic {
    pub info: EffectInfo,
    pub magic_power: u64,
    additions: UnitStats,
}
impl AttackMagic {
    pub fn new(magic_power: u64) -> Self {
        let mut magic = Self::default();
        magic.magic_power = magic_power;
        magic
    }
}
impl Default for AttackMagic {
    fn default() -> Self {
        Self {
            info: EffectInfo { lifetime: 1 },
            magic_power: 15,
            additions: UnitStats::empty(),
        }
    }
}
impl Effect for AttackMagic {
    fn update_stats(&mut self, unit: &mut Unit) {
        let stats = unit.stats;
        let damage = stats.damage;
        let defence = stats.defence;
        if damage.hand > 0 {
            self.additions.damage.hand = min(1 + self.magic_power / 10, stats.damage.hand);
        }
        if damage.ranged > 0 {
            self.additions.damage.ranged = min(1 + self.magic_power / 10, stats.damage.ranged);
        }
        if defence.hand_units > 0 {
            self.additions.defence.hand_units =
                min(1 + self.magic_power / 5, stats.defence.hand_units);
        }
        if defence.ranged_units > 0 {
            self.additions.defence.ranged_units =
                min(1 + self.magic_power / 5, stats.defence.ranged_units);
        }
        unit.stats = unit.stats - self.additions;
    }
    fn on_tick(&mut self) -> bool {
        self.info.lifetime -= 1;
        true
    }
    fn on_battle_end(&mut self) -> bool {
        self.info.lifetime = 0;
        true
    }
    fn kill(&mut self, unit: &mut Unit) {
        unit.stats = unit.stats - self.additions;
    }
    fn is_dead(&self) -> bool {
        self.info.lifetime < 1
    }
    fn get_kind(&self) -> EffectKind {
        EffectKind::MageCurse
    }
}

const POISON_PERCENT: Percent = Percent::const_new(15);
#[derive(Copy, Clone, Debug)]
pub struct Poison {
    pub info: EffectInfo,
}
impl Effect for Poison {
    fn update_stats(&mut self, unit: &mut Unit) {
        unit.stats.hp -= POISON_PERCENT.calc(unit.stats.hp);
    }
    fn on_battle_end(&mut self) -> bool {
        self.info.lifetime = 0;
        true
    }
    fn is_dead(&self) -> bool {
        self.info.lifetime < 1
    }
    fn get_kind(&self) -> EffectKind {
        EffectKind::Poison
    }
}

impl Default for Poison {
    fn default() -> Self {
        Self {
            info: EffectInfo { lifetime: -1 },
        }
    }
}

const FIRE_PERCENT: Percent = Percent::const_new(10);
const FIRE_SLOWNESS_PERCENT: Percent = Percent::const_new(10);
#[derive(Copy, Clone, Debug)]
pub struct Fire {
    pub info: EffectInfo,
    addition_speed: u64,
}
impl Effect for Fire {
    fn update_stats(&mut self, unit: &mut Unit) {
        let mut unitstats = unit.stats;
        unitstats.hp -= FIRE_PERCENT.calc(unitstats.hp);
        self.addition_speed = FIRE_SLOWNESS_PERCENT.calc(unitstats.speed);
        unitstats.speed -= self.addition_speed;
    }
    fn on_tick(&mut self) -> bool {
        self.info.lifetime -= 1;
        true
    }
    fn on_battle_end(&mut self) -> bool {
        self.info.lifetime = 0;
        true
    }
    fn kill(&mut self, unit: &mut Unit) {
        unit.stats.speed = unit.stats.speed - self.addition_speed;
    }
    fn is_dead(&self) -> bool {
        self.info.lifetime < 1
    }
    fn get_kind(&self) -> EffectKind {
        EffectKind::Fire
    }
}
impl Default for Fire {
    fn default() -> Self {
        Self {
            info: EffectInfo { lifetime: 5 },
            addition_speed: 0,
        }
    }
}

#[derive(Copy, Clone, Debug)]
pub struct ArtilleryEffect {
    pub info: EffectInfo,
}
impl Effect for ArtilleryEffect {
    fn update_stats(&mut self, unit: &mut Unit) {
        unit.stats.speed += 20;
    }
    fn on_tick(&mut self) -> bool {
        self.info.lifetime -= 1;
        true
    }
    fn kill(&mut self, unit: &mut Unit) {
        unit.stats.speed -= 20;
    }
    fn is_dead(&self) -> bool {
        self.info.lifetime < 1
    }
}

#[derive(Copy, Clone, Debug)]
pub struct SpearEffect {
    pub info: EffectInfo,
    pub added_defence: Defence,
}
impl Effect for SpearEffect {
    fn update_stats(&mut self, unit: &mut Unit) {
        let percent_0 = Percent::new(0);
        self.added_defence = Defence {
            death_magic: percent_0,
            elemental_magic: percent_0,
            life_magic: percent_0,
            hand_percent: percent_0,
            ranged_percent: percent_0,
            magic_units: 0,
            hand_units: unit.stats.defence.hand_units * 2,
            ranged_units: unit.stats.defence.ranged_units * 2,
        };
        unit.stats.defence.hand_units *= 3;
        unit.stats.defence.ranged_units *= 3;
    }
    fn on_tick(&mut self) -> bool {
        self.info.lifetime -= 1;
        true
    }
    fn kill(&mut self, unit: &mut Unit) {
        unit.stats.defence = unit.stats.defence - self.added_defence;
    }
    fn is_dead(&self) -> bool {
        self.info.lifetime < 1
    }
}

#[derive(Copy, Clone, Debug)]
pub struct ItemEffect {
    pub info: EffectInfo,
    pub additions: UnitStats,
}
impl Effect for ItemEffect {
    fn update_stats(&mut self, unit: &mut Unit) {
        unit.stats = unit.stats + self.additions;
    }
    fn kill(&mut self, unit: &mut Unit) {
        unit.stats = unit.stats - self.additions;
    }
    fn get_kind(&self) -> EffectKind {
        EffectKind::Item
    }
}

#[derive(Copy, Clone, Debug)]
pub struct ToEndEffect {
    pub info: EffectInfo,
    pub additions: UnitStats,
}
impl Effect for ToEndEffect {
    fn update_stats(&mut self, unit: &mut Unit) {
        unit.stats = unit.stats + self.additions
    }
    fn on_battle_end(&mut self) -> bool {
        self.info.lifetime = 0;
        true
    }
    fn kill(&mut self, unit: &mut Unit) {
        unit.stats = unit.stats - self.additions;
    }
    fn is_dead(&self) -> bool {
        self.info.lifetime < 1
    }
    fn get_kind(&self) -> EffectKind {
        EffectKind::Bonus
    }
}
