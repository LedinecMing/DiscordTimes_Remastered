// wgpu_ui — порт quad_ui на wgpu + winit. Точка входа, ApplicationHandler,
// кадр: input -> пассы по меню -> present. egui сознательно отсутствует.
pub mod assets;
pub mod bake;
pub mod battle_view;
pub mod building_view;
pub mod camera;
pub mod files;
pub mod gfx;
pub mod map_view;
pub mod screens;
pub mod state;
pub mod text;
pub mod ui;

use camera::Camera;
use gfx::{colors, Gfx, Rt, TexId};
use state::{Game, Menu, State};
use std::sync::{Arc, LazyLock};
use text::TextRenderer;
use ui::{InputState, Skin};
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

/// Два скина (порт main_skin/dop_skin, quad_ui main.rs:2164-2230).
pub struct Skins {
    pub main: Skin,
    pub dop: Skin,
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
        Skins {
            main: Skin {
                font,
                font_size: 32,
                text_color: colors::WHITE,
                text_hovered: colors::DARKBLUE,
                button_tex: button,
                window_tex: menu,
            },
            dop: Skin {
                font,
                font_size: 32,
                text_color: colors::BLACK,
                text_hovered: colors::DARKBLUE,
                button_tex: button,
                window_tex: paper,
            },
        }
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
            delta,
            rt: &state.rt,
            map_settings: std::cell::RefCell::new(None),
            glow_tex: self.glow_tex,
        };
        let t0 = std::time::Instant::now();
        dispatch(&mut ctx);
        let t1 = std::time::Instant::now();
        drop(ctx);
        self.input.end_frame();
        self.gfx.as_mut().unwrap().end_frame();
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
}

fn dispatch(ctx: &mut Ctx) {
    match &mut ctx.ui.main {
        Menu::Main => screens::main_menu(ctx),
        Menu::Atlas => screens::atlas(ctx),
        Menu::Info => screens::info(ctx),
        Menu::Message(_) => screens::message(ctx),
        Menu::RoomCreation => screens::room_creation(ctx),
        Menu::BattleSetup => screens::battle_setup(ctx),
        Menu::Battle => screens::battle(ctx),
        Menu::Map(_) => map_view::map_screen(ctx),
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
            &state.game,
            &state.registry,
            &state.tile_pixels,
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
        self.rts = Some((rt0, rt1));
        self.skins = Some(skins);
        self.gfx = Some(gfx);
        self.text = Some(text);
        self.state = Some(state);
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
                // Накопление за кадр (порт mouse_delta_position): несколько
                // событий CursorMoved между кадрами суммируются.
                self.input.mouse_delta[0] += position.x as f32 - self.input.mouse[0];
                self.input.mouse_delta[1] += position.y as f32 - self.input.mouse[1];
                self.input.mouse = [position.x as f32, position.y as f32];
            }
            WindowEvent::MouseInput { state, button, .. } => {
                let idx = match button {
                    winit::event::MouseButton::Left => 0,
                    winit::event::MouseButton::Right => 1,
                    winit::event::MouseButton::Middle => 2,
                    _ => 3,
                };
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
            WindowEvent::MouseWheel { delta, .. } => match delta {
                winit::event::MouseScrollDelta::LineDelta(_, y) => self.input.wheel += y * 10.,
                winit::event::MouseScrollDelta::PixelDelta(pos) => self.input.wheel += pos.y as f32,
            },
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
    };
    event_loop.run_app(&mut app_state).unwrap();
}
