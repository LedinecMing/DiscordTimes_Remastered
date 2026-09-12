//! RadiusRangeCheck (гайд 5.13/9.9, ТЗ §5: «радиус фонарика от 0 до 24»).
//!
//! Фонарик (light) с радиусом вне 0..=24 вызывает вылеты игры.
//! В dt_lib фонарик — `convert::LightOrEvent` (x, y, light_radius) в бинарной
//! .DTm-карте; в проекте редактора фонарики — [`editor_core::project::Light`].

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
        for light in &project.lights {
            if light.radius > LIGHT_RADIUS_MAX {
                issues.push(Issue::error(
                    self.name(),
                    self.guide_reference(),
                    format!(
                        "Радиус фонарика ({}, {}) равен {} — максимум {LIGHT_RADIUS_MAX}; вылет при выходе за диапазон",
                        light.x, light.y, light.radius
                    ),
                    IssueLocation::Map(light.x, light.y),
                ));
            }
        }
        issues
    }
}



#[cfg(test)]
mod tests {
    use super::*;
    use editor_core::project::Light;

    fn project_with_lights(lights: Vec<Light>) -> MapProject {
        let mut project = MapProject::new(50, 0);
        project.lights = lights;
        project
    }

    #[test]
    fn positive_radius_in_range_passes() {
        let project = project_with_lights(vec![
            Light { x: 1, y: 2, radius: 0 },
            Light { x: 3, y: 4, radius: 24 },
        ]);
        assert!(RadiusRangeCheck.validate(&project).is_empty());
    }

    #[test]
    fn negative_radius_over_24_fails() {
        let project = project_with_lights(vec![Light { x: 5, y: 6, radius: 25 }]);
        let issues = RadiusRangeCheck.validate(&project);
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].severity, crate::Severity::Error);
        assert!(issues[0].message.contains("25"));
        assert_eq!(issues[0].location, IssueLocation::Map(5, 6));
    }

    #[test]
    fn no_lights_passes() {
        let project = MapProject::new(10, 0);
        assert!(RadiusRangeCheck.validate(&project).is_empty());
    }
}
