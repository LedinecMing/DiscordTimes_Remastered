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



    /// Видимые UI-настройки меняются тоже через команды — но чтение свободное.
    pub(crate) fn project_mut(&mut self) -> &mut MapProject {
        &mut self.project
    }

    /// Курсор канваса — UI-настройка, не изменение документа.
    pub fn set_cursor(&mut self, cursor: Pos) {
        self.cursor = cursor;
    }



}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::project::MapProject;

    #[test]
    fn state_is_read_only_from_outside() {
        let state = EditorState::new(MapProject::new(4, 0));
        assert_eq!(state.project().tile((1, 1)), Some(0));
        // Единственный способ изменения — команды; прямых сеттеров нет.
        assert_eq!(state.active_tile(), 0);
    }
}
