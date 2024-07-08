use std::fmt::Debug;

use notan::{
    draw::Draw,
    prelude::{App, Assets, Graphics, Plugins},
};
use notan_ui::{
    defs::{DrawFunction, ObjPosForm, Positionable, UIStateCl},
    form::Form,
    rect::{Position, Rect, Size},
};

#[derive(Clone)]
pub struct TextureRenderer<State: UIStateCl> {
    pub texture_id: (String, String),
    pub to_draw: DrawFunction<State, Self>,
    pub rect: Rect,
}
impl<State: UIStateCl> Debug for TextureRenderer<State> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TextureRenderer")
            .field("texture_id", &self.texture_id)
            .field("rect", &self.rect)
            .finish()
    }
}
impl<State: UIStateCl> Form<State> for TextureRenderer<State> {
    fn draw(
        &mut self,
        app: &mut App,
        assets: &mut Assets,
        gfx: &mut Graphics,
        plugins: &mut Plugins,
        state: &mut State,
        draw: &mut Draw,
    ) {
        (self.to_draw)(self, app, assets, gfx, plugins, state, draw);
    }
    fn after(&mut self, _: &mut App, _: &mut Assets, _: &mut Plugins, _: &mut State) {}
}
impl<State: UIStateCl> Positionable for TextureRenderer<State> {
    fn add_pos(&mut self, to_add: Position) {
        self.rect.pos += to_add;
    }
    fn with_pos(&self, to_add: Position) -> Self {
        let mut new = self.clone();
        new.rect.pos += to_add;
        new
    }
    fn get_pos(&self) -> Position {
        self.rect.pos
    }
    fn get_size(&self) -> Size {
        self.rect.size
    }
    fn get_rect(&self) -> Rect {
        self.rect
    }
}

#[derive(Clone)]
pub struct SubWindowSys<State: UIStateCl> {
    pub windows: Vec<Box<dyn ObjPosForm<State>>>,
    pub select_window: for<'a> fn(&mut Self, &'a State) -> usize,
    pub rect: Rect,
}
impl<State: UIStateCl> Debug for SubWindowSys<State> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SubWindowSys")
            .field("windows", &self.windows)
            .finish()
    }
}
impl<State: UIStateCl> Form<State> for SubWindowSys<State> {
    fn after(
        &mut self,
        app: &mut App,
        assets: &mut Assets,
        plugins: &mut Plugins,
        state: &mut State,
    ) {
        let window = (self.select_window)(self, state);
        let pos = self.rect.pos;
        let window = &mut self.windows[window];
        window.add_pos_obj(pos);
        window.after(app, assets, plugins, state);
        window.add_pos_obj(-pos);
    }
    fn draw(
        &mut self,
        app: &mut App,
        assets: &mut Assets,
        gfx: &mut Graphics,
        plugins: &mut Plugins,
        state: &mut State,
        draw: &mut Draw,
    ) {
        let window = (self.select_window)(self, state);
        let pos = self.rect.pos;
        let window = &mut self.windows[window];
        window.add_pos_obj(pos);
        window.draw(app, assets, gfx, plugins, state, draw);
        window.add_pos_obj(-pos);
    }
}
impl<State: UIStateCl> Positionable for SubWindowSys<State> {
    fn add_pos(&mut self, to_add: Position) {
        self.rect.pos += to_add;
    }
    fn with_pos(&self, to_add: Position) -> Self {
        let mut new = self.clone();
        new.rect.pos += to_add;
        new
    }
    fn get_pos(&self) -> Position {
        self.rect.pos
    }
    fn get_rect(&self) -> Rect {
        self.rect
    }
    fn get_size(&self) -> Size {
        self.rect.size
    }
}
