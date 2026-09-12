//! Точка входа редактора карт (bin/editor, ТЗ §8).

use eframe::egui;

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1280.0, 800.0])
            .with_title("DT Map Editor"),
        ..Default::default()
    };
    eframe::run_native(
        "DT Map Editor",
        options,
        Box::new(|_cc| Ok(Box::new(editor_ui::MapEditorApp::new()))),
    )
}
