use {
    std::{
        marker::PhantomData
    },
    notan::{
        prelude::{AppState, Color, Graphics, Plugins, App},
        app::Event,
        draw::*
    },
    dyn_clone::{DynClone, clone_box},
    super::rect::*
};


#[derive(Copy, Clone, PartialEq)]
pub enum AlignHorizontal {
    Left,
    Center,
    Right
}
#[derive(Copy, Clone, PartialEq)]
pub enum AlignVertical {
    Top,
    Center,
    Bottom
}
#[derive(Copy, Clone, PartialEq)]
pub enum Direction {
    Right,
    Left,
    Top,
    Bottom
}

pub trait UIState: AppState {
    fn mut_fonts(&mut self) -> &mut Vec<Font>;
    fn fonts(&self) -> &Vec<Font>;
    fn mut_draw(&mut self) -> &mut Draw;
    fn draw(&self) -> &Draw;
}
pub trait Positionable {
    fn with_pos(&self, to_add: Position) -> Self;
    fn add_pos(&mut self, to_add: Position);
    fn get_size(&self) -> Position;
}
pub trait Form<State: UIState>: DynClone {
    fn draw(&mut self, app: &mut App, gfx: &mut Graphics, plugins: &mut Plugins, state:&mut State);
    fn after(&mut self, app: &mut App, gfx: &mut Graphics, plugins: &mut Plugins, state: &mut State);
    fn on_event(&mut self, event: Event, app: &mut App, gfx: &mut Graphics, plugins: &mut Plugins, state: &mut State) {}
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
    pub text: String,
    pub font: FontId,
    pub align_h: AlignHorizontal,
    pub align_v: AlignVertical,
    pub pos: Position,
    pub size: f32,
    pub max_width: Option<f32>,
    pub color: Color,
    pub boo: PhantomData<State>
}
impl<State: UIState> Form<State> for Text<State> where State: Clone {
    fn draw(&mut self, app: &mut App, gfx: &mut Graphics, plugins: &mut Plugins, state: &mut State) {
        let font = state.fonts().get(self.font.0).expect(&*format!("Cant find font with index {}", self.font.0)).clone();
        let mut draw = state.mut_draw();
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
        if self.align_v == AlignVertical::Center || true {
            pos.1 += text_size.1 / 2.;
        }
        if self.align_h == AlignHorizontal::Center || true {
            pos.0 += text_size.0 / 2.;
        }
        text_builder
            .position(pos.0, pos.1)
            .size(self.size)
            .color(self.color);
        if let Some(width) = self.max_width { text_builder.max_width(width); }
    }
    fn after(&mut self, app: &mut App, gfx: &mut Graphics, plugins: &mut Plugins, state: &mut State) {}
}
impl<State: UIState> Positionable for Text<State> where State: Clone {
    fn with_pos(&self, to_add: Position) -> Self {
        Self { pos: self.pos + to_add, ..self.clone() }
    }
    fn add_pos(&mut self, to_add: Position) {
        self.pos+=to_add;
    }
    fn get_size(&self) -> Position {
        if let Some(width) = self.max_width {
            return Position(width, self.size);
        }
        Position(self.size * (self.text.chars().count() as f32), self.size)
}   }
impl<State: UIState> Default for Text<State> {
    fn default() -> Self {
        Self {
            text: "".into(),
            font: FontId(0),
            align_h: AlignHorizontal::Left,
            align_v: AlignVertical::Top,
            pos: Position(0., 0.),
            size: 0.,
            max_width: None,
            color: Color::BLACK,
            boo: PhantomData
}   }   }
impl<State: UIState> Text<State> {
    pub fn new(text: impl Into<String>, font: impl Into<FontId>,
           align_h: AlignHorizontal,
           align_v: AlignVertical,
           pos: Position,
           size: f32,
           max_width: Option<f32>,
           color: Color) -> Self {
        Self {
            text: text.into(), font: font.into(), align_h, align_v, pos, size, max_width, color,
            boo: PhantomData
}   }   }
#[derive(Clone)]
pub struct Button<State: UIState, T: Form<State> + Positionable> where State: Clone, T: Clone {
    pub inside: Option<T>,
    pub rect: Rect,
    pub if_hovered: Option<fn(&mut Self, &mut App, &mut Graphics, &mut Plugins, &mut State)>,
    pub if_clicked: Option<fn(&mut Self, &mut App, &mut Graphics, &mut Plugins, &mut State)>,
    pub focused: bool,
    pub selected: bool
}
impl<State: UIState + Clone, T: Form<State> + Positionable + Clone> Form<State> for Button<State, T> {
    fn draw(&mut self, app: &mut App, gfx: &mut Graphics, plugins: &mut Plugins, state: &mut State) {
        if let Some(form) = &mut self.inside {
            form.with_pos(self.rect.pos).draw( app, gfx, plugins, state);
    }   }
    fn after(&mut self, app: &mut App, gfx: &mut Graphics, plugins: &mut Plugins, state: &mut State) {
        let mouse_pos = Position(app.mouse.x, app.mouse.y);
        self.focused = self.rect.collides(mouse_pos);
        self.selected = app.mouse.left_was_released() && self.focused;
        if self.focused {
            if let Some(func) = self.if_hovered { func(self, app, gfx, plugins, state); }
            if self.selected {
                if let Some(func) = self.if_clicked { func(self, app, gfx, plugins, state); }
        }   }
        if let Some(form) = &mut self.inside {
            form.after( app, gfx, plugins, state);
}    }   }
impl<State: UIState + Clone, T: Form<State> + Positionable + Clone> Positionable for Button<State, T> {
    fn with_pos(&self, to_add: Position) -> Self {
        Self { rect: Rect { pos: self.rect.pos + to_add, size: self.rect.size }, ..self.clone() }
    }
    fn add_pos(&mut self, to_add: Position) {
        self.rect.pos += to_add;
    }
    fn get_size(&self) -> Position {
        self.rect.size
}   }
impl<State: UIState + Clone, T: Form<State> + Positionable + Clone> Button<State, T> {
    pub fn new(mut inside: Option<T>, rect: Rect,
        if_hovered: Option<fn(&mut Self, &mut App, &mut Graphics, &mut Plugins, &mut State)>,
        if_clicked: Option<fn(&mut Self, &mut App, &mut Graphics, &mut Plugins, &mut State)>
    ) -> Self {
        Self {
            inside,
            rect,
            if_hovered,
            if_clicked,
            focused: false,
            selected: false
}   }   }

#[derive(Clone)]
pub struct Container<State: UIState + Clone, T: Form<State> + Positionable + Clone> {
    pub inside: Vec<T>,
    pub pos: Position,
    pub align_direction: Direction,
    pub interval: Position,
    pub boo: PhantomData<State>
}
impl<State: UIState + Clone, T: Form<State> + Positionable + Clone> Form<State> for Container<State, T> {
    fn draw(&mut self, app: &mut App, gfx: &mut Graphics, plugins: &mut Plugins, state: &mut State) {
        self.calc_insides().iter_mut().for_each(|form| form.draw(app, gfx, plugins, state));
    }
    fn after(&mut self, app: &mut App, gfx: &mut Graphics, plugins: &mut Plugins, state: &mut State) {
        let inside_len = self.inside.len();
        let inside_sizes = self.inside.iter().map(|form| form.get_size()).collect::<Vec<Position>>();
        self.inside.iter_mut().zip(0..inside_len).for_each(|(inside, i): (&mut T, usize)| {
            let sizes = &inside_sizes[0..i];
            let interval = self.interval * ((i - 1) as f32);
            let pos = self.pos;
            let to_add = match self.align_direction {
                Direction::Right => {
                    Position(sizes.iter().map(|&size| size.0).sum::<f32>() + interval.0 + pos.0, pos.1 + interval.1)
                }
                Direction::Left => {
                    Position(-sizes.iter().map(|&size| size.0).sum::<f32>() + interval.0 + pos.0, pos.1 + interval.1)
                }
                Direction::Top => {
                    Position(pos.0 + interval.0, -sizes.iter().map(|&size| size.1).sum::<f32>() + interval.1 + pos.1)
                }
                Direction::Bottom => {
                    Position(pos.0 + interval.0, sizes.iter().map(|&size| size.1).sum::<f32>() + interval.1 + pos.1)
                }
            };
            inside.add_pos(to_add);
            inside.after(app, gfx, plugins, state);
            inside.add_pos(-to_add);
        });
}   }
impl<State: UIState + Clone, T: Form<State> + Positionable + Clone> Container<State, T> {
    pub fn calc_insides(&self) -> Vec<T> {
        let inside_sizes = self.inside.iter().map(|form| form.get_size()).collect::<Vec<Position>>();

        self.inside.iter().zip(0..(self.inside.len())).map(|(inside, i): (&T, usize)| {
            let sizes = &inside_sizes[0..i];
            let interval = self.interval * ((i - 1) as f32);
            let to_add = match self.align_direction {
                Direction::Right => {
                    Position(sizes.iter().map(|&size| size.0).sum::<f32>() + interval.0 + self.pos.0, self.pos.1 + interval.1)
                }
                Direction::Left => {
                    Position(-sizes.iter().map(|&size| size.0).sum::<f32>() + interval.0 + self.pos.0, self.pos.1 + interval.1)
                }
                Direction::Top => {
                    Position(self.pos.0 + interval.0, -sizes.iter().map(|&size| size.1).sum::<f32>() + interval.1 + self.pos.1)
                }
                Direction::Bottom => {
                    Position(self.pos.0 + interval.0, sizes.iter().map(|&size| size.1).sum::<f32>() + interval.1 + self.pos.1)
                }
            };
            inside.with_pos(to_add)
        }).collect()
}   }
impl<State: UIState + Clone, T: Form<State> + Positionable + Clone> Positionable for Container<State, T> {
    fn with_pos(&self, to_add: Position) -> Self {
        Self { pos: self.pos + to_add, ..self.clone() }
    }
    fn add_pos(&mut self, to_add: Position) {
        self.pos += to_add;
    }
    fn get_size(&self) -> Position {
        if self.inside.is_empty() {
            Position(0., 0.);
        }
        let sizes = self.calc_insides().iter().map(|form| form.get_size()).collect::<Vec<Position>>();
        let (sizes_h, sizes_v) = (
            sizes.iter().map(|form| form.0),
            sizes.iter().map(|form| form.1));

        let (mut summed_h, mut summed_v) = (0., 0.);
        match self.align_direction {
            Direction::Left | Direction::Right => {
                summed_h = sizes_h.clone().sum::<f32>();
            }
            Direction::Bottom | Direction::Top => {
                summed_v = sizes_v.clone().sum::<f32>();
        }   }
        let interval = self.interval * (sizes.len() - 1) as f32;
        Position(
            match self.align_direction {
                Direction::Right => {
                    summed_h + interval.0
                }
                Direction::Left => {
                    -summed_h + interval.0
                },
                _ => sizes_v.max_by(|a, b| a.partial_cmp(b).unwrap()).unwrap()
            },
            match self.align_direction {
                Direction::Top => {
                    -summed_v + interval.1
                }
                Direction::Bottom => {
                    summed_v + interval.1
                },
                _ => sizes_h.max_by(|a, b| a.partial_cmp(b).unwrap()).unwrap()
            }
        )
}   }
impl<State: UIState + Clone, T: Form<State> + Positionable + Clone> Default for Container<State, T> {
    fn default() -> Self {
        Self {
            inside: vec![],
            pos: Position(0., 0.),
            align_direction: Direction::Right,
            interval: Position(0., 0.),
            boo: PhantomData
}   }   }

#[derive(Clone)]
pub struct SingleContainer<State: UIState + Clone, T: Form<State> + Positionable + Clone> {
    pub inside: Option<T>,
    pub on_draw: Option<fn(&mut Self, &mut App, &mut Graphics, &mut Plugins, &mut State)>,
    pub after_draw: Option<fn(&mut Self, &mut App, &mut Graphics, &mut Plugins, &mut State)>,
    pub pos: Position
}
impl<State: UIState + Clone, T: Form<State> + Positionable + Clone> Form<State> for SingleContainer<State, T> {
    fn draw(&mut self, app: &mut App, gfx: &mut Graphics, plugins: &mut Plugins, state: &mut State) {
        if let Some(form) = &mut self.inside {
            form.add_pos(self.pos);
        }
        if let Some(func) = self.on_draw {
            func(self, app, gfx, plugins, state);
        }
        if let Some(form) = &mut self.inside {
            form.draw(app, gfx, plugins, state);
            form.add_pos(-self.pos);
    }   }

    fn after(&mut self, app: &mut App, gfx: &mut Graphics, plugins: &mut Plugins, state: &mut State) {
        if let Some(form) = &mut self.inside {
            form.add_pos(self.pos);
        }
        if let Some(func) = &mut self.after_draw {
            func(self, app, gfx, plugins, state);
        }
        if let Some(form) = &mut self.inside {
            form.after(app, gfx, plugins, state);
            form.add_pos(-self.pos);
}   }   }
impl<State: UIState + Clone, T: Form<State> + Positionable + Clone> Positionable for SingleContainer<State, T> {
    fn with_pos(&self, to_add: Position) -> Self {
        let mut cloned = self.clone();
        cloned.add_pos(to_add);
        cloned
    }
    fn add_pos(&mut self, to_add: Position) {
        self.pos += to_add;
    }
    fn get_size(&self) -> Position {
        if let Some(form) = &self.inside {
            return form.get_size();
        }
        Position(0., 0.)
}   }
impl<State: UIState + Clone, T: Form<State> + Positionable + Clone> Default for SingleContainer<State, T> {
    fn default() -> Self {
        Self {
            inside: None,
            on_draw: None,
            after_draw: None,
            pos: Position(0., 0.)
}   }   }

#[derive(Clone)]
struct Slider<State: UIState + Clone, T: Form<State> + Positionable + Clone> {
    pub rect: Rect,
    pub slider_inside: T,
    pub scroll: f32,
    pub boo: PhantomData<State>
}
impl<State: UIState + Clone, T: Form<State> + Positionable + Clone> Form<State> for Slider<State, T> {
    fn draw(&mut self, app: &mut App, gfx: &mut Graphics, plugins: &mut Plugins, state: &mut State) {
        self.slider_inside.with_pos(self.rect.size + Position(0.,self.scroll)).draw(app, gfx, plugins, state);
    }
    fn after(&mut self, app: &mut App, gfx: &mut Graphics, plugins: &mut Plugins, state: &mut State) {
        self.slider_inside.with_pos(self.rect.size + Position(0., self.scroll)).after(app, gfx, plugins, state);
}   }
impl<State: UIState + Clone, T: Form<State> + Positionable + Clone> Positionable for Slider<State, T> {
    fn with_pos(&self, to_add: Position) -> Self {
        let mut cloned = self.clone();
        cloned.add_pos(to_add);
        cloned
    }
    fn add_pos(&mut self, to_add: Position) {
        self.rect.pos += to_add;
    }
    fn get_size(&self) -> Position {
        self.rect.size
}   }

#[derive(Clone)]
struct SliderContainer<State: UIState + Clone, T: Form<State> + Positionable + Clone> {
    pub inside: T,
    pub slider: Slider<State, T>
}
impl<State: UIState + Clone, T: Form<State> + Positionable + Clone> Form<State> for SliderContainer<State, T> {
    fn draw(&mut self, app: &mut App, gfx: &mut Graphics, plugins: &mut Plugins, state: &mut State) {
        self.slider.draw(app, gfx, plugins, state);
        self.inside.with_pos(Position(0.,self.slider.scroll)).draw(app, gfx, plugins, state);
    }

    fn after(&mut self, app: &mut App, gfx: &mut Graphics, plugins: &mut Plugins, state: &mut State) {
        self.inside.add_pos(Position(0.,self.slider.scroll));
        self.inside.after(app, gfx, plugins, state);
        self.inside.add_pos(-Position(0.,self.slider.scroll));
    }
    fn on_event(&mut self, event: Event, app: &mut App, gfx: &mut Graphics, plugins: &mut Plugins, state: &mut State) {
        match event {
            Event::MouseWheel { delta_x: _delta_x, delta_y } => {
                self.slider.scroll = delta_y;
            },
            _ => {}
}   }   }
impl<State: UIState + Clone, T: Form<State> + Positionable + Clone> Positionable for SliderContainer<State, T> {
    fn with_pos(&self, to_add: Position) -> Self {
        Self { inside: self.inside.clone(), slider: self.slider.with_pos(to_add) }
    }
    fn add_pos(&mut self, to_add: Position) {
        self.slider.add_pos(to_add);
    }
    fn get_size(&self) -> Position {
        self.inside.get_size()
}   }