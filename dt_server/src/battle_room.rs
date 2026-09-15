//! Битва, принадлежащая комнате (RoomBattle): armies + BattleInfo +
//! правила RoomConfig. Портирован process_message старого сервера:
//! SetUnit/SetItem/Status/Action/GetState, но рассылка — всей комнате,
//! а не двум анонимным сокетам. Валидация армий — RoomConfig::check_army.

use alkahest::{deserialize, serialize_to_vec, Formula};
use axum::extract::ws::Message as AWsMessage;
use dt_lib::battle::battlefield::{handle_action, BattleInfo, BattleUnitPos};
use dt_lib::battle::army::{Army, ArmyStats};
use dt_lib::battle::control::Control;
use dt_lib::network::room::{now_unix, RoomConfig};
use dt_lib::registry::GameInfo;
use std::sync::Arc;

// Протокол боя (алкаhest) — перенесён из старого dt_server main.rs.
#[derive(alkahest::Deserialize, alkahest::Serialize, Formula)]
pub enum Outcoming {
    Id(usize),
    Battle((Vec<Army>, BattleInfo)),
    Status([bool; 2]),
}

#[derive(alkahest::Deserialize, alkahest::Serialize, Formula)]
pub enum Incoming {
    Disconnect,
    GetState,
    Action(BattleUnitPos),
    SetItem((BattleUnitPos, (usize, Option<usize>))),
    SetUnit((BattleUnitPos, Option<usize>)),
    Status(bool),
}

pub fn incoming_from_bytes(data: &[u8]) -> Option<Incoming> {
    deserialize::<Incoming, Incoming>(data).ok()
}

pub fn outcoming_to_bytes(msg: Outcoming) -> Vec<u8> {
    let mut buf = vec![];
    serialize_to_vec::<Outcoming, Outcoming>(msg, &mut buf);
    buf
}

/// Игровое состояние комнаты.
#[derive(Clone)]
pub struct RoomBattle {
    pub config: RoomConfig,
    pub armies: Vec<Army>,
    pub battle: BattleInfo,
    pub acceptance: [bool; 2],
    pub ini_seed: u64,
}

impl RoomBattle {
    /// Пустые армии по бюджету конфига (§1.3: сетап покупкой, старт по
    /// check_army). players — имена из room.players (0 = army1, 1 = army2).
    pub fn new(players: &[String], registry: &GameInfo) -> Self {
        let _ = players; // v1: пустые армии; состав игроки купят в сетапе.
        let config = RoomConfig::default();
        let armies = vec![
            gen_army(0, config.gold_per_side, registry),
            gen_army(1, config.gold_per_side, registry),
        ];
        let battle = BattleInfo::new(&armies, 0, 1);
        Self {
            config,
            armies,
            battle,
            acceptance: [false; 2],
            ini_seed: now_unix(),
        }
    }

    /// Полный снапшот для Outcoming::Battle (Binary-кадр).
    pub fn state_frame(&self) -> AWsMessage {
        let msg = (self.armies.clone(), self.battle.clone());
        let mut buf = vec![];
        serialize_to_vec::<Outcoming, Outcoming>(Outcoming::Battle(msg), &mut buf);
        AWsMessage::Binary(buf.into())
    }

    pub fn status_frame(&self) -> AWsMessage {
        let frame = outcoming_to_bytes(Outcoming::Status(self.acceptance));
        AWsMessage::Binary(frame.into())
    }

    /// Обработка входящего кадра боя (порт process_message). army —
    /// логическая сторона отправителя; usize::MAX = зритель.
    pub fn process(
        &mut self,
        incoming: Incoming,
        army: usize,
        registry: &Arc<GameInfo>,
    ) -> (Option<String>, bool) {
        match incoming {
            Incoming::Disconnect => (None, false),
            Incoming::GetState => (None, true),
            Incoming::Action(action) => {
                if army >= 2 {
                    return (Some("Наблюдатель не может ходить".into()), false);
                }
                if self.battle.winner.is_some() {
                    return (Some("Битва окончена".into()), false);
                }
                if self.battle.active_unit.is_some_and(|x| x.army != army) {
                    return (Some("Not your turn".into()), false);
                }
                let BattleUnitPos { pos, army: target } = action;
                handle_action((pos, target), &mut self.battle, &mut self.armies, registry);
                (None, true)
            }
            Incoming::SetItem((pos, (index, item_id))) => {
                if army >= 2 || self.acceptance == [true, true] {
                    return (Some("pashalka".into()), false);
                }
                let sent = self.armies[army].set_item_unit_at(item_id, pos, index, registry);
                if sent {
                    (None, true)
                } else {
                    (Some("pashalka".into()), false)
                }
            }
            Incoming::SetUnit((pos, unit_id)) => {
                if army >= 2 || self.acceptance == [true, true] {
                    return (Some("pashalka".into()), false);
                }
                self.armies[army].set_unit_at(unit_id, pos, registry);
                (None, true)
            }
            Incoming::Status(status) => {
                if army >= 2 || self.acceptance == [true, true] {
                    return (Some("pashalka".into()), false);
                }
                self.acceptance[army] = status;
                if self.acceptance == [true, true] {
                    // Монетка инициативы (§1.3) по общему сиду, затем старт.
                    if self.config.coin_flip_initiative {
                        self.battle
                            .coin_flip_initiative(&mut self.armies, registry, self.ini_seed);
                    }
                    self.battle.start(&mut self.armies, registry);
                }
                (None, false)
            }
        }
    }
}

/// Пустая армия стороны (порт gen_army): нет юнитов, бюджет от конфига.
fn gen_army(army_num: usize, gold: u64, registry: &GameInfo) -> Army {
    let _ = (army_num, gold);
    Army::new(
        vec![],
        ArmyStats {
            gold: 0,
            mana: 0,
            army_name: String::new(),
        },
        vec![],
        (0, 0),
        true,
        Control::PC,
        registry,
    )
}
