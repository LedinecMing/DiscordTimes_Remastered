use crate::{
    effects::{self, EffectInfo}, locale::{self, Locale}, registry::{self, BonusId, GameInfo, Items, Units}, units::{unit::UnitType, unitstats::ModifyUnitStats}
};

use crate::{
    units::unit::{MagicType, Unit},
};
use advini::{Ini, IniParseError, Sections};
use alkahest::alkahest;
use once_cell::sync::Lazy;
use std::{collections::HashMap, sync::RwLock};
use tracing_mutex::stdsync::TracingMutex as Mutex;
#[derive(Debug, Clone, Ini, PartialEq)]
pub enum ItemType {
    Artifact,
    Potion,
}
#[derive(Debug, Clone, Ini, PartialEq)]
pub enum ArtifactType {
    Weapon(WeaponType),
    Armor,
    Shield,
    Helmet,
    Ring,
    Amulet,
    Item,
    Potion,
}
#[derive(Debug, Clone, Ini, PartialEq)]
pub enum WeaponType {
    Hand,
    Ranged,
    Magic,
}
#[derive(Clone, Debug, Sections, PartialEq)]
pub struct ItemInfo {
    pub name: String,
    pub description: String,
    pub cost: u64,
    pub icon: String,
    pub sells: bool,
    pub itemtype: ArtifactType,
    pub magic_req: MagicVariants,
    pub bonus: Option<BonusId>,
	#[inline_parsing]
    pub modify: ModifyUnitStats,
}
impl ItemInfo {
    pub fn can_equip(&self, unit: &Unit, registry: &GameInfo) -> bool {
        let unit_info = &registry.units[unit.id];
        if unit_info.unit_type == UnitType::Mecha {
            return false;
        }
        if !magic_relates(unit_info.magic_type, self.magic_req) {
            return false;
        }
        match &self.itemtype {
            ArtifactType::Item => false,
            ArtifactType::Potion => true,
            ArtifactType::Amulet
            | ArtifactType::Armor
            | ArtifactType::Shield
            | ArtifactType::Ring
            | ArtifactType::Helmet => unit
                .inventory
                .items
                .iter()
                .filter_map(|x| x.and_then(|x| Some(x)))
                .filter(|item| item.get_info(&registry.items).itemtype == self.itemtype)
                .last()
                .is_none(),
            ArtifactType::Weapon(weapon_type) => {
                let damage = unit.modified.damage;
                (match weapon_type {
                    WeaponType::Hand => damage.hand > 0,
                    WeaponType::Ranged => damage.ranged > 0,
                    WeaponType::Magic => damage.magic > 0,
                }) && {
                    unit.inventory
                        .items
                        .iter()
                        .filter(|item| {
                            if let Some(item) = item {
                                if let ArtifactType::Weapon(_) = item.get_info(&registry.items).itemtype {
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
    pub fn display_strings(&self, locale: &Locale) -> Vec<String> {
		let mut strings = vec![];
        strings.push(self.name.clone());
        strings.push(self.description.clone());
        strings.push(format!("{:?}", self.itemtype));
        strings.push(format!("Sells: {}", self.sells));
        strings.push(format!("Magic: {:?}", self.magic_req));
        strings.append(&mut self.modify.display_string(locale));
        strings.push(format!("Cost: {}", self.cost));
        if let Some(bonus) = self.bonus {
			// TODO
            // let (name, desc) = bonus.locale_id();
            // strings.push(locale.get(name));
            // strings.push(locale.get(desc));
        }
        strings
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
#[alkahest(Deserialize, Serialize, SerializeRef, Formula)]
pub struct Item {
    pub index: usize,
}
impl Ini<'_> for Item {
	type Arg = ();
	fn eat<'a>(input: &'a str, _additional: Self::Arg) -> Result<(&'a str, Self), IniParseError> {
		let res = usize::eat(input, _additional)?;
		Ok((res.0, Self {
			index: res.1,
		}))
	}
	fn vomit(&self, additional: Self::Arg) -> String {
		self.index.to_string()
	}
	
}
impl Item {
    pub fn get_info<'a>(&self, registry: &'a Items) -> &'a ItemInfo {
		&registry[self.index]
    }
    pub fn can_equip(&self, unit: &Unit, registry: &GameInfo) -> bool {
        let info = self.get_info(&registry.items);
        info.can_equip(unit, registry)
    }
}
#[derive(Debug, Clone, Copy, Ini, PartialEq)]
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
        (MagicType::DeathMagic, MagicVariants::Death) => true,
        (MagicType::LifeMagic, MagicVariants::Life) => true,
        (MagicType::ElementalMagic, MagicVariants::Elemental) => true,
        (_, MagicVariants::Any) => true,
        (_, _) => false,
    }
}
