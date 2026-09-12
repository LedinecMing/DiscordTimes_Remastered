//! PatrolBounds (гайд 9.6/9.12, ТЗ §5/§7).
//!
//! Баги: армия со свободным передвижением на большой открытой карте —
//! нехватка памяти для маршрута; патруль за нижней границей карты — вылет
//! при построении маршрута.
//!
//! Проверки: позиция армии внутри карты; радиус патруля таков, что окружность
//! патрулирования не выходит за границы карты (иначе игра строит маршрут
//! за нижнюю границу).

use crate::issue::{Issue, IssueLocation};
use crate::Validator;
use editor_core::MapProject;

pub struct PatrolBounds;

impl Validator for PatrolBounds {
    fn name(&self) -> &'static str {
        "PatrolBounds"
    }

    fn guide_reference(&self) -> &'static str {
        "guide 9.6/9.12"
    }

    fn validate(&self, project: &MapProject) -> Vec<Issue> {
        let mut issues = Vec::new();
        let size = project.size();
        if size == 0 {
            return issues;
        }
        for (index, army) in project.map.armys.iter().enumerate() {
            let (x, y) = army.pos;
            if x >= size || y >= size {
                issues.push(Issue::error(
                    self.name(),
                    self.guide_reference(),
                    format!(
                        "Армия «{}» стоит за границей карты: ({x}, {y}) вне 0..{size} x 0..{size}",
                        army.stats.army_name
                    ),
                    IssueLocation::Object { kind: "army", index },
                ));
                continue;
            }
            // Радиус патруля: до нижней/правой границы не должно быть выхода.
            if let Some(pc) = &army.pc_settings {
                if let Some(radius) = pc.patrol_radius {
                    if y + radius as usize >= size {
                        issues.push(Issue::error(
                            self.name(),
                            self.guide_reference(),
                            format!(
                                "Патруль армии «{}» (радиус {radius}) выходит за нижнюю границу карты: y={y}, размер={size}",
                                army.stats.army_name
                            ),
                            IssueLocation::Object { kind: "army", index },
                        ));
                    }
                    if x + radius as usize >= size {
                        issues.push(Issue::error(
                            self.name(),
                            self.guide_reference(),
                            format!(
                                "Патруль армии «{}» (радиус {radius}) выходит за правую границу карты: x={x}, размер={size}",
                                army.stats.army_name
                            ),
                            IssueLocation::Object { kind: "army", index },
                        ));
                    }
                }
            }
        }
        issues
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dt_lib::battle::army::Army;
    use dt_lib::battle::control::PC_ControlSetings;

    fn project_with_army(army: Army) -> MapProject {
        let mut project = MapProject::new(50, 0);
        project.map.armys.push(army);
        project
    }

    fn army_at(x: usize, y: usize, patrol_radius: Option<u64>) -> Army {
        Army {
            pos: (x, y),
            pc_settings: Some(PC_ControlSetings {
                patrol_radius,
                ..Default::default()
            }),
            stats: dt_lib::battle::army::ArmyStats {
                army_name: "Test".into(),
                ..Default::default()
            },
            ..Default::default()
        }
    }

    #[test]
    fn positive_army_inside_bounds_passes() {
        let project = project_with_army(army_at(10, 10, Some(5)));
        assert!(PatrolBounds.validate(&project).is_empty());
    }

    #[test]
    fn positive_no_patrol_passes() {
        let project = project_with_army(army_at(49, 49, None));
        assert!(PatrolBounds.validate(&project).is_empty());
    }

    #[test]
    fn negative_patrol_crosses_bottom_border() {
        // y=48, радиус 5: 48+5=53 >= 50 — вылет за нижнюю границу.
        let project = project_with_army(army_at(10, 48, Some(5)));
        let issues = PatrolBounds.validate(&project);
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].severity, crate::Severity::Error);
        assert!(issues[0].message.contains("нижнюю границу"));
    }

    #[test]
    fn negative_patrol_crosses_right_border() {
        let project = project_with_army(army_at(47, 3, Some(4)));
        let issues = PatrolBounds.validate(&project);
        assert_eq!(issues.len(), 1);
        assert!(issues[0].message.contains("правую границу"));
    }

    #[test]
    fn negative_army_outside_map() {
        let project = project_with_army(army_at(60, 3, None));
        let issues = PatrolBounds.validate(&project);
        assert_eq!(issues.len(), 1);
        assert!(issues[0].message.contains("за границей карты"));
    }

    #[test]
    fn negative_both_borders_reported() {
        let project = project_with_army(army_at(48, 48, Some(3)));
        let issues = PatrolBounds.validate(&project);
        assert_eq!(issues.len(), 2);
    }
}
