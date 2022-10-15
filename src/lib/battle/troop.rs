use {
    std:: {
        fmt::{
            Display,
            Formatter
    }   },
    crate::lib:: {
        battle::army::{Army, TroopType},
        mutrc::MutRc,
        bonuses::bonuses::NoBonus,
        units:: {
            unit::*,
            units::Hand,
}  }  };

#[derive(Clone, Debug)]
pub struct Troop {
    pub was_payed: bool,
    pub is_free: bool,
    pub is_main: bool,
    pub custom_name: Option<String>,
    pub unit: Box<dyn Unit>
}

impl Troop {
    pub fn on_pay(&self, army: &mut Army) -> u64 {
        if self.is_free {
            return 0;
        }
        self.unit.get_info().cost
    }
    pub fn on_hour(&self, army: &mut Army) -> bool {
        true
    }
    pub fn is_dead(&self) -> bool {
        self.unit.is_dead()
    }
    pub fn empty() -> Self {
        Self {
            was_payed: true,
            is_free: false,
            is_main: false,
            custom_name: None,
            unit: Box::new(Hand::new(UnitData {
                stats: UnitStats::empty(),
                info: UnitInfo::empty(),
                inventory: UnitInventory::empty(),
                bonus: Box::new(NoBonus {}),
                effects: vec![]
            }))
}   }   }
impl Display for Troop {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let custom_name = match &self.custom_name {
            Some(name) => name.clone(),
            None => "".into()
        };
        let unitdata = self.unit.get_data();
        let unit_name = &unitdata.info.name;
        let name = format!("{}-{}", custom_name, unit_name);
        let stats = &self.unit.get_data().stats;
        write!(f, "| {name} |\n| {hp}/{maxhp}ХП ({is_dead}) |\n| {hand_attack} ближнего урона; {ranged_attack} дальнего урона; {magic_attack} магии |\n| {hand_def} ближней защиты.-{ranged_def} дальней защиты. |\n| Защита от магии: {magic_def_percent}%|\n|Регенерация {regen_percent}% |\n| Вампиризм {vamp_percent}% |\n| {speed} инициативы |\n| {moves}/{max_moves} ходов |", name=name,
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
               magic_def_percent = stats.defence.magic_percent,
               regen_percent = stats.regen,
               vamp_percent = stats.vamp,
               speed = stats.speed,
               moves = stats.moves,
               max_moves = stats.max_moves)
}   }
impl From<Troop> for TroopType {
    fn from(troop: Troop) -> Self {
        MutRc::new(Some(troop))
}   }
