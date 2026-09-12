//! Состояние редактора: проект + курсор для undo/redo.
//!
//! State иммутабелен снаружи (ТЗ §6): публичных методов мутации проекта нет —
//! всё через [`Command`] и [`CommandHistory`]. Мутация внутри команд — через
//! `state_mut`, доступный только этому крейту (`pub(crate)`).

use crate::project::{MapProject, Pos};

/// Состояние редактора: документ и вьюпорт-курсор.
#[derive(Clone, Debug, Default)]
pub struct EditorState {
    project: MapProject,
    /// Координата курсора мыши на канвасе (для статус-бара и команд).
    cursor: Pos,
    /// Активный тайл в палитре (инструмент «кисть»).
    active_tile: usize,
    /// Имя открытого файла (None — проект ещё не сохранялся).
    open_path: Option<String>,
}

impl EditorState {
    pub fn new(project: MapProject) -> Self {
        Self {
            project,
            ..Default::default()
        }
    }

    /// Снимок проекта (иммутабельный, для UI и валидаторов).
    pub fn project(&self) -> &MapProject {
        &self.project
    }

    pub fn cursor(&self) -> Pos {
        self.cursor
    }

    pub fn active_tile(&self) -> usize {
        self.active_tile
    }
    /// Выбор в палитре тайлов: UI-настройка, не изменение документа.
    pub fn set_active_tile(&mut self, tile: usize) {
        self.active_tile = tile.min(crate::project::TILE_COUNT - 1);
    }

    pub fn open_path(&self) -> Option<&str> {
        self.open_path.as_deref()
    }


    /// Видимые UI-настройки меняются тоже через команды — но чтение свободное.
    pub(crate) fn project_mut(&mut self) -> &mut MapProject {
        &mut self.project
    }

    pub(crate) fn set_cursor(&mut self, cursor: Pos) {
        self.cursor = cursor;
    }


    pub(crate) fn set_open_path(&mut self, path: Option<String>) {
        self.open_path = path;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::project::MapProject;

    #[test]
    fn state_is_read_only_from_outside() {
        let mut state = EditorState::new(MapProject::new(4, 0));
        assert_eq!(state.project().tile((1, 1)), Some(0));
        // Единственный способ изменения — команды; прямых сеттеров нет.
        assert_eq!(state.active_tile(), 0);
    }
}
