use std::marker::PhantomData;

use notan::prelude::{Color, Texture};

use {
    std::fmt::Debug,
    notan::{
        prelude::{AppState, App, Assets, Graphics, Plugins},
        draw::*
    },
    super::{
        form::Form,
        rect::*
    },
    derive_builder::UninitializedFieldError,
    dyn_clone::clone_box
};

pub trait UIState = AppState + Access<Draw> + Access<Vec<Font>> + Debug;
pub trait Access<T> {
    fn get_mut(&mut self) -> &mut T;
    fn get(&self) -> &T;
}
pub fn get_mut<T, K>(from: &mut T) -> &mut K where T: Access<K> {
    Access::<K>::get_mut(from)
}
pub fn get<T, K>(from: &T) -> &K where T: Access<K> {
    Access::<K>::get(from)
}

pub trait Positionable {
    fn with_pos(&self, to_add: Position) -> Self;
    fn add_pos(&mut self, to_add: Position);
    fn get_size(&self) -> Size;
    fn get_pos(&self) -> Position;
    fn get_rect(&self) -> Rect {
        Rect { pos: self.get_pos(), size: self.get_size() }
}   }

pub trait PartPositional {
    fn add_pos_obj(&mut self, to_add: Position);
    fn get_size_obj(&self) -> Size;
    fn get_pos_obj(&self) -> Position;
}
impl<T: Positionable> PartPositional for T {
    fn add_pos_obj(&mut self, to_add: Position) {
        Positionable::add_pos(self, to_add);
    }
    fn get_size_obj(&self) -> Size { Positionable::get_size(self) }
    fn get_pos_obj(&self) -> Position { Positionable::get_pos(self) }
}
pub trait ObjPosForm<State: UIStateCl>: Form<State> + PartPositional + Send + Debug {}
impl<State: UIStateCl, T: Form<State> + PartPositional + Debug> ObjPosForm<State> for T {}

impl<State: UIStateCl> Clone for Box<dyn ObjPosForm<State>> {
    fn clone(&self) -> Self {
        clone_box(&**self)
}   }

pub trait PosForm<State: UIState>: Form<State> + Positionable + Clone + Debug {
    type State = State;
}
impl<T, State: UIState> PosForm<State> for T where T: Form<State> + Positionable + Clone + Debug {}
pub trait UIStateCl = UIState + Clone + Send + Sync;

pub type DrawFunction<State: UIStateCl, Form: PosForm<State>> = fn(&mut Form, &mut App, &mut Assets, &mut Graphics, &mut Plugins, &mut State, &mut Draw);
pub type UpdateFunction<State: UIStateCl, Form: PosForm<State>> = fn(&mut Form, &mut App, &mut Assets, &mut Plugins, &mut State);

#[derive(Copy, Clone, PartialEq, Debug)]
pub enum AlignHorizontal {
    Left,
    Center,
    Right
}
#[derive(Copy, Clone, PartialEq, Debug)]
pub enum AlignVertical {
    Top,
    Center,
    Bottom
}
#[derive(Copy, Clone, PartialEq, Debug)]
pub enum Direction {
    Right,
    Left,
    Top,
    Bottom
}

#[derive(Debug)]
pub struct StructBuildError(pub String);
impl From<UninitializedFieldError> for StructBuildError {
    fn from(ufe: UninitializedFieldError) -> StructBuildError { StructBuildError(ufe.to_string()) }
}
impl From<StructBuildError> for String {
    fn from(value: StructBuildError) -> Self {
        value.0
    }
}

pub trait TryToWith<State, Into>: Clone + Send {
	fn with_to(&self, _state: &State) -> Into;
	fn try_to(&self) -> Option<Into>;
}
macro_rules! impl_trytowith {
	($typ:tt, $typ1:tt) => {
		impl<State> TryToWith<State, $typ1> for $typ where $typ: TryInto< $typ1 > + Into<$typ1> {
			fn with_to(&self, _state: &State) -> $typ1 { <Self as Into<$typ1>>::into(self) }
			fn try_to(&self) -> Option<$typ1> { <Self as TryInto<$typ1>>::try_into(self).ok() }
		}
	}
}
macro_rules! impl_trytome {
	($typ:tt) => {
		impl<State> TryToWith<State, $typ > for $typ {
			fn with_to(&self, _state: &State) -> $typ { *self }
			fn try_to(&self) -> Option<$typ> { Some(*self) }
		}
		impl<State> TryToWith<State, Option<$typ> > for $typ {
			fn with_to(&self, _state: &State) -> Option<$typ> { Some(*self) }
			fn try_to(&self) -> Option<Option<$typ>> { Some(Some(*self)) }
		}
	}
}
macro_rules! impl_trytome_owned {
	($typ:tt) => {
		impl<State> TryToWith<State, $typ > for $typ {
			fn with_to(&self, _state: &State) -> $typ { self.into() }
			fn try_to(&self) -> Option<$typ> { Some(self.into()) }
		}
		impl<State> TryToWith<State, Option<$typ> > for $typ {
			fn with_to(&self, _state: &State) -> Option<$typ> { Some(self.into()) }
			fn try_to(&self) -> Option<Option<$typ>> { Some(Some(self.into())) }
		}
	}
}
impl<Into, F: Fn(&State) -> Into + Clone + Send, State> TryToWith<State, Into> for F {
	fn with_to(&self, _state: &State) -> Into {
		(self)(_state)
	}
	fn try_to(&self) -> Option<Into> { None	}
}
type Fuckingstr = &'static str;
impl_trytome! { usize }
impl_trytome! { Fuckingstr }
impl_trytowith! { Fuckingstr, String }
impl_trytome_owned!{ String }
pub trait ToTexture<'a, State: 'a> = TryToWith<State, &'a Texture>;
#[derive(Debug, Clone)]
pub enum BackSize {
	Size(Size),
	Max
}
#[derive(Clone)]
pub enum Back<'a, State, Tex: ToTexture<'a, State>> {
	Image(Tex),
	Color(Color),
	_10маленькихНегритят(PhantomData<&'a State>)
}
impl<'a, State, Tex: ToTexture<'a, State>> Debug for Back<'a, State, Tex> {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			Self::Color(c) => f.debug_struct("Back::Color").field("color", c).finish(),
			Self::Image(i) => f.debug_struct("Back::Image").finish(),
			_ => f.debug_struct("oh").finish()
		}
	}
}
pub struct Style<'a, State, Tex: ToTexture<'a, State>> {
	pub background: (Back<'a, State, Tex>, BackSize),
	pub centered: (bool, bool),
	_10маленьких_негритят: PhantomData<&'a State>,
}
impl<'a, State, Tex: ToTexture<'a, State>> Style<'a, State, Tex> {
	fn new(background: (Back<'a, State, Tex>, BackSize), centered: (bool, bool)) -> Self {
		Self { background, centered, _10маленьких_негритят: PhantomData}
	}
}
impl<'a, State, Tex: ToTexture<'a, State>> Default for Style<'a, State, Tex> {
	fn default() -> Self {
		Self {
			background: (Back::Color(Color::from_rgba(0.,0.,0.,1.)), BackSize::Max),
			centered: (false, false),
			_10маленьких_негритят: PhantomData
		}
	}
}
pub trait Styled<'a, State: UIStateCl, Tex: ToTexture<'a, State>> {
	type Output: PosForm<State> + 'a;
	fn style(self, style: Style<'a, State, Tex>) -> Self::Output;
}
