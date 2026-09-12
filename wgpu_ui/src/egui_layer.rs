//! egui-интеграция wgpu_ui: платформа ввода + рендер поверх игровых пассов.
//!
//! egui нужен ТОЛЬКО как UI-оболочка редактора (Menu::Editor): игровые
//! экраны продолжают рисовать родным ui.rs. egui-wgpu 0.36 работает на том
//! же device/queue (wgpu 30), что и игра; Renderer.render рисует в view
//! экрана ПОСЛЕ игровых пассов.
//!
//! Платформа своя, а не egui-winit: egui-wgpu собран с default-features=false
//! (без winit-фичи), ввод конвертируется из InputState вручную — полный
//! контроль и ноль конфликтов версий.

use crate::ui::InputState;
use egui::{Context, RawInput};

/// Полный цикл egui на кадр: begin_pass → UI-замыкание → end_pass → render.
pub struct EguiLayer {
    pub ctx: Context,
    renderer: egui_wgpu::Renderer,
    /// Модификаторы, накопленные на прошлых кадрах (Ctrl/Shift для хоткеев).
    modifiers: egui::Modifiers,
    /// Полный вывод run_ui текущего кадра.
    pending_output: Option<egui::FullOutput>,
    /// Прямоугольник экрана последнего кадра.
    last_screen_rect: egui::Rect,
    /// Результат end_frame, потребляемый render()/after_render().
    primitives: Vec<egui::ClippedPrimitive>,
    screen_descriptor: egui_wgpu::ScreenDescriptor,
    pending_free: Vec<egui::TextureId>,
}

impl EguiLayer {
    pub fn new(device: &wgpu::Device, output_format: wgpu::TextureFormat) -> Self {
        let ctx = Context::default();
        ctx.set_visuals(egui::Visuals::dark());
        let renderer = egui_wgpu::Renderer::new(
            device,
            output_format,
            egui_wgpu::RendererOptions::default(),
        );
        Self {
            ctx,
            renderer,
            modifiers: egui::Modifiers::default(),
            pending_output: None,
            last_screen_rect: egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1., 1.)),
            primitives: Vec::new(),
            screen_descriptor: egui_wgpu::ScreenDescriptor {
                size_in_pixels: [1, 1],
                pixels_per_point: 1.0,
            },
            pending_free: Vec::new(),
        }
    }

    /// Полный egui-кадр: run_ui(input, ui_fn) — панели рисуются внутри
    /// замыкания (в 0.36 панели требуют родительский &mut Ui из run_ui).
    /// После: finish() заливает текстуры/буферы; render() рисует в view.
    pub fn run(
        &mut self,
        input: &InputState,
        pixels_per_point: f32,
        ui_fn: impl FnMut(&mut egui::Ui),
    ) {
        let raw = self.build_input(input, pixels_per_point);
        self.pending_output = Some(self.ctx.run_ui(raw, ui_fn));
    }

    /// Был ли egui-кадр (run отработал, finish ещё нет)?
    pub fn has_output(&self) -> bool {
        self.pending_output.is_some()
    }

    /// Конвертация InputState → RawInput (вызывается из run()).
    fn build_input(&mut self, input: &InputState, pixels_per_point: f32) -> RawInput {
        let screen = input.screen;
        let mut events = Vec::new();
        // Текстовые символы (editbox'ы egui: имена карт и т.п.).
        events.extend(
            input
                .chars
                .iter()
                .map(|c| c.to_string())
                .map(egui::Event::Text),
        );
        // Модификаторы — состояние кнопок на этот кадр.
        let modifiers = egui::Modifiers {
            alt: input.key_down(winit::keyboard::KeyCode::AltLeft)
                || input.key_down(winit::keyboard::KeyCode::AltRight),
            ctrl: input.key_down(winit::keyboard::KeyCode::ControlLeft)
                || input.key_down(winit::keyboard::KeyCode::ControlRight),
            shift: input.key_down(winit::keyboard::KeyCode::ShiftLeft)
                || input.key_down(winit::keyboard::KeyCode::ShiftRight),
            mac_cmd: false,
            command: false,
        };
        // Клавиши: только именованные — egui-fокус в editbox'ах и шорткаты.
        for key in &input.pressed {
            if let Some(egui_key) = map_key(*key) {
                events.push(egui::Event::Key {
                    key: egui_key,
                    physical_key: None,
                    pressed: true,
                    repeat: false,
                    modifiers,
                });
            }
        }
        for key in &input.released {
            if let Some(egui_key) = map_key(*key) {
                events.push(egui::Event::Key {
                    key: egui_key,
                    physical_key: None,
                    pressed: false,
                    repeat: false,
                    modifiers,
                });
            }
        }
        events.push(egui::Event::ModifiersChanged(modifiers));
        // Мышь: движение + кнопки (egui сам различает drag/click по таймингу).
        let mouse_pos = egui::pos2(input.mouse[0], input.mouse[1]);
        events.push(egui::Event::PointerMoved(mouse_pos));
        events.push(egui::Event::PointerButton {
            pos: mouse_pos,
            button: egui::PointerButton::Primary,
            pressed: input.mouse_button_down(0),
            modifiers,
        });
        events.push(egui::Event::PointerButton {
            pos: mouse_pos,
            button: egui::PointerButton::Secondary,
            pressed: input.mouse_button_down(1),
            modifiers,
        });
        events.push(egui::Event::PointerButton {
            pos: mouse_pos,
            button: egui::PointerButton::Middle,
            pressed: input.mouse_button_down(2),
            modifiers,
        });
        // Колесо: egui-панели редактора скроллятся; игра читает тот же wheel
        // для зума карты — экраны не пересекаются.
        if input.wheel != 0. {
            events.push(egui::Event::MouseWheel {
                unit: egui::MouseWheelUnit::Point,
                delta: egui::vec2(0., input.wheel),
                modifiers,
                phase: egui::TouchPhase::Move,
            });
        }
        let mut viewports = egui::ViewportIdMap::default();
        let mut info = egui::ViewportInfo::default();
        info.native_pixels_per_point = Some(pixels_per_point);
        info.inner_rect = Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(screen[0], screen[1]),
        ));
        viewports.insert(egui::ViewportId::ROOT, info);
        let raw = RawInput {
            viewports,
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(screen[0], screen[1]),
            )),
            time: Some(crate::time_secs()),
            events,
            ..Default::default()
        };
        self.modifiers = modifiers;
        raw
    }

    /// Заливка текстур и буферов после run(): вызывает FRAME до render().
    pub fn finish(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
    ) {
        let Some(full_output) = self.pending_output.take() else {
            return;
        };
        let screen = self
            .ctx
            .input(|i: &egui::InputState| i.raw.screen_rect)
            .unwrap_or(self.last_screen_rect);
        self.last_screen_rect = screen;
        let screen_descriptor = egui_wgpu::ScreenDescriptor {
            size_in_pixels: [screen.width() as u32, screen.height() as u32],
            pixels_per_point: self.ctx.pixels_per_point(),
        };
        for (id, deltas) in &full_output.textures_delta.set {
            for image_delta in deltas {
                self.renderer.update_texture(device, queue, *id, image_delta);
            }
        }
        let primitives = self
            .ctx
            .tessellate(full_output.shapes, screen_descriptor.pixels_per_point);
        self.renderer
            .update_buffers(device, queue, encoder, &primitives, &screen_descriptor);
        // free-текстуры удаляются после render (см. after_render).
        self.pending_free = full_output.textures_delta.free.iter().copied().collect();
        self.primitives = primitives;
        self.screen_descriptor = screen_descriptor;
    }

    /// Отрисовать egui в текущий view экрана (после игровых пассов).
    pub fn render(&mut self, render_pass: &mut wgpu::RenderPass<'static>) {
        self.renderer
            .render(render_pass, &self.primitives, &self.screen_descriptor);
    }

    /// Освободить текстуры egui после render (вызывать в конце кадра).
    pub fn after_render(&mut self) {
        for id in self.pending_free.drain(..) {
            self.renderer.free_texture(&id);
        }
    }
}

/// winit KeyCode → egui Key (именованное подмножество: editbox + хоткеи).
fn map_key(code: winit::keyboard::KeyCode) -> Option<egui::Key> {
    use winit::keyboard::KeyCode as K;
    Some(match code {
        K::ArrowDown => egui::Key::ArrowDown,
        K::ArrowLeft => egui::Key::ArrowLeft,
        K::ArrowRight => egui::Key::ArrowRight,
        K::ArrowUp => egui::Key::ArrowUp,
        K::Escape => egui::Key::Escape,
        K::Tab => egui::Key::Tab,
        K::Backspace => egui::Key::Backspace,
        K::Enter | K::NumpadEnter => egui::Key::Enter,
        K::Space => egui::Key::Space,
        K::Insert => egui::Key::Insert,
        K::Delete => egui::Key::Delete,
        K::Home => egui::Key::Home,
        K::End => egui::Key::End,
        K::PageUp => egui::Key::PageUp,
        K::PageDown => egui::Key::PageDown,
        K::KeyA => egui::Key::A,
        K::KeyB => egui::Key::B,
        K::KeyC => egui::Key::C,
        K::KeyD => egui::Key::D,
        K::KeyE => egui::Key::E,
        K::KeyF => egui::Key::F,
        K::KeyG => egui::Key::G,
        K::KeyH => egui::Key::H,
        K::KeyI => egui::Key::I,
        K::KeyJ => egui::Key::J,
        K::KeyK => egui::Key::K,
        K::KeyL => egui::Key::L,
        K::KeyM => egui::Key::M,
        K::KeyN => egui::Key::N,
        K::KeyO => egui::Key::O,
        K::KeyP => egui::Key::P,
        K::KeyQ => egui::Key::Q,
        K::KeyR => egui::Key::R,
        K::KeyS => egui::Key::S,
        K::KeyT => egui::Key::T,
        K::KeyU => egui::Key::U,
        K::KeyV => egui::Key::V,
        K::KeyW => egui::Key::W,
        K::KeyX => egui::Key::X,
        K::KeyY => egui::Key::Y,
        K::KeyZ => egui::Key::Z,
        K::Digit0 => egui::Key::Num0,
        K::Digit1 => egui::Key::Num1,
        K::Digit2 => egui::Key::Num2,
        K::Digit3 => egui::Key::Num3,
        K::Digit4 => egui::Key::Num4,
        K::Digit5 => egui::Key::Num5,
        K::Digit6 => egui::Key::Num6,
        K::Digit7 => egui::Key::Num7,
        K::Digit8 => egui::Key::Num8,
        K::Digit9 => egui::Key::Num9,
        _ => return None,
    })
}
