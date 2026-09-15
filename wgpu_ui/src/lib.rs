// wgpu_ui — порт quad_ui на wgpu + winit. Точка входа, ApplicationHandler,
// кадр: input -> пассы по меню -> present. egui сознательно отсутствует.
pub mod assets;
pub mod bake;
pub mod battle_view;
pub mod building_view;
pub mod camera;
pub mod egui_layer;

use egui as egui_crate;
pub mod editor_ui;
pub mod editor_view;
pub mod files;
pub mod gfx;
pub mod map_view;
pub mod rich_presence;
pub mod pvp_online;
pub mod screens;
pub mod state;
pub mod text;
pub mod ui;

use camera::Camera;
use gfx::{colors, Gfx, Rt, TexId};
use state::{Game, Menu, State};
use std::sync::{Arc, LazyLock};
use text::TextRenderer;
use ui::{InputState, Skin, WindowDecor};
use winit::application::ApplicationHandler;
use winit::event::{ElementState, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::window::{Fullscreen, Window};

static START: LazyLock<std::time::Instant> = LazyLock::new(std::time::Instant::now);
static WAKES: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
static REDRAWS: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
pub fn time_secs() -> f64 {
    START.elapsed().as_secs_f64()
}

/// Замена macroquad get_fps(): значение обновляется в frame().
static FPS_X100: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
pub fn fps() -> f32 {
    FPS_X100.load(std::sync::atomic::Ordering::Relaxed) as f32 / 100.
}

/// Тип карты виджет-состояния (порт ui.get_any/get_bool).
pub type WidgetMap = ahash::HashMap<u64, ui::Val>;

/// Скины (порт main_skin/dop_skin, quad_ui main.rs:2164-2230). building —
/// окно строений на плитке Win-marble; battle — боевой экран на Win-red.
pub struct Skins {
    pub main: Skin,
    pub dop: Skin,
    pub building: Skin,
    pub battle: Skin,
}

impl Skins {
    pub fn build(gfx: &mut Gfx, assets: &assets::Assets) -> Skins {
        // В macroquad скиновые текстуры создавались из пикселей уже загруженных
        // текстур при дефолтном Nearest-фильтре; здесь — из тех же файлов, Nearest.
        let mut load = |name: &str| -> gfx::TexId {
            let bytes = files::read_file_bytes(&format!("assets/Window/{name}"));
            gfx.push_texture_bytes(&bytes, gfx::Filter::Nearest)
        };
        let button = load("button.png");
        let menu = load("Menu.png");
        let paper = load("Paper.png");
        let font = assets.get_font(assets::BENGUIAT);
        // Кнопки/кресты/декор из оригинальных ассетов Window.
        let btn_up = load("Btn1Up.png");
        let btn_down = load("Btn1Down.png");
        let close_up = load("CloseButtonRed-Up.png");
        let close_down = load("CloseButtonRed-Down2.png");
        let confirm = load("CloseButtonGreen-Up.png");
        let marble = load("Win-marble.png");
        let red = load("Win-red.png");
        let band = load("WinLong.png");
        let decor = WindowDecor {
            tile: marble,
            band,
            corner_lu: load("Corner_Frame-LU.png"),
            corner_ru: load("Corner_Frame-RU.png"),
            corner_ld: load("Corner_Frame-LD.png"),
            corner_rd: load("Corner_Frame-RD.png"),
        };
        let red_decor = WindowDecor { tile: red, ..decor };
        let base = Skin {
            font,
            font_size: 32,
            text_color: colors::WHITE,
            text_hovered: colors::DARKBLUE,
            button_tex: button,
            button_down_tex: button,
            window_tex: menu,
            window_decor: None,
            close_up,
            close_down,
            confirm_tex: confirm,
        };
        let dop = Skin { text_color: colors::BLACK, window_tex: paper, ..base.clone() };
        // Вкладки/кнопки строений — Btn1 (старый button.png остался для прочих экранов).
        let mut building = base.clone();
        building.button_tex = btn_up;
        building.button_down_tex = btn_down;
        building.window_decor = Some(decor);
        let mut battle = base.clone();
        battle.button_tex = btn_up;
        battle.button_down_tex = btn_down;
        battle.window_decor = Some(red_decor);
        Skins { main: base, dop, building, battle }
    }
}

/// Контекст экрана: split-borrow всех кусков State + рендер-слоёв.
pub struct Ctx<'a> {
    pub gfx: &'a mut Gfx,
    pub text: &'a mut TextRenderer,
    pub input: &'a InputState,
    pub skins: &'a Skins,
    pub widgets: &'a mut WidgetMap,
    pub ui: &'a mut state::UiState,
    pub game: &'a mut Game,
    pub registry: &'a mut dt_lib::registry::GameInfo,
    pub assets: &'a assets::Assets,
    pub tile_pixels: &'a [image::RgbaImage],
    pub textures: &'a state::RenderTextures,
    pub rts: &'a (Rt, Rt),
    pub window: &'a Window,
    pub delta: &'a mut f32,
    pub rt: &'a tokio::runtime::Runtime,
    /// Сохранённые настройки карты (камера/флаги) на время просмотра событий.
    pub map_settings: std::cell::RefCell<Option<state::MapRenderSettings>>,
    /// Состояние окна строения: выделения рынка, дабл-клик, лог сделки.
    pub building_ui: &'a mut state::BuildingUi,
    /// ПВП-лобби: ник, мок RoomManager, фильтры (Этапы 1-2).
    pub pvp: &'a mut state::PvpState,
    /// Редактор карт: проект, история, запечка (экран Menu::Editor).
    pub editor: &'a mut state::EditorUi,
    /// egui-слой редактора: экран рисует панели через ctx.egui.run(...).
    pub egui: &'a mut crate::egui_layer::EguiLayer,
    /// Радиальная градиент-текстура свечения (белая, альфа-фейд к краю).
    pub glow_tex: TexId,
}

struct App {
    window: Option<Arc<Window>>,
    gfx: Option<Gfx>,
    text: Option<TextRenderer>,
    state: Option<State>,
    input: InputState,
    skins: Option<Skins>,
    widgets: WidgetMap,
    rts: Option<(Rt, Rt)>,
    last_frame: std::time::Instant,
    fps_frames: u32,
    fps_since: std::time::Instant,
    glow_tex: TexId,
    /// egui-слой (оболочка редактора): ввод+рендер поверх игровых пассов.
    egui: Option<crate::egui_layer::EguiLayer>,
}

impl App {
    fn frame(&mut self) {
        let gfx = self.gfx.as_mut().unwrap();
        let (w, h) = (gfx.config.width as f32, gfx.config.height as f32);
        gfx.viewport = (w, h);
        self.input.begin_frame([w, h]);
        let frame_dt = self.last_frame.elapsed().as_secs_f32();
        self.last_frame = std::time::Instant::now();
        self.fps_frames += 1;
        let fps_elapsed = self.fps_since.elapsed().as_secs_f32();
        if fps_elapsed >= 0.5 {
            FPS_X100.store(
                (self.fps_frames as f32 / fps_elapsed * 100.) as u32,
                std::sync::atomic::Ordering::Relaxed,
            );
            self.fps_frames = 0;
            self.fps_since = std::time::Instant::now();
        }
        gfx.begin_frame();

        let state = self.state.as_mut().unwrap();
        state.delta += frame_dt;
        state.ui.camera = Camera::from_display_rect(0., 1080., 1920., -1080.);
        let (rt0, rt1) = self.rts.as_ref().unwrap();

        // Split-borrow: поля State живут независимо.
        let State {
            assets,
            registry,
            game,
            ui,
            delta,
            tile_pixels,
            textures,
            building_ui,
            pvp,
            editor,
            ..
        } = state;
        if frame_dt > 0.1 {
            println!("frame gap {:.2}s, fullscreen={:?}", frame_dt,
                self.window.as_ref().unwrap().fullscreen().is_some());
        }
        let mut ctx = Ctx {
            gfx,
            text: self.text.as_mut().unwrap(),
            input: &self.input,
            skins: self.skins.as_ref().unwrap(),
            widgets: &mut self.widgets,
            ui,
            game,
            registry,
            assets,
            tile_pixels,
            textures,
            rts: self.rts.as_ref().unwrap(),
            window: self.window.as_ref().unwrap(),
            building_ui,
            pvp,
            editor,
            delta,
            rt: &state.rt,
            map_settings: std::cell::RefCell::new(None),
            glow_tex: self.glow_tex,
            egui: self.egui.as_mut().expect("egui layer"),
        };
        let t0 = std::time::Instant::now();
        dispatch(&mut ctx);
        let t1 = std::time::Instant::now();
        drop(ctx);
        // egui: тесселяция/буферы ПОСЛЕ игровых пассов: редактор читает RT
        // (rt_as_texture) в этом же кадре — порядок wgpu: write → read.
        let egui_had_frame = self
            .egui
            .as_ref()
            .is_some_and(|egui| egui.has_output());
        {
            let gfx = self.gfx.as_mut().unwrap();
            gfx.egui_pending = egui_had_frame;
        }
        self.input.end_frame();
        self.gfx.as_mut().unwrap().end_frame();
        if let (Some(egui), Some(gfx)) = (self.egui.as_mut(), self.gfx.as_mut()) {
            let mut encoder = gfx
                .device
                .create_command_encoder(&Default::default());
            egui.finish(&gfx.device, &gfx.queue, &mut encoder);
            gfx.queue.submit([encoder.finish()]);
        }
        // egui: рендер поверх кадра (present внутри render_egui).
        if egui_had_frame {
            let egui = self.egui.as_mut().unwrap();
            let gfx = self.gfx.as_mut().unwrap();
            gfx.render_egui(|rpass| egui.render(rpass));
            egui.after_render();
        }
        // Discord Rich Presence: статус по активному экрану (не в Ctx, чтобы
        // экраны не трогали IPC; идемпотентно — внутри дедуп по стейту).
        self.update_presence();
        let t2 = std::time::Instant::now();
        if t2.elapsed().as_secs_f32() > 0.1 {
            println!(
                "slow frame: dispatch {:.3}s end_frame {:.3}s, wakes {}, redraws {}",
                t1.duration_since(t0).as_secs_f32(),
                t2.duration_since(t1).as_secs_f32(),
                WAKES.load(std::sync::atomic::Ordering::Relaxed),
                REDRAWS.load(std::sync::atomic::Ordering::Relaxed)
            );
        }
        // Непрерывный игровой цикл: перезапрос из конца кадра. На X11
        // request_redraw только из about_to_wait давал ~0.75 FPS.
        if let Some(w) = &self.window {
            w.request_redraw();
        }
    }

    /// Discord Rich Presence: собрать PresenceState по текущему экрану.
    /// Бой на карте: Menu::Battle → «сражается» + имя армии врага;
    /// PvE-бой из главного меню и сетапы → тоже карта, но без врага;
    /// Main/Atlas/Info → главное меню.
    fn update_presence(&mut self) {
        let Some(state) = self.state.as_mut() else {
            return;
        };
        let scenario = state.game.executor.gamemap.start.name.clone();
        // Золото игрока: сингл — армия executor-игрока (players[0]),
        // онлайн — назначенная сервером.
        let gold_army = match &state.game.variant {
            state::GameVariant::Online(online) => online.army,
            state::GameVariant::Single(_) => state
                .game
                .executor
                .players
                .first()
                .map(|player| player.army)
                .unwrap_or(0),
        };
        let gold = state
            .game
            .executor
            .gamemap
            .armys
            .get(gold_army)
            .map(|army| army.stats.gold)
            .unwrap_or(0);
        // Имя врага: другая армия активного боя. «Наша» армия в бою — как в
        // screens::battle(): сингл — армия 0, онлайн — online.army.
        let my_battle_army = match &state.game.variant {
            state::GameVariant::Online(online) => online.army,
            state::GameVariant::Single(_) => 0,
        };
        let enemy = state.game.executor.battle.as_ref().and_then(|battle| {
            let other = if battle.army1 == my_battle_army {
                battle.army2
            } else if battle.army2 == my_battle_army {
                battle.army1
            } else {
                return None;
            };
            state
                .game
                .executor
                .gamemap
                .armys
                .get(other)
                .map(|army| army.stats.army_name.clone())
        });
        let recent_wins = state.game.recent_wins;
        let presence = match &state.ui.main {
            Menu::Battle => rich_presence::PresenceState::Map {
                scenario,
                gold,
                in_battle: true,
                enemy,
                recent_wins,
            },
            Menu::Map(_) | Menu::Building(..) | Menu::Message(_) | Menu::RoomCreation
            | Menu::BattleSetup | Menu::PvpLobby | Menu::PvpRoomSetup | Menu::PvpRoom => {
                rich_presence::PresenceState::Map {
                    scenario,
                    gold,
                    in_battle: false,
                    enemy,
                    recent_wins,
                }
            }
            Menu::Editor => rich_presence::PresenceState::Map {
                scenario,
                gold,
                in_battle: false,
                enemy,
                recent_wins,
            },
            Menu::Main | Menu::Atlas | Menu::Info => rich_presence::PresenceState::Menu,
        };
        state.rpc.update(&presence);
    }
}

fn dispatch(ctx: &mut Ctx) {
    match &mut ctx.ui.main {
        Menu::Main => screens::main_menu(ctx),
        Menu::Atlas => screens::atlas(ctx),
        Menu::PvpLobby => screens::pvp_lobby(ctx),
        Menu::PvpRoomSetup => screens::pvp_room_setup(ctx),
        Menu::PvpRoom => screens::pvp_room(ctx),
        Menu::Info => screens::info(ctx),
        Menu::Message(_) => screens::message(ctx),
        Menu::RoomCreation => screens::room_creation(ctx),
        Menu::BattleSetup => screens::battle_setup(ctx),
        Menu::Battle => screens::battle(ctx),
        Menu::Map(_) => map_view::map_screen(ctx),
        Menu::Editor => {
            if !editor_view::editor_screen(ctx) {
                ctx.ui.main = Menu::Main;
            }
        }
        Menu::Building(..) => {
            if building_view::building_screen(ctx) {
                // Выход из окна: восстановить карту с сохранённой камерой.
                let settings = ctx.map_settings.borrow_mut().take().unwrap_or_default();
                ctx.ui.main = Menu::Map(settings);
            }
        }
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }
        let attrs = Window::default_attributes()
            .with_title("DT REMASTERED")
            .with_resizable(false)
            .with_fullscreen(Some(Fullscreen::Borderless(event_loop.primary_monitor())))
            .with_window_icon(window_icon());
        let window = Arc::new(
            event_loop
                .create_window(attrs)
                .expect("window creation failed"),
        );
        let mut gfx = Gfx::new(&window);
        let mut text = TextRenderer::new(&mut gfx);
        // game_init — async (FileAccess); блокирующе через временный runtime.
        let init_rt = tokio::runtime::Runtime::new().unwrap();
        let state = init_rt.block_on(state::game_init(&mut gfx, &mut text));
        let size = state.game.executor.gamemap.tilemap.size as u32;
        let rt0 = gfx.create_rt(
            state::SIZE.0 as u32 * size,
            state::SIZE.1 as u32 * size,
        );
        let rt1 = gfx.create_rt(
            state::SIZE.0 as u32 * size,
            state::SIZE.1 as u32 * size,
        );
        // Запечка карты и слоя декораций (порт prepare_textures из main()).
        let textures = bake::prepare_textures(
            &mut gfx,
            (&rt0, &rt1),
            &state.assets,
            &state.game.executor.gamemap,
            &state.registry,
            &state.tile_pixels,
            true,
        );
        let mut state = state;
        state.textures = state::RenderTextures {
            map: textures.0,
            decos: textures.1,
        };
        let glow_tex = gfx.push_glow_ring(192);
        // Сабмит запечённых пассов (экрана в этом кадре нет — present no-op).
        gfx.end_frame();
        println!("init: bake submitted, rts ready");
        // Проверка градиента — после сабмита, иначе читаем пустой RT.
        bake::debug_check_gradient(&gfx, &rt0, &state.game, &state.tile_pixels);
        let skins = Skins::build(&mut gfx, &state.assets);
        self.glow_tex = glow_tex;
        // egui-слой (оболочка редактора) на тех же device/queue и формате
        // экрана; ввод конвертируется вручную (см. egui_layer).
        self.egui = Some(crate::egui_layer::EguiLayer::new(
            &gfx.device,
            gfx.config.format,
        ));
        self.rts = Some((rt0, rt1));
        self.skins = Some(skins);
        self.gfx = Some(gfx);
        self.text = Some(text);
        self.state = Some(state);
        // DT_EGUI_DEBUG=1: пропустить игру — первый экран сразу редактор
        // (диагностика связки egui↔wgpu в изоляции от экранов игры).
        if std::env::var("DT_EGUI_DEBUG").is_ok() {
            self.state.as_mut().unwrap().ui.main = state::Menu::Editor;
            eprintln!("DT_EGUI_DEBUG: старт с Menu::Editor (без игровых экранов)");
        }
        self.window = Some(window);
        self.last_frame = std::time::Instant::now();
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, window_id: winit::window::WindowId, event: WindowEvent) {
        let Some(window) = &self.window else {
            return;
        };
        if window_id != window.id() {
            return;
        }
        use winit::keyboard::PhysicalKey;
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => {
                self.gfx.as_mut().unwrap().resize(size.width, size.height);
            }
            WindowEvent::KeyboardInput { event, .. } => {
                let code = match event.physical_key {
                    PhysicalKey::Code(c) => c,
                    PhysicalKey::Unidentified(_) => return,
                };
                match event.state {
                    ElementState::Pressed => {
                        if !self.input.key_down(code) {
                            self.input.pressed.push(code);
                        }
                        self.input.keys_down.insert(code);
                        if let Some(text) = &event.text {
                            for ch in text.chars() {
                                self.input.chars.push(ch);
                            }
                        }
                    }
                    ElementState::Released => {
                        self.input.released.push(code);
                        self.input.keys_down.remove(&code);
                    }
                }
            }
            WindowEvent::CursorMoved { position, .. } => {
                self.input.mouse_delta[0] += position.x as f32 - self.input.mouse[0];
                self.input.mouse_delta[1] += position.y as f32 - self.input.mouse[1];
                self.input.mouse = [position.x as f32, position.y as f32];
                if std::env::var("DT_EGUI_DEBUG").is_ok() {
                    eprintln!("[winit] move: {:.0},{:.0}", position.x, position.y);
                }
            }
            WindowEvent::MouseInput { state, button, .. } => {
                let idx = match button {
                    winit::event::MouseButton::Left => 0,
                    winit::event::MouseButton::Right => 1,
                    winit::event::MouseButton::Middle => 2,
                    _ => 3,
                };
                if std::env::var("DT_EGUI_DEBUG").is_ok() {
                    let down = state == ElementState::Pressed;
                    eprintln!("[winit] btn{idx} down={down}");
                }
                if idx < 3 {
                    match state {
                        ElementState::Pressed => self.input.mouse_down[idx] = true,
                        ElementState::Released => {
                            self.input.mouse_down[idx] = false;
                            self.input.mouse_released[idx] = true;
                        }
                    }
                }
            }
            WindowEvent::MouseWheel { delta, .. } => {
                if std::env::var("DT_EGUI_DEBUG").is_ok() {
                    eprintln!("[winit] wheel: {delta:?}");
                }
                match delta {
                    winit::event::MouseScrollDelta::LineDelta(_, y) => self.input.wheel += y * 10.,
                    winit::event::MouseScrollDelta::PixelDelta(pos) => self.input.wheel += pos.y as f32,
                }
            }
            WindowEvent::RedrawRequested => {
                REDRAWS.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                self.frame();
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        if let Some(window) = &self.window {
            window.request_redraw();
        }
        WAKES.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    }
}

/// Иконка окна из dt/assets/icon-*.png (порт set(), quad_ui main.rs:426).
fn window_icon() -> Option<winit::window::Icon> {
    let bytes = files::read_file_bytes("assets/icon-64.png");
    let img = image::load_from_memory(&bytes).ok()?.to_rgba8();
    let (w, h) = img.dimensions();
    winit::window::Icon::from_rgba(img.into_raw(), w, h).ok()
}

pub fn run() {
    let mut app = App {
        window: None,
        gfx: None,
        text: None,
        state: None,
        input: InputState::new(),
        skins: None,
        widgets: ahash::HashMap::default(),
        rts: None,
        last_frame: std::time::Instant::now(),
        fps_frames: 0,
        fps_since: std::time::Instant::now(),
        glow_tex: gfx::TexId(0),
        egui: None,
    };
    let event_loop = EventLoop::new().unwrap();
    event_loop.run_app(&mut app).unwrap();
}

#[cfg(not(target_os = "android"))]
#[allow(dead_code)]
fn main() {
    run();
}

// Android: NativeActivity через android-activity (native-activity feature).
#[cfg(target_os = "android")]
#[no_mangle]
fn android_main(app: android_activity::AndroidApp) {
    use winit::platform::android::EventLoopBuilderExtAndroid;
    let _ = files::ANDROID_APP.set(app.clone());
    let event_loop = winit::event_loop::EventLoop::builder()
        .with_android_app(app)
        .build()
        .unwrap();
    let mut app_state = App {
        window: None,
        gfx: None,
        text: None,
        state: None,
        input: InputState::new(),
        skins: None,
        widgets: ahash::HashMap::default(),
        rts: None,
        last_frame: std::time::Instant::now(),
        fps_frames: 0,
        fps_since: std::time::Instant::now(),
        glow_tex: gfx::TexId(0),
        egui: None,
    };
    event_loop.run_app(&mut app_state).unwrap();
}
