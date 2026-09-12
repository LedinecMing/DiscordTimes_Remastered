//! editor-core: домен редактора карт без UI.
//!
//! Принципы (ТЗ §6):
//! - Разделение модели и представления: здесь нет ни egui, ни рендера.
//! - Иммутабельный снаружи state: изменения только через [`Command`].
//! - Поток изменений через команды: [`CommandHistory`] даёт undo/redo.
pub mod command;
pub mod project;
pub mod render;
pub mod state;
pub use command::{Command, CommandHistory, CommandResult};
pub use project::MapProject;
pub use state::EditorState;
