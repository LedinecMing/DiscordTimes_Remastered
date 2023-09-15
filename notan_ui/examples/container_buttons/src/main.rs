use {
    once_cell::sync::Lazy,
    notan::{
        prelude::*,
        app::AppState,
        draw::*,
        text::TextConfig
    },
    notan_ui::{
        defs::*,
        form::Form,
        text::{Text, text},
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

static FORMS: Lazy<Mutex<DynContainer<State>>> =
    Lazy::new(|| Mutex::new(DynContainer::default()));
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
                            align_h: AlignHorizontal::Left,
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
                        state.draw.rect(rect.pos.into(), rect.size.into()).color(Color::YELLOW);
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
            .create_font(include_bytes!("UbuntuMono-RI.ttf"))
            .expect("shit happens")],
}   }
fn draw(
    app: &mut App,
    assets: &mut Assets,
    gfx: &mut Graphics,
    plugins: &mut Plugins,
    state: &mut State,
) {
	let mut draw = gfx.create_draw();
    FORMS
        .lock()
        .unwrap()
        .draw(app, assets, gfx, plugins, state, &mut draw);
    gfx.render(&draw);
}
fn update(app: &mut App, assets: &mut Assets, plugins: &mut Plugins, state: &mut State) {
    FORMS
        .lock()
        .unwrap()
        .after(app, assets, plugins, state);
}

#[notan_main]
fn main() -> Result<(), String> {
    let win = WindowConfig::new()
        .set_title("notan_ui - Container Buttons")
        .set_vsync(true)
        .set_lazy_loop(false)
        .set_high_dpi(true)
        .set_size(900, 1200)
        .set_resizable(false);
    notan::init_with(setup)
        .add_config(win)
        .add_config(DrawConfig)
        .draw(draw)
        .build()
}
