//! Тесты бонусов юнитов (Bonus1-21, dt/Rus_Locale.ini «bonus_*»).
//! Изолированы от tests.rs (там сценарии occupancy другого агента).
use super::tests::{make_army_from_ids, make_test_registry, perform_attack};
use crate::{
    battle::{BattleInfo, BattleUnit},
    bonuses::{AbilityCondition, AbilityTowardsTroop, ListenTo, Mechanic, MechanicCondition},
    registry::GameInfo,
    units::{
        unit::{Unit, UnitType},
        unitstats::{Modify, ModifyUnitStats},
    },
};
use math_thingies::Percent;

/// Регистрация бонуса с BattleStart-правилом поверх тестового реестра.
fn register_battle_start_bonus(
    registry: &mut GameInfo,
    id: &str,
    mechanics: Vec<Mechanic>,
) {
    let mut rules = indexmap::IndexMap::new();
    for cond in [
        AbilityCondition::BattleStart,
        AbilityCondition::Turn,
        AbilityCondition::Kill,
        AbilityCondition::Attacked,
        AbilityCondition::Attacking,
        AbilityCondition::Moves,
        AbilityCondition::Skips,
    ] {
        let list = if cond == AbilityCondition::BattleStart {
            mechanics.clone()
        } else {
            vec![]
        };
        rules.insert(cond, (ListenTo::Myself, list));
    }
    registry.bonuses.register(
        crate::bonuses::BonusInfo {
            id: id.into(),
            name: id.into(),
            desc: "".into(),
            rules,
            ..Default::default()
        },
        id,
    );
}

/// Назначить бонус юниту армии напрямую (id в реестре уже должен быть).
fn give_bonus(armies: &mut Vec<crate::battle::Army>, army: usize, index: usize, bonus_id: &str, registry: &GameInfo) {
    let id = registry
        .bonuses
        .str_to_id(&bonus_id.to_string())
        .expect("bonus must be registered");
    let mut troop = armies[army].troops[index].get();
    troop.unit.bonus = Some(id);
}

fn full_battle(armies: &mut Vec<crate::battle::Army>, registry: &GameInfo) -> BattleInfo {
    let mut battle = BattleInfo::new(armies, 0, 1);
    battle.start(armies, registry);
    battle
}

// =====================
// BONUS 3 (SpearDefense / «Длинное Оружие»): утроенная защита на 1-м ходу
// =====================

#[test]
fn speardefense_triples_defence_on_first_turn() {
    let mut registry = make_test_registry();
    // hand_units 10 → на BattleStart ×3 (percent_add 200) на 1 ход с decay.
    register_battle_start_bonus(
        &mut registry,
        "speardefense",
        vec![Mechanic {
            affects_self: true,
            conditions: MechanicCondition::True,
            ability: vec![AbilityTowardsTroop::AddEffectModify {
                modify: ModifyUnitStats {
                    defence: crate::units::unitstats::ModifyDefence {
                        hand_units: Modify {
                            percent_add: Some(math_thingies::Percent::new(200)),
                            ..Default::default()
                        },
                        ..Default::default()
                    },
                    ..Default::default()
                },
                lifetime: crate::effects::EffectLifetime {
                    lifetime: Some(1),
                    decay: 1,
                    remove_on_battle_end: true,
                },
            }],
            affects: Vec::new(),
            works_after_death: false,
        }],
    );
    // База hand_units = 0; percent-модификатор от нуля не виден — задаём базу
    // в РЕЕСТРЕ (recalc восстанавливает modified от base-статов).
    registry.units[0].stats.defence.hand_units = 10;
    let mut army1 = make_army_from_ids(&registry, &[(0, 7)]);
    {
        let id = registry.bonuses.str_to_id(&"speardefense".to_string()).unwrap();
        let mut troop = army1.troops[0].get();
        troop.unit.bonus = Some(id);
    }
    let army2 = make_army_from_ids(&registry, &[(1, 8)]);
    let mut armies = vec![army1, army2];

    full_battle(&mut armies, &registry);

    let after_start = armies[0].troops[0].get().unit.modified.defence.hand_units;
    assert_eq!(
        after_start, 30,
        "speardefense must triple hand_units defence at battle start"
    );
}

// =====================
// BONUS 2 (FastAttack / «Быстрая атака»): +1 манёвр на 1-м ходу
// =====================

#[test]
fn fastattack_grants_extra_move_on_first_turn() {
    let mut registry = make_test_registry();
    register_battle_start_bonus(
        &mut registry,
        "fastattack",
        vec![Mechanic {
            affects_self: true,
            conditions: MechanicCondition::True,
            ability: vec![AbilityTowardsTroop::AddEffectModify {
                modify: ModifyUnitStats {
                    max_moves: Modify { add: Some(1), ..Default::default() },
                    ..Default::default()
                },
                lifetime: crate::effects::EffectLifetime {
                    lifetime: Some(1),
                    decay: 1,
                    remove_on_battle_end: true,
                },
            }],
            affects: Vec::new(),
            works_after_death: false,
        }],
    );
    let mut army1 = make_army_from_ids(&registry, &[(0, 7)]);
    {
        let id = registry.bonuses.str_to_id(&"fastattack".to_string()).unwrap();
        let mut troop = army1.troops[0].get();
        troop.unit.bonus = Some(id);
    }
    let army2 = make_army_from_ids(&registry, &[(1, 8)]);
    let mut armies = vec![army1, army2];

    full_battle(&mut armies, &registry);

    let unit = armies[0].troops[0].get().unit.clone();
    assert_eq!(
        unit.modified.max_moves, 4,
        "fastattack must add +1 move at battle start (3 base → 4)"
    );
    assert_eq!(
        unit.moves, 4,
        "start() sets moves to modified.max_moves — bonus move must be usable"
    );
}

// =====================
// BONUS 19/«Шквальная Атака»-часть Squall («Всегда первый»): макс. скорость на BattleStart
// =====================

#[test]
fn squall_moves_first_in_order_via_max_speed() {
    let mut registry = make_test_registry();
    // Медленный юнит со «Шквалом»: скорость становится 100 — выше любого врага.
    register_battle_start_bonus(
        &mut registry,
        "squall",
        vec![Mechanic {
            affects_self: true,
            conditions: MechanicCondition::True,
            ability: vec![AbilityTowardsTroop::AddEffectModify {
                modify: ModifyUnitStats {
                    speed: Modify { set: Some(100), ..Default::default() },
                    ..Default::default()
                },
                lifetime: crate::effects::EffectLifetime {
                    lifetime: Some(1),
                    decay: 1,
                    remove_on_battle_end: true,
                },
            }],
            affects: Vec::new(),
            works_after_death: false,
        }],
    );
    let mut army1 = make_army_from_ids(&registry, &[(0, 7)]); // speed 5
    {
        let id = registry.bonuses.str_to_id(&"squall".to_string()).unwrap();
        let mut troop = army1.troops[0].get();
        troop.unit.bonus = Some(id);
    }
    let army2 = make_army_from_ids(&registry, &[(0, 8)]); // speed 5
    let mut armies = vec![army1, army2];

    let mut battle = full_battle(&mut armies, &registry);

    assert_eq!(
        battle.active_unit,
        Some(BattleUnit { army: 0, index: 0 }),
        "squall unit must act first despite equal base speed"
    );
}

// =====================
// Артиллерия (Bonus19-часть «+30 инициативы в начале битвы») — через speed add 30
// =====================

#[test]
fn artillery_gains_30_initiative_at_start() {
    let mut registry = make_test_registry();
    register_battle_start_bonus(
        &mut registry,
        "artillery",
        vec![Mechanic {
            affects_self: true,
            conditions: MechanicCondition::True,
            ability: vec![AbilityTowardsTroop::AddEffectModify {
                modify: ModifyUnitStats {
                    speed: Modify { add: Some(30), ..Default::default() },
                    ..Default::default()
                },
                lifetime: crate::effects::EffectLifetime {
                    lifetime: Some(1),
                    decay: 1,
                    remove_on_battle_end: true,
                },
            }],
            affects: Vec::new(),
            works_after_death: false,
        }],
    );
    let mut army1 = make_army_from_ids(&registry, &[(0, 7)]);
    {
        let id = registry.bonuses.str_to_id(&"artillery".to_string()).unwrap();
        let mut troop = army1.troops[0].get();
        troop.unit.bonus = Some(id);
    }
    let army2 = make_army_from_ids(&registry, &[(0, 8)]);
    let mut armies = vec![army1, army2];

    full_battle(&mut armies, &registry);

    assert_eq!(
        armies[0].troops[0].get().unit.modified.speed, 35,
        "artillery must gain +30 speed at battle start (5 base → 35)"
    );
}

// =====================
// BONUS 3 / 14 / 19-«Шквал»-часть (Проникающий Удар): pierce — доля
// units-защиты цели, которую удар проходит мимо. Бонус = стат pierce
// (add_modify), движок читает универсальный стат в compute_attack_damage.
// =====================

#[test]
fn defencepiercing_bypasses_unit_defence() {
    let registry = make_test_registry();
    // Атакующий 20 (30 рукопашного), цель — юнит 0 с сильной units-защитой.
    let mut army1 = make_army_from_ids(&registry, &[(20, 7)]);
    {
        let id = registry.bonuses.str_to_id(&"defencepiercing".to_string()).unwrap();
        let mut troop = army1.troops[0].get();
        troop.unit.bonus = Some(id);
    }
    let mut army2 = make_army_from_ids(&registry, &[(0, 8)]);
    {
        // hand_units 20: без pierce 30−20=10; с pierce 100% защита мимо → 30.
        let mut troop = army2.troops[0].get();
        troop.unit.modified.defence.hand_units = 20;
    }
    let mut armies = vec![army1, army2];

    // Старт боя применяет add_modify бонуса (pierce 100%), затем атака.
    full_battle(&mut armies, &registry);
    perform_attack(
        &registry,
        &mut armies,
        BattleUnit { army: 0, index: 0 },
        BattleUnit { army: 1, index: 0 },
    );

    let hp = armies[1].troops[0].get().unit.hp;
    assert_eq!(
        hp, 70,
        "pierce 100% must fully bypass hand_units defence: 30 dmg, hp 100−30"
    );
}

#[test]
fn pierce_is_percent_of_defence_not_extra_damage() {
    let registry = make_test_registry();
    let mut army1 = make_army_from_ids(&registry, &[(20, 7)]);
    {
        // Тот же канал, что у бонуса: modified.pierce.percent_add 50% —
        // сквозь защиту идёт 50% урона: 30 − (20*50%)=10 → 20 урона.
        let mut troop = army1.troops[0].get();
        troop.unit.modified.pierce.percent_add = Some(Percent::new(50));
    }
    let mut army2 = make_army_from_ids(&registry, &[(0, 8)]);
    {
        let mut troop = army2.troops[0].get();
        troop.unit.modified.defence.hand_units = 20;
    }
    let mut armies = vec![army1, army2];

    perform_attack(
        &registry,
        &mut armies,
        BattleUnit { army: 0, index: 0 },
        BattleUnit { army: 1, index: 0 },
    );

    assert_eq!(
        armies[1].troops[0].get().unit.hp, 80,
        "pierce 50% must halve effective unit defence (30 − 10 = 20 урона)"
    );
}

// =====================
// BONUS 7/8 (Кара Господня / Гнев Господен): +10/+20 урона сверх атаки,
// игнорируя любые защиты — через стат true_damage (add_modify бонуса).
// =====================

#[test]
fn godanger_adds_10_true_damage() {
    let registry = make_test_registry();
    let mut army1 = make_army_from_ids(&registry, &[(20, 7)]);
    {
        let id = registry.bonuses.str_to_id(&"godanger".to_string()).unwrap();
        let mut troop = army1.troops[0].get();
        troop.unit.bonus = Some(id);
    }
    // Цель с огромной units-защитой: обычный урон уходит в 0, true_damage нет.
    let mut army2 = make_army_from_ids(&registry, &[(0, 8)]);
    {
        let mut troop = army2.troops[0].get();
        troop.unit.modified.defence.hand_units = 1000;
    }
    let mut armies = vec![army1, army2];

    full_battle(&mut armies, &registry);
    perform_attack(
        &registry,
        &mut armies,
        BattleUnit { army: 0, index: 0 },
        BattleUnit { army: 1, index: 0 },
    );

    let hp = armies[1].troops[0].get().unit.hp;
    assert_eq!(
        hp, 90,
        "godanger: 30 hand урона в ноль от защиты, но +10 true_damage мимо неё"
    );
}

#[test]
fn godstrike_adds_20_true_damage() {
    let registry = make_test_registry();
    let mut army1 = make_army_from_ids(&registry, &[(20, 7)]);
    {
        let id = registry.bonuses.str_to_id(&"godstrike".to_string()).unwrap();
        let mut troop = army1.troops[0].get();
        troop.unit.bonus = Some(id);
    }
    let mut army2 = make_army_from_ids(&registry, &[(0, 8)]);
    {
        let mut troop = army2.troops[0].get();
        troop.unit.modified.defence.hand_units = 1000;
    }
    let mut armies = vec![army1, army2];

    full_battle(&mut armies, &registry);
    perform_attack(
        &registry,
        &mut armies,
        BattleUnit { army: 0, index: 0 },
        BattleUnit { army: 1, index: 0 },
    );

    assert_eq!(
        armies[1].troops[0].get().unit.hp, 80,
        "godstrike: обычный урон в ноль, +20 true_damage игнорирует защиту"
    );
}

#[test]
fn true_damage_reverts_after_battle_end() {
    let registry = make_test_registry();
    let mut army1 = make_army_from_ids(&registry, &[(20, 7)]);
    {
        let id = registry.bonuses.str_to_id(&"godanger".to_string()).unwrap();
        let mut troop = army1.troops[0].get();
        troop.unit.bonus = Some(id);
    }
    let army2 = make_army_from_ids(&registry, &[(0, 8)]);
    let mut armies = vec![army1, army2];

    let mut battle = full_battle(&mut armies, &registry);
    assert_eq!(
        armies[0].troops[0].get().unit.modified.true_damage.add,
        Some(10),
        "в бою бонус поднял true_damage +10"
    );
    battle.winner = Some(0);
    battle.end(&mut armies, &registry);
    assert_ne!(
        armies[0].troops[0].get().unit.modified.true_damage.add,
        Some(10),
        "после боя add_modify бонуса откатывается (+10 больше не действует)"
    );
}

// =====================
// BONUS 4 (Лекарское Умение): отряд с медиком восстанавливает 15% жизни
// всем раненым воинам каждый день (AbilityCondition::Day, Executor::advance_day).
// =====================

#[test]
fn medic_heals_15_percent_per_day() {
    let (mut gamemap, registry) = super::tests::make_map_with_two_armies();
    // Раненый юнит армии-игрока.
    gamemap.armys[0].troops[0].get().unit.hp = 40;
    // Медик — третья армия карты: ранен (20/100), Day-правило лечит свою армию.
    gamemap.armys.push({
        let mut medic_army = make_army_from_ids(&registry, &[(0, 0)]);
        medic_army.pos = (2, 1);
        medic_army.active = true;
        medic_army.troops[0].get().unit.hp = 20;
        {
            let id = registry.bonuses.str_to_id(&"medic".to_string()).unwrap();
            let mut troop = medic_army.troops[0].get();
            troop.unit.bonus = Some(id);
        }
        medic_army
    });
    let mut executor = crate::network::server::Executor {
        gamemap,
        events: vec![],
        battle: None,
        execution_queue: vec![],
        players: vec![crate::battle::control::Player {
            army: 0,
            questbook: None,
            execution_queue: vec![],
            wait_until: None,
        }],
        last_day: 0,
    };

    executor.advance_day(&registry);

    assert_eq!(
        executor.gamemap.armys[0].troops[0].get().unit.hp,
        40,
        "медик чужой армии не лечит игрока"
    );
    assert_eq!(
        executor.gamemap.armys[2].troops[0].get().unit.hp,
        35,
        "медик лечит свою армию (себя): 20 + 15% от 100 = 35"
    );
}

