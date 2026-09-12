//! Тип проблемы, найденной валидатором.

use serde::{Deserialize, Serialize};

/// Тяжесть проблемы (ТЗ §5: error блокирует сохранение).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Severity {
    Info,
    Warning,
    Error,
}

/// Где именно проблема.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum IssueLocation {
    /// Позиция на карте (x, y).
    Map(usize, usize),
    /// Индекс объекта в своём списке (армия/строение/событие).
    Object { kind: &'static str, index: usize },
    /// Без конкретной позиции.
    Global,
}

/// Проблема, найденная валидатором.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Issue {
    pub severity: Severity,
    pub message: String,
    pub location: IssueLocation,
    /// Раздел гайда с описанием бага.
    pub guide_reference: &'static str,
    /// Имя валидатора, нашедшего проблему.
    pub validator: &'static str,
}

impl Issue {
    pub fn error(
        validator: &'static str,
        guide_reference: &'static str,
        message: impl Into<String>,
        location: IssueLocation,
    ) -> Self {
        Self {
            severity: Severity::Error,
            message: message.into(),
            location,
            guide_reference,
            validator,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn severity_ordering_puts_error_last() {
        assert!(Severity::Info < Severity::Warning);
        assert!(Severity::Warning < Severity::Error);
    }
}
