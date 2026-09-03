// Порт состояния игры из quad_ui main.rs: State/Game/GameVariant/Menu/UiState,
// game_init, load_map, process_event и хелперы текстур юнитов.
use crate::assets::{load_assets, load_fonts, Assets, BENGUIAT};
use crate::camera::Camera;
use crate::files::DesktopFiles;
use crate::gfx::{Gfx, TexId};
use crate::text::{FontId, TextRenderer};
use dt_lib::battle::army::*;
use dt_lib::battle::control::{Control, Player};
use dt_lib::battle::troop::Troop;
use dt_lib::locale::{parse_locale, Locale};
use dt_lib::map::convert::{convert_dtm_map, parse_dtm_vec};
use dt_lib::map::map::GameMap;
use dt_lib::mutrc::SendMut;
use dt_lib::map::event::{Event as GameEvent, Events};
use dt_lib::network::server::Executor;
use dt_lib::parse::{
    parse_bonuses, parse_effects, parse_items, parse_objects, parse_settings, parse_units,
};
use dt_lib::registry::GameInfo;
use dt_lib::units::unit::UnitInfo;
use dt_lib::units::{unit::Unit, unitstats::ModifyUnitStats};
use dt_client::{Connection, IncomingEvent};
use std::collections::HashMap;
use tokio::runtime::Runtime;

pub const SIZE: (f32, f32) = (32., 22.);
pub const CARD_SIZE: f32 = 160.;

#[derive(Debug)]
pub struct Scenario {
    pub events: Vec<GameEvent>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum ConnectionStatus {
    NotFull,
    Full(bool),
}

#[derive(Debug)]
pub struct Online {
    pub conn: Connection,
    pub status: ConnectionStatus,
    pub army: usize,
}

#[derive(Debug)]
pub enum GameVariant {
    Single(Scenario),
    Online(Online),
}

#[derive(Debug)]
pub struct Game {
    pub executor: Executor,
    pub variant: GameVariant,
    pub focus: dt_lib::battle::battlefield::BattleUnitPos,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum BlendMode {
    Off,
    Edges,
    EdgesStrong,
    Rounded,
    RoundedStrong,
}
impl BlendMode {
    pub const ALL: [BlendMode; 5] = [
        BlendMode::Off,
        BlendMode::Edges,
        BlendMode::EdgesStrong,
        BlendMode::Rounded,
        BlendMode::RoundedStrong,
    ];
    pub fn next(self) -> BlendMode {
        let i = BlendMode::ALL.iter().position(|m| *m == self).unwrap();
        BlendMode::ALL[(i + 1) % BlendMode::ALL.len()]
    }
    pub fn strong(self) -> bool {
        matches!(self, BlendMode::EdgesStrong | BlendMode::RoundedStrong)
    }
    pub fn rounded(self) -> bool {
        matches!(self, BlendMode::Rounded | BlendMode::RoundedStrong)
    }
}

#[derive(Debug, Clone)]
pub struct MapRenderSettings {
    pub camera: Camera,
    pub deco_render: bool,
    pub buildings_render: bool,
    pub event_render: bool,
    pub tiles_render: bool,
    pub armies_render: bool,
    pub err_render: bool,
    pub seed: u64,
    pub decos_dirty: bool,
    pub blend_mode: BlendMode,
    pub blend_dirty: bool,
}
impl Default for MapRenderSettings {
    fn default() -> Self {
        Self {
            camera: Camera::from_display_rect(0., SIZE.1 * 50., SIZE.0 * 50., -SIZE.1 * 50.),
            deco_render: true,
            buildings_render: true,
            event_render: true,
            tiles_render: true,
            armies_render: true,
            err_render: false,
            seed: 0,
            decos_dirty: false,
            blend_mode: BlendMode::Rounded,
            blend_dirty: false,
        }
    }
}

#[derive(Debug, Clone)]
pub enum Menu {
    Main,
    Atlas,
    Info,
    Map(MapRenderSettings),
    Message(dt_lib::map::event::Message),
    Battle,
    RoomCreation,
    BattleSetup,
}

/// Порт struct Ui (quad_ui main.rs:531): текущее меню, UI-камера, стек.
#[derive(Debug)]
pub struct UiState {
    pub main: Menu,
    pub camera: Camera,
    pub stack: Vec<Menu>,
}

#[derive(Debug)]
pub struct RenderTextures {
    pub map: TexId,
    pub decos: TexId,
}

#[derive(Debug)]
pub struct State {
    pub assets: Assets,
    pub registry: GameInfo,
    pub textures: RenderTextures,
    pub game: Game,
    pub ui: UiState,
    pub delta: f32,
    pub rt: Runtime,
    /// Пиксельные снапшоты тайлов (TILES), для генерации наплывов.
    pub tile_pixels: Vec<image::RgbaImage>,
}

// ---------------- Хелперы юнитов (quad_ui main.rs:696-715) ----------------

pub fn get_unit_texture(assets: &Assets, unit: &Unit, units: &dt_lib::registry::Units) -> TexId {
    assets.get(&format!("unit_{}.png", unit.get_info(units).icon_index - 1))
}

pub fn get_unit_info_texture(assets: &Assets, unit: &UnitInfo) -> TexId {
    assets.get(&format!("unit_{}.png", unit.icon_index - 1))
}

pub fn get_troop_texture(
    assets: &Assets,
    armies: &Vec<Army>,
    unit: usize,
    army: usize,
    units: &dt_lib::registry::Units,
) -> Option<TexId> {
    armies.get(army).and_then(|x| {
        x.troops.get(unit).and_then(|tr| {
            let troop = tr.get();
            Some(assets.get(&format!(
                "unit_{}.png",
                troop.unit.get_info(units).icon_index - 1
            )))
        })
    })
}

// ---------------- load_map / game_init (quad_ui main.rs:139-384) ----------------

pub fn load_map(map: &str, registry: &GameInfo) -> (GameMap, Events) {
    let bytes = crate::files::read_file_bytes(&format!("Maps_Rus/{map}"));
    let (mut gamemap, events) = convert_dtm_map(parse_dtm_vec(bytes).unwrap(), registry);
    gamemap.calc_hitboxes(&registry.objects.inner);
    if gamemap.armys.is_empty() {
        gamemap.armys.push(Army::new(
            vec![SendMut::new(Troop::new(
                (registry.units.inner[0].clone(), &registry.bonuses).into(),
            ))],
            ArmyStats {
                gold: 0,
                army_name: "".into(),
                mana: 0,
            },
            vec![],
            (1, 1),
            true,
            Control::Player(0),
            registry,
        ));
    }
    (gamemap, events)
}

pub async fn game_init(gfx: &mut Gfx, text: &mut TextRenderer) -> State {
    let mut registry = GameInfo::new();
    let settings = parse_settings::<DesktopFiles>().await;
    let mut locale = Locale::new("Rus".into(), "Eng".into());
    {
        locale.set_lang((&settings.locale, &settings.additional_locale));
        parse_locale::<DesktopFiles>(&[&settings.locale, &settings.additional_locale], &mut locale).await;
    }
    registry.locale = locale;
    parse_bonuses::<DesktopFiles>(None, &mut registry).await;
    parse_effects::<DesktopFiles>(None, &mut registry).await;
    let fonts = load_fonts(text);
    let assets = {
        let req_assets_items =
            parse_items::<DesktopFiles>(None, &settings.locale, &mut registry).await;
        let mut res = parse_units::<DesktopFiles>(Some("Units.ini"), &mut registry).await;
        if let Ok(res) = &mut res {
            res.0 = "assets/Icons";
        }
        if let Err(err) = res {
            panic!("{}", err);
        }
        let Ok(req_assets_units) = res else {
            panic!("Unit parsing error")
        };
        let req_assets_objects = parse_objects::<DesktopFiles>(&mut registry).await;
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
        load_assets(gfx, text, &req_assets_list, fonts).await
    };
    // Снапшоты тайлов: декодируем спрайты TILES напрямую (замена
    // get_texture_data снапшотов до build_textures_atlas — атласа больше нет).
    let tile_pixels = dt_lib::map::tile::TILES
        .iter()
        .map(|tile| {
            image::load_from_memory(&crate::files::read_file_bytes(&format!(
                "assets/Terrain/{}",
                tile.sprite()
            )))
            .expect("tile sprite decode failed")
            .to_rgba8()
        })
        .collect::<Vec<_>>();
    let map = "Stinger-Paramount_War_HARD.dtm";
    let (mut gamemap, events) = load_map(map, &registry);
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
    gamemap.calc_hitboxes(&registry.objects.inner);
    let rt = tokio::runtime::Runtime::new().unwrap();
    let camera = Camera::from_display_rect(0., 1080., 1920., -1080.);
    let game = Game {
        executor,
        focus: dt_lib::battle::battlefield::BattleUnitPos { army: 0, pos: 0 },
        variant: GameVariant::Single(Scenario { events }),
    };
    let _ = BENGUIAT;
    State {
        registry,
        rt,
        delta: 0.,
        assets,
        textures: RenderTextures {
            map: TexId(0),
            decos: TexId(0),
        },
        ui: UiState {
            main: Menu::Main,
            camera,
            stack: Vec::new(),
        },
        game,
        tile_pixels,
    }
}

/// Порт process_event (quad_ui main.rs:2105).
pub fn process_event(game: &mut Game, event: IncomingEvent) {
    let GameVariant::Online(conn) = &mut game.variant else {
        return;
    };
    match event {
        IncomingEvent::Id(army) => {
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
            _ => {}
        },
        IncomingEvent::Acceptance(a) => {
            if a == [true, true] {
                conn.status = ConnectionStatus::Full(true);
            }
        }
    }
}

/// Порт screen_center (quad_ui main.rs:562).
pub fn screen_center(input: &crate::ui::InputState) -> [f32; 2] {
    [input.screen_width() / 2., input.screen_height() / 2.]
}

/// Ввод строки для editbox (порт macroquad ui.input_text семантики).
pub fn collect_edit_input(input: &crate::ui::InputState, buf: &mut String) {
    for ch in &input.chars {
        if !ch.is_control() {
            buf.push(*ch);
        }
    }
    if input.key_down(winit::keyboard::KeyCode::Backspace) {
        buf.pop();
    }
}


