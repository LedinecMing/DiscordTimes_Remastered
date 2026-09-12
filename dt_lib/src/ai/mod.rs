//! ИИ-агенты: Battle AI (этапы 1-3 плана notes/AI_AGENTS_PLAN.md) и каркас
//! Map AI (структуры только; логика — этапы 4-6). Правило зависимостей:
//! ИИ НЕ мутирует состояние сам — только выдаёт Decision, исполнение через
//! существующие handle_action / unit_interaction (бой), message_handler(GoTo)
//! (карта, позже).

pub mod battle;
pub mod eval;
#[cfg(test)]
mod smoke;
pub mod policy;
#[cfg(test)]
mod tests;

pub use battle::{AiDecision, BattleAi};
pub use policy::{AiSettings, BattlePolicy, MapPolicy, TargetPriority};
