// Immediate-mode UI: замена macroquad root_ui (Skin/Button/Window/Group/Texture/
// editbox) + InputState (замена is_key_*/mouse_*). egui сознательно не используется.
use crate::gfx::{colors, Gfx, TexId};
use crate::text::{FontId, TextRenderer};
use ahash::RandomState;
use std::collections::HashSet;
use winit::keyboard::KeyCode;

// ---------------- Input ----------------

/// Состояние ввода за кадр. Заполняется из winit-событий в lib.rs.
pub struct InputState {
    pub keys_down: HashSet<KeyCode, RandomState>,
    pub pressed: Vec<KeyCode>,
    pub released: Vec<KeyCode>,
    pub chars: Vec<char>,
    /// Физические пиксели окна (UI-пространство = экранные пиксели, Y-вниз).
    pub mouse: [f32; 2],
    pub mouse_down: [bool; 3],
    pub mouse_released: [bool; 3],
    pub mouse_delta: [f32; 2],
    pub wheel: f32,
    pub close_requested: bool,
    pub screen: [f32; 2],
}

impl InputState {
    pub fn new() -> InputState {
        InputState {
            keys_down: HashSet::with_hasher(RandomState::new()),
            pressed: Vec::new(),
            released: Vec::new(),
            chars: Vec::new(),
            mouse: [0., 0.],
            mouse_down: [false; 3],
            mouse_released: [false; 3],
            mouse_delta: [0., 0.],
            wheel: 0.,
            close_requested: false,
            screen: [1920., 1080.],
        }
    }
    /// Вызывается в НАЧАЛЕ кадра: событийные флаги (pressed/released/chars)
    /// накоплены с прошлого кадра и будут прочитаны этим кадром; чистятся
    /// только в end_frame после dispatch, иначе кнопки мертвы.
    pub fn begin_frame(&mut self, screen: [f32; 2]) {
        self.screen = screen;
    }
    /// Вызывается в КОНЦЕ кадра: сброс одноразовых флагов ввода.
    pub fn end_frame(&mut self) {
        self.pressed.clear();
        self.released.clear();
        self.chars.clear();
        self.mouse_released = [false; 3];
        self.mouse_delta = [0., 0.];
        self.wheel = 0.;
    }
    /// Порт is_key_down.
    pub fn key_down(&self, code: KeyCode) -> bool {
        self.keys_down.contains(&code)
    }
    /// Порт is_key_pressed.
    pub fn key_pressed(&self, code: KeyCode) -> bool {
        self.pressed.contains(&code)
    }
    /// Порт is_key_released.
    pub fn key_released(&self, code: KeyCode) -> bool {
        self.released.contains(&code)
    }
    pub fn mouse_button_down(&self, b: usize) -> bool {
        self.mouse_down[b]
    }
    pub fn mouse_button_released(&self, b: usize) -> bool {
        self.mouse_released[b]
    }
    /// Порт screen_size().
    pub fn screen_size(&self) -> [f32; 2] {
        self.screen
    }
    pub fn screen_width(&self) -> f32 {
        self.screen[0]
    }
    pub fn screen_height(&self) -> f32 {
        self.screen[1]
    }
    /// Порт mouse_position().
    pub fn mouse_position(&self) -> [f32; 2] {
        self.mouse
    }
}

// ---------------- Скины ----------------

#[derive(Clone)]
pub struct Skin {
    pub font: FontId,
    pub font_size: u16,
    pub text_color: [f32; 4],
    pub text_hovered: [f32; 4],
    pub button_tex: TexId,
    pub window_tex: TexId,
}

// ---------------- Значения виджетов ----------------

#[derive(Clone)]
pub enum Val {
    Bool(bool),
    Usize(usize),
    Str(String),
}

/// Порт macroquad hash! — детерминированный между кадрами.
pub fn hash(s: &str) -> u64 {
    let mut h: u64 = 0xcbf29ce484222325;
    for b in s.bytes() {
        h ^= b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    h
}

pub type WidgetState = ahash::HashMap<u64, Val>;

#[derive(Clone, Copy)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl Rect {
    pub fn new(x: f32, y: f32, w: f32, h: f32) -> Rect {
        Rect { x, y, w, h }
    }
    pub fn contains(&self, p: [f32; 2]) -> bool {
        p[0] >= self.x && p[0] < self.x + self.w && p[1] >= self.y && p[1] < self.y + self.h
    }
}

const FOCUS_KEY: &str = "\u{1}edit_focus";

/// Контекст одного экрана: рисует и опрашивает ввод. Позиции — экранные пиксели.
pub const UI_W: f32 = 1920.;
pub const UI_H: f32 = 1080.;

pub struct UiCtx<'a> {
    pub gfx: &'a mut Gfx,
    pub text: &'a mut TextRenderer,
    pub input: &'a InputState,
    pub skin: &'a Skin,
    pub state: &'a mut WidgetState,
    /// viewport / (1920, 1080): экран рисуется в мировых координатах
    /// дисплея-камеры, мышь и скиссор приходят в физических пикселях.
    pub scale: [f32; 2],
    /// Курсор авто-layout (порт макроквадовского стека виджетов):
    /// виджеты с pos=None кладутся сюда и сдвигают курсор вниз.
    pub cursor: [f32; 2],
    /// Начало текущего окна (порт Window::ui: внутренние координаты виджетов
    /// относительны окна, как в macroquad).
    pub origin: [f32; 2],
}

impl<'a> UiCtx<'a> {
    pub fn new(
        gfx: &'a mut Gfx,
        text: &'a mut TextRenderer,
        input: &'a InputState,
        skin: &'a Skin,
        state: &'a mut WidgetState,
    ) -> UiCtx<'a> {
        let s = input.screen_size();
        let scale = [s[0] / UI_W, s[1] / UI_H];
        UiCtx { gfx, text, input, skin, state, scale, cursor: [0., 0.], origin: [0., 0.] }
    }

    /// Мышь в мировых координатах UI (порт Camera2D::screen_to_world
    /// для display rect (0, 1080, 1920, -1080)).
    pub fn mouse(&self) -> [f32; 2] {
        [self.input.mouse[0] / self.scale[0], self.input.mouse[1] / self.scale[1]]
    }

    fn get_val(&mut self, id: &str) -> Val {
        let h = hash(id);
        self.state.entry(h).or_insert(Val::Usize(0)).clone()
    }

    /// Порт ui.get_bool.
    pub fn get_bool(&mut self, id: &str) -> bool {
        matches!(self.get_val(id), Val::Bool(true))
    }
    pub fn set_bool(&mut self, id: &str, v: bool) {
        self.state.insert(hash(id), Val::Bool(v));
    }
    /// Порт ui.get_any::<usize>.
    pub fn get_usize(&mut self, id: &str) -> usize {
        match self.get_val(id) {
            Val::Usize(v) => v,
            _ => 0,
        }
    }
    pub fn set_usize(&mut self, id: &str, v: usize) {
        self.state.insert(hash(id), Val::Usize(v));
    }

    /// Порт is_hovered/is_clicked в UI-координатах (мышь -> мир).
    pub fn is_hovered(&self, r: Rect) -> bool {
        r.contains(self.mouse())
    }
    pub fn is_clicked(&self, r: Rect) -> bool {
        self.input.mouse_button_released(0) && self.is_hovered(r)
    }

    /// Порт ui.label(Some(pos)|None, text): None — авто-layout вниз.
    pub fn label(&mut self, pos: Option<[f32; 2]>, text: &str) {
        let pos = pos.unwrap_or(self.cursor);
        let [x, y] = [pos[0] + self.origin[0], pos[1] + self.origin[1]];
        self.cursor[1] += self.skin.font_size as f32 * 1.4;
        self.text.draw_text(
            self.gfx,
            text,
            x,
            y + self.skin.font_size as f32,
            self.skin.font,
            self.skin.font_size,
            1.,
            self.skin.text_color,
        );
    }

    pub fn label_colored(&mut self, pos: [f32; 2], text: &str, color: [f32; 4]) {
        self.text.draw_text(
            self.gfx,
            text,
            pos[0],
            pos[1] + self.skin.font_size as f32,
            self.skin.font,
            self.skin.font_size,
            1.,
            color,
        );
    }

    fn text_width(&mut self, text: &str) -> f32 {
        self.text
            .measure(self.gfx, text, self.skin.font, self.skin.font_size, 1.)
            .width
    }

    /// Порт ui.button(Some(pos)|None, text): None — авто-layout вниз
    /// (в macroquad виджет встаёт по внутреннему курсору).
    pub fn button(&mut self, pos: Option<[f32; 2]>, text: &str) -> bool {
        let [cx, cy] = pos.unwrap_or(self.cursor);
        let [x, y] = [cx + self.origin[0], cy + self.origin[1]];
        let w = (self.text_width(text) + 20.).max(60.);
        let h = self.skin.font_size as f32 + 14.;
        self.cursor[1] += h + 8.;
        self.button_rect(x, y, w, h, text)
    }

    /// Порт Button::new(text).position(pos).size(size).ui(ui).
    pub fn button_sized(&mut self, pos: [f32; 2], size: [f32; 2], text: &str) -> bool {
        self.button_rect(pos[0], pos[1], size[0], size[1], text)
    }

    fn button_rect(&mut self, x: f32, y: f32, w: f32, h: f32, text: &str) -> bool {
        let r = Rect::new(x, y, w, h);
        let hovered = self.is_hovered(r);
        self.gfx
            .draw_texture(self.skin.button_tex, x, y, w, h, colors::WHITE);
        let color = if hovered {
            self.skin.text_hovered
        } else {
            self.skin.text_color
        };
        let tw = self.text_width(text);
        self.text.draw_text(
            self.gfx,
            text,
            x + (w - tw) / 2.,
            y + h * 0.5 + self.skin.font_size as f32 * 0.35,
            self.skin.font,
            self.skin.font_size,
            1.,
            color,
        );
        self.is_clicked(r)
    }

    /// Порт ui.texture(weak_clone, w, h) — кликабельная текстура.
    pub fn texture(&mut self, tex: TexId, x: f32, y: f32, w: f32, h: f32) -> bool {
        self.gfx.draw_texture(tex, x, y, w, h, colors::WHITE);
        self.is_clicked(Rect::new(x, y, w, h))
    }

    /// Виджет текстуры на авто-layout курсоре (порт ui.texture при None-позиции).
    pub fn texture_cursor(&mut self, tex: TexId, w: f32, h: f32) -> bool {
        let (x, y) = (self.cursor[0], self.cursor[1]);
        self.cursor[1] += h + 8.;
        self.texture(tex, x, y, w, h)
    }

    /// Гарантировать высоту курсора >= y (для переключения вкладок без сброса).
    pub fn set_cursor_y(&mut self, y: f32) {
        self.cursor[1] = y;
    }

    /// Порт Texture::new(...).size(w, h).ui(ui) — некликабельная по умолчанию,
    /// но вызовы в Info/BattleSetup проверяют результат => кликабельная.
    pub fn texture_widget(&mut self, tex: TexId, x: f32, y: f32, w: f32, h: f32) -> bool {
        self.texture(tex, x, y, w, h)
    }

    /// Порт ui.input_text(hash!(), label, &mut string). Фокус хранится в state.
    pub fn input_text(&mut self, id: &str, label: &str, buf: &mut String) {
        let idh = hash(id);
        let focus_key = hash(FOCUS_KEY);
        let focused = matches!(
            self.state.get(&focus_key),
            Some(Val::Usize(v)) if *v == idh as usize
        );
        let (x, y) = (
            self.cursor[0] + self.origin[0],
            self.cursor[1] + self.origin[1],
        );
        let w = 400.;
        let h = self.skin.font_size as f32 + 14.;
        let r = Rect::new(x, y, w, h);
        let hovered = self.is_hovered(r);
        self.gfx
            .draw_texture(self.skin.button_tex, x, y, w, h, colors::WHITE);
        self.label(Some([x + 8., y + 4.]), label);
        self.cursor[1] += h + 8.;
        let label_w = self.text_width(label);
        let mut shown = buf.clone();
        if focused {
            if self.input.key_pressed(KeyCode::Backspace) {
                shown.pop();
            }
            for ch in &self.input.chars {
                if !ch.is_control() {
                    shown.push(*ch);
                }
            }
        }
        let caret = if focused && (crate::time_secs() * 2.) as usize % 2 == 0 {
            "_"
        } else {
            ""
        };
        self.label(
            Some([x + 8. + label_w + 12., y + 4.]),
            &format!("{shown}{caret}"),
        );
        if hovered && self.input.mouse_button_released(0) {
            self.state.insert(focus_key, Val::Usize(idh as usize));
        }
        if focused {
            *buf = shown;
        }
    }

    /// Дропдаун (переиспользуемая абстракция): закрытый виджет closed_tex
    /// размером closed_size; клик открывает скролл-список строк (текстура 48x48
    /// + подпись); клик по строке вызывает pick и закрывает; клик мимо закрывает.
    /// Возвращает true, если в этом кадре сделан выбор.
    #[allow(clippy::too_many_arguments)]
    pub fn dropdown(
        &mut self,
        id: &str,
        closed_tex: TexId,
        closed_size: [f32; 2],
        row_h: f32,
        rows: &[(TexId, String)],
        max_visible: usize,
        pick: impl FnOnce(&mut Self, usize),
    ) -> bool {
        let open_key = hash(&format!("\u{1}dd_open_{id}"));
        let open = matches!(self.state.get(&open_key), Some(Val::Bool(true)));
        let bx = self.cursor[0] + self.origin[0];
        let by = self.cursor[1] + self.origin[1];
        self.cursor[1] += closed_size[1] + 8.;
        let w = closed_size[0];
        let mut picked = false;
        if !open {
            self.gfx
                .draw_texture(closed_tex, bx, by, closed_size[0], closed_size[1], colors::WHITE);
            if self.is_clicked(Rect::new(bx, by, closed_size[0], closed_size[1])) {
                self.state.insert(open_key, Val::Bool(true));
            }
            return false;
        }
        let mut chosen: Option<usize> = None;
        let h = (max_visible as f32 * row_h).max(row_h);
        self.scroll_group(&format!("{id}_list"), bx, by + closed_size[1], w, h, |ui, pos| {
            for (i, (tex, name)) in rows.iter().enumerate() {
                let ry = pos[1] + i as f32 * row_h;
                ui.gfx.draw_texture(*tex, pos[0], ry, 48., 48., colors::WHITE);
                // Прямой draw_text: bx/by уже абсолютные (origin учтён выше).
                ui.text.draw_text(
                    ui.gfx,
                    name,
                    pos[0] + 56.,
                    ry + 34.,
                    ui.skin.font,
                    ui.skin.font_size,
                    1.,
                    ui.skin.text_color,
                );
                if ui.is_clicked(Rect::new(pos[0], ry, w, row_h)) {
                    chosen = Some(i);
                }
            }
        });
        if let Some(i) = chosen {
            pick(self, i);
            self.state.insert(open_key, Val::Bool(false));
            picked = true;
        } else if self.input.mouse_button_released(0) {
            // Клик мимо списка — закрыть.
            self.state.insert(open_key, Val::Bool(false));
        }
        picked
    }

    /// Порт Window::new(...).titlebar(false).ui(...) — окно с фоновой текстурой,
    /// содержимое клипится скиссором.
    /// Скроллируемая группа (замена встроенного скролла Group в macroquad):
    /// содержимое рисуется со смещением -offset, колесо внутри области
    /// двигает offset. offset живёт в widget state по id.
    pub fn scroll_group(
        &mut self,
        id: &str,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        f: impl FnOnce(&mut Self, [f32; 2]),
    ) {
        let key = hash(&format!("\u{1}scroll_{id}"));
        let mut offset = match self.state.get(&key) {
            Some(Val::Usize(v)) => *v as f32,
            _ => 0.,
        };
        let wheel = self.input.wheel;
        if wheel != 0. && Rect::new(x, y, w, h).contains(self.mouse()) {
            offset = (offset - wheel * 40.).max(0.);
        }
        self.state.insert(key, Val::Usize(offset as usize));
        let ox = x;
        let oy = y - offset;
        self.gfx.set_scissor(
            (x * self.scale[0]) as i32,
            (y * self.scale[1]) as i32,
            (w * self.scale[0]) as u32,
            (h * self.scale[1]) as u32,
        );
        f(self, [ox, oy]);
        self.gfx.set_scissor(0, 0, u32::MAX, u32::MAX);
    }

    pub fn window(&mut self, x: f32, y: f32, w: f32, h: f32, f: impl FnOnce(&mut Self)) {
        self.gfx
            .draw_texture(self.skin.window_tex, x, y, w, h, colors::WHITE);
        // Содержимое окна — в координатах относительно окна (как в macroquad).
        let prev_origin = self.origin;
        self.origin = [x, y];
        self.cursor = [0., 0.];
        // Скиссор — в физических пикселях цели: мир * scale.
        let sx = (x * self.scale[0]) as i32;
        let sy = (y * self.scale[1]) as i32;
        self.gfx.set_scissor(
            sx,
            sy,
            (w * self.scale[0]) as u32,
            (h * self.scale[1]) as u32,
        );
        f(self);
        self.origin = prev_origin;
        self.gfx
            .set_scissor(0, 0, u32::MAX, u32::MAX);
    }
}

/// Вертикальная группа (порт Group::new(...).layout(Vertical)): курсор сам
/// сдвигается вниз после каждого виджета.
pub struct GroupCtx<'a, 'b> {
    pub x: f32,
    pub y: f32,
    ctx: &'a mut UiCtx<'b>,
}

impl<'a> UiCtx<'a> {
    pub fn group_vertical(&mut self, x: f32, y: f32, f: impl FnOnce(&mut GroupCtx)) {
        let mut g = GroupCtx { x, y, ctx: self };
        f(&mut g);
    }
}

impl GroupCtx<'_, '_> {
    fn advance(&mut self, dy: f32) {
        self.y += dy;
    }
    pub fn label(&mut self, text: &str) {
        self.ctx.label(Some([self.x, self.y]), text);
        let h = self.ctx.skin.font_size as f32 * 1.4;
        self.advance(h);
    }
    pub fn texture(&mut self, tex: TexId, w: f32, h: f32) -> bool {
        let clicked = self.ctx.texture(tex, self.x, self.y, w, h);
        self.advance(h + 4.);
        clicked
    }
    pub fn button(&mut self, text: &str) -> bool {
        let clicked = self.ctx.button(Some([self.x, self.y]), text);
        self.advance(self.ctx.skin.font_size as f32 + 18.);
        clicked
    }
}
