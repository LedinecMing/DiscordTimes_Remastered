use {
    notan::{
        prelude::{AppState, Color, Graphics, Plugins, App},
        app::{Event, Texture},
        draw::*
    },
    super::{
        form::Form,
        rect::*
    },
    dyn_clone::clone_box
};

pub trait UIState = AppState + Access<Draw> + Access<Vec<Font>>;
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
pub trait ObjPosForm<State: UIStateCl>: Form<State> + PartPositional {}
impl<State: UIStateCl, T: Form<State> + PartPositional> ObjPosForm<State> for T {}

impl<State: UIStateCl> Clone for Box<dyn ObjPosForm<State>> {
    fn clone(&self) -> Self {
        clone_box(&**self)
}   }

pub trait PosForm<State: UIState> = Form<State> + Positionable + Clone;
pub trait UIStateCl = UIState + Clone;

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