//! editor-validators: валидаторы карты по известным багам игры.
//!
//! Каждый валидатор — отдельная проверка из ТЗ §5 (источник — раздел 9 гайда
//! Hristos). Запуск: по кнопке Lint, перед сохранением, фоном (ТЗ §5).
//! Severity: Error блокирует сохранение, Warning — сохранение возможно, Info.

pub mod issue;
pub mod patrol_bounds;
pub mod radius_range;

pub use issue::{Issue, IssueLocation, Severity};
pub use patrol_bounds::PatrolBounds;
pub use radius_range::RadiusRangeCheck;

/// Каркас валидатора: проверка проекта, выдающая список проблем.
pub trait Validator {
    /// Уникальное имя валидатора (для панели, автофиксов, будущих ignore-настроек).
    fn name(&self) -> &'static str;
    /// Ссылка на описание бага в гайде (раздел 9).
    fn guide_reference(&self) -> &'static str;
    /// Проверить проект.
    fn validate(&self, project: &editor_core::MapProject) -> Vec<Issue>;
}

/// Запустить набор валидаторов, слить результаты в один список.
pub fn run_all(project: &editor_core::MapProject) -> Vec<Issue> {
    let validators: [Box<dyn Validator>; 2] =
        [Box::new(PatrolBounds), Box::new(RadiusRangeCheck)];
    let mut issues = Vec::new();
    for validator in validators {
        issues.extend(validator.validate(project));
    }
    issues
}
