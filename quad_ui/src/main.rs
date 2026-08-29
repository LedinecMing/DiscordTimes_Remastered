#![windows_subsystem = "windows"]
mod egui_macroquad;

use egui_macroquad::*;
use ahash::RandomState;
use dt_client::*;
use dt_lib::{
    battle::{
        army::*,
        battlefield::*,
        control::{Control, Player},
        troop::Troop,
    }, effects::*, hwid, items::item::*, locale::{Locale, find_all_matches_in_string, parse_locale}, map::{
        convert::{convert_dtm_map, parse_dtm_map, parse_dtm_map_by_bytes, parse_dtm_vec},
        deco::MapDeco,
        event::{Event as GameEvent, Events, Execute, Message, execute_event},
        map::*,
        object::{MapBuildingdata, ObjectInfo, ObjectType},
        tile::*,
    }, mutrc::*, network::{GameServer, server::Executor}, parse::{
        FileAccess, collect_errors, parse_bonuses, parse_effects, parse_items, parse_objects, parse_settings, parse_story, parse_units
    }, registry::{self, GameInfo, Objects, Registry, Units}, time::time::Data as TimeData, units::{
        unit::{ActionResult, Unit, UnitInfo, UnitPos, UnitType,  display_unit},
        unitstats::ModifyUnitStats,
    }
};
use futures_util::StreamExt;
use macroquad::{
    prelude::*,
    telemetry::Zone,
    ui::{
        hash, root_ui,
        widgets::{self, Button, Group, Texture, Window},
        Drag, Skin, Style,
    },
};
use miniquad::{conf::Icon, log, window::screen_size};
use once_cell::sync::Lazy;
use std::{
    collections::HashMap,
    fmt::Display,
    future,
    num::Saturating,
    ops::{AddAssign, Index, Not},
    path::Path,
    sync::{Mutex, RwLock},
    thread::sleep,
    time::Duration,
};
use tokio::{runtime::Runtime, task::futures};

struct QuadFiles;
impl FileAccess for QuadFiles {
    async fn read(path: &str) -> Vec<u8> {
        load_file(path).await.unwrap()
    }
    async fn read_as_string(path: &str) -> String {
        load_string(path).await.unwrap()
    }
}

#[derive(Debug)]
struct Assets {
    inner: HashMap<String, Texture2D, RandomState>,
    fonts: HashMap<&'static str, Font>,
}
impl Assets {
    fn new(
        inner: HashMap<String, Texture2D, RandomState>,
        fonts: HashMap<&'static str, Font>,
    ) -> Self {
        Assets { inner, fonts }
    }
    fn get<T: Display + ToString + ?Sized>(&self, key: &T) -> &Texture2D {
        self.inner
            .get(&key.to_string())
            .unwrap_or_else(|| panic!("No asset {key}"))
    }
    fn get_font(&self, key: &'static str) -> &Font {
        self.fonts
            .get(&key)
            .unwrap_or_else(|| panic!("No font {key}"))
    }
}
const BENGUIAT: &'static str = "benguiat";
const GOTHIC: &'static str = "gothic";
const Z003: &'static str = "z003";

impl Index<&String> for Assets {
    type Output = Texture2D;
    fn index(&self, index: &String) -> &Self::Output {
        self.get(index)
    }
}

const CARD_SIZE: f32 = 160.;

#[derive(Debug)]
struct State {
    pub assets: Assets,
	pub registry: GameInfo,
    pub textures: RenderTextures,
    pub game: Game,
    pub ui: Ui,
	pub delta: f32,
    pub rt: Runtime,
}
async fn load_assets(
    req_assets_list: &[(&str, Vec<String>)],
    fonts: HashMap<&'static str, Font>,
) -> Assets {
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
    if error_collector.len() > 0 {
        panic!("{}", error_collector.join("\n"));
    }

    let Ok(assets) = assets
        .into_iter()
        .map(|v| v.ok_or(()))
        .collect::<Result<Vec<Texture2D>, ()>>()
    else {
        panic!("No asssets!")
    };
    Assets::new(asset_names.into_iter().zip(assets).collect(), fonts)
}
async fn load_map(map: &str, registry: &GameInfo) -> (GameMap, Events) {
    let bytes = load_file(&format!("Maps_Rus/{}", map)).await.unwrap();
    let (mut gamemap, events) = convert_dtm_map(parse_dtm_vec(bytes).unwrap(), registry);
    gamemap.calc_hitboxes(&registry.objects.inner);
    {
        let objects = &registry.objects.inner;
        let mut count = HashMap::new();
        for deco in gamemap.decomap.iter().filter(|&x| {
            objects
                .iter()
                .find(|el| match el.obj_type {
                    ObjectType::MapDeco { id } => id == x.index,
                    _ => false,
                })
                .is_none()
        }) {
            count
                .entry(deco.index.clone())
                .and_modify(|x: &mut usize| x.add_assign(1))
                .or_insert(1usize);
        }
        dbg!(count);
        export(&events, "events.ini");
    }
    if gamemap.armys.len() == 0 {
        gamemap.armys.push(Army::new(
            vec![SendMut::new(Troop::new((registry.units.inner[0].clone(), &registry.bonuses).into()))],
            ArmyStats {
                gold: 0,
                army_name: "".into(),
                mana: 0,
            },
            vec![],
            (1, 1),
            true,
            dt_lib::battle::control::Control::Player(0),
			registry
        ));
    }
    (gamemap, events)
}
async fn game_init() -> State {
	let mut registry = GameInfo::new();
    let settings = parse_settings::<QuadFiles>().await;
	let mut locale = Locale::new("Rus".into(), "Eng".into());
    {
        locale.set_lang((&settings.locale, &settings.additional_locale));
        parse_locale::<QuadFiles>(&[&settings.locale, &settings.additional_locale], &mut locale).await;
        dbg!(locale.keys_lack());
    }
	registry.locale = locale;
	parse_bonuses::<QuadFiles>(None, &mut registry).await;
	parse_effects::<QuadFiles>(None, &mut registry).await;
    let benguiat = load_ttf_font("Benguiat Rus Regular.ttf")
        .await
        .expect("shit happened");
    let z003 = load_ttf_font("Z003-MediumItalic.ttf")
        .await
        .expect("shit happened");
    let gothic = load_ttf_font("Ru_Gothic.ttf").await.expect("shit happened");
    let mut fonts = HashMap::new();
    fonts.insert(BENGUIAT, benguiat);
    fonts.insert(Z003, z003);
    fonts.insert(GOTHIC, gothic);
    let assets = {
        let req_assets_items = parse_items::<QuadFiles>(None, &settings.locale, &mut registry).await;
        
        let mut res = parse_units::<QuadFiles>(Some("Units.ini"), &mut registry).await;
        if let Ok(ref mut res) = &mut res {
            res.0 = "assets/Icons";
        }
        if let Err(err) = res {
            error!("{}", err);
            panic!("{}", err);
        }
        let Ok(req_assets_units) = res else {
            panic!("Unit parsing error")
        };
		dbg!(&req_assets_units);
        let req_assets_objects = parse_objects::<QuadFiles>(&mut registry).await;
        // let req_assets_tiles = (
        //     "assets/Terrain",
        //     TILES.iter().map(|tile| tile.sprite().to_string()).chain(["grass_tileset.png".to_string()]).collect(),
        //);
        let req_assets_windows = (
            "assets/Window",
            [
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
                "Paper.png",
                "gold.png",
                "red.png",
            ]
            .map(|x| x.to_owned())
            .to_vec(),
        );
        let req_assets_stats_list = (
            "assets/StatsIcons",
            [
                "crossed-swords.png",
                "hearts.png",
                "high-shot.png",
                "sprint.png",
                "mounted-knight.png",
                "breastplate.png",
                "wizard-staff.png",
                "heart-bottle.png",
                "spartan.png",
                "small-fire.png",
                "checked-shield.png",
                "raise-zombie.png",
                "poison-bottle.png",
            ]
            .map(|x| x.to_owned())
            .to_vec(),
        );
        let req_assets_terrain_list = (
            "assets/Terrain",
            [
                "Badground.png",
                "DeepSwamp.png",
                "DeepWater.png",
                "Desert.png",
                "Detail.png",
                "Dust.png",
                "FlameLand.png",
                "Land.png",
                "LowLand.png",
                "Plain.png",
                "Road.png",
                "Rock.png",
                "Shallow.png",
                "Snow.png",
                "Swamp.png",
                "Water.png",
            ]
            .map(|x| x.to_owned())
            .to_vec(),
        );
        let req_assets_armies_list = (
            "assets/Armies",
            [
                "феодал",
                "ГГрыцарь",
                "ГГархимаг",
                "ГГследопыт",
                "крестьянин",
                "разбойник",
                "мертвяк",
                "некромант",
                "призрак",
            ]
            .map(|x| {
                [x].repeat(64)
                    .iter()
                    .enumerate()
                    .map(|x| format!("{}/Unit_{}.png", x.1, x.0))
                    .collect::<Vec<_>>()
            })
            .iter_mut()
            .fold(vec![], |mut acc, x| {
                acc.append(x);
                acc
            })
            .to_vec(),
        );
        let req_assets_list = [
            req_assets_objects,
            req_assets_items,
            req_assets_units,
            req_assets_terrain_list,
            req_assets_windows,
            req_assets_stats_list,
            req_assets_armies_list,
        ];
        load_assets(&req_assets_list, fonts).await
    };
    let map = "Stinger-Paramount_War_HARD.dtm";
    let (mut gamemap, mut events) = load_map(map, &registry).await;
    let mut executor = Executor {
        gamemap: gamemap.clone(),
        events: events.clone(),
        battle: None,
        execution_queue: vec![],
        players: vec![Player {
            army: gamemap.armys.len() - 1,
            questbook: None,
            execution_queue: vec![],
            wait_until: None,
        }],
    };
    executor.tick(&registry);
    dbg!(&executor.execution_queue);
    dbg!(&executor.players[0].execution_queue);
    //let (mut gamemap, events) = parse_story(
    //     units!(),
    //     objects!(),
    //     &settings.locale,
    //     &settings.additional_locale,
    // );
    gamemap.calc_hitboxes(&registry.objects.inner);
    // let mut battle = BattleInfo::new(&mut gamemap.armys, 0, 1);
    let rt = tokio::runtime::Runtime::new().unwrap();
    let mut camera = Camera2D::from_display_rect(Rect {
        x: 0.,
        y: 1080.,
        w: 1920.,
        h: -1080.,
    });
    camera.rotation = 0.;
    /*let mut powers: Vec<_> = units!().iter().map(|x| (x.info.name.clone(), calclate_unit_power(x))).collect();
    powers.sort_by(|x, y| x.1.total_cmp(&y.1));
    dbg!(powers);*/
    let game = Game {
        executor,
        focus: BattleUnitPos { army: 0, pos: 0 },
        variant: GameVariant::Single(Scenario { events }),
    };
    State {
		registry,
        rt,
		delta: 0.,
        assets,
        textures: RenderTextures {
            map: Texture2D::empty(),
            decos: Texture2D::empty(),
        },
        ui: Ui {
            main: Menu::Battle,
            camera,
            stack: Vec::new(),
        },
        game,
    }
}
#[derive(Debug)]
struct Scenario {
    pub events: Vec<GameEvent>,
}
#[derive(Debug, PartialEq, Eq)]
enum ConnectionStatus {
    NotFull,
    Full(bool),
}
#[derive(Debug)]
struct Online {
    conn: Connection,
    status: ConnectionStatus,
    army: usize,
}
#[derive(Debug)]
enum GameVariant {
    Single(Scenario),
    Online(Online),
}
#[derive(Debug)]
struct Game {
    pub executor: Executor,
    pub variant: GameVariant,
    pub focus: BattleUnitPos,
}

fn load_img(bytes: &'static [u8]) -> Image {
    return Image::from_file_with_format(bytes, Some(ImageFormat::Png)).unwrap();
}

fn populate_array(img: Image, array: &mut [u8]) {
    let mut index: usize = 0;
    for pixel in img.get_image_data() {
        for value in pixel.iter() {
            array[index] = *value;
            index += 1;
        }
    }
}

pub fn set() -> Icon {
    let mut array_small: [u8; 1024] = [0; 1024];
    let mut array_medium: [u8; 4096] = [0; 4096];
    let mut array_big: [u8; 16384] = [0; 16384];

    populate_array(
        load_img(include_bytes!("../../dt/assets/icon-16.png")),
        &mut array_small,
    );
    populate_array(
        load_img(include_bytes!("../../dt/assets/icon-32.png")),
        &mut array_medium,
    );
    populate_array(
        load_img(include_bytes!("../../dt/assets/icon-64.png")),
        &mut array_big,
    );

    let custom_icon = Icon {
        small: array_small,
        medium: array_medium,
        big: array_big,
    };
    return custom_icon;
}

fn window_conf() -> Conf {
    Conf {
        high_dpi: true,
        window_title: "DT REMASTERED".into(),
        fullscreen: true,
        window_resizable: false,
        icon: Some(set()),
        ..Default::default()
    }
}
#[derive(Debug)]
struct MapRenderSettings {
    pub camera: Camera2D,
    pub deco_render: bool,
    pub buildings_render: bool,
    pub event_render: bool,
    pub tiles_render: bool,
    pub armies_render: bool,
    pub err_render: bool,
}
impl Default for MapRenderSettings {
    fn default() -> Self {
        Self {
            camera: Default::default(),
            deco_render: true,
            buildings_render: true,
            event_render: true,
            tiles_render: true,
            armies_render: true,
            err_render: false,
        }
    }
}
#[derive(Debug)]
enum Menu {
    Main,
	Atlas,
    Info,
    Map(MapRenderSettings),
    Message(Message),
    Battle,
    RoomCreation,
    BattleSetup,
}
#[derive(Debug)]
struct Ui {
    pub main: Menu,
    pub camera: Camera2D,
    pub stack: Vec<Menu>,
}
fn is_clicked(area: Rect) -> bool {
    is_mouse_button_released(MouseButton::Left) && is_hovered(area)
}
fn is_hovered(area: Rect) -> bool {
    let mouse = Camera2D::from_display_rect(Rect {
        x: 0.,
        y: 1080.,
        w: 1920.,
        h: -1080.,
    })
    .screen_to_world(mouse_position().into());
    area.contains(mouse)
}
fn draw_texture_size(texture: &Texture2D, pos: (f32, f32), size: (f32, f32)) {
    draw_texture_ex(
        texture,
        pos.0,
        pos.1,
        Color::from_rgba(255, 255, 255, 255),
        DrawTextureParams {
            dest_size: Some(size.into()),
            ..Default::default()
        },
    )
}
fn screen_center() -> Vec2 {
    let size = screen_size();
    Rect::new(0., 0., size.0, size.1).center()
}
enum DrawObject<'a> {
	Texture {
		source: &'a Texture2D,
		params: DrawTextureParams
	},
	Text {
		text: &'a str,
		params: TextParams<'a>
	}
}
impl<'a> DrawObject<'a> {
	pub fn new_text(text: &'a str, params: TextParams<'a>) -> Self {
		DrawObject::Text {
			text,
			params
		}
	}
	pub fn new_texture(source: &'a Texture2D, params: DrawTextureParams) -> Self {
		DrawObject::Texture {
			source,
			params
		}
	}
	pub fn get_size(&self) -> Vec2 {
		match self {
			DrawObject::Text { text, params } => {
				let measured = measure_text(text, params.font, params.font_size, params.font_scale);
				vec2(measured.width, measured.height)
			},
			DrawObject::Texture { source, params } => {
				params.dest_size.unwrap_or(source.size())
			}
		}
	}
}
fn draw_objects_line<'a>(objects: Vec<DrawObject<'a>>, centered: bool) {
	
}

/// БРЭЙКИНГ ПОИНТС?!
/// КАЛЬКЬЮЛЭЙТ ВИДТХ БАЙ ЛАЙНС!
fn split_text<'a>(text: &'a str, font: Option<&Font>, font_size: u16, max_width: f32) -> Vec<(Vec2, String)> {
	let lines = text.split("\n");
    let space_size = measure_text(" ", font, font_size, 1.).width;
    // ((line_width, height), line)
    lines
        .map(|line| {
            if line.is_empty() {
                (vec![(vec2(0., font_size as f32), "".to_string())], 0.)
            } else {
                let words = line
                    .split_whitespace()
                    .map(|word| (word, measure_text(word, font, font_size, 1.)));
                // ((line_width, height), line)
                let lines: Vec<(Vec2, String)> = vec![];
                words.fold((lines, 0.), |(mut lines, acc_width), (word, word_size)| {
                    if acc_width + word_size.width > max_width {
                        lines.push((
                            vec2(word_size.width + space_size, word_size.height),
                            word.to_string() + " ",
                        ));
                        (lines, word_size.width + space_size)
                    } else {
                        let last = lines.last_mut();
                        if let Some(last) = last {
                            last.0 = vec2(
                                last.0.x + word_size.width + space_size,
                                last.0.y.max(word_size.height),
                            );
                            last.1.push_str(word);
                            last.1.push_str(" ");
                        } else {
                            lines.push((
                                vec2(word_size.width + space_size, word_size.height),
                                word.to_string() + " ",
                            ))
                        }
                        (lines, acc_width + word_size.width + space_size)
                    }
                })
            }
        })
        .fold(vec![], |mut lines, item| {
            lines.extend(item.0);
            lines
        })
}
/// Returns  drawn text box
fn draw_multiline(
    text: String,
    pos: (f32, f32),
    max_width: f32,
    centered: bool,
    font: Option<&Font>,
    font_size: u16,
    color: Color,
) -> Vec2 {
	let lines = split_text(&text, font, font_size, max_width);
    let mut cur_y_pos = pos.1;
    let mut max_line_width: f32 = 0.;
    let font_line_distance = font_size as f32;

    for (num, (size, line)) in lines.iter().enumerate() {
        max_line_width = max_line_width.max(size.x);
        if num > 0 {
            cur_y_pos += font_line_distance;
        }
        let cur_x_pos = if centered {
            max_width / 2. - size.x / 2. + pos.0
        } else {
            pos.0
        };
        draw_text_ex(
            &line,
            cur_x_pos,
            cur_y_pos,
            TextParams {
                font,
                font_size,
                font_scale: 1.,
                color,
                ..Default::default()
            },
        );
        if num == lines.len() - 1 {
            cur_y_pos += font_line_distance;
        }
    }
    vec2(max_line_width, cur_y_pos - pos.1)
}
fn get_unit_texture<'a>(assets: &'a Assets, unit: &Unit, units: &Units) -> &'a Texture2D {
    assets.get(&format!("unit_{}.png", unit.get_info(units).icon_index - 1))
}
fn get_unit_info_texture<'a>(assets: &'a Assets, unit: &UnitInfo) -> &'a Texture2D {
    assets.get(&format!("unit_{}.png", unit.icon_index - 1))
}
fn get_troop_texture<'a>(
    assets: &'a Assets,
    armies: &Vec<Army>,
    unit: usize,
    army: usize,
	units: &Units
) -> Option<&'a Texture2D> {
    armies.get(army).and_then(|x| {
        x.troops.get(unit).and_then(|tr| {
            let troop = &tr.get();
            Some(assets.get(&format!("unit_{}.png", troop.unit.get_info(units).icon_index - 1)))
        })
    })
}

fn unit_card_battle(
    army: usize,
    unit_pos: usize,
    battle: &mut BattleInfo,
    armies: &mut Vec<Army>,
    pos: (f32, f32),
    assets: &Assets,
    is_in_focus: bool,
    is_battle_active: bool,
    is_my_move: bool,
	registry: &GameInfo
) -> bool {
    fn draw_stats(troop: &Troop, mut pos: (f32, f32), assets: &Assets, draw_size: (f32, f32)) {
        let bg_color = if troop.is_main {
            Color::from_rgba(168, 48, 48, 255)
        } else {
            Color::from_rgba(132, 46, 46, 255)
        };
        draw_rectangle(pos.0, pos.1, draw_size.0, draw_size.1, bg_color);
        let stats = troop.unit.modified;
        let font_size = 26.;
        draw_texture_size(assets.get("hearts.png"), pos, (font_size, font_size));
        draw_text(
            &format!("{}/{}", troop.unit.hp, stats.max_hp),
            pos.0 + font_size,
            pos.1 + font_size,
            font_size,
            WHITE,
        );
        let start_pos = pos;
        if stats.damage.hand > 0 {
            pos.1 += font_size;
            draw_texture_size(
                assets.get("crossed-swords.png"),
                pos,
                (font_size, font_size),
            );
            draw_text(
                &format!("{}", stats.damage.hand),
                pos.0 + font_size,
                pos.1 + font_size,
                font_size,
                WHITE,
            );
        }
        if stats.damage.ranged > 0 {
            pos.1 += font_size;
            draw_texture_size(assets.get("high-shot.png"), pos, (font_size, font_size));
            draw_text(
                &format!("{}", stats.damage.ranged),
                pos.0 + font_size,
                pos.1 + font_size,
                font_size,
                WHITE,
            );
        }
        if stats.damage.magic > 0 {
            pos.1 += font_size;
            draw_texture_size(assets.get("wizard-staff.png"), pos, (font_size, font_size));
            draw_text(
                &format!("{}", stats.damage.magic),
                pos.0 + font_size,
                pos.1 + font_size,
                font_size,
                WHITE,
            );
        }
        if stats.defence.hand_units > 0 || stats.defence.ranged_units > 0 {
            if pos.1 - start_pos.1 >= font_size * 2. {
                pos.0 += measure_text("32", Some(assets.get_font(BENGUIAT)), font_size as u16, 1.)
                    .width
                    + 48.;
            } else {
                pos.1 += font_size;
            }

            draw_texture_size(assets.get("breastplate.png"), pos, (font_size, font_size));
            draw_text(
                &format!(
                    "{}/{}",
                    stats.defence.hand_units, stats.defence.ranged_units
                ),
                pos.0 + font_size,
                pos.1 + font_size,
                font_size,
                WHITE,
            );
        }
        let text = &format!("{}", stats.speed);
        let text_size = measure_text(&text, None, font_size as u16, 1.).width;
        draw_texture_size(
            assets.get("sprint.png"),
            (
                start_pos.0 + draw_size.0 - font_size - text_size,
                start_pos.1,
            ),
            (font_size, font_size),
        );
        draw_text(
            &text,
            start_pos.0 + draw_size.0 - text_size,
            start_pos.1 + font_size,
            font_size,
            WHITE,
        );
        let text = &format!("Moves {}/{}", troop.unit.moves, stats.max_moves);
        let text_size = measure_text(&text, None, font_size as u16, 1.).width;
        draw_text(
            &text,
            start_pos.0 + draw_size.0 - text_size,
            start_pos.1 + font_size * 2.,
            font_size,
            WHITE,
        );
    }
    let Some(unit) = armies[army].get_troop(unit_pos) else {
        let texture = assets.get(
            &match field_type(unit_pos, 12) {
                Field::Back => "backyard.png",
                Field::Front => "front.png",
                Field::Reserve => "tent.png",
            }
            .to_owned(),
        );
        let undercell = assets.get(&"undercell.png".to_owned());
        draw_texture_size(texture, pos, (CARD_SIZE, CARD_SIZE));
        {
            let pos = (pos.0, pos.1 + CARD_SIZE);
            let size = (CARD_SIZE, CARD_SIZE / 2.);
            draw_texture_size(undercell, pos, size);
        }
        if let Some(active_unit) = battle.active_unit {
            let old_pos = armies[active_unit.army].troops[active_unit.index].get().pos;
            if army == active_unit.army
                && (old_pos.0.abs_diff(unit_pos % (12 / 2)) < 2
                    || field_type(unit_pos, 12) == Field::Reserve
                    || field_type(old_pos.whole(6), 12) == Field::Reserve)
            {
                let outline_color =
                    Color::from_rgba(22, 22, 255, (get_time().sin() * 128. + 64.) as u8);
                draw_rectangle_lines(pos.0, pos.1, CARD_SIZE, CARD_SIZE, 15., outline_color);
            }
        }
        return is_clicked(Rect {
            x: pos.0,
            y: pos.1,
            w: CARD_SIZE,
            h: CARD_SIZE,
        });
    };
    fn draw_unit_effects(pos: (f32, f32), unit: &Unit, assets: &Assets) {
        // for (i, (effect, lifetime)) in unit
        //     .effects
        //     .iter()
        //     .filter_map(|effect| match effect {
        //         Effect::BlockEffect(BlockEffect {
        //             info: EffectInfo { lifetime },
        //         }) => Some(("checked-shield.png", *lifetime)),
        //         Effect::MoreMoves(MoreMoves {
        //             info: EffectInfo { lifetime },
        //         }) => Some(("mounted-knight.png", *lifetime)),
        //         Effect::Fire(Fire {
        //             info: EffectInfo { lifetime },
        //             ..
        //         }) => Some(("small-fire.png", *lifetime)),
        //         Effect::RessurectedEffect(_) => Some(("raise-zombie.png", 0)),
        //         Effect::Poison(Poison {
        //             info: EffectInfo { lifetime },
        //         }) => Some(("poison-bottle.png", *lifetime)),
        //         Effect::SpearEffect(SpearEffect {
        //             info: EffectInfo { lifetime },
        //         }) => Some(("spartan.png", *lifetime)),
        //         _ => None,
        //     })
        //     .enumerate()
        // {
        //     let texture = assets.get(effect);
        //     draw_texture_size(texture, (pos.0 + 40. * i as f32, pos.1), (32., 32.));
        //     if lifetime > 0 {
        //         draw_text(
        //             &*lifetime.to_string(),
        //             pos.0 + 40. * i as f32 + 5.,
        //             pos.1 + 32.,
        //             32.,
        //             WHITE,
        //         );
        //     }
        // }
    }
    let troop = unit.get();
    let unit_texture = get_unit_texture(&assets, &troop.unit, &registry.units);
    let is_interactable = battle.can_interact.as_ref().is_some_and(|x| {
        x.contains(&BattleUnitPos {
            army,
            pos: unit_pos,
        })
    });
	let info = &troop.unit.get_info(&registry.units);
    let size = info.size;
    let draw_size = (size.0 as f32 * CARD_SIZE, size.1 as f32 * CARD_SIZE);
    let draw_rect = Rect::new(pos.0, pos.1, draw_size.0, draw_size.1);
    let stats = troop.unit.modified;
    let hp = 1. - stats.hp as f32 / stats.max_hp as f32;
    draw_texture_size(unit_texture, pos, draw_size);
    draw_stats(
        &troop,
        (pos.0, pos.1 + draw_size.1),
        assets,
        (draw_size.0, CARD_SIZE / 2.),
    );
    draw_rectangle(
        pos.0,
        pos.1 + draw_size.1,
        draw_size.0,
        -draw_size.1 * hp,
        Color::new(hp, 0., 0., hp.min(0.9)),
    );
    let outline_color = if let Some(active_unit) = battle.active_unit {
        if is_my_move && is_interactable {
            Some(
                if armies[army].hitmap[unit_pos]
                    .is_some_and(|index| active_unit == BattleUnit { army, index })
                {
                    Color::from_rgba(22, 255, 22, 64 + (get_time().sin() * 128.) as u8)
                } else if active_unit.army != army {
                    Color::from_rgba(255, 22, 22, 64 + (get_time().sin() * 128.) as u8)
                } else {
                    Color::from_rgba(22, 22, 255, 64 + (get_time().sin() * 128.) as u8)
                },
            )
        } else {
            None
        }
    } else {
        None
    };
    if let Some(outline_color) = outline_color {
        draw_rectangle_lines(pos.0, pos.1, draw_size.0, draw_size.1, 15., outline_color);
    }
    if !is_battle_active && is_in_focus {
        draw_rectangle_lines(pos.0, pos.1, draw_size.0, draw_size.1, 15., BLUE);
    }
    draw_unit_effects((pos.0, pos.1 + draw_size.1 - 32.), &troop.unit, assets);
    let items = &troop.unit.inventory.items;
    for (i, item) in items.iter().enumerate() {
        if let Some(item) = item {
            let texture = &item.get_info(&registry.items).icon;
            let texture = assets.get(texture);
            let item_pos = draw_size.0 / 4. * i as f32;
            draw_texture_size(
                texture,
                (pos.0 + item_pos, pos.1),
                (CARD_SIZE / 4., CARD_SIZE / 4.),
            );
        };
    }
    if is_clicked(draw_rect) {
        draw_rectangle_lines(pos.0, pos.1, draw_size.0, draw_size.1, 15., BLACK);
        true
    } else {
        false
    }
}
fn draw_battle(assets: &Assets, game: &mut Game, is_battle_active: bool, registry: &GameInfo) -> Option<usize> {
    let (armies, battle) = (&mut game.executor.gamemap.armys, &mut game.executor.battle);
    let Some(battle) = battle.as_mut() else {
        return None;
    };
    let is_my_move = if let GameVariant::Online(Online { army, .. }) = &game.variant {
        battle
            .active_unit
            .is_some_and(|active| active.army == *army)
    } else {
        true
    };
	let focus = &mut game.focus;
	let half = registry.game_settings.max_troops / 2;
	let (up, down, right, left) = (is_key_released(KeyCode::W) || is_key_released(KeyCode::Up), is_key_released(KeyCode::Down) || is_key_released(KeyCode::S), is_key_released(KeyCode::D) || is_key_released(KeyCode::Right), is_key_released(KeyCode::A) || is_key_released(KeyCode::Left));
	if up || down {
		match (focus.pos, focus.army) {
			(pos, army) if pos >= half && army == battle.army2 => {
				if up {
					focus.army = battle.army1;
				}
				if down {
					focus.pos -= half; 
				}
			}
			(pos, army) if pos >= half && army == battle.army1 => {
				if up {
					focus.pos -= half;
				}
				if down {
					focus.army = battle.army2;
				}
			}
			(pos, army) if pos <= half && army == battle.army2 => {
				if up {
					focus.pos += half;
				}
				if down {
					focus.army = battle.army1;
				}
			}
			(pos, army) if pos <= half && army == battle.army1 => {
				if up {
					focus.army = battle.army2;
				}
				if down {
					focus.pos += half;
				}
			}
			_ => {}
		}
	}
	if right {
		focus.pos = (focus.pos + 1) % (half * 2);
	}
	if left {
		focus.pos = (focus.pos.saturating_sub(1)) % (half * 2);
	}
	
    let half_troops = 12 / 2;
    let winner = battle.winner;
    for army in 0..=1 {
        for row in 0..=1 {
            for i in 0..(12 / 2) {
                let unit_pos = i + (army as i64 - row as i64).abs() as usize * half_troops;
                let army = if army == 0 {
                    battle.army1
                } else {
                    battle.army2
                } as usize;
                // Check for non-single cell units, so they will be skipped
                let hitmap = &armies[army].hitmap;
                let skip = {
                    let vertical = unit_pos >= 12 / 2
                        && hitmap[unit_pos] == hitmap[unit_pos - 12 / 2];
                    let horizontal = field_type(unit_pos, 12) != Field::Reserve
                        && hitmap[unit_pos] == hitmap[unit_pos - 1];
                    (vertical || horizontal) && hitmap[unit_pos].is_some()
                };
                if skip {
                    continue;
                }
                let mut pos = (0., 0.);
                pos.0 = i as f32 * CARD_SIZE;
                pos.1 = army as f32 * CARD_SIZE * 3. + row as f32 * CARD_SIZE * 1.5;
                // f(x) = { 0, 6, 6, 0 } where x = { {0, 0}, {0, 1}, {1, 0}, {1, 1} }
                // abs(0 * 6 - 0 * 6) = 0
                // abs(0 * 6 - 1 * 6) = 6
                // abs(1 * 6 - 0 * 6) = 6
                // abs(1 * 6 - 1 * 6) = 0
                draw_rectangle(
                    pos.0,
                    pos.1,
                    CARD_SIZE,
                    CARD_SIZE,
                    Color::from_rgba(0, 0, 0, 1),
                );
                let is_focused = game.focus.army == army && game.focus.pos == unit_pos;
                let draw_size = (CARD_SIZE, CARD_SIZE);
                let draw_rect = Rect::new(pos.0, pos.1, draw_size.0, draw_size.1);
                if is_clicked(draw_rect) {
                    game.focus = BattleUnitPos {
                        pos: unit_pos,
                        army,
                    };
                }
				if unit_card_battle(
                    army,
                    unit_pos,
                    battle,
                    armies,
                    pos,
                    assets,
                    is_focused,
                    is_battle_active,
                    is_my_move,
					registry
                ) && winner.is_none()
                {
                    game.focus = BattleUnitPos {
                        pos: unit_pos,
                        army,
                    };
                    if !is_battle_active {
                        continue;
                    }
                    if let GameVariant::Online(Online { conn, .. }) = &mut game.variant {
                        conn.send_action(dt_server::Incoming::Action(BattleUnitPos {
                            pos: unit_pos,
                            army,
                        }));
                    } else {
                        handle_action((unit_pos, army), battle, armies, registry);
                    }
                }
                if game.focus
                    == (BattleUnitPos {
                        army,
                        pos: unit_pos,
                    })
                    && !is_battle_active
                {
                    draw_rectangle_lines(pos.0, pos.1, draw_size.0, draw_size.1, 15., BLUE);
                }
            }
        }
    }
    for (i, unit) in battle.move_order.iter().skip(1).take(10).enumerate() {
        let color = if unit.army == 0 { BLUE } else { RED };
        let pos = (i as f32 * 64., CARD_SIZE * 6.);
        let Some(texture) = ({
            let army = unit.army;
            let army = if army == 0 {
                battle.army1
            } else {
                battle.army2
            } as usize;
            get_troop_texture(assets, &armies, unit.index, army, &registry.units)
        }) else {
            continue;
        };
        draw_texture_size(texture, pos, (64., 64.));
        draw_rectangle_lines(pos.0, pos.1, 64., 64., 10., color.with_alpha(0.2));
    }
    winner
}
fn count_tileset_index(map: &GameMap, pos: (usize, usize)) -> usize {
    let is_dirt = |pos: (usize, usize)| {
        if map.tilemap[pos.min((map.tilemap.size - 1, map.tilemap.size - 1))] == 6 {
            1
        } else {
            0
        }
    };

    let bitmask = is_dirt((pos.0 + 1, pos.1 + 1))
        + is_dirt((pos.0.saturating_add_signed(-1), pos.1 + 1)) * 2
        + is_dirt((
            pos.0.saturating_add_signed(-1),
            pos.1.saturating_add_signed(-1),
        )) * 4
        + is_dirt((pos.0 + 1, pos.1.saturating_add_signed(-1))) * 8;
    bitmask
}
const SIZE: (f32, f32) = (32., 22.); //(256., 176.);
#[derive(Debug)]
struct RenderTextures {
    pub map: Texture2D,
    pub decos: Texture2D,
}
fn draw_decos(assets: &Assets, decomap: &Vec<MapDeco>, objects: &Objects) {
    for deco in decomap {
        let (i, j) = (deco.x, deco.y);
        if let Some(obj) = objects.inner.iter().find(|el| match el.obj_type {
            ObjectType::MapDeco { id } => id == deco.index,
            _ => false,
        }) {
            let texture = assets.get(&obj.path.clone());
            let size = texture.size();
            draw_texture_ex(
                texture,
                i as f32 * SIZE.0 - size.x + SIZE.0,
                j as f32 * SIZE.1 - size.y + SIZE.1,
                WHITE,
                DrawTextureParams {
                    dest_size: Some(Vec2::new(
                        size.x as f32 / 32. * SIZE.0,
                        size.y as f32 / 22. * SIZE.1,
                    )),

                    ..Default::default()
                },
            );
        }
    }
}
fn draw_tiles(assets: &Assets, tilemap: &TileMap<usize>) {
    let tile_textures = TILES
        .iter()
        .map(|tile| assets.get(&tile.sprite().to_string()))
        .collect::<Vec<_>>();
    for i in 0..tilemap.size {
        for j in 0..tilemap.size {
            let tile_index = tilemap[(j, i)];

            draw_texture_ex(
                tile_textures[tile_index],
                i as f32 * SIZE.0,
                j as f32 * SIZE.1,
                WHITE,
                DrawTextureParams {
                    dest_size: Some(vec2(SIZE.0, SIZE.1)),
                    ..Default::default()
                },
            );
        }
    }
}
fn draw_buildings(assets: &Assets, buildings: &Vec<MapBuildingdata>, objects: &Objects) {
    for building in buildings {
        let (i, j) = building.pos;
        if let Some(obj) = objects.inner.iter().find(|el| el.index == building.id) {
            let texture = assets.get(&obj.path.clone());
            let size = texture.size();
            draw_texture_ex(
                texture,
                i as f32 * SIZE.0 - size.x + SIZE.0,
                j as f32 * SIZE.1 - size.y + SIZE.1,
                WHITE,
                DrawTextureParams {
                    dest_size: Some(Vec2::new(
                        size.x as f32 / 32. * SIZE.0,
                        size.y as f32 / 22. * SIZE.1,
                    )),

                    ..Default::default()
                },
            );
        }
    }
}
fn prepare_textures(target: &[RenderTarget; 2], assets: &Assets, game: &Game, registry: &GameInfo) -> RenderTextures {
    let size = game.executor.gamemap.tilemap.size as u32;
    let tile_size = (SIZE.0 as u32, SIZE.1 as u32);
    debug!("CAMERA_CHANGE");
    let mut camera = Camera2D::from_display_rect(Rect::new(
        0.,
        SIZE.1 * size as f32,
        SIZE.0 * size as f32,
        -SIZE.1 * size as f32,
    ));
    camera.render_target = Some(target[0].clone());
    set_camera(&camera);

    let find_decos = |name: &'static str| {
        game.executor
            .gamemap
            .decomap
            .iter()
            .filter(|deco| {
                registry.objects.inner
                    .iter()
                    .find(|el| match el.obj_type {
                        ObjectType::MapDeco { id } => id == deco.index,
                        _ => false,
                    })
                    .is_some_and(|obj| obj.name.contains(name))
            })
            .cloned()
            .collect::<Vec<_>>()
    };
    debug!("TILE DRAW");
    draw_tiles(assets, &game.executor.gamemap.tilemap);

    let mut camera = Camera2D::from_display_rect(Rect::new(
        0.,
        SIZE.1 * size as f32,
        SIZE.0 * size as f32,
        -SIZE.1 * size as f32,
    ));
    camera.render_target = Some(target[1].clone());
    set_camera(&camera);
    debug!("DECOS SEARCH");
    let hills = find_decos("Hills");
    let mountains = find_decos("Mountain");
    let trees = find_decos("Tree");
    let rocks = find_decos("Rocks");
    debug!("HILLS DRAW");
    draw_decos(assets, &hills, &registry.objects);
    debug!("BUILDINGS DRAW");
    draw_buildings(assets, &game.executor.gamemap.buildings, &registry.objects);
    debug!("MOUNTAINS DRAW");
    draw_decos(assets, &mountains, &registry.objects);
    debug!("TREES DRAW");
    draw_decos(assets, &trees, &registry.objects);
    debug!("ROCKS DRAW");
    draw_decos(assets, &rocks, &registry.objects);

    // Draw the rest of decos
    draw_decos(
        assets,
        &game
            .executor
            .gamemap
            .decomap
            .iter()
            .filter(|deco| {
                [&hills, &mountains, &trees, &rocks]
                    .iter()
                    .all(|x| !x.contains(&deco))
            })
            .cloned()
            .collect::<Vec<_>>(),
		&registry.objects
    );
    RenderTextures {
        map: target[0].texture.clone(),
        decos: target[1].texture.clone(),
    }
}
fn draw_map(
    assets: &Assets,
    settings: &mut MapRenderSettings,
    textures: &RenderTextures,
    game: &mut Game,
	delta: &mut f32,
	registry: &GameInfo
) -> Option<Menu> {
    let camera = &mut settings.camera;
    set_camera(camera);

    let size = game.executor.gamemap.tilemap.size as u32;
    let find_decos = |name: &'static str| {
        game.executor
            .gamemap
            .decomap
            .iter()
            .filter(|deco| {
                registry.objects.inner
                    .iter()
                    .find(|el| match el.obj_type {
                        ObjectType::MapDeco { id } => id == deco.index,
                        _ => false,
                    })
                    .is_some_and(|obj| obj.name.contains(name))
            })
            .cloned()
            .collect::<Vec<_>>()
    };
    let prerender = true;
    if !prerender {
        draw_tiles(assets, &game.executor.gamemap.tilemap);
        let hills = find_decos("Hills");
        let mountains = find_decos("Mountain");
        let trees = find_decos("Tree");
        let rocks = find_decos("Rocks");
        draw_decos(assets, &hills, &registry.objects);
        draw_buildings(assets, &game.executor.gamemap.buildings, &registry.objects);
        draw_decos(assets, &mountains, &registry.objects);
        draw_decos(assets, &trees, &registry.objects);
        draw_decos(assets, &rocks, &registry.objects);

        // Draw the rest of decos
        draw_decos(
            assets,
            &game
                .executor
                .gamemap
                .decomap
                .iter()
                .filter(|deco| {
                    [&hills, &mountains, &trees, &rocks]
                        .iter()
                        .all(|x| !x.contains(&deco))
                })
                .cloned()
                .collect::<Vec<_>>(),
			&registry.objects
        );
    } else {
        draw_texture(&textures.map, 0., 0., WHITE);
        draw_texture(&textures.decos, 0., 0., WHITE);
    }
    for i in 0..size {
        for j in 0..size {
			let pos = (j as usize, i as usize);
			let events = &game.executor.gamemap.eventmap[pos];
			if events.len() > 0 {
				draw_rectangle(i as f32 * SIZE.0, j as f32 * SIZE.1, SIZE.0, SIZE.1, BLUE)
			}
			let hit = game.executor.gamemap.hitmap[pos];
            let player_army = game.executor.players[0].army;
            let color = if game.executor.gamemap.armys[player_army]
                .path
                .contains(&(i as usize, j as usize))
            {
                VIOLET
            } else if game.executor.gamemap.armys[player_army].pos == (i as usize, j as usize) {
                BLACK
            } else if hit.passable() && !hit.need_transport {
                GREEN
            } else if hit.need_transport {
                SKYBLUE
            } else {
                RED
            }
            .with_alpha(0.8);
            // draw_rectangle(i as f32 * SIZE.0, j as f32 * SIZE.1, SIZE.0, SIZE.1, color);
        }
    }
    let (gamemap, battle) = (&mut game.executor.gamemap, &mut game.executor.battle);
    if settings.armies_render {
        for hit in &gamemap.hitmap.inner {
            if let Some(army) = hit.army {
                let army = &gamemap.armys[army];
                let (i, j) = army.pos;
                // FIX THIS
                let change = (get_time() * 1000. / 50.) as usize % 8;
				let new = army.path.get(0).unwrap_or(&army.pos);
				let step = (new.0 as isize - i as isize, new.1 as isize - j as isize);
				let frame = match step {
					(-1, -1) => change,
					(0, -1) => 8 + change,
					(1, -1) => 16 + change,
					(1, 0) => 24 + change,
					(1, 1) => 32 + change,
					(0, 0) => 40,
					(0, 1) => 40 + change,
					(-1, 1) => 48 + change,
					(-1, 0) => 56 + change,
					_ => 40
				};
                let pic = &format!(
                    "{}/Unit_{frame}.png",
                    match (
                        &army.control,
                        army.troops
                            .get(0)
                            .and_then(|tr| Some(tr.get().unit.get_info(&registry.units).unit_type))
                            .unwrap_or(UnitType::People)
                    ) {
                        (Control::Player(_), _) => "ГГследопыт",
                        (_, unittype) => match unittype {
                            UnitType::Undead => "мертвяк",
                            UnitType::Rogue => "разбойник",
                            UnitType::Hero | UnitType::People => "феодал",
                            _ => "некромант",
                        },
                    }
                );
                let texture = assets.get(pic);
                let size = texture.size();
                draw_texture_ex(
                    texture,
                    i as f32 * SIZE.0,
                    j as f32 * SIZE.1 - SIZE.1,
                    WHITE,
                    DrawTextureParams {
                        dest_size: Some(Vec2::new(
                            SIZE.0,      //size.x as f32 / 32. * SIZE.0,
                            SIZE.1 * 2., //size.y as f32 / 22. * SIZE.1,
                        )),
                        ..Default::default()
                    },
                );
            }
        }
    }
    if is_key_pressed(KeyCode::T) {
        settings.tiles_render = !settings.tiles_render;
    }
    if is_key_pressed(KeyCode::F) {
        settings.armies_render = !settings.armies_render;
    }
    if is_key_pressed(KeyCode::Y) {
        settings.deco_render = !settings.deco_render;
    }
    if is_key_pressed(KeyCode::E) {
        settings.err_render = !settings.err_render;
    }

    if is_key_down(KeyCode::Enter) {
        *battle = Some(BattleInfo::new(&mut gamemap.armys, 0, 1));
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
        //camera.offset -= vec2(0.5, 0.5);
        //camera.offset += dbg!(camera.screen_to_world(mouse_position().into())) / Vec2::from(screen_size()) / 100.;
    }
    let wheel_diff = mouse_wheel().1;
    if wheel_diff != 0. {
        camera.zoom *= wheel_diff.abs()
            * if wheel_diff > 0. {
                vec2(1.1, 1.1)
            } else {
                Vec2::ONE / vec2(1.1, 1.1)
            }
    }
    if is_key_down(KeyCode::Minus) {
        camera.zoom *= Vec2::new(1.1, 1.1);
        //camera.offset -= dbg!(camera.screen_to_world(mouse_position().into())) / Vec2::from(screen_size()) / 100.;
    }
    //if is_key_released(KeyCode::I) {
    let map_size = gamemap.tilemap.size;
    let tile_size = vec2(SIZE.0, SIZE.1);
    let pos = camera.screen_to_world(mouse_position().into());
    let tile = (pos / tile_size).floor();
    draw_text(&format!("{tile:?}"), 0., -60., 50., BLACK);
    if is_mouse_button_released(MouseButton::Left) {
        game.executor.message_handler(
            dt_lib::network::server::ClientMessage::GoTo((
                tile.x as usize % map_size,
                tile.y as usize % map_size,
            )),
            0,
			registry
        );
    }
    if !game.executor.players[0].execution_queue.is_empty() {
        if let Execute::Message(msg) = game.executor.players[0].execution_queue.remove(0) {
            dbg!(&msg);
            return Some(Menu::Message(msg));
        }
    }
    if is_mouse_button_down(MouseButton::Right) {
        //camera.offset += camera.screen_to_world(mouse_position().into()) / vec2(-SIZE.0 * 50., SIZE.1 * 30.);
        camera.offset += mouse_delta_position() * vec2(-1., 1.);
        //dbg!(camera.screen_to_world(mouse_position().into()) / vec2(-SIZE.0 * 50., SIZE.1 * 30.));
    }
    draw_text(&format!("{}", get_fps()), 0., 0., 50., BLACK);
	*delta += get_frame_time();
	if *delta > 0.2 {
		game.executor.tick(registry);
		*delta = 0.;
	}
    //	sleep(Duration::from_millis(50));
    return None;
}
fn process_event(game: &mut Game, event: IncomingEvent) {
    let GameVariant::Online(conn) = &mut game.variant else {
        return;
    };
    match event {
        IncomingEvent::Id(army) => {
            debug!("My id is {army}");
            conn.army = army;
        }
        IncomingEvent::Game(g) => {
            game.executor.gamemap.armys = g.0;
            game.executor.battle = Some(g.1);
        }
        IncomingEvent::Info(text) => match &*text {
            "Wait for another player" => {
                conn.status = ConnectionStatus::NotFull;
            }
            "Room full" => {
                conn.status = ConnectionStatus::Full(false);
            }
            _ => {
                dbg!(text);
            }
        },
        IncomingEvent::Acceptance(a) => {
            if a == [true, true] {
                conn.status = ConnectionStatus::Full(true);
            }
        }
    }
}

#[macroquad::main(window_conf)]
async fn main() {
    let mut state = game_init().await;
    macroquad::texture::build_textures_atlas();
    let size = state.game.executor.gamemap.tilemap.size as u32;
    let tile_size = (SIZE.0 as u32, SIZE.1 as u32);
    let target = [
        render_target(tile_size.0 * size as u32, tile_size.1 * size as u32),
        render_target(tile_size.0 * size as u32, tile_size.1 * size as u32),
    ];
    debug!("START");
    state.textures = prepare_textures(&target, &state.assets, &state.game, &state.registry);
    set_default_camera();
    debug!("DEFAULT CAMERA SET");
    clear_background(WHITE);
    draw_text(
        "Loading game assets...",
        0.,
        screen_height() / 2.,
        20.,
        BLACK,
    );
    next_frame().await;
    let bg = root_ui()
        .style_builder()
        .background(state.assets.get(&"Menu.png".to_owned()).get_texture_data())
        .build();
    let bg1 = root_ui()
        .style_builder()
        .background(state.assets.get(&"Paper.png".to_owned()).get_texture_data())
        .build();
    let button = root_ui()
        .style_builder()
        .font_size(32)
        .color(DARKGREEN)
        .text_color(WHITE)
        .background(
            state
                .assets
                .get(&"button.png".to_owned())
                .get_texture_data(),
        )
        .with_font(state.assets.get_font(BENGUIAT))
        .unwrap()
        .build();
    let group = root_ui()
        .style_builder()
        .font_size(32)
        .text_color(WHITE)
        .background(
            state
                .assets
                .get(&"button.png".to_owned())
                .get_texture_data(),
        )
        .with_font(state.assets.get_font(BENGUIAT))
        .unwrap()
        .build();
    let label = root_ui()
        .style_builder()
        .text_color(BLACK)
        .text_color_hovered(DARKBLUE)
        .font_size(32)
        .with_font(state.assets.get_font(BENGUIAT))
        .unwrap()
        .build();
    let scroll = root_ui().style_builder().color(ORANGE).build();
    let scroll_handle = root_ui().style_builder().color(DARKGREEN).build();
    let main_skin = Skin {
        label_style: label.clone(),
        button_style: button.clone(),
        window_style: bg,
        editbox_style: button.clone(),
        group_style: group.clone(),
        scroll_width: 50.,
        scrollbar_handle_style: scroll_handle.clone(),
        scrollbar_style: scroll.clone(),
        ..root_ui().default_skin()
    };
    let dop_skin = Skin {
        label_style: label,
        button_style: button.clone(),
        editbox_style: button.clone(),
        group_style: group,
        window_style: bg1,
        scroll_width: 50.,
        scrollbar_handle_style: scroll_handle.clone(),
        scrollbar_style: scroll,
        ..root_ui().default_skin()
    };
    let mut input = String::new();
    loop {
        root_ui().push_skin(&main_skin);
        state.ui.camera = Camera2D::from_display_rect(Rect {
            x: 0.,
            y: 1080.,
            w: 1920.,
            h: -1080.,
        });
        set_default_filter_mode(FilterMode::Nearest);
        set_camera(&state.ui.camera);
        clear_background(WHITE);
        match &mut state.ui.main {
            Menu::Main => {
                Window::new(hash!(), vec2(0., 0.), vec2(screen_width(), screen_height()))
                    .titlebar(false)
                    .ui(&mut *root_ui(), |ui| {
                        let locale = &mut state.registry.locale;
						
                        ui.label(Some((50., 50.).into()), &locale.get("menu_game_name"));
                        if ui.button(Some((250., 300.).into()), locale.get("menu_start_title")) {
                            let mut camera = Camera2D::from_display_rect(Rect::new(
                                0.,
                                SIZE.1 * 50.,
                                SIZE.0 * 50.,
                                -SIZE.1 * 50.,
                            ));
                            camera.rotation = 0.;
                            dbg!(&camera);
                            state.ui.main = Menu::Map(MapRenderSettings {
                                camera,
                                ..Default::default()
                            });
                        }
                        if ui.button(Some((50., 150.).into()), "Atlas") {
                        	state.ui.main = Menu::Atlas;
                        }
                        // if ui.button(Some((50., 250.).into()), locale.get("menu_battle_title")) {
                        // 	state.ui.main = Menu::Battle;
                        // }
                        if ui.button(Some((50., 100.).into()), locale.get("menu_pvp_title")) {
                            state.ui.main = Menu::RoomCreation;
                        }
                        if ui.button(Some((50., 150.).into()), locale.get("menu_pve_title")) {
                            state.game.executor.battle = Some(BattleInfo::new(
                                &mut state.game.executor.gamemap.armys,
                                0,
                                1,
                            ));
                            state.ui.main = Menu::BattleSetup;
                        }
                        if ui.button(vec2(50., 200.), locale.get("menu_language")) {
                            locale.switch_lang();
                        }
                        if ui.button(Some((50., 250.).into()), "Info") {
                            state.ui.main = Menu::Info;
                        }
                        draw_texture_ex(
                            state.assets.get("ГГследопыт/Unit_40.png"),
                            500.,
                            500.,
                            WHITE,
                            DrawTextureParams {
                                dest_size: Some(Vec2::new(
                                    10. * SIZE.0,      //size.x as f32 / 32. * SIZE.0,
                                    10. * SIZE.1 * 2., //size.y as f32 / 22. * SIZE.1,
                                )),
                                ..Default::default()
                            },
                        );
                    });
            }
            Menu::Map(settings) => {
                if let Some(menu) =
                    draw_map(&state.assets, settings, &state.textures, &mut state.game, &mut state.delta, &state.registry)
                {
                    state.ui.main = menu;
                };
            }
            Menu::Message(msg) => {
                let units = &state.registry.units;
                let font = state.assets.get_font(BENGUIAT);
                root_ui().pop_skin();
                root_ui().push_skin(&dop_skin);
                draw_texture_size(state.assets.get("Paper.png"), (0., 0.), screen_size());
                let max_line_length = 640.;
                let mut cur_y_pos = 100.;
                if !msg.text.is_empty() {
                    let drawn_size = draw_multiline(
                        msg.text.clone(),
                        (screen_width() / 2. - 320., 100.),
                        max_line_length,
                        false,
                        Some(font),
                        30,
                        BLACK,
                    );
                    cur_y_pos += drawn_size.y;
                }
                if let Some(event) = msg
                    .corresponding_event_id
                    .and_then(|id| state.game.executor.events.get(id))
                {
                    let result = &event.result;
                    if let Some(added_units) = &result.add_units {
                        let valid_units = added_units
                            .iter()
                            .filter_map(|id| units.inner.get(*id))
                            .collect::<Vec<_>>();
                        let textures = valid_units
                            .iter()
                            .map(|unit|  get_unit_info_texture(&state.assets, *unit));
                        const SMALL_CARD_SIZE: f32 = 52.;
                        let offset = SMALL_CARD_SIZE + 20.;
                        let width = valid_units.len() as f32 * offset;
                        for (i, texture) in textures.enumerate() {
                            let pos_x = i as f32 * offset + screen_width() / 2. - width / 2.;
                            draw_texture_size(
                                texture,
                                (pos_x, cur_y_pos),
                                (SMALL_CARD_SIZE, SMALL_CARD_SIZE),
                            );
                        }
                        cur_y_pos += SMALL_CARD_SIZE + 20.;
                    }
					if result.change_gold != 0  || result.change_mana != 0{
						
					}
                }
				if Button::new("Следующее событие")
					.position(vec2(50., 50.))
					.size(vec2(300., 40.))
					.ui(&mut root_ui()) || is_key_pressed(KeyCode::Right)
				{
					let event_id = (msg.corresponding_event_id.unwrap_or(0) + 1) % state.game.executor.events.len();
					let event = state.game.executor.events.get(msg.corresponding_event_id.unwrap_or(0) + 1).unwrap_or_else(|| &state.game.executor.events[0]);
					state.ui.main = Menu::Message(Message::from_event(event, event_id));
					next_frame().await;
					continue;
				}
                let mut clicked = false;
                if msg.variants.is_empty() {
                    clicked = Button::new("Понятно")
                        .position(vec2(screen_center().x - 100., cur_y_pos + 20.))
                        .size(vec2(200., 40.))
                        .ui(&mut root_ui()) || is_key_pressed(KeyCode::Enter);
                } else {
                    let width = measure_text(
                        &msg.variants
                            .iter()
                            .fold(String::new(), |acc, item| acc + &item),
                        Some(font),
                        32,
                        1.,
                    )
                    .width
                        + (msg.variants.len() - 1) as f32 * (32. + 40.);
                    let mut cur_x = screen_center().x - width / 2.;
                    let mut selected = None;
                    for variant in &msg.variants {
                        let size = measure_text(&**variant, Some(font), 32, 1.).width;
                        let clicked = Button::new(&**variant)
                            .position(vec2(cur_x, cur_y_pos + 20.))
                            .size(vec2(size + 20., 40.))
                            .ui(&mut root_ui());
                        if clicked {
                            selected = Some(variant);
                        }
                        cur_x += width + 40.;
                    }
                    if selected.is_some() {
                        clicked = true;
                    }
                }
                if clicked {
                    // TODO: player 0
                    if !state.game.executor.players[0].execution_queue.is_empty() {
                        if let Execute::Message(msg) =
                            state.game.executor.players[0].execution_queue.remove(0)
                        {
                            state.ui.main = Menu::Message(msg);
                        }
                    } else {
                        let mut camera = Camera2D::from_display_rect(Rect::new(
                            0.,
                            SIZE.1 * 50.,
                            SIZE.0 * 50.,
                            -SIZE.1 * 50.,
                        ));
                        camera.rotation = 0.;
                        state.ui.main = Menu::Map(MapRenderSettings {
                            camera,
                            ..Default::default()
                        });
                    }
                };
            }
            Menu::Info => {
				let locale = &state.registry.locale;
                root_ui().pop_skin();
                root_ui().push_skin(&dop_skin);
                Window::new(hash!(), vec2(0., 0.), screen_size().into())
                    .titlebar(false)
                    .ui(&mut *root_ui(), |ui| {
                        if ui.button(None, "Items") {
                            *ui.get_any::<usize>(hash!("menu_info")) = 1;
                        }
                        if ui.button(None, "Units") {
                            *ui.get_any::<usize>(hash!("menu_info")) = 0;
                        }
                        let menu = *ui.get_any::<usize>(hash!("menu_info"));
                        match menu {
                            0 => {
                                let menu_unit_index = *ui.get_any::<usize>(hash!("menu_unit_info"));
                                let units = &state.registry.units.inner;
                                let unit = units.get(menu_unit_index);
                                let change = if let Some(unit) = unit {
                                    let texture = get_unit_info_texture(&state.assets, unit);
                                    Texture::new(texture.weak_clone())
                                        .size(CARD_SIZE, CARD_SIZE)
                                        .ui(ui)
                                } else {
                                    false
                                };
                                let val = ui.get_any::<usize>(hash!("info_unit"));
                                if change {
                                    if *val == 1 {
                                        *val = 0;
                                    } else {
                                        *val = 1
                                    };
                                }
                                if *val == 1 {
                                    let mut menu_unit_index = menu_unit_index;
                                    Group::new(hash!(), vec2(CARD_SIZE + 100., 800.))
                                        .layout(macroquad::ui::Layout::Vertical)
                                        .ui(ui, |ui| {
                                            for (unit_index, unit) in
                                                units.iter().enumerate()
                                            {
												let texture = get_unit_info_texture(&state.assets, unit);
												
                                                if ui.texture(
                                                    texture.weak_clone(),
                                                    CARD_SIZE,
                                                    CARD_SIZE,
                                                ) {
                                                    menu_unit_index = unit_index;
                                                };
                                                ui.label(None, &unit.name);
                                            }
                                        });
                                    *ui.get_any::<usize>(hash!("menu_unit_info")) = menu_unit_index;
                                }

                                Group::new(hash!(), vec2(2000., screen_height() - 100.))
                                    .layout(macroquad::ui::Layout::Vertical)
                                    .position(vec2(CARD_SIZE + 150., 100.))
                                    .ui(ui, |ui| {
                                        if let Some(unit) = unit {
                                            let info = display_unit(&From::from((unit.clone(), &state.registry.bonuses)), &state.registry);
                                            let size = screen_height() - CARD_SIZE - 150.;
                                            for mut string in info {
                                                let text_size = measure_text(
                                                    &string,
                                                    state.assets.fonts.get(BENGUIAT),
                                                    32,
                                                    1.,
                                                );
                                                if text_size.width > size {
                                                    let len = string.len();
                                                    let line_break = string
                                                        .char_indices()
                                                        .skip((size / 32.) as usize - 2)
                                                        .find_map(|(i, x)| {
                                                            (x.is_ascii_punctuation()
                                                                || x.is_whitespace())
                                                            .then(|| i)
                                                        })
                                                        .unwrap_or(len / 2);
                                                    let line2 = string.split_off(line_break);
                                                    ui.label(None, &string);
                                                    ui.label(None, &line2);
                                                } else {
                                                    ui.label(None, &string);
                                                }
                                            }
                                        }
                                    });
                            }
                            _ => {
                                let mut menu_item_index =
                                    *ui.get_any::<usize>(hash!("menu_item_info"));
                                let texture = {
                                    if let Some(item) = state.registry.items.inner.get(menu_item_index) {
                                        let texture = &item.icon;
                                        let texture = state.assets.get(texture);
                                        Some(texture.weak_clone())
                                    } else {
                                        None
                                    }
                                };
                                let change = if let Some(texture) = texture {
                                    Texture::new(texture.weak_clone()).size(50., 50.).ui(ui)
                                } else {
                                    ui.button(None, locale.get("ui_item"))
                                };
                                if change {
                                    let val = ui.get_bool(hash!("menu_items"));
                                    *val = !*val;
                                }
                                if *ui.get_bool(hash!("menu_items")) {
                                    ui.group(hash!(), vec2(160., 400.), |ui| {
                                        let items = &state.registry.items.inner;
                                        let items = items.iter().enumerate();
                                        for (item_index, item) in items {
                                            let texture = &item.icon;
                                            let texture = state.assets.get(&texture);
                                            if ui.texture(texture.weak_clone(), 50., 50.) {
                                                menu_item_index = item_index
                                            }
                                            ui.label(None, &item.name);
                                        }
                                    });
                                }
                                Group::new(hash!(), vec2(2000., 1000.))
                                    .position(vec2(200., 100.))
                                    .ui(ui, |ui| {
                                        if let Some(item) =
                                            state.registry.items.inner.get(menu_item_index)
                                        {
                                            for string in item.display_strings(locale).iter() {
                                                ui.label(None, &string);
                                            }
                                        }
                                    });
                                *ui.get_any::<usize>(hash!("menu_item_info")) = menu_item_index;
                            }
                        }
                        if is_key_released(KeyCode::Escape)
                            || is_quit_requested()
                            || Button::new("Exit")
                                .position(vec2(
                                    screen_width()
                                        - measure_text(
                                            "Exit",
                                            state.assets.get_font(BENGUIAT).into(),
                                            32,
                                            1.,
                                        )
                                        .width,
                                    0.0,
                                ))
                                .ui(ui)
                        {
                            state.ui.main = Menu::Main;
                        }
                    });
            }
            Menu::Atlas => {
				let (mut x, mut y) = (0., 0.);
				let mut max_y = 0.;
				for (_, texture) in state.assets.inner.iter() {
					
					let size = texture.size();
					if size.y > max_y {
						max_y = size.y;
					}
					draw_texture(&texture, x, y, WHITE);
					//draw_text(key, )
					if x + size.x > screen_width() {
						y += max_y;
						max_y = 0.;
						x = 0.;
					} else {
						x += size.x;
					}
				}
				
                // state
                //     .assets
                //     .inner
                //     .values()
                //     .skip(macroquad::time::get_time() as usize % state.assets.inner.len())
                //     .next()
                //     .unwrap(),
                // 0.,
                // 0.,
                // WHITE,
            },
            Menu::Battle => {
				let locale = &state.registry.locale;
                let ui = &mut *root_ui();
                let winner = draw_battle(&state.assets, &mut state.game, true, &state.registry);
                let (my_army, go_back) = if let GameVariant::Online(Online { conn, army, .. }) =
                    &mut state.game.variant
                {
                    let my_army = *army;
                    let go_back = if let Some(winner) = winner {
                        let text = if winner == my_army {
                            locale.get("ui_winner")
                        } else {
                            locale.get("ui_loser")
                        };
                        let size = measure_text(&text, Some(&state.assets.fonts[BENGUIAT]), 32, 1.);
                        let size = (size.width, size.height);
                        let pos = state
                            .ui
                            .camera
                            .world_to_screen(vec2((CARD_SIZE * 6.) / 2., 1080. / 2.));
                        let pos = (pos.x - size.0 / 2., pos.y - size.1 / 2.);
                        if ui.button(Some(pos.into()), text) {
                            conn.send_action(dt_server::Incoming::Disconnect);
                            true
                        } else {
                            false
                        }
                    } else {
                        false
                    };
                    if let Some(mes) = conn.req_one() {
                        process_event(&mut state.game, mes);
                    }
                    (Some(my_army), go_back)
                } else {
                    let go_back = if let Some(_) = winner {
                        let text = locale.get("ui_winner");

                        let size = measure_text(&text, Some(&state.assets.fonts[BENGUIAT]), 32, 1.);
                        let size = (size.width, size.height);
                        let pos = state
                            .ui
                            .camera
                            .world_to_screen(vec2((CARD_SIZE * 6.) / 2., 1080. / 2.));
                        let pos = (pos.x - size.0 / 2., pos.y - size.1 / 2.);
                        if ui.button(Some(pos.into()), text) {
                            true
                        } else {
                            false
                        }
                    } else {
                        false
                    };
                    (None, go_back)
                };
                //let pos = state.ui.camera.world_to_screen( vec2(CARD_SIZE * (12/2) as f32, 0.));
                let pos = CARD_SIZE * (12 / 2) as f32 / 1920. * screen_width();
                let size = screen_width() - pos;
                let battle = state.game.executor.battle.as_ref();
                ui.pop_skin();
                ui.push_skin(&dop_skin);
                Window::new(hash!(), vec2(pos, 000.), vec2(size, screen_height()))
                    .titlebar(false)
                    .close_button(true)
                    .ui(ui, |ui| {
                        if let Some(active) = battle.and_then(|battle| battle.active_unit) {
                            let troop = &state.game.executor.gamemap.armys[active.army].troops
                                [active.index];
                            let troop = troop.get();
							let texture = get_unit_texture(&state.assets, &troop.unit, &state.registry.units);
                            ui.texture(texture.weak_clone(), CARD_SIZE, CARD_SIZE);
                            let info = display_unit(&troop.unit, &state.registry);
                            for mut string in info {
                                let text_size =
                                    measure_text(&string, state.assets.fonts.get(BENGUIAT), 32, 1.);
                                if text_size.width > size {
                                    let len = string.len();
                                    let line_break = string
                                        .char_indices()
                                        .skip((size / 32.) as usize - 2)
                                        .find_map(|(i, x)| {
                                            (x.is_ascii_punctuation() || x.is_whitespace())
                                                .then(|| i)
                                        })
                                        .unwrap_or(len / 2);
                                    let line2 = string.split_off(line_break);
                                    ui.label(None, &string);
                                    ui.label(None, &line2);
                                } else {
                                    ui.label(None, &string);
                                }
                            }
                            let text = if Some(active.army) == my_army || my_army.is_none() {
                                locale.get("ui_my_move")
                            } else {
                                locale.get("ui_not_my_move")
                            };
                            ui.button(None, text);
                        }
                        if Button::new("Exit")
                            .position(vec2(
                                (screen_width() - pos)
                                    - measure_text(
                                        "Exit",
                                        state.assets.get_font(BENGUIAT).into(),
                                        32,
                                        1.,
                                    )
                                    .width,
                                0.0,
                            ))
                            .ui(ui)
                        {
                            state.game.variant = GameVariant::Single(Scenario { events: vec![] });
                            state.ui.main = Menu::Main;
                            *ui.get_bool(hash!("ready")) = false;
                        }
                    });

                if go_back {
                    state.game.variant = GameVariant::Single(Scenario { events: vec![] });
                    state.ui.main = Menu::Main;
                    *ui.get_bool(hash!("ready")) = false;
                }
            }
            Menu::RoomCreation => {
				let locale = &state.registry.locale;
                let connected = matches!(state.game.variant, GameVariant::Online(_));
                let mut connecting = false;
                Window::new(hash!(), vec2(0., 0.), vec2(screen_width(), screen_height()))
                    .close_button(true)
                    .titlebar(false)
                    .ui(&mut *root_ui(), |ui| {
                        ui.label(Some((50., 50.).into()), &locale.get("menu_game_name"));

                        ui.input_text(hash!(), &locale.get("ui_room_code"), &mut input);
                        if !connected
                            && (ui.button(Some((150., 150.).into()), locale.get("ui_connect"))
                                || is_key_released(KeyCode::Enter))
                        {
                            connecting = true;
                        }
                        if connected {
                            ui.label(Some((150., 150.).into()), &locale.get("ui_awaiting"));
                            if ui.button(None, locale.get("ui_cancel_connection")) {
                                state.game.variant =
                                    GameVariant::Single(Scenario { events: vec![] });
                            };
                        }
                        if let GameVariant::Online(Online { status, .. }) = &state.game.variant {
                            if status == &ConnectionStatus::Full(false) {
                                state.ui.main = Menu::BattleSetup;
                            }
                        }
                        if is_key_released(KeyCode::Escape)
                            || is_quit_requested()
                            || Button::new("Exit")
                                .position(vec2(
                                    screen_width()
                                        - measure_text(
                                            "Exit",
                                            state.assets.get_font(BENGUIAT).into(),
                                            32,
                                            1.,
                                        )
                                        .width,
                                    0.0,
                                ))
                                .ui(ui)
                        {
                            state.ui.main = Menu::Main;
                            state.game.variant = GameVariant::Single(Scenario { events: vec![] });
                        }
                    });
                if connecting {
                    // Beware, this sometimes fills the root partition with zeroes
                    let ip = format!("{}:{}", state.registry.settings.ip.to_string(), state.registry.settings.port);
                    debug!("CONNECTING TO {}", &ip);

                    let conn =
                        state
                            .rt
                            .block_on(connect(ip, input.clone(), hwid::get_id().unwrap()));
                    debug!("{:?}", conn);
                    let conn = if conn.is_err() {
                        warn!("CAN'T CONNECT TO MAIN SERVER");
                        state.rt.block_on(connect(
                            "147.185.221.26:58356".into(),
                            input.clone(),
                            hwid::get_id().unwrap(),
                        ))
                    } else {
                        conn
                    };
                    //root_ui().pop_skin();
                    if let Ok(conn) = conn {
                        state.game.variant = GameVariant::Online(Online {
                            conn,
                            status: ConnectionStatus::NotFull,
                            army: 0,
                        });
                    } else {
                        warn!("CAN'T CONNECT TO TUNNEL");
                        connecting = false;
                    }
                }
                if let GameVariant::Online(Online { conn, .. }) = &mut state.game.variant {
                    if let Some(mes) = conn.req_one() {
                        process_event(&mut state.game, mes);
                    }
                }
            }
            Menu::BattleSetup => {
				let locale = &state.registry.locale;
                root_ui().pop_skin();
                root_ui().push_skin(&dop_skin);
                draw_battle(&state.assets, &mut state.game, false, &state.registry);
                if let GameVariant::Online(Online { conn, status, .. }) = &mut state.game.variant {
                    if matches!(status, ConnectionStatus::Full(true)) {
                        conn.send_action(dt_server::Incoming::GetState);
                        state.ui.main = Menu::Battle;
                        continue;
                    }
                };
                if is_key_released(KeyCode::Minus) {
                    let size = screen_size();
                    request_new_screen_size(size.0 - 10., size.1 - 10.);
                }
                //let pos = state.ui.camera.world_to_screen( vec2(CARD_SIZE * (12/2) as f32, 0.));
                let pos = CARD_SIZE * (12 / 2) as f32 / 1920. * screen_width();
                Window::new(
                    hash!(),
                    vec2(pos, 000.),
                    vec2(screen_width() - pos, screen_height()),
                )
                .titlebar(false)
                .ui(&mut *root_ui(), |ui| {
                    {
                        let mut menu = None;
                        let cur_menu = *ui.get_any::<usize>(hash!("creation_menu"));
                        let ready = ui.get_bool(hash!("ready"));
                        if !*ready {
                            if cur_menu != 0 && ui.button(None, locale.get("ui_units_menu")) {
                                menu = Some(0);
                            };
                            if cur_menu != 1 && ui.button(None, locale.get("ui_items_menu")) {
                                menu = Some(1);
                            };
                            if cur_menu != 2 && ui.button(None, locale.get("ui_start_menu")) {
                                menu = Some(2);
                            };
                        }
                        if let Some(menu) = menu {
                            *ui.get_any::<usize>(hash!("creation_menu")) = menu;
                        }
                    };
                    let menu = *ui.get_any::<usize>(hash!("creation_menu"));
					
                    match menu {
                        0 => {
                            if let Some(troop) = &state.game.executor.gamemap.armys
                                [state.game.focus.army]
                                .get_troop(state.game.focus.pos)
                            {
                                let troop = troop.get();
								let texture = get_unit_texture(&state.assets, &troop.unit, &state.registry.units);
                                let info = display_unit(&troop.unit, &state.registry);
                                let size = screen_width() - pos;
                                for mut string in info {
                                    let text_size = measure_text(
                                        &string,
                                        state.assets.fonts.get(BENGUIAT),
                                        32,
                                        1.,
                                    );
                                    if text_size.width > size {
                                        let len = string.len();
                                        let line_break = string
                                            .char_indices()
                                            .skip((size / 32.) as usize - 2)
                                            .find_map(|(i, x)| {
                                                (x.is_ascii_punctuation() || x.is_whitespace())
                                                    .then(|| i)
                                            })
                                            .unwrap_or(len / 2);
                                        let line2 = string.split_off(line_break);
                                        ui.label(None, &string);
                                        ui.label(None, &line2);
                                    } else {
                                        ui.label(None, &string);
                                    }
                                }
                                Texture::new(texture.weak_clone())
                                    .size(CARD_SIZE, CARD_SIZE)
                                    .ui(ui);
                                if ui.button(None, locale.get("ui_displace")) || is_key_released(KeyCode::Delete) {
                                    if let GameVariant::Online(Online { conn, .. }) =
                                        &mut state.game.variant
                                    {
                                        conn.send_action(dt_server::Incoming::SetUnit((
                                            state.game.focus,
                                            None,
                                        )));
                                    } else {
                                        let armies = &mut state.game.executor.gamemap.armys;
                                        let army = state.game.focus.army;
                                        armies[army].set_unit_at(None, state.game.focus, &state.registry)
                                    }
                                    *ui.get_any::<usize>(hash!("unit")) = 0;
                                }
                            } else {
                                let change = ui.button(None, locale.get("ui_unit"));
                                let val = ui.get_any::<usize>(hash!("unit"));
                                if change {
                                    if *val == 1 {
                                        *val = 0;
                                    } else {
                                        *val = 1
                                    };
                                }
                                if *val == 1 {
                                    Group::new(
                                        hash!(),
                                        vec2(CARD_SIZE + 100., screen_height() - 120.),
                                    )
                                    .layout(macroquad::ui::Layout::Vertical)
                                    .ui(ui, |ui| {
                                        for (unit_index, unit) in
                                            state.registry.units.inner.iter().enumerate()
                                        {
                                            let texture = get_unit_info_texture(&state.assets, unit);
                                            if ui.texture(
                                                texture.weak_clone(),
                                                CARD_SIZE,
                                                CARD_SIZE,
                                            ) {
                                                if let GameVariant::Online(Online {
                                                    conn, ..
                                                }) = &mut state.game.variant
                                                {
                                                    conn.send_action(dt_server::Incoming::SetUnit(
                                                        (state.game.focus, Some(unit_index)),
                                                    ));
                                                } else {
                                                    let armies =
                                                        &mut state.game.executor.gamemap.armys;
                                                    let army = state.game.focus.army;
                                                    armies[army].set_unit_at(
                                                        Some(unit_index),
                                                        state.game.focus,
														&state.registry
                                                    );
                                                }
                                            }
                                            ui.label(None, &unit.name);
                                        }
                                    });
                                }
                            }
                        }
                        1 => {
                            if let Some(troop) = &mut state.game.executor.gamemap.armys
                                [state.game.focus.army]
                                .get_troop(state.game.focus.pos)
                            {
                                let troop = &mut troop.get();
                                let items: Vec<_> = troop
                                    .unit
                                    .inventory
                                    .items
                                    .iter()
                                    .map(|item| {
                                        item.and_then(|item| {
                                            let texture = &item.get_info(&state.registry.items).icon;
                                            Some(state.assets.get(&texture).weak_clone())
                                        })
                                    })
                                    .enumerate()
                                    .collect();
                                for (i, texture) in items {
                                    let mut has_item = false;
                                    let change = if let Some(texture) = texture {
                                        has_item = true;
                                        Texture::new(texture.weak_clone()).size(50., 50.).ui(ui)
                                    } else {
                                        ui.button(None, locale.get("ui_item"))
                                    };
                                    if change {
                                        let val = ui.get_any::<usize>(hash!("items"));
                                        if *val == i + 1 {
                                            *val = 0;
                                        } else {
                                            *val = i + 1
                                        };
                                    }
                                    if *ui.get_any::<usize>(hash!("items")) == i + 1 {
                                        let height = if has_item {
                                            70.
                                        } else {
                                            screen_height() - 300.
                                        };
                                        ui.group(hash!(), vec2(500., height), |ui| {
                                            if has_item {
                                                if ui.button(None, locale.get("ui_displace")) {
                                                    if let GameVariant::Online(Online {
                                                        conn,
                                                        ..
                                                    }) = &mut state.game.variant
                                                    {
                                                        conn.send_action(
                                                            dt_server::Incoming::SetItem((
                                                                state.game.focus,
                                                                (i, None),
                                                            )),
                                                        );
                                                    } else {
                                                        troop.unit.swap_item(i, None, &state.registry);
                                                    }
                                                    *ui.get_any::<usize>(hash!("items")) = 0;
                                                }
                                                return;
                                            }
                                            let items = &state.registry.items.inner;
                                            let possible: Vec<_> = items
                                                .iter()
                                                .enumerate()
                                                .filter(|(_, item)| item.can_equip(&troop.unit, &state.registry))
                                                .collect();
                                            for (item_index, item) in possible {
                                                let texture = &item.icon;
                                                let texture = state.assets.get(&texture);
                                                if ui.texture(texture.weak_clone(), 50., 50.) {
                                                    if let GameVariant::Online(Online {
                                                        conn,
                                                        ..
                                                    }) = &mut state.game.variant
                                                    {
                                                        conn.send_action(
                                                            dt_server::Incoming::SetItem((
                                                                state.game.focus,
                                                                (i, Some(item_index)),
                                                            )),
                                                        );
                                                    } else {
                                                        troop.unit.swap_item(
                                                            i,
                                                            Some(Item { index: item_index }),
															&state.registry
                                                        );
                                                    }
                                                }
                                                ui.label(None, &item.name);
                                            }
                                        });
                                    }
                                }
                            }
                        }
                        2 => {
                            let text = if *ui.get_bool(hash!("ready")) {
                                locale.get("ui_not_ready")
                            } else {
                                locale.get("ui_ready")
                            };
                            if ui.button(None, text) {
                                let ready = ui.get_bool(hash!("ready"));
                                *ready = ready.not();
                                if let GameVariant::Online(Online { conn, .. }) =
                                    &mut state.game.variant
                                {
                                    conn.send_action(dt_server::Incoming::Status(*ready));
                                } else {
                                    if *ready {
                                        state.ui.main = Menu::Battle;
                                        state.game.executor.battle.as_mut().and_then(|x| {
                                            Some(x.start(&mut state.game.executor.gamemap.armys, &state.registry))
                                        });
                                    }
                                }
                            }
                            if *ui.get_bool(hash!("ready")) {
                                ui.label(None, &locale.get("ui_awaiting"));
                            }
                        }
                        _ => {}
                    }
                    if Button::new("Exit")
                        .position(vec2(
                            (screen_width() - pos)
                                - measure_text(
                                    "Exit",
                                    state.assets.get_font(BENGUIAT).into(),
                                    32,
                                    1.,
                                )
                                .width,
                            0.0,
                        ))
                        .ui(ui)
                    {
                        state.game.variant = GameVariant::Single(Scenario { events: vec![] });
                        state.ui.main = Menu::Main;
                        *ui.get_bool(hash!("ready")) = false;
                    }
                });
                if let GameVariant::Online(Online { conn, .. }) = &mut state.game.variant {
                    if let Some(mes) = conn.req_one() {
                        process_event(&mut state.game, mes);
                    }
                }
            }
            _ => {}
        }
        root_ui().pop_skin();
        next_frame().await
    }
}
