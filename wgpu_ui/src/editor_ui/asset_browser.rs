//! Переиспользуемый браузер ассетов: грид с поиском, фильтрами категорий
//! и мульти-выбором. Любая палитра редактора (тайлы, декорации, строения,
//! армии, точки событий) реализует [`AssetSource`] и получает виджет
//! целиком — без копипасты грида.

use egui::{Color32, RichText, Sense, Stroke, Ui};

/// Один элемент браузера: id (возвращается наружу), подпись, категория.
pub struct AssetItem<Id> {
    pub id: Id,
    /// Название (по нему ищет поиск, отображается под иконкой).
    pub name: String,
    /// Категория для фильтра (первое слово имени, «Деревья», …).
    pub category: String,
    /// Идентификатор в текстурном кэше (ключ `EditorUi.palette_tex`).
    pub tex_key: String,
    /// Числовой подписи под именем (id объекта, номер тайла).
    pub sub: Option<String>,
}

/// Источник данных для браузера конкретного вида ассетов.
pub trait AssetSource {
    type Id: Copy + PartialEq;
    /// Полный список элементов текущего проекта/реестра.
    fn items(&self) -> Vec<AssetItem<Self::Id>>;
    /// Текстура иконки (уже предзагруженная); None = ячейка без иконки.
    fn icon<'a>(&self, key: &str, tex: &'a std::collections::HashMap<String, egui::TextureHandle>) -> Option<&'a egui::TextureHandle>;
    /// Мульти-выбор включён для этого вида ассетов (палитры кисти).
    fn multi_select(&self) -> bool {
        false
    }
    /// Текущий одиночный выбор (подсветка ячейки).
    fn selected(&self, id: &Self::Id) -> bool;
    /// Нажата ячейка (одиночный выбор/постановка).
    fn pick(&mut self, id: Self::Id);
    /// Ячейка добавлена/убрана в мульти-выбор.
    fn toggle_multi(&mut self, id: Self::Id) {}
    /// В мульти-выборе сейчас (галочка).
    fn in_multi(&self, _id: &Self::Id) -> bool {
        false
    }
}

/// События кликов браузера за кадр.
pub enum BrowserEvent<Id> {
    /// Клик по ячейке (pick).
    Picked(Id),
    /// Двойной клик — подписчики открывают инфо-тайл (goto-definition).
    Opened(Id),
}

const CELL: egui::Vec2 = egui::vec2(72., 80.);

pub fn asset_browser<S: AssetSource>(
    ui: &mut Ui,
    source: &mut S,
    tex: &std::collections::HashMap<String, egui::TextureHandle>,
    search: &mut String,
    category: &mut Option<String>,
    categories: &[String],
    id_salt: &str,
) -> Vec<BrowserEvent<S::Id>> {
    let mut events = Vec::new();
    // Строка поиска + фильтр категории.
    ui.horizontal(|ui| {
        ui.add(
            egui::TextEdit::singleline(search)
                .hint_text("Поиск")
                .desired_width(120.),
        );
        egui::ComboBox::from_id_salt((id_salt, "cat"))
            .selected_text(category.as_deref().unwrap_or("Все категории"))
            .show_ui(ui, |ui| {
                if ui.selectable_label(category.is_none(), "Все категории").clicked() {
                    *category = None;
                }
                for cat in categories {
                    if ui.selectable_label(category.as_deref() == Some(cat.as_str()), cat).clicked() {
                        *category = Some(cat.clone());
                    }
                }
            });
    });
    ui.separator();
    egui::ScrollArea::vertical()
        .id_salt(id_salt)
        .auto_shrink([false, false])
        .show(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                for item in source.items() {
                    let search_hit = search.is_empty()
                        || item.name.to_lowercase().contains(&search.to_lowercase());
                    let cat_hit = category.as_deref().map_or(true, |c| item.category == c);
                    if !search_hit || !cat_hit {
                        continue;
                    }
                    let selected = source.selected(&item.id);
                    let (cell, icon_rect) = cell(ui, CELL);
                    if let Some(icon) = source.icon(&item.tex_key, tex) {
                        draw_icon_fit(ui.painter(), icon, icon_rect);
                    }
                    let mut label = item.name.clone();
                    if let Some(sub) = &item.sub {
                        label.push('\n');
                        label.push_str(sub);
                    }
                    ui.put(
                        egui::Rect::from_min_size(
                            cell.rect.left_bottom() - egui::vec2(0., 30.),
                            egui::vec2(CELL.x, 30.),
                        ),
                        egui::Label::new(
                            RichText::new(label)
                                .color(if selected { Color32::YELLOW } else { Color32::LIGHT_GRAY })
                                .small(),
                        ),
                    );
                    let mut cell = cell.on_hover_text(format!(
                        "{} ({})",
                        item.name,
                        item.sub.as_deref().unwrap_or("")
                    ));
                    if source.multi_select() {
                        let checked = source.in_multi(&item.id);
                        // Галочка мульти-выбора в углу ячейки.
                        let cb = ui.put(
                            egui::Rect::from_min_size(
                                cell.rect.right_top() - egui::vec2(18., 0.),
                                egui::vec2(18., 18.),
                            ),
                            egui::Checkbox::new(&mut checked.clone(), ""),
                        );
                        if cb.clicked() {
                            source.toggle_multi(item.id);
                        }
                    }
                    if cell.clicked() {
                        source.pick(item.id);
                        events.push(BrowserEvent::Picked(item.id));
                    }
                    if cell.double_clicked() {
                        events.push(BrowserEvent::Opened(item.id));
                    }
                    if selected {
                        ui.painter().rect_stroke(
                            cell.rect,
                            3.,
                            Stroke::new(2., Color32::YELLOW),
                            egui::StrokeKind::Inside,
                        );
                    }
                }
            });
        });
    events
}

fn cell(ui: &mut Ui, size: egui::Vec2) -> (egui::Response, egui::Rect) {
    let (rect, resp) = ui.allocate_exact_size(size, Sense::click());
    let icon_rect = egui::Rect::from_min_size(
        rect.left_top() + egui::vec2(4., 4.),
        egui::vec2(size.x - 8., size.y - 34.),
    );
    (resp, icon_rect)
}

/// Aspect-fit текстуры в прямоугольнике (центрирование).
pub fn draw_icon_fit(painter: &egui::Painter, tex: &egui::TextureHandle, rect: egui::Rect) {
    let [tw, th] = tex.size();
    let (tw, th) = (tw as f32, th as f32);
    if tw <= 0. || th <= 0. {
        return;
    }
    let scale = (rect.width() / tw).min(rect.height() / th);
    let (w, h) = (tw * scale, th * scale);
    let min = rect.center() - egui::vec2(w * 0.5, h * 0.5);
    painter.image(
        tex.id(),
        egui::Rect::from_min_size(min, egui::vec2(w, h)),
        egui::Rect::from_min_max(egui::pos2(0., 0.), egui::pos2(1., 1.)),
        Color32::WHITE,
    );
}
