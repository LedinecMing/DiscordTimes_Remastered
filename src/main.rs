mod lib;

use crate::lib::battle::army::{Army, ArmyStats, TroopType, MAX_TROOPS};
use crate::lib::battle::troop::Troop;
use crate::lib::mutrc::SendMut;
use crate::lib::parse::{parse_locale, parse_settings, parse_units, Locale};
use crate::lib::units::unit::{Unit, UnitPos};
use notan_ui::containers::StraightDynContainer;
use rand::prelude::ThreadRng;
use rand::thread_rng;
use std::cmp::min;
use std::fmt::{Debug, Formatter};
use {
    lib::{
        battle::battlefield::*,
        map::{map::*, tile::Tile},
        time::time::Time,
    },
    notan::{
        app::AppState,
        draw::*,
        prelude::*,
        text::{TextConfig, TextExtension},
    },
    notan_ui::{
        containers::{Container, DynContainer, SingleContainer, SliderContainer},
        defs::*,
        form::Form,
        forms::{Data, Drawing},
        rect::*,
        text::*,
        wrappers::{Button, Checkbox},
    },
    once_cell::sync::Lazy,
    rand::Rng,
    std::{
        collections::HashMap,
        fmt::Display,
        fs::{read, read_dir},
        mem::size_of,
    },
    tracing_mutex::stdsync::TracingMutex as Mutex
};

#[derive(Clone, Debug)]
pub enum Value {
    Num(i64),
    Str(String),
}

#[derive(Clone, Debug)]
pub struct State {
    pub fonts: Vec<Font>,
    pub frame: usize,
    pub draw: Draw,
    pub gamemap: GameMap,
    pub units: HashMap<usize, Unit>,
    pub assets: HashMap<&'static str, HashMap<String, Asset<Texture>>>,
    pub menu_data: HashMap<&'static str, Value>,
    pub menu_id: usize,
    pub battle: BattleInfo,
}
impl AppState for State {}
impl Access<Vec<Font>> for State {
    fn get_mut(&mut self) -> &mut Vec<Font> {
        &mut self.fonts
    }
    fn get(&self) -> &Vec<Font> {
        &self.fonts
    }
}
impl Access<Draw> for State {
    fn get_mut(&mut self) -> &mut Draw {
        &mut self.draw
    }
    fn get(&self) -> &Draw {
        &self.draw
    }
}

fn asset_path(path: &str) -> String {
    let base = "./assets";
    format!("{}/{}", base, path)
}

fn get_menu_value_str(state: &mut State, id: &'static str) -> Option<String> {
    match state.menu_data.get(id) {
        Some(Value::Str(string)) => Some(string.clone()),
        _ => None,
    }
}
fn get_menu_value_num(state: &mut State, id: &'static str) -> Option<i64> {
    match state.menu_data.get(id) {
        Some(Value::Num(num)) => Some(*num),
        _ => None,
    }
}

fn set_menu_value_str(state: &mut State, id: &'static str, new: String) {
    match state.menu_data.get_mut(id) {
        Some(value) => match value {
            Value::Str(string) => {
                *string = new;
            }
            _ => {}
        },
        _ => {
            state.menu_data.insert(id, Value::Str(new));
        }
    }
}
fn set_menu_value_num(state: &mut State, id: &'static str, new: i64) {
    match state.menu_data.get_mut(id) {
        Some(value) => match value {
            Value::Num(num) => {
                *num = new;
            }
            _ => {}
        },
        _ => {
            state.menu_data.insert(id, Value::Num(new));
        }
    }
}

fn menu_button(
    text: impl Into<String>,
    on_draw: fn(
        &mut SingleContainer<State, Text<State>>,
        &mut App,
        &mut Assets,
        &mut Graphics,
        &mut Plugins,
        &mut State,
    ),
    if_clicked: fn(
        &mut Button<State, SingleContainer<State, Text<State>>>,
        &mut App,
        &mut Assets,
        &mut Plugins,
        &mut State,
    ),
) -> Box<Button<State, SingleContainer<State, Text<State>>>> {
    Box::new(Button {
        inside: Some(SingleContainer {
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
            size: Size(300., 130.),
        },
        if_clicked: Some(if_clicked),
        ..Button::default()
    })
}

type FormsInside = dyn Form<State>;
static forms: Lazy<Mutex<HashMap<usize, Vec<SingleContainer<State, DynContainer<State>>>>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));
static LOCALE: Lazy<Mutex<Locale>> = Lazy::new(|| Mutex::new(Locale::new()));
static MONITOR_SIZE: Lazy<Mutex<(f32, f32)>> = Lazy::new(|| Mutex::new((0., 0.)));
enum Menu {
    Main = 0,
    Start = 1,
    Load = 2,
    Settings = 3,
    Authors = 4,
    UnitView = 5,
    JustRectangle = 6,
    Battle = 7,
}

fn gen_forms(gfx: &mut Graphics) {
    let draw_back: for<'a, 'b, 'c, 'd, 'e, 'f> fn(
        &'a mut SingleContainer<State, Text<State>>,
        &'b mut App,
        &'f mut Assets,
        &'c mut Graphics,
        &'d mut Plugins,
        &'e mut State,
    ) = |container, app, assets, gfx, plugins, state: &mut State| {
        get_mut::<State, Draw>(state)
            .rect(
                (container.pos - Position(container.get_size().0 / 2., 0.)).into(),
                container.get_size().into(),
            )
            .color(Color::from_hex(0x033121ff));
    };
    let size = gfx.device.size();
    *MONITOR_SIZE.lock().unwrap() = (size.0 as f32, size.1 as f32);
    let mut hashmap = forms.lock().unwrap();
    let locale = LOCALE.lock().unwrap();
    let max_troops = *MAX_TROOPS.lock().unwrap();
    let half_troops = max_troops / 2;
    hashmap.insert(
        Menu::Main as usize,
        vec![SingleContainer {
            inside: Some(DynContainer {
                inside: vec![
                    Box::new(Text {
                        text: locale.get("menu_game_name"),
                        align_h: AlignHorizontal::Center,
                        align_v: AlignVertical::Top,
                        size: 70.0,
                        max_width: None,
                        color: Color::BLACK,
                        ..Text::default()
                    }),
                    menu_button(
                        locale.get("menu_start_title"),
                        draw_back,
                        |container, app, assets, plugins, state: &mut State| {
                            state.menu_id = Menu::Start as usize;
                        },
                    ),
                    menu_button(
                        locale.get("menu_battle_title"),
                        draw_back,
                        |container, app, assets, plugins, state: &mut State| {
                            state.menu_id = Menu::Battle as usize;
                        },
                    ),
                    menu_button(
                        locale.get("menu_load_title"),
                        draw_back,
                        |container, app, assets, plugins, state: &mut State| {
                            state.menu_id = Menu::Load as usize;
                        },
                    ),
                    menu_button(
                        locale.get("menu_settings_title"),
                        draw_back,
                        |container, app, assets, plugins, state: &mut State| {
                            state.menu_id = Menu::Settings as usize;
                        },
                    ),
                    menu_button(
                        locale.get("menu_authors_title"),
                        draw_back,
                        |container, app, assets, plugins, state: &mut State| {
                            state.menu_id = Menu::Authors as usize;
                        },
                    ),
                    menu_button(
                        locale.get("menu_unitview_title"),
                        draw_back,
                        |container, app, assets, plugins, state: &mut State| {
                            state.menu_id = Menu::UnitView as usize;
                        },
                    ),
                    menu_button(
                        locale.get("menu_justrectangle_title"),
                        draw_back,
                        |container, app, assets, plugins, state: &mut State| {
                            state.menu_id = Menu::JustRectangle as usize;
                        },
                    ),
                    menu_button(
                        locale.get("menu_exit_title"),
                        draw_back,
                        |container, app, assets, plugins, state: &mut State| app.exit(),
                    ),
                ],
                pos: Position(MONITOR_SIZE.lock().unwrap().0 / 2., 0.),
                align_direction: Direction::Bottom,
                interval: Position(0., 20.),
            }),
            on_draw: Some(|container, app, assets, gfx, plugins, state: &mut State| {
                get_mut::<State, Draw>(state)
                    .rect((0., 0.), *MONITOR_SIZE.lock().unwrap())
                    .color(Color::ORANGE);
            }),
            ..SingleContainer::default()
        }],
    );

    hashmap.insert(
        Menu::Settings as usize,
        vec![SingleContainer {
            inside: Some(DynContainer {
                inside: vec![Box::new(DynContainer {
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
                            rect: Rect {
                                pos: Position(0., 0.),
                                size: Size(50., 50.),
                            },
                            checked: false,
                            on_draw: Some(
                                |container, app, assets, gfx, plugins, state: &mut State| {
                                    get_mut::<State, Draw>(state)
                                        .rect(
                                            (container.pos
                                                - Position(
                                                    container.get_size().0 / 2.,
                                                    container.get_pos().1,
                                                ))
                                            .into(),
                                            container.get_size().into(),
                                        )
                                        .color(Color::from_hex(0x033121ff));
                                    set_menu_value_num(
                                        state,
                                        "setting_rectangle_rotate_mode",
                                        container.checked as i64,
                                    );
                                },
                            ),
                            pos: Position(20., 0.),
                            ..Default::default()
                        }),
                        Box::new(Text {
                            text: locale.get("settings_rect_rotate_type"),
                            font: FontId(0),
                            align_h: AlignHorizontal::Left,
                            align_v: AlignVertical::Top,
                            pos: Position(20., 0.),
                            size: 20.0,
                            rect_size: None,
                            max_width: None,
                            color: Color::BLACK,
                            boo: Default::default(),
                        }),
                    ],
                    pos: Position(0., 0.),
                    align_direction: Direction::Right,
                    interval: Position(50., 0.),
                })],
                pos: Position(0., 0.),
                align_direction: Direction::Bottom,
                interval: Position(0., 0.),
            }),
            on_draw: Some(|container, app, assets, gfx, plugins, state: &mut State| {
                get_mut::<State, Draw>(state)
                    .rect((0., 0.), *MONITOR_SIZE.lock().unwrap())
                    .color(Color::ORANGE);
            }),
            after_draw: Some(|container, app, assets, plugins, state: &mut State| {
                if app.keyboard.is_down(KeyCode::Escape) {
                    state.menu_id = Menu::Main as usize;
                }
            }),
            pos: Position(0., 0.),
        }],
    );
    hashmap.insert(
        Menu::JustRectangle as usize,
        vec![SingleContainer {
            inside: Some(DynContainer {
                inside: vec![Box::new(SingleContainer {
                    inside: None::<Drawing<State>>,
                    on_draw: Some(
                        |drawing: &mut SingleContainer<State, _>,
                         app,
                         assets,
                         gfx,
                         plugins,
                         state: &mut State| {
                            let deg = get_menu_value_num(state, "map_rotation_deg").unwrap_or(0)
                                as f32
                                / 4.0;
                            let mut center = drawing.pos + Position(25. * 8., 25. * 8.);
                            for i in 0..MAP_SIZE {
                                for j in 0..MAP_SIZE {
                                    let color = Color::RED;
                                    let pos = drawing.pos + Position(i as f32 * 8., j as f32 * 8.);
                                    if get_menu_value_num(state, "setting_rectangle_rotate_mode")
                                        .unwrap_or(0)
                                        == 1
                                    {
                                        center = pos - Position(4., 4.);
                                    }
                                    get_mut::<State, Draw>(state)
                                        .rect(pos.into(), (8., 8.))
                                        .rotate_degrees_from(center.into(), deg)
                                        .color(color);
                                }
                            }
                        },
                    ),
                    after_draw: Some(|container, app, assets, plugins, state: &mut State| {
                        if app.keyboard.is_down(KeyCode::W) {
                            container.add_pos(Position(0., -8.));
                        }
                        if app.keyboard.is_down(KeyCode::A) {
                            container.add_pos(Position(-8., 0.));
                        }
                        if app.keyboard.is_down(KeyCode::S) {
                            container.add_pos(Position(0., 8.));
                        }
                        if app.keyboard.is_down(KeyCode::D) {
                            container.add_pos(Position(8., 0.));
                        }
                        if app.keyboard.is_down(KeyCode::Q) {
                            if let Some(num) = get_menu_value_num(state, "map_rotation_deg") {
                                set_menu_value_num(state, "map_rotation_deg", num - 1)
                            } else {
                                set_menu_value_num(state, "map_rotation_deg", 0);
                            }
                        }
                        if app.keyboard.is_down(KeyCode::E) {
                            if let Some(num) = get_menu_value_num(state, "map_rotation_deg") {
                                set_menu_value_num(state, "map_rotation_deg", num + 1)
                            } else {
                                set_menu_value_num(state, "map_rotation_deg", 0);
                            }
                        }
                        if app.keyboard.is_down(KeyCode::Escape) {
                            state.menu_id = Menu::Main as usize;
                        }
                    }),
                    pos: Position(0., 0.),
                })],
                pos: Position(0., 0.),
                align_direction: Direction::Bottom,
                interval: Position(0., 50.),
            }),
            on_draw: Some(|container, app, assets, gfx, plugins, state: &mut State| {
                get_mut::<State, Draw>(state)
                    .rect((0., 0.), *MONITOR_SIZE.lock().unwrap())
                    .color(Color::ORANGE);
            }),
            after_draw: None,
            pos: Position(0., 0.),
        }],
    );
    let nav_button_rect = Rect {
        pos: Position(0., 0.),
        size: Size(70., 100.),
    };
    let nav_button_text = |text: String| {
        Some(Text {
            text,
            font: FontId(0),
            align_h: AlignHorizontal::Left,
            align_v: AlignVertical::Bottom,
            pos: Position(0., 100.),
            size: 10.,
            rect_size: None,
            max_width: None,
            color: Color::BLACK,
            boo: std::marker::PhantomData,
        })
    };
    hashmap.insert(
        Menu::Start as usize,
        vec![SingleContainer {
            inside: Some(DynContainer {
                inside: vec![Box::new(SingleContainer {
                    inside: None::<Drawing<State>>,
                    on_draw: Some(
                        |drawing: &mut SingleContainer<State, _>,
                         app,
                         assets,
                         gfx,
                         plugins,
                         state: &mut State| {
                            let mut draw = get_mut::<State, Draw>(state);
                            gfx.render(draw);
                            let mut draw = gfx.create_draw();
                            for i in 0..MAP_SIZE {
                                for j in 0..MAP_SIZE {
                                    let asset = state
                                        .assets
                                        .get("assets/Terrain")
                                        .unwrap()
                                        .get(&format!(
                                            "{}",
                                            TILES[state.gamemap.tilemap[i][j]].sprite()
                                        ))
                                        .unwrap();
                                    let texture = asset.lock().unwrap();
                                    let pos = Position(i as f32 * 53., j as f32 * 50.);
                                    draw.image(&texture)
                                        .position(pos.0, pos.1)
                                        .translate(53., 50.);
                                }
                            }
                            state.draw = draw;
                        },
                    ),
                    after_draw: Some(|container, app, assets, plugins, state: &mut State| {
                        if app.keyboard.is_down(KeyCode::Escape) {
                            state.menu_id = Menu::Main as usize;
                        }
                    }),
                    pos: Position(0., 0.),
                })],
                pos: Position(0., 0.),
                align_direction: Direction::Bottom,
                interval: Position(0., 0.),
            }),
            on_draw: Some(|container, app, assets, gfx, plugins, state: &mut State| {
                get_mut::<State, Draw>(state)
                    .rect((0., 0.), *MONITOR_SIZE.lock().unwrap())
                    .color(Color::ORANGE);
            }),
            after_draw: None,
            pos: Position(0., 0.),
        }],
    );
    hashmap.insert(
        Menu::UnitView as usize,
        vec![SingleContainer {
            inside: Some(DynContainer {
                inside: vec![Box::new(StraightDynContainer {
                    inside: vec![
                        Box::new(Drawing {
                            pos: Position(100., 0.),
                            to_draw: |drawing, app, assets, gfx, plugins, state: &mut State| {
                                let mut draw = get_mut::<State, Draw>(state);
                                gfx.render(draw);
                                let mut draw = gfx.create_draw();
                                let num = get_menu_value_num(state, "char_view_selected")
                                    .unwrap_or(1)
                                    - 1;
                                let asset = state
                                    .assets
                                    .get("assets/Icons")
                                    .unwrap()
                                    .get(&format!("img_{}.png", num.to_string()))
                                    .unwrap();
                                let texture = asset.lock().unwrap();
                                draw.image(&texture).position(drawing.pos.0, drawing.pos.1);
                                state.draw = draw;
                            },
                        }),
                        Box::new(Container {
                            inside: vec![
                                Button {
                                    inside: nav_button_text("Пред.".into()),
                                    rect: nav_button_rect,
                                    if_hovered: None,
                                    if_clicked: Some(
                                        |button, app, assets, plugins, state: &mut State| {
                                            match state.menu_data.get_mut("char_view_selected") {
                                                Some(value) => match value {
                                                    Value::Num(num) => {
                                                        if *num > 1 {
                                                            *num -= 1;
                                                        }
                                                    }
                                                    _ => {}
                                                },
                                                None => {
                                                    state.menu_data.insert(
                                                        "char_view_selected",
                                                        Value::Num(1),
                                                    );
                                                }
                                            }
                                            set_menu_value_num(state, "char_view_changed", 1);
                                        },
                                    ),
                                    focused: false,
                                    selected: false,
                                },
                                Button {
                                    inside: nav_button_text("След.".into()),
                                    rect: nav_button_rect,
                                    if_hovered: None,
                                    if_clicked: Some(
                                        |button, app, assets, plugins, state: &mut State| {
                                            match state.menu_data.get_mut("char_view_selected") {
                                                Some(value) => match value {
                                                    Value::Num(num) => *num += 1,
                                                    _ => {}
                                                },
                                                None => {
                                                    state.menu_data.insert(
                                                        "char_view_selected",
                                                        Value::Num(1),
                                                    );
                                                }
                                            }
                                            set_menu_value_num(state, "char_view_changed", 1);
                                        },
                                    ),
                                    focused: false,
                                    selected: false,
                                },
                            ],
                            interval: Position(150., 0.),
                            align_direction: Direction::Right,
                            ..Default::default()
                        }),
                        Box::new(SingleContainer {
                            inside: Some(Text {
                                text: "АБОБА".to_string(),
                                font: FontId(0),
                                align_h: AlignHorizontal::Left,
                                align_v: AlignVertical::Top,
                                pos: Position(0., 0.),
                                size: 20.0,
                                rect_size: None,
                                max_width: None,
                                color: Color::BLACK,
                                boo: Default::default(),
                            }),
                            on_draw: None,
                            after_draw: Some(
                                |container, app, assets, plugins, state: &mut State| {
                                    if app.keyboard.is_down(KeyCode::Escape) {
                                        state.menu_id = Menu::Main as usize;
                                    }
                                    if get_menu_value_num(state, "char_view_changed").unwrap_or(1)
                                        == 1
                                    {
                                        let num = get_menu_value_num(state, "char_view_selected")
                                            .unwrap_or(1);
                                        match &mut container.inside {
                                            Some::<Text<State>>(text) => {
                                                text.text = state
                                                    .units
                                                    .get(&(num as usize))
                                                    .unwrap()
                                                    .to_string();
                                                set_menu_value_num(state, "char_view_changed", 0);
                                            }
                                            None => {}
                                        }
                                    }
                                },
                            ),
                            pos: Position(0., 200.),
                        }),
                    ],
                    pos: Position(0., 0.),
                })],
                ..Default::default()
            }),
            on_draw: Some(|container, app, assets, gfx, plugins, state: &mut State| {
                get_mut::<State, Draw>(state)
                    .rect((0., 0.), *MONITOR_SIZE.lock().unwrap())
                    .color(Color::ORANGE);
            }),
            after_draw: None,
            pos: Position(0., 0.),
        }],
    );
    fn draw_unit_info(unit: &Unit, pos: Position, state: &State, draw: &mut Draw) {
        let stats = &unit.stats;
        let damage_info = &*if stats.damage.hand > 0 {
            format!("A: {}", stats.damage.hand)
        } else if stats.damage.ranged > 0 {
            format!("A: {}", stats.damage.ranged)
        } else if stats.damage.magic > 0 {
            format!("Pwr: {}", stats.damage.magic)
        } else {
            "".into()
        };
        let defence_info = &*format!(
            "D: {}/{}",
            stats.defence.hand_units, stats.defence.ranged_units
        );
        let speed_info = &*format!("Ini: {}", stats.speed);
        let move_info = &*format!("Mnvr: {}/{}", stats.moves, stats.max_moves);
        let health_info = &*format!("Hits: {}/{}", stats.hp, stats.max_hp);
        draw.text(&state.fonts[0], damage_info)
            .size(10.)
            .position(pos.0, pos.1 + 92.);
        draw.text(&state.fonts[0], move_info)
            .size(10.)
            .position(pos.0, pos.1 + 102.);

        draw.text(&state.fonts[0], defence_info)
            .size(10.)
            .position(pos.0 + 40., pos.1 + 92.);
        draw.text(&state.fonts[0], speed_info)
            .size(10.)
            .position(pos.0 + 50., pos.1 + 102.);

        draw.text(&state.fonts[0], health_info)
            .size(10.)
            .position(pos.0 + 46., pos.1 + 112.)
            .h_align_center();
    }
    fn unit_card_draw(
        drawing: &mut Drawing<State>,
        app: &mut App,
        assets: &mut Assets,
        gfx: &mut Graphics,
        plugins: &mut Plugins,
        state: &mut State,
        army: usize,
        index: usize,
    ) {
        let mut draw = get_mut::<State, Draw>(state);
        gfx.render(draw);
        let pos = drawing.pos;
        let mut draw = gfx.create_draw();
        let index = (pos.0 / 102.) as usize + index;
        let troop = state.gamemap.armys[army].troops[index].get();

        if let Some(troop) = troop.as_ref() {
            let unit = &troop.unit;
            let texture = &state
                .assets
                .get("assets/Icons")
                .unwrap()
                .get(&*format!("img_{}.png", unit.info.icon_index))
                .unwrap()
                .lock()
                .unwrap();
            draw.image(&texture).position(pos.0, pos.1);
            draw.rect((pos.0, pos.1 + 92.), (92., 50.))
                .color(if troop.is_main {
                    Color::RED
                } else if troop.is_free {
                    Color::BLUE
                } else {
                    Color::BROWN
                });
            draw_unit_info(unit, pos, &*state, &mut draw);
            draw.text(&state.fonts[0], &*format!("{};{};{}", army, index, pos.0))
                .color(Color::BLACK)
                .position(pos.0, pos.1);
            let health_rect = (
                (pos.0, pos.1 + 92.),
                (
                    92.,
                    -((1. - (unit.stats.hp as f64 / unit.stats.max_hp as f64) as f32) * 92.),
                ),
            );
            draw.rect(health_rect.0, health_rect.1)
                .color(Color::from_rgba(1., 0., 0., 0.7));
            if state.battle.active_unit == Some((army, index)) {
                draw.rect((pos.0, pos.1), (92., 92.))
                    .color(Color::TRANSPARENT)
                    .stroke_color(Color::from_rgba(0., 255., 0., 0.3))
                    .stroke(10.);
            } else if let Some(active_unit) = state.battle.active_unit {
                if let Some(can_interact) = &state.battle.can_interact {
                    if can_interact.contains(&(army, index)) {
                        draw.rect((pos.0, pos.1), (92., 92.))
                            .color(Color::TRANSPARENT)
                            .stroke_color(if active_unit.0 == army {
                                Color::from_rgba(0.1, 0.1, 0.6, 0.5)
                            } else {
                                Color::from_rgba(0.8, 0., 0.01, 0.5)
                            })
                            .stroke(10.);
                    }
                }
            }
        }
        state.draw = draw;
    }
    fn unit_card_clicked(state: &mut State, army: usize, index: usize) {
        let mut unit_index = u64::MAX;
        if let Some(troop) = state.gamemap.armys[army].troops[index].get().as_mut() {
            unit_index = { troop.unit.info.icon_index as u64 + 1 };
        }
        if !unit_index == u64::MAX {
            set_menu_value_num(state, "battle_unit_stat", unit_index as i64);
            set_menu_value_num(state, "battle_unit_stat_changed", 1);
        }
        if let Some(active_unit) = state.battle.active_unit {
            let moves;
            let active_is_dead;
            {
                let mut troop1 = state.gamemap.armys[active_unit.0].troops[active_unit.1].get();
                let mut unit1 = troop1.as_mut().unwrap();
                moves = unit1.unit.stats.moves;
                active_is_dead = unit1.unit.is_dead();
                if moves == 0 || active_is_dead {
                    drop(troop1);
                    state.battle.active_unit = state.battle.search_next_active(&*state);
                    if state.battle.active_unit == None {
                        next_move(state);
                        state.battle.active_unit = state.battle.search_next_active(&*state);
                    }
                    state.battle.can_interact = search_interactions(state);
                    return;
                }
                if !(active_unit == (army, index)) {
                    let mut troop2 = state.gamemap.armys[army].troops[index].get();
                    if let Some(troop) = troop2.as_mut() {
                        let unit2 = &mut troop.unit;
                        if !unit2.is_dead() {
                            if unit1.unit.stats.moves != 0 {
                                if unit1.unit.attack(
                                    unit2,
                                    UnitPos::from_index(index),
                                    UnitPos::from_index(active_unit.1),
                                ) {
                                    unit1.unit.stats.moves -= 1;
                                }
                            }
                        }
                    } else {
                        unit1.unit.stats.moves -= 1;
                        drop(troop1);
                        drop(troop2);
                        let mut add_index = 0;
                        state.gamemap.armys[army].troops.remove(index);
                        if army == active_unit.0 && index < active_unit.1 {
                            add_index = 1;
                        }
                        let troop = state.gamemap.armys[active_unit.0]
                            .troops
                            .remove(active_unit.1 + add_index);
                        state.gamemap.armys[active_unit.0]
                            .troops
                            .insert(active_unit.1 + add_index, SendMut::new(None));
                        state.gamemap.armys[army].troops.insert(index, troop);
                        state.battle.active_unit = state.battle.search_next_active(&*state);
                        if state.battle.active_unit == None {
                            next_move(state);
                            state.battle.active_unit = state.battle.search_next_active(&*state);
                        }
                        state.battle.can_interact = search_interactions(state);
                    }
                } else {
                    unit1.unit.stats.moves -= 1;
                }
            }
            if moves == 0 || active_is_dead {
                state.battle.active_unit = state.battle.search_next_active(&*state);
                if state.battle.active_unit == None {
                    next_move(state);
                    state.battle.active_unit = state.battle.search_next_active(&*state);
                }
                state.battle.can_interact = search_interactions(state);
            }
        }
    }
    hashmap.insert(Menu::Battle as usize, vec![SingleContainer {
        inside: Some(DynContainer {
            inside: vec![
                Box::new(StraightDynContainer {
                    inside: vec![
                        Box::new(SingleContainer {
                            inside: Some(Container {
                                inside: vec![
                                    Container {
                                        inside: (0..half_troops).map(|_| {
                                            Button {
                                                inside: Some(Drawing {
                                                    pos: Position(0., 0.),
                                                    to_draw: |drawing, app, assets, gfx, plugins, state| {
                                                        unit_card_draw(drawing, app, assets, gfx, plugins, state, 0, 0);
                                                    }
                                                }),
                                                rect: Rect { pos: Position(0., 0.), size: Size(92., 92.) },
                                                if_hovered: None,
                                                if_clicked: Some(|button, app, assets, plugins, state| {
                                                    let pos = button.rect.pos;
                                                    let index = (pos.0 / 102.) as usize;
                                                    let army = 0;
                                                    unit_card_clicked(state, army, index);
                                                }),
                                                ..Default::default()
                                            }
                                        }).collect::<Vec<_>>(),
                                        pos: Position(0., 0.),
                                        align_direction: Direction::Right,
                                        interval: Position(10., 0.),
                                        boo: Default::default()
                                    },
                                    Container {
                                        inside: (0..half_troops).map(|_| {
                                            Button {
                                                inside: Some(Drawing {
                                                    pos: Position(0., 0.),
                                                    to_draw: |drawing, app, assets, gfx, plugins, state| {
                                                        unit_card_draw(drawing, app, assets, gfx, plugins, state, 0, *MAX_TROOPS.lock().unwrap() / 2);
                                                    }
                                                }),
                                                rect: Rect { pos: Position(0., 0.), size: Size(92., 92.) },
                                                if_hovered: None,
                                                if_clicked: Some(|button, app, assets, plugins, state| {
                                                    let pos = button.rect.pos;
                                                    let index = (pos.0 / 102.) as usize + *MAX_TROOPS.lock().unwrap() / 2;
                                                    let army = 0;
                                                    unit_card_clicked(state, army, index);
                                                }),
                                                ..Default::default()
                                            }
                                        }).collect::<Vec<_>>(),
                                        pos: Position(0., 0.),
                                        align_direction: Direction::Right,
                                        interval: Position(10., 0.),
                                        boo: Default::default()
                                    },
                                ],
                                pos: Position(0., 0.),
                                align_direction: Direction::Bottom,
                                interval: Position(0., 50.),
                                boo: Default::default()
                            }),
                            on_draw: None,
                            after_draw: None,
                            pos: Position(10., 30.)
                        }),
                        Box::new(SingleContainer {
                            inside: Some(Container {
                                inside: vec![
                                    Container {
                                        inside: (0..half_troops).map(|_| {
                                            Button {
                                                inside: Some(Drawing {
                                                    pos: Position(0., 0.),
                                                    to_draw: |drawing, app, assets, gfx, plugins, state| {
                                                        unit_card_draw(drawing, app, assets, gfx, plugins, state, 1, *MAX_TROOPS.lock().unwrap() / 2);
                                                    }
                                                }),
                                                rect: Rect { pos: Position(0., 0.), size: Size(92., 92.) },
                                                if_hovered: None,
                                                if_clicked: Some(|button, app, assets, plugins, state| {
                                                    let pos = button.rect.pos;
                                                    let army = 1;
                                                    let index = (pos.0 / 102.) as usize + *MAX_TROOPS.lock().unwrap() / 2;
                                                    unit_card_clicked(state, army, index);
                                                }),
                                                ..Default::default()
                                        }}).collect::<Vec<_>>(),
                                        pos: Position(0., 0.),
                                        align_direction: Direction::Right,
                                        interval: Position(10., 0.),
                                        boo: Default::default()
                                    },
                                    Container  {
                                        inside: (0..half_troops).map(|_| {
                                            Button {
                                                inside: Some(Drawing {
                                                    pos: Position(0., 0.),
                                                    to_draw: |drawing, app, assets, gfx, plugins, state| {
                                                        unit_card_draw(drawing, app, assets, gfx, plugins, state, 1, 0);
                                                    }
                                                }),
                                                rect: Rect { pos: Position(0., 0.), size: Size(92., 92.) },
                                                if_hovered: None,
                                                if_clicked: Some(|button, app, assets, plugins, state| {
                                                    let pos = button.rect.pos;
                                                    let index = (pos.0 / 102.) as usize;
                                                    let army = 1;
                                                    unit_card_clicked(state, army, index);
                                                }),
                                                ..Default::default()
                                            }
                                        }).collect::<Vec<_>>(),
                                        pos: Position(0., 0.),
                                        align_direction: Direction::Right,
                                        interval: Position(10., 0.),
                                        boo: Default::default()
                                    },
                                ],
                                pos: Position(0., 0.),
                                align_direction: Direction::Bottom,
                                interval: Position(0., 50.),
                                boo: Default::default()
                            }),
                            on_draw: None,
                            after_draw: None,
                            pos: Position(10., 500.)
                        }),
                        Box::new(SingleContainer {
                            inside: Some(Text {
                                text: "Статистика".to_string(),
                                font: FontId(0),
                                align_h: AlignHorizontal::Left,
                                align_v: AlignVertical::Top,
                                pos: Position(0., 0.),
                                size: 20.0,
                                rect_size: None,
                                max_width: Some(100.),
                                color: Color::BLACK,
                                boo: Default::default()
                            }),
                            on_draw: None,
                            after_draw: Some(|container, app, assets, plugins, state: &mut State| {
                                if app.keyboard.is_down(KeyCode::Escape) { state.menu_id = Menu::Main as usize; }
                                if get_menu_value_num(state, "battle_unit_stat_changed").unwrap_or(0) == 1 {
                                    let num = get_menu_value_num(state, "battle_unit_stat").unwrap_or(1);
                                    match &mut container.inside {
                                        Some::<Text<State>>(text) => {
                                            text.text = state.units.get(&(num as usize)).unwrap().to_string();
                                            set_menu_value_num(state, "battle_unit_stat_changed", 0);
                                        }
                                        None => {}
                                }   }
                            }),
                            pos: Position(900., 200.)
                        }),
                        Box::new(Button {
                            inside: Some(Text {
                                text: "Заново".to_string(),
                                font: FontId(0),
                                align_h: AlignHorizontal::Left,
                                align_v: AlignVertical::Center,
                                pos: Position(50., 0.),
                                size: 10.0,
                                rect_size: None,
                                max_width: None,
                                color: Color::BLACK,
                                boo: Default::default()
                            }),
                            rect: Rect { pos: Position(200., 900.), size: Size(100., 30.) },
                            if_hovered: None,
                            if_clicked: Some(|button, app, assets, plugins, state: &mut State| {
                                let mut rng = thread_rng();
                                state.gamemap.armys = vec![
                                    Army {
                                        troops: (0..*MAX_TROOPS.lock().unwrap()).map(|_| Troop {
                                            was_payed: true,
                                            is_free: false,
                                            is_main: false,
                                            custom_name: None,
                                            unit: {
                                                let mut unit = state.units[&rng.gen_range(1..100)].clone();
                                                unit.army = 1;
                                                unit
                                            }
                                        }.into()).collect::<Vec<_>>(),
                                        stats: ArmyStats {
                                            gold: 0,
                                            mana: 0,
                                            army_name: "hero".to_string()
                                        },
                                        inventory: vec![],
                                        pos: [0, 0]
                                    },
                                    Army {
                                        troops: (0..*MAX_TROOPS.lock().unwrap()).map(|_| Troop {
                                            was_payed: true,
                                            is_free: false,
                                            is_main: false,
                                            custom_name: None,
                                            unit: state.units[&rng.gen_range(1..100)].clone()
                                        }.into()).collect::<Vec<_>>(),
                                        stats: ArmyStats {
                                            gold: 0,
                                            mana: 0,
                                            army_name: "pidoras".to_string()
                                        },
                                        inventory: vec![],
                                        pos: [0, 0]
                                    }
                                ];
                                state.battle.active_unit = state.battle.search_next_active(&*state);
                                state.battle.can_interact = search_interactions(state);
                            }),
                            focused: false,
                            selected: false
                        })
                    ],
                    pos: Position(0., 0.)
                })
            ],
            pos: Position(0., 0.),
            align_direction: Direction::Bottom,
            interval: Position(0., 0.)
        }),
        on_draw: Some(|container, app, assets, gfx, plugins, state: &mut State| {
            get_mut::<State, Draw>(state)
                .rect((0., 0.), *MONITOR_SIZE.lock().unwrap())
                .color(Color::ORANGE);
            if app.keyboard.is_down(KeyCode::Escape) { state.menu_id = Menu::Main as usize; }
            if app.keyboard.was_pressed(KeyCode::Space) {
                let active_unit = state.battle.active_unit;
                if let Some(active_unit) = active_unit {
                    let mut troop = state.gamemap.armys[active_unit.0].troops[active_unit.1].get();
                    let mut unit = &mut troop.as_mut().unwrap().unit;
                    let moves = unit.stats.moves;
                    if moves != 0 {
                        unit.stats.moves -= 1;
                    }
                    drop(troop);
                    if moves == 0 {
                        state.battle.active_unit = state.battle.search_next_active(&*state);
                        if state.battle.active_unit == None {
                            next_move(state);
                            state.battle.active_unit = state.battle.search_next_active(&*state);
                        }
                        state.battle.can_interact = search_interactions(state);
                    }
                }
            }
        }),
        after_draw: Some(|container, app, assets, plugins, state: &mut State| {
        }),
        pos: Position(0., 0.)
    }]);
    dbg!(size_of::<State>());
}
static TILES: Lazy<[Tile; 9]> = Lazy::new(|| {
    [
        Tile::new(2, "Land.png", false),
        Tile::new(1, "Badground.png", false),
        Tile::new(4, "Road.png", false),
        Tile::new(2, "Snow.png", false),
        Tile::new(2, "Plain.png", false),
        Tile::new(2, "LowLand.png", false),
        Tile::new(4, "Water.png", true),
        Tile::new(4, "Shallow.png", true),
        Tile::new(0, "DeepWater.png", false),
    ]
});
fn gen_army_troops(
    rng: &mut ThreadRng,
    units: &HashMap<usize, Unit>,
    army: usize,
) -> Vec<TroopType> {
    let mut troops = (0..*MAX_TROOPS.lock().unwrap())
        .map(|_| {
            Troop {
                was_payed: true,
                is_free: false,
                is_main: false,
                custom_name: None,
                unit: {
                    let mut unit = loop {
                        let unit = units[&rng.gen_range(1..100)].clone();
                        if unit.stats.damage.ranged > 0 || unit.stats.damage.magic > 0 {
                            break unit;
                        }
                    };
                    unit.army = army;
                    unit
                },
            }
            .into()
        })
        .collect::<Vec<TroopType>>();
    troops.append(
        &mut (0..*MAX_TROOPS.lock().unwrap())
            .map(|_| {
                Troop {
                    was_payed: true,
                    is_free: false,
                    is_main: false,
                    custom_name: None,
                    unit: {
                        let mut unit = loop {
                            let unit = units[&rng.gen_range(1..100)].clone();
                            if unit.stats.damage.hand > 0 {
                                break unit;
                            }
                        };
                        unit.army = army;
                        unit
                    },
                }
                .into()
            })
            .collect::<Vec<_>>(),
    );
    troops
}
fn setup(assets: &mut Assets, gfx: &mut Graphics) -> State {
    let mut rng = thread_rng();
    let asset_dirs = ["assets/Icons", "assets/Terrain"];
    let mut my_assets: HashMap<&str, HashMap<String, Asset<Texture>>> = HashMap::new();
    for dir in asset_dirs {
        let mut dir_assets: HashMap<String, Asset<Texture>> = HashMap::new();
        for entry in read_dir(dir).expect("Cant find directory") {
            let entry = entry.expect("erorrere");
            let path = entry.path();
            let asset = assets.load_asset(path.to_str().clone().unwrap()).unwrap();
            dir_assets.insert(
                path.strip_prefix(dir)
                    .unwrap()
                    .to_str()
                    .unwrap()
                    .to_string(),
                asset,
            );
        }
        my_assets.insert(dir, dir_assets);
    }
    let settings = parse_settings();
    {
        *MAX_TROOPS.lock().unwrap() = settings.max_troops;
    }
    parse_locale(settings.locale);
    gen_forms(gfx);
    let units = parse_units();
    let mut state = State {
        fonts: vec![gfx
            .create_font(include_bytes!("Ru_Gothic.ttf"))
            .expect("shit happens")],
        draw: gfx.create_draw(),
        frame: 0,
        gamemap: GameMap {
            time: Time::new(0),
            tilemap: (0..MAP_SIZE)
                .map(|_| {
                    (0..MAP_SIZE)
                        .map(|_| rng.gen_range(0..9))
                        .collect::<Vec<usize>>()
                        .try_into()
                        .unwrap()
                })
                .collect::<Vec<[usize; MAP_SIZE]>>()
                .try_into()
                .unwrap(),
            decomap: [[None; MAP_SIZE]; MAP_SIZE],
            armys: vec![
                Army {
                    troops: gen_army_troops(&mut rng, &units, 0),
                    stats: ArmyStats {
                        gold: 0,
                        mana: 0,
                        army_name: "hero".to_string(),
                    },
                    inventory: vec![],
                    pos: [0, 0],
                },
                Army {
                    troops: gen_army_troops(&mut rng, &units, 1),
                    stats: ArmyStats {
                        gold: 0,
                        mana: 0,
                        army_name: "pidoras".to_string(),
                    },
                    inventory: vec![],
                    pos: [0, 0],
                },
            ],
        },
        units,
        menu_id: 0,
        menu_data: HashMap::new(),
        assets: my_assets,
        battle: BattleInfo {
            army1: 0,
            army2: 1,
            battle_ter: 0,
            active_unit: None,
            can_interact: None,
            move_count: 0,
            dead: vec![],
        },
    };
    let mut battle = state.battle.clone();
    battle.start(&mut state);
    state.battle = battle;
    state.battle.active_unit = state.battle.search_next_active(&state);
    state.battle.can_interact = search_interactions(&mut state);
    state
}
fn draw(
    app: &mut App,
    assets: &mut Assets,
    gfx: &mut Graphics,
    plugins: &mut Plugins,
    state: &mut State,
) {
    state.draw = gfx.create_draw();
    get_mut::<State, Draw>(state).clear(Color::WHITE);
    forms
        .lock()
        .unwrap()
        .get_mut(&state.menu_id)
        .unwrap()
        .iter_mut()
        .for_each(|form| form.draw(app, assets, gfx, plugins, state));
    gfx.render(get_mut::<State, Draw>(state));
    state.frame += 1;
}
fn update(app: &mut App, assets: &mut Assets, plugins: &mut Plugins, state: &mut State) {
    forms
        .lock()
        .unwrap()
        .get_mut(&state.menu_id)
        .unwrap()
        .iter_mut()
        .for_each(|form| form.after(app, assets, plugins, state));
}
#[notan_main]
fn main() -> Result<(), String> {
    let win = WindowConfig::new()
        .title("Discord Times: Remastered")
        .vsync(true)
        .lazy_loop(true)
        .high_dpi(true)
        .fullscreen(true)
        .size(1600, 1200)
        .resizable(true);
    notan::init_with(setup)
        .add_config(win)
        .add_config(TextConfig)
        .add_config(DrawConfig)
        .draw(draw)
        .update(update)
        .build()
}
