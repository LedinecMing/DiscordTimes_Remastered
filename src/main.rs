use {
    std:: {
        collections::HashMap,
        mem::size_of,
    },
    notan:: {
        prelude::*,
        app::AppState,
        draw::*,
        text::{TextConfig, TextExtension}
    },
    notan_ui::{
        form::Form,
        forms::Data,
        defs::*,
        text::*,
        containers::{SingleContainer, SliderContainer, DynContainer},
        wrappers::{Button, Checkbox},
        rect::*
    },
    once_cell::sync::Lazy
};

enum Menu {
    Main = 0,
    Start = 1,
    Load = 2,
    Settings = 3,
    Authors = 4
}

#[derive(Clone)]
struct State {
    pub fonts: Vec<Font>,
    pub draw: Draw,
    pub menu_id: usize
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


fn menu_button(text: impl Into<String>,
               on_draw: fn(&mut SingleContainer<State, Text<State>>, &mut App, &mut Graphics, &mut Plugins, &mut State),
               if_clicked: fn(&mut Button<State, SingleContainer<State, Text<State>>>, &mut App, &mut Graphics, &mut Plugins, &mut State))
    -> Box<Button<State, SingleContainer<State, Text<State>>>> {
    Box::new(Button {
        inside: Some(
            SingleContainer {
                inside: Some(Text {
                    text: text.into(),
                    align_h: AlignHorizontal::Center,
                    align_v: AlignVertical::Top,
                    pos: Position(0., 0.),
                    size: 30.0,
                    color: Color::WHITE,
                    ..Text::default()
                }),
                on_draw: Some(on_draw),
                ..SingleContainer::default()
            }),
        rect: Rect {
            pos: Position(0., 0.),
            size: Size(300., 230.)
        },
        if_clicked: Some(if_clicked),
        ..Button::default()
    })
}

type FormsInside = dyn Form<State>;
static mut forms: Lazy<HashMap<usize, Vec<SingleContainer<State, DynContainer<State>>>>> = Lazy::new(|| {
    let mut hashmap = HashMap::new();
    let draw_back: for<'a, 'b, 'c, 'd, 'e> fn(&'a mut SingleContainer<State, Text<State>>, &'b mut App, &'c mut Graphics, &'d mut Plugins, &'e mut State) =
        |container, app, gfx, plugins, state: &mut State| {
            get_mut::<State, Draw>(state)
            .rect((container.pos - Position(container.get_size().0/2., 0.)).into(),
                  container.get_size().into())
            .color(Color::from_hex(0x033121ff));
    };
    hashmap.insert(Menu::Main as usize, vec![
        SingleContainer {
                inside: Some(DynContainer {
                    inside: vec![
                        Box::new(Text {
                            text: "Времена Раздора".into(),
                            align_h: AlignHorizontal::Center,
                            align_v: AlignVertical::Top,
                            size: 70.0,
                            max_width: None,
                            color: Color::BLACK,
                            ..Text::default()
                        }),
                        menu_button("Начать", draw_back,
                            |container, app, gfx, plugins, state: &mut State| {
                                state.menu_id = Menu::Start as usize;
                            }
                        ),
                        menu_button("Загрузка", draw_back,
                            |container, app, gfx, plugins, state: &mut State| {
                                state.menu_id = Menu::Load as usize;
                            }
                        ),
                        menu_button("Настройки", draw_back,
                            |container, app, gfx, plugins, state: &mut State| {
                                state.menu_id = Menu::Settings as usize;
                            }
                        ),
                        menu_button("Авторы", draw_back,
                            |container, app, gfx, plugins, state: &mut State| {
                                state.menu_id = Menu::Authors as usize;
                            }
                        ),
                        menu_button("Выход", draw_back,
                            |container, app, gfx, plugins, state: &mut State| {
                                app.exit()
                            }
                        ),
                    ],
                    pos: Position(500., 0.),
                    align_direction: Direction::Bottom,
                    interval: Position(0., 20.)
                }),
                on_draw: Some(|container, app, gfx, plugins, state: &mut State| {
                    get_mut::<State, Draw>(state)
                        .rect((0., 0.), (1000., 1200.))
                        .color(Color::ORANGE);
                }),
                ..SingleContainer::default()
            }
    ]);
    hashmap.insert(Menu::Settings as usize, vec![SingleContainer {
        inside: Some(DynContainer {
                inside: vec![
                    Box::new(Checkbox {
                        inside: Some(Text {
                            text: "+".into(),
                            align_h: AlignHorizontal::Left,
                            align_v: AlignVertical::Top,
                            size: 50.0,
                            color: Color::ORANGE,
                            ..Default::default()
                        }),
                        rect: Rect { pos: Position(0., 0.), size: Size(50., 50.) },
                        checked: true,
                        on_draw: Some(|container, app, gfx, plugins, state: &mut State| {
                            get_mut::<State, Draw>(state)
                            .rect((container.pos - Position(container.get_size().0/2., container.get_pos().1)).into(),
                                  container.get_size().into())
                            .color(Color::from_hex(0x033121ff));
                        }),
                        pos: Position(0., 0.),
                        ..Default::default()
                    })
                ],
                pos: Position(0., 0.),
                align_direction: Direction::Bottom,
                interval: Position(0., 50.)
        }),
        on_draw: Some(|container, app, gfx, plugins, state: &mut State| {
            get_mut::<State, Draw>(state)
                .rect((0., 0.), (1000., 1200.))
                .color(Color::ORANGE);
        }),
        after_draw: None,
        pos: Position(0., 0.)
    }]);
    hashmap
});


fn setup(gfx: &mut Graphics) -> State {
    State {
        fonts: vec![gfx
            .create_font(include_bytes!("UbuntuMono-RI.ttf"))
            .expect("shit happens")],
        draw: gfx.create_draw(),
        menu_id: 0
}   }
fn draw(app: &mut App, gfx: &mut Graphics, plugins: &mut Plugins, state: &mut State) {
    state.draw = gfx.create_draw();
    get_mut::<State, Draw>(state).clear(Color::WHITE);
    unsafe {
        forms.get_mut(&state.menu_id).unwrap()
            .iter_mut().for_each(|form| form.draw(app, gfx, plugins, state));
        gfx.render(get_mut::<State, Draw>(state));
        forms.get_mut(&state.menu_id).unwrap()
            .iter_mut().for_each(|form| form.after(app, gfx, plugins, state));
}   }
#[notan_main]
fn main() -> Result<(), String> {
    dbg!(size_of::<SingleContainer<State, SliderContainer<State, TextChain<State>, SingleContainer<State, Data<State, &str, f32>>>>>());
    let win = WindowConfig::new()
        .title("Discord Times: Remastered")
        .vsync(true)
        .lazy_loop(true)
        .high_dpi(true)
        .size(1000,1200);
    notan::init_with(setup)
        .add_config(win)
        .add_config(TextConfig)
        .add_config(DrawConfig)
        .draw(draw)
        .build()
}
