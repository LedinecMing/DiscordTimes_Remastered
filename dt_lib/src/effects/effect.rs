use crate::units::{
    unit::{Unit, *},
    unitstats::*,
};
use alkahest::alkahest;
use dyn_clone::DynClone;
use enum_dispatch::enum_dispatch;
use math_thingies::Percent;
use std::fmt::Debug;

#[derive(PartialEq)]
#[alkahest(Deserialize, Serialize, SerializeRef, Formula)]
pub enum EffectKind {
    MageCurse,
    MageSupport,
    Bonus,
    Item,
    Potion,
    Poison,
    Fire,
}

#[enum_dispatch]
#[derive(Clone, Debug)]
#[alkahest(Deserialize, Serialize, SerializeRef, Formula)]
pub enum Effect {
    MoreMoves(MoreMoves),
    HealMagic(HealMagic),
    DisableMagic(DisableMagic),
    ElementalSupport(ElementalSupport),
    AttackMagic(AttackMagic),
    Poison(Poison),
    Fire(Fire),
    ArtilleryEffect(ArtilleryEffect),
    RessurectedEffect(RessurectedEffect),
    SpearEffect(SpearEffect),
    ItemEffect(ItemEffect),
    ToEndEffect(ToEndEffect),
}

dyn_clone::clone_trait_object!(EffectTrait);
#[enum_dispatch(Effect)]
pub trait EffectTrait: DynClone + Debug + Send + Sync {
    fn update_stats(&mut self, unit: &mut Unit);
    fn on_tick(&mut self) -> bool {
        false
    }
    fn on_battle_end(&mut self) -> bool {
        false
    }
    fn tick(&mut self, unit: &mut Unit) -> bool {
        self.on_tick();
        true
    }
    fn kill(&mut self, unit: &mut Unit) {}
    fn is_dead(&self) -> bool {
        false
    }
    fn get_kind(&self) -> EffectKind {
        EffectKind::Bonus
    }
}

#[derive(PartialEq, Copy, Clone, Debug)]
#[alkahest(Deserialize, Serialize, SerializeRef, Formula)]
pub struct EffectInfo {
    pub lifetime: i32,
}

#[derive(Copy, Clone, Debug)]
#[alkahest(Deserialize, Serialize, SerializeRef, Formula)]
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
impl EffectTrait for MoreMoves {
    fn update_stats(&mut self, unit: &mut Unit) {
        unit.modify.max_moves += *Modify::default().add(1);
        unit.modify.moves += *Modify::default().add(1);
    }
    fn on_tick(&mut self) -> bool {
        self.info.lifetime -= 1;
        true
    }
    fn kill(&mut self, unit: &mut Unit) {
        unit.modify.max_moves -= *Modify::default().add(1);
        unit.modify.moves -= *Modify::default().add(1);
    }
    fn is_dead(&self) -> bool {
        self.info.lifetime < 1
    }
}

#[derive(Copy, Clone, Debug)]
#[alkahest(Deserialize, Serialize, SerializeRef, Formula)]
pub struct HealMagic {
    pub info: EffectInfo,
    pub magic_power: u64,
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
        }
    }
}
impl EffectTrait for HealMagic {
    fn update_stats(&mut self, unit: &mut Unit) {
        let unitstats = unit.stats;
        let damage = unitstats.damage;
        let defence = unitstats.defence;
        let damage_add = (self.magic_power / 5) as i64;
        let defence_add = (self.magic_power / 10) as i64;

        if damage.hand > 0 {
            unit.modify.damage.hand += *Modify::default().add(damage_add);
        }
        if damage.ranged > 0 {
            unit.modify.damage.ranged += *Modify::default().add(damage_add);
        }

        unit.modify.defence.hand_units += *Modify::default().add(defence_add);
        unit.modify.defence.ranged_units += *Modify::default().add(defence_add);
    }
    fn on_tick(&mut self) -> bool {
        self.info.lifetime -= 1;
        true
    }
    fn kill(&mut self, unit: &mut Unit) {
        let unitstats = unit.stats;
        let damage = unitstats.damage;
        let defence = unitstats.defence;
        let damage_add = (self.magic_power / 5) as i64;
        let defence_add = (self.magic_power / 10) as i64;

        if damage.hand > 0 {
            unit.modify.damage.hand -= *Modify::default().add(damage_add);
        }
        if damage.ranged > 0 {
            unit.modify.damage.ranged -= *Modify::default().add(damage_add);
        }

        unit.modify.defence.hand_units -= *Modify::default().add(defence_add);
        unit.modify.defence.ranged_units -= *Modify::default().add(defence_add);
    }
    fn is_dead(&self) -> bool {
        self.info.lifetime < 1
    }
    fn get_kind(&self) -> EffectKind {
        EffectKind::MageSupport
    }
}

#[derive(Copy, Clone, Debug)]
#[alkahest(Deserialize, Serialize, SerializeRef, Formula)]
pub struct DisableMagic {
    pub info: EffectInfo,
    pub magic_power: u64,
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
        }
    }
}
impl EffectTrait for DisableMagic {
    fn update_stats(&mut self, unit: &mut Unit) {
        if self.magic_power < 20 {
            return;
        }
        let add_moves = 1 + (self.magic_power / 50) as i64;
        unit.modify.moves -= *Modify::default().add(add_moves);
        unit.modify.max_moves -= *Modify::default().add(add_moves);
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
    fn kill(&mut self, unit: &mut Unit) {
        if self.magic_power < 20 {
            return;
        }
        let add_moves = 1 + (self.magic_power / 50) as i64;
        unit.modify.moves += *Modify::default().add(add_moves);
        unit.modify.max_moves += *Modify::default().add(add_moves);
    }
    fn get_kind(&self) -> EffectKind {
        EffectKind::MageCurse
    }
}

#[derive(Copy, Clone, Debug)]
#[alkahest(Deserialize, Serialize, SerializeRef, Formula)]
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
impl EffectTrait for ElementalSupport {
    fn update_stats(&mut self, unit: &mut Unit) {
        if self.magic_power < 20 {
            return;
        }
        let add_moves = 1 + (self.magic_power / 50) as i64;
        unit.modify.moves += *Modify::default().add(add_moves);
        unit.modify.max_moves += *Modify::default().add(add_moves);
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
        if self.magic_power < 20 {
            return;
        }
        let add_moves = 1 + (self.magic_power / 50) as i64;
        unit.modify.moves -= *Modify::default().add(add_moves);
        unit.modify.max_moves -= *Modify::default().add(add_moves);
    }
    fn is_dead(&self) -> bool {
        self.info.lifetime < 1
    }
    fn get_kind(&self) -> EffectKind {
        EffectKind::MageSupport
    }
}

#[derive(Copy, Clone, Debug)]
#[alkahest(Deserialize, Serialize, SerializeRef, Formula)]
pub struct AttackMagic {
    pub info: EffectInfo,
    pub magic_power: u64,
    pub magic_type: MagicType,
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
            magic_type: MagicType::Death(MagicDirection::ToEnemy),
        }
    }
}
impl EffectTrait for AttackMagic {
    fn update_stats(&mut self, unit: &mut Unit) {
        let stats = unit.stats;
        let damage = stats.damage;
        let defence = stats.defence;
        let damage_add = 1 + (self.magic_power / 10) as i64;
        let defence_add = 1 + (self.magic_power / 5) as i64;
        if damage.hand > 0 {
            unit.modify.damage.hand -= *Modify::default().add(damage_add);
        }
        if damage.ranged > 0 {
            unit.modify.damage.ranged -= *Modify::default().add(damage_add);
        }
        if defence.hand_units > 0 {
            unit.modify.defence.hand_units -= *Modify::default().add(defence_add);
        }
        if defence.ranged_units > 0 {
            unit.modify.defence.ranged_units -= *Modify::default().add(defence_add);
        }
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
        let stats = unit.stats;
        let damage = stats.damage;
        let defence = stats.defence;
        let damage_add = 1 + (self.magic_power / 10) as i64;
        let defence_add = 1 + (self.magic_power / 5) as i64;
        if damage.hand > 0 {
            unit.modify.damage.hand += *Modify::default().add(damage_add);
        }
        if damage.ranged > 0 {
            unit.modify.damage.ranged += *Modify::default().add(damage_add);
        }
        if defence.hand_units > 0 {
            unit.modify.defence.hand_units += *Modify::default().add(defence_add);
        }
        if defence.ranged_units > 0 {
            unit.modify.defence.ranged_units += *Modify::default().add(defence_add);
        }
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
#[alkahest(Deserialize, Serialize, SerializeRef, Formula)]
pub struct Poison {
    pub info: EffectInfo,
}
impl EffectTrait for Poison {
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
#[alkahest(Deserialize, Serialize, SerializeRef, Formula)]
pub struct Fire {
    pub info: EffectInfo,
    addition_speed: i64,
    additional_power: i64,
}
impl EffectTrait for Fire {
    fn update_stats(&mut self, unit: &mut Unit) {
        let mut unitstats = unit.stats;
        unitstats.hp -= (FIRE_PERCENT + Percent::new(self.additional_power as i16 / 5))
            .calc(unitstats.hp)
            * ((unit.info.unit_type == UnitType::Mecha) as i64 + 1);
        self.addition_speed =
            FIRE_SLOWNESS_PERCENT.calc(unitstats.speed) + self.additional_power / 10;
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
const STANDART_FIRE_LONG: i32 = 5;
impl Fire {
    pub fn new(additional_power: i64) -> Self {
        Self {
            info: EffectInfo {
                lifetime: STANDART_FIRE_LONG + additional_power as i32 / 25,
            },
            addition_speed: 0,
            additional_power,
        }
    }
}
impl Default for Fire {
    fn default() -> Self {
        Self {
            info: EffectInfo { lifetime: 5 },
            addition_speed: 0,
            additional_power: 0,
        }
    }
}

#[derive(Copy, Clone, Debug)]
#[alkahest(Deserialize, Serialize, SerializeRef, Formula)]
pub struct ArtilleryEffect {
    pub info: EffectInfo,
}
impl EffectTrait for ArtilleryEffect {
    fn update_stats(&mut self, unit: &mut Unit) {
        unit.modify.speed += *Modify::default().add(30);
    }
    fn on_tick(&mut self) -> bool {
        self.info.lifetime -= 1;
        true
    }
    fn kill(&mut self, unit: &mut Unit) {
        unit.modify.speed -= *Modify::default().add(30);
    }
    fn is_dead(&self) -> bool {
        self.info.lifetime < 1
    }
}

#[derive(Copy, Clone, Debug)]
#[alkahest(Deserialize, Serialize, SerializeRef, Formula)]
pub struct RessurectedEffect {}
impl EffectTrait for RessurectedEffect {
    fn update_stats(&mut self, unit: &mut Unit) {
        unit.stats.hp += Percent::new(25);
    }
    fn get_kind(&self) -> EffectKind {
        EffectKind::Fire
    }
    fn is_dead(&self) -> bool {
        false
    }
}
impl RessurectedEffect {
    pub fn new() -> Self {
        RessurectedEffect {}
    }
}

const SPEAR_PERCENT: Percent = Percent::const_new(200);
#[derive(Copy, Clone, Debug)]
#[alkahest(Deserialize, Serialize, SerializeRef, Formula)]
pub struct SpearEffect {
    pub info: EffectInfo,
}
impl EffectTrait for SpearEffect {
    fn update_stats(&mut self, unit: &mut Unit) {
        unit.modify.defence.hand_units += *Modify::default().percent_add(SPEAR_PERCENT);
        unit.modify.defence.ranged_units += *Modify::default().percent_add(SPEAR_PERCENT);
    }
    fn on_tick(&mut self) -> bool {
        self.info.lifetime -= 1;
        true
    }
    fn kill(&mut self, unit: &mut Unit) {
        unit.modify.defence.hand_units -= *Modify::default().percent_add(SPEAR_PERCENT);
        unit.modify.defence.ranged_units -= *Modify::default().percent_add(SPEAR_PERCENT);
    }
    fn is_dead(&self) -> bool {
        self.info.lifetime < 1
    }
}

#[derive(Copy, Clone, Debug)]
#[alkahest(Deserialize, Serialize, SerializeRef, Formula)]
pub struct ItemEffect {
    pub info: EffectInfo,
    pub modify: ModifyUnitStats,
}
impl EffectTrait for ItemEffect {
    fn update_stats(&mut self, unit: &mut Unit) {
        unit.modify += self.modify;
    }
    fn kill(&mut self, unit: &mut Unit) {
        unit.modify -= self.modify;
    }
    fn get_kind(&self) -> EffectKind {
        EffectKind::Item
    }
}

#[derive(Copy, Clone, Debug)]
#[alkahest(Deserialize, Serialize, SerializeRef, Formula)]
pub struct ToEndEffect {
    pub info: EffectInfo,
    pub modify: ModifyUnitStats,
}
impl EffectTrait for ToEndEffect {
    fn update_stats(&mut self, unit: &mut Unit) {
        unit.modify += self.modify;
    }
    fn on_battle_end(&mut self) -> bool {
        self.info.lifetime = 0;
        true
    }
    fn kill(&mut self, unit: &mut Unit) {
        unit.modify -= self.modify;
    }
    fn is_dead(&self) -> bool {
        self.info.lifetime < 1
    }
    fn get_kind(&self) -> EffectKind {
        EffectKind::Bonus
    }
}
