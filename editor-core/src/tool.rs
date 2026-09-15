//! Инструменты редактора (шелл-агностичные, ТЗ §3).
//!
//! Здесь только модель инструментов: имена для UI/статус-бара и порядок
//! перечисления для палитры. Обе оболочки (egui-канвас wgpu_ui, будущие)
//! читают один и тот же список — привязки клавиш живут в шелле.

/// Активный инструмент (палитра ТЗ §3 Map View).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Tool {
    /// Кисть тайлов (с фигурой/размером — BrushConfig в UI-состоянии).
    #[default]
    Brush,
    /// Заливка связной области (flood по совпадению тайла; настройки —
    /// BrushConfig: что менять/дальность/макс. объём).
    BucketFill,
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
            Tool::BucketFill => "Заливка",
            Tool::Deco => "Декор",
            Tool::Building => "Строение",
            Tool::Army => "Армия",
            Tool::Interact => "Интеракт",
        }
    }

    /// Все инструменты в порядке палитры.
    pub const ALL: [Tool; 6] = [
        Tool::Brush,
        Tool::BucketFill,
        Tool::Deco,
        Tool::Building,
        Tool::Army,
        Tool::Interact,
    ];

    /// Режим «Рисование» (сегмент тулбара): кисть/заливка/декор/строения/
    /// армии. Интеракт — отдельный режим.
    pub fn is_paint(self) -> bool {
        !matches!(self, Tool::Interact)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_tools_have_unique_labels() {
        let labels: Vec<_> = Tool::ALL.iter().map(|t| t.label()).collect();
        assert_eq!(labels.len(), 6);
        for (i, a) in labels.iter().enumerate() {
            for b in labels.iter().skip(i + 1) {
                assert_ne!(a, b);
            }
        }
    }
}
