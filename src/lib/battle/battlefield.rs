use crate::{
    lib::{
        battle::{army::{Army, TroopType, MAX_TROOPS}, troop::Troop},
		map::map::GameMap,
        units::unit::{UnitPos}, items::item::Item,
    },
    State, Menu,
};
use std::cmp::Ordering::*;
use alkahest::alkahest;

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
pub fn troop_inactive(troop: &Troop) -> bool {
	troop.unit.modified.moves < 1 || troop.unit.is_dead()
}

#[derive(Clone, Default, Debug)]
#[alkahest(Deserialize, Serialize, SerializeRef, Formula)]
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
	pub fn new(gamemap: &mut GameMap, army1: usize, army2: usize) -> Self {
		let mut battle = BattleInfo {
			army1,
			army2,
			battle_ter: gamemap.armys[army2].building.unwrap_or(0),
			winner: None,
			..Default::default()
		};
		battle.start(gamemap);
		battle
	}
    pub fn start(&mut self, gamemap: &mut GameMap) {
        let army1 = &mut gamemap.armys[self.army1];

        army1.troops.iter_mut().for_each(|troop| {
            let unit = &mut troop.get().unit;
            let bonus = unit.get_bonus();
            bonus.on_battle_start(unit, &self);
            unit.bonus = bonus;
            unit.recalc();
        });
        let army2 = &mut gamemap.armys[self.army2];
        army2.troops.iter_mut().for_each(|troop| {
            let unit = &mut troop.get().unit;
            let bonus = unit.get_bonus();
            bonus.on_battle_start(unit, &self);
            unit.bonus = bonus;
            unit.recalc();
        });
		self.winner = None;
		self.active_unit = self.search_next_active(gamemap);
		self.can_interact = search_interactions(self, gamemap);
    }
	pub fn remove_corpses(&mut self, gamemap: &mut GameMap) {
		let army1 = &mut gamemap.armys[self.army1];
		remove_corpses(self, &mut army1.troops);
		army1.recalc_army_hitmap();
		let army2 = &mut gamemap.armys[self.army2];
		remove_corpses(self, &mut army2.troops);
		army2.recalc_army_hitmap();
	}
    pub fn search_next_active(&self, gamemap: &mut GameMap) -> Option<(usize, usize)> {
        let army1 = &gamemap.armys[self.army1];
        let army2 = &gamemap.armys[self.army2];
		if self.winner.is_some() {
			return None;
		}
		
        fn max_speed(troops: &Vec<TroopType>) -> (usize, &TroopType) {
            troops
                .iter()
                .enumerate()
                .max_by(|inf1, inf2| {
                    let (troop1, troop2) = (inf1.1.get(), inf2.1.get());
					let tr1_inactive = troop_inactive(&*troop1);
					let tr2_inactive = troop_inactive(&*troop2);
                    let res = if tr1_inactive {
                        Less
                    } else if tr2_inactive {
                        Greater
                    } else {
                        troop1.unit.modified.speed.cmp(&troop2.unit.modified.speed)
                    };
					res
                }).unwrap()
        }
		let troops = &army1.troops;
        let next1 = max_speed(&troops);
		let troops = &army2.troops;
        let next2 = max_speed(&troops);
        return {
            let (troop1, troop2) = (&next1.1.get(), &next2.1.get());
			
			let tr1_inactive = troop_inactive(&troop1);
			let tr2_inactive = troop_inactive(&troop2);
			if troop1.unit.modified.speed > troop2.unit.modified.speed
                 && !tr1_inactive {
				Some((troop1.unit.army, next1.0))
			 } else if !tr2_inactive {
				Some((troop2.unit.army, next2.0))
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
		fn trigger_end(gamemap: &mut GameMap, battle: &mut BattleInfo) {
			for troop in &mut gamemap.armys[battle.army1].troops {
				troop.get().on_battle_end();
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

pub fn search_interactions(battle: &mut BattleInfo, gamemap: &mut GameMap) -> Option<Vec<(usize, usize)>> {
    let army1 = &gamemap.armys[battle.army1];
    let army2 = &gamemap.armys[battle.army2];
    let mut can_interact = Vec::new();
    if let Some(active_unit) = battle.active_unit {
		let active_army = active_unit.0;
		
		let (tr1, tr2) = (&army1.troops, &army2.troops);
		let (troops1, troops2) = (&tr1, &tr2);//;(Army::get_army_slice(&mut tr1), Army::get_army_slice(&mut tr2));
		let active_troops = if active_army == battle.army1 {
			troops1
		} else {
			troops2
		};
		fn collect_interactions(active_unit: (usize, usize), active_troops: &Vec<TroopType>, troops: &Vec<TroopType>, army: usize, can_interact: &mut Vec<(usize, usize)>) {
			can_interact.append(&mut troops
				.iter()
				.enumerate()
				.map(|(i, troop)| {
					if i == active_unit.1 && active_unit.0 == army {
						return None;
					}
					let troop = &troop.get();
					let index = troop.pos.into();
					let unit = &troop.unit;
					let active_troop = &active_troops[active_unit.1].get();
					let active_unit_unit = &active_troop.unit;
					if active_unit_unit.can_attack(
						unit,
						troop.pos,
						active_troop.pos,
					) {
						Some((0, index))
					} else { None }
				}).filter_map(|e|
					e.and_then(|v| Some(v))
				).collect());
		}
		collect_interactions(active_unit, active_troops, troops1, battle.army1, &mut can_interact);
		collect_interactions(active_unit, active_troops, troops2, battle.army2, &mut can_interact);
		return Some(can_interact);
    }
    None
}
const MAX_MOVES: u64 = 25;
pub fn remove_corpses(battle: &mut BattleInfo, troops: &mut Vec<TroopType>) {
	let mut i = 0;
	loop {
		if troops[i].get().is_dead() {
			battle.dead.push(troops.remove(i));
			i -= 1;
		}
		if i + 1 >= troops.len() {
			break
		}
		i += 1;
	}
}
pub fn restore_moves(troops: &mut Vec<TroopType>) {
	for troop in troops {
		let unit = &mut troop.get().unit;
		unit.tick();
		unit.stats.moves = unit.modified.max_moves;
		unit.recalc()
	}
}
pub fn next_move(battle: &mut BattleInfo, gamemap: &mut GameMap) {
	if battle.winner.is_some() {
		//state.menu_id = Menu::Start as usize;
		battle.end(gamemap);
		return
	}
    let army1 = &mut gamemap.armys[battle.army1];
	restore_moves(&mut army1.troops);
	army1.recalc_army_hitmap();
    let army2 = &mut gamemap.armys[battle.army2];
	restore_moves(&mut army2.troops);
	army2.recalc_army_hitmap();
	battle.move_count += 1;
	check_win(battle, gamemap);
}
pub fn check_win(battle: &mut BattleInfo, gamemap: &mut GameMap) {
	fn check_army_win(army: &Army) -> bool {
		let troops = &army.troops;
		// If there are no more troops or all troops are ready to surrender
		troops.is_empty() ||
			troops.iter().all(|troop| {
				let troop = troop.get();
				troop.unit.info.surrender.is_some() || troop.is_dead()
			})
	}
	
	if battle.move_count == MAX_MOVES {
		battle.winner = Some(battle.army1);
	}
	if check_army_win(&gamemap.armys[battle.army1]) {
		battle.winner = Some(battle.army2);
	} else if check_army_win(&gamemap.armys[battle.army2]) {
		battle.winner = Some(battle.army1);
	}
}
pub fn check_row_fall(battle: &mut BattleInfo, gamemap: &mut GameMap) {
	fn check_row(army: &mut Army) -> bool {
		let max_troops = *MAX_TROOPS;
		let lower_row_empty = army.hitmap.iter().enumerate().skip(max_troops/2).all(|(i, hit)| hit.is_none() || field_type(i, max_troops) == Field::Reserve);
		lower_row_empty
	}
	fn row_fall(army: &mut Army) {
		let max_troops = *MAX_TROOPS;
		let falled = army.hitmap.iter().enumerate().take(max_troops/2).filter(|(i, hit)| hit.is_some() && field_type(*i, max_troops) != Field::Reserve).clone();
		{
			let troops = &army.troops;
			for (index, _) in falled {
				if let Some(mut troop) = troops.get(index).and_then(|t| Some(t.get())) {
					troop.pos = UnitPos::from_index(<UnitPos as Into<usize>>::into(troop.pos) + max_troops / 2);
				}
			}
		}
		army.recalc_army_hitmap();
	}
	
	for army in [battle.army1, battle.army2] {
		let army = &mut gamemap.armys[army];
		if check_row(army) {
			row_fall(army);
		}
	}
}
