#![allow(unused_parens)]

use notan::prelude::Texture;

use super::{
    containers::{center, Centered},
    defs::*,
    form::Form,
    rect::*,
    wrappers::Background,
};
use derive_builder::Builder;
use notan::{
    draw::*,
    prelude::{App, Assets, Color, Graphics, Plugins},
};
use std::{fmt::Debug, marker::PhantomData};

#[derive(Copy, Clone, Debug)]
pub struct FontId(pub usize);
impl From<usize> for FontId {
    fn from(value: usize) -> Self {
        Self(value)
    }
}
impl Default for FontId {
    fn default() -> Self {
        Self(0)
    }
}

pub trait ToText<State> = TryToWith<State, String>;
#[derive(Clone, Builder)]
#[builder(build_fn(error = "StructBuildError"), pattern = "owned")]
pub struct Text<State: UIStateCl, T: ToText<State>> {
    #[builder(setter(into))]
    pub text: T,
    #[builder(setter(into), default)]
    pub font: FontId,
    #[builder(default = "AlignHorizontal::Left")]
    pub align_h: AlignHorizontal,
    #[builder(default = "AlignVertical::Top")]
    pub align_v: AlignVertical,
    #[builder(setter(into), default)]
    pub pos: Position,
    #[builder(default = "0.")]
    pub size: f32,
    #[builder(setter(into, strip_option), default = "None")]
    pub rect_size: Option<Size>,
    #[builder(setter(into, strip_option), default = "None")]
    pub max_width: Option<f32>,
    #[builder(setter(into), default = "Color::BLACK")]
    pub color: Color,
    #[builder(default)]
    pub boo: PhantomData<State>,
}
pub fn text<State: UIStateCl, T: ToText<State>>(text: T) -> TextBuilder<State, T> {
    TextBuilder::default().text(text)
}
impl<State: UIStateCl, T: ToText<State>> Debug for Text<State, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Text")
            .field("pos", &self.pos)
            .field("width", &self.max_width)
            .field("align_v", &self.align_v)
            .field("align_h", &self.align_h)
            .field("color", &self.color)
            .finish_non_exhaustive()
    }
}
impl<State: UIStateCl, T: ToText<State>> Form<State> for Text<State, T> {
    fn draw(
        &mut self,
        app: &mut App,
        assets: &mut Assets,
        gfx: &mut Graphics,
        plugins: &mut Plugins,
        state: &mut State,
        draw: &mut Draw,
    ) {
        let font = Access::<Vec<Font>>::get_mut(state)
            .get(self.font.0)
            .expect(&*format!("Cant find font with index {}", self.font.0))
            .clone();
        {
            let text = &*self.text.with_to(&state);
            let mut text_builder = draw.text(&font, text);
            let mut text_builder = match self.align_h {
                AlignHorizontal::Left => text_builder.h_align_left(),
                AlignHorizontal::Center => text_builder.h_align_center(),
                AlignHorizontal::Right => text_builder.h_align_right(),
            };
            text_builder = match self.align_v {
                AlignVertical::Bottom => text_builder.v_align_bottom(),
                AlignVertical::Center => text_builder.v_align_middle(),
                AlignVertical::Top => text_builder.v_align_top(),
            };
            let mut pos = self.pos;
            text_builder
                .position(pos.0, pos.1)
                .size(self.size)
                .color(self.color);
            if let Some(width) = self.max_width {
                text_builder.max_width(width);
            }
        }
        if self.rect_size == None {
            let bounds = draw.last_text_bounds();
            self.rect_size = Some(Size(bounds.width, bounds.height));
        }
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
impl<State: UIStateCl, T: ToText<State>> Positionable for Text<State, T> {
    fn with_pos(&self, to_add: Position) -> Self {
        Self {
            pos: self.pos + to_add,
            ..self.clone()
        }
    }
    fn add_pos(&mut self, to_add: Position) {
        self.pos += to_add;
    }
    fn get_size(&self) -> Size {
        let mut size = Size(0., 0.);
        if let Some(rect_size) = self.rect_size {
            return rect_size;
        }
        if let Some(width) = self.max_width {
            size.0 = width;
        }
        if let Some(text) = self.text.try_to() {
            return Size(
                if size.0 == 0. {
                    self.size * (text.chars().count() as f32)
                } else {
                    size.0
                },
                self.size,
            );
        }
        size
    }
    fn get_pos(&self) -> Position {
        self.pos
    }
}
impl<'a, State: UIStateCl, T: ToText<State> + 'a, Tex: ToTexture<'a, State> + 'a>
    Styled<'a, State, Tex> for Text<State, T>
{
    type Output = Centered<State, Background<'a, State, Text<State, T>, Tex>>;
    fn style(self, style: Style<'a, State, Tex>) -> Self::Output {
        center(style.centered, Background::new(self, style.background))
    }
}
impl<State: UIStateCl, T: ToText<State> + Default> Default for Text<State, T> {
    fn default() -> Self {
        Self {
            text: Default::default(),
            font: FontId(0),
            align_h: AlignHorizontal::Left,
            align_v: AlignVertical::Top,
            pos: Position(0., 0.),
            size: 0.,
            rect_size: None,
            max_width: None,
            color: Color::BLACK,
            boo: PhantomData,
        }
    }
}
impl<State: UIStateCl, T: ToText<State>> Text<State, T> {
    pub fn new(
        text: T,
        font: impl Into<FontId>,
        align_h: AlignHorizontal,
        align_v: AlignVertical,
        pos: Position,
        size: f32,
        max_width: Option<f32>,
        color: Color,
    ) -> Self {
        Self {
            text,
            font: font.into(),
            align_h,
            align_v,
            pos,
            size,
            rect_size: None,
            max_width,
            color,
            boo: PhantomData,
        }
    }
}
#[derive(Clone, Debug, Builder)]
#[builder(build_fn(error = "StructBuildError"))]
pub struct TextChain<State: UIStateCl, T: ToText<State>> {
    #[builder(default)]
    pub texts: Vec<Text<State, T>>,
    #[builder(setter(into), default)]
    pub pos: Position,
    pub max_width: f32,
}
impl<State: UIStateCl, T: ToText<State>> Form<State> for TextChain<State, T> {
    fn draw(
        &mut self,
        app: &mut App,
        assets: &mut Assets,
        gfx: &mut Graphics,
        plugins: &mut Plugins,
        state: &mut State,
        draw: &mut Draw,
    ) {
        let max_width = self.max_width;
        let mut add_pos = Position(0., 0.);
        self.texts.iter_mut().for_each(|text| {
            if text.max_width > Some(self.max_width) || text.max_width.is_none() {
                text.max_width = Some(max_width);
            }
            text.with_pos(self.pos + add_pos)
                .draw(app, assets, gfx, plugins, state, draw);
            if *text.max_width.as_ref().unwrap() == text.get_size().0 {
                add_pos.0 = 0.;
                add_pos.1 += text.get_size().1;
            } else {
                add_pos.0 += text.get_size().0;
            }
        });
    }
    fn after(
        &mut self,
        app: &mut App,
        assets: &mut Assets,
        plugins: &mut Plugins,
        state: &mut State,
    ) {
        self.texts.iter_mut().for_each(|text| {
            text.add_pos(self.pos);
            text.after(app, assets, plugins, state);
            text.add_pos(-self.pos);
        })
    }
}
impl<State: UIStateCl, T: ToText<State>> Positionable for TextChain<State, T> {
    fn with_pos(&self, to_add: Position) -> Self {
        Self {
            pos: self.pos + to_add,
            ..self.clone()
        }
    }
    fn add_pos(&mut self, to_add: Position) {
        self.pos += to_add;
    }
    fn get_size(&self) -> Size {
        (
            self.max_width,
            self.texts.iter().map(|text| text.get_size().1).sum(),
        )
            .into()
    }
    fn get_pos(&self) -> Position {
        self.pos
    }
}
