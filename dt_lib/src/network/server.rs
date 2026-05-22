use crate::{
    Menu, battle::{
        BattleUnitPos, army::{Army, TroopType, find_path}, battlefield::{BattleInfo, handle_action}, control::{Player, Players}, troop::Troop
    }, map::{
        event::{
            DelayedEvent, Event, Events, Execute, Executions, execute_event, execute_event_as_player
        },
        map::GameMap,
        object::ObjectInfo, tile::TILES,
    }, registry::GameInfo, time::time::Time, units::unit::{Unit, UnitInfo, UnitInventory, UnitLvl, UnitPos, UnitStats}
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
    pub gamemap: GameMap,
    pub events: Events,
    pub battle: Option<BattleInfo>,
    pub execution_queue: Vec<DelayedEvent>,
    pub players: Players,
}
impl Executor {
    pub fn message_handler(&mut self, message: ClientMessage, player: usize, registry: &GameInfo) {
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
                handle_action((to.pos, to.army), battle, &mut self.gamemap.armys, registry);
            }
            Pause => {
                self.gamemap.pause = !self.gamemap.pause;
            }
            GoTo(to) => {
                let from = self.gamemap.armys[player_army].pos;
                if let Some(path) = find_path(&self.gamemap, from, to, self.gamemap.armys[player_army].transport, registry) {
                    self.gamemap.armys[player_army].path = path.0;
                };
            }
            Follow(who) => {}
            _ => {}
        }
    }
	pub fn is_paused(&self) -> bool {
		self.gamemap.pause
            || self.players.iter().any(|player| self
                .gamemap
                .armys
                .get(player.army)
                .is_some_and(|army| army.path.is_empty())
				&& !player.execution_queue.is_empty())
	}
    pub fn tick(&mut self, registry: &GameInfo) {
        if self.is_paused() {
            return;
        }

        fn handle_event_res(executor: &mut Executor, res: Executions, registry: &GameInfo) {
            for (command, player) in res {
                match command {
                    Execute::Sub(e) => {
						let Event {
							name,
							location,
							conditions,
							result,
							message,
							..
						} = &mut executor.events[e];
                        if let Some(res) = execute_event_as_player(
							message,
							result,
							conditions,
							location,
							&mut executor.gamemap,
							&mut executor.players[player],
							player,
							true,
							e,
							registry
						) {
							handle_event_res(executor, res, registry);
						}
					}
                    Execute::Execute(e) => {
                        executor.execution_queue.push(e);
                    }
                    _ => {
                        if let Some(player) = executor.players.get_mut(player) {
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
                &mut self.gamemap,
                &mut self.events,
                false,
				registry
            );
            if let Some(res) = res {
                handle_event_res(self, res, registry);
            }
        }
        let mut new = vec![];
        self.execution_queue.extract_if(0.., |event| {
            if event.time <= self.gamemap.time {
                if let Some(mut res) = execute_event(
                    event.event,
                    &mut self.players,
                    &mut self.gamemap,
                    &mut self.events,
                    false,
					registry
                ) {
                    new.append(&mut res);
                }
                true
            } else {
                false
            }
        });
        handle_event_res(self, new, registry);
        for player in &mut self.players {
            if let Some(execute) = player.execution_queue.get(0) {
                let res = match execute {
                    Execute::Execute(_) | Execute::Sub(_) => true,
                    Execute::Message(_) => false,
                    Execute::StartBattle(with) => {
                        self.battle =
                            Some(BattleInfo::new(&mut self.gamemap.armys, player.army, *with));
                        true
                    }
                };
                if res {
                    player.execution_queue.remove(0);
                }
            };
            if player.execution_queue.is_empty() {
                let player_army = &mut self.gamemap.armys[player.army];
                if let Some(pos) = player_army.path.get(0) {
					if player_army.transport && !TILES[self.gamemap.tilemap[*pos]].need_transport() {
						player_army.transport = false;
					}
                    player_army.pos = *pos;
					let events = &self.gamemap.eventmap[*pos];
					if events.len() > 0 {
						player.execution_queue.append(&mut events.iter().map(|x| Execute::Execute(DelayedEvent::new(Time::new(0), *x))).collect());
					}
					player_army.path.remove(0);
                }
            }
        }
    }
	
}
