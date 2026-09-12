//! Тесты Battle AI (план §5, блок Battle). Живут в ai/tests.rs по брифу:
//! battle/tests.rs не трогаем (там чужой WIP). Реестр — конвенции
//! NOTES_battle_logic (юниты 0-19 generic, 20-24 боевые, эффекты 0-4) +
//! локальные 25 (гибридный стрелок) и 26 (снайпер) для backline/piercing-сценариев.

use crate::{
    ai::{policy::BattlePolicy, AiDecision, BattleAi},
    battle::{
        battlefield::{search_interactions, BattleUnitPos},
        Army, BattleInfo, BattleUnit,
    },
    registry::GameInfo,
    units::unit::{
        AttackSettings, LevelUpInfo, MagicDirection, MagicType, Power, UnitInfo, UnitStats,
        UnitType,
    },
};

fn ai_registry() -> GameInfo {
    let mut registry = crate::battle::tests::make_test_registry();
    // 25 = гибридный стрелок (hand 20 / ranged 10): из Back стреляет на 10,
    // из Front бьёт рукой на 20 — дискриминирует keep_backline.
    registry.units.inner.push(UnitInfo {
        id: 25,
        str_id: "hybrid_archer".into(),
        name: "Hybrid Archer".into(),
        descript: "".into(),
        cost: 0,
        cost_hire: 0,
        icon_index: 26,
        size: (1, 1),
        unit_type: UnitType::People,
        next_unit: vec![],
        magic_type: None,
        magic_direction: MagicDirection::default(),
        surrender: None,
        bonus: None,
        settings: AttackSettings::default(),
        stats: UnitStats {
            hp: 100,
            max_hp: 100,
            moves: 2,
            max_moves: 2,
            speed: 5,
            damage: Power::new(0, 10, 20),
            ..Default::default()
        },
        lvl: LevelUpInfo::default(),
    });
    // 26 = снайпер (ranged 30): достаёт и Front (dist<2), и Back (сам в Back).
    registry.units.inner.push(UnitInfo {
        id: 26,
        str_id: "sniper".into(),
        name: "Sniper".into(),
        descript: "".into(),
        cost: 0,
        cost_hire: 0,
        icon_index: 27,
        size: (1, 1),
        unit_type: UnitType::People,
        next_unit: vec![],
        magic_type: None,
        magic_direction: MagicDirection::default(),
        surrender: None,
        bonus: None,
        settings: AttackSettings::default(),
        stats: UnitStats {
            hp: 100,
            max_hp: 100,
            moves: 2,
            max_moves: 2,
            speed: 5,
            damage: Power::new(0, 30, 0),
            ..Default::default()
        },
        lvl: LevelUpInfo::default(),
    });
    registry
}

/// BattleInfo c заданным активным юнитом и свежим can_interact
/// (копия конвенции battle/tests.rs).
fn battle_with_active(armies: &Vec<Army>, active: BattleUnit, registry: &GameInfo) -> BattleInfo {
    let mut battle = BattleInfo {
        army1: 0,
        army2: 1,
        armies_moved: [
            vec![false; armies[0].max_troops],
            vec![false; armies[1].max_troops],
        ],
        ..Default::default()
    };
    battle.active_unit = Some(active);
    battle.can_interact = Some(search_interactions(&mut battle, active, armies, registry));
    battle
}

fn decide_default(
    battle: &BattleInfo,
    armies: &Vec<Army>,
    my: usize,
    registry: &GameInfo,
) -> AiDecision {
    BattleAi::decide(battle, armies, my, &BattlePolicy::default(), registry)
}

#[test]
fn ai_prefers_kill_over_damage() {
    let registry = ai_registry();
    let army1 = crate::battle::tests::make_army_from_ids(&registry, &[(26, 1)]); // снайпер в Back
    let army2 = crate::battle::tests::make_army_from_ids(&registry, &[(0, 8), (0, 2)]);
    let mut armies = vec![army1, army2];
    armies[1].troops[1].get().unit.hp = 29; // почти мёртвый в Back (слот 2)
    let battle = battle_with_active(&armies, BattleUnit { army: 0, index: 0 }, &registry);

    // Добивание (30 урона по 29 hp) ценнее такого же урона по полной цели,
    // несмотря на одинаковый вес архетипов.
    assert_eq!(
        decide_default(&battle, &armies, 0, &registry),
        AiDecision::Attack { target: BattleUnitPos { army: 1, pos: 2 } }
    );

    // Контроль: без kill_bonus выбирает бóльшую снятую долю hp (полная цель).
    let no_kill = BattlePolicy { kill_bonus: 0., ..Default::default() };
    assert_eq!(
        BattleAi::decide(&battle, &armies, 0, &no_kill, &registry),
        AiDecision::Attack { target: BattleUnitPos { army: 1, pos: 8 } }
    );
}

#[test]
fn ai_keeps_archers_in_backline() {
    let registry = ai_registry();
    let army1 = crate::battle::tests::make_army_from_ids(&registry, &[(25, 1)]); // гибрид в Back
    let army2 = crate::battle::tests::make_army_from_ids(&registry, &[(0, 8)]); // melee враг в Front
    let armies = vec![army1, army2];
    let battle = battle_with_active(&armies, BattleUnit { army: 0, index: 0 }, &registry);

    // Из Back — выстрел на 10; манёвр во Front дал бы 20 рукой, но keep_backline
    // штрафует выход стрелка в первую линию.
    assert_eq!(
        decide_default(&battle, &armies, 0, &registry),
        AiDecision::Attack { target: BattleUnitPos { army: 1, pos: 8 } }
    );

    // Контроль: без keep_backline ИИ выводит стрелка во Front ради большего урона.
    let greedy = BattlePolicy { keep_backline: false, ..Default::default() };
    assert_eq!(
        BattleAi::decide(&battle, &armies, 0, &greedy, &registry),
        AiDecision::MoveAndAttack {
            to: 7,
            target: BattleUnitPos { army: 1, pos: 8 }
        }
    );
}

#[test]
fn ai_respects_hp_threshold() {
    let registry = ai_registry();
    let army1 = crate::battle::tests::make_army_from_ids(&registry, &[(20, 7)]);
    let army2 = crate::battle::tests::make_army_from_ids(&registry, &[(0, 8)]);
    let mut armies = vec![army1, army2];
    armies[0].troops[0].get().unit.hp = 40; // 40% < порога 0.5
    let battle = battle_with_active(&armies, BattleUnit { army: 0, index: 0 }, &registry);

    // Раненый юнит не разменивается (30 урона не добивают 100 hp): отход,
    // а не атака.
    let cautious = BattlePolicy { hp_threshold_retreat: 0.5, ..Default::default() };
    let decision = BattleAi::decide(&battle, &armies, 0, &cautious, &registry);
    assert!(
        !matches!(decision, AiDecision::Attack { .. } | AiDecision::MoveAndAttack { .. }),
        "wounded unit must not trade: got {decision:?}"
    );
    if let AiDecision::Move { to } = decision {
        assert_ne!(
            crate::battle::battlefield::field_type(to, 12),
            crate::battle::Field::Front,
            "retreat must leave the front line"
        );
    }

    // Контроль: с порогом 0 (не отступать) тот же юнит атакует.
    let brave = BattlePolicy { hp_threshold_retreat: 0., ..Default::default() };
    assert_eq!(
        BattleAi::decide(&battle, &armies, 0, &brave, &registry),
        AiDecision::Attack { target: BattleUnitPos { army: 1, pos: 8 } }
    );
}

#[test]
fn ai_decision_is_deterministic() {
    let registry = ai_registry();
    let army1 = crate::battle::tests::make_army_from_ids(&registry, &[(20, 7), (26, 1)]);
    let army2 = crate::battle::tests::make_army_from_ids(&registry, &[(0, 8), (22, 2), (0, 10)]);
    let armies = vec![army1, army2];

    // Разные активные юниты (melee и снайпер против трёх целей) — повторный
    // decide даёт то же решение (нет RNG, порядок перебора стабилен).
    for active in [
        BattleUnit { army: 0, index: 0 },
        BattleUnit { army: 0, index: 1 },
        BattleUnit { army: 1, index: 0 },
    ] {
        let battle = battle_with_active(&armies, active, &registry);
        let first = decide_default(&battle, &armies, active.army, &registry);
        let second = decide_default(&battle, &armies, active.army, &registry);
        assert_eq!(first, second, "decide must be deterministic for {active:?}");
    }
}

#[test]
fn ai_uses_piercing_targeting() {
    let registry = ai_registry();
    let army1 = crate::battle::tests::make_army_from_ids(&registry, &[(26, 1)]); // снайпер в Back
    let army2 = crate::battle::tests::make_army_from_ids(&registry, &[(0, 8), (0, 2)]);
    let mut armies = vec![army1, army2];
    // Обе цели 100 hp, один вес; у первой (слот 8) дальняя защита 15.
    armies[1].troops[0].get().unit.modified.defence.ranged_units = 15;
    let battle = battle_with_active(&armies, BattleUnit { army: 0, index: 0 }, &registry);

    // При равном hp цель с низкой защитой ценнее (реальный урон 30 против 15).
    assert_eq!(
        decide_default(&battle, &armies, 0, &registry),
        AiDecision::Attack { target: BattleUnitPos { army: 1, pos: 2 } }
    );

    // Контроль: защита на другой цели — выбор переворачивается.
    let mut flipped = ai_registry();
    flipped.units.inner.push(UnitInfo::default());
    let _ = flipped;
    let army1 = crate::battle::tests::make_army_from_ids(&registry, &[(26, 1)]);
    let army2 = crate::battle::tests::make_army_from_ids(&registry, &[(0, 8), (0, 2)]);
    let mut armies = vec![army1, army2];
    armies[1].troops[1].get().unit.modified.defence.ranged_units = 15;
    let battle = battle_with_active(&armies, BattleUnit { army: 0, index: 0 }, &registry);
    assert_eq!(
        decide_default(&battle, &armies, 0, &registry),
        AiDecision::Attack { target: BattleUnitPos { army: 1, pos: 8 } }
    );
}
