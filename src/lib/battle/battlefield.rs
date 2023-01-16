use crate::lib::battle::army::{Army, TroopType, MAX_TROOPS};
use crate::lib::mutrc::SendMut;
use crate::lib::units::unit::{Unit, UnitPos};
use crate::State;
use std::cmp::Ordering::*;

#[derive(Clone, Debug)]
pub struct BattleInfo {
    pub army1: usize,
    pub army2: usize,
    pub battle_ter: usize,
    pub active_unit: Option<(usize, usize)>,
    pub move_count: u64,
    pub can_interact: Option<Vec<(usize, usize)>>,
    pub dead: Vec<TroopType>,
}
impl BattleInfo {
    pub fn start(&mut self, state: &mut State) {
        let army1 = &mut state.gamemap.armys[self.army1];

        army1
            .troops
            .iter()
            .zip(0..*MAX_TROOPS.lock().unwrap())
            .for_each(|(troop, i)| {
                let mut troop = troop.get();
                if let Some(troop) = troop.as_mut() {
                    let unit = &mut troop.unit;
                    let bonus = unit.bonus.clone();
                    bonus.on_battle_start(unit);
                    unit.bonus = bonus;
                }
            });

        let army2 = &mut state.gamemap.armys[self.army2];
        army2
            .troops
            .iter()
            .zip(0..*MAX_TROOPS.lock().unwrap())
            .for_each(|(troop, i)| {
                let mut troop = troop.get();
                if let Some(troop) = troop.as_mut() {
                    let unit = &mut troop.unit;
                    let bonus = unit.bonus.clone();
                    bonus.on_battle_start(unit);
                    unit.bonus = bonus;
                }
            });
    }
    pub fn search_next_active(&self, state: &State) -> Option<(usize, usize)> {
        let army1 = &state.gamemap.armys[self.army1];
        let army2 = &state.gamemap.armys[self.army2];

        fn max_speed(army: &Army) -> Option<(&TroopType, usize)> {
            army.troops.iter().zip(0..*MAX_TROOPS.lock().unwrap()).max_by(|inf1, inf2| {
                let (temp1, temp2) = (inf1.0.get(), inf2.0.get());
                let (troop1, troop2) = (temp1.as_ref(), temp2.as_ref());
                if let Some(inf1) = troop1 {
                    if let Some(inf2) = troop2 {
                        if inf1.unit.stats.moves == 0 || inf1.unit.is_dead() {
                            Less
                        } else if inf2.unit.stats.moves == 0 || inf2.unit.is_dead() {
                            Greater
                        } else {
                            inf1.unit.stats.speed.cmp(&inf2.unit.stats.speed)
                        }
                    } else {
                        Greater
                    }
                } else {
                    if let Some(inf2) = troop2 {
                        Less
                    } else {
                        Equal
                    }
                }
            })
        }
        let next1 = max_speed(army1);
        let next2 = max_speed(army2);
        return if let Some(inf1) = next1 {
            if let Some(inf2) = next2 {
                let (temp1, temp2) = (inf1.0.get(), inf2.0.get());
                let (troop1, troop2) = (temp1.as_ref().unwrap(), temp2.as_ref().unwrap());
                if troop1.unit.stats.speed > troop2.unit.stats.speed
                    && troop1.unit.stats.moves != 0
                    && !troop1.unit.is_dead()
                {
                    Some((0, inf1.1))
                } else if troop2.unit.stats.moves != 0 && !troop1.unit.is_dead() {
                    Some((1, inf2.1))
                } else {
                    None
                }
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
    }
}

pub fn search_interactions(state: &mut State) -> Option<Vec<(usize, usize)>> {
    let battle = &mut state.battle;
    let army1 = &state.gamemap.armys[battle.army1];
    let army2 = &state.gamemap.armys[battle.army2];
    let mut can_interact = Vec::new();
    let max_troops = *MAX_TROOPS.lock().unwrap();
    if let Some(active_unit_pos) = battle.active_unit {
        let mut active_troop =
            state.gamemap.armys[active_unit_pos.0].troops[active_unit_pos.1].get();
        let mut active_unit = &active_troop.as_mut().unwrap().unit;
        army1
            .troops
            .iter()
            .zip(0..max_troops)
            .for_each(|(troop, i)| {
                if i == active_unit_pos.1 && active_unit_pos.0 == 0 {
                    return;
                }
                let mut troop = troop.get();
                if let Some(troop) = troop.as_mut() {
                    let unit = &mut troop.unit;
                    if active_unit.can_attack(
                        unit,
                        UnitPos::from_index(i),
                        UnitPos::from_index(active_unit_pos.1),
                    ) {
                        can_interact.push((0, i));
                    }
                }
            });
        army2
            .troops
            .iter()
            .zip(0..max_troops)
            .for_each(|(troop, i)| {
                if i == active_unit_pos.1 && active_unit_pos.0 == 1 {
                    return;
                }
                let mut troop = troop.get();
                if let Some(troop) = troop.as_mut() {
                    let unit = &mut troop.unit;
                    if active_unit.can_attack(
                        unit,
                        UnitPos::from_index(i),
                        UnitPos::from_index(active_unit_pos.1),
                    ) {
                        can_interact.push((1, i));
                    }
                }
            });
    }
    Some(can_interact)
}
pub fn next_move(state: &mut State) {
    let mut dead: Vec<TroopType> = Vec::new();
    let mut dead_index: Vec<usize> = Vec::new();
    let mut dead_counter = 0;
    {
        let mut army1 = &mut state.gamemap.armys[state.battle.army1];
        army1
            .troops
            .iter()
            .zip(0..*MAX_TROOPS.lock().unwrap())
            .for_each(|(troop, i)| {
                let mut troop = troop.get();
                if let Some(troop) = troop.as_mut() {
                    let unit = &mut troop.unit;
                    if unit.is_dead() {
                        dead_index.push(i);
                    } else {
                        unit.stats.moves = unit.stats.max_moves;
                        unit.tick();
                    }
                }
            });
        dead_index.iter().for_each(|i| {
            dead.push(army1.troops.remove(i - dead_counter));
            army1.troops.insert(i - dead_counter, SendMut::new(None));
            dead_counter += 1;
        });
    }

    {
        let mut army2 = &mut state.gamemap.armys[state.battle.army2];
        dead_counter = 0;
        dead_index = Vec::new();
        army2
            .troops
            .iter()
            .zip(0..*MAX_TROOPS.lock().unwrap())
            .for_each(|(troop, i)| {
                let mut troop = troop.get();
                if let Some(troop) = troop.as_mut() {
                    let unit = &mut troop.unit;
                    if unit.is_dead() {
                        dead_index.push(i);
                    } else {
                        unit.stats.moves = unit.stats.max_moves;
                        unit.tick();
                    }
                }
            });
        dead_index.iter().for_each(|i| {
            dead.push(army2.troops.remove(i - dead_counter));
            army2.troops.insert(i - dead_counter, SendMut::new(None));
            dead_counter += 1;
        });
    }
}
