use {
    std::{
        ops::Not,
        marker::PhantomData,
        collections::HashMap,
    },
    notan::{
        prelude::{AppState, Color, Graphics, Plugins, App},
        app::{Event, Texture},
        draw::*
    },
    dyn_clone::{DynClone, clone_box},
    super::{
        form::Form,
        rect::*,
        defs::*
}   };

#[derive(Clone)]
pub struct Data<State: UIStateCl, K: Clone, V: Clone> {
    pub data: HashMap<K, V>,
    pub boo: PhantomData<State>
}
impl<State: UIStateCl, K: Clone, V: Clone> Form<State> for Data<State, K, V> {
    fn draw(&mut self, app: &mut App, gfx: &mut Graphics, plugins: &mut Plugins, state: &mut State) {}
    fn after(&mut self, app: &mut App, gfx: &mut Graphics, plugins: &mut Plugins, state: &mut State) {}
}
impl<State: UIStateCl, K: Clone, V: Clone> Default for Data<State, K, V> {
    fn default() -> Self { Self { data: HashMap::new(), boo: PhantomData } }
}
impl<State: UIStateCl, K: Clone, V: Clone> Positionable for Data<State, K, V> {
    fn with_pos(&self, to_add: Position) -> Self { self.clone() }
    fn add_pos(&mut self, to_add: Position) {}
    fn get_size(&self) -> Size { Default::default() }
    fn get_pos(&self) -> Position { Position::default() }
}

#[derive(Clone)]
struct Image<'a, State: UIStateCl> {
    pub image: &'a Texture,
    pub rect: Rect,
    pub crop: Rect,
    pub color: Color,
    pub boo: PhantomData<State>
}
impl<State: UIStateCl> Form<State> for Image<'_, State> {
    fn draw(&mut self, app: &mut App, gfx: &mut Graphics, plugins: &mut Plugins, state: &mut State) {
        Access::<Draw>::get_mut(state).image(self.image)
            .position(self.rect.pos.0, self.rect.pos.1)
            .size(self.rect.size.0, self.rect.size.1)
            .crop(self.crop.pos.into(), self.crop.size.into())
            .color(self.color);
    }
    fn after(&mut self, app: &mut App, gfx: &mut Graphics, plugins: &mut Plugins, state: &mut State) {}
}
impl<State: UIStateCl> Positionable for Image<'_, State> {
    fn with_pos(&self, to_add: Position) -> Self {
        Self { rect: Rect { pos: self.rect.pos + to_add, size: self.rect.size }, ..self.clone() }
    }
    fn add_pos(&mut self, to_add: Position) {
        self.rect.pos += to_add;
    }
    fn get_size(&self) -> Size { self.rect.size }
    fn get_pos(&self) -> Position { self.rect.pos }
}
