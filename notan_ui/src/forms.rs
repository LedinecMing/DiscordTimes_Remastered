#![allow(unused_parens)]

use std::fmt::{Debug, Formatter};
use notan::prelude::Assets;
use {
    std::{
        marker::PhantomData,
        collections::HashMap,
    },
    notan::{
        prelude::{Color, Graphics, Plugins, App},
        app::{Texture},
        draw::*
    },
    super::{
        form::Form,
        rect::*,
        defs::*
}   };

#[derive(Clone, Debug)]
pub struct Data<State: UIStateCl, K: Clone + Send, V: Clone + Send> {
    pub data: HashMap<K, V>,
    pub boo: PhantomData<State>
}
impl<State: UIStateCl, K: Clone + Send, V: Clone + Send> Form<State> for Data<State, K, V> {
    fn draw(&mut self, app: &mut App, assets: &mut Assets, gfx: &mut Graphics, plugins: &mut Plugins, state: &mut State) {}
    fn after(&mut self, app: &mut App, assets: &mut Assets, plugins: &mut Plugins, state: &mut State) {}
}
impl<State: UIStateCl, K: Clone + Send, V: Clone + Send> Default for Data<State, K, V> {
    fn default() -> Self { Self { data: HashMap::new(), boo: PhantomData } }
}
impl<State: UIStateCl, K: Clone + Send, V: Clone + Send> Positionable for Data<State, K, V> {
    fn with_pos(&self, to_add: Position) -> Self { self.clone() }
    fn add_pos(&mut self, to_add: Position) {}
    fn get_size(&self) -> Size { Default::default() }
    fn get_pos(&self) -> Position { Position::default() }
}

#[derive(Clone, Debug)]
struct Image<'a, State: UIStateCl> {
    pub image: &'a Texture,
    pub rect: Rect,
    pub crop: Rect,
    pub color: Color,
    pub boo: PhantomData<State>
}
impl<State: UIStateCl> Form<State> for Image<'_, State> {
    fn draw(&mut self, app: &mut App, assets: &mut Assets, gfx: &mut Graphics, plugins: &mut Plugins, state: &mut State) {
        Access::<Draw>::get_mut(state).image(self.image)
            .position(self.rect.pos.0, self.rect.pos.1)
            .size(self.rect.size.0, self.rect.size.1)
            .crop(self.crop.pos.into(), self.crop.size.into())
            .color(self.color);
    }
    fn after(&mut self, app: &mut App, assets: &mut Assets, plugins: &mut Plugins, state: &mut State) {}
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

#[derive(Clone)]
pub struct Drawing<State: UIStateCl> {
    pub pos: Position,
    pub to_draw: fn(&mut Self, &mut App, &mut Assets, &mut Graphics, &mut Plugins, &mut State)
}
impl<State: UIStateCl> Debug for Drawing<State> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Drawing")
            .field("pos", &self.pos)
            .finish()
}   }
impl<State: UIStateCl> Form<State> for Drawing<State> {
    fn draw(&mut self, app: &mut App, assets: &mut Assets, gfx: &mut Graphics, plugins: &mut Plugins, state: &mut State) {
        (self.to_draw)(self, app, assets, gfx, plugins, state);
    }
    fn after(&mut self, app: &mut App, assets: &mut Assets, plugins: &mut Plugins, state: &mut State) {}
}
impl<State: UIStateCl> Positionable for Drawing<State> {
    fn with_pos(&self, to_add: Position) -> Self {
        let mut cloned = self.clone();
        cloned.add_pos(to_add);
        cloned
    }
    fn add_pos(&mut self, to_add: Position) {
        self.pos+=to_add;
    }
    fn get_size(&self) -> Size { Size::default() }
    fn get_pos(&self) -> Position { self.pos }
}
