use crate::{
    lib::{
        battle::troop::Troop,
        items::item::Item,
        map::map::{GameMap, MAP_SIZE},
        mutrc::SendMut,
    },
    TILES,
};
use num::{integer::sqrt, pow};
use once_cell::sync::Lazy;
use pathfinding::directed::astar::astar;
use std::iter::successors;
use tracing_mutex::stdsync::TracingMutex as Mutex;

#[derive(Clone, Debug, Default)]
pub struct Relations {
    player: u8,
    ally: u8,
    neighbour: u8,
    enemy: u8
}

#[derive(Clone, Debug)]
pub struct ArmyStats {
    pub gold: u64,
    pub mana: u64,
    pub army_name: String,
}
#[derive(Clone, Debug)]
pub struct Army {
    pub troops: Vec<TroopType>,
    pub stats: ArmyStats,
    pub inventory: Vec<Item>,
    pub pos: (usize, usize),
    pub path: Vec<(usize, usize)>,
}
pub type TroopType = SendMut<Option<Troop>>;
pub static MAX_TROOPS: Lazy<Mutex<usize>> = Lazy::new(|| Mutex::new(0));
impl Army {
    pub fn add_troop(&mut self, troop: Troop) -> Result<(), ()> {
        let index = self
            .troops
            .iter()
            .position(|el| el.get().as_ref().is_none());
        match index {
            Some(index) => {
                self.troops[index] = SendMut::new(Some(troop));
                Ok(())
            }
            None => match self.troops.len() + 1 < *MAX_TROOPS.lock().unwrap() {
                true => {
                    self.troops.push(SendMut::new(Some(troop)));
                    Ok(())
                }
                false => return Err(()),
            },
        }
    }
    pub fn add_item(&mut self, item: Item) {
        self.inventory.push(item)
    }

    pub fn get_troop(&self, pos: usize) -> Option<TroopType> {
        return match self.troops.get(pos) {
            Some(probably_troop_ref) => {
                if probably_troop_ref.get().as_ref().is_some() {
                    return Some(probably_troop_ref.clone());
                } else {
                    None
                }
            }
            None => return None,
        };
    }
    pub fn new(
        troops: Vec<TroopType>,
        stats: ArmyStats,
        inventory: Vec<Item>,
        pos: (usize, usize),
        path: Vec<(usize, usize)>,
    ) -> Self {
        let mut fixed_troops = troops;
        for _ in fixed_troops.len()..*MAX_TROOPS.lock().unwrap() {
            fixed_troops.push(SendMut::new(None));
        }
        Self {
            troops: fixed_troops,
            stats,
            inventory,
            pos,
            path,
        }
    }
}
fn dist(p1: &(usize, usize), p2: &(usize, usize)) -> u32 {
    sqrt((pow(p1.0 as isize - p2.0 as isize, 2) + pow(p1.1 as isize - p2.1 as isize, 2)) as u32)
}
pub fn find_path(
    gamemap: &GameMap,
    start: (usize, usize),
    goal: (usize, usize),
    on_transport: bool,
) -> Option<(Vec<(usize, usize)>, u32)> {
    let path = astar(
        &start,
        |&(x, y)| {
            let edge = (
                (x as f32 / (MAP_SIZE as f32 - 1.)),
                (y as f32 / (MAP_SIZE as f32 - 1.)),
            );
            match edge {
                (0., 0.) => vec![(x + 1, y), (x + 1, y + 1), (x, y + 1)],
                (0., 1.) => vec![(x + 1, y), (x + 1, y - 1), (x, y - 1)],
                (1., 1.) => vec![(x - 1, y), (x - 1, y - 1), (x, y - 1)],
                (1., 0.) => vec![(x - 1, y), (x - 1, y + 1), (x, y + 1)],
                (0., _) => vec![
                    (x + 1, y),
                    (x + 1, y + 1),
                    (x, y + 1),
                    (x + 1, y - 1),
                    (x, y - 1),
                ],
                (1., _) => vec![
                    (x - 1, y),
                    (x - 1, y + 1),
                    (x, y + 1),
                    (x - 1, y - 1),
                    (x, y - 1),
                ],
                (_, 0.) => vec![
                    (x - 1, y),
                    (x - 1, y + 1),
                    (x, y + 1),
                    (x + 1, y + 1),
                    (x + 1, y),
                ],
                (_, 1.) => vec![
                    (x - 1, y),
                    (x - 1, y - 1),
                    (x, y - 1),
                    (x + 1, y - 1),
                    (x + 1, y),
                ],
                (_, _) => vec![
                    (x - 1, y - 1),
                    (x, y - 1),
                    (x + 1, y - 1),
                    (x + 1, y),
                    (x + 1, y + 1),
                    (x, y + 1),
                    (x - 1, y + 1),
                    (x - 1, y),
                ],
            }
            .into_iter()
            .filter(|p: &(usize, usize)| {
                let hitbox = &gamemap.hitmap[p.0][p.1];
                hitbox.passable && !(hitbox.need_transport ^ on_transport)
            })
            .map(|p| (p, 10 / TILES[gamemap.tilemap[p.0][p.1]].walkspeed))
        },
        |&p| dist(&p, &goal),
        |&p| p == goal,
    )?;
    Some((path.0[1..].to_vec(), path.1))
}
