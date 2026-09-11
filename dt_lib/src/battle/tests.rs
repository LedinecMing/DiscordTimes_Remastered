use crate::{
    battle::{
        Army, ArmyStats, BattleInfo, BattleUnit, BattleUnitInfo,
        Field, TroopType,
        battlefield::{field_type, possible_movement, unit_interaction, check_win,
                      handle_action, search_interactions, BattleUnitPos},
        troop::Troop, control::Control,
    },
    registry::GameInfo,
    units::{
        unit::{AttackSettings, LevelUpInfo, MagicDirection, MagicType, Power, Unit, UnitInfo,
               UnitInventory, UnitLvl, UnitPos, UnitStats, UnitType, ActionResult, Defence,
               attack},
        unitstats::{ModifyUnitStats, Modify, ModifyPower},
    },
    items::item::{Item, ItemInfo, ArtifactType, WeaponType, MagicVariants},
    map::object::{Market, RecruitUnit, Recruitment},
    bonuses::BonusInfo,
    effects::{EffectInfo, EffectLifetime},
};
use rand::prelude::IteratorRandom;
use rand::thread_rng;
use math_thingies::Percent;

fn make_test_registry() -> GameInfo {
    let mut registry = GameInfo::new();

    // Register generic units 0..19 (unit 1 is Undead)
    for i in 0..20 {
        registry.units.inner.push(UnitInfo {
            id: i,
            str_id: format!("generic_unit_{}", i),
            name: format!("Generic Unit {}", i),
            descript: "".into(),
            cost: 0, cost_hire: 0, icon_index: i + 1,
            size: (1, 1),
            unit_type: if i == 1 { UnitType::Undead } else { UnitType::People },
            next_unit: vec![],
            magic_type: None,
            magic_direction: Default::default(),
            surrender: None,
            bonus: None,
            settings: AttackSettings::default(),
            stats: UnitStats {
                hp: 100, max_hp: 100, moves: 3, max_moves: 3, speed: 5,
                damage: Power::new(0, 0, 15),
                ..Default::default()
            },
            lvl: LevelUpInfo::default(),
        });
    }

    // Unit 20 = melee attacker (30 hand damage)
    registry.units.inner.push(UnitInfo {
        id: 20, str_id: "melee_attacker".into(), name: "Melee Attacker".into(), descript: "".into(),
        cost: 0, cost_hire: 0, icon_index: 20, size: (1, 1), unit_type: UnitType::People,
        next_unit: vec![], magic_type: None, magic_direction: Default::default(),
        surrender: None, bonus: None, settings: AttackSettings::default(),
        stats: UnitStats {
            hp: 100, max_hp: 100, moves: 3, max_moves: 3, speed: 5,
            damage: Power::new(0, 0, 30), ..Default::default()
        },
        lvl: LevelUpInfo::default(),
    });

    // Unit 21 = defender with surrender
    registry.units.inner.push(UnitInfo {
        id: 21, str_id: "defender".into(), name: "Defender".into(), descript: "".into(),
        cost: 0, cost_hire: 0, icon_index: 21, size: (1, 1), unit_type: UnitType::People,
        next_unit: vec![], magic_type: None, magic_direction: Default::default(),
        surrender: Some(10), bonus: None, settings: AttackSettings::default(),
        stats: UnitStats {
            hp: 100, max_hp: 100, moves: 3, max_moves: 3, speed: 5,
            damage: Power::new(0, 0, 0), ..Default::default()
        },
        lvl: LevelUpInfo::default(),
    });

    // Unit 22 = death mage (ToAll, 40 magic)
    registry.units.inner.push(UnitInfo {
        id: 22, str_id: "death_mage".into(), name: "Death Mage".into(), descript: "".into(),
        cost: 0, cost_hire: 0, icon_index: 22, size: (1, 1), unit_type: UnitType::People,
        next_unit: vec![], magic_type: Some(MagicType::DeathMagic),
        magic_direction: MagicDirection::ToAll,
        surrender: None, bonus: None, settings: AttackSettings::default(),
        stats: UnitStats {
            hp: 80, max_hp: 80, moves: 2, max_moves: 2, speed: 4,
            damage: Power::new(40, 0, 0), ..Default::default()
        },
        lvl: LevelUpInfo::default(),
    });

    // Unit 23 = life mage (ToAll, 30 magic)
    registry.units.inner.push(UnitInfo {
        id: 23, str_id: "life_mage".into(), name: "Life Mage".into(), descript: "".into(),
        cost: 0, cost_hire: 0, icon_index: 23, size: (1, 1), unit_type: UnitType::People,
        next_unit: vec![], magic_type: Some(MagicType::LifeMagic),
        magic_direction: MagicDirection::ToAll,
        surrender: None, bonus: None, settings: AttackSettings::default(),
        stats: UnitStats {
            hp: 80, max_hp: 80, moves: 2, max_moves: 2, speed: 4,
            damage: Power::new(30, 0, 0), ..Default::default()
        },
        lvl: LevelUpInfo::default(),
    });

    // Эффекты, нужные боевым веткам: add_modify_effect пишет id 2 (mage_curse).
	registry.effects.inner.push(EffectInfo {
        id: "internal".into(),
        name: "internal".into(),
        desc: "".into(),
        kind: "internal".into(),
        lifetime: EffectLifetime::default(),
        stacks: true,
        added_modify: Default::default(),
        rules: Default::default(),
        power_scales: false,
        power_scales_with_lifetime: false,
    });

    // Unit 24 = elemental mage (ToEnemy) — аналог Архимага из Units.ini
    registry.units.inner.push(UnitInfo {
        id: 24, str_id: "elemental_mage".into(), name: "Elemental Mage".into(), descript: "".into(),
        cost: 0, cost_hire: 0, icon_index: 24, size: (1, 1), unit_type: UnitType::People,
        next_unit: vec![], magic_type: Some(MagicType::ElementalMagic),
        magic_direction: MagicDirection::ToEnemy,
        surrender: None, bonus: None, settings: AttackSettings::default(),
        stats: UnitStats {
            hp: 50, max_hp: 50, moves: 2, max_moves: 2, speed: 8,
            damage: Power { magic: 25, ranged: 0, hand: 0 },
            defence: Defence::default(),
            flank_mod: Default::default(), pierce: Default::default(),
            true_damage: Default::default(), vamp: Default::default(),
            regen: Default::default(),
        },
        lvl: LevelUpInfo::default(),
    });

    registry.effects.inner.push(EffectInfo {
        id: "mage_support".into(),
        name: "Mage Support".into(),
        desc: "".into(),
        kind: "mage_support".into(),
        lifetime: EffectLifetime::default(),
        stacks: true,
        added_modify: Default::default(),
        rules: Default::default(),
        power_scales: false,
        power_scales_with_lifetime: false,
    });
    registry.effects.inner.push(EffectInfo {
        id: "mage_curse".into(),
        name: "Mage Curse".into(),
        desc: "".into(),
        kind: "mage_curse".into(),
        lifetime: EffectLifetime::default(),
        stacks: false,
        added_modify: Default::default(),
        rules: Default::default(),
        power_scales: false,
        power_scales_with_lifetime: false,
    });
    registry.effects.inner.push(EffectInfo {
        id: "abstract_effect".into(),
        name: "Abstract".into(),
        desc: "".into(),
        kind: "abstract".into(),
        lifetime: EffectLifetime::default(),
        stacks: true,
        added_modify: Default::default(),
        rules: Default::default(),
        power_scales: true,
        power_scales_with_lifetime: false,
    });
	registry.effects.inner.push(EffectInfo {
        id: "elemental_support".into(),
        name: "Elemental Support".into(),
        desc: "".into(),
        kind: "elemental_support".into(),
        lifetime: EffectLifetime::default(),
        stacks: true,
        added_modify: Default::default(),
        rules: Default::default(),
        power_scales: false,
        power_scales_with_lifetime: false,
    });
    registry
}

/// Create an army from registry unit IDs placed at given hitmap positions.
fn make_army_from_ids(registry: &GameInfo, unit_placements: &[(u32, usize)]) -> Army {
    let mut army = Army::new(
        vec![], ArmyStats::default(), vec![],
        (0, 0), true, Control::PC, registry,
    );
    for &(unit_id, position) in unit_placements {
        let id = unit_id as usize;
        let unit_info_stats = registry.units[id].stats.clone();
        let unit = Unit {
            id,
            hp: unit_info_stats.hp,
            moves: unit_info_stats.moves,
            modified: unit_info_stats,
            modify: ModifyUnitStats::default(),
            settings: registry.units[id].settings.clone(),
            lvl: UnitLvl::default(),
            bonus: None,
            effects: vec![],
            inventory: UnitInventory { items: vec![None; 4] },
        };
        let mut troop = Troop::new(unit);
        troop.pos = UnitPos::from_index(position, 6);
        army.troops.push(troop.into());
    }
    army.recalc_army_hitmap(&registry.units);
    army
}

/// Perform a single attack between two units, returning the action result.
fn perform_attack(
    registry: &GameInfo,
    armies: &mut Vec<Army>,
    attacker: BattleUnit,
    target: BattleUnit,
) -> Option<ActionResult> {
    let battle_info = BattleInfo {
        army1: 0, army2: 1,
        armies_moved: [
            vec![false; armies[0].max_troops],
            vec![false; armies[1].max_troops],
        ],
        ..Default::default()
    };
    let effects = attack(attacker, target, armies, registry)?;
    crate::units::unit::apply_attack(effects, attacker, target, armies, &battle_info, registry);
    Some(effects)
}

/// Create a generic unit struct with given parameters, used for non‑army tests.
fn make_test_unit(
    id: usize, hp: i64, moves: i64, speed: i64,
    hand_damage: u64, ranged_damage: u64, magic_damage: u64,
) -> Unit {
    Unit {
        id, hp, moves,
        modified: UnitStats {
            hp, max_hp: hp, moves, max_moves: moves, speed,
            damage: Power::new(magic_damage, ranged_damage, hand_damage),
            ..Default::default()
        },
        modify: ModifyUnitStats::default(),
        settings: AttackSettings::default(),
        lvl: UnitLvl::default(),
        bonus: None,
        effects: vec![],
        inventory: UnitInventory { items: vec![None; 4] },
    }
}

// =====================
// BATTLE FLOW TESTS
// =====================

#[test]
fn full_battle_runs_without_crash() {
    let registry = make_test_registry();
    let army1 = make_army_from_ids(&registry, &[(20, 7), (20, 8)]);
    let army2 = make_army_from_ids(&registry, &[(20, 10)]);
    let mut armies = vec![army1, army2];
    let mut battle = BattleInfo::new(&mut armies, 0, 1);
    for _ in 0..10 {
        if battle.winner.is_some() {
            break;
        }
        // Perform a random interaction if available
        if let Some(interactions) = &battle.can_interact.clone() {
            if let Some(interaction) = interactions.iter().choose(&mut thread_rng()) {
                unit_interaction(&mut battle, &mut armies,
                    (interaction.army, BattleUnitInfo::Pos(interaction.pos)), &registry);
            }
        }
        battle.after_single_move(&mut armies, &registry);
    }
}

#[test]
fn surrender_triggers_win_for_opponent() {
    let registry = make_test_registry();
    let army1 = make_army_from_ids(&registry, &[(21, 7)]);  // unit with surrender
    let mut army2 = make_army_from_ids(&registry, &[(20, 8)]);
    {
        let mut troop = army2.troops[0].get();
        troop.unit.hp = 0;
    }
    let mut armies = vec![army1, army2];
    let mut battle = BattleInfo::new(&mut armies, 0, 1);
    check_win(&mut battle, &armies, &registry);
    assert_eq!(battle.winner, Some(1), "Opponent should win when one army surrenders");
}

#[test]
fn max_move_count_triggers_win() {
    let registry = make_test_registry();
    let army1 = make_army_from_ids(&registry, &[(0, 7)]);
    let army2 = make_army_from_ids(&registry, &[(0, 8)]);
    let mut armies = vec![army1, army2];
    let mut battle = BattleInfo::new(&mut armies, 0, 1);
    battle.move_count = 25;
    check_win(&mut battle, &armies, &registry);
    assert!(battle.winner.is_some(), "Battle should end when max moves reached");
}

// =====================
// DAMAGE AND DEFENCE TESTS
// =====================

#[test]
fn melee_attack_deals_damage() {
    let registry = make_test_registry();
    let army1 = make_army_from_ids(&registry, &[(20, 7)]);
    let army2 = make_army_from_ids(&registry, &[(0, 8)]);
    let mut armies = vec![army1, army2];
    perform_attack(&registry, &mut armies, BattleUnit { army: 0, index: 0 }, BattleUnit { army: 1, index: 0 });
    assert!(armies[1].troops[0].get().unit.hp < 100, "Target should take damage");
}

#[test]
fn hand_defence_reduces_damage() {
    let registry = make_test_registry();
    let army1 = make_army_from_ids(&registry, &[(20, 7)]);
    let mut army2 = make_army_from_ids(&registry, &[(0, 8)]);
    {
        let mut troop = army2.troops[0].get();
        troop.unit.modified.defence.hand_units = 15;
    }
    let mut armies = vec![army1, army2];
    perform_attack(&registry, &mut armies, BattleUnit { army: 0, index: 0 }, BattleUnit { army: 1, index: 0 });
    assert!(armies[1].troops[0].get().unit.hp > 70, "Hand defence should block some damage");
}

#[test]
fn percent_defence_reduces_ranged_damage() {
    let registry = make_test_registry();
    let mut army1 = make_army_from_ids(&registry, &[(0, 1)]); // back row
    {
        let mut troop = army1.troops[0].get();
        troop.unit.modified.damage.ranged = 30;
    }
    let mut army2 = make_army_from_ids(&registry, &[(0, 8)]);
    {
        let mut troop = army2.troops[0].get();
        troop.unit.modified.defence.ranged_percent = Percent::new(50);
    }
    let mut armies = vec![army1, army2];
    perform_attack(&registry, &mut armies, BattleUnit { army: 0, index: 0 }, BattleUnit { army: 1, index: 0 });
    assert!(armies[1].troops[0].get().unit.hp > 50, "Percent defence should block");
}

#[test]
fn magic_defence_reduces_magic_damage() {
    let registry = make_test_registry();
    // Use magic with strike (damages directly without effect) — position back (pos 1) vs front (pos 10)
    let mut army1 = make_army_from_ids(&registry, &[(0, 1)]); // back row
    {
        let mut troop = army1.troops[0].get();
        troop.unit.modified.damage = Power::new(40, 0, 0); // 40 magic damage
    }
    let mut army2 = make_army_from_ids(&registry, &[(0, 10)]);
    {
        let mut troop = army2.troops[0].get();
        troop.unit.modified.defence.death_magic = Percent::new(50);
    }
    let mut armies = vec![army1, army2];
    perform_attack(&registry, &mut armies, BattleUnit { army: 0, index: 0 }, BattleUnit { army: 1, index: 0 });
    assert!(armies[1].troops[0].get().unit.hp > 60, "Magic defence should block: hp={}", armies[1].troops[0].get().unit.hp);
}

#[test]
fn vampirism_heals_attacker() {
    let registry = make_test_registry();
    let mut army1 = make_army_from_ids(&registry, &[(20, 7)]);
    {
        let mut troop = army1.troops[0].get();
        troop.unit.modified.vamp = Percent::new(50);
        // Set hp lower so healing is visible
        troop.unit.hp = 50;
        troop.unit.modified.hp = 50;
    }
    let army2 = make_army_from_ids(&registry, &[(0, 8)]);
    let mut armies = vec![army1, army2];
    let hp_before = armies[0].troops[0].get().unit.hp;
    perform_attack(&registry, &mut armies, BattleUnit { army: 0, index: 0 }, BattleUnit { army: 1, index: 0 });
    let hp_after = armies[0].troops[0].get().unit.hp;
    assert!(hp_after > hp_before, "Vampirism should heal attacker: {} -> {}", hp_before, hp_after);
}

#[test]
fn regen_heals_on_tick() {
    let registry = make_test_registry();
    let mut army = make_army_from_ids(&registry, &[(0, 7)]);
    {
        let mut troop = army.troops[0].get();
        troop.unit.hp = 50;
        troop.unit.modified.regen = Percent::new(10);
        troop.unit.modified.max_hp = 100;
    }
    let hp_before = army.troops[0].get().unit.hp;
    army.troops[0].get().unit.tick(&registry);
    assert!(army.troops[0].get().unit.hp > hp_before, "Regen should heal");
}

#[test]
fn true_damage_bypasses_defence() {
    let registry = make_test_registry();
    let mut army1 = make_army_from_ids(&registry, &[(20, 7)]);
    let mut army2 = make_army_from_ids(&registry, &[(0, 8)]);
    {
        let mut troop = army2.troops[0].get();
        troop.unit.modified.defence.hand_units = 200;
    }
    {
        let mut troop = army1.troops[0].get();
        troop.unit.modified.true_damage = Modify { add: Some(10), ..Default::default() };
    }
    let mut armies = vec![army1, army2];
    perform_attack(&registry, &mut armies, BattleUnit { army: 0, index: 0 }, BattleUnit { army: 1, index: 0 });
    assert!(armies[1].troops[0].get().unit.hp < 100, "True damage should bypass defence");
}

// =====================
// CORRECT_DAMAGE FORMULA TESTS
// =====================

#[test]
fn correct_damage_basic() {
    let registry = make_test_registry();
    let unit = make_test_unit(0, 100, 3, 5, 0, 0, 0);
    let result = unit.correct_damage(&Power::new(0, 0, 50), None, None);
    assert_eq!(result.hand, 50, "No defence should take full damage");
}

#[test]
fn correct_damage_with_defence() {
    let registry = make_test_registry();
    let mut unit = make_test_unit(0, 100, 3, 5, 0, 0, 0);
    unit.modified.defence.hand_units = 10;
    let result = unit.correct_damage(&Power::new(0, 0, 50), None, None);
    assert_eq!(result.hand, 40, "Defence should subtract from damage");
}

#[test]
fn flank_increases_damage() {
    let registry = make_test_registry();
    let mut unit = make_test_unit(0, 100, 3, 5, 0, 0, 0);
    unit.modified.defence.hand_units = 20;
    let normal = unit.correct_damage(&Power::new(0, 0, 30), None, None);
    let flank = unit.correct_damage(&Power::new(0, 0, 30), None, Some(1.0));
    assert!(flank.hand > normal.hand, "Flank should deal more damage than normal attack");
}

// =====================
// RANGED ATTACK TESTS
// =====================

#[test]
fn ranged_attack_from_back_row_hits() {
    let registry = make_test_registry();
    let mut army1 = make_army_from_ids(&registry, &[(0, 1)]); // back row
    {
        let mut troop = army1.troops[0].get();
        troop.unit.modified.damage.ranged = 25;
    }
    let army2 = make_army_from_ids(&registry, &[(0, 8)]);
    let mut armies = vec![army1, army2];
    perform_attack(&registry, &mut armies, BattleUnit { army: 0, index: 0 }, BattleUnit { army: 1, index: 0 });
    assert!(armies[1].troops[0].get().unit.hp < 100, "Ranged attack should damage target");
}

#[test]
fn back_row_cannot_melee_without_ranged() {
    let registry = make_test_registry();
    let army1 = make_army_from_ids(&registry, &[(0, 1)]); // back row, only hand damage
    let army2 = make_army_from_ids(&registry, &[(0, 8)]);
    let mut armies = vec![army1, army2];
    let result = perform_attack(
        &registry, &mut armies,
        BattleUnit { army: 0, index: 0 },
        BattleUnit { army: 1, index: 0 },
    );
    assert!(result.is_none(), "Back row without ranged/magic should not attack");
}

// =====================
// RESERVE TESTS
// =====================

#[test]
fn reserve_cannot_attack_without_perk() {
    let registry = make_test_registry();
    let mut army1 = make_army_from_ids(&registry, &[(20, 0)]); // reserve
    {
        let mut troop = army1.troops[0].get();
        troop.unit.settings.attacks_from_reserve = false;
    }
    let army2 = make_army_from_ids(&registry, &[(0, 8)]);
    let mut armies = vec![army1, army2];
    let result = perform_attack(
        &registry, &mut armies,
        BattleUnit { army: 0, index: 0 },
        BattleUnit { army: 1, index: 0 },
    );
    assert!(result.is_none(), "Reserve unit without perk should not attack");
}

#[test]
fn reserve_can_attack_with_perk() {
    let registry = make_test_registry();
    let mut army1 = make_army_from_ids(&registry, &[(20, 0)]); // reserve
    {
        let mut troop = army1.troops[0].get();
        troop.unit.settings.attacks_from_reserve = true;
    }
    let army2 = make_army_from_ids(&registry, &[(0, 8)]);
    let mut armies = vec![army1, army2];
    let result = perform_attack(
        &registry, &mut armies,
        BattleUnit { army: 0, index: 0 },
        BattleUnit { army: 1, index: 0 },
    );
    assert!(result.is_some(), "Reserve unit with perk should be able to attack");
}

// =====================
// MAGIC INTERACTION TESTS
// =====================

#[test]
fn magic_all_targets_enemy() {
    let registry = make_test_registry();
    // Position mage at back (pos 1) and enemy further away (pos 8)
    // distance = 8 - 1 = 7 > 1 → magic should work
    let army1 = make_army_from_ids(&registry, &[(22, 1)]);
    let army2 = make_army_from_ids(&registry, &[(0, 8)]);
    let armies = vec![army1, army2];
    // Guard'ы troops не держим: attack захватывает те же мьютексы (SendMut).
    let can_target = attack(
        BattleUnit { army: 0, index: 0 },
        BattleUnit { army: 1, index: 0 },
        &armies, &registry,
    );
    assert!(can_target.is_some(), "ToAll mage should be able to target enemy");
}

#[test]
fn mage_has_magic_power() {
    let registry = make_test_registry();
    let army = make_army_from_ids(&registry, &[(22, 7)]);
    assert!(army.troops[0].get().unit.modified.damage.magic > 0, "Mage should have magic power");
}

// =====================
// ITEM TESTS
// =====================

#[test]
fn equip_and_remove_restores_stats() {
    let mut registry = make_test_registry();
    registry.items.inner.push(ItemInfo {
        name: "Sword".into(), description: "".into(),
        cost: 10, icon: "".into(), sells: false,
        itemtype: ArtifactType::Weapon(WeaponType::Hand),
        magic_req: MagicVariants::Any, bonus: None,
        modify: ModifyUnitStats {
            damage: ModifyPower { hand: Modify { add: Some(10), ..Default::default() }, ..Default::default() },
            ..Default::default()
        },
    });

    let mut unit = make_test_unit(20, 100, 3, 5, 30, 0, 0);
    let base_damage = unit.modified.damage.hand;

    assert!(unit.add_item(Some(Item { index: 0 }), 0, &registry), "Should equip item");
    assert_eq!(unit.modified.damage.hand, base_damage + 10, "Stats should increase after equip");

    unit.remove_item(0, &registry);
    assert_eq!(unit.modified.damage.hand, base_damage, "Stats should return to base after remove");
}

#[test]
fn set_modifier_ordering_works_correctly() {
    let mut registry = make_test_registry();

    for &set_value in &[25i64, 50] {
        registry.items.inner.push(ItemInfo {
            name: "Ring".into(), description: "".into(),
            cost: 10, icon: "".into(), sells: false,
            itemtype: ArtifactType::Ring,
            magic_req: MagicVariants::Any, bonus: None,
            modify: ModifyUnitStats {
                damage: ModifyPower {
                    hand: Modify { set: Some(set_value), ..Default::default() },
                    ..Default::default()
                },
                ..Default::default()
            },
        });
    }

    let mut unit = Unit {
        id: 20, hp: 100, moves: 3,
        modified: registry.units[20].stats.clone(),
        modify: ModifyUnitStats::default(),
        settings: AttackSettings::default(),
        lvl: UnitLvl::default(),
        inventory: UnitInventory { items: vec![None; 4] },
        bonus: None,
        effects: vec![],
    };
    assert_eq!(unit.modified.damage.hand, 30, "Initial damage should be 30");

    // Equip ring (set=25), damage becomes 25
    unit.add_item(Some(Item { index: 0 }), 0, &registry);
    assert_eq!(unit.modified.damage.hand, 25, "Ring set=25 should apply");

    // Remove ring, damage returns to 30
    unit.remove_item(0, &registry);
    assert_eq!(unit.modified.damage.hand, 30, "After remove, base 30 should restore");

    // Equip the other ring (set=50), damage becomes 50
    unit.add_item(Some(Item { index: 1 }), 0, &registry);
    assert_eq!(unit.modified.damage.hand, 50, "Ring set=50 should apply");

    // Remove it, damage returns to 30
    unit.remove_item(0, &registry);
    assert_eq!(unit.modified.damage.hand, 30, "After remove, base 30 should restore");
}

#[test]
fn item_modify_applies_and_removes() {
    let mut registry = make_test_registry();

    registry.items.inner.push(ItemInfo {
        name: "Sword".into(), description: "".into(),
        cost: 10, icon: "".into(), sells: false,
        itemtype: ArtifactType::Weapon(WeaponType::Hand),
        magic_req: MagicVariants::Any, bonus: None,
        modify: ModifyUnitStats {
            damage: ModifyPower { hand: Modify { add: Some(10), ..Default::default() }, ..Default::default() },
            ..Default::default()
        },
    });

    let mut unit = make_test_unit(20, 100, 3, 5, 30, 0, 0);
    let base_damage = unit.modified.damage.hand;

    assert!(unit.add_item(Some(Item { index: 0 }), 0, &registry), "Should equip item");
    assert_eq!(unit.modified.damage.hand, base_damage + 10, "Item modify should increase damage");

    unit.remove_item(0, &registry);
    assert_eq!(unit.modified.damage.hand, base_damage, "After remove, damage should restore");
}

#[test]
fn item_magic_restriction_blocks_unequipped_unit_types() {
    let mut registry = make_test_registry();

    registry.items.inner.push(ItemInfo {
        name: "Holy Amulet".into(), description: "".into(),
        cost: 10, icon: "".into(), sells: false,
        itemtype: ArtifactType::Amulet,
        magic_req: MagicVariants::Life, bonus: None,
        modify: ModifyUnitStats::default(),
    });

    // Unit without magic type should not be able to equip Life‑req item
    let mut normal_unit = make_test_unit(0, 100, 3, 5, 0, 0, 0);
    let result = normal_unit.add_item(Some(Item { index: 0 }), 0, &registry);
    assert!(!result, "Unit without magic should not equip Life‑required item");
}

#[test]
fn item_magic_type_allows_equip_for_matching_mage() {
    let mut registry = make_test_registry();

    registry.items.inner.push(ItemInfo {
        name: "Death Amulet".into(), description: "".into(),
        cost: 10, icon: "".into(), sells: false,
        itemtype: ArtifactType::Amulet,
        magic_req: MagicVariants::Death, bonus: None,
        modify: ModifyUnitStats::default(),
    });

    // Unit 22 is a Death mage — should equip Death‑req item
    let mut death_mage = make_test_unit(22, 80, 2, 4, 0, 0, 40);
    let result = death_mage.add_item(Some(Item { index: 0 }), 0, &registry);
    assert!(result, "Death mage should equip Death‑required item");
}

// =====================
// MOVEMENT TESTS
// =====================

#[test]
fn unit_can_move_to_adjacent_position() {
    let registry = make_test_registry();
    let army1 = make_army_from_ids(&registry, &[(0, 7)]);
    let army2 = make_army_from_ids(&registry, &[(0, 8)]);
    let mut armies = vec![army1, army2];
    let mut battle = BattleInfo::new(&mut armies, 0, 1);
    battle.active_unit = Some(BattleUnit { army: 0, index: 0 });

    let old_position = armies[0].troops[0].get().pos;
    unit_interaction(&mut battle, &mut armies, (0, BattleUnitInfo::Pos(8)), &registry);
    assert_ne!(
        old_position.whole(6),
        armies[0].troops[0].get().pos.whole(6),
        "Unit position should change after move",
    );
}

#[test]
fn unit_can_move_to_reserve() {
    let registry = make_test_registry();
    let army1 = make_army_from_ids(&registry, &[(0, 7)]);
    let army2 = make_army_from_ids(&registry, &[(0, 8)]);
    let mut armies = vec![army1, army2];
    let mut battle = BattleInfo::new(&mut armies, 0, 1);
    battle.active_unit = Some(BattleUnit { army: 0, index: 0 });

    let result = unit_interaction(&mut battle, &mut armies, (0, BattleUnitInfo::Pos(0)), &registry);
    assert_eq!(result, Some(ActionResult::Move), "Move to reserve should succeed");
}

// =====================
// FIELD TYPE TESTS
// =====================

#[test]
fn field_type_distribution_is_correct() {
    let fields: Vec<Field> = (0..12).map(|index| field_type(index, 12)).collect();
    use crate::battle::Field::*;
    assert_eq!(
        fields,
        vec![
            Reserve, Back, Back, Back, Back, Reserve,
            Reserve, Front, Front, Front, Front, Reserve,
        ],
    );
}

#[test]
fn possible_movement_returns_non_empty_list() {
    assert!(!possible_movement(3, 6).is_empty(), "Possible moves should not be empty");
}

// =====================
// EDGE CASE TESTS
// =====================

#[test]
fn effect_removal_restores_stats() {
    let mut registry = make_test_registry();

    registry.effects.inner.push(crate::effects::effect::EffectInfo {
        id: "test_effect".into(), kind: "buff".into(), name: "Test Buff".into(),
        desc: "".into(), rules: Default::default(),
        lifetime: crate::effects::effect::EffectLifetime {
            lifetime: Some(1), decay: 1, remove_on_battle_end: true,
        },
        stacks: true, power_scales: true, power_scales_with_lifetime: false,
        added_modify: ModifyUnitStats {
            damage: ModifyPower { hand: Modify { add: Some(20), ..Default::default() }, ..Default::default() },
            ..Default::default()
        },
    });

    let mut unit = make_test_unit(0, 100, 3, 5, 15, 0, 0);
    let base_damage = unit.modified.damage.hand;

    // Add effect through modify and recalc
    unit.modify += ModifyUnitStats {
        damage: ModifyPower { hand: Modify { add: Some(20), ..Default::default() }, ..Default::default() },
        ..Default::default()
    };
    unit.recalc(&registry);
    assert_eq!(unit.modified.damage.hand, base_damage + 20, "Effect should increase damage");

    // Clear the modify to simulate effect removal
    unit.modify = ModifyUnitStats::default();
    unit.recalc(&registry);
    assert_eq!(unit.modified.damage.hand, base_damage, "After clearing modify, stats should restore");
}

#[test]
fn empty_army_does_not_crash() {
    let registry = make_test_registry();
    let army1 = Army::new(vec![], ArmyStats::default(), vec![], (0, 0), true, Control::PC, &registry);
    let army2 = Army::new(vec![], ArmyStats::default(), vec![], (0, 0), true, Control::PC, &registry);
    let mut armies = vec![army1, army2];
    let mut battle = BattleInfo::new(&mut armies, 0, 1);
    battle.after_single_move(&mut armies, &registry);
}

#[test]
fn large_damage_kills_target() {
    let registry = make_test_registry();
    let mut army1 = make_army_from_ids(&registry, &[(20, 7)]);
    {
        let mut troop = army1.troops[0].get();
        troop.unit.modified.damage.hand = 9999;
    }
    let army2 = make_army_from_ids(&registry, &[(0, 8)]);
    let mut armies = vec![army1, army2];
    perform_attack(&registry, &mut armies, BattleUnit { army: 0, index: 0 }, BattleUnit { army: 1, index: 0 });
    assert_eq!(armies[1].troops[0].get().unit.hp, 0, "Massive damage should kill target");
}

    // =====================
    // MAGE FULL INTERACTION PATH (handle_action)
    // =====================

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

    #[test]
    fn mage_debuffs_enemy_via_handle_action() {
        let registry = make_test_registry();
        let army1 = make_army_from_ids(&registry, &[(22, 1)]); // death mage, back
        let army2 = make_army_from_ids(&registry, &[(0, 8)]); // enemy
        let mut armies = vec![army1, army2];
        let active = BattleUnit { army: 0, index: 0 };
        let mut battle = battle_with_active(&armies, active, &registry);
		armies[0].troops[0].get().unit.moves = 2;
		armies[1].troops[0].get().unit.moves = 2;
        // can_interact должен покрывать врага (иначе UI не рисует цель)
        assert!(
            battle
                .can_interact
                .as_ref()
                .unwrap()
                .contains(&BattleUnitPos { army: 1, pos: 8 }),
            "can_interact must contain enemy pos 8 for ToAll death mage"
        );

        let before = armies[1].troops[0].get().unit.hp;
        let effects = handle_action((8, 1), &mut battle, &mut armies, &registry);
        assert_eq!(effects.map(|(r, _)| r), Some(ActionResult::Debuff));
		
        let after = armies[1].troops[0].get().unit.hp;
        assert!(before == after, "enemy should NOT take magic damage ({} -> {})", before, after);
		// Should have a curse effect
		assert!(armies[1].troops[0].get().unit.effects.len() > 0);
		// Second time - should strike
		battle.active_unit = Some(active);
		dbg!(&armies[1].troops[0].get().unit.effects);
		let effects = handle_action((8, 1), &mut battle, &mut armies, &registry);
		assert_eq!(effects.map(|(r, _)| r), Some(ActionResult::MagicDamage));
		let after = armies[1].troops[0].get().unit.hp;
        assert!(before > after, "enemy should take magic damage ({} -> {})", before, after);
    }

    #[test]
    fn mage_heals_undead_ally_via_handle_action() {
        let registry = make_test_registry();
        // Death-маг лечит (necromancy) Undead-союзника
        let army1 = make_army_from_ids(&registry, &[(22, 1), (1, 0)]);
        let army2 = make_army_from_ids(&registry, &[(0, 8)]);
        {
            let mut ally = army1.troops[1].get();
            ally.unit.hp = 10; // ранен
        }
        let mut armies = vec![army1, army2];
        let active = BattleUnit { army: 0, index: 0 };
        let mut battle = battle_with_active(&armies, active, &registry);

        assert!(
            battle
                .can_interact
                .as_ref()
                .unwrap()
                .contains(&BattleUnitPos { army: 0, pos: 0 }),
            "can_interact must contain wounded undead ally pos 0"
        );

        let before = armies[1 - 1].troops[1].get().unit.hp;
        let effects = handle_action((0, 0), &mut battle, &mut armies, &registry);
        assert_eq!(effects.map(|(r, _)| r), Some(ActionResult::Buff));
        let after = armies[0].troops[1].get().unit.hp;
        assert!(after > before, "undead ally must be healed ({} -> {})", before, after);
    }

    #[test]
    fn mage_cannot_death_heal_people_ally() {
        let registry = make_test_registry();
        // Death-маг + People-союзник: can_attack запрещает, интеракция = None
        let army1 = make_army_from_ids(&registry, &[(22, 1), (0, 0)]);
        let army2 = make_army_from_ids(&registry, &[(0, 8)]);
        {
            let mut ally = army1.troops[1].get();
            ally.unit.hp = 10;
        }
        let mut armies = vec![army1, army2];
        let active = BattleUnit { army: 0, index: 0 };
        let mut battle = battle_with_active(&armies, active, &registry);

        assert!(
            !battle
                .can_interact
                .as_ref()
                .unwrap()
                .contains(&BattleUnitPos { army: 0, pos: 0 }),
            "death mage must NOT target people ally"
        );

        let before = armies[0].troops[1].get().unit.hp;
        let effects = handle_action((0, 0), &mut battle, &mut armies, &registry);
        assert!(effects.is_none(), "invalid ally target must yield no action");
        let after = armies[0].troops[1].get().unit.hp;
        assert_eq!(after, before, "ally hp must be unchanged");
    }

    #[test]
    fn front_row_mage_vs_blocked_path_game_layout() {
        // Репродукция игрового кейса: Elemental ToEnemy маг стоит во ФРОНТОВОМ
        // ряду (слот 8 -> Field::Front), перед ним свои (слот 7), враг во
        // фронте врага (слот 10) с прикрытием (слот 9). is_path_empty = false,
        // is_in_back = false, field_checks = false (враг).
        let registry = make_test_registry();
        let army1 = make_army_from_ids(&registry, &[(20, 7), (24, 8)]); // свой фронт + маг
        let army2 = make_army_from_ids(&registry, &[(0, 9), (0, 10)]); // враг с прикрытием
        let mut armies = vec![army1, army2];
        let active = BattleUnit { army: 0, index: 1 }; // маг
        let mut battle = battle_with_active(&armies, active, &registry);

        let interact = battle.can_interact.as_ref().unwrap();
        println!("DEBUG front mage can_interact = {:?}", interact);
        assert!(
            interact.contains(&BattleUnitPos { army: 1, pos: 10 }),
            "front-row elemental mage MUST be able to strike enemy (game reports no magic at all)"
        );
        let effects = handle_action((10, 1), &mut battle, &mut armies, &registry);
        assert!(effects.is_some(), "handle_action must apply magic damage");
    }

    #[test]
    fn back_row_mage_control_group() {
        // Контроль: тот же маг в заднем ряду колдует нормально.
        let registry = make_test_registry();
        let army1 = make_army_from_ids(&registry, &[(24, 1)]); // маг в бэке
        let army2 = make_army_from_ids(&registry, &[(0, 9)]);
        let mut armies = vec![army1, army2];
        let active = BattleUnit { army: 0, index: 0 };
        let mut battle = battle_with_active(&armies, active, &registry);
        let effects = handle_action((9, 1), &mut battle, &mut armies, &registry);
        assert!(effects.is_some(), "back-row mage must cast");
    }

    // =====================
    // UNITS.INI PARSING (гипотеза: Magic=... не парсится в magic_type)
    // =====================

    #[test]
    fn units_ini_archmage_full_section_parses() {
        use advini::Sections;
        // Секция скопирована дословно из dt/Units.ini ([2 Архимаг]).
        let ini = r#"
[2 Архимаг]
GlobalIndex=2
Name=Архимаг
Descript=Архимаг способен обращаться к могущественным силам мирозданья, неподвластным простым людям.
Cost=300
CostMultipler=100
CostGoldDiv=1
Magic=ElementalMagic
MagicDirection=ToEnemy
StartExpirience=90
LevelMultipler=160
IconIndex=76
// боевые характеристики
Hits=50
MagicPower=25
ProtectLife=35
ProtectDeath=35
ProtectElemental=35
Initiative=26
Manevres=2
// поуровневые изменения характеристики
d-Hits=4
d-MagicPower=5
d-ProtectLife=10
d-ProtectDeath=10
d-ProtectElemental=10
d-Initiative=1
"#;
        let sections = advini::parse_for_sections(ini);
        assert_eq!(sections.len(), 1, "one section expected");
        let (unit, _) = UnitInfo::from_section(sections[0].1.clone(), Default::default()).unwrap();
        println!("PARSED UNIT = {unit:#?}");

        // КРИТИЧНО: тип и направление магии обязаны распарситься, иначе маг
        // в бою не сможет колдовать (attack вернёт None).
        assert_eq!(
            unit.magic_type,
            Some(MagicType::ElementalMagic),
            "Magic=ElementalMagic must parse into Some(Elemental)"
        );
        assert_eq!(unit.magic_direction, MagicDirection::ToEnemy);

        // Остальные поля секции — на месте (маппинг: Hits->max_hp,
        // MagicPower->damage.magic, Manevres->max_moves, Initiative->speed).
        assert_eq!(unit.name, "Архимаг");
        assert_eq!(unit.stats.damage.magic, 25, "MagicPower -> damage.magic");
        assert_eq!(unit.stats.max_hp, 50, "Hits -> max_hp");
        assert_eq!(unit.stats.max_moves, 2, "Manevres -> max_moves");
        assert_eq!(unit.stats.speed, 26, "Initiative -> speed");
        // GlobalIndex и IconIndex алиасятся на одно поле — побеждает первый
        // найденный ключ (GlobalIndex=2). Поведение парсера, не бага магии.
        assert_eq!(unit.icon_index, 2);
    }

    #[test]
    fn elemental_mage_cannot_buff_elemental_mage() {
        // Правило пользователя: элементальную магию нельзя баффать на
        // элементального мага (он и так под своим стихийным резонансом).
        let registry = make_test_registry();
        let army1 = make_army_from_ids(&registry, &[(24, 1), (24, 2)]); // маг + маг-союзник
        let army2 = make_army_from_ids(&registry, &[(0, 8)]);
        let mut armies = vec![army1, army2];
        let active = BattleUnit { army: 0, index: 0 }; // элементальный маг
        let mut battle = battle_with_active(&armies, active, &registry);

        assert!(
            !battle
                .can_interact
                .as_ref()
                .unwrap()
                .contains(&BattleUnitPos { army: 0, pos: 2 }),
            "elemental ally mage must NOT be a buff target"
        );

        let effects = handle_action((2, 0), &mut battle, &mut armies, &registry);
        assert!(effects.is_none(), "buff on elemental mage must be rejected");
    }

    #[test]
    fn magic_cannot_target_enemy_in_reserve() {
        // Враг в резерве (slot 0 = Field::Reserve) недостижим для ЛЮБОЙ магии.
        let registry = make_test_registry();
        let army1 = make_army_from_ids(&registry, &[(22, 1)]); // death mage ToAll, back
        let army2 = make_army_from_ids(&registry, &[(0, 0)]); // враг в резерве
        let mut armies = vec![army1, army2];
        armies[0].troops[0].get().unit.moves = 2;
        armies[1].troops[0].get().unit.moves = 2;
        let active = BattleUnit { army: 0, index: 0 };
        let mut battle = battle_with_active(&armies, active, &registry);

        assert!(
            !battle
                .can_interact
                .as_ref()
                .unwrap()
                .contains(&BattleUnitPos { army: 1, pos: 0 }),
            "reserve enemy must not be in can_interact"
        );
        let effects = handle_action((0, 1), &mut battle, &mut armies, &registry);
        assert!(effects.is_none(), "magic vs reserve enemy must be forbidden");
    }

    #[test]
    fn blessed_ally_not_target_and_move_not_wasted() {
        // Уже благословлённый союзник: не цель, повторный клик не тратит ход.
        let registry = make_test_registry();
        let army1 = make_army_from_ids(&registry, &[(22, 1), (1, 0)]); // маг + undead союзник
        let army2 = make_army_from_ids(&registry, &[(0, 8)]);
        let mut armies = vec![army1, army2];
        armies[0].troops[0].get().unit.moves = 3;
        let mage = BattleUnit { army: 0, index: 0 };

        // Первый каст: благословление ложится (эффект id 1).
        let mut battle = battle_with_active(&armies, mage, &registry);
        let effects = handle_action((0, 0), &mut battle, &mut armies, &registry);
        assert_eq!(effects.map(|(r, _)| r), Some(ActionResult::Buff));
        assert!(armies[0].troops[1].get().unit.has_effect_id(1), "bless must land");

        // Новый ход того же мага: повторный каст на благословлённого = None,
        // маневры не тратятся.
        let moves_before = armies[0].troops[0].get().unit.moves;
        let mut battle = battle_with_active(&armies, mage, &registry);
        assert!(
            !battle
                .can_interact
                .as_ref()
                .unwrap()
                .contains(&BattleUnitPos { army: 0, pos: 0 }),
            "blessed ally must not be in can_interact"
        );
        let effects = handle_action((0, 0), &mut battle, &mut armies, &registry);
        assert!(effects.is_none(), "re-bless must be rejected");
        let moves_after = armies[0].troops[0].get().unit.moves;
        assert_eq!(
            moves_after, moves_before,
            "rejected action must not consume maneuvers"
        );
    }

    #[test]
    fn life_mage_heals_wounded_people_ally() {
        // Life-маг (ToAll) лечит раненого People-союзника: Buff + HP растёт.
        let registry = make_test_registry();
        let army1 = make_army_from_ids(&registry, &[(23, 1), (0, 0)]);
        let army2 = make_army_from_ids(&registry, &[(0, 8)]);
        {
            let mut ally = army1.troops[1].get();
            ally.unit.hp = 10;
        }
        let mut armies = vec![army1, army2];
        armies[0].troops[0].get().unit.moves = 2;
        let active = BattleUnit { army: 0, index: 0 };
        let mut battle = battle_with_active(&armies, active, &registry);

        assert!(
            battle
                .can_interact
                .as_ref()
                .unwrap()
                .contains(&BattleUnitPos { army: 0, pos: 0 }),
            "wounded people ally must be a valid life-mage target"
        );
        let before = armies[0].troops[1].get().unit.hp;
        let effects = handle_action((0, 0), &mut battle, &mut armies, &registry);
        assert_eq!(effects.map(|(r, _)| r), Some(ActionResult::Buff));
        let after = armies[0].troops[1].get().unit.hp;
        assert!(after > before, "ally must be healed ({} -> {})", before, after);
    }

    #[test]
    fn turn_rollover_recomputes_can_interact() {
        // Когда маневры текущего хода кончились и ход переходит дальше,
        // can_interact ОБЯЗАН пересчитаться под нового активного юнита,
        // а не остаться мусором от прошлого активного.
        let registry = make_test_registry();
        let army1 = make_army_from_ids(&registry, &[(22, 1)]); // death mage
        let army2 = make_army_from_ids(&registry, &[(0, 8)]);
        let mut armies = vec![army1, army2];
        let mage = BattleUnit { army: 0, index: 0 };
        let mut battle = battle_with_active(&armies, mage, &registry);

        // Симуляция перехода хода: активный сброшен, can_interact — мусор.
        battle.active_unit = None;
        battle.can_interact = Some(vec![BattleUnitPos { army: 1, pos: 8 }]);

        battle.after_single_move(&mut armies, &registry);

        let expected = battle
            .active_unit
            .map(|a| search_interactions(&mut battle, a, &armies, &registry));
        assert_eq!(
            battle.can_interact, expected,
            "can_interact must be recomputed for the new active unit on turn rollover"
        );
    }

    #[test]
    fn reserve_mage_cannot_cast_on_allies_without_perk() {
        // Маг в резерве БЕЗ перка attacks_from_reserve не действует вовсе,
        // в том числе на своих в резерве.
        let registry = make_test_registry();
        let army1 = make_army_from_ids(&registry, &[(22, 0), (1, 2)]); // маг в резерве + союзник вне резерва
        let army2 = make_army_from_ids(&registry, &[(0, 8)]);
        {
            let mut ally = army1.troops[1].get();
            ally.unit.hp = 10;
        }
        let mut armies = vec![army1, army2];
        armies[0].troops[0].get().unit.moves = 2;
        let active = BattleUnit { army: 0, index: 0 }; // маг в резерве
        let mut battle = battle_with_active(&armies, active, &registry);

        assert!(
            !battle
                .can_interact
                .as_ref()
                .unwrap()
                .contains(&BattleUnitPos { army: 0, pos: 2 }),
            "reserve mage without perk must not act on allies outside reserve"
        );
        let before = armies[0].troops[1].get().unit.hp;
        let effects = handle_action((2, 0), &mut battle, &mut armies, &registry);
        assert!(effects.is_none(), "reserve cast without perk must be rejected");
        let after = armies[0].troops[1].get().unit.hp;
        assert_eq!(after, before, "no effect must apply");
    }

    #[test]
    fn reserve_mage_with_perk_casts_on_reserve_ally() {
        // С перком attacks_from_reserve маг в резерве действует на своих
        // в резерве.
        let registry = make_test_registry();
        let army1 = make_army_from_ids(&registry, &[(22, 0), (1, 5)]);
        let army2 = make_army_from_ids(&registry, &[(0, 8)]);
        {
            let mut ally = army1.troops[1].get();
            ally.unit.hp = 10;
        }
        {
            let mut mage = army1.troops[0].get();
            mage.unit.settings.attacks_from_reserve = true;
        }
        let mut armies = vec![army1, army2];
        armies[0].troops[0].get().unit.moves = 2;
        let active = BattleUnit { army: 0, index: 0 };
        let mut battle = battle_with_active(&armies, active, &registry);

        assert!(
            battle
                .can_interact
                .as_ref()
                .unwrap()
                .contains(&BattleUnitPos { army: 0, pos: 5 }),
            "reserve ally must be targetable with attacks_from_reserve perk"
        );
        let before = armies[0].troops[1].get().unit.hp;
        let effects = handle_action((5, 0), &mut battle, &mut armies, &registry);
        assert_eq!(effects.map(|(r, _)| r), Some(ActionResult::Buff));
        let after = armies[0].troops[1].get().unit.hp;
        assert!(after > before, "ally must be healed ({} -> {})", before, after);
    }

    #[test]
    fn reserve_mage_interacts_with_reserve_ally_without_perk() {
        // Оба союзных юнита в резерве — взаимодействовать можно (field_checks,
        // правило «оба в резерве и союзники — можно»), перк не требуется.
        let registry = make_test_registry();
        let army1 = make_army_from_ids(&registry, &[(22, 0), (1, 5)]); // оба в резерве
        let army2 = make_army_from_ids(&registry, &[(0, 8)]);
        {
            let mut ally = army1.troops[1].get();
            ally.unit.hp = 10;
        }
        let mut armies = vec![army1, army2];
        armies[0].troops[0].get().unit.moves = 2;
        let active = BattleUnit { army: 0, index: 0 }; // маг в резерве
        let mut battle = battle_with_active(&armies, active, &registry);

        assert!(
            battle
                .can_interact
                .as_ref()
                .unwrap()
                .contains(&BattleUnitPos { army: 0, pos: 5 }),
            "both-in-reserve allies must be able to interact"
        );
        let before = armies[0].troops[1].get().unit.hp;
        let effects = handle_action((5, 0), &mut battle, &mut armies, &registry);
        assert_eq!(effects.map(|(r, _)| r), Some(ActionResult::Buff));
        let after = armies[0].troops[1].get().unit.hp;
        assert!(after > before, "ally must be healed ({} -> {})", before, after);
    }

    // =====================
    // ЭФФЕКТЫ: тик/спад, конец боя, статы, стакание
    // =====================

    #[test]
    fn effect_adds_and_reverts_stats() {
        let registry = make_test_registry();
        // Юнит от РЕЕСТРОВОЙ базы: recalc считает от registry.units[20], а не
        // от крафтовых статов make_test_unit.
        let mut unit = Unit {
            id: 20,
            hp: registry.units[20].stats.hp,
            moves: registry.units[20].stats.moves,
            modified: registry.units[20].stats.clone(),
            modify: ModifyUnitStats::default(),
            settings: registry.units[20].settings.clone(),
            lvl: UnitLvl::default(),
            bonus: None,
            effects: vec![],
            inventory: UnitInventory { items: vec![None; 4] },
        };
        let base_hand = unit.modified.damage.hand;

        let modify = ModifyUnitStats {
            damage: crate::units::unitstats::ModifyPower {
                hand: Modify { add: Some(-3), ..Default::default() },
                ..Default::default()
            },
            ..Default::default()
        };
        assert!(crate::effects::effect::add_modify_effect(&mut unit, modify.clone(), 2, &registry));
        // В игре modified пересчитывается на тике; в тесте — явно.
        unit.recalc(&registry);
        assert!(unit.has_effect_id(2), "curse effect must be present");
        assert!(
            unit.modified.damage.hand < base_hand,
            "curse must reduce hand attack ({} -> {})",
            base_hand, unit.modified.damage.hand
        );

        // Откат: removal возвращает added_modify из modify.
        let effect = unit.get_effect_by_id(2).unwrap().clone();
        effect.removal(&mut unit, &registry);
        unit.recalc(&registry);
        assert_eq!(
            unit.modified.damage.hand, base_hand,
            "removal must restore attack ({} -> {})",
            base_hand, unit.modified.damage.hand
        );
    }

    #[test]
    fn stacking_true_effect_accumulates_lifetime() {
        // Стакающийся эффект (id 1, stacks=true): повторное наложение
        // суммирует lifetime и возвращает true.
        let registry = make_test_registry();
        let mut unit = make_test_unit(20, 100, 3, 5, 65, 0, 0);
        let modify = ModifyUnitStats::default();

        assert!(crate::effects::effect::add_modify_effect(&mut unit, modify.clone(), 1, &registry), "first bless ok");
        assert!(crate::effects::effect::add_modify_effect(&mut unit, modify, 1, &registry), "second bless stacks");
        let effect = unit.get_effect_by_id(1).unwrap();
        assert_eq!(
            effect.lifetime.lifetime,
            Some(2),
            "two blesses with lifetime 1 must stack to 2"
        );
        assert_eq!(unit.effects.iter().filter(|e| e.id == 1).count(), 1, "one slot");
    }

    #[test]
    fn stacking_false_effect_rejects_second_add() {
        // Не-стакающийся эффект (id 2, stacks=false): повторное наложение
        // возвращает false, слот один.
        let registry = make_test_registry();
        let mut unit = make_test_unit(20, 100, 3, 5, 65, 0, 0);
        let modify = ModifyUnitStats::default();

        assert!(crate::effects::effect::add_modify_effect(&mut unit, modify.clone(), 2, &registry), "first curse ok");
        assert!(
            !crate::effects::effect::add_modify_effect(&mut unit, modify, 2, &registry),
            "non-stacking curse must reject second add"
        );
        assert_eq!(unit.effects.iter().filter(|e| e.id == 2).count(), 1, "one slot");
    }

    #[test]
    fn effects_decay_and_expire_on_turn_tick() {
        // Абстрактный эффект (id 4, lifetime Some(1), decay 1) обязан спасть
        // на следующем ходу: тик обнуляет lifetime и снимает эффект,
        // добавленные модификаторы откатываются.
        let registry = make_test_registry();
        // База от реестра: recalc считает от registry.units[20].
        let mut unit = Unit {
            id: 20,
            hp: registry.units[20].stats.hp,
            moves: registry.units[20].stats.moves,
            modified: registry.units[20].stats.clone(),
            modify: ModifyUnitStats::default(),
            settings: registry.units[20].settings.clone(),
            lvl: UnitLvl::default(),
            bonus: None,
            effects: vec![],
            inventory: UnitInventory { items: vec![None; 4] },
        };
        let base_hand = registry.units[20].stats.damage.hand;
        let modify = ModifyUnitStats {
            damage: crate::units::unitstats::ModifyPower {
                hand: Modify { add: Some(-2), ..Default::default() },
                ..Default::default()
            },
            ..Default::default()
        };
        assert!(crate::effects::effect::add_modify_effect(&mut unit, modify, 4, &registry));
        assert!(unit.has_effect_id(4));

        unit.tick(&registry);
        assert!(
            !unit.has_effect_id(4),
            "effect with lifetime 1 must expire after one tick"
        );
        assert_eq!(unit.modified.damage.hand, base_hand, "stats must revert");
    }

    #[test]
    fn battle_end_removes_battle_effects() {
        // remove_on_battle_end=true: по окончании боя эффекты сняты,
        // добавленные модификаторы откачены.
        let registry = make_test_registry();
        let army1 = make_army_from_ids(&registry, &[(20, 7)]);
        let army2 = make_army_from_ids(&registry, &[(0, 8)]);
        let mut armies = vec![army1, army2];
        let base_hand = armies[0].troops[0].get().unit.modified.damage.hand;

        let mut battle = BattleInfo {
            army1: 0, army2: 1,
            armies_moved: [
                vec![false; armies[0].max_troops],
                vec![false; armies[1].max_troops],
            ],
            ..Default::default()
        };
        let modify = ModifyUnitStats {
            damage: crate::units::unitstats::ModifyPower {
                hand: Modify { add: Some(-3), ..Default::default() },
                ..Default::default()
            },
            ..Default::default()
        };
        {
            let mut t = armies[0].troops[0].get();
            assert!(crate::effects::effect::add_modify_effect(&mut t.unit, modify, 2, &registry));
        }
        battle.winner = Some(0);

        battle.end(&mut armies, &registry);

        let t = armies[0].troops[0].get();
        assert!(
            !t.unit.has_effect_id(2),
            "battle end must clear battle effects"
        );
        assert_eq!(
            t.unit.modified.damage.hand, base_hand,
            "battle end must revert added stat modifies"
        );
    }

// =====================
// BUILDING SERVICES (найм/лечение/воскрешение/рынок)
// =====================

fn make_building_registry() -> GameInfo {
    let mut registry = make_test_registry();
    // Юнит 0: cost_hire = 50, cost = 100 для ценовых тестов.
    registry.units[0].cost_hire = 50;
    registry.units[0].cost = 100;
    // Предмет 0: стоимость 100, продаётся; предмет 1: не продаётся (sells=false).
        registry.items.inner.push(ItemInfo {
        name: "Sword".into(),
        description: "".into(),
        cost: 100,
        icon: "".into(),
        sells: true,
        itemtype: ArtifactType::Item,
        magic_req: MagicVariants::Any,
        bonus: None,
        modify: Default::default(),
    });
        registry.items.inner.push(ItemInfo {
        name: "QuestItem".into(),
        description: "".into(),
        cost: 500,
        icon: "".into(),
        sells: false,
        itemtype: ArtifactType::Item,
        magic_req: MagicVariants::Any,
        bonus: None,
        modify: Default::default(),
    });
    registry
}

#[test]
fn recruitment_buy_spends_gold_and_adds_troop() {
    let mut registry = make_building_registry();
    let mut army = make_army_from_ids(&registry, &[(20, 7)]);
    army.stats.gold = 200;
    let mut rec = Recruitment::new(vec![RecruitUnit { unit: 0, count: 2 }], 1.0);
    // Успех: 50 <= 200, счётчик 2 -> 1, золото 200 -> 150, юнит добавлен.
    assert!(rec.buy(&mut army, 0, &registry).is_ok());
    assert_eq!(army.stats.gold, 150);
    assert_eq!(rec.units[0].count, 1);
    assert_eq!(army.troops.len(), 2);
    // Исчерпание счётчика: count=0 -> Err, золото не меняется.
    assert!(rec.buy(&mut army, 0, &registry).is_ok());
    assert_eq!(rec.units[0].count, 0);
    let gold = army.stats.gold;
    assert!(rec.buy(&mut army, 0, &registry).is_err());
    assert_eq!(army.stats.gold, gold);
}

#[test]
fn market_buy_takes_item_and_gold() {
    let registry = make_building_registry();
    let mut army = make_army_from_ids(&registry, &[(20, 7)]);
    army.stats.gold = 150;
    let mut market = Market::new((0, 1000), vec![Item { index: 0 }], 5);
    market.buy(&mut army, 0, &registry.items);
    assert_eq!(army.stats.gold, 50); // 150 - 100
    assert_eq!(market.items.len(), 0);
    assert!(army.inventory.iter().any(|it| it.is_some()));
}

#[test]
fn market_sell_pays_half_and_refills() {
    let registry = make_building_registry();
    let mut army = make_army_from_ids(&registry, &[(20, 7)]);
    army.stats.gold = 0;
    army.add_item(Item { index: 0 });
    let mut market = Market::new((0, 1000), vec![], 5);
    let revenue = market.sell(&mut army, 0, &registry.items).unwrap();
    assert_eq!(revenue, 50); // 50% от 100
    assert_eq!(army.stats.gold, 50);
    assert!(army.inventory[0].is_none(), "слот инвентаря освобождён");
    assert_eq!(market.items.len(), 1, "предмет вернулся на рынок");
    // Пустой слот продать нельзя.
    assert!(market.sell(&mut army, 0, &registry.items).is_err());
}

#[test]
fn heal_for_gold_partial_and_full() {
    let registry = make_building_registry();
    let mut army = make_army_from_ids(&registry, &[(0, 7)]);
    // Раним юнита: 100 -> 40.
    army.troops[0].get().unit.hp = 40;
    let troop = &mut army.troops[0];
    let mut troop = troop.get();
    // Частичное лечение: 30 золота = 30 HP.
    let spent = crate::map::object::heal_for_gold(&mut troop, 30, &registry).unwrap();
    assert_eq!(spent, 30);
    assert_eq!(troop.unit.hp, 70);
    // Лечение до конца: просим 1000, потратится только 30 (недостаёт 30).
    let spent = crate::map::object::heal_for_gold(&mut troop, 1000, &registry).unwrap();
    assert_eq!(spent, 30);
    assert_eq!(troop.unit.hp, 100);
    // Полный юнит лечить нельзя.
    assert!(crate::map::object::heal_for_gold(&mut troop, 10, &registry).is_err());
    let cost = crate::map::object::heal_cost(&troop);
    assert_eq!(cost, 0);
}

#[test]
fn resurrect_dead_troop_costs_hire() {
    let registry = make_building_registry();
    let mut army = make_army_from_ids(&registry, &[(0, 7)]);
    army.stats.gold = 200;
    // Убиваем юнита.
    army.troops[0].get().unit.kill(&registry);
    assert!(army.troops[0].get().is_dead());
    {
        let troop = &mut army.troops[0];
        let mut troop = troop.get();
        let cost = crate::map::object::resurrect_cost(&troop, &registry);
        assert_eq!(cost, 50); // cost_hire юнита 0
        let spent = crate::map::object::resurrect(&mut troop, army.stats.gold, &registry).unwrap();
        army.stats.gold -= spent;
        assert_eq!(troop.unit.hp, 100, "воскрешение даёт полное здоровье");
    }
    assert_eq!(army.stats.gold, 150);
    // Живого воскресить нельзя.
    {
        let troop = &mut army.troops[0];
        let mut troop = troop.get();
        assert!(crate::map::object::resurrect(&mut troop, army.stats.gold, &registry).is_err());
    }
    // Не хватает золота.
    army.stats.gold = 10;
    army.troops[0].get().unit.kill(&registry);
    {
        let troop = &mut army.troops[0];
        let mut troop = troop.get();
        assert!(crate::map::object::resurrect(&mut troop, army.stats.gold, &registry).is_err());
        assert!(troop.unit.is_dead(), "юнит не воскрешён при нехватке денег");
    }
}
#[test]
fn spell_learn_records_book_without_applying_effect() {
    let registry = make_building_registry();
    let mut army = make_army_from_ids(&registry, &[(20, 7)]);
    let hero_effects_before = army.troops[0].get().unit.effects.len();
    // Изучение: id попадает в книгу армии.
    assert_eq!(army.learn_spell(3), Ok(true));
    assert!(army.spells.contains(&3));
    // Эффект на юнита-героя НЕ накладывается.
    assert_eq!(
        army.troops[0].get().unit.effects.len(),
        hero_effects_before,
        "изучение не даёт эффектов — только запись в книгу"
    );
    // Повторное изучение того же заклинания отклонено.
    assert_eq!(army.learn_spell(3), Ok(false));
    assert_eq!(army.spells.iter().filter(|s| **s == 3).count(), 1);
}
