use crate::{
    lib::{
        battle::{army::{Army, TroopType, MAX_TROOPS}, troop::Troop},
		map::map::GameMap,
        mutrc::SendMut,
        units::unit::{Unit, UnitPos}, items::item::Item,
    },
    State,
};
use std::cmp::Ordering::*;

#[derive(Copy, Clone, Debug, PartialEq, PartialOrd)]
pub enum Field {
    Front,
    Back,
    Reserve,
}
pub fn field_type(index: usize, max_troops: usize) -> Field {
    match index {
        index if index == 0 || index == max_troops / 2 - 1 => Field::Reserve,
        index if index < max_troops / 2 => Field::Back,
        _ => Field::Front,
    }
}
#[derive(Clone, Default, Debug)]
pub struct BattleInfo {
    pub army1: usize,
    pub army2: usize,
    pub battle_ter: usize,
    pub active_unit: Option<(usize, usize)>,
    pub move_count: u64,
    pub can_interact: Option<Vec<(usize, usize)>>,
	pub winner: Option<usize>,
    pub dead: Vec<TroopType>,
}
impl BattleInfo {
	pub fn new(state: &mut State, army1: usize, army2: usize) -> Self {
		let mut battle = BattleInfo {
			army1,
			army2,
			battle_ter: state.gamemap.armys[army2].building.unwrap_or(0),
			winner: None,
			..Default::default()
		};
		battle.start(state);
		battle
	}
    pub fn start(&mut self, state: &mut State) {
        let army1 = &mut state.gamemap.armys[self.army1];

        army1.troops.iter().for_each(|troop| {
            let mut troop = troop.get();
            let unit = &mut troop.unit;
            let bonus = unit.get_bonus();
            bonus.on_battle_start(unit);
            unit.bonus = bonus;
            unit.recalc();
        });
        let army2 = &mut state.gamemap.armys[self.army2];
        army2.troops.iter().for_each(|troop| {
            let mut troop = troop.get();
            let unit = &mut troop.unit;
            let bonus = unit.get_bonus();
            bonus.on_battle_start(unit);
            unit.bonus = bonus;
            unit.recalc();
        });
		self.winner = None;
		self.active_unit = self.search_next_active(state);
		self.can_interact = search_interactions(state);
    }
    pub fn search_next_active(&self, state: &State) -> Option<(usize, usize)> {
        let army1 = &state.gamemap.armys[self.army1];
        let army2 = &state.gamemap.armys[self.army2];
		if state.battle.winner.is_some() {
			return None;
		}
		fn troop_inactive(troop: &Troop) -> bool {
			troop.unit.modified.moves == 0 || troop.unit.is_dead()
		}
		
        fn max_speed(army: &Army) -> (usize, &TroopType) {
            army.troops
                .iter()
                .enumerate()
                .max_by(|inf1, inf2| {
                    let (troop1, troop2) = (inf1.1.get(), inf2.1.get());
					
					let tr1_inactive = troop_inactive(&*troop1);
					let tr2_inactive = troop_inactive(&*troop2);
					
                    if tr1_inactive {
                        Less
                    } else if tr2_inactive {
                        Greater
                    } else {
                        troop1.unit.modified.speed.cmp(&troop2.unit.modified.speed)
                    }
                }).unwrap()
        }
        let next1 = max_speed(army1);
        let next2 = max_speed(army2);
        return {
            let (troop1, troop2) = (&next1.1.get(), &next2.1.get());
			let tr1_inactive = troop_inactive(&*troop1);
			let tr2_inactive = troop_inactive(&*troop2);
			if troop1.unit.modified.speed > troop2.unit.modified.speed
                && !tr1_inactive {
				Some((0, next1.0))
			} else if !tr2_inactive {
				Some((1, next2.0))
			} else {
                None
            }
        }
    }
	pub fn end(&mut self, gamemap: &mut GameMap) {
		fn restore_corpses(gamemap: &mut GameMap, battle: &mut BattleInfo, _winner: usize) {
			let mut corpses = Vec::new();
			corpses.append(&mut battle.dead);
			for dead in corpses {
				let army = dead.get().unit.army;
				gamemap.armys[army].add_troop(dead).ok();
			}
		}
		fn move_goods(gamemap: &mut GameMap, battle: &mut BattleInfo, winner: usize) -> (Vec<Item>, u64, u64){
			let loose = match winner {
				winner if winner == battle.army1 => battle.army2,
				_ => battle.army1
			};
			let mut items = Vec::new();
			items.append(&mut gamemap.armys[loose].inventory);
			gamemap.armys[winner].inventory.append(&mut items.clone());
			let gold = gamemap.armys[loose].stats.gold;
			gamemap.armys[loose].stats.gold = 0;
			gamemap.armys[winner].stats.gold += gold;
			let mana = gamemap.armys[loose].troops.iter().map(|troop| troop.get().unit.info.surrender).sum::<Option<u64>>().unwrap_or(0);
			gamemap.armys[winner].stats.mana += mana;
			{
				let army = &mut gamemap.armys[loose];
				army.defeated = true;
			}
			(items, gold, mana)
		}
		if let Some(winner) = self.winner {
			move_goods(gamemap, self, winner);
			restore_corpses(gamemap, self, winner);
		}
		/*
    // такстические стоимости (силы) армий
    CalkArmyCost(Army[1]); CalkArmyCost(Army[2]);
    // вычисляем полученный опыт исходя из тактических стоимостей
    For I:=1 to 2 do If Army[I].FirstMaxUnits>0 then begin
      // соотношение армий с учетом защиты замков
      dExp:=Army[3-I].FirstTacticCost/Army[I].FirstTacticCost;
      If dExp>=1
        then dExp:=1+(dExp-1)*ExpCorrection/100
        else dExp:=1-(1-dExp)*ExpCorrection/100;
      If dExp>4 then dExp:=4; If dExp<0.25 then dExp:=0.25;
      // тут считаем старый вариант расчета опыта
      If Army[I].FirstTacticCost>0 then begin
        NewExp:=(Army[3-I].FirstTacticCost-Army[3-I].TacticCost)*dExp;
        Army[I].OldNewExpirience:=Round(NewExp*MainExpCorrection/100);
      end else Army[I].OldNewExpirience:=0;
      // базовое значение опыта
      BaseExp:=Army[3-I].FirstTacticCost div 20;
      // учитываем сложность битвы исходя из потери хитов
      If Army[I].LostHit>0 then begin
        dExp:=Army[I].NormalLostHit/Army[I].LostHit;
        If dExp>3 then dExp:=3; If dExp<0.8 then dExp:=0.8;
        If Army[I].NormalLostHit=0 then begin
          dExp:=(Army[I].AllHit-Army[I].LostHit)/Army[I].AllHit;
          If dExp<0 then dExp:=0;
          NewExp:=BaseExp*dExp;
        end else NewExp:=BaseExp+Army[I].NormalLostHit*dExp+Army[I].TurnMaxHit
      end else NewExp:=BaseExp+Army[I].NormalLostHit*3+Army[I].TurnMaxHit;
      Army[I].NewExpirience:=Round(NewExp);
      // распределяем по всем учавствовавшим в бою персонажам в зависимости от их действий в бою
      If Army[I].MaxUnits>0 then For J:=1 to Army[I].MaxUnits do begin
        dExp:=0.25*Army[I].NewExpirience/Army[I].FirstMaxUnits;
        If (Army[I].Units[J].AllMov+Army[I].Units[J].MovFree)>0
          then dExp:=(4-Army[I].Units[J].Row)*dExp+Army[I].Units[J].Row*dExp*Army[I].Units[J].UseMov/(Army[I].Units[J].AllMov+Army[I].Units[J].MovFree);
        If dExp<0.5 then dExp:=1;
        Army[I].Units[J].NewExpirience:=Round(dExp);
      end;
    end;		
		 */
		
	} 
}

pub fn search_interactions(state: &mut State) -> Option<Vec<(usize, usize)>> {
    let battle = &mut state.battle;
    let army1 = &state.gamemap.armys[battle.army1];
    let army2 = &state.gamemap.armys[battle.army2];
    let mut can_interact = Vec::new();
    if let Some(active_unit) = battle.active_unit {
		let active_army = active_unit.0;
        let active_troop =
            &state.gamemap.armys[active_army].troops[active_unit.1];
		let active_index = active_troop.get().pos;
		let active_unit_unit = &active_troop.get().unit;
        army1
            .troops
            .iter()
            .enumerate()
            .for_each(|(i, troop)| {
                if i == active_unit.1 && active_unit.0 == 0 {
                    return;
                }
                let mut troop = troop.get();
				let index = troop.pos.into();
                let unit = &mut troop.unit;
                if active_unit_unit.can_attack(
                    unit,
                    UnitPos::from_index(index),
                    UnitPos::from_index(active_index.into()),
                ) {
                    can_interact.push((0, index));
                }
            });
        army2
            .troops
            .iter()
            .enumerate()
            .for_each(|(i, troop)| {
                if i == active_unit.1 && active_unit.0 == 1 {
                    return;
                }
                let mut troop = troop.get();
				let index = troop.pos.into();
                let unit = &mut troop.unit;
                if active_unit_unit.can_attack(
                    unit,
                    UnitPos::from_index(index),
                    UnitPos::from_index(active_index.into()),
                ) {
                    can_interact.push((1, index));
                }
            });
    }
    Some(can_interact)
}
const MAX_MOVES: u64 = 25;
pub fn next_move(state: &mut State) {
	if state.battle.winner.is_some() {
		state.battle.end(&mut state.gamemap);
		return
	}
	fn remove_corpses(battle: &mut BattleInfo, troops: &mut Vec<TroopType>) {
		for i in 0..troops.len() {
			if troops[i].get().is_dead() {
				battle.dead.push(troops.remove(i));
			}
			if i + 1 >= troops.len() {
				break
			}
		}
	}
	fn restore_moves(troops: &mut Vec<TroopType>) {
		for troop in troops {
			let unit = &mut troop.get().unit;
			unit.tick();
			unit.stats.moves = unit.modified.max_moves;
			unit.recalc()
		}
	}
	
    let army1 = &mut state.gamemap.armys[state.battle.army1];
	remove_corpses(&mut state.battle, &mut army1.troops);
	restore_moves(&mut army1.troops);
	army1.recalc_army_hitmap();
    let army2 = &mut state.gamemap.armys[state.battle.army2];
	remove_corpses(&mut state.battle, &mut army2.troops);
	restore_moves(&mut army2.troops);
	army2.recalc_army_hitmap();
	state.battle.move_count += 1;
	check_win(state);
}
pub fn check_win(state: &mut State) {
	fn check_army_win(state: &mut State, army: usize) -> bool {
		let army = &state.gamemap.armys[army];
		// If there are no more troops or all troops are ready to surrender
		army.troops.is_empty() ||
			army.troops.iter().all(|troop| troop.get().unit.info.surrender.is_some())
	}
	
	if state.battle.move_count == MAX_MOVES {
		state.battle.winner = Some(state.battle.army1);
	}
	if check_army_win(state, state.battle.army1) {
		state.battle.winner = Some(state.battle.army2);
	} else if check_army_win(state, state.battle.army2) {
		state.battle.winner = Some(state.battle.army1);
	}
	if state.battle.winner.is_some() {
		state.battle.end(&mut state.gamemap);
	}
}
pub fn check_row_fall(state: &mut State) {
	fn check_row(army: &mut Army) -> bool {
		let max_troops = *MAX_TROOPS;
		let lower_row_empty = army.hitmap.iter().enumerate().skip(max_troops/2).all(|(i, hit)| hit.is_none() || field_type(i, max_troops) == Field::Reserve);
		lower_row_empty
	}
	fn row_fall(army: &mut Army) {
		let max_troops = *MAX_TROOPS;
		let falled = army.hitmap.iter().enumerate().take(max_troops/2).filter(|(i, hit)| hit.is_some() && field_type(*i, max_troops) != Field::Reserve).clone();
		for (index, _) in falled {
			if let Some(troop) = army.get_troop(index) {
				troop.get().pos = UnitPos::from_index(index + max_troops / 2);
			}
		}
		army.recalc_army_hitmap();
	}
	
	for army in [state.battle.army1, state.battle.army2] {
		let army = &mut state.gamemap.armys[army];
		if check_row(army) {
			row_fall(army);
		}
	}
}
