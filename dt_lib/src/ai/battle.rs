//! Battle AI: жадный 1-ply выбор действия активного юнита (план §2).
//! `decide` чиста по состоянию: читает бой короткими SendMut-guard'ами
//! (дедлок-гигиена AGENTS.md #1), лишь временно подставляет позицию активного
//! войска для симуляции манёвра и возвращает её. Мутаций боя нет — решение
//! исполняется существующим `handle_action`.

use crate::{
    ai::{
        eval::{self, UnitSnapshot},
        policy::BattlePolicy,
    },
    battle::{
        army::Army,
        battlefield::{field_type, possible_movement, BattleInfo, Field},
        BattleUnit, BattleUnitPos,
    },
    registry::GameInfo,
    units::unit::{self, ActionResult},
};

/// Решение ИИ (план §2.1). Исполнение — через handle_action.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AiDecision {
    /// Атака цели, доступной прямо сейчас (валидация та же, что в
    /// search_interactions — чистый unit::attack).
    Attack { target: BattleUnitPos },
    /// Манёвр на клетку to, затем атака цели.
    MoveAndAttack { to: usize, target: BattleUnitPos },
    /// Манёвр без атаки (сближение/отход).
    Move { to: usize },
    /// Пропустить ход (семантика клика по себе).
    Skip,
    /// Каст заклинания — фаза 2 (пока не порождается, cast_spells=false).
    Cast { spell: usize, target: BattleUnitPos },
}

/// Плоский эквивалент ценности дебаффа: урона нет, но проклятие ослабляет цель.
const DEBUFF_VALUE: f64 = 6.;

pub struct BattleAi;

impl BattleAi {
    /// Жадный 1-ply (план §2.3): перебор прямых атак и симулированных
    /// манёвров-с-атакой, максимум score; иначе сближение или Skip.
    /// Детерминирована: без RNG, стабильный порядок перебора, ties — первый.
    pub fn decide(
        battle: &BattleInfo,
        armies: &Vec<Army>,
        my: usize,
        policy: &BattlePolicy,
        registry: &GameInfo,
    ) -> AiDecision {
        let Some(active) = battle.active_unit.filter(|u| u.army == my) else {
            return AiDecision::Skip;
        };
        let Some(me) = eval::snapshot_active(armies, active, policy, registry) else {
            return AiDecision::Skip;
        };
        if me.moves < 1 {
            return AiDecision::Skip;
        }
        let foe_army = if my == battle.army1 { battle.army2 } else { battle.army1 };
        let max_troops = armies[my].max_troops;
        let columns = max_troops / 2;
        let foe_columns = armies[foe_army].max_troops / 2;
        let foes = eval::snapshot_army(armies, foe_army, policy, registry);

        // §2.4 hp_threshold_retreat: раненый ниже порога не разменивается.
        if policy.hp_threshold_retreat > 0.
            && (me.hp.max(1) as f32 / me.max_hp.max(1) as f32) < policy.hp_threshold_retreat
        {
            return retreat(&me, armies, my, active, max_troops, columns);
        }

        let mut best: Option<(f64, AiDecision)> = None;

        // 1) Прямые атаки (эквивалент search_interactions: валидация unit::attack).
        for foe in &foes {
            let target = BattleUnit { army: foe_army, index: foe.index };
            let Some(kind) = unit::attack(active, target, armies, registry) else {
                continue;
            };
            let score = attack_score(classified_damage(kind, &me, foe), foe, &me, policy);
            keep_best(&mut best, score, attack_decision(foe, foe_army, foe_columns));
        }

        // 2) MoveAndAttack: перебор достижимых клеток (possible_movement),
        //    симуляция «встал сюда» — attack чистая, дёшево.
        if me.moves >= 2 {
            let sim_map = freed_hitmap(armies, my, &me, columns);
            let backline_unit = me.damage.ranged > 0 || me.magic_type.is_some();
            for cell in possible_movement(me.pos.whole(columns), columns) {
                if cell == me.pos.whole(columns)
                    || field_type(cell, max_troops) == Field::Reserve
                {
                    continue; // резерв не для атаки: без перка оттуда не бьют
                }
                if policy.keep_backline
                    && backline_unit
                    && field_type(cell, max_troops) == Field::Front
                {
                    continue; // §2.4: стрелок/маг не выходит в первую линию
                }
                let np = crate::units::unit::UnitPos::from_index(cell, columns);
                if !Army::fit_to(&sim_map, me.size, columns, 2, np.1, np.0) {
                    continue;
                }
                let me_sim = UnitSnapshot { pos: np, ..me };
                simulate_pos(armies, my, active, np);
                for foe in &foes {
                    let target = BattleUnit { army: foe_army, index: foe.index };
                    if let Some(kind) = unit::attack(active, target, armies, registry) {
                        let score =
                            attack_score(classified_damage(kind, &me_sim, foe), foe, &me_sim, policy);
                        keep_best(
                            &mut best,
                            score,
                            AiDecision::MoveAndAttack {
                                to: cell,
                                target: BattleUnitPos {
                                    army: foe_army,
                                    pos: foe.pos.whole(foe_columns),
                                },
                            },
                        );
                    }
                }
                simulate_pos(armies, my, active, me.pos);
            }
        }

        if best.is_some() {
            return best.unwrap().1;
        }

        // 3) Нет выгодной атаки — сближение с лучшей целью (план §2.3 п.4).
        if let Some(decision) = approach(&me, &foes, armies, my, active, max_troops, columns, policy)
        {
            return decision;
        }

        // 4) Ничего не gagner — Skip (skip_when_no_gain).
        AiDecision::Skip
    }
}

/// score = dmg × unit_value(цель) + kill_bonus − threat × threat_weight × (1.5 − aggression)
/// (план §2.3 п.2; aggression 0..1 — вес атаки над самосохранением).
fn attack_score(dmg: f64, foe: &UnitSnapshot, me: &UnitSnapshot, policy: &BattlePolicy) -> f64 {
    let kill = foe.hp > 0 && dmg >= foe.hp as f64;
    let value = eval::unit_value(foe);
    let threat = if kill {
        0. // мёртвый не отвечает
    } else {
        eval::threat_of(foe, me) as f64
    };
    let mut score = dmg * value;
    if kill {
        score += policy.kill_bonus as f64;
    }
    score - threat * policy.threat_weight as f64 * (1.5 - policy.aggression as f64).max(0.)
}

/// Урон по классификации движка: Melee/MagicDamage — «ближняя» компонента,
/// Ranged — дальняя, Debuff — плоская ценность проклятия.
fn classified_damage(kind: ActionResult, me: &UnitSnapshot, foe: &UnitSnapshot) -> f64 {
    match kind {
        ActionResult::Ranged => eval::expected_damage(me, foe, true),
        ActionResult::Debuff | ActionResult::Buff => DEBUFF_VALUE,
        _ => eval::expected_damage(me, foe, false),
    }
}

fn attack_decision(foe: &UnitSnapshot, foe_army: usize, foe_columns: usize) -> AiDecision {
    AiDecision::Attack {
        target: BattleUnitPos {
            army: foe_army,
            pos: foe.pos.whole(foe_columns),
        },
    }
}

/// Сохранить лучший вариант; ties — первый (детерминированность).
fn keep_best(best: &mut Option<(f64, AiDecision)>, score: f64, decision: AiDecision) {
    if best.as_ref().is_none_or(|(b, _)| score > *b) {
        *best = Some((score, decision));
    }
}

/// Хитмап своей армии без клеток активного юнита (для проверки fit_to).
fn freed_hitmap(armies: &Vec<Army>, my: usize, me: &UnitSnapshot, columns: usize) -> Vec<Option<usize>> {
    let mut map = armies[my].hitmap.clone();
    for j in 0..me.size.1 {
        for i in 0..me.size.0 {
            if let Some(cell) = map.get_mut((me.pos.1 + j) * columns + me.pos.0 + i) {
                *cell = None;
            }
        }
    }
    map
}

/// Временная подстановка позиции для симуляции: guard короткий, attack() зовётся
/// без удержания (иначе дедлок — AGENTS.md #1).
fn simulate_pos(armies: &Vec<Army>, my: usize, active: BattleUnit, pos: crate::units::unit::UnitPos) {
    let mut troop = armies[my].troops[active.index].get();
    troop.pos = pos;
}

/// Отход раненого юнита: первый пустой Back, иначе первый Reserve (§2.4).
fn retreat(
    me: &UnitSnapshot,
    armies: &Vec<Army>,
    my: usize,
    active: BattleUnit,
    max_troops: usize,
    columns: usize,
) -> AiDecision {
    let sim_map = freed_hitmap(armies, my, me, columns);
    let from = me.pos.whole(columns);
    let mut reserve_choice = None;
    for cell in possible_movement(from, columns) {
        if cell == from {
            continue;
        }
        let np = crate::units::unit::UnitPos::from_index(cell, columns);
        if !Army::fit_to(&sim_map, me.size, columns, 2, np.1, np.0) {
            continue;
        }
        match field_type(cell, max_troops) {
            Field::Back => return AiDecision::Move { to: cell },
            Field::Reserve => {
                reserve_choice.get_or_insert(cell);
            }
            Field::Front => {}
        }
    }
    reserve_choice
        .map(|to| AiDecision::Move { to })
        .unwrap_or(AiDecision::Skip)
}

/// Сближение: цель — наименьший hp (затем наибольшая ценность, затем индекс),
/// клетка — минимум манхэттен-дистанции; без улучшения — Skip.
#[allow(clippy::too_many_arguments)]
fn approach(
    me: &UnitSnapshot,
    foes: &[UnitSnapshot],
    armies: &Vec<Army>,
    my: usize,
    active: BattleUnit,
    max_troops: usize,
    columns: usize,
    policy: &BattlePolicy,
) -> Option<AiDecision> {
    let target = foes.iter().fold(None::<&UnitSnapshot>, |acc, foe| {
        better_target(acc, foe)
    })?;
    let dist = |pos: crate::units::unit::UnitPos| -> usize {
        pos.0.abs_diff(target.pos.0) + pos.1.abs_diff(target.pos.1)
    };
    let current = dist(me.pos);
    let sim_map = freed_hitmap(armies, my, me, columns);
    let backline_unit = me.damage.ranged > 0 || me.magic_type.is_some();
    let mut best: Option<(usize, usize)> = None; // (дистанция, клетка)
    for cell in possible_movement(me.pos.whole(columns), columns) {
        if cell == me.pos.whole(columns) || field_type(cell, max_troops) == Field::Reserve {
            continue;
        }
        if policy.keep_backline
            && backline_unit
            && field_type(cell, max_troops) == Field::Front
        {
            continue;
        }
        let np = crate::units::unit::UnitPos::from_index(cell, columns);
        if !Army::fit_to(&sim_map, me.size, columns, 2, np.1, np.0) {
            continue;
        }
        let d = dist(np);
        if d < current && best.is_none_or(|(b, _)| d < b) {
            best = Some((d, cell));
        }
    }
    // skip_when_no_gain: без сокращения дистанции best=None → Skip.
    best.map(|(_, cell)| AiDecision::Move { to: cell })
}

/// Цель сближения: меньше hp; при равенстве — выше ценность; затем меньше индекс.
fn better_target<'a>(
    acc: Option<&'a UnitSnapshot>,
    foe: &'a UnitSnapshot,
) -> Option<&'a UnitSnapshot> {
    match acc {
        None => Some(foe),
        Some(best) => {
            let foe_key = (
                std::cmp::Reverse(foe.hp),
                std::cmp::Reverse((foe.value_weight * 1000.) as i64),
                foe.index,
            );
            let best_key = (
                std::cmp::Reverse(best.hp),
                std::cmp::Reverse((best.value_weight * 1000.) as i64),
                best.index,
            );
            Some(if foe_key < best_key { foe } else { best })
        }
    }
}

impl BattleAi {
    /// Исполнить решение активного юнита через существующий handle_action:
    /// ИИ не мутирует состояние сам (правило зависимостей §1 плана).
    /// MoveAndAttack исполняется манёвром сейчас — юнит с остатком moves
    /// остаётся активным (search_next_active держит его первым), следующим
    /// решением ИИ доиграет атаку.
    pub fn execute(
        decision: AiDecision,
        battle: &mut BattleInfo,
        armies: &mut Vec<Army>,
        registry: &GameInfo,
    ) -> Option<ActionResult> {
        let active = battle.active_unit?;
        let my_pos = {
            let columns = armies[active.army].max_troops / 2;
            armies[active.army].troops[active.index].get().pos.whole(columns)
        };
        let action = match decision {
            AiDecision::Attack { target } => (target.pos, target.army),
            AiDecision::MoveAndAttack { to, .. } => (to, active.army),
            AiDecision::Move { to } => (to, active.army),
            // Skip = «клик по себе»: спуск хода (селф-каст, если позволяет
            // magic_direction) — Space-семантика из плана §2.1.
            AiDecision::Skip | AiDecision::Cast { .. } => (my_pos, active.army),
        };
        crate::battle::battlefield::handle_action(action, battle, armies, registry)
            .map(|(res, _)| res)
    }
}
