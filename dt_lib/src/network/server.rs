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

    /// Комнатный протокол ПВП (serde_json внутри; см. network::room).
    Room(String),
}

#[derive(Clone, Debug)]
#[alkahest(Deserialize, Serialize, SerializeRef, Formula)]
pub enum ServerMessage {
    State((Option<BattleInfo>, GameMap)),
    ChangeMenu(usize),

    /// Комнатный протокол ПВП (serde_json внутри; см. network::room).
    Room(String),

    /// Ваша армия в ПВП-бою (§1.8): индекс логической стороны (0 = army1,
    /// 1 = army2) + сид монетки инициативы (§1.3) для синхронного старта.
    YourArmy { your_army: usize, ini_seed: u64 },
}

#[derive(Clone, Debug)]
pub struct Executor {
    pub gamemap: GameMap,
    pub events: Events,
    pub battle: Option<BattleInfo>,
    pub execution_queue: Vec<DelayedEvent>,
    pub players: Players,
    /// Последний обработанный день (get_days): Day-условия бонусов
    /// (Лекарь и пр.) диспатчатся один раз при смене суток.
    pub last_day: u64,
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
                // Новый приказ сбрасывает преследование.
                self.gamemap.armys[player_army].chasing = None;
            }
            Follow(who) => {
                // Преследование чужой армии (дабл-клик по ней): путь к
                // СОСЕДНЕЙ клетке цели — на её клетку зайти нельзя
                if who != player_army && self.gamemap.armys.get(who).is_some() {
                    let from = self.gamemap.armys[player_army].pos;
                    let path = chase_path(from, who, &self.gamemap, registry);
                    let army = &mut self.gamemap.armys[player_army];
                    army.chasing = Some(who);
                    army.path = path;
                }
            }
            _ => {}
        }
    }

    /// Ежедневный тик бонусов (Bonus4 Лекарь и др.): диспатчит
    /// AbilityCondition::Day каждому живому юниту с бонусом. Вызывается из
    /// tick() при смене gamemap.time.get_days(); тесты — напрямую.
    pub fn advance_day(&mut self, registry: &GameInfo) {
        use crate::bonuses::{AbilityCondition, AbilityTowardsTroop};
        // Собираем (армия, индекс, клон бонуса) до мутаций.
        let mut bonus_units = vec![];
        for (army_index, army) in self.gamemap.armys.iter().enumerate() {
            for (index, troop) in army.troops.iter().enumerate() {
                let t = troop.get();
                if t.unit.is_dead() {
                    continue;
                }
                if let Some(bonus) = t.unit.get_bonus(registry).cloned() {
                    bonus_units.push((army_index, index, bonus));
                }
            }
        }
        for (army_index, index, bonus) in bonus_units {
            let Some((_, mechanics)) = bonus.rules.get(&AbilityCondition::Day) else {
                continue;
            };
            for mechanic in mechanics {
                // Вне боя RelativeUnit-таргеты не определены — применяем
                // способности к владельцу и (по смыслу Лекаря) всей его армии.
                let targets: Vec<usize> = if mechanic.affects.is_empty() {
                    vec![]
                } else {
                    (0..self.gamemap.armys[army_index].troops.len()).collect()
                };
                let mut target_list: Vec<usize> = targets;
                if mechanic.affects_self && !target_list.contains(&index) {
                    target_list.push(index);
                }
                for target_index in target_list {
                    let mut t = self.gamemap.armys[army_index].troops[target_index].get();
                    if t.unit.is_dead() {
                        continue;
                    }
                    for ability in &mechanic.ability {
                        if matches!(ability, AbilityTowardsTroop::Attack { .. }) {
                            continue; // атаки вне боя не диспатчатся
                        }
                        ability.apply(&mut t.unit, 1, registry);
                    }
                }
            }
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
        // Смена суток: Day-бонусы (Лекарь) — один раз на новый день.
        let day = self.gamemap.time.get_days();
        if day != self.last_day {
            self.last_day = day;
            self.advance_day(registry);
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
        // Преследование: перезапрос пути при смещении цели + контактный бой.
        // Отдельно от цикла players (цель может быть не игроком).
        let mut contacts: Vec<(usize, usize)> = vec![];
        for i in 0..self.gamemap.armys.len() {
            let Some(target) = self.gamemap.armys[i].chasing else {
                continue;
            };
            let Some(target_pos) = self.gamemap.armys.get(target).map(|a| a.pos) else {
                // Цель исчезла (победлена/удалена) — сброс.
                self.gamemap.armys[i].chasing = None;
                continue;
            };
            let my = self.gamemap.armys[i].pos;
            let chebyshev = (my.0 as isize - target_pos.0 as isize)
                .abs()
                .max((my.1 as isize - target_pos.1 as isize).abs());
            if chebyshev <= 1 {
                // Соседство: контакт — бой (как MapClick в net.rs).
                contacts.push((i, target));
                self.gamemap.armys[i].chasing = None;
                self.gamemap.armys[i].path.clear();
                continue;
            }
            // Цель ушла от конца текущего пути — перезапрос пути к её клетке.
            let stale = self.gamemap.armys[i].path.last().is_none_or(|last| {
                let (lx, ly) = (*last, *last);
                let (lx, ly) = (lx.0 as isize, ly.1 as isize);
                (lx - target_pos.0 as isize).abs() > 1
                    || (ly - target_pos.1 as isize).abs() > 1
            });
            if stale {
                let path = chase_path(my, target, &self.gamemap, registry);
                self.gamemap.armys[i].path = path;
            }
        }
        for (a, b) in contacts {
            if self.battle.is_none() {
                self.battle = Some(BattleInfo::new(&mut self.gamemap.armys, a, b));
            }
        }
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
					// На клетке чужая армия — не двигаемся (ждём освобождения).
					if self.gamemap.hitmap[*pos].army.is_some() {
						continue;
					}
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

/// Путь преследования: до ближайшей проходимой СОСЕДНЕЙ клетки цели
/// (на её клетку зайти нельзя — hitmap.army блокирует и step, и фильтр
/// find_path). Пустой путь — цель недоступна.
fn chase_path(
    from: (usize, usize),
    target: usize,
    gamemap: &GameMap,
    registry: &GameInfo,
) -> Vec<(usize, usize)> {
    let Some(target_pos) = gamemap.armys.get(target).map(|a| a.pos) else {
        return vec![];
    };
    let (tx, ty) = (target_pos.0 as isize, target_pos.1 as isize);
    let mut best: Option<Vec<(usize, usize)>> = None;
    for (dx, dy) in [
        (-1isize, 0isize),
        (1, 0),
        (0, -1),
        (0, 1),
        (-1, -1),
        (1, -1),
        (-1, 1),
        (1, 1),
    ] {
        let (nx, ny) = (tx + dx, ty + dy);
        if nx < 0 || ny < 0 {
            continue;
        }
        let (nx, ny) = (nx as usize, ny as usize);
        if nx >= gamemap.tilemap.size || ny >= gamemap.tilemap.size {
            continue;
        }
        // Клетка соседа цели, свободная от армий: стартуем поиск в неё.
        if gamemap.hitmap[(nx, ny)].army.is_some() {
            continue;
        }
        if let Some((path, _)) = find_path(gamemap, from, (nx, ny), false, registry) {
            if best.as_ref().is_none_or(|b| path.len() < b.len()) {
                best = Some(path);
            }
        }
    }
    best.unwrap_or_default()
}
