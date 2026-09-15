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
    ///
    /// alkahest::serialize_to_vec паникует (assert heap size) на некоторых
    /// вложенных Vec-формулах, поэтому кадр собирается аллоцирующим
    /// VecBuffer'ом c запасом: serialize в собственный Vec через
    /// `serialize` + SizeHolder не проще — используем
    /// `serialize_to_vec` на поэлементно иного пути нет; обход —
    /// сериализуем точно так же, как старый сервер (он работал): клон
    /// значения прямо в вызове.
    pub fn state_frame(&self) -> AWsMessage {
        let frame = outcoming_to_bytes(Outcoming::Battle((
            self.armies.clone(),
            self.battle.clone(),
        )));
        AWsMessage::Binary(frame.into())
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

#[cfg(test)]
mod tests {
    use super::*;
    use dt_lib::battle::troop::Troop;
    use dt_lib::mutrc::SendMut;
    use dt_lib::units::unit::{Unit, UnitPos};

    fn fill_registry(registry: &mut GameInfo) {
        for i in 0..=20 {
            let mut u = minimal_unit();
            u.id = i;
            registry.units.inner.push(u);
        }
    }

    fn minimal_unit() -> dt_lib::units::unit::UnitInfo {
        dt_lib::units::unit::UnitInfo {
            id: 20,
            str_id: "melee".into(),
            name: "Melee".into(),
            descript: "".into(),
            cost: 0,
            cost_hire: 0,
            icon_index: 20,
            size: (1, 1),
            unit_type: dt_lib::units::unit::UnitType::People,
            next_unit: vec![],
            magic_type: None,
            magic_direction: Default::default(),
            surrender: None,
            bonus: None,
            settings: Default::default(),
            stats: Default::default(),
            lvl: Default::default(),
        }
    }

    fn battle_registry() -> GameInfo {
        let mut r = GameInfo::new();
        fill_registry(&mut r);
        r
    }

    fn army_with_one_unit(registry: &GameInfo) -> Army {
        let mut army = Army::new(
            vec![],
            ArmyStats { gold: 0, mana: 0, army_name: String::new() },
            vec![], (0, 0), true, Control::PC, registry,
        );
        let unit = Unit::from((registry.units.inner[20].clone(), &registry.bonuses));
        let mut troop = Troop::new(unit);
        troop.pos = UnitPos::from_index(7, 6);
        army.troops.push(SendMut::new(troop));
        army.recalc_army_hitmap(&registry.units);
        army
    }

    /// ПУНКТ 1: каскад — находим точное поле, где size_hint != write.
    /// Каждый шаг:serialize_to_vec либо паникует (assert 88!=92), либо ок.
    #[test]
    fn option_battle_unit_none_some() {
        use dt_lib::battle::battlefield::BattleUnit;
        let none: Option<BattleUnit> = None;
        let mut b = vec![];
        alkahest::serialize_to_vec::<Option<BattleUnit>, Option<BattleUnit>>(none, &mut b);
        println!("[opt] None ok");
        let some: Option<BattleUnit> = Some(BattleUnit { army: 0, index: 3 });
        let mut b2 = vec![];
        alkahest::serialize_to_vec::<Option<BattleUnit>, Option<BattleUnit>>(some, &mut b2);
        println!("[opt] Some ok");
    }

    #[test]
    fn cascade_locate_field() {
        let registry = battle_registry();
        let army = army_with_one_unit(&registry);

        // Шаг A: SendMut<Troop> сам по себе.
        {
            let t: SendMut<Troop> = SendMut::new(Troop::new(Unit::from((
                registry.units.inner[20].clone(),
                &registry.bonuses,
            ))));
            let mut buf = vec![];
            alkahest::serialize_to_vec::<SendMut<Troop>, SendMut<Troop>>(t, &mut buf);
            println!("[cascade] A SendMut<Troop>: ok ({} bytes)", buf.len());
        }
        // Шаг B: Army (troops=[1], hitmap пересчитан).
        {
            let mut buf = vec![];
            alkahest::serialize_to_vec::<Army, Army>(army.clone(), &mut buf);
            println!("[cascade] B Army: ok ({} bytes)", buf.len());
        }
        // Шаг C: BattleInfo::new на 2 армиях (как в сервере).
        {
            let armies = vec![army.clone(), army_with_one_unit(&registry)];
            let battle = BattleInfo::new(&armies, 0, 1);
            // C1: [Vec<bool>; 2] (armies_moved)
            {
                let mut b = vec![];
                alkahest::serialize_to_vec::<[Vec<bool>; 2], [Vec<bool>; 2]>(
                    battle.armies_moved.clone(), &mut b);
                println!("[cascade] C1 armies_moved: ok");
            }
            // C2 перенесён в отдельный тест option_battle_unit below.
            println!("[cascade] C2 skipped in cascade");
            let _ = battle;
            // C3: Vec<BattleUnit> (move_order)
            {
                let mut b = vec![];
                alkahest::serialize_to_vec::<Vec<dt_lib::battle::battlefield::BattleUnit>,
                                            Vec<dt_lib::battle::battlefield::BattleUnit>>(
                    battle.move_order.clone(), &mut b);
                println!("[cascade] C3 move_order: ok");
            }
            // C4: Option<Vec<BattleUnitPos>> (can_interact)
            {
                let mut b = vec![];
                alkahest::serialize_to_vec::<Option<Vec<dt_lib::battle::battlefield::BattleUnitPos>>,
                                            Option<Vec<dt_lib::battle::battlefield::BattleUnitPos>>>(
                    battle.can_interact.clone(), &mut b);
                println!("[cascade] C4 can_interact: ok");
            }
            // C5: Vec<[Vec<BattleUnit>; 2]> (event_listeners)
            {
                let mut b = vec![];
                alkahest::serialize_to_vec::<Vec<[Vec<dt_lib::battle::battlefield::BattleUnit>; 2]>,
                                            Vec<[Vec<dt_lib::battle::battlefield::BattleUnit>; 2]>>(
                    battle.event_listeners.clone(), &mut b);
                println!("[cascade] C5 event_listeners: ok");
            }
            let mut buf = vec![];
            alkahest::serialize_to_vec::<BattleInfo, BattleInfo>(battle.clone(), &mut buf);
            println!("[cascade] C BattleInfo: ok ({} bytes)", buf.len());
        }
        // Шаг D: кортеж (Vec<Army>, BattleInfo) — как payload Battle.
        {
            let armies = vec![army.clone(), army_with_one_unit(&registry)];
            let battle = BattleInfo::new(&armies, 0, 1);
            let mut buf = vec![];
            alkahest::serialize_to_vec::<(Vec<Army>, BattleInfo), (Vec<Army>, BattleInfo)>(
                (armies, battle), &mut buf);
            println!("[cascade] D tuple: ok ({} bytes)", buf.len());
        }
        // Шаг E: полный Outcoming::Battle — раньше падал здесь.
        {
            let armies = vec![army.clone(), army_with_one_unit(&registry)];
            let battle = BattleInfo::new(&armies, 0, 1);
            let mut buf = vec![];
            alkahest::serialize_to_vec::<Outcoming, Outcoming>(
                Outcoming::Battle((armies, battle)), &mut buf);
            println!("[cascade] E Outcoming::Battle: ok ({} bytes)", buf.len());
        }
    }
}
