use crate::{
    battle::{
        army::{find_path, Army, TroopType},
        battlefield::{handle_action, BattleInfo},
        control::{Player, Players},
        troop::Troop,
        BattleUnitPos,
    },
    map::{
        event::{
            execute_event, execute_event_as_player, DelayedEvent, Event, Events, Execute,
            Executions,
        },
        map::GameMap,
        object::ObjectInfo,
    },
    parse::SETTINGS,
    time::time::Time,
    units::unit::{Unit, UnitInfo, UnitInventory, UnitLvl, UnitPos, UnitStats},
    Menu,
};
use alkahest::*;
use log;

#[derive(Clone, Debug)]
#[alkahest(Deserialize, Serialize, SerializeRef, Formula)]
pub enum ClientMessage {
    Action(BattleUnitPos),

    Pause,
    Wait(Time),

    Answer(Option<usize>),

    Spell { apply_to: usize, spell: usize },

    BuyItem(usize),
    SellItem(usize),

    BuySpell(usize),

    Hire(usize),

    Abandon(UnitPos),
    MoveUnit { from: UnitPos, to: UnitPos },
    EvolveUnit { who: usize, to: usize },

    GoTo((usize, usize)),
    Follow(usize),
}

#[derive(Clone, Debug)]
#[alkahest(Deserialize, Serialize, SerializeRef, Formula)]
pub enum ServerMessage {
    State((Option<BattleInfo>, GameMap)),
    ChangeMenu(usize),
}

#[derive(Clone, Debug)]
pub struct Executor {
    pub map: GameMap,
    pub events: Events,
    pub battle: Option<BattleInfo>,
    pub execution_queue: Vec<DelayedEvent>,
    pub players: Players,
}
impl Executor {
    pub fn message_handler(&mut self, message: ClientMessage, player: usize) {
        let player_army = self.players[player].army;
        use ClientMessage::*;
        match message {
            Action(to) => {
                let Some(battle) = &mut self.battle else {
                    return;
                };
                if !([battle.army1, battle.army2].contains(&player_army)
                    && battle
                        .active_unit
                        .is_some_and(|unit| unit.army == player_army))
                {
                    return;
                }
                handle_action((to.pos, to.army), battle, &mut self.map.armys);
            }
            Pause => {
                self.map.pause = !self.map.pause;
            }
            GoTo(to) => {
                let from = self.map.armys[player_army].pos;
                if let Some(path) = find_path(&self.map, &[], from, to, false) {
                    self.map.armys[player_army].path = path.0;
                };
            }
            Follow(who) => {}
            _ => {}
        }
    }
    pub fn tick(&mut self, units: &Vec<Unit>) {
        let is_paused = self.map.pause
            || dbg!(self.players.iter().any(|player| self
                .map
                .armys
                .get(player.army)
                .is_some_and(|army| army.path.is_empty())
                || !player.execution_queue.is_empty()));
        if is_paused {
            return;
        }

        fn handle_event_res(players: &mut Players, queue: &mut Vec<DelayedEvent>, res: Executions) {
            for (command, player) in res {
                match command {
                    Execute::Sub(e) => {
                        // execute_event(
                        //     e,
                        //     &mut self.players,
                        //     &mut self.map,
                        //     &mut self.events,
                        //     units,
                        //     false,
                        // );
                    }
                    Execute::Execute(e) => {
                        queue.push(e);
                    }
                    _ => {
                        if let Some(player) = players.get_mut(player) {
                            player.execution_queue.push(command);
                        }
                    }
                }
            }
        }
        for i in 0..(self.events.len()) {
            let res = execute_event(
                i,
                &mut self.players,
                &mut self.map,
                &mut self.events,
                units,
                false,
            );
            if let Some(res) = res {
                handle_event_res(&mut self.players, &mut self.execution_queue, res);
            }
        }
        let mut new = vec![];
        self.execution_queue.extract_if(0.., |event| {
            if event.time <= self.map.time {
                if let Some(mut res) = execute_event(
                    event.event,
                    &mut self.players,
                    &mut self.map,
                    &mut self.events,
                    units,
                    false,
                ) {
                    new.append(&mut res);
                }
                true
            } else {
                false
            }
        });
        handle_event_res(&mut self.players, &mut self.execution_queue, new);
        for player in &mut self.players {
            if let Some(execute) = player.execution_queue.get(0) {
                let res = match execute {
                    Execute::Execute(_) => true,
                    Execute::Message(_) => false,
                    Execute::StartBattle(with) => {
                        self.battle =
                            Some(BattleInfo::new(&mut self.map.armys, player.army, *with));
                        true
                    }
                    Execute::Wait(until) => until <= &self.map.time,
                    Execute::Sub(_) => true,
                };
                if res {
                    player.execution_queue.remove(0);
                }
            };
            if player.execution_queue.is_empty() {
                let player_army = &mut self.map.armys[player.army];
                if let Some(pos) = player_army.path.get(0) {
                    player_army.pos = *pos;
                    player_army.path.remove(0);
                }
            }
        }
    }
}
