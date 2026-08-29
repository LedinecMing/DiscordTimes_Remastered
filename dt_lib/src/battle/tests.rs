use crate::{
    battle::{
        Army, ArmyStats, BattleInfo, BattleUnit, BattleUnitInfo,
        Field, TroopType,
        battlefield::{field_type, possible_movement, unit_interaction, check_win},
        troop::Troop, control::Control,
    },
    registry::GameInfo,
    units::{
        unit::{AttackSettings, LevelUpInfo, MagicDirection, MagicType, Power, Unit, UnitInfo,
               UnitInventory, UnitLvl, UnitPos, UnitStats, UnitType, ActionResult, Defence,
               attack_indexed},
        unitstats::{ModifyUnitStats, Modify, ModifyPower},
    },
    items::item::{Item, ItemInfo, ArtifactType, WeaponType, MagicVariants},
    bonuses::BonusInfo,
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
            cost: 0, cost_hire: 0, icon_index: i,
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
        next_unit: vec![], magic_type: Some(MagicType::Death),
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
        next_unit: vec![], magic_type: Some(MagicType::Life),
        magic_direction: MagicDirection::ToAll,
        surrender: None, bonus: None, settings: AttackSettings::default(),
        stats: UnitStats {
            hp: 80, max_hp: 80, moves: 2, max_moves: 2, speed: 4,
            damage: Power::new(30, 0, 0), ..Default::default()
        },
        lvl: LevelUpInfo::default(),
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
    attack_indexed(attacker, target, armies, &battle_info, registry)
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
    let mage = &armies[0].troops[0].get();
    let target = &armies[1].troops[0].get();
    let can_target = mage.unit.can_attack(
        &target.unit, target.pos, mage.pos, true,
        &armies[0].hitmap, &registry, 6, 12,
    );
    assert!(can_target, "ToAll mage should be able to target enemy");
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