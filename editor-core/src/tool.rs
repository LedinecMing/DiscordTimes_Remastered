//! Инструменты редактора (шелл-агностичные, ТЗ §3).
//!
//! Здесь только модель инструментов: имена для UI/статус-бара и порядок
//! перечисления для палитры. Обе оболочки (egui-канвас wgpu_ui, будущие)
//! читают один и тот же список — привязки клавиш живут в шелле.

/// Активный инструмент (палитра ТЗ §3 Map View).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Tool {
    /// Кисть тайлов.
    #[default]
    Brush,
    /// Постановка декора.
    Deco,
    /// Постановка строения.
    Building,
    /// Постановка армии.
    Army,
    /// Выбор/перенос объектов (инфоокно, drag/click-click перенос).
    /// ЛКМ НЕ рисует: выбирает армию/строение/точку событий.
    Interact,
}

impl Tool {
    /// Человекочитаемое имя (статус-бар, панель инструментов).
    pub fn label(self) -> &'static str {
        match self {
            Tool::Brush => "Кисть",
            Tool::Deco => "Декор",
            Tool::Building => "Строение",
            Tool::Army => "Армия",
            Tool::Interact => "Интеракт",
        }
    }

    /// Все инструменты в порядке палитры.
    pub const ALL: [Tool; 5] = [
        Tool::Brush,
        Tool::Deco,
        Tool::Building,
        Tool::Army,
        Tool::Interact,
    ];
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_tools_have_unique_labels() {
        let labels: Vec<_> = Tool::ALL.iter().map(|t| t.label()).collect();
        assert_eq!(labels.len(), 5);
        for (i, a) in labels.iter().enumerate() {
            for b in labels.iter().skip(i + 1) {
                assert_ne!(a, b);
            }
        }
    }
}
