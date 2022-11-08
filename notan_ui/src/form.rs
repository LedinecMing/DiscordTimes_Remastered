use {
    super::{
        defs::*,
        rect::*
    },
    notan::{
        prelude::{AppState, Color, Graphics, Plugins, App},
        app::{Event, Texture},
        draw::*
    },
    dyn_clone::{clone_box, DynClone}
};

pub trait Form<State: UIState>: DynClone {
    fn draw(&mut self, app: &mut App, gfx: &mut Graphics, plugins: &mut Plugins, state:&mut State);
    fn after(&mut self, app: &mut App, gfx: &mut Graphics, plugins: &mut Plugins, state: &mut State);
    fn on_event(&mut self, event: Event, app: &mut App, gfx: &mut Graphics, plugins: &mut Plugins, state: &mut State) {}
}
impl<State: UIState> Clone for Box<dyn Form<State>> {
    fn clone(&self) -> Self {
        clone_box(&**self)
}   }