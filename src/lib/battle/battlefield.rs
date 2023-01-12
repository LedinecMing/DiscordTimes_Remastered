use std::cmp::Ordering::*;
use crate::lib::battle::army::{Army, MAX_TROOPS, TroopType};
use crate::State;

#[derive(Clone, Debug)]
pub struct BattleInfo {
    pub army1: usize,
    pub army2: usize,
    pub battle_ter: usize,
    pub active_unit: Option<(usize, usize)>
}
impl BattleInfo {
    pub fn search_next_active(&self, state: &State) -> Option<(usize, usize)> {
        let army1 = &state.gamemap.armys[self.army1];
        let army2 = &state.gamemap.armys[self.army2];

        fn max_speed(army: &Army) -> Option<(&TroopType, usize)> {
            army.troops.iter().zip(0..MAX_TROOPS).max_by(|inf1, inf2| {
                let (temp1, temp2) = (inf1.0.get(), inf2.0.get());
                let (troop1, troop2) = (temp1.as_ref(), temp2.as_ref());
                if let Some(inf1) = troop1 {
                    if let Some(inf2) = troop2 {
                        if inf1.unit.stats.moves == 0 { Less }
                        else if inf2.unit.stats.moves == 0 { Greater }
                        else { inf1.unit.stats.speed.cmp(&inf2.unit.stats.speed) }
                    } else { Greater }
                } else {
                    if let Some(inf2) = troop2 { Less }
                    else { Equal }
                }
            })
        }
        let next1 = max_speed(army1);
        let next2 = max_speed(army2);
        return if let Some(inf1) = next1 {
            if let Some(inf2) = next2 {
                let (temp1, temp2) = (inf1.0.get(), inf2.0.get());
                let (troop1, troop2) = (temp1.as_ref().unwrap(), temp2.as_ref().unwrap());
                if troop1.unit.stats.speed > troop2.unit.stats.speed && troop1.unit.stats.moves != 0 {
                    Some((0, inf1.1))
                } else if troop2.unit.stats.moves != 0 {
                    Some((1, inf2.1))
                } else { None }
            } else {
                Some((0, inf1.1))
            }
        } else {
            if let Some(inf2) = next2 {
                Some((1, inf2.1))
            } else {
                None
            }
        };
}   }
