#![windows_subsystem = "windows"]

use ahash::RandomState;
use dt_client::*;
use dt_lib::{
    battle::{army::*, battlefield::*, control::Player, troop::Troop},
    effects::*,
    hwid,
    items::item::*,
    locale::{find_all_matches_in_string, parse_locale, Locale},
    map::{
        convert::{convert_dtm_map, parse_dtm_map, parse_dtm_map_by_bytes, parse_dtm_vec},
        event::{execute_event, Event as GameEvent, Execute},
        map::*,
        object::ObjectInfo,
        tile::*,
    },
    network::{server::Executor, GameServer},
    parse::{
        collect_errors, parse_items, parse_objects, parse_settings, parse_story, parse_units,
        FileAccess, SETTINGS,
    },
    time::time::Data as TimeData,
    units::{
        unit::{calclate_unit_power, display_unit, ActionResult, Unit, UnitPos},
        unitstats::ModifyUnitStats,
    },
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
    sync::{Mutex, RwLock},
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
impl Index<&String> for Assets {
    type Output = Texture2D;
    fn index(&self, index: &String) -> &Self::Output {
        self.get(index)
    }
}
use dt_lib::parse::LOCALE;
static UNITS: Lazy<RwLock<Vec<Unit>>> = Lazy::new(|| RwLock::new(vec![]));
static OBJECTS: Lazy<RwLock<Vec<ObjectInfo>>> = Lazy::new(|| RwLock::new(vec![]));
const CARD_SIZE: f32 = 180.;
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
    pub game_local: Executor,
    pub ui: Ui,
    pub rt: Runtime,
}
impl State {
    fn new() -> Self {
        todo!()
    }
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
async fn game_init() -> State {
    let settings = parse_settings::<QuadFiles>().await;
    {
        let locale = &mut LOCALE.write().unwrap();
        locale.set_lang((&settings.locale, &settings.additional_locale));
        parse_locale::<QuadFiles>(&[&settings.locale, &settings.additional_locale], locale).await;
        dbg!(locale.keys_lack());
    }
    let benguiat = load_ttf_font("Benguiat Rus Regular.ttf")
        .await
        .expect("shit happened");
    let z003 = load_ttf_font("Z003-MediumItalic.ttf")
        .await
        .expect("shit happened");
    let gothic = load_ttf_font("Ru_Gothic.ttf").await.expect("shit happened");
    let mut fonts = HashMap::new();
    fonts.insert("benguiat", benguiat);
    fonts.insert("z003", z003);
    fonts.insert("gothic", gothic);
    let assets = {
        let req_assets_items = parse_items::<QuadFiles>(None, &settings.locale).await;
        assert!(ITEMS.read().unwrap().len() > 0);
        let mut res = parse_units::<QuadFiles>(Some("Units.ini")).await;
        if let Ok(ref mut res) = &mut res {
            res.1 .0 = "assets/Icons";
        }
        if let Err(err) = res {
            error!("{}", err);
            panic!("{}", err);
        }
        let Ok((mut units, req_assets_units)) = res else {
            panic!("Unit parsing error")
        };
        units_mut!().append(&mut units);
        let (mut objects, req_assets_objects) = parse_objects::<QuadFiles>().await;
        objects_mut!().append(&mut objects);
        dbg!(units!().len(), objects!().len());
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
        let req_assets_list = [
            req_assets_objects,
            req_assets_items,
            req_assets_units,
            req_assets_terrain_list,
            req_assets_windows,
            req_assets_stats_list,
            ("assets/", vec!["Army.png".to_string()]),
        ];
        load_assets(&req_assets_list, fonts).await
    };
    let map = /*"РК5-Столица в огне.DTm";*/ "Stinger-Paramount_War_HARD.dtm";
    let bytes = load_file(&format!("Maps_Rus/{}", map)).await.unwrap();
    let (mut gamemap, events) = convert_dtm_map(parse_dtm_vec(bytes).unwrap());
    {
        let objects = objects!();
        let mut count = HashMap::new();
        for deco in gamemap
            .decomap
            .iter()
            .filter(|&x| objects.iter().find(|el| el.id == x.index).is_none())
        {
            count
                .entry(deco.index.clone())
                .and_modify(|x: &mut usize| x.add_assign(1))
                .or_insert(1usize);
        }
        dbg!(count);
        export(&events, "events.ini");
    }
    let mut exec = Executor {
        map: gamemap.clone(),
        events: events.clone(),
        battle: None,
        execution_queue: vec![],
        players: vec![Player {
            army: 0,
            questbook: None,
            execution_queue: vec![],
        }],
    };
    exec.tick(units!());
    dbg!(&exec.execution_queue);
    dbg!(&exec.players[0].execution_queue);
    //let (mut gamemap, events) = parse_story(
    //     units!(),
    //     objects!(),
    //     &settings.locale,
    //     &settings.additional_locale,
    // );
    gamemap.calc_hitboxes(objects!());
    let mut battle = BattleInfo::new(&mut gamemap.armys, 0, 1);
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
    State {
        rt,
        assets,
        ui: Ui {
            main: Menu::Main,
            camera,
            stack: Vec::new(),
        },
        game: Game {
            gamemap,
            focus: BattleUnitPos { army: 0, pos: 0 },
            battle: Some(battle),
            variant: GameVariant::Single(Scenario { events }),
        },
        game_local: exec,
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
    pub gamemap: GameMap,
    pub battle: Option<BattleInfo>,
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
            tiles_render: true,
            armies_render: true,
            err_render: true,
        }
    }
}
#[derive(Debug)]
enum Menu {
    Main,
    Info,
    Map(MapRenderSettings),
    Atlas,
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
            &format!("{}/{}", troop.unit.stats.hp, stats.max_hp),
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
                pos.0 += measure_text(
                    "32",
                    Some(assets.get_font("benguiat")),
                    font_size as u16,
                    1.,
                )
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
        let text = &format!("Moves {}/{}", troop.unit.stats.moves, stats.max_moves);
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
            &match field_type(unit_pos, *MAX_TROOPS) {
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
                && (old_pos.0.abs_diff(unit_pos % (*MAX_TROOPS / MAX_LINES)) < 2
                    || field_type(unit_pos, *MAX_TROOPS) == Field::Reserve
                    || field_type(old_pos.whole(), *MAX_TROOPS) == Field::Reserve)
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
        for (i, (effect, lifetime)) in unit
            .effects
            .iter()
            .filter_map(|effect| match effect {
                Effect::BlockEffect(BlockEffect {
                    info: EffectInfo { lifetime },
                }) => Some(("checked-shield.png", *lifetime)),
                Effect::MoreMoves(MoreMoves {
                    info: EffectInfo { lifetime },
                }) => Some(("mounted-knight.png", *lifetime)),
                Effect::Fire(Fire {
                    info: EffectInfo { lifetime },
                    ..
                }) => Some(("small-fire.png", *lifetime)),
                Effect::RessurectedEffect(_) => Some(("raise-zombie.png", 0)),
                Effect::Poison(Poison {
                    info: EffectInfo { lifetime },
                }) => Some(("poison-bottle.png", *lifetime)),
                Effect::SpearEffect(SpearEffect {
                    info: EffectInfo { lifetime },
                }) => Some(("spartan.png", *lifetime)),
                _ => None,
            })
            .enumerate()
        {
            let texture = assets.get(effect);
            draw_texture_size(texture, (pos.0 + 40. * i as f32, pos.1), (32., 32.));
            if lifetime > 0 {
                draw_text(
                    &*lifetime.to_string(),
                    pos.0 + 40. * i as f32 + 5.,
                    pos.1 + 32.,
                    32.,
                    WHITE,
                );
            }
        }
    }
    let troop = unit.get();
    let unit_texture = assets.get(&format!("unit_{}.png", troop.unit.info.icon_index));
    let is_interactable = battle.can_interact.as_ref().is_some_and(|x| {
        x.contains(&BattleUnitPos {
            army,
            pos: unit_pos,
        })
    });
    let size = troop.unit.info.size;
    let draw_size = (size.0 as f32 * CARD_SIZE, size.1 as f32 * CARD_SIZE);
    let draw_rect = Rect::new(pos.0, pos.1, draw_size.0, draw_size.1);
    let stats = troop.unit.modified;
    let hp = 1. - troop.unit.stats.hp as f32 / stats.max_hp as f32;
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
            let texture = item.get_info().icon;
            let texture = assets.get(&texture);
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
fn draw_battle(assets: &Assets, game: &mut Game, is_battle_active: bool) -> Option<usize> {
    let (armies, battle) = (&mut game.gamemap.armys, &mut game.battle);
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
    let half_troops = *MAX_TROOPS / 2;
    let winner = battle.winner;
    for army in 0..=1 {
        for row in 0..=1 {
            for i in 0..(*MAX_TROOPS / 2) {
                let unit_pos = i + (army as i64 - row as i64).abs() as usize * half_troops;
                let army = if army == 0 {
                    battle.army1
                } else {
                    battle.army2
                } as usize;
                // Check for non-single cell units, so they will be skipped
                let hitmap = &armies[army].hitmap;
                let skip = {
                    let vertical = unit_pos >= *MAX_TROOPS / 2
                        && hitmap[unit_pos] == hitmap[unit_pos - *MAX_TROOPS / 2];
                    let horizontal = field_type(unit_pos, *MAX_TROOPS) != Field::Reserve
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
                        handle_action((unit_pos, army), battle, armies);
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
const SIZE: (f32, f32) = (256., 176.);
fn draw_map(
    assets: &Assets,
    settings: &mut MapRenderSettings,
    game: &mut Game,
    executor: &mut Executor,
) -> Option<Menu> {
    let (gamemap, battle) = (&mut game.gamemap, &mut game.battle);
    let camera = &mut settings.camera;
    set_camera(camera);

    if settings.tiles_render {
        for i in 0..gamemap.tilemap.size {
            for j in 0..gamemap.tilemap.size {
                let tile_index = gamemap.tilemap[(j, i)];
                let tile = TILES[tile_index];
                let tileset_index = count_tileset_index(&gamemap, (j, i));

                draw_texture_ex(
                    assets.get(&tile.sprite().to_string()),
                    i as f32 * SIZE.0,
                    j as f32 * SIZE.1,
                    WHITE,
                    DrawTextureParams {
                        dest_size: Some(vec2(SIZE.0, SIZE.1)),
                        ..Default::default()
                    },
                );
                if is_key_down(KeyCode::F) {
                    continue;
                }
                // if tileset_index != 0 {
                // 	draw_texture_ex(
                // 		assets.get(&"grass_tileset.png".to_string()),
                // 		i as f32 * SIZE.0,
                // 		j as f32 * SIZE.1,
                // 		WHITE,
                // 		DrawTextureParams {
                // 			source: Rect::new(
                // 				(tileset_index % 4) as f32 * 256.,
                // 				(tileset_index / 4) as f32 * 242.,
                // 				256., 242.
                // 			).into(),
                // 			..Default::default()
                // 		}
                // 	)
                //}
                //draw_text(&*format!("{};{}", i, j), i as f32 * 256., j as f32 * 242., 30., BLACK);
            }
        }
    }
    let err_texture = assets.get(&"small-fire.png".to_string());
    if settings.deco_render {
        for deco in &gamemap.decomap {
            let (i, j) = (deco.x, deco.y);
            if let Some(obj) = objects!().iter().find(|x| x.id == deco.index) {
                let texture = assets.get(&obj.path.clone());
                let size = texture.size();
                draw_texture_ex(
                    texture,
                    i as f32 * SIZE.0,
                    j as f32 * SIZE.1,
                    WHITE,
                    DrawTextureParams {
                        dest_size: Some(Vec2::new(
                            size.x as f32 / 32. * SIZE.0,
                            size.y as f32 / 22. * SIZE.1,
                        )),
                        ..Default::default()
                    },
                );
            } else if settings.err_render {
                let size = err_texture.size();
                draw_texture_ex(
                    err_texture,
                    i as f32 * SIZE.0,
                    j as f32 * SIZE.1,
                    WHITE,
                    DrawTextureParams {
                        dest_size: Some(Vec2::new(SIZE.0, SIZE.1)),
                        //dest_size: Some(Vec2::new(size.x as f32 / 32. * SIZE.0, size.y as f32 / 22. * SIZE.1)),
                        ..Default::default()
                    },
                );
            }
        }
    }
    if settings.armies_render {
        for hit in &gamemap.hitmap.inner {
            if let Some(army) = hit.army {
                let (i, j) = gamemap.armys[army].pos;
                let size = vec2(50., 90.);
                draw_texture_ex(
                    assets.get(&"Army.png".to_string()),
                    i as f32 * SIZE.0,
                    j as f32 * SIZE.1,
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
    if is_key_pressed(KeyCode::T) {
        settings.tiles_render = !settings.tiles_render;
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
    if is_mouse_button_released(MouseButton::Left) {
        let tile_size = vec2(SIZE.0, SIZE.1);
        let pos = camera.screen_to_world(mouse_position().into());
        let tile = (pos / tile_size).floor();
        executor.message_handler(
            dt_lib::network::server::ClientMessage::GoTo((
                tile.x as usize % 200,
                tile.y as usize % 200,
            )),
            0,
        );
    }
    if !executor.players[0].execution_queue.is_empty() {
        if let Execute::Message(msg) = executor.players[0].execution_queue.remove(0) {
            dbg!(msg.text);
        }
    }
    if is_mouse_button_down(MouseButton::Right) {
        //camera.offset += camera.screen_to_world(mouse_position().into()) / vec2(-SIZE.0 * 50., SIZE.1 * 30.);
        camera.offset += mouse_delta_position() * vec2(-1., 1.);
        //dbg!(camera.screen_to_world(mouse_position().into()) / vec2(-SIZE.0 * 50., SIZE.1 * 30.));
    }
    executor.tick(units!());
    return None;
}
fn process_event(state: &mut State, event: IncomingEvent) {
    let GameVariant::Online(conn) = &mut state.game.variant else {
        return;
    };
    match event {
        IncomingEvent::Id(army) => {
            println!("My id is {army}");
            conn.army = army;
        }
        IncomingEvent::Game(game) => {
            state.game.gamemap.armys = game.0;
            state.game.battle = Some(game.1);
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
        .with_font(state.assets.get_font("benguiat"))
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
        .with_font(state.assets.get_font("benguiat"))
        .unwrap()
        .build();
    let label = root_ui()
        .style_builder()
        .text_color(BLACK)
        .text_color_hovered(DARKBLUE)
        .font_size(32)
        .with_font(state.assets.get_font("benguiat"))
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
                        let mut locale = LOCALE.write().unwrap();
                        ui.label(Some((50., 50.).into()), &locale.get("menu_game_name"));
                        if ui.button(Some((250., 300.).into()), locale.get("menu_start_title")) {
                            let mut camera = Camera2D::from_display_rect(Rect::new(
                                0.,
                                SIZE.1 * 30.,
                                SIZE.0 * 50.,
                                -SIZE.1 * 30.,
                            ));
                            camera.rotation = 0.;
                            dbg!(&camera);
                            state.ui.main = Menu::Map(MapRenderSettings {
                                camera,
                                ..Default::default()
                            });
                        }
                        // if ui.button(Some((50., 150.).into()), "Atlas") {
                        // 	state.ui.main = Menu::Atlas;
                        // }
                        // if ui.button(Some((50., 250.).into()), locale.get("menu_battle_title")) {
                        // 	state.ui.main = Menu::Battle;
                        // }
                        if ui.button(Some((50., 100.).into()), locale.get("menu_pvp_title")) {
                            state.ui.main = Menu::RoomCreation;
                        }
                        if ui.button(Some((50., 150.).into()), locale.get("menu_pve_title")) {
                            state.ui.main = Menu::BattleSetup;
                        }
                        if ui.button(vec2(50., 200.), locale.get("menu_language")) {
                            locale.switch_lang();
                        }
                        if ui.button(Some((50., 250.).into()), "Info") {
                            state.ui.main = Menu::Info;
                        }
                    });
            }
            Menu::Map(settings) => {
                if let Some(menu) = draw_map(
                    &state.assets,
                    settings,
                    &mut state.game,
                    &mut state.game_local,
                ) {
                    state.ui.main = menu;
                };
            }
            Menu::Info => {
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
                                let units = UNITS.read().unwrap();
                                let unit = units.get(menu_unit_index);
                                let change = if let Some(unit) = unit {
                                    let texture = state
                                        .assets
                                        .get(&format!("unit_{}.png", unit.info.icon_index));
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
                                                UNITS.read().unwrap().iter().enumerate()
                                            {
                                                let texture = state.assets.get(&format!(
                                                    "unit_{}.png",
                                                    unit.info.icon_index
                                                ));
                                                if ui.texture(
                                                    texture.weak_clone(),
                                                    CARD_SIZE,
                                                    CARD_SIZE,
                                                ) {
                                                    menu_unit_index = unit_index;
                                                }
                                                ui.label(None, &unit.info.name);
                                            }
                                        });
                                    *ui.get_any::<usize>(hash!("menu_unit_info")) = menu_unit_index;
                                }

                                Group::new(hash!(), vec2(2000., screen_height() - 100.))
                                    .layout(macroquad::ui::Layout::Vertical)
                                    .position(vec2(CARD_SIZE + 150., 100.))
                                    .ui(ui, |ui| {
                                        if let Some(unit) = unit {
                                            let info = display_unit(unit);
                                            let size = screen_height() - CARD_SIZE - 150.;
                                            for mut string in info {
                                                let text_size = measure_text(
                                                    &string,
                                                    state.assets.fonts.get("benguiat"),
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
                                    if let Some(item) = ITEMS.read().unwrap().get(menu_item_index) {
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
                                    let locale = LOCALE.read().unwrap();
                                    ui.button(None, locale.get("ui_item"))
                                };
                                if change {
                                    let val = ui.get_bool(hash!("menu_items"));
                                    *val = !*val;
                                }
                                if *ui.get_bool(hash!("menu_items")) {
                                    ui.group(hash!(), vec2(160., 400.), |ui| {
                                        let items = ITEMS.read().unwrap();
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
                                            ITEMS.read().unwrap().get(menu_item_index)
                                        {
                                            for string in item.display_strings().iter() {
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
                                            state.assets.get_font("benguiat").into(),
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
                let ui = &mut *root_ui();
                let locale = LOCALE.read().unwrap();
                let winner = draw_battle(&state.assets, &mut state.game, true);
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
                        let size =
                            measure_text(&text, Some(&state.assets.fonts["benguiat"]), 32, 1.);
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
                        process_event(&mut state, mes);
                    }
                    (Some(my_army), go_back)
                } else {
                    let go_back = if let Some(_) = winner {
                        let text = locale.get("ui_winner");

                        let size =
                            measure_text(&text, Some(&state.assets.fonts["benguiat"]), 32, 1.);
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
                //let pos = state.ui.camera.world_to_screen( vec2(CARD_SIZE * (*MAX_TROOPS/2) as f32, 0.));
                let pos = CARD_SIZE * (*MAX_TROOPS / 2) as f32 / 1920. * screen_width();
                let size = screen_width() - pos;
                let battle = state.game.battle.as_ref();
                ui.pop_skin();
                ui.push_skin(&dop_skin);
                Window::new(hash!(), vec2(pos, 000.), vec2(size, screen_height()))
                    .titlebar(false)
                    .close_button(true)
                    .ui(ui, |ui| {
                        let locale = LOCALE.read().unwrap();
                        if let Some(active) = battle.and_then(|battle| battle.active_unit) {
                            let troop = &state.game.gamemap.armys[active.army].troops[active.index];
                            let troop = troop.get();
                            let texture = state
                                .assets
                                .get(&format!("unit_{}.png", troop.unit.info.icon_index));
                            ui.texture(texture.weak_clone(), CARD_SIZE, CARD_SIZE);
                            let info = display_unit(&troop.unit);
                            for mut string in info {
                                let text_size = measure_text(
                                    &string,
                                    state.assets.fonts.get("benguiat"),
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
                                        state.assets.get_font("benguiat").into(),
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
                let connected = matches!(state.game.variant, GameVariant::Online(_));
                let mut connecting = false;
                Window::new(hash!(), vec2(0., 0.), vec2(screen_width(), screen_height()))
                    .close_button(true)
                    .titlebar(false)
                    .ui(&mut *root_ui(), |ui| {
                        let locale = LOCALE.read().unwrap();
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
                                            state.assets.get_font("benguiat").into(),
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
                    let ip = unsafe { format!("{}:{}", SETTINGS.ip.to_string(), SETTINGS.port) };
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
                        process_event(&mut state, mes);
                    }
                }
            }
            Menu::BattleSetup => {
                root_ui().pop_skin();
                root_ui().push_skin(&dop_skin);
                draw_battle(&state.assets, &mut state.game, false);
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
                //let pos = state.ui.camera.world_to_screen( vec2(CARD_SIZE * (*MAX_TROOPS/2) as f32, 0.));
                let pos = CARD_SIZE * (*MAX_TROOPS / 2) as f32 / 1920. * screen_width();
                Window::new(
                    hash!(),
                    vec2(pos, 000.),
                    vec2(screen_width() - pos, screen_height()),
                )
                .titlebar(false)
                .ui(&mut *root_ui(), |ui| {
                    let locale = LOCALE.read().unwrap();
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
                            if let Some(troop) = &state.game.gamemap.armys[state.game.focus.army]
                                .get_troop(state.game.focus.pos)
                            {
                                let troop = troop.get();
                                let texture = state
                                    .assets
                                    .get(&format!("unit_{}.png", troop.unit.info.icon_index));
                                let info = display_unit(&troop.unit);
                                let size = screen_width() - pos;
                                for mut string in info {
                                    let text_size = measure_text(
                                        &string,
                                        state.assets.fonts.get("benguiat"),
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
                                if ui.button(None, locale.get("ui_displace")) {
                                    if let GameVariant::Online(Online { conn, .. }) =
                                        &mut state.game.variant
                                    {
                                        conn.send_action(dt_server::Incoming::SetUnit((
                                            state.game.focus,
                                            None,
                                        )));
                                    } else {
                                        let armies = &mut state.game.gamemap.armys;
                                        let army = state.game.focus.army;
                                        armies[army].set_unit_at(None, state.game.focus)
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
                                            UNITS.read().unwrap().iter().enumerate()
                                        {
                                            let texture = state
                                                .assets
                                                .get(&format!("unit_{}.png", unit.info.icon_index));
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
                                                    let armies = &mut state.game.gamemap.armys;
                                                    let army = state.game.focus.army;
                                                    armies[army].set_unit_at(
                                                        Some(unit_index),
                                                        state.game.focus,
                                                    );
                                                }
                                            }
                                            ui.label(None, &unit.info.name);
                                        }
                                    });
                                }
                            }
                        }
                        1 => {
                            if let Some(troop) = &mut state.game.gamemap.armys
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
                                            let texture = item.get_info().icon;
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
                                                        troop.unit.swap_item(i, None);
                                                    }
                                                    *ui.get_any::<usize>(hash!("items")) = 0;
                                                }
                                                return;
                                            }
                                            let items = ITEMS.read().unwrap();
                                            let possible: Vec<_> = items
                                                .iter()
                                                .enumerate()
                                                .filter(|(_, item)| item.can_equip(&troop.unit))
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
                                        state.game.battle.as_mut().and_then(|x| {
                                            Some(x.start(&mut state.game.gamemap.armys))
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
                                    state.assets.get_font("benguiat").into(),
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
                        process_event(&mut state, mes);
                    }
                }
            }
            _ => {}
        }
        root_ui().pop_skin();
        next_frame().await
    }
}
