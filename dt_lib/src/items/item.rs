use crate::effects::{self, EffectInfo};
use crate::parse::LOCALE;
use crate::units::unit::UnitType;
use crate::units::unitstats::ModifyUnitStats;

use crate::{
    bonuses::bonus::Bonus,
    units::unit::{MagicType, Unit},
};
use advini::{Ini, IniParseError};
use alkahest::alkahest;
use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::sync::RwLock;
use tracing_mutex::stdsync::TracingMutex as Mutex;
#[derive(Debug, Clone, PartialEq)]
pub enum ItemType {
    Artifact,
    Potion,
}
#[derive(Debug, Clone, PartialEq)]
pub enum ArtifactType {
    Weapon(WeaponType),
    Armor,
    Shield,
    Helmet,
    Ring,
    Amulet,
    Item,
	Potion
}
#[derive(Debug, Clone, PartialEq)]
pub enum WeaponType {
    Hand,
    Ranged,
    Magic,
}
#[derive(Clone, Debug)]
pub struct ItemInfo {
    pub name: String,
    pub description: String,
    pub cost: u64,
    pub icon: String,
    pub sells: bool,
    pub itemtype: ArtifactType,
	pub magic_req: MagicVariants,
    pub bonus: Option<Bonus>,
    pub modify: ModifyUnitStats,
}
impl ItemInfo {
	pub fn can_equip(&self, unit: &Unit) -> bool {
		let info = self;
		if unit.info.unit_type == UnitType::Mecha {
			return false;
		}
		if !magic_relates(unit.info.magic_info.and_then(|x| Some(x.0)), self.magic_req) {
			return false;
		}
        match &info.itemtype {
            ArtifactType::Item => false,
			ArtifactType::Potion => {
				true
			},
            ArtifactType::Amulet
            | ArtifactType::Armor
            | ArtifactType::Shield
            | ArtifactType::Ring
            | ArtifactType::Helmet => unit
                .inventory
                .items
                .iter()
                .filter_map(|x| x.and_then(|x| Some(x)))
                .filter(|item| item.get_info().itemtype == info.itemtype)
                .last()
                .is_none(),
            ArtifactType::Weapon(weapon_type) => {
                let damage = unit.modified.damage;
                (match weapon_type {
                    WeaponType::Hand => damage.hand > 0,
                    WeaponType::Ranged => damage.ranged > 0,
                    WeaponType::Magic => {
                        damage.magic > 0
                    }
                }) && {
                    unit.inventory
                        .items
                        .iter()
                        .filter(|item| {
                            if let Some(item) = item {
                                if let ArtifactType::Weapon(_) = item.get_info().itemtype {
                                    true
                                } else {
                                    false
                                }
                            } else {
                                false
                            }
                        })
                        .next()
                        .is_none()
                }
            }
        }
    }
	pub fn display_strings(&self) -> Vec<String> {
		let locale = LOCALE.read().unwrap();
		let mut strings = vec![];
		strings.push(self.name.clone());
		strings.push(self.description.clone());
		strings.push(format!("{:?}", self.itemtype));
		strings.push(format!("Sells: {}", self.sells));
		strings.push(format!("Magic: {:?}", self.magic_req));
		strings.append(&mut self.modify.display_string());
		strings.push(format!("Cost: {}", self.cost));
		if let Some(bonus) = self.bonus {
			let (name, desc) = bonus.locale_id();
			strings.push(locale.get(name));
			strings.push(locale.get(desc));
		}
		strings
	}
}
pub static ITEMS: Lazy<RwLock<Vec<ItemInfo>>> = Lazy::new(|| RwLock::new(Vec::new()));
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[alkahest(Deserialize, Serialize, SerializeRef, Formula)]
pub struct Item {
    pub index: usize,
}
impl Ini for Item {
    fn eat(chars: std::str::Chars) -> Result<(Self, std::str::Chars), IniParseError> {
        match <usize as Ini>::eat(chars) {
            Ok(v) => Ok((Self { index: v.0 }, v.1)),
            Err(err) => Err(err),
        }
    }
    fn vomit(&self) -> String {
        self.index.vomit()
    }
}
impl Item {
    pub fn get_info(&self) -> ItemInfo {
        ITEMS.read().unwrap().get(self.index).unwrap().clone()
    }
    pub fn can_equip(&self, unit: &Unit) -> bool {
        let info = self.get_info();
        info.can_equip(unit)
    }
}
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MagicVariants {
    Any,
    Death,
    Life,
    Elemental,
}
pub fn magic_relates(magic_type: Option<MagicType>, magic_variant: MagicVariants) -> bool {
	let Some(magic_type) = magic_type else {
		return matches!(magic_variant, MagicVariants::Any);
	};
    match (magic_type, magic_variant) {
        (MagicType::Death, MagicVariants::Death) => true,
        (MagicType::Life, MagicVariants::Life) => true,
        (MagicType::Elemental, MagicVariants::Elemental) => true,
        (_, MagicVariants::Any) => true,
        (_, _) => false,
    }
}
