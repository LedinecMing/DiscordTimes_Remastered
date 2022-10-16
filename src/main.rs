mod lib;


use {
    std:: {
        collections::HashMap,
    },
    notan:: {
        prelude::*,
        app::AppState,
        draw::*,
        text::{TextConfig, TextExtension}
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
        parse::parse_units,
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
      Box::new(Container {
          inside: (0..10).map(move |i|
            Button::new(
                  Some(Text {
                    text: "Абоба".into(),
                    size: 10.0,
                    max_width: Some(80.),
                    ..Text::default()
                }),
                Rect {
                    pos: Position(0., 0.),
                    size: Position(100., 100.)
                },
                None,
                Some(|button, app, gfx, plugins, state| {
                    button.inside = Some(Text::<State> {
                        text: "Клик!".into(),
                        size: 10.0,
                        max_width: Some(80.),
                        ..Text::default()
                    })
                })
            )
          ).collect(),
          interval: Position(10., 10.),
          align_direction: Direction::Right,
          ..Container::default()
      })
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
    state.draw = gfx.create_draw();
    state.mut_draw().clear(Color::WHITE);
    unsafe {
        forms.iter_mut().for_each(|form: &mut Box<dyn Form<State>>| form.draw(app, gfx, plugins, state));
    }
    gfx.render(state.draw());
    unsafe {
        forms.iter_mut().for_each(|form: &mut Box<dyn Form<State>>| form.after(app, gfx, plugins, state))
    }
}
#[notan_main]
fn main() -> Result<(), String> {
    dbg!(parse_units());
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
