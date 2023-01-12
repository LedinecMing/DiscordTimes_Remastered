use std::fmt::{Debug, Formatter};
use {
    std::{
        marker::PhantomData,
        collections::HashMap,
    },
    notan::{
        prelude::{AppState, Color, Graphics, Plugins, App, Assets},
        app::{Event, Texture},
        draw::*
    },
    super::{
        form::Form,
        wrappers::Slider,
        rect::*,
        defs::*
}   };

#[derive(Clone, Debug)]
pub struct Container<State: UIStateCl, T: PosForm<State>> {
    pub inside: Vec<T>,
    pub pos: Position,
    pub align_direction: Direction,
    pub interval: Position,
    pub boo: PhantomData<State>
}
impl<State: UIStateCl, T: PosForm<State>> Form<State> for Container<State, T> {
    fn draw(&mut self, app: &mut App, assets: &mut Assets, gfx: &mut Graphics, plugins: &mut Plugins, state: &mut State) {
        self.calc_insides().iter_mut().for_each(|form| form.draw(app, assets, gfx, plugins, state));
    }
    fn after(&mut self, app: &mut App, assets: &mut Assets, plugins: &mut Plugins, state: &mut State) {
        let inside_len = self.inside.len();
        let inside_sizes = self.inside.iter().map(|form| form.get_size()).collect::<Vec<Size>>();
        self.inside.iter_mut().zip(0..inside_len).for_each(|(inside, i): (&mut T, usize)| {
            let sizes = &inside_sizes[0..i];
            let interval = self.interval * i as f32;
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
            inside.after(app, assets, plugins, state);
            inside.add_pos(-to_add);
        });
    }   }
impl<State: UIStateCl, T: PosForm<State>> Container<State, T> {
    pub fn calc_insides(&self) -> Vec<T> {
        let inside_sizes = self.inside.iter().map(|form| form.get_size()).collect::<Vec<Size>>();

        self.inside.iter().zip(0..(self.inside.len())).map(|(inside, i): (&T, usize)| {
            let sizes = &inside_sizes[0..i];
            let interval = self.interval * i as f32;
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
impl<State: UIStateCl, T: PosForm<State>> Positionable for Container<State, T> {
    fn with_pos(&self, to_add: Position) -> Self {
        Self { pos: self.pos + to_add, ..self.clone() }
    }
    fn add_pos(&mut self, to_add: Position) {
        self.pos += to_add;
    }
    fn get_size(&self) -> Size {
        if self.inside.is_empty() {
            Position(0., 0.);
        }
        let sizes = self.calc_insides().iter().map(|form| form.get_size()).collect::<Vec<Size>>();
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
            }
        }
        let interval = self.interval * sizes.len() as f32;
        Size(
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
    }
    fn get_pos(&self) -> Position { self.pos }
}
impl<State: UIStateCl, T: PosForm<State>> Default for Container<State, T> {
    fn default() -> Self {
        Self {
            inside: vec![],
            pos: Position(0., 0.),
            align_direction: Direction::Right,
            interval: Position(0., 0.),
            boo: PhantomData
}   }   }

#[derive(Clone)]
pub struct SingleContainer<State: UIStateCl, T: PosForm<State> + Debug> {
    pub inside: Option<T>,
    pub on_draw: Option<fn(&mut Self, &mut App, &mut Assets, &mut Graphics, &mut Plugins, &mut State)>,
    pub after_draw: Option<fn(&mut Self, &mut App, &mut Assets, &mut Plugins, &mut State)>,
    pub pos: Position
}
impl<State: UIStateCl, T: PosForm<State> + Debug> Debug for SingleContainer<State, T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SingleContainer")
            .field("inside", &self.inside)
            .field("pos", &self.pos)
            .finish()
}   }
impl<State: UIStateCl, T: PosForm<State> + Debug> Form<State> for SingleContainer<State, T> {
    fn draw(&mut self, app: &mut App, assets: &mut Assets, gfx: &mut Graphics, plugins: &mut Plugins, state: &mut State) {
        if let Some(form) = &mut self.inside {
            form.add_pos(self.pos);
        }
        if let Some(func) = self.on_draw {
            func(self, app, assets, gfx, plugins, state);
        }
        if let Some(form) = &mut self.inside {
            form.draw(app, assets, gfx, plugins, state);
            form.add_pos(-self.pos);
        }   }

    fn after(&mut self, app: &mut App, assets: &mut Assets, plugins: &mut Plugins, state: &mut State) {
        if let Some(form) = &mut self.inside {
            form.add_pos(self.pos);
        }
        if let Some(func) = &mut self.after_draw {
            func(self, app, assets, plugins, state);
        }
        if let Some(form) = &mut self.inside {
            form.after(app, assets, plugins, state);
            form.add_pos(-self.pos);
}   }   }
impl<State: UIStateCl, T: PosForm<State> + Debug> Positionable for SingleContainer<State, T> {
    fn with_pos(&self, to_add: Position) -> Self {
        let mut cloned = self.clone();
        cloned.add_pos(to_add);
        cloned
    }
    fn add_pos(&mut self, to_add: Position) {
        self.pos += to_add;
    }
    fn get_size(&self) -> Size {
        if let Some(form) = &self.inside {
            return form.get_size();
        }
        (0., 0.).into()
    }
    fn get_pos(&self) -> Position { self.pos }
}
impl<State: UIStateCl, T: PosForm<State> + Debug> Default for SingleContainer<State, T> {
    fn default() -> Self {
        Self {
            inside: None,
            on_draw: None,
            after_draw: None,
            pos: Position(0., 0.)
}   }   }

#[derive(Clone, Debug)]
pub struct SliderContainer<State: UIStateCl, T: PosForm<State>, K: PosForm<State>> {
    pub inside: T,
    pub slider: Slider<State, K>,
    pub slide_speed: f32,
}
impl<State: UIStateCl, T: PosForm<State>, K: PosForm<State>> Form<State> for SliderContainer<State, T, K> {
    fn draw(&mut self, app: &mut App, assets: &mut Assets, gfx: &mut Graphics, plugins: &mut Plugins, state: &mut State) {
        self.slider.draw(app, assets, gfx, plugins, state);
        self.inside.with_pos(Position(0.,self.slider.scroll)).draw(app, assets, gfx, plugins, state);
    }
    fn after(&mut self, app: &mut App, assets: &mut Assets, plugins: &mut Plugins, state: &mut State) {
        self.inside.add_pos(Position(0.,self.slider.scroll));
        let mouse = &app.mouse;
        let mouse_pos = (mouse.x, mouse.y).into();
        let inside_rect = Rect { pos: self.inside.get_pos(), size: self.inside.get_size() };
        let slider_rect = Rect { pos: self.slider.slider_inside.get_pos(), size: self.slider.slider_inside.get_size()};
        self.inside.after(app, assets, plugins, state);
        self.inside.add_pos(-Position(0.,self.slider.scroll));
        if inside_rect.collides(mouse_pos) || slider_rect.collides(mouse_pos) {
            self.slider.scroll += app.mouse.wheel_delta.y * self.slide_speed;
}   }   }
impl<State: UIStateCl, T: PosForm<State>, K: PosForm<State>> Positionable for SliderContainer<State, T, K> {
    fn with_pos(&self, to_add: Position) -> Self {
        Self { inside: self.inside.clone(), slider: self.slider.with_pos(to_add), slide_speed: self.slide_speed }
    }
    fn add_pos(&mut self, to_add: Position) {
        self.slider.add_pos(to_add);
    }
    fn get_size(&self) -> Size {
        self.inside.get_size()
    }
    fn get_pos(&self) -> Position { self.inside.get_pos() }
}
#[derive(Clone, Debug)]
pub struct StraightDynContainer<State: UIStateCl> {
    pub inside: Vec<Box<dyn ObjPosForm<State>>>,
    pub pos: Position
}
impl<State: UIStateCl> Form<State> for StraightDynContainer<State> {
    fn draw(&mut self, app: &mut App, assets: &mut Assets,  gfx: &mut Graphics, plugins: &mut Plugins, state: &mut State) {
        self.inside.iter_mut().for_each(|form| form.draw(app, assets, gfx, plugins, state));
    }
    fn after(&mut self, app: &mut App, assets: &mut Assets, plugins: &mut Plugins, state: &mut State) {
        self.inside.iter_mut().for_each(|inside: &mut Box<dyn ObjPosForm<State>>| {
            inside.add_pos_obj(self.pos);
            inside.after(app, assets, plugins, state);
            inside.add_pos_obj(-self.pos);
        });
    }   }
impl<State: UIStateCl> Positionable for StraightDynContainer<State> {
    fn with_pos(&self, to_add: Position) -> Self {
        let mut cloned = self.clone();
        cloned.add_pos(to_add);
        cloned
    }
    fn add_pos(&mut self, to_add: Position) {
        self.pos += to_add;
    }
    fn get_size(&self) -> Size { Default::default() }
    fn get_pos(&self) -> Position { self.pos }
}
impl<State: UIStateCl> Default for StraightDynContainer<State> {
    fn default() -> Self { Self {
        inside: vec![],
        pos: Position(0., 0.),
}   }   }

#[derive(Clone, Debug)]
pub struct DynContainer<State: UIStateCl> {
    pub inside: Vec<Box<dyn ObjPosForm<State>>>,
    pub pos: Position,
    pub align_direction: Direction,
    pub interval: Position
}
impl<State: UIStateCl> Form<State> for DynContainer<State> {
    fn draw(&mut self, app: &mut App, assets: &mut Assets,  gfx: &mut Graphics, plugins: &mut Plugins, state: &mut State) {
        self.calc_insides().iter_mut().for_each(|form| form.draw(app, assets, gfx, plugins, state));
    }
    fn after(&mut self, app: &mut App, assets: &mut Assets, plugins: &mut Plugins, state: &mut State) {
        let inside_len = self.inside.len();
        let inside_sizes = self.inside.iter().map(|form| form.get_size_obj()).collect::<Vec<Size>>();
        self.inside.iter_mut().zip(0..inside_len).for_each(|(inside, i): (&mut Box<dyn ObjPosForm<State>>, usize)| {
            let sizes = &inside_sizes[0..i];
            let interval = self.interval * i as f32;
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
            inside.add_pos_obj(to_add);
            inside.after(app, assets, plugins, state);
            inside.add_pos_obj(-to_add);
        });
}   }
impl<State: UIStateCl> DynContainer<State> {
    pub fn calc_insides(&self) -> Vec<Box<dyn ObjPosForm<State>>> {
        let inside_sizes = self.inside.iter().map(|form| form.get_size_obj()).collect::<Vec<Size>>();

        self.inside.iter().zip(0..(self.inside.len())).map(|(inside, i): (&Box<dyn ObjPosForm<State>>, usize)| {
            let sizes = &inside_sizes[0..i];
            let interval = self.interval * i as f32;
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
            let mut clonned = inside.clone();
            clonned.add_pos_obj(to_add);
            clonned
        }).collect()
    }   }
impl<State: UIStateCl> Positionable for DynContainer<State> {
    fn with_pos(&self, to_add: Position) -> Self {
        let mut cloned = self.clone();
        cloned.add_pos(to_add);
        cloned
    }
    fn add_pos(&mut self, to_add: Position) {
        self.pos += to_add;
    }
    fn get_size(&self) -> Size { Default::default() }
    fn get_pos(&self) -> Position { self.pos }
}
impl<State: UIStateCl> Default for DynContainer<State> {
    fn default() -> Self { Self {
        inside: vec![],
        pos: Position(0., 0.),
        align_direction: Direction::Right,
        interval: Position(0., 0.)
    }   }   }
