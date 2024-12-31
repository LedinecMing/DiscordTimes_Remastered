use ahash::RandomState;
use dt_lib::{
    battle::{army::*, battlefield::*, troop::Troop}, hwid, items::item::*, locale::{parse_locale, Locale}, map::{
        convert::{convert_dtm_map, parse_dtm_map}, event::{execute_event, Event as GameEvent, Execute}, map::*, object::ObjectInfo, tile::*
    }, network::GameServer, parse::{collect_errors, parse_items, parse_objects, parse_settings, parse_story, parse_units}, time::time::Data as TimeData, units::{
        unit::{ActionResult, Unit, UnitPos},
        unitstats::ModifyUnitStats,
    }
};
use futures_util::StreamExt;
use macroquad::{
    prelude::*, telemetry::Zone, ui::{
        hash, root_ui,
        widgets::{self, Window}, Skin, Style,
    }
};
use dt_client::*;
use once_cell::sync::Lazy;
use tokio::runtime::Runtime;
use std::{
    collections::HashMap, num::Saturating, ops::Index, sync::{Mutex, RwLock}
};
#[derive(Debug)]
struct Assets {
    inner: HashMap<String, Texture2D, RandomState>,
	fonts: HashMap<&'static str, Font>
}
impl Assets {
    fn new(inner: HashMap<String, Texture2D, RandomState>, fonts: HashMap<&'static str, Font>) -> Self {
        Assets { inner, fonts }
    }
    fn get(&self, key: &String) -> &Texture2D {
        self.inner.get(key).unwrap_or_else(|| panic!("No asset {key}"))
    }
	fn get_font(&self, key: &'static str) -> &Font {
		self.fonts.get(&key).unwrap_or_else(|| panic!("No font {key}"))
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
static UNITS: Lazy<RwLock<Vec<Unit>>> = Lazy::new(|| RwLock::new(vec![]));
static OBJECTS: Lazy<RwLock<Vec<ObjectInfo>>> = Lazy::new(|| RwLock::new(vec![]));
macro_rules! units {
    () => {
        &UNITS.read().unwrap()
    };
}
macro_rules! units_mut {
    () => {
        &mut UNITS.write().unwrap()
    };
}
macro_rules! objects {
    () => {
        &OBJECTS.read().unwrap()
    };
}
macro_rules! objects_mut {
    () => {
        &mut OBJECTS.write().unwrap()
    };
}
#[derive(Debug)]
struct State {
    pub assets: Assets,
    pub game: Game,
    pub ui: Ui,
	pub rt: Runtime
}
impl State {
    fn new() -> Self {
        todo!()
    }
}
async fn load_assets(req_assets_list: &[(&str, Vec<String>)], fonts: HashMap<&'static str, Font>) -> Assets {
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
        panic!("No asssets!")
    };
    println!("{}", error_collector.join("\n"));
    Assets::new(asset_names.into_iter().zip(assets).collect(), fonts)
}
async fn game_init() -> State {
    let settings = parse_settings();
    {
        let locale = &mut LOCALE.write().unwrap();
        locale.set_lang((&settings.locale, &settings.additional_locale));
        parse_locale(&[&settings.locale, &settings.additional_locale], locale);
    }
	let benguiat = load_ttf_font("Benguiat Rus Regular.ttf").await.expect("shit happened");
	let z003 = load_ttf_font("Z003-MediumItalic.ttf").await.expect("shit happened");
	let gothic = load_ttf_font("Ru_Gothic.ttf").await.expect("shit happened");
	let mut fonts = HashMap::new();
	fonts.insert("benguiat", benguiat);
	fonts.insert("z003", z003);
	fonts.insert("gothic", gothic);
    let assets = {
        let req_assets_items = parse_items(None, &settings.locale);
        let res = parse_units(None);
        if let Err(err) = res {
            error!("{}", err);
            panic!("{}", err);
        }
        let Ok((mut units, req_assets_units)) = res else {
            panic!("Unit parsing error")
        };
        units_mut!().append(&mut units);
        let (mut objects, req_assets_objects) = parse_objects();
        objects_mut!().append(&mut objects);
        dbg!(units!().len(), objects!().len());
        let req_assets_tiles = (
            "assets/Terrain",
            TILES.iter().map(|tile| tile.sprite().to_string()).chain(["grass_tileset.png".to_string()]).collect(),
        );
	    let req_assets_windows = ("assets/Window",
            ["front.png",
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
            "red.png"].map(|x| x.to_owned()).to_vec(),
        );

        let req_assets_list = [
            req_assets_objects,
            req_assets_items,
            req_assets_units,
            req_assets_tiles,
			req_assets_windows
        ];
        load_assets(&req_assets_list, fonts).await
    };
	let (mut gamemap, events) = (convert_dtm_map(parse_dtm_map(std::path::Path::new("./Maps_Rus/Проклятое озеро.DTm")).unwrap()), vec![]);
    //let (mut gamemap, events) = parse_story(
    //     units!(),
    //     objects!(),
    //     &settings.locale,
    //     &settings.additional_locale,
    // );
    gamemap.calc_hitboxes(objects!());
	let mut battle = BattleInfo::new(&mut gamemap.armys, 0, 1);
	battle.start(&mut gamemap.armys);
	let rt = tokio::runtime::Builder::new_current_thread().enable_io().enable_time().build().unwrap();
    State {
		rt,
        assets,
        ui: Ui {
            main: Menu::Main,
            stack: Vec::new(),
        },
        game: Game {
            gamemap,
            battle: Some(battle),
            variant: GameVariant::Single(Scenario {events} ),
        },
    }
}
#[derive(Debug)]
struct Scenario {
    pub events: Vec<GameEvent>,
}
#[derive(Debug, PartialEq, Eq)]
enum ConnectionStatus {
	NotFull,
	Full
}
#[derive(Debug)]
struct Online {
	conn: Connection,
	status: ConnectionStatus,
}
#[derive(Debug)]
enum GameVariant {
    Single(Scenario),
    Online(Online),
}
#[derive(Debug)]
struct Game {
	pub gamemap: GameMap,
	pub battle: Option<BattleInfo>,
	pub variant: GameVariant
}
fn window_conf() -> Conf {
    Conf {
        high_dpi: true,
        window_title: "DT REMASTERED".into(),
        ..Default::default()
    }
}
#[derive(Debug)]
enum Menu {
    Main,
    Map(Camera2D),
    Atlas,
    Battle,
	RoomCreation,
	RoomOnline,
    EventMessage,
}
#[derive(Debug)]
struct Ui {
    pub main: Menu,
    pub stack: Vec<Menu>,
}
fn is_clicked(area: Rect) -> bool {
	is_mouse_button_released(MouseButton::Left) && area.contains(mouse_position().into())
}
fn unit_card_battle(army: usize, unit_index: usize, battle: &mut BattleInfo, armies: &mut Vec<Army>, pos: (f32, f32), assets: &Assets) {
	let Some(unit) = armies[army].get_troop(unit_index) else {
		let texture = assets.get(&match field_type(unit_index, *MAX_TROOPS) {
			Field::Back => { "backyard.png" },
			Field::Front => { "front.png" },
			Field::Reserve => { "tent.png" }
		}.to_owned());
		let undercell = assets.get(&"undercell.png".to_owned());
		draw_texture(texture, pos.0, pos.1, WHITE);
		draw_texture(undercell, pos.0, pos.1 + 102., WHITE);
		return;
	};
	let troop = unit.get();
	let unit_texture = assets.get(&format!("unit_{}.png", troop.unit.info.icon_index));
	let is_active = battle.active_unit == Some(BattleUnit{ army, index: unit_index});
	let is_interactable = battle.can_interact.as_ref().is_some_and(|x| x.contains(&BattleUnit { army, index: unit_index}));
	//draw_texture_ex(undercell_texture, pos.0, pos.1, Color::default(), DrawTextureParams {
	//	dest_size: Some(Vec2::from_array([5., 5.])),
	//	..Default::default()
	//}	dest_size: Some(Vec2::from_array([5., 5.])),
	//);
	let bg_color = if troop.is_main {
		Color::from_rgba(168, 48, 48, 255)
	} else {
		Color::from_rgba(132, 46, 46, 255)
	};
	let size = troop.unit.info.size;
	let draw_size = (size.0 as f32 * 102., size.1 as f32 * 102.);
	let draw_rect = Rect::new(pos.0, pos.1, draw_size.0, draw_size.1);
	draw_rectangle(pos.0, pos.1, draw_size.0, draw_size.1 + 52., bg_color);
	draw_texture(unit_texture, pos.0, pos.1, Color::from_rgba(255, 255, 255, 255));
	let outline_color = if is_active {
		Some(Color::from_rgba(91, 255, 66, 128 + (get_time().sin() * 128.) as u8))
	} else if is_interactable {
		Some(Color::from_rgba(255, 22, 22, 128 + (get_time().sin() * 128.) as u8))
	} else { None };
	if let Some(outline_color) = outline_color {
		draw_rectangle_lines(pos.0, pos.1, draw_size.0, draw_size.1, 15., outline_color);
	}
	if is_clicked(draw_rect) {
		draw_rectangle_lines(pos.0, pos.1, draw_size.0, draw_size.1, 15., BLACK);
		handle_action((unit_index, army), battle, armies);
	}
}
fn draw_battle(assets: &Assets, game: &mut Game) {
	let (armies, battle) = (&mut game.gamemap.armys, &mut game.battle);
	let Some(battle) = battle.as_mut() else { return; };
	let half_troops = *MAX_TROOPS / 2;
    for army in 0..=1 {
        for row in 0..=1 {
            for i in 0..(*MAX_TROOPS / 2) {
                let mut pos = (0., 0.);
                pos.0 = i as f32 * 102.;
                pos.1 = army as f32 * 360. + row as f32 * 152.;
                // f(x) = { 0, 6, 6, 0 } where x = { {0, 0}, {0, 1}, {1, 0}, {1, 1} }
                // abs(0 * 6 - 0 * 6) = 0
                // abs(0 * 6 - 1 * 6) = 6
                // abs(1 * 6 - 0 * 6) = 6
                // abs(1 * 6 - 1 * 6) = 0
				draw_rectangle(pos.0, pos.1, 92., 102., Color::from_rgba(0, 0, 0, 1));
				let army = if army == 0 {
					battle.army1
				} else {
					battle.army2
				} as usize;
				let unit_index = i + (army as i64 - row as i64).abs() as usize * half_troops;
				unit_card_battle(army, unit_index, battle, armies, pos, assets);
            }
        }
    }
}
fn count_tileset_index(map: &GameMap, pos: (usize, usize)) -> usize {
	const DIRT: usize = 0;
	let is_dirt = |pos: (usize, usize)| if map.tilemap[pos.min((map.tilemap.size - 1, map.tilemap.size - 1))] == 6 { 1 } else { 0 };
	
	let bitmask = 
		is_dirt((pos.0 + 1, pos.1 + 1)) +
		is_dirt((pos.0.saturating_add_signed(-1), pos.1 + 1)) * 2 +
		is_dirt((pos.0.saturating_add_signed(-1), pos.1.saturating_add_signed(-1))) * 4 +
		is_dirt((pos.0 + 1, pos.1.saturating_add_signed(-1))) * 8;
	bitmask
}
const SIZE: (f32, f32) = (256., 242.);
fn draw_map(assets: &Assets, camera: &mut Camera2D, game: &mut Game) -> Option<Menu> {
    let (gamemap, battle) = (&mut game.gamemap, &mut game.battle);
    set_camera(camera);
    for i in 0..gamemap.tilemap.size {
        for j in 0..gamemap.tilemap.size {
			let tile_index = gamemap.tilemap[(j, i)];
            let tile = TILES[tile_index];
			let tileset_index = count_tileset_index(&gamemap, (j, i));
			
			draw_texture(
				assets.get(&tile.sprite().to_string()),
				i as f32 * SIZE.0,
				j as f32 * SIZE.1,
				WHITE,
			);
			if is_key_down(KeyCode::F) {
				continue
			}
			if tileset_index != 0 {
				draw_texture_ex(
					assets.get(&"grass_tileset.png".to_string()),
					i as f32 * SIZE.0,
					j as f32 * SIZE.1,
					WHITE,
					DrawTextureParams {
						source: Rect::new(
							(tileset_index % 4) as f32 * 256.,
							(tileset_index / 4) as f32 * 242.,
							256., 242.
						).into(),
						..Default::default()
					}
				)
			}
			//draw_text(&*format!("{};{}", i, j), i as f32 * 256., j as f32 * 242., 30., BLACK);
        }
    }
	if is_key_down(KeyCode::Enter) {
		battle.insert(BattleInfo::new(&mut gamemap.armys, 0, 1));
		return Some(Menu::Battle);
	}
	if is_key_down(KeyCode::Escape) {
		return Some(Menu::Main);
	}
    draw_text("Loading game map...", 0., 0., 0.5, BLACK);
    if is_key_down(KeyCode::W) {
        camera.offset += Vec2::new(0., -0.01)
    }
    if is_key_down(KeyCode::D) {
        camera.offset += Vec2::new(-0.01, 0.);
    }
    if is_key_down(KeyCode::A) {
        camera.offset += Vec2::new(0.01, 0.);
    }
    if is_key_down(KeyCode::S) {
        camera.offset += Vec2::new(0., 0.01);
    }
    if is_key_down(KeyCode::Equal) {
        camera.zoom /= Vec2::new(1.1, 1.1);
    }
    if is_key_down(KeyCode::Minus) {
        camera.zoom *= Vec2::new(1.1, 1.1);
    }
	return None;
}
fn process_event(state: &mut State, event: IncomingEvent) {
	let GameVariant::Online(conn) = &mut state.game.variant else { return; };
	match dbg!(event) {
		IncomingEvent::Game(game) => {
			state.game.gamemap.armys = game.0; state.game.battle = Some(game.1);
		},
		IncomingEvent::Info(text) => {
			match &*text {
				"Wait for another player" => {
					conn.status = ConnectionStatus::NotFull;
				},
				"Room full" => {
					conn.status = ConnectionStatus::Full;
				}
				_ => { dbg!(text); }
			}
		}
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
	let bg = root_ui().style_builder().background(state.assets.get(&"Menu.png".to_owned()).get_texture_data()).build();
	let button = root_ui().style_builder().text_color(WHITE).background(state.assets.get(&"button.png".to_owned()).get_texture_data()).with_font(state.assets.get_font("benguiat")).unwrap().build();
	let label = root_ui().style_builder().text_color(BLACK).with_font(state.assets.get_font("gothic")).unwrap().build();
	let skin = Skin {
		label_style: label,
		button_style: button.clone(),
		window_style: bg,
		editbox_style: button,
		..root_ui().default_skin()
	};
	let mut input = String::new();
	root_ui().push_skin(&skin);
    loop {
        clear_background(WHITE);
		match &mut state.ui.main {
			Menu::Main => {
				Window::new(hash!(), vec2(0., 0.), vec2(screen_height(), screen_width()))
					.titlebar(false)
					.ui(&mut *root_ui(), |ui| {
						ui.texture(state.assets.get(&"Menu.png".to_owned()).weak_clone(), screen_width(), screen_height());
						ui.label(Some((50., 50.).into()), "Discord Times");
						if ui.button(Some((50., 100.).into()), "Start") {
							state.ui.main = Menu::Map(Camera2D::from_display_rect(Rect::new(
								0.,
								0.,
								256. * 50.,
								242. * 30.,
							)));
						}
						if ui.button(Some((50., 150.).into()), "Atlas") {
							state.ui.main = Menu::Atlas;
						}
						if ui.button(Some((50., 250.).into()), "Battle") {
							state.ui.main = Menu::Battle;
						}
						if ui.button(Some((50., 300.).into()), "Mp") {
							state.ui.main = Menu::RoomCreation;
						}
					});
			}
			Menu::Map(camera) => {
				if let Some(menu) = draw_map(&state.assets, camera, &mut state.game) {
					state.ui.main = menu;
				};
			}
			Menu::Atlas => draw_texture(
				state
					.assets
					.inner
					.values()
					.skip(macroquad::time::get_time() as usize % state.assets.inner.len())
					.next()
					.unwrap(),
				0.,
				0.,
				WHITE,
			),
			Menu::Battle => {
				draw_battle(&state.assets, &mut state.game);
			},
			Menu::RoomCreation => {
				let connected = matches!(state.game.variant, GameVariant::Online(_));
				let mut connecting = false;
				Window::new(hash!(), vec2(0., 0.), vec2(screen_height(), screen_width()))
					.titlebar(false)
					.ui(&mut *root_ui(), |ui| {
						ui.label(Some((50., 50.).into()), "Discord Times");
						
						ui.input_text(hash!(), "Room Code: ", &mut input);
						if !connected && (ui.button(Some((150., 150.).into()), "Connect") || is_key_released(KeyCode::Enter)) {
							connecting = true;
						}
						if connected {
							ui.label(Some((150., 150.).into()), "Waiting for second player...");
						}
						if let GameVariant::Online(Online {status, .. }) = &state.game.variant {
							if status == &ConnectionStatus::Full {
								state.ui.main = Menu::Battle;
							}
						}
					});
				if connecting {
					let conn = connect(input.clone(), hwid::get_id().unwrap()).await;
					state.game.variant = GameVariant::Online(Online {conn, status: ConnectionStatus::NotFull });
					//state.rt.spawn(conn.incoming_events.for_each(|x| async { dbg!(x); }));
				}
				match &mut state.game.variant {
					GameVariant::Online(v) => {
						if let Some(event) = v.conn.req_one() {
							process_event(&mut state, event);
						}
					},
					_ => {}
				}
			}
			_ => {}
		}
        next_frame().await
    }
}
