//! Смоук полного боя под ИИ: обе армии ходят BattleAi::decide+execute,
//! бой обязан завершиться (без зависаний/пинг-понга) — стабильность
//! исполнения через существующий handle_action.

use crate::{ai::policy::BattlePolicy, battle::BattleInfo};

#[test]
fn ai_vs_ai_battle_terminates() {
    let registry = crate::battle::tests::make_test_registry();
    let army1 = crate::battle::tests::make_army_from_ids(&registry, &[(20, 7), (22, 2)]);
    let army2 = crate::battle::tests::make_army_from_ids(&registry, &[(0, 8), (0, 9)]);
    let mut armies = vec![army1, army2];
    let mut battle = BattleInfo::new(&mut armies, 0, 1);
    battle.start(&mut armies, &registry);
    let policy = BattlePolicy::default();
    let mut steps = 0usize;
    while battle.winner.is_none() && steps < 500 {
        let Some(active) = battle.active_unit else {
            break;
        };
        let decision = crate::ai::BattleAi::decide(&battle, &armies, active.army, &policy, &registry);
        // ИИ не мутирует сам — только через handle_action-обёртку execute.
        crate::ai::BattleAi::execute(decision, &mut battle, &mut armies, &registry);
        steps += 1;
    }
    // Победитель обязателен: MAX_MOVES/капитуляции не дают бесконечного боя.
    assert!(
        battle.winner.is_some(),
        "AI-vs-AI battle must terminate, steps={steps}, move_count={}",
        battle.move_count
    );
}
