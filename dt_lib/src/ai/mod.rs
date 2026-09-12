//! ИИ-агенты: Battle AI (этап 1-3 плана notes/AI_AGENTS_PLAN.md) и каркас
//! Map AI (структуры только; логика — этапы 4-6). Правило зависимостей:
//! ИИ НЕ мутирует состояние сам — только выдаёт Decision, исполнение через
//! существующие handle_action (бой) / message_handler(GoTo) (карта).

pub mod battle;
pub mod eval;
pub mod policy;
#[cfg(test)]
mod tests;

pub use battle::{AiDecision, BattleAi};
pub use policy::{AiSettings, BattlePolicy, MapPolicy, TargetPriority};

/// Уровень качества ИИ (Опция10 «Улучшенный интеллект» → Enhanced).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum AiQuality {
    #[default]
    Greedy,
    Enhanced,
}

impl AiQuality {
    /// Политика по уровню качества: Enhanced включает осторожность
    /// (hp_threshold_retreat) — весовой 2-ply lookahead остаётся этапом 5.
    pub fn battle_policy(self) -> BattlePolicy {
        match self {
            AiQuality::Greedy => BattlePolicy::default(),
            AiQuality::Enhanced => BattlePolicy {
                hp_threshold_retreat: 0.35,
                threat_weight: 0.5,
                lookahead: 2,
                ..BattlePolicy::default()
            },
        }
    }
}
