use super::defs::*;
use dyn_clone::{clone_box, DynClone};
use notan::{
    app::Event,
    draw::*,
    prelude::{App, Assets, Graphics, Plugins},
};

pub trait Form<State: UIState>: DynClone + Send {
    fn draw(
        &mut self,
        app: &mut App,
        assets: &mut Assets,
        gfx: &mut Graphics,
        plugins: &mut Plugins,
        state: &mut State,
        draw: &mut Draw,
    );
    fn after(
        &mut self,
        app: &mut App,
        assets: &mut Assets,
        plugins: &mut Plugins,
        state: &mut State,
    );
    fn on_event(
        &mut self,
        event: Event,
        app: &mut App,
        assets: &mut Assets,
        gfx: &mut Graphics,
        plugins: &mut Plugins,
        state: &mut State,
    ) {
    }
}
impl<State: UIState> Clone for Box<dyn Form<State>> {
    fn clone(&self) -> Self {
        clone_box(&**self)
    }
}
