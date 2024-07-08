#![allow(unused_parens)]

use super::{defs::*, form::Form, rect::*};
use derive_builder::Builder;
use notan::{
    app::Texture,
    draw::*,
    prelude::{App, Assets, Color, Graphics, Plugins},
};
use std::{
    collections::HashMap,
    fmt::{Debug, Formatter},
    marker::PhantomData,
};

#[derive(Clone, Debug, Builder)]
#[builder(build_fn(error = "StructBuildError"), pattern = "owned")]
pub struct Data<State: UIStateCl, K: Clone + Send, V: Clone + Send> {
    #[builder(default)]
    pub data: HashMap<K, V>,
    #[builder(setter(skip), default)]
    pub boo: PhantomData<State>,
}
impl<State: UIStateCl, K: Clone + Send, V: Clone + Send> Form<State> for Data<State, K, V> {
    fn draw(
        &mut self,
        app: &mut App,
        assets: &mut Assets,
        gfx: &mut Graphics,
        plugins: &mut Plugins,
        state: &mut State,
        draw: &mut Draw,
    ) {
    }
    fn after(
        &mut self,
        app: &mut App,
        assets: &mut Assets,
        plugins: &mut Plugins,
        state: &mut State,
    ) {
    }
}
impl<State: UIStateCl, K: Clone + Send, V: Clone + Send> Default for Data<State, K, V> {
    fn default() -> Self {
        Self {
            data: HashMap::new(),
            boo: PhantomData,
        }
    }
}
impl<State: UIStateCl, K: Clone + Send, V: Clone + Send> Positionable for Data<State, K, V> {
    fn with_pos(&self, to_add: Position) -> Self {
        self.clone()
    }
    fn add_pos(&mut self, to_add: Position) {}
    fn get_size(&self) -> Size {
        Default::default()
    }
    fn get_pos(&self) -> Position {
        Position::default()
    }
}

#[derive(Clone, Debug, Builder)]
#[builder(build_fn(error = "StructBuildError"), pattern = "owned")]
struct Image<'a, State: UIStateCl> {
    pub image: &'a Texture,
    #[builder(default)]
    pub rect: Rect,
    #[builder(default)]
    pub crop: Rect,
    pub color: Color,
    #[builder(setter(skip), default)]
    pub boo: PhantomData<State>,
}
impl<State: UIStateCl> Form<State> for Image<'_, State> {
    fn draw(
        &mut self,
        app: &mut App,
        assets: &mut Assets,
        gfx: &mut Graphics,
        plugins: &mut Plugins,
        state: &mut State,
        draw: &mut Draw,
    ) {
        Access::<Draw>::get_mut(state)
            .image(self.image)
            .position(self.rect.pos.0, self.rect.pos.1)
            .size(self.rect.size.0, self.rect.size.1)
            .crop(self.crop.pos.into(), self.crop.size.into())
            .color(self.color);
    }
    fn after(
        &mut self,
        app: &mut App,
        assets: &mut Assets,
        plugins: &mut Plugins,
        state: &mut State,
    ) {
    }
}
impl<State: UIStateCl> Positionable for Image<'_, State> {
    fn with_pos(&self, to_add: Position) -> Self {
        Self {
            rect: Rect {
                pos: self.rect.pos + to_add,
                size: self.rect.size,
            },
            ..self.clone()
        }
    }
    fn add_pos(&mut self, to_add: Position) {
        self.rect.pos += to_add;
    }
    fn get_size(&self) -> Size {
        self.rect.size
    }
    fn get_pos(&self) -> Position {
        self.rect.pos
    }
}

#[derive(Clone, Builder)]
#[builder(build_fn(error = "StructBuildError"), pattern = "owned")]
pub struct Drawing<State: UIStateCl> {
    #[builder(default)]
    pub pos: Position,
    #[builder(setter(strip_option))]
    pub to_draw: DrawFunction<State, Drawing<State>>,
}
impl<State: UIStateCl> Debug for Drawing<State> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Drawing").field("pos", &self.pos).finish()
    }
}
impl<State: UIStateCl> Form<State> for Drawing<State> {
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
    fn after(
        &mut self,
        app: &mut App,
        assets: &mut Assets,
        plugins: &mut Plugins,
        state: &mut State,
    ) {
    }
}
impl<State: UIStateCl> Positionable for Drawing<State> {
    fn with_pos(&self, to_add: Position) -> Self {
        let mut cloned = self.clone();
        cloned.add_pos(to_add);
        cloned
    }
    fn add_pos(&mut self, to_add: Position) {
        self.pos += to_add;
    }
    fn get_size(&self) -> Size {
        Size::default()
    }
    fn get_pos(&self) -> Position {
        self.pos
    }
}
