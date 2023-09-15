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
    derive_builder::Builder,
    super::{
        form::Form,
        wrappers::Slider,
        rect::*,
        defs::*
}   };
use crate::wrappers::SliderBuilder;

#[derive(Clone, Debug, Builder)]
#[builder(build_fn(error = "StructBuildError"), pattern="owned")]
pub struct Container<State: UIStateCl, T: PosForm<State>> {
    pub inside: Vec<T>,
    #[builder(setter(into), default)]
    pub pos: Position,
    #[builder(default = "Direction::Right")]
    pub align_direction: Direction,
    #[builder(setter(into), default)]
    pub interval: Position,
    #[builder(setter(skip), default)]
    pub boo: PhantomData<State>
}
pub fn container<State: UIStateCl, T: PosForm<State>>(inside: Vec<T>) -> ContainerBuilder<State, T> {
    ContainerBuilder::default()
        .inside(inside)
}
impl<State: UIStateCl, T: PosForm<State>> Form<State> for Container<State, T> {
    fn draw(&mut self, app: &mut App, assets: &mut Assets, gfx: &mut Graphics, plugins: &mut Plugins, state: &mut State, draw: &mut Draw) {
        self.calc_insides().iter_mut().for_each(|form| form.draw(app, assets, gfx, plugins, state, draw));
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


#[derive(Clone, Debug, Builder)]
#[builder(build_fn(error = "StructBuildError"), pattern="owned")]
pub struct TupleContainer<State: UIStateCl, T> {
    pub inside: T,
    #[builder(setter(into), default)]
    pub pos: Position,
    #[builder(default = "Direction::Right")]
    pub align_direction: Direction,
    #[builder(setter(into), default)]
    pub interval: Position,
    #[builder(setter(skip), default)]
    pub boo: PhantomData<State>
}

macro_rules! tuple_impls {
    () => {};
    (($idx:tt => $typ:ident), $( ($nidx:tt => $ntyp:ident), )*) => {
        tuple_impls!([($idx, $typ);] $( ($nidx => $ntyp), )*);
        tuple_impls!($( ($nidx => $ntyp), )*); // invoke macro on tail
    };
    ([$(($accIdx: tt, $accTyp: ident);)+]  ($idx:tt => $typ:ident), $( ($nidx:tt => $ntyp:ident), )*) => {
      tuple_impls!([($idx, $typ); $(($accIdx, $accTyp); )*] $( ($nidx => $ntyp), ) *);
    };

    ([($idx:tt, $typ:ident); $( ($nidx:tt, $ntyp:ident); )*]) => {
        impl<State: UIStateCl, $typ, $( $ntyp ),*> Form<State> for TupleContainer<State, ($typ, $( $ntyp ),*)>
        where
            $typ: ObjPosForm<State> + Clone,
            $( $ntyp: ObjPosForm<State> + Clone),*
        {
            fn draw(&mut self, app: &mut App, assets: &mut Assets, gfx: &mut Graphics, plugins: &mut Plugins, state: &mut State, draw: &mut Draw) {
                let mut inside: &mut [&mut dyn ObjPosForm<State>] = &mut [&mut self.inside.$idx, $( &mut self.inside.$nidx ),*];

                let mut size = Size(0., 0.);
                let mut interval = Position(0., 0.);
                inside.iter_mut().for_each(|inside| {
                    let to_add = match self.align_direction {
                        Direction::Right => {
                            Position(size.0 + interval.0 + self.pos.0, self.pos.1 + interval.1)
                        }
                        Direction::Left => {
                            Position(-size.0 + interval.0 + self.pos.0, self.pos.1 + interval.1)
                        }
                        Direction::Top => {
                            Position(self.pos.0 + interval.0, -size.1 + interval.1 + self.pos.1)
                        }
                        Direction::Bottom => {
                            Position(self.pos.0 + interval.0, size.1 + interval.1 + self.pos.1)
                        }
                    };

                    inside.add_pos_obj(to_add);
                    inside.draw(app, assets, gfx, plugins, state, draw);
                    inside.add_pos_obj(-to_add);
                    size += inside.get_size_obj();
                    interval += self.interval;
                });
            }
            fn after(&mut self, app: &mut App, assets: &mut Assets, plugins: &mut Plugins, state: &mut State) {
                let inside: &mut [&mut dyn ObjPosForm<State>] = &mut [&mut self.inside.$idx, $( &mut self.inside.$nidx ),*];

                let mut size = Size(0., 0.);
                let mut interval = Position(0., 0.);
                inside.iter_mut().for_each(|inside| {
                    let to_add = match self.align_direction {
                        Direction::Right => {
                            Position(size.0 + interval.0 + self.pos.0, self.pos.1 + interval.1)
                        }
                        Direction::Left => {
                            Position(-size.0 + interval.0 + self.pos.0, self.pos.1 + interval.1)
                        }
                        Direction::Top => {
                            Position(self.pos.0 + interval.0, -size.1 + interval.1 + self.pos.1)
                        }
                        Direction::Bottom => {
                            Position(self.pos.0 + interval.0, size.1 + interval.1 + self.pos.1)
                        }
                    };
                    inside.add_pos_obj(to_add);
                    inside.after(app, assets, plugins, state);
                    inside.add_pos_obj(-to_add);
                    size += inside.get_size_obj();
                    interval += self.interval;
                });
            }
        }
        impl<State: UIStateCl, $typ, $( $ntyp ),*> Positionable for TupleContainer<State, ($typ, $( $ntyp ),*)>
        where
            $typ: ObjPosForm<State> + Clone,
            $( $ntyp: ObjPosForm<State> + Clone),*
        {
            fn with_pos(&self, to_add: Position) -> Self {
                Self { pos: self.pos + to_add, ..self.clone() }
            }
            fn add_pos(&mut self, to_add: Position) {
                self.pos += to_add;
            }
            fn get_size(&self) -> Size {
                let inside: &[&dyn ObjPosForm<State>] = &[&self.inside.$idx, $( &self.inside.$nidx ),*];
                let sizes = inside.iter().map(|form| form.get_size_obj()).collect::<Vec<Size>>();
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
    };
}

tuple_impls!(
    (9 => J),
    (8 => I),
    (7 => H),
    (6 => G),
    (5 => F),
    (4 => E),
    (3 => D),
    (2 => C),
    (1 => B),
    (0 => A),
);

#[derive(Clone, Builder)]
#[builder(build_fn(error = "StructBuildError"), pattern="owned")]
pub struct SingleContainer<State: UIStateCl, T: PosForm<State> + Debug> {
    #[builder(setter(strip_option), default="None")]
    pub inside: Option<T>,
    #[builder(setter(strip_option), default="None")]
    pub on_draw: Option<DrawFunction<State, SingleContainer<State, T>>>,
    #[builder(setter(strip_option), default="None")]
    pub after_draw: Option<UpdateFunction<State, SingleContainer<State, T>>>,
    #[builder(default)]
    pub pos: Position
}
pub fn single<State: UIStateCl, T: PosForm<State>>(form: T) -> SingleContainerBuilder<State, T> {
    SingleContainerBuilder::default()
        .inside(form)
}
impl<State: UIStateCl, T: PosForm<State> + Debug> Debug for SingleContainer<State, T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SingleContainer")
            .field("inside", &self.inside)
            .field("pos", &self.pos)
            .finish()
}   }
impl<State: UIStateCl, T: PosForm<State> + Debug> Form<State> for SingleContainer<State, T> {
    fn draw(&mut self, app: &mut App, assets: &mut Assets, gfx: &mut Graphics, plugins: &mut Plugins, state: &mut State, draw: &mut Draw) {
        if let Some(form) = &mut self.inside {
            form.add_pos(self.pos);
        }
        if let Some(func) = self.on_draw {
            func(self, app, assets, gfx, plugins, state, draw);
        }
        if let Some(form) = &mut self.inside {
            form.draw(app, assets, gfx, plugins, state, draw);
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

#[derive(Clone, Debug, Builder)]
#[builder(build_fn(error = "StructBuildError"), pattern="owned")]
pub struct SliderContainer<State: UIStateCl, T: PosForm<State>, K: PosForm<State>> {
    pub inside: T,
    pub slider: Slider<State, K>,
    #[builder(default="1.")]
    pub slide_speed: f32,
}
pub fn slider<State: UIStateCl, T: PosForm<State>, K: PosForm<State>>(form: T, slider_inside: K, slider_rect: Rect, max_scroll: f32) -> SliderContainerBuilder<State, T, K> {
    SliderContainerBuilder::default()
        .inside(form)
        .slider(SliderBuilder::default()
            .rect(slider_rect)
            .slider_inside(slider_inside)
            .max_scroll(max_scroll)
            .build().unwrap())
}
impl<State: UIStateCl, T: PosForm<State>, K: PosForm<State>> Form<State> for SliderContainer<State, T, K> {
    fn draw(&mut self, app: &mut App, assets: &mut Assets, gfx: &mut Graphics, plugins: &mut Plugins, state: &mut State, draw: &mut Draw) {
        self.slider.draw(app, assets, gfx, plugins, state, draw);
        self.inside.with_pos(Position(0.,self.slider.scroll)).draw(app, assets, gfx, plugins, state, draw);
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
#[derive(Clone, Debug, Builder)]
#[builder(build_fn(error = "StructBuildError"), pattern="owned")]
pub struct StraightDynContainer<State: UIStateCl> {
    #[builder(default)]
    pub inside: Vec<Box<dyn ObjPosForm<State>>>,
    #[builder(setter(into), default)]
    pub pos: Position
}
pub fn straight_dyn<State: UIStateCl>(forms: Vec<Box<dyn ObjPosForm<State>>>) -> StraightDynContainerBuilder<State> {
    StraightDynContainerBuilder::default()
        .inside(forms)
}
impl<State: UIStateCl> Form<State> for StraightDynContainer<State> {
    fn draw(&mut self, app: &mut App, assets: &mut Assets,  gfx: &mut Graphics, plugins: &mut Plugins, state: &mut State, draw: &mut Draw) {
        self.inside.iter_mut().for_each(|form| form.draw(app, assets, gfx, plugins, state, draw));
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

#[derive(Clone, Debug, Builder)]
#[builder(build_fn(error = "StructBuildError"), pattern="owned")]
pub struct DynContainer<State: UIStateCl> {
    #[builder(default)]
    pub inside: Vec<Box<dyn ObjPosForm<State>>>,
    #[builder(setter(into), default)]
    pub pos: Position,
    #[builder(default = "Direction::Right")]
    pub align_direction: Direction,
    #[builder(default)]
    pub interval: Position
}

pub fn dyn_cont<State: UIStateCl>(forms: Vec<Box<dyn ObjPosForm<State>>>) -> DynContainerBuilder<State> {
    DynContainerBuilder::default()
        .inside(forms)
}
impl<State: UIStateCl> Form<State> for DynContainer<State> {
    fn draw(&mut self, app: &mut App, assets: &mut Assets,  gfx: &mut Graphics, plugins: &mut Plugins, state: &mut State, draw: &mut Draw) {
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
            inside.draw(app, assets, gfx, plugins, state, draw);
            inside.add_pos_obj(-to_add);
        });
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
impl<State: UIStateCl> Positionable for DynContainer<State> {
    fn with_pos(&self, to_add: Position) -> Self {
        let mut cloned = self.clone();
        cloned.add_pos(to_add);
        cloned
    }
    fn add_pos(&mut self, to_add: Position) {
        self.pos += to_add;
    }
    fn get_size(&self) -> Size {
		let size: Size = self.inside.iter().map(|v| v.get_size_obj()).sum();
		Size(size.0 + self.interval.0 * self.inside.len() as f32, size.1 + self.interval.1 * self.inside.len() as f32)
	}
    fn get_pos(&self) -> Position { self.pos }
}
impl<State: UIStateCl> Default for DynContainer<State> {
    fn default() -> Self { Self {
        inside: vec![],
        pos: Position(0., 0.),
        align_direction: Direction::Right,
        interval: Position(0., 0.)
}   }   }

#[derive(Clone, Debug)]
pub struct Centered<State: UIStateCl, F: PosForm<State>> {
	pub vertical_center: bool,
	pub horizontal_center: bool,
	pub inside: F,
	pub boo: PhantomData<State>
}
pub fn center<State: UIStateCl, F: PosForm<State>>(v_h: (bool, bool), inside: F) -> Centered<State, F> {
	Centered { vertical_center: v_h.0, horizontal_center: v_h.1, inside, boo: Default::default() }
}
impl<State: UIStateCl, F: PosForm<State>> Form<State> for Centered<State, F> {
	fn draw(&mut self, app: &mut App, assets: &mut Assets, gfx: &mut Graphics, plugins: &mut Plugins, state: &mut State, draw: &mut Draw) {
		let size = self.inside.get_size();
		self.inside.add_pos(
			Position(
				if self.horizontal_center { -size.0/2. } else { 0. },
				if self.vertical_center { -size.1/2. } else { 0. }
			)
		);
		self.inside.draw(app, assets, gfx, plugins, state, draw);
	}
	fn after(&mut self, app: &mut App, assets: &mut Assets, plugins: &mut Plugins, state: &mut State) {
		let size = self.inside.get_size();
		self.inside.add_pos(
			Position(
				if self.horizontal_center { -size.0/2. } else { 0. },
				if self.vertical_center { -size.1/2. } else { 0. }
			)
		);
		self.inside.after(app, assets, plugins, state);		
	}
}
impl<State: UIStateCl, F: PosForm<State>> Positionable for Centered<State, F> {
	fn add_pos(&mut self, to_add: Position) { self.inside.add_pos(to_add); }
	fn get_pos(&self) -> Position { let size = self.inside.get_size();self.inside.get_pos() - Position(size.0/2., size.1/2.) }
	fn get_rect(&self) -> Rect {
		let size = self.inside.get_size();
		Rect {
			pos: self.inside.get_pos() - Position(
				if self.horizontal_center { -size.0/2. } else { 0. },
				if self.vertical_center { -size.1/2. } else { 0. }
			),
			size: self.inside.get_size()
		}
	}
	fn get_size(&self) -> Size { self.inside.get_size() }
	fn with_pos(&self, to_add: Position) -> Self {
		let mut new = self.clone();
		new.inside = self.inside.with_pos(to_add);
		new
	}
}
