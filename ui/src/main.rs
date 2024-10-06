#![feature(let_chains)]
#![allow(unused_variables)]
#![allow(dead_code)]

use alkahest::{serialize, serialized_size};
use dt_lib::{
    battle::{army::*, battlefield::*, troop::Troop},
    items::item::*,
    locale::{parse_locale, Locale},
    map::{
        event::{execute_event, Event as GameEvent, Execute},
        map::*,
        object::ObjectInfo,
        tile::*,
    },
    network::net::*,
    parse::{parse_items, parse_objects, parse_settings, parse_story, parse_units},
    time::time::Data as TimeData,
    units::unit::{ActionResult, Unit, UnitPos},
};
use notan::{draw::*, fragment_shader, log, prelude::*, text::TextConfig};
use notan_ui::{
    animation::*, containers::*, defs::*, form::Form, forms::*, rect::*, text::*, wrappers::*,
};
use num::clamp;
use once_cell::sync::Lazy;
use parking_lot::MappedRwLockReadGuard;
use rand::{prelude::ThreadRng, thread_rng, Rng};
use std::{
    array::from_fn,
    collections::{HashMap, VecDeque},
    fmt::{Debug, Display},
    mem::size_of,
    time::{Duration, Instant},
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
    pub animations: Vec<AnimationTime<HashMap<&'static str, HashMap<String, Asset<Texture>>>>>,
    pub gamemap: GameMap,
    pub connection: Option<ConnectionManager>,
    pub gameevents: Vec<GameEvent>,
    pub gameloop_time: Duration,
    pub pause: bool,
    pub execution_queue: VecDeque<Execute>,
    pub units: Vec<Unit>,
    pub objects: Vec<ObjectInfo>,
    pub assets: HashMap<&'static str, HashMap<String, Asset<Texture>>>,
    pub menu_data: HashMap<&'static str, Value>,
    pub menu_id: usize,
    pub battle: Option<BattleInfo>,
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
trait GetTexture {
    fn get_texture(&self, dir: &'static str, img: &str) -> MappedRwLockReadGuard<'_, Texture>;
}
impl GetTexture for HashMap<&'static str, HashMap<String, Asset<Texture>>> {
    fn get_texture(&self, dir: &'static str, img: &str) -> MappedRwLockReadGuard<'_, Texture> {
        self.get(dir)
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

fn menu_button<T: ToText<State>>(
    justtext: T,
    on_draw: DrawFunction<State, SingleContainer<State, Button<State, Text<State, T>>>>,
    if_clicked: fn(
        &mut Button<State, Text<State, T>>,
        &mut App,
        &mut Assets,
        &mut Plugins,
        &mut State,
    ),
) -> Box<SingleContainer<State, Button<State, Text<State, T>>>> {
    Box::new(
        single(
            button(
                text(justtext)
                    .align_v(AlignVertical::Center)
                    .align_h(AlignHorizontal::Left)
                    .color(Color::WHITE)
                    .size(30.)
                    .font(FontId(0))
                    .pos(Position(0., 25.))
                    .build()
                    .unwrap(),
                Rect {
                    pos: Position(0., 0.),
                    size: Size(300., 50.),
                },
            )
            .if_clicked(if_clicked)
            .build()
            .unwrap(),
        )
        .on_draw(on_draw)
        .pos(Position(0., -25.))
        .build()
        .unwrap(),
    )
}

static FORMS: Lazy<Mutex<HashMap<usize, SingleContainer<State, DynContainer<State>>>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));
static LOCALE: Lazy<Mutex<Locale>> =
    Lazy::new(|| Mutex::new(Locale::new("Rus".into(), "Eng".into())));
static MONITOR_SIZE: Lazy<Mutex<(f32, f32)>> = Lazy::new(|| Mutex::new((0., 0.)));

#[repr(u32)]
enum Menu {
    Main,
    Start,
    Load,
    Settings,
    Authors,
    UnitView,
    Battle,
    Items,
    Connect,
    ConnectBattle,
}
fn move_thing(battle: &mut BattleInfo, armys: &mut Vec<Army>) {
    check_win(battle, armys);
    check_row_fall(battle, armys);
    battle.remove_corpses(armys);
    if battle.winner.is_none() {
        if battle.active_unit == None {
            next_move(battle, armys);
            battle.active_unit = battle.search_next_active(armys);
        }
        battle.can_interact = search_interactions(battle, armys);
    } else {
        //state.menu_id = Menu::Start as usize;
    }
}

fn gen_forms(size: (f32, f32)) -> Result<(), String> {
    let draw_back_centered: DrawFunction<
        State,
        SingleContainer<State, Button<State, Text<State, String>>>,
    > = |container, _app, _assets, _gfx, _plugins, _: &mut State, draw| {
        draw.rect(
            (container.pos - Position(container.get_size().0 / 2., 0.)).into(),
            container.get_size().into(),
        )
        .color(Color::from_hex(0x033121ff));
    };
    let draw_back: DrawFunction<State, SingleContainer<State, Button<State, Text<State, String>>>> =
        |container, _app, _assets, _gfx, _plugins, _: &mut State, draw| {
            draw.rect(
                (container.pos.0, container.pos.1).into(),
                container.get_size().into(),
            )
            .color(Color::from_hex(0x033121ff));
        };
    fn redirect_menu<T: PosForm<State>, const MENU: usize>(
        _: &mut T,
        _: &mut App,
        _: &mut Assets,
        _: &mut Plugins,
        state: &mut State,
    ) {
        state.menu_id = MENU;
    }
    let half_size = (size.0 as f32 / 2., size.1 as f32 / 2.);
    let mut hashmap = FORMS.lock().unwrap();
    hashmap.clear();
    let locale = LOCALE.lock().unwrap();
    let max_troops = *MAX_TROOPS;
    let half_troops = max_troops / 2;
    let nav_button_rect = Rect {
        pos: Position(100., 0.),
        size: Size(70., 100.),
    };
    let nav_button_text = |text: String| Text {
        text,
        font: FontId(2),
        align_h: AlignHorizontal::Left,
        align_v: AlignVertical::Bottom,
        pos: Position(0., 0.),
        size: 10.,
        rect_size: None,
        max_width: None,
        color: Color::BLACK,
        boo: std::marker::PhantomData,
    };
    hashmap.insert(
        Menu::Main as usize,
        SingleContainerBuilder::default()
            .inside(
                DynContainerBuilder::default()
                    .inside(vec![
                        Box::new(
                            text(locale.get("menu_game_name"))
                                .align_h(AlignHorizontal::Left)
                                .size(100.0)
                                .font(FontId(2))
                                .pos(Position(0., 0.))
                                .build()?,
                        ) as Box<dyn ObjPosForm<State>>,
                        menu_button(
                            locale.get("menu_start_title"),
                            draw_back,
                            redirect_menu::<_, { Menu::Start as usize }>,
                        ),
                        menu_button(
                            locale.get("menu_battle_title"),
                            draw_back,
                            redirect_menu::<_, { Menu::Battle as usize }>,
                        ),
                        menu_button(
                            locale.get("menu_multiplayer_title"),
                            draw_back,
                            redirect_menu::<_, { Menu::Connect as usize }>,
                        ),
                        menu_button(
                            locale.get("menu_pvp_title"),
                            draw_back,
                            redirect_menu::<_, { Menu::ConnectBattle as usize }>,
                        ),
                        menu_button(
                            locale.get("menu_load_title"),
                            draw_back,
                            redirect_menu::<_, { Menu::Load as usize }>,
                        ),
                        menu_button(
                            locale.get("menu_settings_title"),
                            draw_back,
                            redirect_menu::<_, { Menu::Settings as usize }>,
                        ),
                        menu_button(
                            locale.get("menu_unitview_title"),
                            draw_back,
                            redirect_menu::<_, { Menu::UnitView as usize }>,
                        ),
                        menu_button(
                            locale.get("menu_items_title"),
                            draw_back,
                            redirect_menu::<_, { Menu::Items as usize }>,
                        ),
                        menu_button(
                            locale.get("menu_exit_title"),
                            draw_back,
                            |_container, app, _assets, _plugins, _state: &mut State| app.exit(),
                        ),
                    ])
                    .pos(Position(30., 50.))
                    .align_direction(Direction::Bottom)
                    .interval(Position(0., 30.))
                    .build()?,
            )
            .on_draw(
                |_container, _app, _assets, _gfx, _plugins, state: &mut State, draw| {
                    let size = *MONITOR_SIZE.lock().unwrap();
                    draw.rect((0., 0.), *MONITOR_SIZE.lock().unwrap())
                        .color(Color::ORANGE);
                    draw.image(&state.assets.get_texture("assets/Window", "Menu.png"))
                        .position(0., 0.)
                        .size(size.0, size.1);
                },
            )
            .build()?,
    );
    fn items_unit_card_draw(
        drawing: &mut Drawing<State>,
        gfx: &mut Graphics,
        state: &mut State,
        draw: &mut Draw,
        army: usize,
        index: usize,
    ) {
        let pos = drawing.pos;
        let index = (pos.0 / 102.) as usize + index;
        if let Some(troop) = state.gamemap.armys[army].get_troop(index) {
            let troop = troop.get();
            let unit = &troop.unit;
            let texture = &state.get_texture(
                "assets/Icons",
                &*format!("unit_{}.png", unit.info.icon_index),
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
            draw_unit_info(unit, pos, &state.fonts[0], draw);
            draw.text(&state.fonts[0], &*format!("{};{};{}", army, index, pos.0))
                .color(Color::BLACK)
                .position(pos.0, pos.1);
        }
    }
    fn items_unit_buttons<const ARMY: usize, const ADD: usize>(
        half_troops: usize,
    ) -> Container<State, Button<State, Drawing<State>>> {
        container(
            (0..half_troops)
                .map(|_| {
                    button(
                        Drawing {
                            pos: Position(0., 0.),
                            to_draw: |drawing, _, _, gfx, _, state, draw| {
                                items_unit_card_draw(
                                    drawing,
                                    gfx,
                                    state,
                                    draw,
                                    ARMY,
                                    ADD * *MAX_TROOPS / 2,
                                );
                            },
                        },
                        Rect {
                            pos: Position(0., 0.),
                            size: Size(92., 92.),
                        },
                    )
                    .if_clicked(|button, _app, _assets, _plugins, state| {
                        let pos = button.rect.pos;
                        let index = (pos.0 / 102.) as usize + ADD * *MAX_TROOPS / 2;
                        set_menu_value_num(state, "items_unit_stat_changed", 1);
                        set_menu_value_num(state, "items_unit_stat_index", index as i64);
                        set_menu_value_num(state, "items_unit_stat_army", ARMY as i64);
                    })
                    .build()
                    .unwrap()
                })
                .collect::<Vec<_>>(),
        )
        .interval(Position(10., 0.))
        .build()
        .unwrap()
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
                                        items_unit_buttons::<0, 0>(half_troops),
										items_unit_buttons::<0, 1>(half_troops)
                                    ])
                                    .align_direction(Direction::Bottom)
                                    .interval(Position(0., 50.))
                                    .build()?,
                                    container(vec![
										items_unit_buttons::<1, 1>(half_troops),
										items_unit_buttons::<1, 0>(half_troops)
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
                            .to_draw(|drawing: &mut Drawing<State>, _app, _assets, _gfx, _plugins, state: &mut State, draw| {
                                if let Some(item_index) = get_menu_value_num(state, "items_item_index") {
                                    let pos = drawing.pos;
                                    draw.image(&state.get_texture("assets/Items", &*ITEMS.lock().unwrap().get(&(item_index as usize)).as_ref().expect(&*item_index.to_string()).icon))
                                        .position(pos.0, pos.1);
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
                            .if_clicked(|_button: &mut Button<State, Text<State, &str>>, app, assets, plugins, state| {
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
                            .if_clicked(|_button: &mut Button<State, Text<State, &str>>, _app, _assets, _plugins, state| {
                                set_menu_value_num(state, "items_item_index",
                                                   clamp(get_menu_value_num(state, "items_item_index").unwrap_or(0) + 1, 1, 166));
                            })
                            .build()?
                        ),
                        Box::new(
                            SingleContainerBuilder::default()
                                .inside(DrawingBuilder::<State>::default()
                                        .to_draw(|drawing, _app, _assets, _gfx, _plugins, state, draw| {
                                            let pos = drawing.pos;
                                            draw.image(&state.get_texture("assets/Icons", &*format!("unit_{}.png", get_menu_value_num(state, "items_unit_stat_icon").unwrap_or(1))))
                                                .position(pos.0, pos.1);
                                        })
                                        .pos(Position(900., 100.))
                                        .build()?
                                )
                                .build()?
                        ),
                        Box::new(
                            single(text("Статистика".to_string())
                                   .size(20.0)
                                   .max_width(MONITOR_SIZE.lock().unwrap().0 - 900.)
                                .build()?
                            )
                            .after_draw(|container, app, _assets, _plugins, state: &mut State| {
                                if app.keyboard.is_down(KeyCode::Escape) { state.menu_id = Menu::Main as usize; }
                                if get_menu_value_num(state, "items_unit_stat_changed").unwrap_or(0) == 1 {
                                    let index = get_menu_value_num(state, "items_unit_stat_index").unwrap_or(1) as usize;
                                    let army = get_menu_value_num(state, "items_unit_stat_army").unwrap_or(0) as usize;
                                    match &mut container.inside {
                                        Some::<Text<State, String>>(_) => {
											let troops = &state.gamemap.armys[army].troops;
											let troop = state.gamemap.armys[army].hitmap[index].and_then(|e| Some(&troops[e]));
											if let Some(troop) = troop {
												let icon_index = {
													let unit = &troop.get().unit;
													let text = unit.to_string();
													container.inside.as_mut().unwrap().text = text;
													unit.info.icon_index as i64
												};
                                                set_menu_value_num(state, "items_unit_stat_icon", icon_index);											
                                            };
                                            set_menu_value_num(state, "items_unit_stat_changed", 0);
                                        }
                                        None => {}
									}
								}
							})
                            .pos(Position(900., 200.))
                            .build()?
                        ),
                        Box::new(
                            button(DrawingBuilder::default()
                                .to_draw(|drawing, _app, _assets, _gfx, _plugins, state: &mut State, draw| {
                                    let pos = drawing.pos;
                                    if let (Some(army), Some(index)) = (get_menu_value_num(state, "items_unit_stat_army"), get_menu_value_num(state, "items_unit_stat_index")) {
										let troops = &state.gamemap.armys[army as usize].troops;
										let troop = state.gamemap.armys[army as usize].hitmap[index as usize].and_then(|e| Some(&troops[e]));
										if let Some(troop) = troop {
											for i in 0..4 {
                                                draw.rect((pos.0 + (53. + 5.) * i as f32, pos.1), (53., 53.))
                                                    .stroke_color(Color::BLACK)
                                                    .stroke(5.);
												if let Some(item) = &troop.get().unit.inventory.items[i] {
                                                    let texture = state.get_texture("assets/Items", &*item.get_info().icon);
                                                    draw.image(&texture)
                                                        .position(pos.0 + (53. + 5.) * i as f32, pos.1);
                                                }
                                            }
										}
									}
								})
                                .build()?,
                                Rect { pos:Position(1000., 147.), size: Size(212., 53.)}
                            ).if_clicked(|button, app, _assets, _plugins, state: &mut State| {
                                let slot = ((app.mouse.position().0 - 1. - button.rect.pos.0)/53.) as usize;
                                if let (Some(army), Some(index)) = (get_menu_value_num(state, "items_unit_stat_army"), get_menu_value_num(state, "items_unit_stat_index")) {
									if let Some(troop) = state.gamemap.armys[army as usize].get_troop(index as usize) {
										//if let Some(troop) = state.gamemap.armys[army as usize].get_troop(index as usize) {
										let troop = &mut troop.get();
										let unit = &mut troop.unit;
                                        if unit.inventory.items[slot].is_some() {
                                            unit.remove_item(slot);
                                            set_menu_value_num(state, "items_unit_stat_changed", 1);
                                            return;
                                        }
                                        if let Some(item_index) = get_menu_value_num(state, "items_item_index") {
                                            unit.add_item(Item { index: item_index as usize }.into(), slot);
                                            set_menu_value_num(state, "items_unit_stat_changed", 1);
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
             .on_draw(|_container, _app, _assets, _gfx, _plugins, _state: &mut State, draw| {
                draw.rect((0., 0.), *MONITOR_SIZE.lock().unwrap())
                    .color(Color::ORANGE);
             })
             .after_draw(|_container, app, _assets, _plugins, state: &mut State| {
                 if app.keyboard.is_down(KeyCode::Escape) {
                     state.menu_id = Menu::Main as usize;
                 }
             })
             .build()?
    );
    hashmap.insert(
        Menu::Settings as usize,
        single(
            dyn_cont(vec![Box::new(
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
                            },
                        )
                        .on_draw(|container, _, _, _, _, state: &mut State, draw| {
                            draw.rect(
                                (container.pos
                                    - Position(container.get_size().0 / 2., container.get_pos().1))
                                .into(),
                                container.get_size().into(),
                            )
                            .color(Color::from_hex(0x033121ff));
                            LOCALE.lock().unwrap().switch_lang();
                        })
                        .build()?,
                        text("Language").pos(Position(20., 0.)).size(20.0).build()?,
                        button(
                            text(locale.get("settings_regenerate_map"))
                                .size(20.)
                                .build()?,
                            Rect {
                                pos: Position(0., 100.),
                                size: Size(200., 50.),
                            },
                        )
                        .if_clicked(
                            |_button: &mut Button<State, Text<State, String>>,
                             _,
                             _,
                             _,
                             state: &mut State| {
                                let tilemap = gen_tilemap();
                                let decomap = gen_decomap(
                                    tilemap.1,
                                    state
                                        .objects
                                        .iter()
                                        .position(|obj| obj.path == "Tree0.png")
                                        .unwrap(),
                                );
                                state.gamemap.tilemap = tilemap.0;
                                state.gamemap.decomap = decomap;
                                let (objects, gamemap) = (&state.objects, &mut state.gamemap);
                                state.gamemap.calc_hitboxes(&state.objects);
                            },
                        )
                        .build()?,
                    ))
                    .build()?,
            )])
            .build()?,
        )
        .on_draw(
            |_container, _app, _assets, _gfx, _plugins, _state: &mut State, draw| {
                draw.rect((0., 0.), *MONITOR_SIZE.lock().unwrap())
                    .color(Color::ORANGE);
            },
        )
        .after_draw(|_container, app, _assets, _plugins, state: &mut State| {
            if app.keyboard.is_down(KeyCode::Escape) {
                state.menu_id = Menu::Main as usize;
            }
        })
        .pos(Position(0., 0.))
        .build()?,
    );
    fn get_form_map() -> Result<DynContainer<State>, StructBuildError> {
        let size = *MONITOR_SIZE.lock().unwrap();
        dyn_cont(vec![
			Box::new(button(
				single(
					center((true, true), SubWindowSys {
							windows: vec![
								Box::new(Drawing { // Map
									pos: Position(3., 0.),
									to_draw: |cont, app, assets, gfx, plugins, state: &mut State, draw| {}
								}),
								Box::new( // Battle
									single(
										button(
											Drawing {
												pos: Default::default(),
												to_draw: |cont, app, assets, gfx, plugins, state: &mut State, draw| {}
											},
											Rect { pos: Position(0., 0.), size: Size(1000., 1000.) }
										)
											.build()?
									)
										.on_draw(|cont, app, assets, gfx, plugins, state: &mut State, draw| {
											draw_battle(cont, app, gfx, state, draw);
										})
										.after_draw(|cont,app,assets,plugins,state: &mut State| {
											let Some(battle) = &mut state.battle else { return; };
											let half_troops = *MAX_TROOPS / 2;
											for army in 0..=1 {
												for row in 0..=1 {
													for i in 0..half_troops {
														let mut pos = Position(0., 0.);
														pos.0 = cont.pos.0 + i as f32 * 102.;
														pos.1 = cont.pos.1 + army as f32 * 250. + row as f32 * 112.;
														if app.mouse.left_was_released() && (Rect { pos: pos, size: Size(92., 102.) }).collides((app.mouse.x, app.mouse.y).into()) {
															handle_action(Action::Cell(i + (army as i64 - row as i64).abs() as usize * half_troops, army), battle, &mut state.gamemap.armys);
														}
													}
												}
											}
											if app.keyboard.was_pressed(KeyCode::Escape) {
												set_menu_value_num(state, "start_menu", 0);
											}
										})
										.build()?
								),
								Box::new( // Building
									single(
										Drawing {
											pos: Default::default(),
											to_draw: |cont, app, assets, gfx, plugins, state: &mut State, draw| {}
										}
									)
										.on_draw(|cont, app, assets, gfx, plugins, state: &mut State, draw| {
											get_building_menu_form(state, state.gamemap.armys[0].building.unwrap_or(0))
												.draw(app, assets, gfx, plugins, state, draw);
										})
										.after_draw(|cont,app,assets,plugins,state: &mut State| {
											//handle_start_menu(cont, app, state, assets, plugins, StartSubMenu::Building);
										})
										.build()?
								),
								Box::new( // Army
									single(
										Drawing {
											pos: Default::default(),
											to_draw: |cont, app, assets, gfx, plugins, state: &mut State, draw| {}
										}
									)
										.on_draw(|cont, app, assets, gfx, plugins, state: &mut State, draw| {
											get_building_menu_form(state, state.gamemap.armys[0].building.unwrap_or(0))
												.draw(app, assets, gfx, plugins, state, draw);
										})
										.after_draw(|cont,app,assets,plugins,state: &mut State| {
											//handle_start_menu(cont, app, state, assets, plugins, StartSubMenu::Building);
										})
										.build()?
								),
								Box::new(
									single(
										dyn_cont(vec![
											Box::new(
												button(
													text("Очень важное событие")
														.size(50.)
														.pos(Position(0., 0.))
														.build()?,
													Rect { size: Size(500., 60.), pos: Default::default() }
												)
													.build()?
											),
											Box::new(
												text(|state: &State| get_menu_value_str(state, "current_message").unwrap_or("".to_string()))
													.size(20.)
													.pos(Position(0., 0.))
													.build()?
											),
											Box::new(
												button(
													text(
														"ну ок"
													)
														.pos(Position(0., 0.))
														.size(50.)
														.build()?,
													Rect {
														pos: (0., 100.).into(),
														size: Size(500., 150.)
													}
												)
													.if_clicked(|butt, _, _, _, state: &mut State| {
														set_menu_value_num(state, "start_menu", 0);
													})
													.build()?
											)
										])
											.align_direction(Direction::Bottom)
											.build()?
									)
										.on_draw(|cont, _, _, _, _, _: &mut State, draw| {
											draw.rect(cont.pos.into(), cont.get_size().into())
												.color(Color::ORANGE);
										})
										.build()?
								),
							],
						select_window: |cont, state: &State| {
							get_menu_value_num(state, "start_menu").unwrap_or(0) as usize
							},
							rect: Rect {
								pos: Position(size.0/2., size.1/2.),
								size: Size(1000., 1000.)
							}
						}
					))
					.on_draw(
						|cont, app, assets, gfx, plugins, state: &mut State, draw| {
							draw_gamemap(cont, app, gfx, &state.assets, &state.gamemap, &state.objects, draw);
						}
					)
					.after_draw(|cont,app,assets,plugins,state: &mut State| {
						if get_menu_value_num(state, "start_menu").unwrap_or(0) == 0 {
							if app.keyboard.was_pressed(KeyCode::Space) {
								state.pause = !state.pause;
							}
							if app.keyboard.was_pressed(KeyCode::Escape) {
								state.menu_id = Menu::Main as usize;
							}
							if app.keyboard.was_pressed(KeyCode::D) {
								if state.gamemap.armys[0].building.is_some() {
									set_menu_value_num(state, "start_menu", 2);
								}
							}
							let rect = Rect {
								pos: cont.pos,
								size: (SIZE.0 * MAP_SIZE as f32, SIZE.1 * MAP_SIZE as f32).into()
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
								if let Some(army) = state.gamemap.hitmap[goal.0][goal.1].army  {
									if army != 0 {
										let pos = state.gamemap.armys[0].pos;
										let diff = (pos.0 as i64 - goal.0 as i64, pos.1 as i64 - goal.1 as i64);
										if -1 <= diff.0 && diff.0 <= 1 && -1 <= diff.1 && diff.1 <= 1 {
											if state.battle.is_none() {
												let battle = BattleInfo::new(&mut state.gamemap.armys, army, 0);
												state.battle = Some(battle);
											}
											state.menu_id = Menu::Battle as usize;
											//set_menu_value_num(state, "start_menu", 1);
										}
									}
								}
								let path = find_path(&state.gamemap, &state.objects, start, goal, false);
								state.gamemap.armys[0].path = if let Some(path) = path {
									state.pause = false;
									path.0
								} else {
									Vec::new()
								};
							}
							// grand gameloopa
							let mut pause = false;
							for i in 0..=1 {
								let army = &mut state.gamemap.armys[i];
								if army.path.len() < 1 {
									if i == 0 {
										pause = true;
									}
									continue;
								}
							}
							state.pause = pause;
							if !state.pause {
								for i in 0..state.gamemap.armys.len() {
									let army = &mut state.gamemap.armys[i];
									if army.path.len() < 1 {
										continue;
									}
									army.pos = army.path.remove(0);
									if let Some(building) = state.gamemap.hitmap[army.pos.0][army.pos.1].building {
										army.building = Some(building);
									} else { army.building = None; }
									state.gamemap.recalc_armies_hitboxes();
								}
								state.gamemap.time.minutes += 10;

								for i in 0..state.gameevents.len() {
									if let Some(executions) = execute_event(i, &mut state.gamemap, &mut state.gameevents, &state.units, false) {
										for exec in executions {
											match exec {
												Execute::Wait(t, _) => {},
												Execute::Execute(event, _) => {
													execute_event(event.event, &mut state.gamemap, &mut state.gameevents, &state.units, true);
												},
												Execute::StartBattle(army, _) => {
													if state.battle.is_none() {
														let battle = BattleInfo::new(&mut state.gamemap.armys, army, 0);
														state.battle = Some(battle);
													}
													set_menu_value_num(state, "start_menu", 1);
												},
												Execute::Message(text, _) => {
													set_menu_value_num(state, "start_menu", 4);
													set_menu_value_str(state, "current_message", text);
												}
											}
										}
										break;
									};
								}
							}
						}
					})
					.build()?,
				Rect { pos: (0., 0.).into(), size: (0., MONITOR_SIZE.lock().unwrap().1-100.).into() }).build()?),
			Box::new(
				single(
					dyn_cont(vec![
						Box::new(text(|state: &State| format!("Золото: {}", state.gamemap.armys[0].stats.gold))
								 .size(70.)
								 .pos(Position(0., 50.))
								 .align_v(AlignVertical::Center)
								 .build()?),
						Box::new(text(|state: &State| format!("Мана: {}", state.gamemap.armys[0].stats.mana))
								 .pos(Position(0., 50.))
								 .align_v(AlignVertical::Center)
								 .size(70.)
								 .build()?),
						Box::new(text(|state: &State| state.gamemap.time.to_data([TimeData::YEAR, TimeData::MONTH, TimeData::DAY, TimeData::HOUR, TimeData::MINUTES], "-"))
								 .pos(Position(0., 50.))
								 .size(70.)
								 .align_v(AlignVertical::Center)
								 .build()?),
					])
						.align_direction(Direction::Right)
						.interval(Position(100.,0.))
						.build()?)
					.on_draw(|cont, _, _, _, _, state: &mut State, draw| {
						draw.rect((cont.pos.0, cont.pos.1), *MONITOR_SIZE.lock().unwrap())
							.color(Color::from_hex(0x033121ff));
					})
					.build()?
			)]
		)
			.align_direction(Direction::Bottom)
			.build()
    }
    fn get_form_server_map() -> Result<DynContainer<State>, StructBuildError> {
        let size = *MONITOR_SIZE.lock().unwrap();
        dyn_cont(vec![
            Box::new(
                button(
                    single(center(
                        (true, true),
                        SubWindowSys {
                            windows: vec![
                                Box::new(Drawing {
                                    // Map
                                    pos: Position(0., 0.),
                                    to_draw: |cont,
                                              app,
                                              assets,
                                              gfx,
                                              plugins,
                                              state: &mut State,
                                              draw| {},
                                }),
                                Box::new(
                                    // Building
                                    single(Drawing {
                                        pos: Default::default(),
                                        to_draw:
                                            |cont,
                                             app,
                                             assets,
                                             gfx,
                                             plugins,
                                             state: &mut State,
                                             draw| {},
                                    })
                                    .on_draw(
                                        |cont,
                                         app,
                                         assets,
                                         gfx,
                                         plugins,
                                         state: &mut State,
                                         draw| {
                                            get_building_menu_form(
                                                state,
                                                state.gamemap.armys[0].building.unwrap_or(0),
                                            )
                                            .draw(app, assets, gfx, plugins, state, draw);
                                        },
                                    )
                                    .after_draw(|cont, app, assets, plugins, state: &mut State| {
                                        //handle_start_menu(cont, app, state, assets, plugins, StartSubMenu::Building);
                                    })
                                    .build()?,
                                ),
                                Box::new(
                                    // Army
                                    single(Drawing {
                                        pos: Default::default(),
                                        to_draw:
                                            |cont,
                                             app,
                                             assets,
                                             gfx,
                                             plugins,
                                             state: &mut State,
                                             draw| {},
                                    })
                                    .on_draw(
                                        |cont,
                                         app,
                                         assets,
                                         gfx,
                                         plugins,
                                         state: &mut State,
                                         draw| {
                                            get_building_menu_form(
                                                state,
                                                state.gamemap.armys[0].building.unwrap_or(0),
                                            )
                                            .draw(app, assets, gfx, plugins, state, draw);
                                        },
                                    )
                                    .after_draw(|cont, app, assets, plugins, state: &mut State| {
                                        //handle_start_menu(cont, app, state, assets, plugins, StartSubMenu::Building);
                                    })
                                    .build()?,
                                ),
                                Box::new(
                                    single(
                                        dyn_cont(vec![
                                            Box::new(
                                                button(
                                                    text("Очень важное событие")
                                                        .size(50.)
                                                        .pos(Position(0., 0.))
                                                        .build()?,
                                                    Rect {
                                                        size: Size(500., 60.),
                                                        pos: Default::default(),
                                                    },
                                                )
                                                .build()?,
                                            ),
                                            Box::new(
                                                text(|state: &State| {
                                                    get_menu_value_str(state, "current_message")
                                                        .unwrap_or("".to_string())
                                                })
                                                .size(20.)
                                                .pos(Position(0., 0.))
                                                .build()?,
                                            ),
                                            Box::new(
                                                button(
                                                    text("ну ок")
                                                        .pos(Position(0., 0.))
                                                        .size(50.)
                                                        .build()?,
                                                    Rect {
                                                        pos: (0., 100.).into(),
                                                        size: Size(500., 150.),
                                                    },
                                                )
                                                .if_clicked(|butt, _, _, _, state: &mut State| {
                                                    set_menu_value_num(state, "start_menu", 0);
                                                })
                                                .build()?,
                                            ),
                                        ])
                                        .align_direction(Direction::Bottom)
                                        .build()?,
                                    )
                                    .on_draw(|cont, _, _, _, _, _: &mut State, draw| {
                                        draw.rect(cont.pos.into(), cont.get_size().into())
                                            .color(Color::ORANGE);
                                    })
                                    .build()?,
                                ),
                            ],
                            select_window: |cont, state: &State| {
                                get_menu_value_num(state, "start_menu").unwrap_or(0) as usize
                            },
                            rect: Rect {
                                pos: Position(size.0 / 2., size.1 / 2.),
                                size: Size(1000., 1000.),
                            },
                        },
                    ))
                    .on_draw(|cont, app, assets, gfx, plugins, state: &mut State, draw| {
                        let Some(conn) = &state.connection else {
                            return;
                        };
                        draw_gamemap(
                            cont,
                            app,
                            gfx,
                            &state.assets,
                            &conn.gamemap,
                            &state.objects,
                            draw,
                        );
                    })
                    .after_draw(|cont, app, assets, plugins, state: &mut State| {
                        if get_menu_value_num(state, "start_menu").unwrap_or(0) == 0 {
                            let Some(conn) = &mut state.connection else {
                                return;
                            };

                            if app.keyboard.was_pressed(KeyCode::Escape) {
                                state.menu_id = Menu::Main as usize;
                            }
                            if app.keyboard.was_pressed(KeyCode::D) {
                                if state.gamemap.armys[0].building.is_some() {
                                    // set_menu_value_num(state, "start_menu", 2);
                                }
                            }
                            let rect = Rect {
                                pos: cont.pos,
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
                                conn.send_message_to_server(
                                    ClientMessage::MapClick(goal),
                                    &state.units,
                                    &state.objects,
                                );
                            }
                            // grand gameloopa
                            conn.updates(&state.units, &state.objects);
                        }
                    })
                    .build()?,
                    Rect {
                        pos: (0., 0.).into(),
                        size: (0., MONITOR_SIZE.lock().unwrap().1 - 100.).into(),
                    },
                )
                .build()?,
            ),
            Box::new(
                single(
                    dyn_cont(vec![
                        Box::new(
                            text(|state: &State| {
                                format!("Золото: {}", state.gamemap.armys[0].stats.gold)
                            })
                            .size(70.)
                            .pos(Position(0., 50.))
                            .align_v(AlignVertical::Center)
                            .build()?,
                        ),
                        Box::new(
                            text(|state: &State| {
                                format!("Мана: {}", state.gamemap.armys[0].stats.mana)
                            })
                            .pos(Position(0., 50.))
                            .align_v(AlignVertical::Center)
                            .size(70.)
                            .build()?,
                        ),
                        Box::new(
                            text(|state: &State| {
                                state.gamemap.time.to_data(
                                    [
                                        TimeData::YEAR,
                                        TimeData::MONTH,
                                        TimeData::DAY,
                                        TimeData::HOUR,
                                        TimeData::MINUTES,
                                    ],
                                    "-",
                                )
                            })
                            .pos(Position(0., 50.))
                            .size(70.)
                            .align_v(AlignVertical::Center)
                            .build()?,
                        ),
                    ])
                    .align_direction(Direction::Right)
                    .interval(Position(100., 0.))
                    .build()?,
                )
                .on_draw(|cont, _, _, _, _, state: &mut State, draw| {
                    draw.rect((cont.pos.0, cont.pos.1), *MONITOR_SIZE.lock().unwrap())
                        .color(Color::from_hex(0x033121ff));
                })
                .build()?,
            ),
        ])
        .align_direction(Direction::Bottom)
        .build()
    }

    const SIZE: (f32, f32) = (52., 40.);
    const VIEW: usize = 20;
    fn draw_gamemap<Form: PosForm<State>>(
        drawing: &mut Form,
        app: &mut App,
        gfx: &mut Graphics,
        assets: &AssetsMap,
        gamemap: &GameMap,
        objects: &Vec<ObjectInfo>,
        draw: &mut Draw,
    ) {
        let terrain = assets.get("assets/Terrain").unwrap();
        let army = assets.get("assets/Armys").unwrap();
        draw.image(
            &assets
                .get("assets/Map")
                .and_then(|map| map.get("Map"))
                .unwrap()
                .lock()
                .unwrap(),
        )
        .position(0., 0.);
        let pos = gamemap.armys[0].pos;
        for i in 0..MAP_SIZE {
            //((pos.0 - VIEW / 2).clamp(0, MAP_SIZE))..((pos.0 + VIEW/2).clamp(0, MAP_SIZE)) {
            for j in 0..MAP_SIZE {
                //((pos.0 - VIEW / 2).clamp(0, MAP_SIZE))..((pos.0 + VIEW/2).clamp(0, MAP_SIZE)) {
                // let asset = terrain
                //     .get(TILES[state.gamemap.tilemap[i][j]].sprite())
                //     .unwrap();
                //let texture = asset.lock().unwrap();

                let pos = Position(i as f32 * SIZE.0, j as f32 * SIZE.1);
                // draw.image(&texture)
                //      .position(pos.0, pos.1)
                //     .size(SIZE.0, SIZE.1);
                if gamemap.armys[0].path.contains(&(i, j)) {
                    draw.rect((pos).into(), (10., 10.)).color(Color::RED);
                }
                if gamemap.armys[1].path.contains(&(i, j)) {
                    draw.rect((pos).into(), (10., 10.)).color(Color::BLUE);
                }
                if gamemap.hitmap[i][j].army.is_some() {
                    let pos: (f32, f32) = (pos - Position(0., SIZE.1)).into();
                    draw.image(&army.get("Army.png").unwrap().lock().unwrap())
                        .position(pos.0, pos.1)
                        .size(SIZE.0, SIZE.1 * 2.);
                }
            }
        }
        for i in 0..0 {
            //i in ((pos.0 - VIEW / 2).clamp(0, MAP_SIZE))..((pos.0 + VIEW/2).clamp(0, MAP_SIZE)) {
            for j in 0..0 {
                //j in ((pos.0 - VIEW / 2).clamp(0, MAP_SIZE))..((pos.0 + VIEW/2).clamp(0, MAP_SIZE)) {
                let asset = terrain.get(TILES[gamemap.tilemap[i][j]].sprite()).unwrap();
                let texture = asset.lock().unwrap();
                const ALPHA: f32 = 0.2;
                const W_SIZE_QUARTER: f32 = SIZE.0 / 4.;
                const H_SIZE_QUARTER: f32 = SIZE.1 / 4.;
                const QUARTER_TILE: f32 = 106. / 4.;
                const THREE_QUARTERS: f32 = QUARTER_TILE * 3.;
                let pos = Position(i as f32 * SIZE.0, j as f32 * SIZE.1);
                (0..4).for_each(|i| {
                    let (quarter_size, crop_start, cropped_size, quarter_pos) = match i {
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
                });
            }
        }
        let objects_textures = assets.get("assets/Objects").unwrap();
        for building in &gamemap.buildings {
            let index = building.id;
            let (i, j) = (building.pos.0, building.pos.1);
            let asset = objects_textures
                .get(&objects[index].path)
                .expect(&*format!("{}", &objects[index].path));
            let texture = asset.lock().unwrap();
            let size = objects[index].size;
            let pos = Position(i as f32 * SIZE.0, j as f32 * SIZE.1);
            draw.image(&texture)
                .position(pos.0, pos.1) // - (size.1 as f32 - 1.) * SIZE.1)
                .size(SIZE.0 * size.0 as f32, SIZE.1 * size.1 as f32);
        }
    }
    #[repr(u64)]
    enum Fields {
        Market = 0,
        Recruit = 1,
        Garrison = 2,
    }
    impl From<i64> for Fields {
        fn from(value: i64) -> Self {
            match value {
                0 => Self::Market,
                1 => Self::Recruit,
                2 => Self::Garrison,
                _ => Self::Market,
            }
        }
    }
    impl Display for Fields {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(
                f,
                "{}",
                match self {
                    Fields::Market => LOCALE.lock().unwrap().get("market"),
                    Fields::Recruit => LOCALE.lock().unwrap().get("recruit"),
                    Fields::Garrison => LOCALE.lock().unwrap().get("garrison"),
                }
            )
        }
    }
    fn get_building_menu_form(state: &State, buildingn: usize) -> impl PosForm<State> {
        let building = &state.gamemap.buildings[buildingn];
        //let screen = *MONITOR_SIZE.lock().unwrap();
        fn submenu_button<const NUM: i64>(
            locale: &str,
        ) -> SingleContainer<State, Button<State, Text<State, String>>> {
            single(
                button(
                    text(LOCALE.lock().unwrap().get(locale))
                        .align_h(AlignHorizontal::Center)
                        .align_v(AlignVertical::Center)
                        .pos((150., 65.))
                        .size(34.)
                        .build()
                        .unwrap(),
                    Rect {
                        pos: (0., 0.).into(),
                        size: (300., 130.).into(),
                    },
                )
                .if_clicked(|_, _, _, _, state| {
                    set_menu_value_num(state, "building_menu", NUM);
                })
                .build()
                .unwrap(),
            )
            .on_draw(|cont, _, _, _, _, _, draw| {
                draw.rect(cont.get_pos().into(), cont.get_size().into())
                    .color(Color::ORANGE)
                    .stroke(10.)
                    .stroke_color(Color::from_rgba(0., 0., 0., 0.2));
            })
            .build()
            .unwrap()
        }
        single(
            straight_dyn(vec![
                Box::new(
                    container({
                        let mut butts = Vec::new();
                        butts.push(submenu_button::<0>("building_main"));
                        if building.market.is_some() {
                            butts.push(submenu_button::<1>("market"));
                        }
                        butts
                    })
                    .align_direction(Direction::Bottom)
                    .pos((20., 20.))
                    .build()
                    .unwrap(),
                ),
                Box::new(
                    submenu_frame(state, buildingn)
                        .pos((500., 20.))
                        .build()
                        .unwrap(),
                ),
            ])
            .pos((20., 20.))
            .build()
            .unwrap(),
        )
        .pos((100., 50.).into())
        .after_draw(|cont, app, _, _, state: &mut State| {
            if app.keyboard.was_released(KeyCode::Escape) {
                set_menu_value_num(state, "start_menu", 0);
            }
            println!("debug");
        })
        .on_draw(|cont, _, _, _, _, _, draw| {
            //let screen = *MONITOR_SIZE.lock().unwrap();
            draw.rect(cont.get_pos().into(), (1000., 900.)) //(screen.0 - 100., screen.1 - 50.))
                .color(Color::ORANGE)
                .stroke(10.)
                .stroke_color(Color::from_rgba(0., 0., 0., 0.2));
        })
        .build()
        .unwrap()
    }
    fn submenu_frame(state: &State, building: usize) -> DynContainerBuilder<State> {
        match get_menu_value_num(state, "building_menu").unwrap_or(0) {
			1 => {
				dyn_cont({
					let building = &state.gamemap.buildings[building];
					let items = ITEMS.lock().unwrap();
					if let Some(market) = &building.market {
					vec![Box::new(container(
							market.items.iter().enumerate().map(|(n, itemn)| {
								let item = &items[itemn];
								button(single(TupleContainerBuilder::default()
									.inside((
										TextureRenderer {
											rect: Rect { pos: Position(0., 0.), size: Size(30., 30.) },
											texture_id: ("assets/Items".into(), format!("img_{}.png", itemn)),
											to_draw: |cont,_,_,_,_, state: &mut State, draw| {
												draw.image(&state.get_texture(&cont.texture_id.0, &cont.texture_id.1))
													.position(cont.rect.pos.0, cont.rect.pos.1);
											}
										},
										text(item.name.clone())
											.size(30.)
											.build().unwrap()
									)).build().unwrap()
								)
									   .after_draw(|cont, app, _, _, state: &mut State| {
										   if app.keyboard.was_pressed(KeyCode::Escape) {
											   set_menu_value_num(state, "start_menu", 0);
										   }
									   })
									   .build().unwrap(),
									   Rect {
										   pos: Position(0., 0.),
										   size: Size(300., 30.)
									   }
								).build().unwrap()
							}).collect()
						).build().unwrap())]
					} else { vec![] }
				})
			},
			_ => {
				dyn_cont({
					let building = &state.gamemap.buildings[building];
					if let Some(recruitment) = &building.recruitment {
						vec![Box::new(container(recruitment.units.iter().enumerate().map(|(n, unit)| {
							let unit = &state.units[unit.unit];
							button(single(TupleContainerBuilder::default()
								.inside((
									TextureRenderer {
										rect: Rect { pos: Position(0., 0.), size: Size(92., 102.) },
										texture_id: ("assets/Icons".into(), format!("unit_{}.png", unit.info.icon_index)),
										to_draw: |cont,_,_,_,_, state: &mut State, draw| {
											draw.image(&state.get_texture(&cont.texture_id.0, &cont.texture_id.1))
												.position(cont.rect.pos.0, cont.rect.pos.1);
										}
									},
									text("Нанять")
										.size(20.)
										.max_width(92.)
										.build().unwrap(),
								   ))
  							       .align_direction(Direction::Bottom)
								   .interval(Position(0., 0.))
										  .build().unwrap())
								   .after_draw(|cont, app, _, _, state: &mut State| {
									   if app.keyboard.was_pressed(KeyCode::Escape) {
											   set_menu_value_num(state, "start_menu", 0);
										   }
									   })
								   .on_draw(|cont,_,_,_,_,_,draw|{
									   draw.rect(cont.get_pos().into(), (92., 122.));
								   }).build().unwrap()
								   ,
								   Rect {
									   size: (92., 122.).into(),
									   pos: (0., 0.).into()
								   }
							)
							.if_clicked(|_,_,_,_,state: &mut State| {
								state.gamemap.buildings[0].recruitment.as_mut().unwrap().buy(&mut state.gamemap.armys[0], 0, &state.units).unwrap_or_else(|_| println!("сё пошло по пизде"));
							}).build().unwrap()
						}).collect())
							.align_direction(Direction::Right)
							.interval((50., 0.))
							.build().unwrap())]
					} else { vec![] }
				},
				).align_direction(Direction::Bottom)
			},
		}
    }
    fn draw_battle<Form: PosForm<State>>(
        drawing: &mut Form,
        app: &mut App,
        gfx: &mut Graphics,
        state: &mut State,
        draw: &mut Draw,
    ) {
        let Some(battle) = &mut state.battle else {
            return;
        };
        let drawing_pos = drawing.get_pos();
        draw.rect(drawing_pos.into(), (1000., 1000.))
            .color(Color::ORANGE);
        let half_troops = *MAX_TROOPS / 2;
        for army in 0..=1 {
            for row in 0..=1 {
                for i in 0..(*MAX_TROOPS / 2) {
                    let mut pos = Position(0., 0.);
                    pos.0 = drawing_pos.0 + i as f32 * 102.;
                    pos.1 = drawing_pos.1 + army as f32 * 250. + row as f32 * 112.;
                    // f(x) = { 0, 6, 6, 0 } where x = { {0, 0}, {0, 1}, {1, 0}, {1, 1} }
                    // abs(0 * 6 - 0 * 6) = 0
                    // abs(0 * 6 - 1 * 6) = 6
                    // abs(1 * 6 - 0 * 6) = 6
                    // abs(1 * 6 - 1 * 6) = 0
                    draw.rect(pos.into(), (92., 102.)).color(Color::BLACK);

                    unit_card_draw(
                        pos,
                        app,
                        gfx,
                        &state.assets,
                        &state.fonts[0],
                        battle,
                        &mut state.gamemap,
                        draw,
                        army,
                        i + (army as i64 - row as i64).abs() as usize * half_troops,
                    );
                }
            }
        }
    }
    #[repr(u64)]
    enum StartSubMenu {
        Map = 0,
        Battle = 1,
        Building = 2,
        Army = 3,
        Message = 4,
    }
    impl From<i64> for StartSubMenu {
        fn from(value: i64) -> Self {
            match value {
                1 => Self::Battle,
                2 => Self::Building,
                3 => Self::Army,
                4 => Self::Message,
                _ => Self::Map,
            }
        }
    }
    hashmap.insert(
        Menu::Start as usize,
        single(get_form_map()?)
            .on_draw(|_, _, _, _, _, _: &mut State, draw| {
                draw.rect((0., 0.), *MONITOR_SIZE.lock().unwrap())
                    .color(Color::ORANGE);
            })
            .build()?,
    );
    hashmap.insert(
        Menu::Connect as usize,
        single(get_form_server_map()?)
            .on_draw(|_, _, _, _, _, _: &mut State, draw| {
                draw.rect((0., 0.), *MONITOR_SIZE.lock().unwrap())
                    .color(Color::ORANGE);
            })
            .build()?,
    );
    hashmap.insert(
        Menu::UnitView as usize,
        SingleContainer {
            inside: Some(DynContainer {
                inside: vec![Box::new(
                    TupleContainerBuilder::default()
                        .inside((
                            Drawing {
                                pos: Position(100., 0.),
                                to_draw: |drawing, _, _, _, _, state: &mut State, draw| {
                                    let num = get_menu_value_num(state, "char_view_selected")
                                        .unwrap_or(1)
                                        - 1;
                                    let asset = state
                                        .assets
                                        .get("assets/Icons")
                                        .unwrap()
                                        .get(&format!("unit_{}.png", num.to_string()))
                                        .unwrap();
                                    let texture = asset.lock().unwrap();
                                    draw.image(&texture).position(drawing.pos.0, drawing.pos.1);
                                },
                            },
                            container(vec![
                                button(nav_button_text("Пред.".into()), nav_button_rect)
                                    .if_clicked(|_, _, _, _, state: &mut State| {
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
                                                state
                                                    .menu_data
                                                    .insert("char_view_selected", Value::Num(1));
                                            }
                                        }
                                        set_menu_value_num(state, "char_view_changed", 1);
                                    })
                                    .build()?,
                                button(nav_button_text("След.".into()), nav_button_rect)
                                    .if_clicked(|_, _, _, _, state: &mut State| {
                                        let _nav_button_rect = Rect {
                                            pos: Position(0., 0.),
                                            size: Size(70., 100.),
                                        };
                                        let _nav_button_text = |text: String| {
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
                                    })
                                    .build()?,
                            ])
                            .interval(Position(150., 0.))
                            .build()?,
                            single(text("АБОБА".into()).size(20.0).build()?)
                                .after_draw(|container, app, _, _, state: &mut State| {
                                    if app.keyboard.is_down(KeyCode::Escape) {
                                        state.menu_id = Menu::Main as usize;
                                    }
                                    if get_menu_value_num(state, "char_view_changed").unwrap_or(1)
                                        == 1
                                    {
                                        let num = get_menu_value_num(state, "char_view_selected")
                                            .unwrap_or(1);
                                        match &mut container.inside {
                                            Some::<Text<State, String>>(text) => {
                                                text.text = state
                                                    .units
                                                    .get(num as usize)
                                                    .unwrap()
                                                    .to_string();
                                                set_menu_value_num(state, "char_view_changed", 0);
                                            }
                                            None => {}
                                        }
                                    }
                                })
                                .pos(Position(0., 200.))
                                .build()?,
                        ))
                        .build()?,
                )],
                ..Default::default()
            }),
            on_draw: Some(|_, _, _, _, _, _: &mut State, draw| {
                draw.rect((0., 0.), *MONITOR_SIZE.lock().unwrap())
                    .color(Color::ORANGE);
            }),
            after_draw: None,
            pos: Position(0., 0.),
        },
    );
    fn draw_unit_info(unit: &Unit, pos: Position, font: &Font, draw: &mut Draw) {
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
        draw.text(font, damage_info)
            .size(10.)
            .position(pos.0, pos.1 + 92.);
        draw.text(font, move_info)
            .size(10.)
            .position(pos.0, pos.1 + 102.);

        draw.text(font, defence_info)
            .size(10.)
            .position(pos.0 + 40., pos.1 + 92.);
        draw.text(font, speed_info)
            .size(10.)
            .position(pos.0 + 50., pos.1 + 102.);

        draw.text(font, health_info)
            .size(10.)
            .position(pos.0 + 46., pos.1 + 112.)
            .h_align_center();
    }
    fn unit_card_draw(
        pos: Position,
        _: &mut App,
        _: &mut Graphics,
        assets: &HashMap<&'static str, HashMap<String, Asset<Texture>>>,
        font: &Font,
        battle: &mut BattleInfo,
        gamemap: &mut GameMap,
        draw: &mut Draw,
        army: usize,
        index: usize,
    ) {
        let bas_army = army;
        let army = if army == 0 {
            battle.army1
        } else {
            battle.army2
        };

        let field = field_type(index, *MAX_TROOPS);
        let cell_texture = assets
            .get("assets/Window")
            .unwrap()
            .get(match field {
                Field::Back => "backyard.png",
                Field::Front => "front.png",
                Field::Reserve => "tent.png",
            })
            .unwrap()
            .lock()
            .unwrap();
        draw.image(&cell_texture).position(pos.0, pos.1);
        let under_cell_texture = assets
            .get("assets/Window")
            .unwrap()
            .get("undercell.png")
            .unwrap()
            .lock()
            .unwrap();
        draw.image(&under_cell_texture).position(pos.0, pos.1 + 92.);
        if let Some(troop) = gamemap.armys[army as usize].get_troop(index) {
            let troop = troop.get();
            let unit = &troop.unit;
            let texture = assets
                .get("assets/Icons")
                .unwrap()
                .get(&*format!("unit_{}.png", unit.info.icon_index))
                .unwrap()
                .lock()
                .unwrap();
            let stats = unit.modified;
            // if <UnitPos as Into<usize>>::into(troop.pos) != index {
            // 	return;
            //}
            let size = unit.info.size;
            draw.image(&texture)
                .position(pos.0, pos.1)
                .size(size.0 as f32 * 92., size.1 as f32 * 92.);
            let size = (1, 1);
            {
                let pos = pos + Position((size.0 - 1) as f32 * 46., (size.1 - 1) as f32 * 92.);
                draw.rect((pos.0, pos.1 + 92.), (92., 50.))
                    .color(if troop.is_main {
                        Color::RED
                    } else if troop.is_free {
                        Color::BLUE
                    } else {
                        Color::BROWN
                    });
                draw_unit_info(unit, pos, font, draw);
            }
            draw.text(font, &*format!("{};{};{};{}", army, index, pos.0, bas_army))
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
                .color(Color::from_rgba(
                    1.,
                    0.,
                    0.,
                    1. - stats.hp as f32 / stats.max_hp as f32,
                ));
            if let Some(active_unit) = battle.active_unit {
                if active_unit.0 == army
                    && let Some(index) = gamemap.armys[army].hitmap[index]
                {
                    if active_unit.1 == index {
                        draw.rect((pos.0, pos.1), (92., 92.))
                            .color(Color::TRANSPARENT)
                            .stroke_color(Color::from_rgba(0., 255., 0., 0.3))
                            .stroke(10.);
                    }
                } else if let Some(can_interact) = &battle.can_interact {
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
    }
    type AssetsMap = HashMap<&'static str, HashMap<String, Asset<Texture>>>;
    fn handle_action_result(
        res: ActionResult,
        active_unit: (usize, usize),
        to: (usize, usize),
    ) -> AnimationTime<AssetsMap> {
        let texture = match res {
            ActionResult::Move => {
                |assets: &AssetsMap| assets.get_texture("assets/Window", "buff.png").clone()
            }
            ActionResult::Buff => {
                |assets: &AssetsMap| assets.get_texture("assets/Window", "buff.png").clone()
            }
            ActionResult::Debuff => {
                |assets: &AssetsMap| assets.get_texture("assets/Window", "debuff.png").clone()
            }
            ActionResult::Melee => {
                |assets: &AssetsMap| assets.get_texture("assets/Window", "melee.png").clone()
            }
            ActionResult::Ranged => {
                |assets: &AssetsMap| assets.get_texture("assets/Window", "ranged.png").clone()
            }
        };
        let line = *MAX_TROOPS / MAX_LINES;
        AnimationTime::new(
            Animation::new(
                MovementChange::new(
                    (
                        (active_unit.0 % line) as f32 * BETWEEN_CELLS + BETWEEN_CELLS / 2. - 25.,
                        ((active_unit.1 as i64).abs() * 500) as f32
                            + (((to.0 as i64 / line as i64).max(MAX_LINES as i64 - 1)).abs() as f32
                                * BETWEEN_CELLS) as f32
                            - 25.,
                    ),
                    (
                        (to.0 % line) as f32 * BETWEEN_CELLS + BETWEEN_CELLS / 2. - 25.,
                        ((to.1 as i64).abs() * 500) as f32
                            + (((to.0 as i64 / line as i64).max(MAX_LINES as i64 - 1)).abs() as f32
                                * BETWEEN_CELLS) as f32
                            - 25.,
                    ),
                ),
                SizeChange::new((50., 50.), (60., 60.)),
                (Instant::now(), Duration::from_secs(1)),
                (false, false),
            ),
            texture,
            Duration::from_secs(1),
        )
    }
    fn handle_animations(
        state: &mut State,
        to: (usize, usize),
        res: Option<(ActionResult, (usize, usize))>,
    ) {
        let Some((res, mut active_unit)) = res else {
            return;
        };
        //let Some(mut active_unit) = state.battle.active_unit.clone() else {return;};
        let Some(battle) = &state.battle else {
            return;
        };
        let gamemap = &state.gamemap;
        let index = active_unit.1;

        active_unit.1 = if active_unit.0 == battle.army1 { 0 } else { 1 };
        let active_index: usize = gamemap.armys[active_unit.0].troops[index].get().pos.into();
        active_unit.0 = active_index;
        state
            .animations
            .push(handle_action_result(res, active_unit, to));
    }
    const BETWEEN_CELLS: f32 = 102.;
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
                                                to_draw: |drawing, app, _, gfx, _, state: &mut State, draw| {
													let Some(battle) = &mut state.battle else { return; };
                                                    unit_card_draw(drawing.pos, app, gfx, &state.assets, &state.fonts[0], battle, &mut state.gamemap, draw, 0, 0 * *MAX_TROOPS / 2 + (drawing.pos.0 / BETWEEN_CELLS) as usize);
                                                }
                                            },
                                            Rect { pos: Position(0., 0.), size: Size(92., 92.) }
                                        ).if_clicked(|button, _app, _assets, _plugins, state| {
                                            let index = (button.rect.pos.0 / BETWEEN_CELLS) as usize;
											if !state.animations.is_empty() {return;}
											let Some(battle) = &mut state.battle else { return; };
											if let Some(_) = battle.winner { state.menu_id = Menu::Start as usize; return; }
                                            let res = handle_action(Action::Cell(index, 0), battle, &mut state.gamemap.armys);
											handle_animations(state, (index, 0), res);
                                        }).build().unwrap()
                                    }).collect::<Vec<_>>())
                                .interval(Position(10., 0.))
                                .build()?,
                            container((0..half_troops).map(|_| {
                                button(
                                    Drawing {
                                        pos: Position(0., 0.),
                                        to_draw: |drawing, app, _, gfx, _, state: &mut State, draw| {
											let Some(battle) = &mut state.battle else { return; };
                                            unit_card_draw(drawing.pos, app, gfx, &state.assets, &state.fonts[0], battle, &mut state.gamemap, draw, 0, *MAX_TROOPS / 2 + (drawing.pos.0 / BETWEEN_CELLS) as usize);
                                        }
                                    },
                                    Rect { pos: Position(0., 0.), size: Size(92., 92.) }
                                ).if_clicked(|button, _app, _assets, _plugins, state| {
                                    let index = (button.rect.pos.0 / BETWEEN_CELLS) as usize + *MAX_TROOPS / 2;
									if !state.animations.is_empty() {return;}
									let Some(battle) = &mut state.battle else { return; };
									if let Some(_) = battle.winner { state.menu_id = Menu::Start as usize; return; }
                                    let res = handle_action(Action::Cell(index, 0), battle, &mut state.gamemap.armys);
									handle_animations(state, (index, 0), res);
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
                                        to_draw: |drawing, app, _, gfx, _, state: &mut State, draw| {
											let Some(battle) = &mut state.battle else { return; };
                                            unit_card_draw(drawing.pos, app, gfx, &state.assets, &state.fonts[0], battle, &mut state.gamemap, draw, 1, *MAX_TROOPS / 2 + (drawing.pos.0 / BETWEEN_CELLS) as usize);
                                        }
                                    },
                                    Rect { pos: Position(0., 0.), size: Size(92., 92.) }
                                ).if_clicked(|button, _app, _assets, _plugins, state| {
                                    let index = (button.rect.pos.0 / BETWEEN_CELLS) as usize + *MAX_TROOPS / 2;
8									if !state.animations.is_empty() {return;}
									let Some(battle) = &mut state.battle else { return; };
									if let Some(_) = battle.winner { state.menu_id = Menu::Start as usize; return; }
                                    let res = handle_action(Action::Cell(index, 1), battle, &mut state.gamemap.armys);
									handle_animations(state, (index, 1), res);
                                }).build().unwrap()
                            }).collect::<Vec<_>>())
                                .interval(Position(10., 0.))
                                .build()?,
                            container((0..half_troops).map(|_| {
                                button(
                                    Drawing {
                                        pos: Position(0., 0.),
                                        to_draw: |drawing, app, assets, gfx, plugins, state: &mut State, draw| {
											let Some(battle) = &mut state.battle else { return; };
                                            unit_card_draw(drawing.pos, app, gfx, &state.assets, &state.fonts[0], battle, &mut state.gamemap, draw, 1, 0 + (drawing.pos.0 / BETWEEN_CELLS) as usize);
                                        }
                                    },
                                    Rect { pos: Position(0., 0.), size: Size(92., 92.) }
                                ).if_clicked(|button, _app, _assets, _plugins, state| {
                                    let index = (button.rect.pos.0 / BETWEEN_CELLS) as usize;
									if !state.animations.is_empty() {return;}
									let Some(battle) = &mut state.battle else { return; };
									if let Some(_) = battle.winner { state.menu_id = Menu::Start as usize; return; }
                                    let res = handle_action(Action::Cell(index, 1), battle, &mut state.gamemap.armys);
									handle_animations(state, (index, 1), res);
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
                                .to_draw(|drawing, _app, _assets, _gfx, _plugins, state, draw| {
                                    let pos = drawing.pos;
                                    draw.image(&state.assets.get("assets/Icons").unwrap().get(&*format!("unit_{}.png", get_menu_value_num(state, "battle_unit_stat").unwrap_or(1)-1)).unwrap().lock().unwrap())
                                        .position(pos.0, pos.1);
									for animation in &state.animations {
										animation.draw(&state.assets, draw, Instant::now());
									}
                                })
                                .build()?
                            )
                            .build()?
                        ),
                        Box::new(
                            single(text("Статистика".into())
                                   .size(20.0)
                                   .max_width((MONITOR_SIZE.lock().unwrap().0 - 900.).max(100.))
                                .build()?
                            )
                            .after_draw(|container, app, _assets, _plugins, state: &mut State| {
                                if app.keyboard.is_down(KeyCode::Escape) { state.menu_id = Menu::Main as usize; }
                                if get_menu_value_num(state, "battle_unit_stat_changed").unwrap_or(0) == 1 {
                                    let num = get_menu_value_num(state, "battle_unit_stat").unwrap_or(1);
                                    match &mut container.inside {
                                        Some::<Text<State, String>>(text) => {
                                            text.text = state.units.get(num as usize).unwrap().to_string();
                                            set_menu_value_num(state, "battle_unit_stat_changed", 0);
                                        }
                                        None => {}
								}   }
                            })
                            .pos(Position(900., 200.))
                            .build()?
                        ),
                    ],
                    pos: Position(0., 0.)
                })
            ],
            pos: Position(0., 0.),
            align_direction: Direction::Bottom,
            interval: Position(0., 0.)
        }),
        on_draw: Some(|_container, _app, _assets, _gfx, _plugins, state: &mut State, draw| {
			let size = *MONITOR_SIZE.lock().unwrap();
			draw.image(&state.get_texture("assets/Window", "red.png"))
				.size(2000., 2001.)
				.position(0., 0.);
			draw
                .rect((0., 0.), size)
                .color(Color::from_rgba(0., 0., 0., 0.5))
                .blend_mode(BlendMode::NORMAL);
        }),
        after_draw: Some(|_container, app, _assets, _plugins, state: &mut State| {
            if app.keyboard.is_down(KeyCode::Escape) { state.menu_id = Menu::Main as usize; }
            if app.keyboard.was_pressed(KeyCode::Space) {
				let Some(battle) = &mut state.battle else { return; };
				if let Some(active_unit) = battle.active_unit {
					handle_action(Action::Cell(active_unit.1, active_unit.0), battle, &mut state.gamemap.armys);
				}
            }
        }),
        pos: Position(0., 0.)
    });

    hashmap.insert(Menu::ConnectBattle as usize, SingleContainer {
        inside: Some(DynContainer {
            inside: vec![
                Box::new(StraightDynContainer {
                    inside: vec![
                        Box::new(single(container(vec![
                                    container((0..half_troops).map(|_| {
                                        button(
                                            Drawing {
                                                pos: Position(0., 0.),
                                                to_draw: |drawing, app, _, gfx, _, state: &mut State, draw| {
													let Some(conn) = &mut state.connection else { return; };
													let gamemap = &mut conn.gamemap;
													let Some(battle) = &mut conn.battle else { return; };
                                                    unit_card_draw(drawing.pos, app, gfx, &state.assets, &state.fonts[0], battle, gamemap, draw, 0, 0 * *MAX_TROOPS / 2 + (drawing.pos.0 / BETWEEN_CELLS) as usize);
                                                }
                                            },
                                            Rect { pos: Position(0., 0.), size: Size(92., 92.) }
                                        ).if_clicked(|button, _app, _assets, _plugins, state| {
                                            let index = (button.rect.pos.0 / BETWEEN_CELLS) as usize;
											handle_server_action(&mut state.connection, (index, 0));
                                        }).build().unwrap()
                                    }).collect::<Vec<_>>())
                                .interval(Position(10., 0.))
                                .build()?,
                            container((0..half_troops).map(|_| {
                                button(
                                    Drawing {
                                        pos: Position(0., 0.),
                                        to_draw: |drawing, app, _, gfx, _, state: &mut State, draw| {
											let Some(conn) = &mut state.connection else { return; };
											let gamemap = &mut conn.gamemap;
											let Some(battle) = &mut conn.battle else { return; };
                                            unit_card_draw(drawing.pos, app, gfx, &state.assets, &state.fonts[0], battle, gamemap, draw, 0, *MAX_TROOPS / 2 + (drawing.pos.0 / BETWEEN_CELLS) as usize);
                                        }
                                    },
                                    Rect { pos: Position(0., 0.), size: Size(92., 92.) }
                                ).if_clicked(|button, _app, _assets, _plugins, state| {
                                    let index = (button.rect.pos.0 / BETWEEN_CELLS) as usize + *MAX_TROOPS / 2;
									handle_server_action(&mut state.connection, (index, 0));
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
                                        to_draw: |drawing, app, _, gfx, _, state: &mut State, draw| {
											let Some(conn) = &mut state.connection else { return; };
											let gamemap = &mut conn.gamemap;
											let Some(battle) = &mut conn.battle else { return; };
                                            unit_card_draw(drawing.pos, app, gfx, &state.assets, &state.fonts[0], battle, gamemap, draw, 1, *MAX_TROOPS / 2 + (drawing.pos.0 / BETWEEN_CELLS) as usize);
                                        }
                                    },
                                    Rect { pos: Position(0., 0.), size: Size(92., 92.) }
                                ).if_clicked(|button, _app, _assets, _plugins, state| {
                                    let index = (button.rect.pos.0 / BETWEEN_CELLS) as usize + *MAX_TROOPS / 2;
									handle_server_action(&mut state.connection, (index, 1));
                                }).build().unwrap()
                            }).collect::<Vec<_>>())
                                .interval(Position(10., 0.))
                                .build()?,
                            container((0..half_troops).map(|_| {
                                button(
                                    Drawing {
                                        pos: Position(0., 0.),
                                        to_draw: |drawing, app, assets, gfx, plugins, state: &mut State, draw| {
											let Some(conn) = &mut state.connection else { return; };
											let gamemap = &mut conn.gamemap;
											let Some(battle) = &mut conn.battle else { return; };
                                            unit_card_draw(drawing.pos, app, gfx, &state.assets, &state.fonts[0], battle, gamemap, draw, 1, 0 + (drawing.pos.0 / BETWEEN_CELLS) as usize);
                                        }
                                    },
                                    Rect { pos: Position(0., 0.), size: Size(92., 92.) }
                                ).if_clicked(|button, _app, _assets, _plugins, state| {
                                    let index = (button.rect.pos.0 / BETWEEN_CELLS) as usize;
									handle_server_action(&mut state.connection, (index, 1));
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
                            single(text("Статус".to_string())
                                   .size(20.0)
                                   .max_width((MONITOR_SIZE.lock().unwrap().0 - 900.).max(100.))
                                .build()?
                            )
							.on_draw(|_, _, _, _, _, state: &mut State, draw| {
								for animation in &state.animations {
										animation.draw(&state.assets, draw, Instant::now());
								}
							})
                            .after_draw(|container, app, _assets, _plugins, state: &mut State| {
                                if app.keyboard.is_down(KeyCode::Escape) { state.menu_id = Menu::Main as usize; }
								if app.keyboard.was_pressed(KeyCode::C) {
									let (client, transport) = create_renet_client();
									state.connection = Some(
										ConnectionManager {
											con: Connection::Client(
												ClientConnection {
													client: Box::new(client),
													transport: Box::new(transport)
												}
											),
											gamemap: state.gamemap.clone(),
											battle: state.battle.clone(),
											events: state.gameevents.clone(),
											last_updated: Instant::now()
										}
									);
								}
								if app.keyboard.was_pressed(KeyCode::H) {
									state.connection = Some(
										ConnectionManager {
											con: Connection::Host(
												GameServer::new(true)
											),
											gamemap: state.gamemap.clone(),
											battle: state.battle.clone(),
											events: state.gameevents.clone(),
											last_updated: Instant::now()
										}
									);
								}
                                if let Some(conn) = &mut state.connection {
                                    container.inside.as_mut().unwrap().text = match &conn.con {
										Connection::Client(client) => {
											if client.client.is_connected() {
												"Client-Connection Connected"
											} else if client.client.is_connecting() {
												"Client-Connection Connecting"
											} else {
												"Client-Connection"
											}
										}
                                        Connection::Host(server) => {
											"Server-Connection"
										}
									}.to_string();
									match conn.updates(&state.units, &state.objects) {
										(Some(menu), _) => {
											state.menu_id = menu;
										},
										(_, Some(text)) => {
											set_menu_value_num(state, "start_menu", 4);        
											set_menu_value_str(state, "current_message", text);
										},
										_ => {}
									}
								}
                            })
                            .pos(Position(900., 200.))
                            .build()?
                        ),
                    ],
                    pos: Position(0., 0.)
                })
            ],
            pos: Position(0., 0.),
            align_direction: Direction::Bottom,
            interval: Position(0., 0.)
        }),
        on_draw: Some(|_container, _app, _assets, _gfx, _plugins, state: &mut State, draw| {
			let size = *MONITOR_SIZE.lock().unwrap();
			draw.clear(Color::ORANGE);
			draw.image(&state.get_texture("assets/Window", "red.png"))
				.size(size.0, size.1)
				.position(0., 0.);
			draw
                .rect((0., 0.), size)
                .color(Color::from_rgba(0., 0., 0., 0.2));

        }),
        after_draw: Some(|_container, app, _assets, _plugins, state: &mut State| {
            if app.keyboard.is_down(KeyCode::Escape) { state.menu_id = Menu::Main as usize; }
            if app.keyboard.was_pressed(KeyCode::Space) {
				let Some(battle) = &mut state.battle else { return; };
				if let Some(active_unit) = battle.active_unit {
					handle_action(Action::Cell(active_unit.1, active_unit.0), battle, &mut state.gamemap.armys);
				}
            }
        }),
        pos: Position(0., 0.)
    });
    dbg!(size_of::<State>());
    Ok(())
}

type Tilemap<T> = [[T; MAP_SIZE]; MAP_SIZE];
fn gen_army_troops(rng: &mut ThreadRng, units: &Vec<Unit>, army: usize) -> Vec<TroopType> {
    let mut troops = (0..*MAX_TROOPS / 2)
        .map(|i| {
            Troop {
                was_payed: true,
                is_free: false,
                is_main: false,
                pos: UnitPos::from_index(i),
                custom_name: None,
                unit: {
                    let mut unit = loop {
                        let unit = units[rng.gen_range(1..100)].clone();
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
        &mut (0..*MAX_TROOPS / 2)
            .map(|i| {
                Troop {
                    was_payed: true,
                    is_free: false,
                    is_main: false,
                    pos: UnitPos::from_index(i),
                    custom_name: None,
                    unit: {
                        let mut unit = loop {
                            let unit = units[rng.gen_range(1..100)].clone();
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
fn gen_decomap(seeds: (u32, u32), first_tree_index: usize) -> Tilemap<Option<usize>> {
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
            color.a = distance(tex_mid, v_uvs)/100.;
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
fn load_assets(
    gfx: &mut Graphics,
    assets: &mut HashMap<&'static str, HashMap<String, Asset<Texture>>>,
    req_assets: Vec<String>,
    path: &'static str,
) -> Result<(), String> {
    let mut asset_map = HashMap::new();
    for req in req_assets {
        asset_map.insert(req.clone(), load_asset(gfx, &format!("{path}/{req}"))?);
    }
    assets.insert(path, asset_map);
    Ok(())
}
fn setup(app: &mut App, app_assets: &mut Assets, gfx: &mut Graphics) -> State {
    let mut assets = HashMap::new();
    assets.insert("assets/Terrain", HashMap::new());
    for tile in TILES {
        let path = tile.sprite();
        let asset = assets.get_mut("assets/Terrain").unwrap().insert(
            path.to_string(),
            load_asset(gfx, &format!("assets/Terrain/{path}")).unwrap(),
        );
    }
    assets.insert("assets/Armys", {
        let mut mapa = HashMap::new();
        mapa.insert(
            "Army.png".into(),
            load_asset(gfx, &format!("assets/Army.png")).unwrap(),
        );
        mapa
    });
    assets.insert("assets/Window", {
        let mut mapa = HashMap::new();
        for asset in [
            "front.png",
            "backyard.png",
            "tent.png",
            "undercell.png",
            "melee.png",
            "ranged.png",
            "buff.png",
            "debuff.png",
            "button.png",
            "buttonblue.png",
            "cursor.png",
            "Menu.png",
            "gold.png",
            "red.png",
        ] {
            mapa.insert(
                asset.to_string(),
                load_asset(gfx, &format!("assets/Window/{asset}")).unwrap(),
            );
        }
        mapa
    });

    let settings = parse_settings();

    app.window().set_fullscreen(settings.fullscreen);
    app.window()
        .set_size(settings.init_size.0, settings.init_size.1);
    let req_assets = parse_items(None, &settings.locale);
    load_assets(gfx, &mut assets, req_assets.1, req_assets.0).expect("Loading items assets failed");
    {
        let locale = &mut LOCALE.lock().unwrap();
        dbg!(&settings);
        locale.set_lang((&settings.locale, &settings.additional_locale));
        parse_locale(&[&settings.locale, &settings.additional_locale], locale);
    }
    let res = parse_units(None);
    if let Err(err) = res {
        log::error!("{}", err);
        panic!("{}", err);
    }
    let Ok((units, req_assets)) = res else {
        panic!("Unit parsing error")
    };
    load_assets(gfx, &mut assets, req_assets.1, req_assets.0).expect("Loading units assets failed");
    let (objects, req_assets) = parse_objects();
    load_assets(gfx, &mut assets, req_assets.1, req_assets.0)
        .expect("Loading objects assets failed");

    let (mut gamemap, gameevents) = parse_story(
        &units,
        &objects,
        &settings.locale,
        &settings.additional_locale,
    );
    gamemap.calc_hitboxes(&objects);

    let terrain = assets.get("assets/Terrain").unwrap();
    let mut draw: Draw = gfx.create_draw();
    for i in 0..MAP_SIZE {
        for j in 0..MAP_SIZE {
            let asset = terrain.get(TILES[gamemap.tilemap[i][j]].sprite()).unwrap();
            draw.image(&*asset.lock().unwrap())
                .position(i as f32 * 52., j as f32 * 40.)
                .size(52., 40.);
        }
    }
    let texture = gfx
        .create_render_texture(MAP_SIZE as u32 * 52, MAP_SIZE as u32 * 40)
        .build()
        .unwrap();
    gfx.render_to(&texture, &draw);
    assets.insert("assets/Map", {
        let mut mapa = HashMap::new();
        mapa.insert(
            "Map".to_string(),
            Asset::from_data("map", texture.take_inner()),
        );
        mapa
    });
    // let tilemap = gen_tilemap();
    // gamemap.tilemap = tilemap.0;
    // gamemap.decomap = gen_decomap(
    // 	tilemap.1,
    //     objects
    //         .iter()
    //         .position(|obj| obj.path == "Tree0.png")
    //         .unwrap(),
    // );
    let mut state = State {
        execution_queue: VecDeque::new(),
        fonts: vec![
            gfx.create_font(include_bytes!("Benguiat Rus Regular.ttf"))
                .expect("shit happens"),
            gfx.create_font(include_bytes!("Z003-MediumItalic.ttf"))
                .expect("shit happens"),
            gfx.create_font(include_bytes!("Ru_Gothic.ttf"))
                .expect("shit happens"),
        ],
        animations: Vec::new(),
        draw: gfx.create_draw(),
        frame: 0,
        pause: true,
        connection: None,
        gamemap,
        gameevents,
        gameloop_time: Duration::new(0, 0),
        units,
        menu_id: 0,
        menu_data: HashMap::new(),
        assets,
        battle: None,
        objects,
        shaders: gen_shaders(gfx),
    };
    state.gamemap.calc_hitboxes(&state.objects);
    let mut battle = state.battle.clone();
    state.battle = battle;
    let size = gfx.size();
    let size = (size.0 as f32, size.1 as f32);
    *MONITOR_SIZE.lock().unwrap() = size;
    gen_forms(size).expect("gen_forms failed:()");
    state
}
fn draw(
    app: &mut App,
    assets: &mut Assets,
    gfx: &mut Graphics,
    plugins: &mut Plugins,
    state: &mut State,
) {
    if !state.animations.is_empty() {
        let mut i = 0;
        loop {
            if state.animations[i].is_over(Instant::now()) {
                state.animations.remove(i);
                i -= 1;
            }
            if i + 1 >= state.animations.len() {
                break;
            }
            i += 1;
        }
    }
    let mut draw = gfx.create_draw();
    FORMS
        .lock()
        .unwrap()
        .get_mut(&state.menu_id)
        .unwrap()
        .draw(app, assets, gfx, plugins, state, &mut draw);
    gfx.render(&draw);
}
fn update(app: &mut App, assets: &mut Assets, plugins: &mut Plugins, state: &mut State) {
    FORMS
        .lock()
        .unwrap()
        .get_mut(&state.menu_id)
        .unwrap()
        .after(app, assets, plugins, state);
}
fn event(state: &mut State, event: notan::Event) {
    match event {
        notan::Event::WindowResize { width, height } => {
            *MONITOR_SIZE.lock().unwrap() = (width as f32, height as f32);
            gen_forms((width as f32, height as f32)).ok();
        }
        _ => {}
    }
}
#[notan_main]
fn main() -> Result<(), String> {
    let win = WindowConfig::new()
        .set_title("Discord Times: Remastered")
        .set_vsync(true)
        .set_high_dpi(true)
        .set_size(1600, 1200)
        .set_fullscreen(false)
        .set_resizable(true);
    notan::init_with(setup)
        .add_config(win)
        .add_config(TextConfig)
        .add_config(DrawConfig)
        .add_config(log::LogConfig::info())
        .draw(draw)
        .event(event)
        .update(update)
        .build()
}
