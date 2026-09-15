//! Селекторы ссылок на игровые объекты со встроенным goto-definition:
//! кнопка «→» открывает (или фокусирует) инфо-тайл объекта в док-панели.
//! Работает для событий, армий, строений, юнитов — любых ресурсов,
//! у которых есть инфо-панель.

use egui::Ui;

/// Единый ключ игрового объекта: открывает инфо-тайл, участвует в
/// селекторах и палитрах.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum GameRef {
    Event(usize),
    Army(usize),
    Building(usize),
    Unit(usize),
    Lantern(usize),
}

/// Имя объекта по ссылке (для подписи селектора и заголовка тайла).
pub trait RefName {
    fn ref_name(&self, r: GameRef) -> Option<String>;
}

/// Открыть/поднять инфо-тайл объекта. Реализация — в editor_view
/// (добавляет Selection в egui_tiles-дерево; id уже есть → фокус).
pub trait PropertiesDock {
    fn open(&mut self, r: GameRef);
}

/// Селектор ссылки: `[имя (id)] [v] [→]`.
/// `options` — (id, имя) существующих объектов данного вида;
/// `current` — текущая ссылка (None = не выбрано);
/// клик по «→» вызывает `dock.open(GameRef::…)` с конструктором вида.
pub fn ref_selector(
    ui: &mut Ui,
    current: &mut Option<usize>,
    options: &[(usize, String)],
    dock: &mut dyn PropertiesDock,
    to_ref: fn(usize) -> GameRef,
) -> egui::Response {
    let selected_name = current
        .and_then(|id| options.iter().find(|(oid, _)| *oid == id))
        .map(|(_, n)| n.clone())
        .unwrap_or_else(|| "—".to_string());
    ui.horizontal(|ui| {
        egui::ComboBox::from_id_salt(("refsel", options.len(), *current))
            .selected_text(selected_name)
            .show_ui(ui, |ui| {
                if ui.selectable_label(current.is_none(), "—").clicked() {
                    *current = None;
                }
                for (id, name) in options {
                    if ui.selectable_label(*current == Some(*id), name).clicked() {
                        *current = Some(*id);
                    }
                }
            });
        // goto-definition: открыть инфо-тайл выбранного объекта.
        if let Some(id) = *current {
            if ui.small_button("→").clicked() {
                dock.open(to_ref(id));
            }
        }
    })
    .response
}
