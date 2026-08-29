use crate::{
    battle::{
        army::{Army, MAX_TROOPS, TroopType},
        troop::Troop,
    }, bonuses::{AbilityCondition, ListenTo}, effects::EffectInfo, items::item::Item, map::map::GameMap, network::net::*, registry::{self, GameInfo}, units::{
        unit::*,
        unitstats::{Modify, ModifyPower, ModifyUnitStats},
    }
};
use alkahest::{alkahest, serialize, serialized_size};
use itertools::Itertools;
use renet::DefaultChannel;
use std::{cmp::{Ordering::{self, *}, max_by, max_by_key}, collections::HashMap};

use super::{HitMap, MAX_LINES};

#[derive(Copy, Clone, Debug, PartialEq, PartialOrd)]
pub enum Field {
    Front,
    Back,
    Reserve,
}
pub fn field_type(index: usize, max_troops: usize) -> Field {
    match index {
        index
            if index == 0
                || index == max_troops / 2 - 1
                || index == max_troops / 2
                || index == max_troops - 1 =>
        {
            Field::Reserve
        }
        index if index < max_troops / 2 => Field::Back,
        _ => Field::Front,
    }
}
pub fn troop_inactive(troop: &Troop) -> bool {
    troop.unit.moves < 1 || troop.unit.is_dead()
}
#[derive(Clone, Copy, Default, PartialEq, Eq, Debug)]
#[alkahest(Deserialize, Serialize, SerializeRef, Formula)]
pub struct BattleUnit {
    pub army: usize,
    pub index: usize,
}
#[derive(Clone, Copy, Default, Debug, PartialEq, Eq)]
#[alkahest(Deserialize, Serialize, SerializeRef, Formula)]
pub struct BattleUnitPos {
    pub army: usize,
    pub pos: usize,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BattleUnitInfo {
    Pos(usize),
    Index(usize),
}
impl From<BattleUnit> for BattleUnitInfo {
    fn from(value: BattleUnit) -> Self {
        BattleUnitInfo::Index(value.index)
    }
}
impl From<BattleUnitPos> for BattleUnitInfo {
    fn from(value: BattleUnitPos) -> Self {
        BattleUnitInfo::Pos(value.pos)
    }
}
impl BattleUnitInfo {
    pub fn pos(&self, hitmap: &HitMap) -> Option<usize> {
        match self {
            BattleUnitInfo::Pos(v) => Some(*v),
            BattleUnitInfo::Index(v) => hitmap
                .iter()
                .enumerate()
                .find_map(|x| (*x.1 == Some(*v)).then(|| x.0)),
        }
    }
    pub fn index(&self, hitmap: &HitMap) -> Option<usize> {
        match self {
            BattleUnitInfo::Pos(v) => hitmap.get(*v).into_iter().flatten().next().cloned(),
            BattleUnitInfo::Index(v) => Some(*v),
        }
    }
    pub fn to_index(&self, hitmap: &HitMap, army: usize) -> Option<BattleUnit> {
        Some(match self {
            BattleUnitInfo::Pos(v) => {
                let Some(index) = hitmap.get(*v).unwrap_or(&None) else {
                    return None;
                };
                BattleUnit {
                    army,
                    index: *index,
                }
            }
            BattleUnitInfo::Index(v) => BattleUnit { army, index: *v },
        })
    }
    pub fn to_pos(&self, hitmap: &HitMap, army: usize) -> BattleUnitPos {
        match self {
            BattleUnitInfo::Index(v) => BattleUnitPos {
                army,
                pos: hitmap
                    .iter()
                    .enumerate()
                    .find(|x| *x.1 == Some(*v))
                    .unwrap_or((0, &None))
                    .1
                    .unwrap_or(0),
            },
            BattleUnitInfo::Pos(v) => BattleUnitPos { army, pos: *v },
        }
    }
}
#[derive(Clone, Default, Debug)]
#[alkahest(Deserialize, Serialize, SerializeRef, Formula)]
pub struct BattleInfo {
    pub army1: usize,
    pub army2: usize,
    pub armies_moved: [Vec<bool>; 2],
    pub battle_ter: usize,
    pub active_unit: Option<BattleUnit>,
	pub move_order: Vec<BattleUnit>,
    pub move_count: u64,
    pub can_interact: Option<Vec<BattleUnitPos>>,
	// Indexed by AbilityTroopCondition's num tag
	pub event_listeners: Vec<[Vec<BattleUnit>; 2]>,
    pub winner: Option<usize>,
    pub dead: Vec<TroopType>,
}
impl BattleInfo {
    pub fn new(armys: &mut Vec<Army>, army1: usize, army2: usize) -> Self {
        let mut battle = BattleInfo {
            army1,
            army2,
            armies_moved: [vec![], vec![]],
            battle_ter: armys[army2].building.unwrap_or(0),
            winner: None,
            ..Default::default()
        };
        battle
    }
    pub fn start(&mut self, armies: &mut Vec<Army>, registry: &GameInfo) {
        dbg!("Battle start!");
		// TODO BONUS START
        let army1 = &armies[self.army1];
        self.armies_moved[0] = [false].repeat(MAX_TROOPS);
        army1.troops.iter().enumerate().for_each(|(index, troop)| {
            let (bonus, pos) = {
				let troop = &mut troop.get();
				let unit = &mut troop.unit;
				unit.moves = unit.modified.max_moves;
				
				(unit.get_bonus(registry), troop.pos)
			};
			let unit_info = BattleUnit { army: self.army1, index };
			if let Some(bonus) = bonus {
				bonus.apply_rules(AbilityCondition::BattleStart, unit_info, pos.into(), armies, &vec![unit_info], &*self, registry);
			}
        });
        let army2 = &armies[self.army2];
        self.armies_moved[1] = [false].repeat(MAX_TROOPS);
		army2.troops.iter().enumerate().for_each(|(index, troop)| {
            let (bonus, pos) = {
				let troop = &mut troop.get();
				let unit = &mut troop.unit;
				unit.moves = unit.modified.max_moves;
				
				(unit.get_bonus(registry), troop.pos)
			};
			let unit_info = BattleUnit { army: self.army2, index };
			if let Some(bonus) = bonus {
				bonus.apply_rules(AbilityCondition::BattleStart, unit_info, pos.into(), armies, &vec![unit_info], &*self, registry);
			}
		});
        self.winner = None;
        self.after_single_move(armies, registry);
    }
	pub fn recalc_event_listeners(&mut self, armies: &Vec<Army>, registry: &GameInfo) {
		let mut event_listeners: Vec<[Vec<BattleUnit>; 2]> = (0..(AbilityCondition::Skips as u8)).map(|_| [vec!{}, vec!{}]).collect();
		for (army_index, army) in self.get_armies_refs(armies).iter().enumerate() {
			for (i, troop) in army.troops.iter().enumerate() {
				let troop = &troop.get();
				if troop_inactive(troop) { continue; }
				if let Some(bonus) = troop.unit.get_bonus(registry) {
					for (key, mechanics) in bonus.rules.iter() {
						let (listens_to, _) = mechanics;
						let armies: &[usize] = match listens_to {
							ListenTo::Myself => { continue; },
							ListenTo::Everyone => {
								&[0, 1]
							},
							ListenTo::MyArmy(my_army) => &[if *my_army { army_index } else { 1 - army_index } ]
						};
						let index = *key as usize;
						for &army in armies {
							let unit = BattleUnit { army, index: i };
							if !event_listeners[index][army].contains(&unit) {
								event_listeners[index][army].push(unit);
							}
						}
					};
				}
			}
		}
		self.event_listeners = event_listeners;
	}	
	pub fn get_armies_refs<'a>(&self, armies: &'a Vec<Army>) -> [&'a Army; 2] {
		[&armies[self.army1], &armies[self.army2]]
	}
	pub fn get_armies_mut<'a>(&self, armies: &'a mut Vec<Army>) -> Option<[&'a mut Army; 2]> {
		armies.get_disjoint_mut(self.get_army_indices()).ok()
	}
	pub fn get_army_indices(&self) -> [usize; 2] {
		[self.army1, self.army2]
	}
    pub fn recalc_hitmaps(&mut self, armies: &mut Vec<Army>, registry: &GameInfo) {
		for army in self.get_army_indices() {
			armies[army].recalc_army_hitmap(&registry.units);
		}
    }
    pub fn search_next_active(&mut self, armies: &Vec<Army>) -> Option<BattleUnit> {
        let army1 = &armies[self.army1];
        let army2 = &armies[self.army2];

        fn get_move_order(tr: &Vec<TroopType>, tr1: &Vec<TroopType>) -> Vec<BattleUnit> {
			tr.iter().enumerate().map(|x| (x.0, x.1, 0)).chain(tr1.iter().enumerate().map(|x| (x.0, x.1, 1)))
				.filter(|tr| !troop_inactive(&tr.1.get()))
				.sorted_by(|(tr_index, tr, tr_army), (tr1_index, tr1, tr1_army)| {
					let (troop1, troop2) = (tr.get(), tr1.get());
                    troop1
                        .unit
                        .modified
                        .speed
                        .cmp(&troop2.unit.modified.speed)
                        .then(tr_index.cmp(tr1_index).reverse())
                        .then(tr_army.cmp(tr1_army).reverse())
                        .reverse()
				})
				.map(|tr| BattleUnit { army: tr.2, index: tr.0 })
				.collect()
        }
		self.move_order = get_move_order(&army1.troops, &army2.troops);
		self.active_unit = self.move_order.first().cloned();
		self.active_unit.clone()
    }
    pub fn end(&mut self, armies: &mut Vec<Army>, registry: &GameInfo) {
        fn trigger_end(armys: &mut Vec<Army>, battle: &mut BattleInfo, registry: &GameInfo) {
            for troop in &mut armys[battle.army1].troops {
                troop.get().on_battle_end(registry);
            }
        }
        fn move_goods(
            armys: &mut Vec<Army>,
            battle: &mut BattleInfo,
            winner: usize,
			registry: &GameInfo
        ) -> (Vec<Item>, u64, u64) {
            let loose = match winner {
                winner if winner == battle.army1 => battle.army2,
                _ => battle.army1,
            };
            let mut items = Vec::new();
            items.append(&mut armys[loose].inventory);
            armys[winner].inventory.append(&mut items.clone());
            let gold = armys[loose].stats.gold;
            armys[loose].stats.gold = 0;
            armys[winner].stats.gold += gold;
            let mana = armys[loose]
                .troops
                .iter()
                .map(|troop| troop.get().unit.get_info(&registry.units).surrender)
                .sum::<Option<u64>>()
                .unwrap_or(0);
            armys[winner].stats.mana += mana;
            {
                let army = &mut armys[loose];
                army.defeated = true;
            }
            (vec![], gold, mana)
        }
        if let Some(winner) = self.winner {
            move_goods(armies, self, winner, registry);
        }
        trigger_end(armies, self, registry);
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
          // базоzвое значение опыта
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
    pub fn next_move_seq(&mut self, armies: &mut Vec<Army>, registry: &GameInfo) {
		self.armies_moved[0] = [false].repeat(registry.game_settings.max_troops);
		self.armies_moved[1] = [false].repeat(registry.game_settings.max_troops);
		let per_army = |armies: &mut Vec<Army>, army_num: usize| {
			let mut troops = armies[army_num].troops.iter().enumerate();
			for (index, troop) in troops {
				let mut troop = troop.get();
				if troop.is_dead() {
					continue
				}
				let pos = troop.pos;
				troop.unit.tick(registry);
				if let Some(bonus) = troop.unit.get_bonus(registry).clone() {
					bonus.apply_rules(AbilityCondition::Turn, BattleUnit { index, army: army_num }, pos.whole(), armies, &vec![], &self, registry);
				};
			}
			let mut troops = &mut armies[army_num].troops;
			lower_magic(troops, registry);
			for mut troop in troops.iter_mut().filter_map(|troop| {
				let troop = troop.get();
				if troop.is_dead() {
					None
				} else {
					Some(troop)
				}
			}) {
				let unit = &mut troop.unit;
				unit.tick(registry);
				unit.moves = unit.modified.max_moves;
				unit.recalc(registry)
			}
		};
		per_army(armies, self.army1);
		per_army(armies, self.army2);
        self.move_count += 1;
        check_win(self, armies, registry);
    }
    pub fn after_single_move(&mut self, armies: &mut Vec<Army>, registry: &GameInfo) {
        self.recalc_hitmaps(armies, registry);
        check_win(self, &armies, registry);
        check_row_fall(self, armies, registry);
		self.search_next_active(&armies);
        if self.winner.is_none() {
			if let Some(active_unit) = self.active_unit {
				self.can_interact = Some(search_interactions(self, active_unit, &armies, registry));
			} else {
                self.next_move_seq(armies, registry);
                self.search_next_active(&armies);
            }
        } else {
            self.end(armies, registry);
        }
    }
}

pub fn build_game_tree() {
	/*
	[Action] | [Move -> Action]
	[Reserve]

	[Dangers]
	attack_lines
	
	 */
}
struct AttackLine {
	// who can attack
	pub attacks: Vec<usize>
}
pub fn evaluate_position(battle: &mut BattleInfo, armies: &mut Vec<Army>, my_army: usize) {

}
pub fn search_interactions(
    battle: &mut BattleInfo,
	active_unit: BattleUnit,
    armies: &Vec<Army>,
	registry: &GameInfo,
) -> Vec<BattleUnitPos> {
    let army1 = &armies[battle.army1];
    let army2 = &armies[battle.army2];
    let mut can_interact = Vec::new();
    
    let active_army = active_unit.army;
    let (tr1, tr2) = (&army1.troops, &army2.troops);
    let (troops1, troops2) = (&tr1, &tr2);
    let active_troops = if active_army == battle.army1 {
        troops1
    } else {
        troops2
    };
    let active_pos: usize = { active_troops[active_unit.index].get().pos.into() };
    fn collect_interactions(
        active_unit: BattleUnit,
        active_troops: &Vec<TroopType>,
        troops: &Vec<TroopType>,
        hitmap: &HitMap,
        army: usize,
        active_pos: usize,
        can_interact: &mut Vec<BattleUnitPos>,
		registry: &GameInfo,
    ) {
        can_interact.append(
            &mut troops
                .iter()
                .enumerate()
                .map(|(i, troop)| {
                    if i == active_unit.index && active_unit.army == army {
                        return Some(BattleUnitPos {
                            army,
                            pos: active_pos,
                        });
                    }
                    let troop = &troop.get();
                    let pos = troop.pos.into();
                    let unit = &troop.unit;
                    let active_troop = &active_troops[active_unit.index].get();
                    let active_unit_unit = &active_troop.unit;
                    let is_enemy = active_unit.army != army;
                    if active_unit_unit.can_attack(
                        unit,
                        troop.pos,
                        active_troop.pos,
                        is_enemy,
                        hitmap,
						registry
                    ) {
                        Some(BattleUnitPos { army, pos })
                    } else {
                        None
                    }
                })
                .filter_map(|e| e.and_then(|v| Some(v)))
                .collect(),
        );
    }
    collect_interactions(
        active_unit,
        active_troops,
        troops1,
        &army1.hitmap,
        battle.army1,
        active_pos,
        &mut can_interact,
		registry,
    );
    collect_interactions(
        active_unit,
        active_troops,
        troops2,
        &army2.hitmap,
        battle.army2,
        active_pos,
        &mut can_interact,
		registry,
    );
    return can_interact;
}
const MAX_MOVES: u64 = 25;
pub fn lower_magic(troops: &mut Vec<TroopType>, registry: &GameInfo) {
    for mut troop in troops.iter_mut().filter_map(|troop| {
        let troop = troop.get();
        if troop.is_dead() {
            None
        } else {
            Some(troop)
        }
    }) {
        let unit = &mut troop.unit;
		let unit_info = unit.get_info(&registry.units);
        let minus = match unit_info.magic_type {
            Some(MagicType::Death | MagicType::Life) => -2,
            Some(_) => -5,
            None => {
                continue;
            }
        };
		// TODO
        // unit.add_effect(ToEndEffect {
        //     info: EffectInfo { lifetime: 1 },
        //     modify: ModifyUnitStats {
        //         damage: ModifyPower {
        //             magic: *Modify::default().add_val(minus),
        //             ..Default::default()
        //         },
        //         ..Default::default()
        //     },
        //});
    }
}
pub fn restore_moves(troops: &mut Vec<TroopType>, registry: &GameInfo) {
    for mut troop in troops.iter_mut().filter_map(|troop| {
        let troop = troop.get();
        if troop.is_dead() {
            None
        } else {
            Some(troop)
        }
    }) {
        let unit = &mut troop.unit;
        unit.tick(registry);
        unit.moves = unit.modified.max_moves;
        unit.recalc(registry)
    }
}
pub fn check_win(battle: &mut BattleInfo, armys: &Vec<Army>, registry: &GameInfo) {
    fn check_army_win(army: &Army, registry: &GameInfo) -> bool {
        let troops = &army.troops;
        // If there are no more troops or all troops are ready to surrender
        troops.is_empty()
            || troops.iter().all(|troop| {
                let troop = troop.get();
                troop.unit.get_info(&registry.units).surrender.is_some() || troop.is_dead()
            })
    }

    if battle.move_count == MAX_MOVES {
        battle.winner = Some(battle.army1);
    }
    if check_army_win(&armys[battle.army1], registry) {
        battle.winner = Some(battle.army2);
    } else if check_army_win(&armys[battle.army2], registry) {
        battle.winner = Some(battle.army1);
    }
}
pub fn check_row_fall(battle: &mut BattleInfo, armys: &mut Vec<Army>, registry: &GameInfo) {
    fn check_row(army: &mut Army) -> (bool, bool) {
        let max_troops = MAX_TROOPS;
        let lower_row_empty = army
            .hitmap
            .iter()
            .enumerate()
            .skip(max_troops / 2)
            .all(|(i, hit)| hit.is_none() || field_type(i, max_troops) == Field::Reserve);
        let upper_row_empty = army
            .hitmap
            .iter()
            .enumerate()
            .take(max_troops / 2)
            .all(|(i, hit)| hit.is_none() || field_type(i, max_troops) == Field::Reserve);
        (lower_row_empty, upper_row_empty)
    }
    fn row_fall(army: &mut Army, registry: &GameInfo) {
        let max_troops = MAX_TROOPS;
        let fell = army
            .hitmap
            .iter()
            .enumerate()
            .take(max_troops / 2)
            .dedup_by(|x, y| x.1 == y.1)
            .filter_map(|(i, hit)| {
                if hit.is_some() && field_type(i, max_troops) != Field::Reserve {
                    *hit
                } else {
                    None
                }
            })
            .clone();
        {
            let troops = &army.troops;
            for troop_index in fell {
                if let Some(mut troop) = troops.get(troop_index).and_then(|t| Some(t.get())) {
                    troop.pos = UnitPos::from_index(
                        <UnitPos as Into<usize>>::into(troop.pos) + max_troops / 2,
                    );
                }
            }
        }
        army.recalc_army_hitmap(&registry.units);
    }
    fn reserve_fall(army: &mut Army) {
        let max_troops = MAX_TROOPS;
        let fell = army
            .hitmap
            .iter()
            .enumerate()
            .filter_map(|(i, hit)| {
                if field_type(i, max_troops) == Field::Reserve {
                    *hit
                } else {
                    None
                }
            })
            .clone();
        for (i, troop_index) in fell.enumerate() {
            if let Some(mut troop) = army.troops.get(troop_index).and_then(|t| Some(t.get())) {
                troop.pos = UnitPos::from_index(max_troops / 2 + 1 + i);
            }
        }
    }
    for army in [battle.army1, battle.army2] {
        let army = &mut armys[army];
        let (lower, higher) = check_row(army);
        match (lower, higher) {
            (true, false) => row_fall(army, registry),
            (true, true) => reserve_fall(army),
            _ => (),
        }
    }
}

pub fn possible_movement(from: usize) -> Vec<usize> {
    let half = MAX_TROOPS / MAX_LINES;
    let from = from % half;
    let mut possible = vec![];
    possible.push(0);
    possible.push(half - 1);
    possible.push(half);
    possible.push(half * 2 - 1);
    for i in 0..=1 {
        for j in 0..=2 {
            possible.push(from.abs_diff(1 - j) + i * half);
        }
    }
    possible.dedup();
    possible
}

fn unit_interaction(
    battle: &mut BattleInfo,
    armies: &mut Vec<Army>,
    (army, info): (usize, BattleUnitInfo),
	registry: &GameInfo,
) -> Option<ActionResult> {
    let mut action_result = None;

    let Some(active_unit) = battle.active_unit else {
        return None;
    };
    let Some(pos) = info.pos(&armies[army].hitmap) else {
        return None;
    };
    // move unit
    let Some(target_index) = info.index(&armies[army].hitmap) else {
        let army_index = if active_unit.army == battle.army1 {
            0
        } else {
            1
        };
        if army == active_unit.army {
            let active_army = &mut armies[active_unit.army];
            let troop = &mut active_army.troops[active_unit.index].get();
            let old_pos = troop.pos;
            let npos = UnitPos::from_index(pos);
            if !Army::fit_to(
                &active_army.hitmap,
                troop.unit.get_info(&registry.units).size,
                MAX_TROOPS / 2,
                MAX_LINES,
                npos.1,
                npos.0,
            ) {
                return None;
            }
            if !(old_pos.0.abs_diff(npos.0) < 2
                || field_type(pos, MAX_TROOPS) == Field::Reserve
                || field_type(old_pos.whole(), MAX_TROOPS) == Field::Reserve)
            {
                return None;
            }
            if field_type(pos, MAX_TROOPS) == Field::Reserve
                || field_type(old_pos.whole(), MAX_TROOPS) == Field::Reserve
			{
				// fix
                if !battle.armies_moved[army_index][active_unit.index] {
                    battle.armies_moved[army_index][active_unit.index] = true;
                } else {
                    return None;
                }
            }
            troop.unit.moves -= 1;
            troop.pos = npos;
            return Some(ActionResult::Move);
        } else {
            return None;
        }
    };

    // skip move
    if active_unit.army == army && target_index == active_unit.index {
        let active_army = &mut armies[active_unit.army];
        let Some(mut troop) = active_army
            .troops
            .get(active_unit.index)
            .and_then(|x| Some(x.get()))
        else {
            return None;
        };
        let active_unit_pos = troop.pos;
        let unit = &mut troop.unit;
		let unit_info = unit.get_info(&registry.units);
		let unit_type = unit_info.unit_type;
		let unit_magic = unit_info.magic_type;
        if let Some(magic_type) = unit_magic {
            let damage = unit.modified.damage;
            if damage.magic > 0
                && matches!(
                    unit_info.magic_direction,
                    MagicDirection::ToAlly | MagicDirection::ToAll | MagicDirection::CureOnly
                )
            {
                match magic_type {
                    MagicType::Death | MagicType::Life => heal_bless(unit, damage, magic_type, unit_type, registry),
                    MagicType::Elemental => elemental_bless(unit, damage, unit_type, unit_magic, registry),
                };
            }
        }
		// TODO
        //unit.get_bonus(registry).on_move_skip(unit);
        unit.moves -= 1;
    } else {
        // let (action_result, size, target_is_dead) = {
        //     let active_army = &armies[active_unit.army];
        //     let Some(mut troop) = active_army
        //         .troops
        //         .get(active_unit.index)
        //         .and_then(|x| Some(x.get()))
        //     else {
        //         return None;
        //     };
        //     let active_unit_pos = troop.pos;
        //     let unit = &mut troop.unit;
        //     let target_army = &armies[army];
        //     let target_troops = &target_army.troops;
        //     let target_hitmap = &target_army.hitmap;
        //     let the_troops = target_troops.get(target_index);
        //     let Some(target_troop) = the_troops else {
        //         return None;
        //     };
        //     let mut target_troop = target_troop.get();
        //     let unit2 = &mut target_troop.unit;
		// 	todo!()
		// 	// TODO
        //     // let res = if unit.get_bonus(registry) != Bonus::ManyTargets {
        //     //     unit.attack(
        //     //         unit2,
        //     //         UnitPos::from_index(pos),
        //     //         active_unit_pos,
        //     //         &battle,
        //     //         target_hitmap,
        //     //         active_unit.army != army,
        //     //     )
        //     // } else {
        //     //     let targets = [0, 2];
        //     //     let targets: Vec<_> = targets
        //     //         .iter()
        //     //         .filter_map(|diff| {
        //     //             let new = pos + diff - 1;
        //     //             if field_type(new, MAX_TROOPS) != Field::Reserve {
        //     //                 if let Some(index) = target_army.hitmap[new] {
        //     //                     Some((new, index))
        //     //                 } else {
        //     //                     None
        //     //                 }
        //     //             } else {
        //     //                 None
        //     //             }
        //     //         })
        //     //         .collect();
        //     //     let target_amount = targets.len() + 1;
        //     //     let mut damage = unit.modified.damage;
        //     //     damage.hand /= target_amount as u64;
        //     //     damage.ranged /= target_amount as u64;
        //     //     damage.magic /= target_amount as u64;
        //     //     unit2.being_attacked(
        //     //         &damage,
        //     //         unit,
        //     //         UnitPos::from_index(pos),
        //     //         target_hitmap,
        //     //         active_unit_pos,
        //     //         &battle,
        //     //     );
        //     //     for (target_pos, target_index) in targets {
        //     //         let unit2 = &mut target_troops[target_index].get().unit;
        //     //         unit2.being_attacked(
        //     //             &damage,
        //     //             unit,
        //     //             UnitPos::from_index(target_pos),
        //     //             target_hitmap,
        //     //             active_unit_pos,
        //     //             &battle,
        //     //         );
        //     //     }
        //     //     Some(ActionResult::Ranged)
        //     // };
        //     // let res = res?;
        //     // unit.moves -= 1;
        //     // (res, unit2.info.size, unit2.is_dead())
        // };
        // if target_is_dead {}
    };
    action_result
}

/// A method that processes an action with the given battle and gamemap, action is done by currently active unit, it will return None if action is impossible due to game rules.
pub fn handle_action(
    (pos, army): (usize, usize),
    battle: &mut BattleInfo,
    armies: &mut Vec<Army>,
	registry: &GameInfo,
) -> Option<(ActionResult, BattleUnit)> {
    if let Some(_) = battle.winner {
        return None;
    }
    let active = battle.active_unit;
    let res = unit_interaction(battle, armies, (army, BattleUnitInfo::Pos(pos)), registry);
    battle.after_single_move(armies, registry);
    res.and_then(|v| Some((v, active.unwrap())))
}

/// Used for processing an action in a context of using a server
pub fn handle_server_action(connection: &mut Option<ConnectionManager>, action: (usize, usize), registry: &GameInfo) {
    let Some(connection) = connection else {
        return;
    };
    match &mut connection.con {
        Connection::Client(con) => {
            let message = ClientMessage::Action(action);
            let size = serialized_size::<ClientMessage, _>(&message);
            let mut output = vec![0u8; size.0];
            serialize::<ClientMessage, ClientMessage>(message, &mut output).ok();
            con.client.send_message(
                renet::DefaultChannel::ReliableOrdered,
                renet::Bytes::copy_from_slice(&output),
            );
        }
        Connection::Host(server) => {
            let Some(army) = server.auth.get(&HOST_CLIENT_ID) else {
                return;
            };
            let Some(battle) = &mut connection.battle else {
                return;
            };
            if Some(army) != battle.active_unit.and_then(|v| Some(v.army)).as_ref() {
                return;
            }
            handle_action(action, battle, &mut connection.gamemap.armys, registry);
            let message = ServerMessage::State((Some(battle.clone()), connection.gamemap.clone()));
            let size = serialized_size::<ServerMessage, _>(&message);
            let mut output = vec![0u8; size.0];
            serialize::<ServerMessage, ServerMessage>(message, &mut output).ok();
            server.server.broadcast_message_except(
                HOST_CLIENT_ID,
                DefaultChannel::ReliableOrdered,
                renet::Bytes::copy_from_slice(&output),
            );
        }
    };
}

#[cfg(test)]
mod tests {
    use crate::{
        battle::{self, ArmyStats}, mutrc::SendMut, parse::{StupidReader, parse_items, parse_units}, registry::Registry, units::unitstats::ModifyUnitStats
    };
    use rand::{seq::IteratorRandom, thread_rng, Rng};

    use super::*;
	
    fn get_unit(id: usize, moves: i64, speed: i64, army: usize, registry: &GameInfo) -> Unit {
		let info = UnitInfo {
			id,
			str_id: "".into(),
            name: "".into(),
            descript: "".into(),
            cost: 0,
            cost_hire: 0,
            icon_index: 0,
            size: (1, 1),
            unit_type: UnitType::People,
            next_unit: Vec::new(),
            magic_type: None,
			magic_direction: Default::default(),
            surrender: None,
            lvl: LevelUpInfo::default(),

			stats: UnitStats {
                speed,
                hp: 100,
                max_hp: 100,
                moves,
                max_moves: moves,
                ..Default::default()
            },
        };

        let mut unit = Unit {
			id,
			hp: 100,
			moves,
            bonus: todo!(),
            modified: UnitStats {
                speed,
                ..Default::default()
            },
            modify: ModifyUnitStats::default(),
            effects: Vec::new(),
            lvl: UnitLvl::default(),
            inventory: UnitInventory::default(),
        };
        unit.recalc(registry);
        unit
    }
    fn gen_army(army_num: usize, registry: &GameInfo) -> Army {
        let mut army = Army::new(
            vec![],
            ArmyStats {
                gold: 0,
                mana: 0,
                army_name: String::new(),
            },
            vec![],
            (0, 0),
            true,
            crate::battle::control::Control::PC,
        );
        for _ in 0..10 {
            army.add_troop(
                Troop::new(get_unit(
					0,
                    thread_rng().gen_range(1..=3),
                    thread_rng().gen_range(1..10),
                    army_num,
					registry
                ))
                .into(),
            )
            .ok();
        }
        army
    }
    fn gen_army_from_units(army_num: usize, units: &Vec<Unit>) -> Army {
        let mut army = Army::new(
            vec![],
            ArmyStats {
                gold: 0,
                mana: 0,
                army_name: String::new(),
            },
            vec![],
            (0, 0),
            true,
            crate::battle::control::Control::PC,
        );
        for _ in 0..10 {
            army.add_troop(
                Troop::new({
                    let mut unit = units.iter().choose(&mut thread_rng()).unwrap().clone();
                    unit
                })
                .into(),
            )
            .ok();
        }
        army
    }
    #[test]
    fn selecting_active() {
        fn test1(registry: &GameInfo) {
            let army = gen_army(0, registry);
            army.troops;
        }
		let registry = todo!();
        for iteration in 0..100 {
            let army1 = gen_army(0, registry);
            let army2 = gen_army(1, registry);
            let mut armies = vec![army1, army2];
            let mut battle = BattleInfo::new(&mut armies, 0, 1);

            let mut been = vec![];
            while let Some(active_unit) = battle.search_next_active(&armies) {
                if !been.contains(&active_unit) {
                    been.push(active_unit);
                }
                let troop = &mut armies[active_unit.army].troops[active_unit.index].get();
                assert!(!troop_inactive(troop));
                dbg!(troop.unit.modified.moves);
                troop.unit.moves -= 1;
                troop.unit.recalc(registry);
            }
            dbg!(&been);
            let gen_expectations = |a| (0..10).map(move |v| BattleUnit { army: a, index: v });
            let mut expected = gen_expectations(0).chain(gen_expectations(1));
            let left_out = expected.filter(|v| !been.contains(&v)).collect::<Vec<_>>();
            if !left_out.is_empty() {
                dbg!(iteration);
                for unit in left_out {
                    let troop = &armies[unit.army].troops[unit.index].get();
                    let unit = &troop.unit;
                }
                panic!("God damn it!");
            }
        }
    }
    #[test]
    fn process_battles() {
        // for entry in std::fs::read_dir(".").unwrap() {
        // 	dbg!(entry);
        // }
        // let res = parse_units::<StupidReader>(Some("../dt/Units.ini"));
        // let Ok((units, _)) = res.await else {
        //     panic!("Unit parsing error")
        // };
        // let _ = parse_items::<StupidReader>(Some("../dt/Rus_Artefacts.ini"), &"Rus".into());
        // for _ in 0..=1 {
        // 	break;
        //     let army1 = gen_army_from_units(0, &units);
        //     let army2 = gen_army_from_units(1, &units);
        //     let mut armys = vec![army1, army2];
        //     let mut battle = BattleInfo::new(&mut armys, 0, 1);
        //     while battle.winner.is_none() {
        //         if let Some(interactions) = &battle.can_interact.clone() {
        //             if let Some(interaction) = interactions.iter().choose(&mut thread_rng()) {
        // 				unit_interaction(&mut battle, &mut armys, (interaction.army, BattleUnitInfo::Pos(interaction.pos)));
        //             }
        //         }
        //         battle.after_single_move(&mut armys);
        // 		println!("Battle Info: {}; Active unit is: {:?}; Can interact?: {:?};", battle.move_count, battle.active_unit, battle.can_interact);

        //     }
        //     battle.end(&mut armys);
        // }
    }
    fn gen_army_fixed(army_num: usize, units: &Vec<Unit>) -> Army {
        let mut army = Army::new(
            vec![],
            ArmyStats {
                gold: 0,
                mana: 0,
                army_name: String::new(),
            },
            vec![None, None, None, None],
            (0, 0),
            true,
            crate::battle::control::Control::PC,
        );
        for _ in 0..12 {
            let mut unit = units.get(4).unwrap().clone();
            unit.inventory.items = vec![None; 4];
            army.add_troop(Troop::new(units.get(4).unwrap().clone()).into())
                .ok();
        }
        army
    }
    #[test]
    fn can_attack() {
        trait Pos {
            fn pos(self, pos: UnitPos) -> Troop;
        }
        impl Pos for Unit {
            fn pos(self, pos: UnitPos) -> Troop {
                let mut tr = Troop::new(self);
                tr.pos = pos;
                tr
            }
        }
        fn get_troop_at(pos: UnitPos) -> TroopType {
            let unit = UNITS.read().unwrap()[4].clone();
            SendMut::new(unit.pos(pos))
        }
		let registry = todo!();
        parse_units::<StupidReader>(Some("dt/Units.ini"));
        let tr1 = vec![get_troop_at(UnitPos::from_index(6))];
        let tr2 = vec![get_troop_at(UnitPos::from_index(7))];
        let army_test = gen_army_fixed(1, &UNITS.read().unwrap());
        fn army(tr: Vec<SendMut<Troop>>) -> Army {
            Army::new(
                tr,
                ArmyStats::default(),
                vec![],
                (0, 0),
                true,
                crate::battle::control::Control::Player(0),
            )
        }
        let mut armies = vec![army(tr1), army_test, army(tr2)];
        let mut battle = BattleInfo::new(&mut armies, 0, 1);
        battle.active_unit = Some(BattleUnit { index: 0, army: 0 });
        assert_eq!(
            search_interactions(&mut battle, battle.active_unit.unwrap(), &armies, registry),
            vec![BattleUnitPos { pos: 0, army: 0 }]
        );
        let mut battle = BattleInfo::new(&mut armies, 2, 1);
        battle.active_unit = Some(BattleUnit { index: 0, army: 2 });
        assert_eq!(
            search_interactions(&mut battle, battle.active_unit.unwrap(), &armies, registry),
            vec![BattleUnitPos { pos: 0, army: 2 }]
        );
    }
    #[test]
    fn field_type_check() {
        let mut fields = vec![];
        for i in 0..12 {
            fields.push(field_type(i, 12));
        }
        use crate::battle::Field::*;
        assert_eq!(
            fields,
            vec![
                Reserve, Back, Back, Back, Back, Reserve, Reserve, Front, Front, Front, Front,
                Reserve
            ]
        );
    }
    #[test]
    fn ini() {
        let mut parser = ini_core::Parser::new(
            "[4 Ополченец]
GlobalIndex=4
Name=Ополченец
Descript=Ополченцем становится рекрут, которому дали в руки копье и научили ходить стоем.
Cost=50
CostMultipler=100
CostGoldDiv=1
StartExpirience=60
LevelMultipler=140
IconIndex=02
Bonus=FlankStrike
// развитие отряда
NextUnit1=Стражник
NextUnit1Level=1
NextUnit2=Пехотинец
NextUnit2Level=1
// боевые характеристики
Hits=50
AttackBlow=25
Initiative=9
Manevres=1
// поуровневые изменения характеристики
d-Hits=5
d-AttackBlow=5
d-DefenceBlow=3

",
        )
        .auto_trim(true);
    }
}
