use {
    std::{
        marker::PhantomData,
        collections::HashMap,
    },
    notan::{
        prelude::{AppState, Color, Graphics, Plugins, App},
        app::{Event, Texture},
        draw::*
    },
    super::{
        form::Form,
        rect::*,
        defs::*
}   };

#[derive(Copy, Clone)]
pub struct FontId(pub usize);
impl From<usize> for FontId {
    fn from(value: usize) -> Self {
        Self(value)
}   }
#[derive(Clone)]
pub struct Text<State: UIStateCl> {
    pub text: String,
    pub font: FontId,
    pub align_h: AlignHorizontal,
    pub align_v: AlignVertical,
    pub pos: Position,
    pub size: f32,
    pub rect_size: Option<Size>,
    pub max_width: Option<f32>,
    pub color: Color,
    pub boo: PhantomData<State>
}
impl<State: UIStateCl> Form<State> for Text<State> {
    fn draw(&mut self, app: &mut App, gfx: &mut Graphics, plugins: &mut Plugins, state: &mut State) {
        let font = Access::<Vec<Font>>::get_mut(state)
           .get(self.font.0)
            .expect(&*format!("Cant find font with index {}", self.font.0)).clone();
        let draw = Access::<Draw>::get_mut(state);
        {
            let mut text_builder = draw.text(&font, &*self.text);
            let mut text_builder = match self.align_h {
                AlignHorizontal::Left => text_builder.h_align_left(),
                AlignHorizontal::Center => text_builder.h_align_center(),
                AlignHorizontal::Right => text_builder.h_align_right()
            };
            text_builder = match self.align_v {
                AlignVertical::Bottom => text_builder.v_align_bottom(),
                AlignVertical::Center => text_builder.v_align_middle(),
                AlignVertical::Top => text_builder.v_align_top()
            };
            let text_size = self.get_size();
            let mut pos = self.pos;
            text_builder
                .position(pos.0, pos.1)
                .size(self.size)
                .color(self.color);
            if let Some(width) = self.max_width { text_builder.max_width(width); }
        }
        if self.rect_size == None {
            let bounds = draw.last_text_bounds();
            self.rect_size = Some(Size(bounds.width, bounds.height));
    }   }
    fn after(&mut self, app: &mut App, gfx: &mut Graphics, plugins: &mut Plugins, state: &mut State) {}
}
impl<State: UIStateCl> Positionable for Text<State> {
    fn with_pos(&self, to_add: Position) -> Self {
        Self { pos: self.pos + to_add, ..self.clone() }
    }
    fn add_pos(&mut self, to_add: Position) {
        self.pos+=to_add;
    }
    fn get_size(&self) -> Size {
        let mut size = Size(0., 0.);
        if let Some(rect_size) = self.rect_size {
            size = rect_size;
        }
        if let Some(width) = self.max_width {
            size.0 = width;
        }
        if size == Size(0., 0.) && self.text.chars().count() > 0 {
            return Size(self.size * (self.text.chars().count() as f32), self.size)
        }
        size
    }
    fn get_pos(&self) -> Position { self.pos }
}
impl<State: UIStateCl> Default for Text<State> {
    fn default() -> Self {
        Self {
            text: "".into(),
            font: FontId(0),
            align_h: AlignHorizontal::Left,
            align_v: AlignVertical::Top,
            pos: Position(0., 0.),
            size: 0.,
            rect_size: None,
            max_width: None,
            color: Color::BLACK,
            boo: PhantomData
}   }   }
impl<State: UIStateCl> Text<State> {
    pub fn new(text: impl Into<String>, font: impl Into<FontId>,
               align_h: AlignHorizontal,
               align_v: AlignVertical,
               pos: Position,
               size: f32,
               max_width: Option<f32>,
               color: Color) -> Self {
        Self {
            text: text.into(),
            font: font.into(),
            align_h,
            align_v,
            pos,
            size,
            rect_size: None,
            max_width,
            color,
            boo: PhantomData
}   }   }

#[derive(Clone)]
pub struct TextChain<State: UIStateCl> {
    pub texts: Vec<Text<State>>,
    pub pos: Position,
    pub max_width: f32
}
impl<State: UIStateCl> Form<State> for TextChain<State> {
    fn draw(&mut self, app: &mut App, gfx: &mut Graphics, plugins: &mut Plugins, state: &mut State) {
        let max_width = self.max_width;
        let mut add_pos = Position(0., 0.);
        self.texts.iter_mut().for_each(|text| {
            if text.max_width > Some(self.max_width) || text.max_width.is_none() {
                text.max_width = Some(max_width);
            }
            text.with_pos(self.pos + add_pos).draw(app, gfx, plugins, state);
            if *text.max_width.as_ref().unwrap() == text.get_size().0 {
                add_pos.0 = 0.;
                add_pos.1 += text.get_size().1;
            } else {
                add_pos.0 += text.get_size().0;
            }
        });
    }
    fn after(&mut self, app: &mut App, gfx: &mut Graphics, plugins: &mut Plugins, state: &mut State) {
        self.texts.iter_mut().for_each(|text| {
            text.add_pos(self.pos);
            text.after(app, gfx, plugins, state);
            text.add_pos(-self.pos);
        })
}   }
impl<State: UIStateCl> Positionable for TextChain<State> {
    fn with_pos(&self, to_add: Position) -> Self { Self { pos: self.pos + to_add, ..self.clone() } }
    fn add_pos(&mut self, to_add: Position) { self.pos += to_add; }
    fn get_size(&self) -> Size { (self.max_width, self.texts.iter().map(|text| text.get_size().1).sum()).into() }
    fn get_pos(&self) -> Position { self.pos }
}
