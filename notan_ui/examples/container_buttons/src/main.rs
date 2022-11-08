use {
    std::{
        iter::Extend,
    },
    once_cell::sync::Lazy,
    notan::{
        prelude::*,
        app::AppState,
        draw::*,
        text::TextConfig
    },
    notan_ui::{
        wrappers::Button,
        containers::{Container, SingleContainer},
        rect::*
}   };

#[derive(Clone)]
struct State {
    pub fonts: Vec<Font>,
    pub draw: Draw
}
impl AppState for State {}
impl Access<Vec<Font>> for State {
    fn get_mut(&mut self) -> &mut Vec<Font> { &mut self.fonts }
    fn get(&self) -> &Vec<Font> { &self.fonts }
}
impl Access<Draw> for State {
    fn get_mut(&mut self) -> &mut Draw { &mut self.draw }
    fn get(&self) -> &Draw { &self.draw }
}


static mut forms: Lazy<Vec<Box<dyn Form<State>>>> = Lazy::new(|| {
    vec![
        Box::new(Container {
            inside: (0..10).map(move |i|
                SingleContainer {
                    inside: Some(Button::new(
                        Some(Text {
                            text: "Click!".into(),
                            size: 20.0,
                            max_width: Some(100.),
                            align_h: AlignHorizontal::Center,
                            align_v: AlignVertical::Top,
                            ..Text::default()
                        }),
                        Rect {
                            pos: Position(0., 0.),
                            size: Size(100., 100.)
                        },
                        None,
                        Some(|button, app, gfx, plugins, state: &mut State| {
                            button.inside.as_mut().unwrap().text = "Clicked!".into();
                        }))),
                    on_draw: Some(|container, app, gfx, plugins, state: &mut State| {
                        let rect = container.inside.as_ref().unwrap().rect;
                        Access::<Draw>::get_mut(state).rect(rect.pos.into(), rect.size.into()).color(Color::YELLOW);
                    }),
                    after_draw: None,
                    pos: Position(0., 0.)
                }
            ).collect(),
            interval: Position(10., 10.),
            align_direction: Direction::Bottom,
            ..Container::default()
        })
    ]
});


fn setup(gfx: &mut Graphics) -> State {
    State {
        fonts: vec![gfx
            .create_font(include_bytes!("../../UbuntuMono-RI.ttf"))
            .expect("shit happens")],
        draw: gfx.create_draw()
}   }
fn draw(app: &mut App, gfx: &mut Graphics, plugins: &mut Plugins, state: &mut State) {
    state.draw = gfx.create_draw();
    get_mut::<State, Draw>(state).clear(Color::WHITE);
    unsafe {
        forms.iter_mut().for_each(|form: &mut Box<dyn Form<State>>| form.draw(app, gfx, plugins, state));
        gfx.render(get::<State, Draw>(state));
        forms.iter_mut().for_each(|form: &mut Box<dyn Form<State>>| form.after(app, gfx, plugins, state));
}   }

#[notan_main]
fn main() -> Result<(), String> {
    let win = WindowConfig::new()
        .title("notan_ui - Container Buttons")
        .vsync(true)
        .lazy_loop(true)
        .high_dpi(true)
        .size(900, 1200);
    notan::init_with(setup)
        .add_config(win)
        .add_config(TextConfig)
        .add_config(DrawConfig)
        .draw(draw)
        .build()
}
