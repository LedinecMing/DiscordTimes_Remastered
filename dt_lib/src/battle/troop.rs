use crate::units::unitstats::ModifyUnitStats;

use crate::{battle::army::Army, bonuses::Bonus, effects::effect::EffectTrait, units::unit::*};
use alkahest::alkahest;
use std::fmt::{Debug, Display, Formatter};

#[derive(Clone)]
#[alkahest(Deserialize, Serialize, SerializeRef, Formula)]
pub struct Troop {
    pub was_payed: bool,
    pub is_free: bool,
    pub is_main: bool,
    pub pos: UnitPos,
    pub custom_name: Option<String>,
    pub unit: Unit,
}
impl Debug for Troop {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Troop")
            .field("Name", &self.unit.info.name)
            .field("Army", &self.unit.army)
            .field("Pos", &self.pos)
            .finish_non_exhaustive()
    }
}
impl Troop {
    pub fn new(unit: Unit) -> Self {
        Troop {
            was_payed: true,
            is_free: false,
            is_main: false,
            pos: UnitPos::from_index(0),
            custom_name: None,
            unit,
        }
    }
    pub fn on_pay(&self, army: &mut Army) -> u64 {
        if self.is_free {
            return 0;
        }
        self.unit.info.cost
    }
    pub fn on_hour(&self, army: &mut Army) -> bool {
        true
    }
    pub fn on_battle_end(&mut self) {
        let unit = &mut self.unit;
        let mut i = 0;
        loop {
            if unit.effects[i].on_battle_end() && unit.effects[i].is_dead() {
                let mut effect = unit.effects.remove(i);
                effect.kill(unit);
                i -= 1;
            };
            if i + 1 >= unit.effects.len() {
                break;
            }
            i += 1;
        }
        unit.recalc();
    }
    pub fn is_dead(&self) -> bool {
        self.unit.is_dead()
    }
    pub fn empty() -> Self {
        Self {
            was_payed: true,
            is_free: false,
            is_main: false,
            pos: UnitPos::from_index(0),
            custom_name: None,
            unit: Unit {
                stats: UnitStats::empty(),
                modified: UnitStats::empty(),
                info: UnitInfo::empty(),
                lvl: UnitLvl::empty(),
                inventory: UnitInventory::empty(),
                army: 0,
                modify: ModifyUnitStats::default(),
                bonus: Bonus::NoBonus,
                effects: vec![],
            },
        }
    }
}
impl Display for Troop {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let custom_name = match &self.custom_name {
            Some(name) => name.clone(),
            None => "".into(),
        };
        let unitdata = &self.unit;
        let unit_name = &unitdata.info.name;
        let name = format!("{}-{}", custom_name, unit_name);
        let stats = &self.unit.modified;
        write!(f, "| {name} |\n| {hp}/{maxhp}ХП ({is_dead}) |\n| {hand_attack} ближнего урона; {ranged_attack} дальнего урона; {magic_attack} магии |\n| {hand_def} ближней защиты.-{ranged_def} дальней защиты. |\n| Защита от магии: {magic_def_percent}|\n|Регенерация {regen_percent}% |\n| Вампиризм {vamp_percent}% |\n| {speed} инициативы |\n| {moves}/{max_moves} ходов |", name=name,
               hp = stats.hp,
               maxhp = stats.max_hp,
               is_dead = match self.is_dead() {
                   true => "мёртв.",
                   false => "живой"
               },
               hand_attack = stats.damage.hand,
               ranged_attack = stats.damage.ranged,
               magic_attack = stats.damage.magic,
               hand_def = stats.defence.hand_units,
               ranged_def = stats.defence.ranged_units,
               magic_def_percent = format!("Магия смерти: {}\n Магия жизни: {}\nМагия стихий: {}", stats.defence.death_magic, stats.defence.life_magic, stats.defence.elemental_magic),
               regen_percent = stats.regen,
               vamp_percent = stats.vamp,
               speed = stats.speed,
               moves = stats.moves,
               max_moves = stats.max_moves)
    }
}
