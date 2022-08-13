use
{
    std::fmt::Debug,
    dyn_clone::DynClone,
    crate::lib::
    {
        bonuses::bonuses::*,
        battle::
        {
            army::Army,
            battlefield::BattleField,
        },
        effects::
        {
            effect::Effect,
            effects::MoreHealth
        },
        items::item::Item,
        bonuses::bonus::Bonus
    },
    derive_more::Add
};

fn nat(a: i32) -> i32
{
    if a >= 0
    {
        return a;
    }
    0
}

#[derive(Copy, Clone, Debug, Add)]
pub struct Defence
{
    pub magic_percent: i32,
    pub hand_percent: i32,
    pub ranged_percent: i32,
    pub magic_units: i32,
    pub hand_units: i32,
    pub ranged_units: i32
}
impl Defence
{
    pub fn empty() -> Self
    {
        Self
        {
            magic_percent: 0,
            hand_percent: 0,
            ranged_percent: 0,
            magic_units: 0,
            hand_units: 0,
            ranged_units: 0
        }
    }
}
#[derive(Copy, Clone, Debug, Add)]
pub struct Power
{
    pub magic: i32,
    pub ranged: i32,
    pub hand: i32
}
impl Power
{
    pub fn empty() -> Self
    {
        Self
        {
            magic: 0,
            ranged: 0,
            hand: 0
        }
    }

}

#[derive(Copy, Clone, Debug, Add)]
pub struct UnitStats
{
    pub hp: i32,
    pub max_hp: i32,
    pub damage: Power,
    pub defence: Defence,
    pub moves: i32,
    pub max_moves: i32,
    pub speed: i32,
}
impl UnitStats
{
    pub fn empty() -> Self
    {
        Self
        {
            hp: 0,
            max_hp: 0,
            damage: Power {
                magic: 0,
                ranged: 0,
                hand: 0
            },
            defence: Defence {
                magic_percent: 0,
                hand_percent: 0,
                ranged_percent: 0,
                magic_units: 0,
                hand_units: 0,
                ranged_units: 0
            },
            moves: 0,
            max_moves: 0,
            speed: 0
        }
    }
}
#[derive(Clone, Debug)]
pub struct UnitInfo
{
    pub name: String,
    pub cost: u64
}
impl UnitInfo
{
    pub fn empty() -> Self
    {
        Self
        {
            name: "".into(),
            cost: 0
        }
    }
}
#[derive(Clone, Debug)]
pub struct UnitInventory
{
    pub items: Vec<Item>
}

#[derive(Clone, Debug)]
pub struct UnitData
{
    pub stats: UnitStats,
    pub info: UnitInfo,
    pub inventory: UnitInventory,
    pub bonus: Box<dyn Bonus>,
    pub effects: Vec<Box<dyn Effect>>
}

dyn_clone::clone_trait_object!(Unit);
pub trait Unit : DynClone + Debug
{
    fn attack(&mut self, target: &mut dyn Unit, battle: &mut BattleField) -> bool;
    fn get_effected_stats(&self) -> UnitStats
    {
        let mut previous: UnitStats = self.get_data().stats.clone();
        let effects = &self.get_data().effects;
        effects.iter().for_each(|effect|
            {
                previous = effect.update_stats(previous);
            });
        let inventory = &self.get_data().inventory;
        inventory.items.iter().for_each(|item|
            {
                previous = item.effect.update_stats(previous);
            });
        previous
    }
    fn get_mut_data(&mut self) -> &mut UnitData;
    fn get_data(&self) -> &UnitData;
    fn get_bonus(&self) -> Box<dyn Bonus>;
    fn add_effect(&mut self, effect: Box<dyn Effect>) -> bool
    {
        self.get_mut_data().effects.push(effect);
        true
    }
    fn add_item(&mut self, item: Item) -> bool
    {
        self.get_mut_data().inventory.items.push(item);
        true
    }
    fn being_attacked(&mut self, damage: &Power, sender: &mut dyn Unit) -> i32;
    fn correct_damage(&self, damage: &Power) -> Power
    {
        let defence: Defence = self.get_effected_stats().defence;
        println!("Использую защиту {:?}", defence);
        Power {
            ranged: (nat(damage.ranged - defence.ranged_units) as f32 * (1.0 - defence.ranged_percent as f32 / 100.0)) as i32,
            magic: (nat(damage.magic-defence.magic_units) as f32 * (1.0 - defence.magic_percent as f32 / 100.0)) as i32,
            hand: (nat(damage.hand-defence.hand_units) as f32 * (1.0 - defence.hand_percent as f32 / 100.0)) as i32
        }
    }
    fn tick(&mut self) -> bool;
}
