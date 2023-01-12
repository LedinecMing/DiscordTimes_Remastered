use crate::lib:: {
    items::item::Item,
    battle::troop::Troop,
    mutrc::SendMut
};


#[derive(Clone, Debug)]
pub struct ArmyStats {
    pub gold: u64,
    pub mana: u64,
    pub army_name: String
}
#[derive(Clone, Debug)]
pub struct Army {
    pub troops: Vec<TroopType>,
    pub stats: ArmyStats,
    pub inventory: Vec<Item>,
    pub pos: [usize; 2]
}
pub type TroopType = SendMut<Option<Troop>>;
pub const MAX_TROOPS: usize = 12;
impl Army {
    pub fn add_troop(&mut self, troop: Troop) -> Result<(), ()> {
        let index = self.troops.iter().position(|el| el.get().as_ref().is_none() );
        match index {
            Some(index) => {
                self.troops[index] = SendMut::new(Some(troop));
                Ok(())
            },
            None => match self.troops.len() + 1 < MAX_TROOPS {
                true => {
                    self.troops.push(SendMut::new(Some(troop)));
                    Ok(())
                },
                false => return Err(())
        }   }
    }
    pub fn add_item(&mut self, item: Item) {
        self.inventory.push(item)
    }

    pub fn get_troop(&self, pos: usize) -> Option<TroopType> {
        return match self.troops.get(pos) {
            Some(probably_troop_ref) => {
                if probably_troop_ref.get().as_ref().is_some() {
                    return Some(probably_troop_ref.clone())
                } else {
                    None
                }
            }
            None => {
                return None
    }   }   }
    pub fn new(troops: Vec<TroopType>, stats: ArmyStats, inventory: Vec<Item>, pos: [usize; 2]) -> Self {
        let mut fixed_troops = troops;
        for _ in fixed_troops.len()..MAX_TROOPS {
            fixed_troops.push(SendMut::new(None));
        };
        Self {
            troops: fixed_troops,
            stats,
            inventory,
            pos
}   }   }
