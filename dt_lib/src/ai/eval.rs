//! Чистые функции оценки боя (план §2.2). Никаких мутаций armies/battle —
//! только чтение короткими SendMut-guard'ами (дедлок-гигиена AGENTS.md #1).
//! `expected_damage` повторяет формулу движка (`compute_attack_damage` в
//! unit.rs): защиты от магии/стрельбы/рукопашной + фланг.

use crate::{
    ai::policy::BattlePolicy,
    battle::{
        army::Army,
        battlefield::{field_type, BattleInfo, Field},
        BattleUnit, BattleUnitPos, TroopType,
    },
    registry::GameInfo,
    units::unit::{Defence, MagicType, Power, UnitPos},
};
use math_thingies::Percent;

/// Снапшот данных юнита, достаточный для оценки (все guard'ы отпущены).
#[derive(Clone, Copy, Debug)]
pub struct UnitSnapshot {
    /// Индекс войска в армии (для BattleUnit).
    pub index: usize,
    pub pos: UnitPos,
    pub size: (usize, usize),
    pub hp: i64,
    pub max_hp: i64,
    pub damage: Power,
    pub defence: Defence,
    pub speed: i64,
    pub moves: i64,
    pub is_main: bool,
    pub magic_type: Option<MagicType>,
    /// Вес архетипа (mage/ranged/melee × main) — посчитан сразу.
    pub value_weight: f32,
}

/// Снять снапшот войска (короткий guard, без вызовов attack внутри).
pub fn snapshot_troop(
    troop: &TroopType,
    index: usize,
    registry: &GameInfo,
    policy: &BattlePolicy,
) -> UnitSnapshot {
    let t = troop.get();
    let info = t.unit.get_info(&registry.units);
    build_snapshot(
        index,
        t.pos,
        info.size,
        t.unit.hp,
        t.unit.modified,
        t.unit.moves,
        t.is_main,
        info.magic_type,
        policy,
    )
}

/// Вариант без registry (фасад battlefield::evaluate_position): архетип — по
/// modified-статам (magic>0 → маг, ranged>hand → стрелок).
pub fn snapshot_troop_no_reg(troop: &TroopType, index: usize, policy: &BattlePolicy) -> UnitSnapshot {
    let t = troop.get();
    build_snapshot(
        index,
        t.pos,
        (1, 1),
        t.unit.hp,
        t.unit.modified,
        t.unit.moves,
        t.is_main,
        None,
        policy,
    )
}

#[allow(clippy::too_many_arguments)]
fn build_snapshot(
    index: usize,
    pos: UnitPos,
    size: (usize, usize),
    hp: i64,
    modified: crate::units::unit::UnitStats,
    moves: i64,
    is_main: bool,
    magic_type: Option<MagicType>,
    policy: &BattlePolicy,
) -> UnitSnapshot {
    UnitSnapshot {
        index,
        pos,
        size,
        hp,
        max_hp: modified.max_hp,
        damage: modified.damage,
        defence: modified.defence,
        speed: modified.speed,
        moves,
        is_main,
        magic_type,
        value_weight: archetype_weight(magic_type, &modified.damage, is_main, policy),
    }
}

fn archetype_weight(
    magic: Option<MagicType>,
    damage: &Power,
    is_main: bool,
    policy: &BattlePolicy,
) -> f32 {
    let p = &policy.target_priority;
    let weight = if magic.is_some() || damage.magic > 0 {
        p.mage
    } else if damage.ranged > damage.hand {
        p.ranged
    } else {
        p.melee
    };
    if is_main {
        weight * p.main
    } else {
        weight
    }
}

/// Ожидаемый урон me → foe с учётом всех защит (формула compute_attack_damage).
/// ranged=true — компонента стрельбы, false — рукопашная; магия считается
/// всегда (если есть). Возврат — урон одной полной атакой.
pub fn expected_damage(me: &UnitSnapshot, foe: &UnitSnapshot, ranged: bool) -> f64 {
    let is_flank = me.pos.0.abs_diff(foe.pos.0) > 1;
    let percent_100 = Percent::new(100);
    let mut total = 0f64;

    if me.damage.magic > 0 {
        let mut magic = me.damage.magic;
        if let Some(magic_type) = me.magic_type {
            let magic_def = match magic_type {
                MagicType::LifeMagic => foe.defence.life_magic,
                MagicType::DeathMagic => foe.defence.death_magic,
                MagicType::ElementalMagic => foe.defence.elemental_magic,
            };
            magic = (percent_100 - magic_def).calc(magic.saturating_sub(foe.defence.magic_units));
        }
        total += magic as f64;
    }

    if ranged && me.damage.ranged > 0 {
        let ranged_dmg = (percent_100 - foe.defence.ranged_percent)
            .calc(me.damage.ranged.saturating_sub(foe.defence.ranged_units));
        total += ranged_dmg as f64;
    }

    if !ranged && me.damage.hand > 0 {
        let mut hand = me.damage.hand;
        let hand_defence = if is_flank {
            (foe.defence.hand_units as f32 / 2.).ceil() as u64
        } else {
            foe.defence.hand_units
        };
        hand = (percent_100 - foe.defence.hand_percent).calc(hand.saturating_sub(hand_defence));
        total += hand as f64;
    }

    total.max(1.)
}

/// Ответная угроза foe → me: урон врага, который ходит после нас и достанет
/// (dist ≤ speed), — «кто ходит после меня и достанет ли» (план §2.2).
pub fn threat_of(foe: &UnitSnapshot, me: &UnitSnapshot) -> f64 {
    let dist = foe.pos.0.abs_diff(me.pos.0) as i64;
    let reach = dist <= foe.speed.max(1) || foe.damage.ranged > 0 || foe.damage.magic > 0;
    if !reach || foe.moves < 1 {
        return 0.;
    }
    expected_damage(foe, me, true).max(expected_damage(foe, me, false))
}

/// Цена войска: hp/max_hp × вес архетипа (+бонус героя).
pub fn unit_value(troop: &UnitSnapshot) -> f64 {
    let hp_ratio = (troop.hp.max(0) as f64 / troop.max_hp.max(1) as f64).clamp(0., 1.);
    hp_ratio * troop.value_weight as f64
}

/// Позиционные слагаемые для стороны my (план §2.4):
/// - keep_backline: штраф за стрелка/мага в Front;
/// - protect_mages: штраф за врага вплотную к своей дорогой цели.
pub fn positional_score(
    mine: &[UnitSnapshot],
    foes: &[UnitSnapshot],
    max_troops: usize,
    policy: &BattlePolicy,
) -> f64 {
    let mut score = 0.;
    for me in mine {
        let field = field_type(me.pos.whole(max_troops / 2), max_troops);
        if policy.keep_backline
            && field == Field::Front
            && (me.damage.ranged > me.damage.hand || me.magic_type.is_some())
        {
            score -= 6.;
        }
        if me.magic_type.is_some() || me.value_weight > 1.0 {
            // Дорогая цель в Front (row 1) и враг в той же колонке рядом.
            let in_danger = me.pos.1 == 1
                && foes
                    .iter()
                    .any(|foe| foe.pos.0 == me.pos.0 && foe.pos.1 == 1);
            if in_danger {
                score -= 4.;
            }
        }
    }
    score
}

/// Оценка боевой позиции стороны my: сумма наших ценностей минус вражеских,
/// плюс позиционные слагаемые (план §2.2 eval_battle_position).
pub fn eval_battle_position(
    battle: &BattleInfo,
    armies: &Vec<Army>,
    my: usize,
    policy: &BattlePolicy,
    registry: &GameInfo,
) -> f64 {
    let foe = if my == battle.army1 {
        battle.army2
    } else {
        battle.army1
    };
    let max_troops = armies[my].max_troops;
    let mine = snapshot_army(armies, my, policy, registry);
    let foes = snapshot_army(armies, foe, policy, registry);
    let my_value: f64 = mine.iter().map(unit_value).sum();
    let foe_value: f64 = foes.iter().map(unit_value).sum();
    my_value - foe_value + positional_score(&mine, &foes, max_troops, policy)
}

/// Вариант без registry — фасад battlefield::evaluate_position (сигнатура
/// последней не меняется, архетипы по modified-статам).
pub fn eval_battle_position_no_reg(
    battle: &BattleInfo,
    armies: &Vec<Army>,
    my: usize,
    policy: &BattlePolicy,
) -> f64 {
    let foe = if my == battle.army1 {
        battle.army2
    } else {
        battle.army1
    };
    let max_troops = armies[my].max_troops;
    let mine: Vec<UnitSnapshot> = armies[my]
        .troops
        .iter()
        .enumerate()
        .filter(|(_, t)| !t.get().is_dead())
        .map(|(i, t)| snapshot_troop_no_reg(t, i, policy))
        .collect();
    let foes: Vec<UnitSnapshot> = armies[foe]
        .troops
        .iter()
        .enumerate()
        .filter(|(_, t)| !t.get().is_dead())
        .map(|(i, t)| snapshot_troop_no_reg(t, i, policy))
        .collect();
    let my_value: f64 = mine.iter().map(unit_value).sum();
    let foe_value: f64 = foes.iter().map(unit_value).sum();
    my_value - foe_value + positional_score(&mine, &foes, max_troops, policy)
}

/// Живые войска армии (снапшоты, короткие guard'ы).
pub fn snapshot_army(
    armies: &Vec<Army>,
    army: usize,
    policy: &BattlePolicy,
    registry: &GameInfo,
) -> Vec<UnitSnapshot> {
    armies[army]
        .troops
        .iter()
        .enumerate()
        .filter(|(_, t)| !t.get().is_dead())
        .map(|(i, t)| snapshot_troop(t, i, registry, policy))
        .collect()
}

/// Найти снапшот юнита по BattleUnitPos (по hitmap, короткие guard'ы).
pub fn snapshot_at(
    armies: &Vec<Army>,
    target: BattleUnitPos,
    policy: &BattlePolicy,
    registry: &GameInfo,
) -> Option<UnitSnapshot> {
    let army = armies.get(target.army)?;
    let index = *army.hitmap.get(target.pos)?.as_ref()?;
    let troop = army.troops.get(index)?;
    if troop.get().is_dead() {
        return None;
    }
    Some(snapshot_troop(troop, index, registry, policy))
}

/// Снапшот активного юнита.
pub fn snapshot_active(
    armies: &Vec<Army>,
    active: BattleUnit,
    policy: &BattlePolicy,
    registry: &GameInfo,
) -> Option<UnitSnapshot> {
    let troop = armies.get(active.army)?.troops.get(active.index)?;
    let snap = snapshot_troop(troop, active.index, registry, policy);
    if snap.hp < 1 {
        return None;
    }
    Some(snap)
}
