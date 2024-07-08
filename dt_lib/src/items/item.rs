use crate::units::unitstats::ModifyUnitStats;

use crate::{
    bonuses::bonus::Bonus,
    units::unit::{MagicType, Unit},
};
use advini::{Ini, IniParseError};
use alkahest::alkahest;
use once_cell::sync::Lazy;
use std::collections::HashMap;
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
}
#[derive(Debug, Clone, PartialEq)]
pub enum WeaponType {
    Hand,
    Ranged,
    Magic(MagicVariants),
}
#[derive(Clone, Debug)]
pub struct ItemInfo {
    pub name: String,
    pub description: String,
    pub cost: u64,
    pub icon: String,
    pub sells: bool,
    pub itemtype: ArtifactType,
    pub bonus: Option<Bonus>,
    pub modify: ModifyUnitStats,
}
pub static ITEMS: Lazy<Mutex<HashMap<usize, ItemInfo>>> = Lazy::new(|| Mutex::new(HashMap::new()));
#[derive(Clone, Copy, Debug)]
#[alkahest(Deserialize, Serialize, SerializeRef, Formula)]
pub struct Item {
    pub index: usize,
}
impl Ini for Item {
    fn eat<'a>(chars: std::str::Chars<'a>) -> Result<(Self, std::str::Chars<'a>), IniParseError> {
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
        ITEMS.lock().unwrap().get(&self.index).unwrap().clone()
    }
    pub fn can_equip(&self, unit: &Unit) -> bool {
        let info = self.get_info();
        match info.itemtype {
            ArtifactType::Item => false,
            ArtifactType::Amulet
            | ArtifactType::Armor
            | ArtifactType::Shield
            | ArtifactType::Ring
            | ArtifactType::Helmet => unit
                .inventory
                .items
                .iter()
                .filter(|item| item.is_some() && item.unwrap().get_info().itemtype == info.itemtype)
                .last()
                .is_none(),
            ArtifactType::Weapon(weapon_type) => {
                let damage = unit.modified.damage;
                (match weapon_type {
                    WeaponType::Hand => damage.hand > 0,
                    WeaponType::Ranged => damage.ranged > 0,
                    WeaponType::Magic(magic_dir) => {
                        damage.magic > 0 && magic_relates(unit.info.magic_type.unwrap(), magic_dir)
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
}
#[derive(Debug, Clone, PartialEq)]
pub enum MagicVariants {
    Any,
    Death,
    Life,
    Elemental,
}
pub fn magic_relates(magic_type: MagicType, magic_variant: MagicVariants) -> bool {
    match (magic_type, magic_variant) {
        (MagicType::Death(_), MagicVariants::Death) => true,
        (MagicType::Life(_), MagicVariants::Life) => true,
        (MagicType::Elemental(_), MagicVariants::Elemental) => true,
        (_, MagicVariants::Any) => true,
        (_, _) => false,
    }
}
