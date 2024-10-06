use crate::{
    battle::{
        army::{Army, TroopType, MAX_TROOPS},
        troop::Troop,
    },
    items::item::Item,
    map::map::GameMap,
    network::net::*,
    units::unit::*,
};
use alkahest::{alkahest, serialize, serialized_size};
use renet::DefaultChannel;
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
    pub fn new(armys: &mut Vec<Army>, army1: usize, army2: usize) -> Self {
        let mut battle = BattleInfo {
            army1,
            army2,
            battle_ter: armys[army2].building.unwrap_or(0),
            winner: None,
            ..Default::default()
        };
        battle.start(armys);
        battle
    }
    pub fn start(&mut self, armys: &mut Vec<Army>) {
        let army1 = &mut armys[self.army1];

        army1.troops.iter_mut().for_each(|troop| {
            let unit = &mut troop.get().unit;
            let bonus = unit.get_bonus();
            bonus.on_battle_start(unit, &self);
            unit.bonus = bonus;
            unit.recalc();
        });
        let army2 = &mut armys[self.army2];
        army2.troops.iter_mut().for_each(|troop| {
            let unit = &mut troop.get().unit;
            let bonus = unit.get_bonus();
            bonus.on_battle_start(unit, &self);
            unit.bonus = bonus;
            unit.recalc();
        });
        self.winner = None;
        self.active_unit = self.search_next_active(&*armys);
        self.can_interact = search_interactions(self, &*armys);
    }
    pub fn remove_corpses(&mut self, armys: &mut Vec<Army>) {
        let army1 = &mut armys[self.army1];
        remove_corpses(self, &mut army1.troops);
        army1.recalc_army_hitmap();
        let army2 = &mut armys[self.army2];
        remove_corpses(self, &mut army2.troops);
        army2.recalc_army_hitmap();
    }
    pub fn search_next_active(&self, armys: &Vec<Army>) -> Option<(usize, usize)> {
        let army1 = &armys[self.army1];
        let army2 = &armys[self.army2];
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
                })
                .unwrap()
        }
        let troops = &army1.troops;
        let next1 = max_speed(&troops);
        let troops = &army2.troops;
        let next2 = max_speed(&troops);
        return {
            let (troop1, troop2) = (&next1.1.get(), &next2.1.get());

            let tr1_inactive = troop_inactive(&troop1);
            let tr2_inactive = troop_inactive(&troop2);
            if troop1.unit.modified.speed > troop2.unit.modified.speed && !tr1_inactive {
                Some((troop1.unit.army, next1.0))
            } else if !tr2_inactive {
                Some((troop2.unit.army, next2.0))
            } else {
                None
            }
        };
    }
    pub fn end(&mut self, armys: &mut Vec<Army>) {
        fn restore_corpses(armys: &mut Vec<Army>, battle: &mut BattleInfo, _winner: usize) {
            let mut corpses = Vec::new();
            corpses.append(&mut battle.dead);
            for dead in corpses {
                let army = dead.get().unit.army;
                armys[army].add_troop(dead).ok();
            }
        }
        fn trigger_end(armys: &mut Vec<Army>, battle: &mut BattleInfo) {
            for troop in &mut armys[battle.army1].troops {
                troop.get().on_battle_end();
            }
        }
        fn move_goods(
            armys: &mut Vec<Army>,
            battle: &mut BattleInfo,
            winner: usize,
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
                .map(|troop| troop.get().unit.info.surrender)
                .sum::<Option<u64>>()
                .unwrap_or(0);
            armys[winner].stats.mana += mana;
            {
                let army = &mut armys[loose];
                army.defeated = true;
            }
            (items, gold, mana)
        }
        if let Some(winner) = self.winner {
            move_goods(armys, self, winner);
            restore_corpses(armys, self, winner);
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

pub fn search_interactions(
    battle: &mut BattleInfo,
    armys: &Vec<Army>,
) -> Option<Vec<(usize, usize)>> {
    let army1 = &armys[battle.army1];
    let army2 = &armys[battle.army2];
    let mut can_interact = Vec::new();
    if let Some(active_unit) = battle.active_unit {
        let active_army = active_unit.0;

        let (tr1, tr2) = (&army1.troops, &army2.troops);
        let (troops1, troops2) = (&tr1, &tr2);
        let active_troops = if active_army == battle.army1 {
            troops1
        } else {
            troops2
        };
        fn collect_interactions(
            active_unit: (usize, usize),
            active_troops: &Vec<TroopType>,
            troops: &Vec<TroopType>,
            army: usize,
            can_interact: &mut Vec<(usize, usize)>,
        ) {
            can_interact.append(
                &mut troops
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
                        if active_unit_unit.can_attack(unit, troop.pos, active_troop.pos) {
                            Some((0, index))
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
            battle.army1,
            &mut can_interact,
        );
        collect_interactions(
            active_unit,
            active_troops,
            troops2,
            battle.army2,
            &mut can_interact,
        );
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
            break;
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
pub fn next_move(battle: &mut BattleInfo, armys: &mut Vec<Army>) {
    if battle.winner.is_some() {
        //state.menu_id = Menu::Start as usize;
        battle.end(armys);
        return;
    }
    let army1 = &mut armys[battle.army1];
    restore_moves(&mut army1.troops);
    army1.recalc_army_hitmap();
    let army2 = &mut armys[battle.army2];
    restore_moves(&mut army2.troops);
    army2.recalc_army_hitmap();
    battle.move_count += 1;
    check_win(battle, armys);
}
pub fn check_win(battle: &mut BattleInfo, armys: &Vec<Army>) {
    fn check_army_win(army: &Army) -> bool {
        let troops = &army.troops;
        // If there are no more troops or all troops are ready to surrender
        troops.is_empty()
            || troops.iter().all(|troop| {
                let troop = troop.get();
                troop.unit.info.surrender.is_some() || troop.is_dead()
            })
    }

    if battle.move_count == MAX_MOVES {
        battle.winner = Some(battle.army1);
    }
    if check_army_win(&armys[battle.army1]) {
        battle.winner = Some(battle.army2);
    } else if check_army_win(&armys[battle.army2]) {
        battle.winner = Some(battle.army1);
    }
}
pub fn check_row_fall(battle: &mut BattleInfo, armys: &mut Vec<Army>) {
    fn check_row(army: &mut Army) -> bool {
        let max_troops = *MAX_TROOPS;
        let lower_row_empty = army
            .hitmap
            .iter()
            .enumerate()
            .skip(max_troops / 2)
            .all(|(i, hit)| hit.is_none() || field_type(i, max_troops) == Field::Reserve);
        lower_row_empty
    }
    fn row_fall(army: &mut Army) {
        let max_troops = *MAX_TROOPS;
        let falled = army
            .hitmap
            .iter()
            .enumerate()
            .take(max_troops / 2)
            .filter(|(i, hit)| hit.is_some() && field_type(*i, max_troops) != Field::Reserve)
            .clone();
        {
            let troops = &army.troops;
            for (index, _) in falled {
                if let Some(mut troop) = troops.get(index).and_then(|t| Some(t.get())) {
                    troop.pos = UnitPos::from_index(
                        <UnitPos as Into<usize>>::into(troop.pos) + max_troops / 2,
                    );
                }
            }
        }
        army.recalc_army_hitmap();
    }

    for army in [battle.army1, battle.army2] {
        let army = &mut armys[army];
        if check_row(army) {
            row_fall(army);
        }
    }
}

/// Action represents possibilities in a battle, Cell
pub enum Action {
    Cell(usize, usize),
    Move(usize, usize, usize),
}
pub fn move_thing(battle: &mut BattleInfo, armys: &mut Vec<Army>) {
    check_win(battle, &armys);
    check_row_fall(battle, armys);
    battle.remove_corpses(armys);
    if battle.winner.is_none() {
        if battle.active_unit == None {
            next_move(battle, armys);
            battle.active_unit = battle.search_next_active(&armys);
        }
        battle.can_interact = search_interactions(battle, &armys);
    } else {
        //state.menu_id = Menu::Start as usize;
    }
}

fn unit_interaction(
    battle: &mut BattleInfo,
    armys: &mut Vec<Army>,
    pos: usize,
    army: usize,
) -> (Option<ActionResult>, bool) {
    if let Some(_) = battle.winner {
        return (None, false);
    }
    let mut action_result = None;
    let army = if army == 0 {
        battle.army1
    } else {
        battle.army2
    };

    let mut unit_inactive = false;

    let Some(active_unit) = battle.active_unit else {
        battle.active_unit = battle.search_next_active(armys);
        move_thing(battle, armys);
        return (None, false);
    };
    let Some(target_index) = armys[army].hitmap[pos] else {
        return if army == active_unit.0 {
            (
                handle_action(
                    Action::Move(active_unit.0, active_unit.1, pos),
                    battle,
                    armys,
                )
                .and_then(|v| Some(v.0)),
                true,
            )
        } else {
            (None, false)
        };
    };
    if active_unit.0 == army && target_index == active_unit.1 {
        let troop = &mut armys[army].troops[target_index].get();
        if troop_inactive(&troop) {
            return (None, false);
        }
        let unit = &mut troop.unit;
        unit.stats.moves -= 1;
        unit.recalc();
        return (None, troop_inactive(&troop));
    } else {
        let target_troops = &armys[army].troops;
        let the_troops = {
            let active_index = active_unit.1;
            let active_army = active_unit.0;
            (
                armys[active_army].troops.get(active_index),
                target_troops.get(target_index),
            )
        };
        let (Some(active_troop), Some(target_troop)) = the_troops else {
            return (None, unit_inactive);
        };
        let (mut active_troop, mut target_troop) = (active_troop.get(), target_troop.get());
        if troop_inactive(&active_troop) {
            return (None, true);
        }
        let active_unit_index = active_troop.pos.into();
        let unit1 = &mut active_troop.unit;
        let unit2 = &mut target_troop.unit;
        if !unit2.is_dead() {
            let res = unit1.attack(
                unit2,
                UnitPos::from_index(pos),
                UnitPos::from_index(active_unit_index),
                &battle,
            );
            if res.is_some() {
                unit1.stats.moves -= 1;
                unit1.recalc();
                if unit1.is_dead() || unit1.modified.moves < 1 {
                    unit_inactive = true;
                }
            }
            action_result = res;
        }
    };
    (action_result, unit_inactive)
}

/// A method that processes an action with the given battle and gamemap, action is done by currently active unit, it will return None if action is impossible due to game rules.
pub fn handle_action(
    action: Action,
    battle: &mut BattleInfo,
    armys: &mut Vec<Army>,
) -> Option<(ActionResult, (usize, usize))> {
    match action {
        Action::Cell(pos, army) => {
            if let Some(_) = battle.winner {
                // state.menu_id = Menu::Start as usize;
                return None;
            }
            let active = battle.active_unit;
            let res = unit_interaction(battle, armys, pos, army);
            if res.1 {
                battle.active_unit = battle.search_next_active(&armys);
            }
            move_thing(battle, armys);
            res.0.and_then(|v| Some((v, active.unwrap())))
        }
        Action::Move(army, troop, to) => {
            let army = &mut armys[army];
            let unit_inactive = {
                let troop = &mut army.troops[troop].get();
                troop.pos = UnitPos::from_index(to);
                let unit = &mut troop.unit;
                unit.stats.moves -= 1;
                unit.recalc();
                unit.modified.moves < 1 || unit.is_dead()
            };
            army.recalc_army_hitmap();
            let active = battle.active_unit;
            if unit_inactive {
                battle.active_unit = battle.search_next_active(&armys);
            }
            move_thing(battle, armys);
            Some((ActionResult::Move, active.unwrap()))
        }
    }
}

/// Used for processing an action in a context of using a server
pub fn handle_server_action(connection: &mut Option<ConnectionManager>, action: (usize, usize)) {
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
            if Some(army) != battle.active_unit.and_then(|v| Some(v.0)).as_ref() {
                return;
            }
            handle_action(
                Action::Cell(action.0, action.1),
                battle,
                &mut connection.gamemap.armys,
            );
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
        battle::ArmyStats,
        parse::{parse_items, parse_units},
        units::unitstats::ModifyUnitStats,
    };
    use rand::{seq::IteratorRandom, thread_rng, Rng};

    use super::*;
    fn get_unit(moves: i64, speed: i64, army: usize) -> Unit {
        let mut unit = Unit {
            bonus: crate::bonuses::Bonus::NoBonus,
            stats: UnitStats {
                speed,
                hp: 1,
                max_hp: 1,
                moves,
                max_moves: moves,
                ..Default::default()
            },
            modified: UnitStats {
                speed,
                ..Default::default()
            },
            modify: ModifyUnitStats::default(),
            info: UnitInfo {
                name: "".into(),
                descript: "".into(),
                cost: 0,
                cost_hire: 0,
                icon_index: 0,
                size: (1, 1),
                unit_type: UnitType::People,
                next_unit: Vec::new(),
                magic_type: None,
                surrender: None,
                lvl: LevelUpInfo::empty(),
            },
            effects: Vec::new(),
            lvl: UnitLvl::empty(),
            inventory: UnitInventory::empty(),
            army,
        };
        unit.recalc();
        unit
    }
    fn gen_army(army_num: usize) -> Army {
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
            crate::battle::Control::PC,
        );
        for _ in 0..10 {
            army.add_troop(Troop::new(get_unit(1, thread_rng().gen_range(1..10), army_num)).into())
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
            crate::battle::Control::PC,
        );
        for _ in 0..10 {
            army.add_troop(
                Troop::new({
                    let mut unit = units.iter().choose(&mut thread_rng()).unwrap().clone();
                    unit.army = army_num;
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
        for _ in 0..100 {
            let army1 = gen_army(0);
            let army2 = gen_army(1);
            let mut armys = vec![army1, army2];
            let battle = BattleInfo::new(&mut armys, 0, 1);

            let mut been = vec![];
            while let Some(active_unit) = battle.search_next_active(&armys) {
                if !been.contains(&active_unit) {
                    been.push(active_unit);
                }
                let troop = &mut armys[active_unit.0].troops[active_unit.1].get();
                assert!(!troop_inactive(troop));
                troop.unit.stats.moves -= 1;
                troop.unit.recalc();
            }
            let gen_expectations = |a| (0..10).map(move |v| (a, v));
            let mut expected = gen_expectations(0).chain(gen_expectations(1));
            let left_out = expected.filter(|v| !been.contains(&v)).collect::<Vec<_>>();
            assert!(left_out.is_empty(), "{left_out:?}");
        }
    }
    #[test]
    fn process_battles() {
        let res = parse_units(Some("dt/Units.ini"));
        let Ok((units, _)) = res else {
            panic!("Unit parsing error")
        };
        let _ = parse_items(Some("dt/Rus_Artefacts.ini"), &"Rus".into());
        for _ in 0..1000 {
            let army1 = gen_army_from_units(0, &units);
            let army2 = gen_army_from_units(1, &units);
            let mut armys = vec![army1, army2];
            let mut battle = BattleInfo::new(&mut armys, 0, 1);
            while battle.winner.is_none() {
                if let Some(interactions) = &battle.can_interact.clone() {
                    if let Some(interaction) = interactions.iter().choose(&mut thread_rng()) {
                        unit_interaction(&mut battle, &mut armys, interaction.0, interaction.1);
                    }
                }
                move_thing(&mut battle, &mut armys);
            }
            battle.end(&mut armys);
        }
    }
}
