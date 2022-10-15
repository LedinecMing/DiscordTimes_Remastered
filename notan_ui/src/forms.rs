use std::marker::PhantomData;
use {
    notan::{
        prelude::{AppState, Color, Graphics, Plugins, App},
        app::Event,
        draw::*
    },
    dyn_clone::{DynClone, clone_box},
    super::rect::*
};


#[derive(Copy, Clone)]
pub enum AlignHorizontal {
    Left,
    Center,
    Right
}
#[derive(Copy, Clone)]
pub enum AlignVertical {
    Top,
    Center,
    Bottom
}
pub trait UIState: AppState {
    fn mut_fonts(&mut self) -> &mut Vec<Font>;
    fn fonts(&self) -> &Vec<Font>;
    fn mut_draw(&mut self) -> &mut Draw;
    fn draw(&self) -> &Draw;
}
pub trait Positionable {
    fn add_pos(&mut self, to_add: Position) -> Self;
}
pub trait Form<State: UIState>: DynClone {
    fn draw(&mut self, gfx: &mut Graphics, _plugins: &mut Plugins, _state: &mut State, _app: &mut App);
    fn after(&mut self, _gfx: &mut Graphics, _plugins: &mut Plugins, _state: &mut State, _app: &mut App);
    fn on_event(&mut self, event: Event, _gfx: &mut Graphics, _plugins: &mut Plugins, _state: &mut State, _app: &mut App) {}
}
impl<State: UIState> Clone for Box<dyn Form<State>> {
    fn clone(&self) -> Self {
        clone_box(&**self)
    }
}
#[derive(Copy, Clone)]
pub struct FontId(pub usize);
impl From<usize> for FontId {
    fn from(value: usize) -> Self {
        Self(value)
}   }
#[derive(Clone)]
pub struct Text<State: UIState> {
    text: String,
    font: FontId,
    align_h: AlignHorizontal,
    align_v: AlignVertical,
    pos: Position,
    size: f32,
    color: Color,
    boo: PhantomData<State>
}
impl<State: UIState> Form<State> for Text<State> where State: Clone {
    fn draw(&mut self, gfx: &mut Graphics, _plugins: &mut Plugins, _state: &mut State, _app: &mut App) {
        let font = _state.fonts().get(self.font.0).expect("Cant find font with index 011").clone();
        let mut draw = _state.mut_draw();
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
        text_builder
            .position(self.pos.0, self.pos.1)
            .size(self.size)
            .color(self.color);
    }
    fn after(&mut self, _gfx: &mut Graphics, _plugins: &mut Plugins, _state: &mut State, _app: &mut App) {}
}
impl<State: UIState> Positionable for Text<State> where State: Clone {
    fn add_pos(&mut self, to_add: Position) -> Self {
        Self { pos: self.pos + to_add, ..self.clone() }
    }
}
impl<State: UIState> Default for Text<State> {
    fn default() -> Self {
        Self {
            text: "".into(),
            font: FontId(0),
            align_h: AlignHorizontal::Left,
            align_v: AlignVertical::Top,
            pos: Position(0., 0.),
            size: 0.,
            color: Color::BLACK,
            boo: PhantomData
}   }   }
impl<State: UIState> Text<State> {
    pub fn new(text: impl Into<String>, font: impl Into<FontId>,
           align_h: AlignHorizontal,
           align_v: AlignVertical,
           pos: Position,
           size: f32,
           color: Color) -> Self {
        Self {
            text: text.into(), font: font.into(), align_h, align_v, pos, size, color,
            boo: PhantomData
}   }   }
#[derive(Clone)]
pub struct Button<State: UIState, T: Form<State> + Positionable> where State: Clone, T: Clone {
    inside: Option<T>,
    pub rect: Rect,
    if_hovered: Option<fn(&mut Self, _gfx: &mut Graphics, _plugins: &mut Plugins, _state: &mut State, _app: &mut App)>,
    if_clicked: Option<fn(&mut Self, _gfx: &mut Graphics, _plugins: &mut Plugins, _state: &mut State, _app: &mut App)>,
    focused: bool,
    selected: bool
}
impl<State: UIState + Clone, T: Form<State> + Positionable + Clone> Form<State> for Button<State, T> {
    fn draw(&mut self, gfx: &mut Graphics, _plugins: &mut Plugins, _state: &mut State, _app: &mut App) {
        if let Some(form) = &mut self.inside {
            form.draw(gfx, _plugins, _state, _app);
    }   }
    fn after(&mut self, _gfx: &mut Graphics, _plugins: &mut Plugins, _state: &mut State, _app: &mut App) {
        let mouse_pos = Position(_app.mouse.x, _app.mouse.y);
        self.focused = self.rect.collides(mouse_pos);
        self.selected = _app.mouse.left_was_released() && self.focused;
        if self.focused {
            if let Some(func) = self.if_hovered { func(self, _gfx, _plugins, _state, _app); }
            if self.selected {
                if let Some(func) = self.if_clicked { func(self, _gfx, _plugins, _state, _app); }
        }   }
        if let Some(form) = &mut self.inside {
            form.after(_gfx, _plugins, _state, _app);
}    }   }
impl<State: UIState + Clone, T: Form<State> + Positionable + Clone> Positionable for Button<State, T> {
    fn add_pos(&mut self, to_add: Position) -> Self {
        Self { rect: Rect { pos: self.rect.pos + to_add, size: self.rect.size}, ..self.clone() }
}   }
impl<State: UIState + Clone, T: Form<State> + Positionable + Clone> Button<State, T> {
    pub fn new(mut inside: Option<T>, rect: Rect,
        if_hovered: Option<fn(&mut Self, _gfx: &mut Graphics, _plugins: &mut Plugins, _state: &mut State, _app: &mut App)>,
        if_clicked: Option<fn(&mut Self, _gfx: &mut Graphics, _plugins: &mut Plugins, _state: &mut State, _app: &mut App)>
    ) -> Self {
        Button { inside: match &mut inside {
                Some(inside) => Some(inside.add_pos(rect.pos)),
                None => None
            },
            rect,
            if_hovered,
            if_clicked,
            focused: false,
            selected: false
        }
}   }