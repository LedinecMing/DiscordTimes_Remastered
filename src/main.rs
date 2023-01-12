mod lib;


use std::fmt::{Debug, Formatter};
use rand::thread_rng;
use {
    std:: {
        collections::HashMap,
        mem::size_of,
        fmt::Display,
        fs::{read_dir, read},
        sync::Mutex
    },
    notan:: {
        prelude::*,
        app::AppState,
        draw::*,
        text::{TextConfig, TextExtension}
    },
    notan_ui::{
        form::Form,
        forms::{Data, Drawing},
        defs::*,
        text::*,
        containers::{SingleContainer, SliderContainer, Container, DynContainer},
        wrappers::{Button, Checkbox},
        rect::*
    },
    lib::{
        map::{
            map::*,
            tile::Tile
        },
        time::time::Time,
    },
    rand::Rng,
    once_cell::sync::Lazy
};
use notan_ui::containers::StraightDynContainer;
use crate::lib::battle::army::{Army, ArmyStats, MAX_TROOPS};
use crate::lib::battle::battlefield::BattleInfo;
use crate::lib::battle::troop::Troop;
use crate::lib::parse::parse_units;
use crate::lib::units::unit::{Unit, UnitPos};

#[derive(Clone, Debug)]
pub enum Value {
    Num(i64),
    Str(String)
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
    pub battle: BattleInfo
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

fn asset_path(path: &str) -> String {
    let base = "./assets";
    format!("{}/{}", base, path)
}

fn get_menu_value_str(state: &mut State, id: &'static str) -> Option<String> {
    match state.menu_data.get(id) {
        Some(Value::Str(string)) => Some(string.clone()),
        _ => None
}   }
fn get_menu_value_num(state: &mut State, id: &'static str) -> Option<i64> {
    match state.menu_data.get(id) {
        Some(Value::Num(num)) => Some(*num),
        _ => None
}   }

fn set_menu_value_str(state: &mut State, id: &'static str, new: String) {
    match state.menu_data.get_mut(id) {
        Some(value) => match value {
            Value::Str(string) => {*string = new;},
            _ => {}
        }
        _ => {state.menu_data.insert(id, Value::Str(new));}
}   }
fn set_menu_value_num(state: &mut State, id: &'static str, new: i64) {
    match state.menu_data.get_mut(id) {
        Some(value) => match value {
            Value::Num(num) => {*num = new;},
            _ => {}
        }
        _ => {state.menu_data.insert(id, Value::Num(new));}
}   }


fn menu_button(text: impl Into<String>,
               on_draw: fn(&mut SingleContainer<State, Text<State>>, &mut App, &mut Assets, &mut Graphics, &mut Plugins, &mut State),
               if_clicked: fn(&mut Button<State, SingleContainer<State, Text<State>>>, &mut App, &mut Assets, &mut Plugins, &mut State))
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
            size: Size(300., 130.)
        },
        if_clicked: Some(if_clicked),
        ..Button::default()
    })
}

type FormsInside = dyn Form<State>;
static forms: Mutex<Lazy<HashMap<usize, Vec<SingleContainer<State, DynContainer<State>>>>>> = Mutex::new(Lazy::new(||HashMap::new()));
static MONITOR_SIZE: Mutex<(f32, f32)> = Mutex::new((0., 0.));

enum Menu {
    Main = 0,
    Start = 1,
    Load = 2,
    Settings = 3,
    Authors = 4,
    UnitView = 5,
    JustRectangle = 6,
    Battle = 7
}

fn gen_forms(gfx: &mut Graphics) {
    let draw_back: for<'a, 'b, 'c, 'd, 'e, 'f> fn(&'a mut SingleContainer<State, Text<State>>, &'b mut App, &'f mut Assets, &'c mut Graphics, &'d mut Plugins, &'e mut State) =
        |container, app, assets, gfx, plugins, state: &mut State| {
            get_mut::<State, Draw>(state)
            .rect((container.pos - Position(container.get_size().0/2., 0.)).into(),
                  container.get_size().into())
            .color(Color::from_hex(0x033121ff));
    };
    let size = gfx.device.size();
    *MONITOR_SIZE.lock().unwrap() = (size.0 as f32, size.1 as f32);
    let mut hashmap = forms.lock().unwrap();
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
                            |container, app, assets, plugins, state: &mut State| {
                                state.menu_id = Menu::Start as usize;
                            }
                        ),
                        menu_button("Боёвка", draw_back,
                                    |container, app, assets, plugins, state: &mut State| {
                                        state.menu_id = Menu::Battle as usize;
                                    }
                        ),
                        menu_button("Загрузка", draw_back,
                            |container, app, assets, plugins, state: &mut State| {
                                state.menu_id = Menu::Load as usize;
                            }
                        ),
                        menu_button("Настройки", draw_back,
                            |container, app, assets, plugins, state: &mut State| {
                                state.menu_id = Menu::Settings as usize;
                            }
                        ),
                        menu_button("Авторы", draw_back,
                            |container, app, assets, plugins, state: &mut State| {
                                state.menu_id = Menu::Authors as usize;
                            }
                        ),
                        menu_button("Персонажи", draw_back,
                            |container, app, assets, plugins, state: &mut State| {
                                state.menu_id = Menu::UnitView as usize;
                            }
                        ),
                        menu_button("квадратик", draw_back,
                                    |container, app, assets, plugins, state: &mut State| {
                                state.menu_id = Menu::JustRectangle as usize;
                            }
                        ),
                        menu_button("Выход", draw_back,
                            |container, app, assets, plugins, state: &mut State| {
                                app.exit()
                            }
                        ),
                    ],
                    pos: Position(800., 0.),
                    align_direction: Direction::Bottom,
                    interval: Position(0., 20.)
                }),
                on_draw: Some(|container, app, assets, gfx, plugins, state: &mut State| {
                    get_mut::<State, Draw>(state)
                        .rect((0., 0.), *MONITOR_SIZE.lock().unwrap())
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
                        checked: false,
                        on_draw: Some(|container, app, assets, gfx, plugins, state: &mut State| {
                            get_mut::<State, Draw>(state)
                            .rect((container.pos - Position(container.get_size().0/2., container.get_pos().1)).into(),
                                  container.get_size().into())
                            .color(Color::from_hex(0x033121ff));
                            set_menu_value_num(state, "setting_rectangle_rotate_mode", container.checked as i64);
                        }),
                        pos: Position(20., 0.),
                        ..Default::default()
                    }),
                    Box::new(Text {
                        text: "Квадратик крутится вокруг своего центра или вокруг центра квадратиков".to_string(),
                        font: FontId(0),
                        align_h: AlignHorizontal::Left,
                        align_v: AlignVertical::Top,
                        pos: Position(20., 0.),
                        size: 20.0,
                        rect_size: None,
                        max_width: None,
                        color: Color::BLACK,
                        boo: Default::default()
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
        }),
        after_draw: Some(|container, app, assets, plugins, state: &mut State| {
            if app.keyboard.is_down(KeyCode::Escape) { state.menu_id = Menu::Main as usize; }
        }),
        pos: Position(0., 0.)
    }]);
    hashmap.insert(Menu::JustRectangle as usize, vec![
        SingleContainer {
            inside: Some(DynContainer {
                    inside: vec![
                        Box::new(
                            SingleContainer {
                                inside: None::<Drawing<State>>,
                                on_draw: Some(|drawing: &mut SingleContainer<State, _>, app, assets, gfx, plugins, state: &mut State| {
                                    let deg = get_menu_value_num(state, "map_rotation_deg").unwrap_or(0) as f32 / 4.0;
                                    let mut center = drawing.pos + Position(25. * 8., 25. * 8.);
                                    for i in 0..MAP_SIZE {
                                         for j in 0..MAP_SIZE {
                                             let color = Color::RED;
                                             let pos = drawing.pos + Position(i as f32 * 8., j as f32 * 8.);
                                             if get_menu_value_num(state, "setting_rectangle_rotate_mode").unwrap_or(0) == 1 {
                                                 center = pos - Position(4., 4.);
                                             }
                                             get_mut::<State, Draw>(state)
                                                .rect(pos.into(), (8., 8.)).rotate_degrees_from(center.into(), deg)
                                                .color(color);
                                    }   }
                                }),
                                after_draw: Some(|container, app, assets, plugins, state: &mut State| {
                                    if app.keyboard.is_down(KeyCode::W) { container.add_pos(Position(0., -8.)); }
                                    if app.keyboard.is_down(KeyCode::A) { container.add_pos(Position(-8., 0.)); }
                                    if app.keyboard.is_down(KeyCode::S) { container.add_pos(Position(0., 8.)); }
                                    if app.keyboard.is_down(KeyCode::D) { container.add_pos(Position(8., 0.)); }
                                    if app.keyboard.is_down(KeyCode::Q) {
                                        if let Some(num) = get_menu_value_num(state, "map_rotation_deg") {
                                            set_menu_value_num(state, "map_rotation_deg", num - 1)
                                    }   else {
                                            set_menu_value_num(state, "map_rotation_deg", 0);
                                    }   }
                                    if app.keyboard.is_down(KeyCode::E) {
                                        if let Some(num) = get_menu_value_num(state, "map_rotation_deg") {
                                            set_menu_value_num(state, "map_rotation_deg", num + 1)
                                        }   else {
                                            set_menu_value_num(state, "map_rotation_deg", 0);
                                    }   }
                                    if app.keyboard.is_down(KeyCode::Escape) { state.menu_id = Menu::Main as usize; }
                                }),
                                pos: Position(0., 0.)
                            }
                        )
                    ],
                    pos: Position(0., 0.),
                    align_direction: Direction::Bottom,
                    interval: Position(0., 50.)
            }),
            on_draw: Some(|container, app, assets, gfx, plugins, state: &mut State| {
                get_mut::<State, Draw>(state)
                    .rect((0., 0.), *MONITOR_SIZE.lock().unwrap())
                    .color(Color::ORANGE);
            }),
            after_draw: None,
            pos: Position(0., 0.)
        }
    ]);
    let nav_button_rect = Rect {pos: Position(0., 0.), size: Size(70., 100.)};
    let nav_button_text = |text: String| Some(Text {
        text,
        font: FontId(0),
        align_h: AlignHorizontal::Left,
        align_v: AlignVertical::Bottom,
        pos: Position(0., 100.), size: 10.,
        rect_size: None,
        max_width: None,
        color: Color::BLACK,
        boo: std::marker::PhantomData
    });
    hashmap.insert(Menu::Start as usize, vec![
        SingleContainer {
            inside: Some(DynContainer {
                inside: vec![
                    Box::new(
                        SingleContainer {
                            inside: None::<Drawing<State>>,
                            on_draw: Some(|drawing: &mut SingleContainer<State, _>, app, assets, gfx, plugins, state: &mut State| {
                                let mut draw = get_mut::<State, Draw>(state);
                                gfx.render(draw);
                                let mut draw = gfx.create_draw();
                                for i in 0..MAP_SIZE {
                                    for j in 0..MAP_SIZE {
                                        let asset = state.assets.get("assets/Terrain").unwrap().get(&format!("{}", TILES[state.gamemap.tilemap[i][j]].sprite())).unwrap();
                                        let texture = asset.lock().unwrap();
                                        let pos = Position(i as f32 * 53., j as f32 * 50.);
                                        draw
                                            .image(&texture)
                                            .position(pos.0, pos.1)
                                            .translate(53., 50.);
                                }   }
                                state.draw = draw;
                            }),
                            after_draw: Some(|container, app, assets, plugins, state: &mut State| {
                                if app.keyboard.is_down(KeyCode::Escape) { state.menu_id = Menu::Main as usize; }
                            }),
                            pos: Position(0., 0.)
                        }
                    )
                ],
                pos: Position(0., 0.),
                align_direction: Direction::Bottom,
                interval: Position(0., 0.)
            }),
            on_draw: Some(|container, app, assets, gfx, plugins, state: &mut State| {
                get_mut::<State, Draw>(state)
                    .rect((0., 0.), *MONITOR_SIZE.lock().unwrap())
                    .color(Color::ORANGE);
            }),
            after_draw: None,
            pos: Position(0., 0.)
        }
    ]);
    hashmap.insert(Menu::UnitView as usize, vec![
        SingleContainer {
            inside: Some(
                DynContainer {
                    inside: vec![
                        Box::new(Drawing {
                            pos: Position(0., 0.),
                            to_draw: |drawing, app, assets, gfx, plugins, state: &mut State| {
                                let mut draw = get_mut::<State, Draw>(state);
                                gfx.render(draw);
                                let mut draw = gfx.create_draw();
                                let num = get_menu_value_num(state, "char_view_selected").unwrap_or(1) - 1;
                                let asset = state.assets.get("assets/Icons").unwrap().get(&format!("img_{}.png", num.to_string())).unwrap();
                                let texture = asset.lock().unwrap();
                                draw.image(&texture).position(drawing.pos.0, drawing.pos.1);
                                state.draw = draw;
                            }
                        }),
                        Box::new(Container{
                            inside: vec![
                                Button {
                                    inside: nav_button_text("Пред.".into()),
                                    rect: nav_button_rect,
                                    if_hovered: None,
                                    if_clicked: Some(|button, app, assets, plugins, state: &mut State| {
                                        match state.menu_data.get_mut("char_view_selected") {
                                            Some(value) => {
                                                match value {
                                                    Value::Num(num) => {
                                                        if *num > 1 {
                                                            *num -= 1;
                                                    }   },
                                                    _ => {}
                                            }   },
                                            None => {state.menu_data.insert("char_view_selected", Value::Num(1));}
                                        }
                                        set_menu_value_num(state, "char_view_changed", 1);
                                    }),
                                    focused: false,
                                    selected: false
                                },
                                Button {
                                    inside: nav_button_text("След.".into()),
                                    rect: nav_button_rect,
                                    if_hovered: None,
                                    if_clicked: Some(|button, app, assets, plugins, state: &mut State| {
                                        match state.menu_data.get_mut("char_view_selected") {
                                            Some(value) => {
                                                match value {
                                                    Value::Num(num) => {*num += 1},
                                                    _ => {}
                                                }
                                            },
                                            None => {state.menu_data.insert("char_view_selected", Value::Num(1));}
                                        }
                                        set_menu_value_num(state, "char_view_changed", 1);
                                    }),
                                    focused: false,
                                    selected: false
                                }
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
                            boo: Default::default()
                        }),
                        on_draw: None,
                        after_draw: Some(|container, app, assets, plugins, state: &mut State| {
                            if app.keyboard.is_down(KeyCode::Escape) { state.menu_id = Menu::Main as usize; }
                            if get_menu_value_num(state, "char_view_changed").unwrap_or(0) == 1 {
                                let num = get_menu_value_num(state, "char_view_selected").unwrap_or(1);
                                match &mut container.inside {
                                    Some::<Text<State>>(text) => {
                                        text.text = state.units.get(&(num as usize)).unwrap().to_string();
                                        set_menu_value_num(state, "char_view_changed", 0);
                                    }
                                    None => {}
                            }   }
                        }),
                        pos: Position(0., 200.)
                    })
                    ],
                    ..Default::default()
                }),
            on_draw: Some(|container, app, assets, gfx, plugins, state: &mut State| {
                get_mut::<State, Draw>(state)
                    .rect((0., 0.), *MONITOR_SIZE.lock().unwrap())
                    .color(Color::ORANGE);
            }),
            after_draw: None,
            pos: Position(0., 0.)
        }
    ]);
    fn draw_unit_info(unit: &Unit, pos: Position, state: &State, draw: &mut Draw) {
        let stats = &unit.stats;
        let damage_info = &* if stats.damage.hand > 0 {
            format!("A: {}", stats.damage.hand)
        } else if stats.damage.ranged > 0 {
            format!("A: {}", stats.damage.ranged)
        } else if stats.damage.magic > 0 {
            format!("Pwr: {}", stats.damage.magic)
        } else { "".into() };
        let defence_info = &*format!("D: {}/{}", stats.defence.hand_units, stats.defence.ranged_units);
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
    fn unit_card_draw(drawing: &mut Drawing<State>, app: &mut App, assets: &mut Assets, gfx: &mut Graphics, plugins: &mut Plugins, state: &mut State, army: usize, index: usize) {
        let mut draw = get_mut::<State, Draw>(state);
        gfx.render(draw);
        let pos = drawing.pos;
        let mut draw = gfx.create_draw();
        let index = (pos.0 / 100.) as usize + index;
        let troop = state.gamemap.armys[army].troops[index].get();
        let unit = &troop.as_ref().unwrap().unit;
        let texture = &state.assets
            .get("assets/Icons").unwrap()
            .get(&*format!("img_{}.png", unit.info.icon_index))
            .unwrap().lock().unwrap();
        draw.image(&texture)
            .position(pos.0, pos.1);
        draw.rect((pos.0, pos.1 + 92.), (92., 50.))
            .color(if troop.as_ref().unwrap().is_main {
                Color::RED
            } else if troop.as_ref().unwrap().is_free {
                Color::BLUE
            } else { Color::BROWN });
        draw_unit_info(unit, pos, &*state, &mut draw);
        if state.battle.active_unit == Some((army, index)) {
            draw.rect((pos.0, pos.1), (92., 92.))
                .color(Color::TRANSPARENT)
                .stroke_color(Color::from_rgba(0., 255., 0., 128.))
                .stroke(10.);
        }
        state.draw = draw;
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
                                        inside: (0..(MAX_TROOPS / 2)).map(|_| {
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
                                                    let index = (pos.0 / 100.) as usize;
                                                    let unit_index = {
                                                        let troop = state.gamemap.armys[0].troops[index].get();
                                                        troop.as_ref().unwrap().unit.info.icon_index + 1
                                                    };
                                                    set_menu_value_num(state, "battle_unit_stat", unit_index as i64);
                                                    set_menu_value_num(state, "battle_unit_stat_changed", 1);
                                                    if let Some(active_unit) = state.battle.active_unit {
                                                        let mut troop2 = state.gamemap.armys[0].troops[index].get();
                                                        let mut troop1 = state.gamemap.armys[active_unit.0].troops[active_unit.1].get();
                                                        troop1.as_mut().unwrap().unit.attack(&mut troop2.as_mut().unwrap().unit, UnitPos(index % 6, index / 6), UnitPos(active_unit.1 % 6, active_unit.1 / 6));
                                                    }
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
                                        inside: (0..(MAX_TROOPS / 2)).map(|_| {
                                            Button {
                                                inside: Some(Drawing {
                                                    pos: Position(0., 0.),
                                                    to_draw: |drawing, app, assets, gfx, plugins, state| {
                                                        unit_card_draw(drawing, app, assets, gfx, plugins, state, 0, 6);
                                                    }
                                                }),
                                                rect: Rect { pos: Position(0., 0.), size: Size(92., 92.) },
                                                if_hovered: None,
                                                if_clicked: Some(|button, app, assets, plugins, state| {
                                                    let pos = button.rect.pos;
                                                    let index = (pos.0 / 100.) as usize + 6;
                                                    let unit_index = {
                                                        let troop = state.gamemap.armys[0].troops[index].get();
                                                        troop.as_ref().unwrap().unit.info.icon_index + 1
                                                    };
                                                    set_menu_value_num(state, "battle_unit_stat", unit_index as i64);
                                                    set_menu_value_num(state, "battle_unit_stat_changed", 1);
                                                    if let Some(active_unit) = state.battle.active_unit {
                                                        let mut troop2 = state.gamemap.armys[0].troops[index].get();
                                                        let mut troop1 = state.gamemap.armys[active_unit.0].troops[active_unit.1].get();
                                                        troop1.as_mut().unwrap().unit.attack(&mut troop2.as_mut().unwrap().unit, UnitPos(index % 6, index / 6), UnitPos(active_unit.1 % 6, active_unit.1 / 6));
                                                    }
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
                                        inside: (0..(MAX_TROOPS / 2)).map(|_| {
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
                                                    let index = (pos.0 / 100.) as usize;
                                                    let unit_index = {
                                                        let troop = state.gamemap.armys[1].troops[index].get();
                                                        troop.as_ref().unwrap().unit.info.icon_index + 1
                                                    };
                                                    set_menu_value_num(state, "battle_unit_stat", unit_index as i64);
                                                    set_menu_value_num(state, "battle_unit_stat_changed", 1);
                                                    if let Some(active_unit) = state.battle.active_unit {
                                                        let mut troop2 = state.gamemap.armys[0].troops[index].get();
                                                        let mut troop1 = state.gamemap.armys[active_unit.0].troops[active_unit.1].get();
                                                        troop1.as_mut().unwrap().unit.attack(&mut troop2.as_mut().unwrap().unit, UnitPos(index % 6, index / 6), UnitPos(active_unit.1 % 6, active_unit.1 / 6));
                                                    }
                                                }),
                                                ..Default::default()
                                        }}).collect::<Vec<_>>(),
                                        pos: Position(0., 0.),
                                        align_direction: Direction::Right,
                                        interval: Position(10., 0.),
                                        boo: Default::default()
                                    },
                                    Container  {
                                        inside: (0..(MAX_TROOPS / 2)).map(|_| {
                                            Button {
                                                inside: Some(Drawing {
                                                    pos: Position(0., 0.),
                                                    to_draw: |drawing, app, assets, gfx, plugins, state| {
                                                        unit_card_draw(drawing, app, assets, gfx, plugins, state, 1, 6);
                                                    }
                                                }),
                                                rect: Rect { pos: Position(0., 0.), size: Size(92., 92.) },
                                                if_hovered: None,
                                                if_clicked: Some(|button, app, assets, plugins, state| {
                                                    let pos = button.rect.pos;
                                                    let index = (pos.0 / 100.) as usize + 6;
                                                    let unit_index = {
                                                        let troop = state.gamemap.armys[1].troops[index].get();
                                                        troop.as_ref().unwrap().unit.info.icon_index + 1
                                                    };
                                                    set_menu_value_num(state, "battle_unit_stat", unit_index as i64);
                                                    set_menu_value_num(state, "battle_unit_stat_changed", 1);
                                                    if let Some(active_unit) = state.battle.active_unit {
                                                        let mut troop2 = state.gamemap.armys[0].troops[index].get();
                                                        let mut troop1 = state.gamemap.armys[active_unit.0].troops[active_unit.1].get();
                                                        troop1.as_mut().unwrap().unit.attack(&mut troop2.as_mut().unwrap().unit, UnitPos(index % 6, index / 6), UnitPos(active_unit.1 % 6, active_unit.1 / 6));
                                                    }
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
                                max_width: None,
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
                            rect: Rect { pos: Position(0., 500.), size: Size(100., 30.) },
                            if_hovered: None,
                            if_clicked: Some(|button, app, assets, plugins, state: &mut State| {
                                let mut rng = thread_rng();
                                state.gamemap.armys = vec![
                                    Army {
                                        troops: (0..MAX_TROOPS).map(|_| Troop {
                                            was_payed: true,
                                            is_free: false,
                                            is_main: false,
                                            custom_name: None,
                                            unit: state.units[&rng.gen_range(1..90)].clone()
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
                                        troops: (0..MAX_TROOPS).map(|_| Troop {
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
            if app.keyboard.is_down(KeyCode::Space) {
                let active_unit = state.battle.active_unit;
                if let Some(active_unit) = active_unit {
                    let mut troop = state.gamemap.armys[active_unit.0].troops[active_unit.1].get();
                    let mut unit = &mut troop.as_mut().unwrap().unit;
                    unit.stats.moves -= 1;
                    let moves = unit.stats.moves;
                    drop(troop);
                    if moves == 0 {
                        state.battle.active_unit = state.battle.search_next_active(&*state);
                    }
                    app.keyboard.down.insert(KeyCode::Space, 0.);
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
        Tile::new(0, "DeepWater.png", false)
    ]
});

fn setup(assets: &mut Assets, gfx: &mut Graphics) -> State {
    let mut rng = rand::thread_rng();
    let asset_dirs = ["assets/Icons", "assets/Terrain"];
    let mut my_assets: HashMap<&str, HashMap<String, Asset<Texture>>> = HashMap::new();
    for dir in asset_dirs {
        let mut dir_assets: HashMap<String, Asset<Texture>> = HashMap::new();
        for entry in read_dir(dir).expect("Cant find directory") {
            let entry = entry.expect("erorrere");
            let path = entry.path();
            let asset = assets.load_asset(path.to_str().clone().unwrap()).unwrap();
            dir_assets.insert(path.strip_prefix(dir).unwrap().to_str().unwrap().to_string(), asset);
        };
        my_assets.insert(dir, dir_assets);
    }
    gen_forms(gfx);
    let units = parse_units();
    let mut state = State {
        fonts: vec![gfx
            .create_font(include_bytes!("UbuntuMono-RI.ttf"))
            .expect("shit happens")],
        draw: gfx.create_draw(),
        frame: 0,
        gamemap: GameMap {
            time: Time::new(0),
            tilemap: (0..MAP_SIZE).map(|_| { (0..MAP_SIZE).map(|_| rng.gen_range(0..9)).collect::<Vec<usize>>().try_into().unwrap() }).collect::<Vec<[usize; MAP_SIZE]>>().try_into().unwrap(),
            decomap: [[None; MAP_SIZE]; MAP_SIZE],
            armys: vec![
                Army {
                    troops: (0..MAX_TROOPS).map(|_| Troop {
                        was_payed: true,
                        is_free: false,
                        is_main: false,
                        custom_name: None,
                        unit: units[&rng.gen_range(1..100)].clone()
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
                    troops: (0..MAX_TROOPS).map(|_| Troop {
                        was_payed: true,
                        is_free: false,
                        is_main: false,
                        custom_name: None,
                        unit: units[&rng.gen_range(1..100)].clone()
                    }.into()).collect::<Vec<_>>(),
                    stats: ArmyStats {
                        gold: 0,
                        mana: 0,
                        army_name: "pidoras".to_string()
                    },
                    inventory: vec![],
                    pos: [0, 0]
                }
            ]
        },
        units,
        menu_id: 0,
        menu_data: HashMap::new(),
        assets: my_assets,
        battle: BattleInfo {
            army1: 0,
            army2: 1,
            battle_ter: 0,
            active_unit: None
        }
    };
    state.battle.active_unit = state.battle.search_next_active(&state);
    dbg!(state.battle.active_unit);
    state
}
fn draw(app: &mut App, assets: &mut Assets, gfx: &mut Graphics, plugins: &mut Plugins, state: &mut State) {
    state.draw = gfx.create_draw();
    get_mut::<State, Draw>(state).clear(Color::WHITE);
    forms.lock().unwrap().get_mut(&state.menu_id).unwrap()
        .iter_mut().for_each(|form| form.draw(app, assets, gfx, plugins, state));
    gfx.render(get_mut::<State, Draw>(state));
    state.frame+=1;
}
fn update(app: &mut App, assets: &mut Assets, plugins: &mut Plugins, state: &mut State) {
    forms.lock().unwrap().get_mut(&state.menu_id).unwrap()
        .iter_mut().for_each(|form| form.after(app, assets, plugins, state));
}
#[notan_main]
fn main() -> Result<(), String> {
    dbg!(size_of::<SingleContainer<State, SliderContainer<State, TextChain<State>, SingleContainer<State, Data<State, &str, f32>>>>>());
    let win = WindowConfig::new()
        .title("Discord Times: Remastered")
        .vsync(true)
        .lazy_loop(true)
        .high_dpi(true)
        .fullscreen(true)
        .size(1600, 1200);
    notan::init_with(setup)
        .add_config(win)
        .add_config(TextConfig)
        .add_config(DrawConfig)
        .draw(draw)
        .update(update)
        .build()
}
