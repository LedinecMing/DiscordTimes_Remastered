use ahash::RandomState;
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
    parse::{collect_errors, parse_items, parse_objects, parse_settings, parse_story, parse_units},
    time::time::Data as TimeData,
    units::{
        unit::{ActionResult, Unit, UnitPos},
        unitstats::ModifyUnitStats,
    },
};
use macroquad::{
    prelude::*,
    ui::{
        hash, root_ui,
        widgets::{self, Window},
    },
};
use once_cell::sync::Lazy;
use std::{
    collections::HashMap,
    ops::Index,
    sync::{Mutex, RwLock},
};
#[derive(Clone, Debug)]
struct Assets {
    inner: HashMap<String, Texture2D, RandomState>,
}
impl Assets {
    fn new(inner: HashMap<String, Texture2D, RandomState>) -> Self {
        Assets { inner: inner }
    }
    fn get(&self, key: &String) -> &Texture2D {
        self.inner.get(key).expect(&format!("No asset {key}"))
    }
}
impl Index<&String> for Assets {
    type Output = Texture2D;
    fn index(&self, index: &String) -> &Self::Output {
        self.get(index)
    }
}
static LOCALE: Lazy<RwLock<Locale>> =
    Lazy::new(|| RwLock::new(Locale::new("Rus".into(), "Eng".into())));
#[derive(Clone, Debug)]
struct State {
    pub assets: Assets,
    pub units: Vec<Unit>,
    pub objects: Vec<ObjectInfo>,
    pub game: Game,
    pub ui: Ui,
}
impl State {
    fn new() -> Self {
        todo!()
    }
}
async fn load_assets(req_assets_list: &[(&str, Vec<String>)]) -> Assets {
    let mut asset_names = Vec::new();
    let mut assets = Vec::new();
    let mut error_collector: Vec<String> = Vec::new();
    for req_assets in req_assets_list {
        for asset in &req_assets.1 {
            assets.push(collect_errors(
                load_texture(&format!("{}/{}", req_assets.0, &asset)).await,
                &mut error_collector,
                "Failed to load image asset",
            ));
            asset_names.push(asset.clone());
        }
    }
    let Ok(assets) = assets
        .into_iter()
        .map(|v| v.ok_or(()))
        .collect::<Result<Vec<Texture2D>, ()>>()
    else {
        panic!("{}", error_collector.join("\n"));
    };
    Assets::new(asset_names.into_iter().zip(assets).collect())
}
async fn game_init() -> State {
    let settings = parse_settings();
    {
        let locale = &mut LOCALE.write().unwrap();
        locale.set_lang((&settings.locale, &settings.additional_locale));
        parse_locale(&[&settings.locale, &settings.additional_locale], locale);
    }
    let req_assets_items = parse_items(None, &settings.locale);
    let res = parse_units(None);
    if let Err(err) = res {
        error!("{}", err);
        panic!("{}", err);
    }
    let Ok((units, req_assets_units)) = res else {
        panic!("Unit parsing error")
    };
    let (objects, req_assets_objects) = parse_objects();
    let req_assets_list = [req_assets_items, req_assets_objects, req_assets_units];
    let assets = load_assets(&req_assets_list).await;
    let (mut gamemap, events) = parse_story(
        &units,
        &objects,
        &settings.locale,
        &settings.additional_locale,
    );
    gamemap.calc_hitboxes(&objects);

    State {
        assets,
        units,
        objects,
        ui: Ui {
            main: Menu::Main,
            stack: Vec::new(),
        },
        game: Game::Single(Scenario {
            gamemap,
            battle: None,
            events,
        }),
    }
}
#[derive(Clone, Debug)]
struct Scenario {
    pub gamemap: GameMap,
    pub battle: Option<BattleInfo>,
    pub events: Vec<GameEvent>,
}
#[derive(Clone, Debug)]
enum Game {
    Single(Scenario),
    Online(ConnectionManager),
}
fn window_conf() -> Conf {
    Conf {
        high_dpi: true,
        window_title: "DT REMASTERED".into(),
        ..Default::default()
    }
}
#[derive(Clone, Copy, Debug)]
enum Menu {
    Main,
    Map,
    Battle,
    EventMessage,
}
#[derive(Clone, Debug)]
struct Ui {
    pub main: Menu,
    pub stack: Vec<Menu>,
}
fn draw_menu(menu: Menu, state: &mut State) {
    match menu {
        Menu::Main => {
            Window::new(hash!(), vec2(0., 0.), vec2(screen_height(), screen_width()))
                .titlebar(false)
                .ui(&mut *root_ui(), |ui| {
                    ui.label(Some((50., 50.).into()), "Discord Times");
                    if ui.button(Some((50., 100.).into()), "Start") {
                        state.ui.main = Menu::Map;
                    }
                });
        }
        Menu::Map => {}
        _ => {}
    }
}
fn draw_ui(state: &mut State) {
    draw_menu(state.ui.main, state);
    for menu in state.ui.stack.clone() {
        draw_menu(menu, state);
    }
}
#[macroquad::main(window_conf)]
async fn main() {
    clear_background(WHITE);
    draw_text(
        "Loading game assets...",
        0.,
        screen_height() / 2.,
        20.,
        BLACK,
    );
    next_frame().await;
    let mut state = game_init().await;
    loop {
        clear_background(WHITE);
        draw_ui(&mut state);
        next_frame().await
    }
}
