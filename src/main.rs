mod lib;


use {
    std:: {
        collections::HashMap,
    },
    notan:: {
        prelude::*,
        app::AppState,
        draw::*,
        text::*
    },
    notan_ui::{
        forms::{self, *},
        rect::*
    },
    once_cell::sync::Lazy,
    lib:: {
        units::units::*,
        units::unit::{Unit},
        battle::army::{Army, ArmyStats},
        battle::troop::Troop,
        time::time::Time,
        mutrc::MutRc,
        map::{
            map::{GameMap, MAP_SIZE},
            tile::Tile,
            object::MapObject
}   }   };

#[derive(Clone)]
struct State {
    pub fonts: Vec<Font>,
    pub draw: Draw,
    pub value: i32
}
static mut forms: Lazy<Vec<Box<dyn Form<State>>>> = Lazy::new(||
  vec![
      Box::new(forms::Text::new("Говно залупа пенис хер!",
                                 0, AlignHorizontal::Left, AlignVertical::Bottom, Position(400., 30.), 10.0, Color::WHITE)),
      Box::new(forms::Button::new(
          Some(forms::Text::new("Я люблю кушать кексики", 0, AlignHorizontal::Center, AlignVertical::Center, Position(10., 10.), 10., Color::RED)),
                Rect { pos: Position(400., 30.), size: Position(200., 30.) },
          Some(|button, gfx, plugins, state, app| {
              let mut state: &mut State = state;
              state.value = 2;
            }),
          Some(|button, gfx, plugins, state, app| {
              button.rect.pos.0 += 10.;
              state.value = 3;
          })
      ))
  ]
);


impl AppState for State {}

impl UIState for State {
    fn mut_fonts(&mut self) -> &mut Vec<Font> { &mut self.fonts }
    fn fonts(&self) -> &Vec<Font> { &self.fonts }
    fn mut_draw(&mut self) -> &mut Draw { &mut self.draw }
    fn draw(&self) -> &Draw { &self.draw }
}
fn setup(gfx: &mut Graphics) -> State {
    State {
        fonts: vec![gfx
            .create_font(include_bytes!("UbuntuMono-RI.ttf"))
            .expect("shit happens")],
        draw: gfx.create_draw(),
        value: 0
}   }
fn draw(app: &mut App, gfx: &mut Graphics, plugins: &mut Plugins, state: &mut State) {
    state.mut_draw().clear(Color::BLACK);
    unsafe {
        forms.iter_mut().for_each(|form: &mut Box<dyn Form<State>>| form.draw(gfx, plugins, state, app))
    }
    if state.value != 0 {
        forms::Text::new(if state.value == 2 { "Ты навел" } else if state.value == 3 { "Ты нажал" } else { "Че ты наделал??" }, 0, AlignHorizontal::Left, AlignVertical::Top, Position(500., 300.), 30., Color::MAGENTA).draw(gfx, plugins, state, app);
    }
    gfx.render(state.draw());
    unsafe {
        forms.iter_mut().for_each(|form: &mut Box<dyn Form<State>>| form.after(gfx, plugins, state, app))
    }
}
#[notan_main]
fn main() -> Result<(), String> {
    let win = WindowConfig::new()
        .vsync(true)
        .lazy_loop(true)
        .high_dpi(false);

    notan::init_with(setup)
        .add_config(win)
        .add_config(TextConfig)
        .add_config(DrawConfig)
        .draw(draw)
        .build()
}
