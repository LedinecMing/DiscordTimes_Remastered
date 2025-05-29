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
#[derive(Clone, Debug, PartialEq)]
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
	BlockEffect(BlockEffect),
}

dyn_clone::clone_trait_object!(EffectTrait);
#[enum_dispatch(Effect)]
pub trait EffectTrait: DynClone + Debug + Send + Sync {
    fn update_stats(&mut self, unit: &mut Unit);
    fn on_tick(&mut self, unit: &mut Unit) -> bool {
        false
    }
    fn on_battle_end(&mut self) -> bool {
        false
    }
    fn kill(&mut self, unit: &mut Unit) {}
    fn is_dead(&self) -> bool {
        false
    }
    fn get_kind(&self) -> EffectKind {
        EffectKind::Bonus
    }
}

#[derive(Copy, Clone, Debug, PartialEq)]
#[alkahest(Deserialize, Serialize, SerializeRef, Formula)]
pub struct EffectInfo {
    pub lifetime: i32,
}

#[derive(Copy, Clone, Debug, PartialEq)]
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
    }
    fn on_tick(&mut self, unit: &mut Unit) -> bool {
        self.info.lifetime -= 1;
        true
    }
    fn kill(&mut self, unit: &mut Unit) {
        unit.modify.max_moves -= *Modify::default().add(1);
    }
    fn is_dead(&self) -> bool {
        self.info.lifetime < 1
    }
}

#[derive(Copy, Clone, Debug, PartialEq)]
#[alkahest(Deserialize, Serialize, SerializeRef, Formula)]
pub struct HealMagic {
    pub info: EffectInfo,
    pub magic_power: u64,
	pub magic_type: MagicType
}
impl HealMagic {
    pub fn new(magic_power: u64, magic_type: MagicType) -> Self {
        let mut magic = Self::default();
        magic.magic_power = magic_power;
		magic.magic_type = magic_type;
        magic
    }
}
impl Default for HealMagic {
    fn default() -> Self {
        Self {
            info: EffectInfo { lifetime: 1 },
            magic_power: 15,
			magic_type: MagicType::Life
        }
    }
}
impl EffectTrait for HealMagic {
    fn update_stats(&mut self, unit: &mut Unit) {
        let unitstats = unit.stats;
        let damage = unitstats.damage;
        let defence = unitstats.defence;
		let magic = self.magic_power as i64;
        let mut add_attack = (self.magic_power / 5) as i64;
        let mut add_defence = (self.magic_power / 10) as i64;
		match self.magic_type {
			MagicType::Death => {
				add_attack = 1 + magic / 6;
				add_defence = magic / 12;
			},
			MagicType::Life => {
				add_attack = magic / 8;
				add_defence = 1 + magic / 4;
			},
			_ => {}
		}
        if damage.hand > 0 {
            unit.modify.damage.hand += *Modify::default().add(add_attack);
        }
        if damage.ranged > 0 {
            unit.modify.damage.ranged += *Modify::default().add(add_attack);
        }

        unit.modify.defence.hand_units += *Modify::default().add(add_defence);
        unit.modify.defence.ranged_units += *Modify::default().add(add_defence);
    }
    fn on_tick(&mut self, unit: &mut Unit) -> bool {
        self.info.lifetime -= 1;
        true
    }
    fn kill(&mut self, unit: &mut Unit) {
        let unitstats = unit.stats;
        let damage = unitstats.damage;
        let defence = unitstats.defence;
        let magic = self.magic_power as i64;
        let mut add_attack = (self.magic_power / 5) as i64;
        let mut add_defence = (self.magic_power / 10) as i64;
		match self.magic_type {
			MagicType::Death => {
				add_attack = 1 + magic / 6;
				add_defence = magic / 12;
			},
			MagicType::Life => {
				add_attack = magic / 8;
				add_defence = 1 + magic / 4;
			},
			_ => {}
		}
        if damage.hand > 0 {
            unit.modify.damage.hand -= *Modify::default().add(add_attack);
        }
        if damage.ranged > 0 {
            unit.modify.damage.ranged -= *Modify::default().add(add_attack);
        }

        unit.modify.defence.hand_units -= *Modify::default().add(add_defence);
        unit.modify.defence.ranged_units -= *Modify::default().add(add_defence);
    }
    fn is_dead(&self) -> bool {
        self.info.lifetime < 1
    }
    fn get_kind(&self) -> EffectKind {
        EffectKind::MageSupport
    }
}

#[derive(Copy, Clone, Debug, PartialEq)]
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
        let add_moves = match self.magic_power {
			0..20 => 0,
			20..45 => 1,
			45..100 => 2,
			100..256 => 3,
			_ => 3 + (self.magic_power - 256) / 50 / 5
		} as i64;
        unit.stats.moves -= add_moves;
        unit.modify.max_moves -= *Modify::default().add(add_moves);
    }
    fn on_tick(&mut self, unit: &mut Unit) -> bool {
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
        let add_moves = match self.magic_power {
			0..20 => 0,
			20..45 => 1,
			45..100 => 2,
			100..256 => 3,
			_ => 3 + (self.magic_power - 256) / 50 / 5
		} as i64;
        unit.stats.moves += add_moves;
        unit.modify.max_moves += *Modify::default().add(add_moves);
    }
    fn get_kind(&self) -> EffectKind {
        EffectKind::MageCurse
    }
}

#[derive(Copy, Clone, Debug, PartialEq)]
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
		let add_moves = match self.magic_power {
			0..=19 => 0,
			20..=44 => 1,
			45..=99 => 2,
			100..=255 => 3,
			_ => self.magic_power as i64 / 64
		};
        unit.stats.moves += add_moves;
        unit.modify.max_moves += *Modify::default().add(add_moves);
    }
    fn on_tick(&mut self, unit: &mut Unit) -> bool {
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
        let add_moves = match self.magic_power {
			0..=19 => 0,
			20..=44 => 1,
			45..=99 => 2,
			100..=255 => 3,
			_ => self.magic_power as i64 / 64
		};
        unit.stats.moves -= add_moves;
        unit.modify.max_moves -= *Modify::default().add(add_moves);
    }
    fn is_dead(&self) -> bool {
        self.info.lifetime < 1
    }
    fn get_kind(&self) -> EffectKind {
        EffectKind::MageSupport
    }
}

#[derive(Copy, Clone, Debug, PartialEq)]
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
            magic_type: MagicType::Death,
        }
    }
}
impl EffectTrait for AttackMagic {
    fn update_stats(&mut self, unit: &mut Unit) {
        let stats = unit.stats;
        let damage = stats.damage;
        let defence = stats.defence;
		let magic = self.magic_power as i64;
        let mut minus_attack = 1 + (magic / 10);
        let mut minus_defence = 1 + (magic / 5);
		match self.magic_type {
			MagicType::Death => {
				minus_defence = magic / 10;
				minus_attack = 1 + magic / 5;
			},
			MagicType::Life => {
				minus_defence = 1 + magic / 3;
				minus_attack = magic / 10;
			},
			_ => {}
		}
        if damage.hand > 0 {
            unit.modify.damage.hand -= *Modify::default().add(minus_attack);
        }
        if damage.ranged > 0 {
            unit.modify.damage.ranged -= *Modify::default().add(minus_attack);
        }

		unit.modify.defence.hand_units -= *Modify::default().add(minus_defence);
        unit.modify.defence.ranged_units -= *Modify::default().add(minus_defence);
    }
    fn on_tick(&mut self, unit: &mut Unit) -> bool {
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
		let magic = self.magic_power as i64;
        let mut minus_attack = 1 + (magic / 10);
        let mut minus_defence = 1 + (magic / 5);
		match self.magic_type {
			MagicType::Death => {
				minus_defence = magic / 10;
				minus_attack = 1 + magic / 5;
			},
			MagicType::Life => {
				minus_defence = 1 + magic / 3;
				minus_attack = magic / 10;
			},
			_ => {}
		}
        if damage.hand > 0 {
            unit.modify.damage.hand += *Modify::default().add(minus_attack);
        }
        if damage.ranged > 0 {
            unit.modify.damage.ranged += *Modify::default().add(minus_attack);
        }
		
        unit.modify.defence.hand_units += *Modify::default().add(minus_defence);
        unit.modify.defence.ranged_units += *Modify::default().add(minus_defence);
    }
    fn is_dead(&self) -> bool {
        self.info.lifetime < 1
    }
    fn get_kind(&self) -> EffectKind {
        EffectKind::MageCurse
    }
}

const POISON_PERCENT: Percent = Percent::const_new(15);
#[derive(Copy, Clone, Debug, PartialEq)]
#[alkahest(Deserialize, Serialize, SerializeRef, Formula)]
pub struct Poison {
    pub info: EffectInfo,
}
impl EffectTrait for Poison {
    fn update_stats(&mut self, unit: &mut Unit) {
		
    }
    fn on_battle_end(&mut self) -> bool {
        self.info.lifetime = 0;
        true
    }
	fn on_tick(&mut self,unit: &mut Unit) -> bool {
		unit.stats.hp -= POISON_PERCENT.calc(unit.modified.max_hp);
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
            info: EffectInfo { lifetime: 1 },
        }
    }
}

const FIRE_PERCENT: Percent = Percent::const_new(10);
const FIRE_SLOWNESS_PERCENT: Percent = Percent::const_new(10);
#[derive(Copy, Clone, Debug, PartialEq)]
#[alkahest(Deserialize, Serialize, SerializeRef, Formula)]
pub struct Fire {
    pub info: EffectInfo,
    addition_speed: i64,
    additional_power: i64,
}
impl EffectTrait for Fire {
    fn update_stats(&mut self, unit: &mut Unit) {
        self.addition_speed =
            FIRE_SLOWNESS_PERCENT.calc(unit.modified.speed) + self.additional_power / 10;
		unit.modify.speed -= *Modify::default().add(self.addition_speed);
    }
    fn on_tick(&mut self, unit: &mut Unit) -> bool {
		unit.stats.hp -= (FIRE_PERCENT + Percent::new(self.additional_power as i16 / 5))
            .calc(unit.modified.max_hp)
            * ((unit.info.unit_type == UnitType::Mecha) as i64 + 1);
        self.info.lifetime -= 1;
        true
    }
    fn on_battle_end(&mut self) -> bool {
        self.info.lifetime = 0;
        true
    }
    fn kill(&mut self, unit: &mut Unit) {
		unit.modify.speed += *Modify::default().add(self.addition_speed);
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

#[derive(Copy, Clone, Debug, PartialEq)]
#[alkahest(Deserialize, Serialize, SerializeRef, Formula)]
pub struct ArtilleryEffect {
    pub info: EffectInfo,
}
impl EffectTrait for ArtilleryEffect {
    fn update_stats(&mut self, unit: &mut Unit) {
        unit.modify.speed += *Modify::default().add(30);
    }
    fn on_tick(&mut self, unit: &mut Unit) -> bool {
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

#[derive(Copy, Clone, Debug, PartialEq)]
#[alkahest(Deserialize, Serialize, SerializeRef, Formula)]
pub struct RessurectedEffect {}
impl EffectTrait for RessurectedEffect {
    fn update_stats(&mut self, unit: &mut Unit) {
        unit.stats.hp += Percent::new(25).calc(unit.modified.max_hp);
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
#[derive(Copy, Clone, Debug, PartialEq)]
#[alkahest(Deserialize, Serialize, SerializeRef, Formula)]
pub struct SpearEffect {
    pub info: EffectInfo,
}
impl EffectTrait for SpearEffect {
    fn update_stats(&mut self, unit: &mut Unit) {
        unit.modify.defence.hand_units += *Modify::default().percent_add(SPEAR_PERCENT);
        unit.modify.defence.ranged_units += *Modify::default().percent_add(SPEAR_PERCENT);
    }
    fn on_tick(&mut self, unit: &mut Unit) -> bool {
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

const BLOCK_PERCENT: Percent = Percent::const_new(100);
#[derive(Copy, Clone, Debug, PartialEq)]
#[alkahest(Deserialize, Serialize, SerializeRef, Formula)]
pub struct BlockEffect {
    pub info: EffectInfo,
}
impl EffectTrait for BlockEffect {
    fn update_stats(&mut self, unit: &mut Unit) {
        unit.modify.defence.hand_units += *Modify::default().percent_add(BLOCK_PERCENT);
        unit.modify.defence.ranged_units += *Modify::default().percent_add(BLOCK_PERCENT);
    }
    fn on_tick(&mut self, unit: &mut Unit) -> bool {
        self.info.lifetime -= 1;
        true
    }
    fn kill(&mut self, unit: &mut Unit) {
        unit.modify.defence.hand_units -= *Modify::default().percent_add(BLOCK_PERCENT);
        unit.modify.defence.ranged_units -= *Modify::default().percent_add(BLOCK_PERCENT);
    }
    fn is_dead(&self) -> bool {
        self.info.lifetime < 1
    }
}

#[derive(Copy, Clone, Debug, PartialEq)]
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

#[derive(Copy, Clone, Debug, PartialEq)]
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
