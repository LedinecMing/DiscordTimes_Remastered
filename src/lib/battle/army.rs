use {
    std::{
        cell::RefCell,
        rc::Rc
    },
    crate::lib::{
        items::item::Item,
        battle::troop::Troop
    },
    crate::MutRc
};


#[derive(Debug)]
pub struct ArmyStats {
    pub gold: u64,
    pub mana: u64,
    pub army_name: String
}
#[derive(Debug)]
pub struct Army {
    pub troops: Vec<MutRc<Option<Troop>>>,
    pub stats: ArmyStats,
    pub inventory: Vec<Item>
}
const MAX_TROOPS: usize = 12;
impl Army {
    pub fn add_troop(&mut self, troop: Troop) {
        let index = self.troops.iter().position(|el| el.borrow().is_none() );
        match index {
            Some(index) => self.troops[index] = troop.into(),
            None => if self.troops.len() + 1 < MAX_TROOPS {
                self.troops.push(troop.into())
            }
        }
    }
    pub fn add_item(&mut self, item: Item) {
        self.inventory.push(item)
    }

    pub fn get_troop(&self, pos: usize) -> Option<MutRc<Option<Troop>>> {
        return match self.troops.get(pos) {
            Some(probably_troop_ref) => {
                if probably_troop_ref.borrow().as_ref().is_some() {
                    return Some(Rc::clone(probably_troop_ref))
                }
                else {
                    println!("В этом месте никого нет");
                    None
                }
            }
            None => {
                println!("Вы вышли за границы клеточек вашей армии");
                return None
            }
        }
    }

    pub fn new(mut troops: Vec<MutRc<Option<Troop>>>, stats: ArmyStats, inventory: Vec<Item>) -> Self {
        for _ in troops.len()..MAX_TROOPS {
            troops.push(Rc::new(RefCell::new(None)));
        };
        Self {
            troops,
            stats,
            inventory
        }
    }
}
impl From<Army> for MutRc<Army> {
    fn from(army: Army) -> Self {
        Rc::new(RefCell::new(army))
    }
}
