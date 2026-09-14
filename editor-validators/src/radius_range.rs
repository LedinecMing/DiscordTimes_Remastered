//! RadiusRangeCheck (гайд 5.13/9.9, ТЗ §5: «радиус фонарика от 0 до 24»).
//!
//! Фонарик (light) с радиусом вне 0..=24 вызывает вылеты игры.
//! В dt_lib фонарик — `convert::LightOrEvent` (x, y, light_radius) в бинарной
//! .DTm-карте; в проекте редактора точки событий/фонарики — единая
//! структура `MapLantern` (MapProject.lanterns).

use crate::issue::{Issue, IssueLocation};
use crate::Validator;
use editor_core::MapProject;

/// Максимум радиуса фонарика: вылеты при выходе за диапазон (гайд 9.9).
pub const LIGHT_RADIUS_MAX: u64 = 24;

pub struct RadiusRangeCheck;

impl Validator for RadiusRangeCheck {
    fn name(&self) -> &'static str {
        "RadiusRangeCheck"
    }

    fn guide_reference(&self) -> &'static str {
        "guide 9.9"
    }

    fn validate(&self, project: &MapProject) -> Vec<Issue> {
        let mut issues = Vec::new();
        for lantern in &project.lanterns {
            if lantern.light_radius as u64 > LIGHT_RADIUS_MAX {
                issues.push(Issue::error(
                    self.name(),
                    self.guide_reference(),
                    format!(
                        "Радиус фонарика ({}, {}) равен {} — максимум {LIGHT_RADIUS_MAX}; вылет при выходе за диапазон",
                        lantern.x, lantern.y, lantern.light_radius
                    ),
                    IssueLocation::Map(lantern.x, lantern.y),
                ));
            }
        }
        issues
    }
}



#[cfg(test)]
mod tests {
    use super::*;
    use dt_lib::map::map::MapLantern;

    fn project_with_lanterns(lanterns: Vec<MapLantern>) -> MapProject {
        let mut project = MapProject::new(50, 0);
        project.lanterns = lanterns;
        project
    }

    #[test]
    fn positive_radius_in_range_passes() {
        let project = project_with_lanterns(vec![
            MapLantern { x: 1, y: 2, light_radius: 0, ..Default::default() },
            MapLantern { x: 3, y: 4, light_radius: 24, ..Default::default() },
        ]);
        assert!(RadiusRangeCheck.validate(&project).is_empty());
    }

    #[test]
    fn negative_radius_over_24_fails() {
        let project = project_with_lanterns(vec![MapLantern {
            x: 5,
            y: 6,
            light_radius: 25,
            ..Default::default()
        }]);
        let issues = RadiusRangeCheck.validate(&project);
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].severity, crate::Severity::Error);
        assert!(issues[0].message.contains("25"));
        assert_eq!(issues[0].location, IssueLocation::Map(5, 6));
    }

    #[test]
    fn no_lanterns_passes() {
        let project = MapProject::new(10, 0);
        assert!(RadiusRangeCheck.validate(&project).is_empty());
    }
}
