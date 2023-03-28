mod lib;

use crate::lib::{
    battle::{
        army::{find_path, Army, ArmyStats, TroopType, MAX_TROOPS},
        troop::Troop,
    },
    parse::{parse_items, parse_locale, parse_objects, parse_settings, parse_units, Locale},
    units::{
        unit::{Unit, UnitPos},
        unitstats::ModifyUnitStats,
    },
};
use lib::{
    battle::battlefield::*,
    items::item::*,
    map::{map::*, object::ObjectInfo, tile::*},
    time::time::Time,
};
use notan::{
    draw::*,
    prelude::*,
    text::TextConfig,
};
use notan_ui::{
    containers::*,
    defs::*,
    form::Form,
    forms::*,
    rect::*,
    text::*,
    wrappers::*,
};
use num::clamp;
use once_cell::sync::Lazy;
use parking_lot::MappedRwLockReadGuard;
use rand::{
    prelude::ThreadRng,
    thread_rng, Rng,
};
use std::{
    array::from_fn,
    collections::HashMap,
    fmt::Debug,
    fs::read_dir,
    mem::size_of,
};
use tracing_mutex::stdsync::TracingMutex as Mutex;
use worldgen::{
    constraint,
    noise::perlin::*,
    noisemap::*,
    world::{
        tile::{Tile as GenTile, *},
        Size as GenSize, *,
    },
};
use crate::lib::battle::army::Relations;

#[derive(Clone, Debug)]
pub enum Value {
    Num(i64),
    Str(String),
}

#[derive(Clone, Debug, AppState)]
pub struct State {
    pub fonts: Vec<Font>,
    pub frame: usize,
    pub draw: Draw,
    pub shaders: Vec<(Pipeline, Buffer)>,
    pub gamemap: GameMap,
    pub units: HashMap<usize, Unit>,
    pub objects: HashMap<usize, ObjectInfo>,
    pub assets: HashMap<&'static str, HashMap<String, Asset<Texture>>>,
    pub menu_data: HashMap<&'static str, Value>,
    pub menu_id: usize,
    pub battle: BattleInfo,
}
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
impl State {
    pub fn get_texture(&self, dir: &str, img: &str) -> MappedRwLockReadGuard<'_, Texture> {
        self.assets
            .get(dir)
            .expect(&*format!("No such dir - {dir}"))
            .get(img)
            .expect(&*format!("No such img - {img}"))
            .lock()
            .unwrap()
    }
}


fn get_menu_value_str(state: &State, id: &'static str) -> Option<String> {
    match state.menu_data.get(id) {
        Some(Value::Str(string)) => Some(string.clone()),
        _ => None,
    }
}
fn get_menu_value_num(state: &State, id: &'static str) -> Option<i64> {
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
    justtext: impl Into<String>,
    on_draw: fn(
        &mut SingleContainer<State, Button<State, Text<State>>>,
        &mut App,
        &mut Assets,
        &mut Graphics,
        &mut Plugins,
        &mut State,
    ),
    if_clicked: fn(
        &mut Button<State, Text<State>>,
        &mut App,
        &mut Assets,
        &mut Plugins,
        &mut State,
    ),
) -> Box<SingleContainer<State, Button<State, Text<State>>>> {
    Box::new(
        single(
            button(
                text(justtext)
                .align_v(AlignVertical::Center)
                .align_h(AlignHorizontal::Center)
                .color(Color::WHITE)
                .size(30.)
                .pos(Position(150., 25.))
                .build().unwrap(),
                Rect {
                    pos: Position(-150., 0.),
                    size: Size(300., 50.)
                }
            )
            .if_clicked(if_clicked)
            .build().unwrap()
        )
        .on_draw(on_draw)
        .pos(Position(0., -25.))
        .build().unwrap()
    )
}

static FORMS: Lazy<Mutex<HashMap<usize, SingleContainer<State, DynContainer<State>>>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));
static LOCALE: Lazy<Mutex<Locale>> = Lazy::new(|| Mutex::new(Locale::new()));
static MONITOR_SIZE: Lazy<Mutex<(f32, f32)>> = Lazy::new(|| Mutex::new((0., 0.)));

#[repr(u32)]
enum Menu {
    Main,
    Start,
    Load,
    Settings,
    Authors,
    UnitView,
    JustRectangle,
    Battle,
    Items,
}

fn gen_forms(gfx: &mut Graphics) -> Result<(), String> {
    let draw_back: for<'a, 'b, 'c, 'd, 'e, 'f> fn(
        &'a mut SingleContainer<State, Button<State, Text<State>>>,
        &'b mut App,
        &'f mut Assets,
        &'c mut Graphics,
        &'d mut Plugins,
        &'e mut State,
    ) = |container, _app, _assets, _gfx, _plugins, state: &mut State| {
        get_mut::<State, Draw>(state)
            .rect(
                (container.pos - Position(container.get_size().0 / 2., 0.)).into(),
                container.get_size().into(),
            )
            .color(Color::from_hex(0x033121ff));
    };
	fn redirect_menu<T: PosForm<State>, const MENU: usize>(_: &mut T, _: &mut App, _: &mut Assets, _: &mut Plugins, state: &mut State) {
		state.menu_id = MENU;
	}
    let size = gfx.device.size();
    *MONITOR_SIZE.lock().unwrap() = (size.0 as f32, size.1 as f32);
	let half_size = (size.0 as f32 / 2., size.1 as f32 / 2.);
    let mut hashmap = FORMS.lock().unwrap();
    let locale = LOCALE.lock().unwrap();
    let max_troops = *MAX_TROOPS.lock().unwrap();
    let half_troops = max_troops / 2;
    let nav_button_rect = Rect {
        pos: Position(100., 0.),
        size: Size(70., 100.),
    };
    let nav_button_text = |text: String| {
        Text {
            text,
            font: FontId(0),
            align_h: AlignHorizontal::Left,
            align_v: AlignVertical::Bottom,
            pos: Position(0., 0.),
            size: 10.,
            rect_size: None,
            max_width: None,
            color: Color::BLACK,
            boo: std::marker::PhantomData,
        }
    };
    hashmap.insert(
        Menu::Main as usize,
        SingleContainerBuilder::default()
            .inside(
                DynContainerBuilder::default()
                    .inside(vec![
                        Box::new(
                            TextBuilder::default()
                                .text(locale.get("menu_game_name"))
                                .align_h(AlignHorizontal::Center)
                                .size(70.0)
                                .build()?,
                        ) as Box<dyn ObjPosForm<State>>,
                        menu_button(
                            locale.get("menu_start_title"),
                            draw_back,
                            redirect_menu::<_, {Menu::Start as usize}>,
                        ),
                        menu_button(
                            locale.get("menu_battle_title"),
                            draw_back,
                            redirect_menu::<_, {Menu::Battle as usize}>
                        ),
                        menu_button(
                            locale.get("menu_load_title"),
                            draw_back,
                            redirect_menu::<_, {Menu::Load as usize}>
                        ),
                        menu_button(
                            locale.get("menu_settings_title"),
                            draw_back,
                            redirect_menu::<_, {Menu::Settings as usize}>
                        ),
                        menu_button(
                            locale.get("menu_authors_title"),
                            draw_back,
                            redirect_menu::<_, {Menu::Authors as usize}>
                        ),
                        menu_button(
                            locale.get("menu_unitview_title"),
                            draw_back,
                            redirect_menu::<_, {Menu::UnitView as usize}> 
                        ),
                        menu_button(
                            locale.get("menu_items_title"),
                            draw_back,
                            redirect_menu::<_, {Menu::Items as usize}>
                        ),
                        menu_button(
                            locale.get("menu_exit_title"),
                            draw_back,
                            |_container, app, _assets, _plugins, _state: &mut State| app.exit(),
                        ),
                    ])
                    .pos(Position(half_size.0, 0.))
                    .align_direction(Direction::Bottom)
                    .interval(Position(0., 20.))
                    .build()?,
            )
            .on_draw(|_container, _app, _assets, _gfx, _plugins, state: &mut State| {
                get_mut::<State, Draw>(state)
                    .rect((0., 0.), *MONITOR_SIZE.lock().unwrap())
                    .color(Color::ORANGE);
            })
            .build()?
    );
    fn items_unit_card_draw(
        drawing: &mut Drawing<State>,
        _: &mut App,
        _: &mut Assets,
        gfx: &mut Graphics,
        _: &mut Plugins,
        state: &mut State,
        army: usize,
        index: usize,
    ) {
        let draw = get_mut::<State, Draw>(state);
        gfx.render(draw);
        let pos = drawing.pos;
        let mut draw = gfx.create_draw();
        let index = (pos.0 / 102.) as usize + index;
        let troop = state.gamemap.armys[army].troops[index].get();
        if let Some(troop) = troop.as_ref() {
            let unit = &troop.unit;
            let texture = &state.get_texture(
                "assets/Icons",
                &*format!("img_{}.png", unit.info.icon_index),
            );
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
        }
        state.draw = draw;
    }
    hashmap.insert(
        Menu::Items as usize,
        SingleContainerBuilder::default()
             .inside(DynContainerBuilder::default()
                .inside(vec![Box::new(StraightDynContainerBuilder::default()
                    .inside(vec![
                        Box::new(TupleContainerBuilder::default()
                            .inside(
                                (
                                    container(vec![
                                        container((0..half_troops).map(|_| {
                                            button(Drawing {
                                                    pos: Position(0., 0.),
                                                    to_draw: |drawing, app, assets, gfx, plugins, state| {
                                                        items_unit_card_draw(drawing, app, assets, gfx, plugins, state, 0, 0 * *MAX_TROOPS.lock().unwrap() / 2);
                                                    }
                                                },
                                                Rect { pos: Position(0., 0.), size: Size(92., 92.) }
                                            ).if_clicked(|button, _app, _assets, _plugins, state| {
                                                let pos = button.rect.pos;
                                                let index = (pos.0 / 102.) as usize + 0 * *MAX_TROOPS.lock().unwrap() / 2;
                                                let army = 0;
                                                set_menu_value_num(state, "items_unit_stat_changed", 1);
                                                set_menu_value_num(state, "items_unit_stat_index", index as i64);
                                                set_menu_value_num(state, "items_unit_stat_army", army);
                                            })
                                            .build().unwrap()
                                        }).collect::<Vec<_>>())
                                        .interval(Position(10., 0.))
                                        .build().unwrap(),
                                        container((0..half_troops).map(|_| {
                                            button(Drawing {
                                                pos: Position(0., 0.),
                                                to_draw: |drawing, app, assets, gfx, plugins, state| {
                                                    items_unit_card_draw(drawing, app, assets, gfx, plugins, state, 0, 1 * *MAX_TROOPS.lock().unwrap() / 2);
                                                }
                                            },
                                               Rect { pos: Position(0., 0.), size: Size(92., 92.) }
                                            ).if_clicked(|button, _app, _assets, _plugins, state| {
                                                let pos = button.rect.pos;
                                                let index = (pos.0 / 102.) as usize + 1 * *MAX_TROOPS.lock().unwrap() / 2;
                                                let army = 0;
                                                set_menu_value_num(state, "items_unit_stat_changed", 1);
                                                set_menu_value_num(state, "items_unit_stat_index", index as i64);
                                                set_menu_value_num(state, "items_unit_stat_army", army);
                                            })
                                            .build().unwrap()
                                        }).collect::<Vec<_>>())
                                        .interval(Position(10., 0.))
                                        .build().unwrap(),
                                    ])
                                    .align_direction(Direction::Bottom)
                                    .interval(Position(0., 50.))
                                    .build()?,
                                    container(vec![
                                        container((0..half_troops).map(|_| {
                                            button(Drawing {
                                                    pos: Position(0., 0.),
                                                    to_draw: |drawing, app, assets, gfx, plugins, state| {
                                                        items_unit_card_draw(drawing, app, assets, gfx, plugins, state, 1, 1 * *MAX_TROOPS.lock().unwrap() / 2);
                                                    }
                                                },
                                                Rect { pos: Position(0., 0.), size: Size(92., 92.) }
                                            ).if_clicked(|button, app, assets, plugins, state| {
                                                let pos = button.rect.pos;
                                                let index = (pos.0 / 102.) as usize + 1 * *MAX_TROOPS.lock().unwrap() / 2;
                                                let army = 1;
                                                set_menu_value_num(state, "items_unit_stat_changed", 1);
                                                set_menu_value_num(state, "items_unit_stat_index", index as i64);
                                                set_menu_value_num(state, "items_unit_stat_army", army);
                                            })
                                            .build().unwrap()
                                        }).collect::<Vec<_>>())
                                        .interval(Position(10., 0.))
                                        .build().unwrap(),
                                        container((0..half_troops).map(|_| {
                                            button(Drawing {
                                                pos: Position(0., 0.),
                                                to_draw: |drawing, app, assets, gfx, plugins, state| {
                                                    items_unit_card_draw(drawing, app, assets, gfx, plugins, state, 1, 0 * *MAX_TROOPS.lock().unwrap() / 2);
                                                }
                                            },
                                                   Rect { pos: Position(0., 0.), size: Size(92., 92.) }
                                            ).if_clicked(|button, _app, _assets, _plugins, state| {
                                                let pos = button.rect.pos;
                                                let index = (pos.0 / 102.) as usize + 0 * *MAX_TROOPS.lock().unwrap() / 2;
                                                let army = 1;
                                                set_menu_value_num(state, "items_unit_stat_changed", 1);
                                                set_menu_value_num(state, "items_unit_stat_index", index as i64);
                                                set_menu_value_num(state, "items_unit_stat_army", army);
                                            })
                                            .build().unwrap()
                                        }).collect::<Vec<_>>())
                                        .interval(Position(10., 0.))
                                        .build().unwrap(),
                                    ])
                                    .align_direction(Direction::Bottom)
                                    .interval(Position(0., 50.))
                                    .build()?,
                            )   )
                            .interval(Position(0., 300.))
                            .align_direction(Direction::Bottom)
                            .pos(Position(0., 0.))
                            .build()?
                        ),
                        Box::new(DrawingBuilder::default()
                            .to_draw(|drawing: &mut Drawing<State>, app, assets, gfx, plugins, state: &mut State| {
                                if let Some(item_index) = get_menu_value_num(state, "items_item_index") {
                                    let draw = get_mut::<State, Draw>(state);
                                    gfx.render(draw);
                                    let pos = drawing.pos;
                                    let mut draw = gfx.create_draw();
                                    draw.image(&state.get_texture("assets/Items", &*ITEMS.lock().unwrap().get(&(item_index as usize)).as_ref().expect(&*item_index.to_string()).icon))
                                        .position(pos.0, pos.1);
                                    state.draw = draw;
                                }
                            })
                            .pos(Position(1150., 40.))
                            .build()?
                        ),
                        Box::new(ButtonBuilder::default()
                            .inside(TextBuilder::default()
                                .text("Пред.")
                                .pos(Position(0., 50.))
                                .align_v(AlignVertical::Bottom)
                                .size(20.)
                                .build()?
                            )
                            .rect(Rect { pos: Position(1100., 40.), size: Size(100., 50.) })
                            .if_clicked(|button: &mut Button<State, Text<State>>, app, assets, plugins, state| {
                                set_menu_value_num(state, "items_item_index",
                                                   clamp(get_menu_value_num(state, "items_item_index").unwrap_or(0) - 1, 1, 166));
                            })
                            .build()?
                        ),
                        Box::new(ButtonBuilder::default()
                            .inside(TextBuilder::default()
                                .text("След.")
                                .pos(Position(0., 50.))
                                .align_v(AlignVertical::Bottom)
                                .size(20.)
                                .build()?
                            )
                            .rect(Rect { pos: Position(1300., 40.), size: Size(100., 50.) })
                            .if_clicked(|button: &mut Button<State, Text<State>>, app, assets, plugins, state| {
                                set_menu_value_num(state, "items_item_index",
                                                   clamp(get_menu_value_num(state, "items_item_index").unwrap_or(0) + 1, 1, 166));
                            })
                            .build()?
                        ),
                        Box::new(
                            SingleContainerBuilder::default()
                                .inside(DrawingBuilder::<State>::default()
                                        .to_draw(|drawing, app, assets, gfx, plugins, state| {
                                            let draw = get_mut::<State, Draw>(state);
                                            gfx.render(draw);
                                            let pos = drawing.pos;
                                            let mut draw = gfx.create_draw();
                                            draw.image(&state.get_texture("assets/Icons", &*format!("img_{}.png", get_menu_value_num(state, "items_unit_stat_icon").unwrap_or(1))))
                                                .position(pos.0, pos.1);
                                            state.draw = draw;
                                        })
                                        .pos(Position(900., 100.))
                                        .build()?
                                )
                                .build()?
                        ),
                        Box::new(
                            single(text("Статистика")
                                   .size(20.0)
                                   .max_width(MONITOR_SIZE.lock().unwrap().0 - 900.)
                                .build()?
                            )
                            .after_draw(|container, app, assets, plugins, state: &mut State| {
                                if app.keyboard.is_down(KeyCode::Escape) { state.menu_id = Menu::Main as usize; }
                                if get_menu_value_num(state, "items_unit_stat_changed").unwrap_or(0) == 1 {
                                    let index = get_menu_value_num(state, "items_unit_stat_index").unwrap_or(1) as usize;
                                    let army = get_menu_value_num(state, "items_unit_stat_army").unwrap_or(0) as usize;
                                    match &mut container.inside {
                                        Some::<Text<State>>(text) => {
                                            if let Some(troop) = &(state.gamemap.armys[army as usize]).get_troop(index) {
                                                if let Some(troop) = &*troop.get() {
                                                    container.inside.as_mut().unwrap().text = troop.unit.to_string();
                                                    set_menu_value_num(state, "items_unit_stat_icon", troop.unit.info.icon_index as i64);
                                                }
                                            };
                                            set_menu_value_num(state, "items_unit_stat_changed", 0);
                                        }
                                        None => {}
                                }   }
                            })
                            .pos(Position(900., 200.))
                            .build()?
                        ),
                        Box::new(
                            button(DrawingBuilder::default()
                                .to_draw(|drawing, app, assets, gfx, plugins, state: &mut State| {
                                    let draw = get_mut::<State, Draw>(state);
                                    gfx.render(draw);
                                    let mut draw = gfx.create_draw();
                                    let pos = drawing.pos;
                                    if let (Some(army), Some(index)) = (get_menu_value_num(state, "items_unit_stat_army"), get_menu_value_num(state, "items_unit_stat_index")) {
                                        if let Some(troop) = state.gamemap.armys[army as usize].get_troop(index as usize) {
                                            if let Some(troop) = &*troop.get() {
                                                for i in 0..4 {
                                                    draw.rect((pos.0 + (53. + 5.) * i as f32, pos.1), (53., 53.))
                                                            .stroke_color(Color::BLACK)
                                                            .stroke(5.);
                                                    if let Some(item) = &troop.unit.inventory.items[i] {
                                                        let texture = state.get_texture("assets/Items", &*item.get_info().icon);
                                                        draw.image(&texture)
                                                            .position(pos.0 + (53. + 5.) * i as f32, pos.1);
                                                    }
                                                }
                                            }
                                        }
                                    }
                                    state.draw = draw;
                                })
                                .build()?,
                                Rect { pos:Position(1000., 147.), size: Size(212., 53.)}
                            ).if_clicked(|button, app, assets, plugins, state: &mut State| {
                                let slot = ((app.mouse.position().0 - 1. - button.rect.pos.0)/53.) as usize;
                                if let (Some(army), Some(index)) = (get_menu_value_num(state, "items_unit_stat_army"), get_menu_value_num(state, "items_unit_stat_index")) {
                                    if let Some(troop) = state.gamemap.armys[army as usize].get_troop(index as usize) {
                                        if let Some(troop) = &mut *troop.get() {
                                            if troop.unit.inventory.items[slot].is_some() {
                                                troop.unit.remove_item(slot);
                                                set_menu_value_num(state, "items_unit_stat_changed", 1);
                                                return;
                                            }
                                            if let Some(item_index) = get_menu_value_num(state, "items_item_index") {
                                                troop.unit.add_item(Item { index: item_index as usize }.into(), slot);
                                                set_menu_value_num(state, "items_unit_stat_changed", 1);
                                            }
                                        }
                                    }
                                }
                            })
                            .build()?
                        )
                    ])
                    .build()?,
                )])
                .build()?
             )
             .on_draw(|container, app, assets, gfx, plugins, state: &mut State| {
                get_mut::<State, Draw>(state)
                    .rect((0., 0.), *MONITOR_SIZE.lock().unwrap())
                    .color(Color::ORANGE);
             })
             .after_draw(|container, app, assets, plugins, state: &mut State| {
                 if app.keyboard.is_down(KeyCode::Escape) {
                     state.menu_id = Menu::Main as usize;
                 }
             })
             .build()?
    );
    hashmap.insert(
        Menu::Settings as usize,
        single(
            dyn_cont(
                vec![
                    Box::new(
                        TupleContainerBuilder::default()
                            .inside((
                                checkbox(
                                    text("+")
                                        .align_h(AlignHorizontal::Left)
                                        .size(50.0)
                                        .color(Color::ORANGE)
                                        .build()?,
                                    Rect {
                                        pos: Position(0., 0.),
                                        size: Size(50., 50.),
                                    })
                                    .on_draw(
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
                                    )
                                    .pos(Position(20., 0.))
                                    .build()?,
                                text(locale.get("settings_rect_rotate_type"))
                                    .pos(Position(20., 0.))
                                    .size(20.0)
                                    .build()?,
                                button(
                                    text(locale.get("settings_regenerate_map"))
                                        .size(20.)
                                        .build()?,
                                    Rect {
                                        pos: Position(0., 100.),
                                        size: Size(200., 50.)
                                    }
                                )
                                    .if_clicked(|button: &mut Button<State, Text<State>>, app, assets, plugins, state: &mut State| {
                                        let tilemap = gen_tilemap();
                                        let decomap = gen_decomap(tilemap.1, *state.objects.iter().find(|obj| obj.1.path == "Tree0.png").unwrap().0);
                                        state.gamemap.tilemap = tilemap.0;
                                        state.gamemap.decomap = decomap;
                                    })
                                    .build()?
                            ))
                            .build()?
                        )
                    ]
                ).build()?
            ).on_draw(|container, app, assets, gfx, plugins, state: &mut State| {
                get_mut::<State, Draw>(state)
                    .rect((0., 0.), *MONITOR_SIZE.lock().unwrap())
                    .color(Color::ORANGE);
            })
            .after_draw(|container, app, assets, plugins, state: &mut State| {
                if app.keyboard.is_down(KeyCode::Escape) {
                    state.menu_id = Menu::Main as usize;
                }
            })
            .pos(Position(0., 0.))
            .build()?
    );
    let rect_draw: for<'a, 'b, 'c, 'd, 'e, 'f> fn(
        &'a mut SingleContainer<State, Text<State>>,
        &'b mut App,
        &'f mut Assets,
        &'c mut Graphics,
        &'d mut Plugins,
        &'e mut State,
    ) = |drawing, app, assets, gfx, plugins, state: &mut State| {
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
                    .rect(pos.into(), (8., 8.))
                    .rotate_degrees_from(center.into(), deg)
                    .color(color);
            }
        }
    };
    let rect_handle: for<'a, 'b, 'd, 'e, 'f> fn(
        &'a mut SingleContainer<State, Text<State>>,
        &'b mut App,
        &'f mut Assets,
        &'d mut Plugins,
        &'e mut State,
    ) = |container, app, assets, plugins, state: &mut State| {
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
    };
    hashmap.insert(
        Menu::JustRectangle as usize,
        SingleContainerBuilder::default()
            .inside(
                DynContainerBuilder::default()
                    .inside(vec![Box::new(
                        SingleContainerBuilder::default()
                            .on_draw(rect_draw)
                            .after_draw(rect_handle)
                            .build()?,
                    )])
                    .align_direction(Direction::Bottom)
                    .interval(Position(0., 50.))
                    .build()?,
            )
            .on_draw(|container, app, assets, gfx, plugins, state: &mut State| {
                get_mut::<State, Draw>(state)
                    .rect((0., 0.), *MONITOR_SIZE.lock().unwrap())
                    .color(Color::ORANGE);
            })
            .build()?
    );
    fn draw_player_stats(drawing: &mut SingleContainer<State, Drawing<State>>, app: &mut App, gfx: &mut Graphics, state: &mut State) {
        let mut draw = get_mut::<State, Draw>(state);
        gfx.render(draw);
        let mut draw = gfx.create_draw();
        let monitor_size = MONITOR_SIZE.lock().unwrap();
        draw.rect((0., monitor_size.1), (monitor_size.0, -100.))
            .color(Color::from_hex(0x033121ff));
        let army = &state.gamemap.armys[0];
        draw.text(&state.fonts[0], &*format!("Золото: {}", army.stats.gold))
            .position(10., monitor_size.1 - 50.)
            .v_align_middle()
            .size(20.)
            .color(Color::YELLOW);
        draw.text(&state.fonts[0], &*format!("Мана: {}", army.stats.mana))
            .position(200., monitor_size.1 - 50.)
            .v_align_middle()
            .size(20.)
            .color(Color::BLUE);
        state.draw = draw;
    }
    const SIZE: (f32, f32) = (52., 40.);
    hashmap.insert(
        Menu::Start as usize,
        single(
            dyn_cont(
                vec![
                    Box::new(
                        TupleContainerBuilder::default()
                            .inside(
                                (
                                    SingleContainer {
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
                                                let terrain = state.assets.get("assets/Terrain").unwrap();
                                                for i in 0..MAP_SIZE {
                                                    for j in 0..MAP_SIZE {
                                                        let asset = terrain
                                                            .get(TILES[state.gamemap.tilemap[i][j]].sprite())
                                                            .unwrap();
                                                        let texture = asset.lock().unwrap();

                                                        let pos = Position(i as f32 * SIZE.0, j as f32 * SIZE.1);
                                                        draw.image(&texture)
                                                            .position(pos.0, pos.1)
                                                            .size(SIZE.0, SIZE.1);
                                                        if state.gamemap.armys[0].path.contains(&(i, j)) {
                                                            draw.rect((pos).into(), (10., 10.)).color(Color::RED);
                                                        }
                                                        if state.gamemap.hitmap[i][j].army.is_some() {
                                                            draw.rect((pos).into(), (10., 10.)).color(Color::BLUE);
                                                        }

                                                    }
                                                }
                                                for i in 0..0 {//MAP_SIZE {
                                                    for j in 0..MAP_SIZE {
                                                        let asset = terrain
                                                            .get(TILES[state.gamemap.tilemap[i][j]].sprite())
                                                            .unwrap();
                                                        let texture = asset.lock().unwrap();

                                                        const ALPHA: f32 = 0.2;
                                                        const W_SIZE_QUARTER: f32 = SIZE.0 / 4.;
                                                        const H_SIZE_QUARTER: f32 = SIZE.1 / 4.;
                                                        const QUARTER_TILE: f32 = 106. / 4.;
                                                        const THREE_QUARTERS: f32 = QUARTER_TILE * 3.;
                                                        let pos = Position(i as f32 * SIZE.0, j as f32 * SIZE.1);
                                                        (0..4).for_each(|i| {
                                                            let (quarter_size, crop_start, cropped_size, quarter_pos) =
                                                                match i {
                                                                    0 => (
                                                                        (W_SIZE_QUARTER, SIZE.1),
                                                                        (THREE_QUARTERS, 0.),
                                                                        (QUARTER_TILE, 106.),
                                                                        (pos.0 - W_SIZE_QUARTER, pos.1),
                                                                    ),
                                                                    1 => (
                                                                        (W_SIZE_QUARTER, SIZE.1),
                                                                        (0., 0.),
                                                                        (QUARTER_TILE, 106.),
                                                                        (pos.0 + W_SIZE_QUARTER, pos.1),
                                                                    ),
                                                                    2 => (
                                                                        (SIZE.0, H_SIZE_QUARTER),
                                                                        (0., THREE_QUARTERS),
                                                                        (106., QUARTER_TILE),
                                                                        (pos.0, pos.1 - H_SIZE_QUARTER),
                                                                    ),
                                                                    3 => (
                                                                        (SIZE.0, H_SIZE_QUARTER),
                                                                        (0., 0.),
                                                                        (106., QUARTER_TILE),
                                                                        (pos.0, pos.1 + H_SIZE_QUARTER),
                                                                    ),
                                                                    _ => ((0., 0.), (0., 0.), (0., 0.), (0., 0.)),
                                                                };
                                                            gfx.set_buffer_data(&state.shaders[0].1, &[i % 2]);
                                                            draw.image_pipeline()
                                                                .pipeline(&state.shaders[0].0)
                                                                .uniform_buffer(&state.shaders[0].1);
                                                            draw.image(&texture)
                                                                .position(quarter_pos.0, quarter_pos.1)
                                                                .crop(crop_start, cropped_size)
                                                                .size(quarter_size.0, quarter_size.1)
                                                                .alpha(ALPHA);
                                                        });
                                                        draw.image_pipeline().remove();
                                                    }
                                                }
                                                let objects = state.assets.get("assets/Objects").unwrap();
                                                for i in 0..MAP_SIZE {
                                                    for j in 0..MAP_SIZE {
                                                        if let Some(index) = state.gamemap.decomap[i][j] {
                                                            let asset = objects
                                                                .get(&state.objects[&index].path)
                                                                .expect(&*format!("{}", &state.objects[&index].path));
                                                            let texture = asset.lock().unwrap();
                                                            let size = state.objects[&index].size;
                                                            let pos = Position(i as f32 * SIZE.0, j as f32 * SIZE.1);
                                                            draw.image(&texture)
                                                                .position(pos.0, pos.1 - (size.1 as f32 - 1.) * SIZE.1)
                                                                .size(SIZE.0 * size.0 as f32, SIZE.1 * size.1 as f32);
                                                        }
                                                    }
                                                }
                                                state.draw = draw;
                                                draw_player_stats(drawing, app, gfx, state);
                                            },
                                        ),
                                        after_draw: Some(|container, app, assets, plugins, state: &mut State| {
                                            if app.keyboard.is_down(KeyCode::Escape) {
                                                state.menu_id = Menu::Main as usize;
                                            }
                                            let rect = Rect {
                                                pos: container.pos,
                                                size: (SIZE.0 * MAP_SIZE as f32, SIZE.1 * MAP_SIZE as f32).into(),
                                            };
                                            if rect.collides((app.mouse.x, app.mouse.y).into())
                                                && app.mouse.left_was_released()
                                            {
                                                let clicked_at = app.mouse.position();
                                                let start = state.gamemap.armys[0].pos;
                                                let goal = (
                                                    (clicked_at.0 / SIZE.0) as usize,
                                                    (clicked_at.1 / SIZE.1) as usize,
                                                );
                                                let mut army = &mut state.gamemap.armys[0];
                                                if army.path.contains(&goal) {
                                                    army.pos = army.path.remove(0);
                                                    state.gamemap.time.minutes += 10;
                                                    state.gamemap.recalc_armies_hitboxes();
                                                }
                                                let path = find_path(&state.gamemap, start, goal, false);
                                                state.gamemap.armys[0].path = if let Some(path) = path {
                                                    path.0
                                                } else {
                                                    Vec::new()
                                                };
                                            }
                                        }),
                                        pos: Position(0., 0.),
                                    },
                                ),
                            )
                            .build()?,
                    )
                ]
            ).build()?)
            .on_draw(|container, app, assets, gfx, plugins, state: &mut State| {
                get_mut::<State, Draw>(state)
                    .rect((0., 0.), *MONITOR_SIZE.lock().unwrap())
                    .color(Color::ORANGE);
            })
            .build()?,
    );
    hashmap.insert(
        Menu::UnitView as usize,
        SingleContainer {
            inside: Some(DynContainer {
                inside: vec![Box::new(
                    TupleContainerBuilder::default().inside((
                        Drawing {
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
                        },
                        container(vec![
                            button(nav_button_text("Пред.".into()), nav_button_rect)
                            .if_clicked(
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
                            )
                            .build()?,
                            button(nav_button_text("След.".into()), nav_button_rect)
                            .if_clicked(
                                |button, app, assets, plugins, state: &mut State| {
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
                                            boo: std::marker::PhantomData::<State>,
                                        })
                                    };
                                    set_menu_value_num(state, "char_view_changed", 1);
                                },
                            )
                            .build()?
                        ])
                        .interval(Position(150., 0.))
                        .build()?,
                        single(text("АБОБА")
                            .size(20.0)
                            .build()?
                        )
                        .after_draw(
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
                        )
                        .pos(Position(0., 200.))
                        .build()?
                    ))
                    .build()?
                )],
                ..Default::default()
            }),
            on_draw: Some(|container, app, assets, gfx, plugins, state: &mut State| {
                get_mut::<State, Draw>(state)
                    .rect((0., 0.), *MONITOR_SIZE.lock().unwrap())
                    .color(Color::ORANGE);
            }),
            after_draw: None,
            pos: Position(0., 0.),
        }
    );
    fn draw_unit_info(unit: &Unit, pos: Position, state: &State, draw: &mut Draw) {
        let stats = &unit.modified;
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
            let stats = unit.modified;
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
                    -((1. - (stats.hp as f64 / stats.max_hp as f64) as f32) * 92.),
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
	fn move_thing(state: &mut State) {
		state.battle.active_unit = state.battle.search_next_active(&*state);
		if state.battle.active_unit == None {
			println!("next_move start");
            next_move(state);
			println!("next_move done");
            state.battle.active_unit = state.battle.search_next_active(&*state);
        }
        state.battle.can_interact = search_interactions(state);
	}
    fn unit_card_clicked(state: &mut State, army: usize, index: usize) {
        let mut unit_index = u64::MAX;
        if let Some(troop) = state.gamemap.armys[army].troops[index].get().as_mut() {
            unit_index = troop.unit.info.icon_index as u64 + 1;
        }
        if !(unit_index == u64::MAX) {
            set_menu_value_num(state, "battle_unit_stat", unit_index as i64);
            set_menu_value_num(state, "battle_unit_stat_changed", 1);
        }
        if let Some(active_unit) = state.battle.active_unit {
            let mut moves;
            let active_is_dead;
            {
                let mut troop1 = state.gamemap.armys[active_unit.0].troops[active_unit.1].get();
                let mut unit1 = troop1.as_mut().unwrap();
                moves = unit1.unit.modified.moves;
                active_is_dead = unit1.unit.is_dead();
                if moves == 0 || active_is_dead {
					
                }
                else if !(active_unit == (army, index)) {
                    let mut troop2 = state.gamemap.armys[army].troops[index].get();
                    if let Some(troop) = troop2.as_mut() {
                        let unit2 = &mut troop.unit;
                        if !unit2.is_dead() {
                            if unit1.unit.modified.moves != 0 {
                                if unit1.unit.attack(
                                    unit2,
                                    UnitPos::from_index(index),
                                    UnitPos::from_index(active_unit.1),
                                ) {
                                    unit1.unit.stats.moves -= 1;
                                    unit1.unit.recalc();
                                    moves = unit1.unit.modified.moves;
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
                            .insert(active_unit.1 + add_index, None.into());
                        state.gamemap.armys[army].troops.insert(index, troop);
                        moves = 0;
                    }
                } else {
                    unit1.unit.stats.moves -= 1;
                    unit1.unit.recalc();
                }
            }
            if moves == 0 || active_is_dead {
                move_thing(state);
            }
        }
    }
    hashmap.insert(Menu::Battle as usize, SingleContainer {
        inside: Some(DynContainer {
            inside: vec![
                Box::new(StraightDynContainer {
                    inside: vec![
                        Box::new(single(container(vec![
                                    container((0..half_troops).map(|_| {
                                        button(
                                            Drawing {
                                                pos: Position(0., 0.),
                                                to_draw: |drawing, app, assets, gfx, plugins, state| {
                                                    unit_card_draw(drawing, app, assets, gfx, plugins, state, 0, 0 * *MAX_TROOPS.lock().unwrap() / 2);
                                                }
                                            },
                                            Rect { pos: Position(0., 0.), size: Size(92., 92.) }
                                        ).if_clicked(|button, _app, _assets, _plugins, state| {
                                            let pos = button.rect.pos;
                                            let army = 0;
                                            let index = (pos.0 / 102.) as usize + 0 * *MAX_TROOPS.lock().unwrap() / 2;
                                            unit_card_clicked(state, army, index);
                                        }).build().unwrap()
                                    }).collect::<Vec<_>>())
                                        .interval(Position(10., 0.))
                                        .build()?,
                                    container((0..half_troops).map(|_| {
                                        button(
                                            Drawing {
                                                pos: Position(0., 0.),
                                                to_draw: |drawing, app, assets, gfx, plugins, state| {
                                                    unit_card_draw(drawing, app, assets, gfx, plugins, state, 0, *MAX_TROOPS.lock().unwrap() / 2);
                                                }
                                            },
                                            Rect { pos: Position(0., 0.), size: Size(92., 92.) }
                                        ).if_clicked(|button, _app, _assets, _plugins, state| {
                                            let pos = button.rect.pos;
                                            let index = (pos.0 / 102.) as usize + *MAX_TROOPS.lock().unwrap() / 2;
                                            let army = 0;
                                            unit_card_clicked(state, army, index);
                                        })
                                            .build().unwrap()
                                    }).collect::<Vec<_>>())
                                        .interval(Position(10., 0.))
                                        .build()?
                                ])
                                .align_direction(Direction::Bottom)
                                .interval(Position(0., 50.))
                                .build()?
                            )
                            .pos(Position(10., 30.))
                            .build()?
                        ),
                        Box::new(single(container(vec![
                                    container((0..half_troops).map(|_| {
                                        button(
                                            Drawing {
                                                pos: Position(0., 0.),
                                                    to_draw: |drawing, app, assets, gfx, plugins, state| {
                                                        unit_card_draw(drawing, app, assets, gfx, plugins, state, 1, *MAX_TROOPS.lock().unwrap() / 2);
                                                    }
                                                },
                                            Rect { pos: Position(0., 0.), size: Size(92., 92.) }
                                        ).if_clicked(|button, _app, _assets, _plugins, state| {
                                            let pos = button.rect.pos;
                                            let army = 1;
                                            let index = (pos.0 / 102.) as usize + *MAX_TROOPS.lock().unwrap() / 2;
                                            unit_card_clicked(state, army, index);
                                        }).build().unwrap()
                                    }).collect::<Vec<_>>())
                                    .interval(Position(10., 0.))
                                    .build()?,
                                    container((0..half_troops).map(|_| {
                                        button(
                                            Drawing {
                                                pos: Position(0., 0.),
                                                to_draw: |drawing, app, assets, gfx, plugins, state| {
                                                    unit_card_draw(drawing, app, assets, gfx, plugins, state, 1, 0);
                                                }
                                            },
                                            Rect { pos: Position(0., 0.), size: Size(92., 92.) }
                                        ).if_clicked(|button, _app, _assets, _plugins, state| {
                                                let pos = button.rect.pos;
                                                let index = (pos.0 / 102.) as usize;
                                                let army = 1;
                                                unit_card_clicked(state, army, index);
                                        })
                                        .build().unwrap()
                                    }).collect::<Vec<_>>())
                                    .interval(Position(10., 0.))
                                    .build()?
                                ])
                                .align_direction(Direction::Bottom)
                                .interval(Position(0., 50.))
                                .build()?
                            )
                            .pos(Position(10., 500.))
                            .build()?
                        ),
                        Box::new(
                            single(DrawingBuilder::<State>::default()
                                .to_draw(|drawing, _app, _assets, gfx, _plugins, state| {
                                    let draw = get_mut::<State, Draw>(state);
                                    gfx.render(draw);
                                    let pos = drawing.pos;
                                    let mut draw = gfx.create_draw();
                                    draw.image(&state.assets.get("assets/Icons").unwrap().get(&*format!("img_{}.png", get_menu_value_num(state, "battle_unit_stat").unwrap_or(1)-1)).unwrap().lock().unwrap())
                                        .position(pos.0, pos.1);
                                })
                                .build()?
                            )
                            .build()?
                        ),
                        Box::new(
                            single(text("Статистика")
                                   .size(20.0)
                                   .max_width(MONITOR_SIZE.lock().unwrap().0 - 900.)
                                .build()?
                            )
                            .after_draw(|container, app, _assets, _plugins, state: &mut State| {
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
                            })
                            .pos(Position(900., 200.))
                            .build()?
                        ),
                        Box::new(
                            button(
                                text("Заново")
                                .align_v(AlignVertical::Center)
                                .pos(Position(50., 0.))
                                .size(10.0)
                                .build()?,
                                Rect { pos: Position(200., 900.), size: Size(100., 30.) },
                            )
                            .if_clicked(|_button, _app, _assets, _plugins, state: &mut State| {
                                let mut rng = thread_rng();
                                state.gamemap.armys = vec![
                                    Army {
                                        troops: gen_army_troops(&mut rng, &state.units, 0),
                                        stats: ArmyStats {
                                            gold: 0,
                                            mana: 0,
                                            army_name: "hero".to_string()
                                        },
                                        inventory: vec![],
                                        pos: (0, 0),
                                        path: vec![]
                                    },
                                    Army {
                                        troops: gen_army_troops(&mut rng, &state.units, 1),
                                        stats: ArmyStats {
                                            gold: 0,
                                            mana: 0,
                                            army_name: "pidoras".to_string()
                                        },
                                        inventory: vec![],
                                        pos: (0, 0),
                                        path: vec![]
                                    }
                                ];
                                let mut battle = state.battle.clone();
                                battle.start(state);
                                state.battle = battle;
                                state.battle.active_unit = state.battle.search_next_active(&*state);
                                state.battle.can_interact = search_interactions(state);
                            })
                            .build()?
                        )
                    ],
                    pos: Position(0., 0.)
                })
            ],
            pos: Position(0., 0.),
            align_direction: Direction::Bottom,
            interval: Position(0., 0.)
        }),
        on_draw: Some(|_container, _app, _assets, _gfx, _plugins, state: &mut State| {
            get_mut::<State, Draw>(state)
                .rect((0., 0.), *MONITOR_SIZE.lock().unwrap())
                .color(Color::ORANGE);
        }),
        after_draw: Some(|_container, app, _assets, _plugins, state: &mut State| {
            if app.keyboard.is_down(KeyCode::Escape) { state.menu_id = Menu::Main as usize; }
            if app.keyboard.was_pressed(KeyCode::Space) {
                let active_unit = state.battle.active_unit;
                if let Some(active_unit) = active_unit {
                    let mut troop = state.gamemap.armys[active_unit.0].troops[active_unit.1].get();
                    let mut unit = &mut troop.as_mut().unwrap().unit;
                    let mut moves = unit.modified.moves;
                    if moves != 0 {
                        unit.stats.moves -= 1;
                        unit.recalc();
                        moves = unit.modified.moves;
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
        pos: Position(0., 0.)
    });
    dbg!(size_of::<State>());
    Ok(())
}

type Tilemap<T> = [[T; MAP_SIZE]; MAP_SIZE];
fn gen_army_troops(
    rng: &mut ThreadRng,
    units: &HashMap<usize, Unit>,
    army: usize,
) -> Vec<TroopType> {
    let mut troops = (0..*MAX_TROOPS.lock().unwrap() / 2)
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
        &mut (0..*MAX_TROOPS.lock().unwrap() / 2)
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
fn gen_tilemap() -> (Tilemap<usize>, (u32, u32)) {
    let noise = PerlinNoise::new();
    let mut rng = thread_rng();
    let seeds = (rng.gen_range(0..10000), rng.gen_range(0..10000));
    let nm1 = NoiseMap::new(noise)
        .set(Seed::of(seeds.0))
        .set(Step::of(0.005, 0.005));

    let nm2 = NoiseMap::new(noise)
        .set(Seed::of(seeds.1))
        .set(Step::of(0.05, 0.05));

    let nm = Box::new(nm1 + nm2 * 3);

    let world = World::new()
        .set(GenSize::of(50, 50))
        // Water
        .add(GenTile::new(6usize).when(constraint!(nm.clone(), < -0.1)))
        // Sand
        .add(GenTile::new(9).when(constraint!(nm.clone(), < 0.)))
        // Grass
        .add(GenTile::new(5).when(constraint!(nm.clone(), < 0.1)))
        .add(GenTile::new(0).when(constraint!(nm.clone(), < 0.45)))
        // Mountains
        .add(GenTile::new(1).when(constraint!(nm.clone(), > 0.8)))
        // Hills
        .add(GenTile::new(4));
    let mut w = world.generate(0, 0).unwrap();
    (
        w.iter_mut()
            .map(|item| from_fn(|_| item.pop().unwrap()))
            .collect::<Vec<_>>()
            .try_into()
            .unwrap(),
        seeds,
    )
}
fn gen_decomap(
    seeds: (u32, u32),
    first_tree_index: usize,
) -> Tilemap<Option<usize>> {
    let noise = PerlinNoise::new();
    let mut rng = thread_rng();
    let nm1 = NoiseMap::new(noise)
        .set(Seed::of(seeds.0))
        .set(Step::of(0.005, 0.005));

    let nm2 = NoiseMap::new(noise)
        .set(Seed::of(seeds.1))
        .set(Step::of(0.05, 0.05));

    let nm = Box::new(nm1 + nm2 * 3);

    let w = World::new()
        .set(GenSize::of(50, 50))
        .add(GenTile::new(Some(0..30))
            .when(constraint!(nm.clone(), > 0.44)))
        .add(GenTile::new(None)
            .when(constraint!(nm.clone(), < 0.)))
        // pine
        .add(GenTile::new(Some(113..124))
            .when(constraint!(nm.clone(), < 0.25)))
        // GreenTrees
        .add(GenTile::new(Some(44..50))
            .when(constraint!(nm.clone(), < 0.30)))
        .add(GenTile::new(Some(0..8))
            .when(constraint!(nm.clone(), < 0.35)))
        // Orange tres
        .add(GenTile::new(Some(51..59))
            .when(constraint!(nm.clone(), < 0.40)))
        .add(GenTile::new(Some(104..112))
            .when(constraint!(nm.clone(), < 0.45)))
        // palm
        //.add(GenTile::new(125..127))
        ;
    w.generate(0, 0)
        .expect("bl")
        .iter_mut()
        .map(|item| {
            from_fn(|_| {
                let val = item.pop().unwrap();
                if let Some(val) = val {
                    if rng.gen_range(0..100) > 70 {
                        (rng.gen_range(val) + first_tree_index).into()
                    } else {
                        None
                    }
                } else {
                    None
                }
            })
        })
        .collect::<Vec<_>>()
        .try_into()
        .expect("bla")
}

fn gen_shaders(gfx: &mut Graphics) -> Vec<(Pipeline, Buffer)> {
    const TILE_ALPHA: ShaderSource = fragment_shader! {
        r#"
        #version 450
        precision mediump float;

        layout(location = 0) in vec2 v_uvs;
        layout(location = 1) in vec4 v_color;
        layout(binding = 0) uniform sampler2D u_texture;
        layout(set = 0, binding = 1) uniform Facet {
            uint facet;
        };
        layout(location = 0) out vec4 color;

        void main() {
            vec2 tex_size = textureSize(u_texture, 0);
            vec2 coord = fract(v_uvs) * tex_size;
            color = texture(u_texture, coord / tex_size) * v_color;
            bool relay = (tex_size.x < tex_size.y);
            vec2 tex_mid;
            if (!relay) {
                tex_mid = vec2(0.5, float(facet)*1.0);
            } else {
                tex_mid = vec2(float(facet)*1.0, 0.5);
            };
            color.a = distance(tex_mid, v_uvs);

        }
        "#
    };
    let pipeline = create_image_pipeline(gfx, Some(&TILE_ALPHA)).unwrap();
    let uniforms = gfx
        .create_uniform_buffer(0, "facet")
        .with_data(&[0])
        .build()
        .unwrap();
    vec![(pipeline, uniforms)]
}
fn setup(app_assets: &mut Assets, gfx: &mut Graphics) -> State {
    let mut rng = thread_rng();
    let mut assets = HashMap::new();
    let asset_dirs = [
        "assets/Icons",
        "assets/Terrain",
        "assets/Objects",
        "assets/Items",
    ];
    for dir in asset_dirs {
        let mut dir_assets: HashMap<String, Asset<Texture>> = HashMap::new();
        for entry in read_dir(dir).expect("Cant find directory") {
            let entry = entry.expect("erorrere");
            let path = entry.path();
            let asset = app_assets
                .load_asset(path.to_str().clone().unwrap())
                .unwrap();
            dir_assets.insert(
                path.strip_prefix(dir)
                    .unwrap()
                    .to_str()
                    .unwrap()
                    .to_string(),
                asset,
            );
        }
        assets.insert(dir, dir_assets);
    }
    let settings = parse_settings();
    {
        *MAX_TROOPS.lock().unwrap() = settings.max_troops;
    }
    parse_items(&settings.locale);
    parse_locale(settings.locale);
    let units = parse_units();
    let objects = parse_objects();

    let tilemap = gen_tilemap();
    let decomap = gen_decomap(
        tilemap.1,
        *objects
            .iter()
            .find(|obj| obj.1.path == "Tree0.png")
            .unwrap()
            .0,
    );
    let tilemap = tilemap.0;
    let mut state = State {
        fonts: vec![gfx
            .create_font(include_bytes!("Ru_Gothic.ttf"))
            .expect("shit happens")],
        draw: gfx.create_draw(),
        frame: 0,
        gamemap: GameMap {
            time: Time::new(0),
            tilemap,
            decomap,
            hitmap: [[HitboxTile {
                passable: true,
                need_transport: false,
                building: None,
                army: None
            }; MAP_SIZE]; MAP_SIZE],
            buildings: vec![],
            armys: vec![
                Army {
                    troops: gen_army_troops(&mut rng, &units, 0),
                    stats: ArmyStats {
                        gold: 0,
                        mana: 0,
                        army_name: "hero".to_string(),
                    },
                    inventory: vec![],
                    pos: (0, 0),
                    path: vec![],
                },
                Army {
                    troops: gen_army_troops(&mut rng, &units, 1),
                    stats: ArmyStats {
                        gold: 0,
                        mana: 0,
                        army_name: "pidoras".to_string(),
                    },
                    inventory: vec![],
                    pos: (0, 0),
                    path: vec![],
                },
            ],
            relations: FractionsRelations {
                ally: Relations::default(),
                neighbour: Relations::default(),
                enemy: Relations::default()
            }
        },
        units,
        menu_id: 0,
        menu_data: HashMap::new(),
        assets,
        battle: BattleInfo {
            army1: 0,
            army2: 1,
            battle_ter: 0,
            active_unit: None,
            can_interact: None,
            move_count: 0,
            dead: vec![],
        },
        objects,
        shaders: gen_shaders(gfx),
    };
    state.gamemap.calc_hitboxes();
    let mut battle = state.battle.clone();
    battle.start(&mut state);
    state.battle = battle;
    state.battle.active_unit = state.battle.search_next_active(&state);
    state.battle.can_interact = search_interactions(&mut state);
	gen_forms(gfx).expect("gen_forms failed:()");
    dbg!(size_of::<ModifyUnitStats>());
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
    FORMS
        .lock()
        .unwrap()
        .get_mut(&state.menu_id)
        .unwrap()
        .draw(app, assets, gfx, plugins, state);
    gfx.render(get_mut::<State, Draw>(state));
    state.frame += 1;
}
fn update(app: &mut App, assets: &mut Assets, plugins: &mut Plugins, state: &mut State) {
    FORMS
        .lock()
        .unwrap()
        .get_mut(&state.menu_id)
        .unwrap()
        .after(app, assets, plugins, state);
}
#[notan_main]
fn main() -> Result<(), String> {
    let win = WindowConfig::new()
        .title("Discord Times: Remastered")
        .vsync(true)
        .lazy_loop(true)
        .high_dpi(true)
        .size(1600, 1200)
        .fullscreen(false)
        .resizable(true);
    notan::init_with(setup)
        .add_config(win)
        .add_config(TextConfig)
        .add_config(DrawConfig)
        .draw(draw)
        .update(update)
        .build()
}
