// Текст на fontdue (тот же растеризатор, что внутри macroquad) + глифовый атлас.
// Замена load_ttf_font / measure_text / draw_text / draw_text_ex / draw_multiline.
use crate::gfx::{Filter, Gfx, TexId, WHITE};
use ahash::RandomState;
use std::collections::HashMap;

pub type FontId = usize;

const ATLAS: u32 = 2048;

#[derive(Clone, Copy)]
struct Glyph {
    /// 4 угла UV в атласе (порядок как у квада: tl, tr, br, bl).
    uv: [[f32; 2]; 4],
    w: f32,
    h: f32,
    /// Смещение от пера: xmin и верх глифа относительно baseline.
    xmin: f32,
    top: f32,
    advance: f32,
}

pub struct TextRenderer {
    atlas: TexId,
    atlas_w: u32,
    atlas_h: u32,
    cursor_x: u32,
    cursor_y: u32,
    row_h: u32,
    fonts: Vec<fontdue::Font>,
    glyphs: HashMap<(FontId, char, u16), Glyph, RandomState>,
}

/// Порт macroquad TextDimensions (используются width/height).
#[derive(Clone, Copy, Debug)]
pub struct TextDimensions {
    pub width: f32,
    pub height: f32,
    pub offset_y: f32,
}

impl TextRenderer {
    pub fn new(gfx: &mut Gfx) -> TextRenderer {
        let atlas = gfx.push_texture_rgba(&vec![0u8; (ATLAS * ATLAS * 4) as usize], ATLAS, ATLAS, Filter::Nearest);
        TextRenderer {
            atlas,
            atlas_w: ATLAS,
            atlas_h: ATLAS,
            cursor_x: 1,
            cursor_y: 1,
            row_h: 0,
            fonts: Vec::new(),
            glyphs: HashMap::with_hasher(RandomState::new()),
        }
    }

    /// Порт load_ttf_font.
    pub fn add_font(&mut self, bytes: &[u8]) -> FontId {
        let font = fontdue::Font::from_bytes(
            bytes,
            fontdue::FontSettings { collection_index: 0, scale: 40., load_substitutions: true },
        )
        .expect("font parse failed");
        self.fonts.push(font);
        self.fonts.len() - 1
    }

    fn ensure_glyph(&mut self, gfx: &mut Gfx, font: FontId, ch: char, px: u16) -> Glyph {
        if let Some(g) = self.glyphs.get(&(font, ch, px)) {
            return *g;
        }
        let (metrics, cov) = self.fonts[font].rasterize(ch, px as f32);
        let (gw, gh) = (metrics.width as u32, metrics.height as u32);
        // shelf packing; 1px отбивка от кровотечения семплера
        if self.cursor_x + gw + 1 > self.atlas_w {
            self.cursor_x = 1;
            self.cursor_y += self.row_h + 1;
            self.row_h = 0;
        }
        assert!(
            self.cursor_y + gh + 1 <= self.atlas_h,
            "glyph atlas overflow: increase ATLAS"
        );
        // атлас белый, альфа = coverage: цвет задаёт vertex color
        let mut pixels = vec![255u8; (gw * gh * 4) as usize];
        for (i, c) in cov.iter().enumerate() {
            pixels[i * 4 + 3] = *c;
        }
        if gw > 0 && gh > 0 {
            upload_sub(gfx, self.atlas, &pixels, gw, gh, self.cursor_x, self.cursor_y, self.atlas_w);
        }
        let (ax, ay) = (self.cursor_x as f32, self.cursor_y as f32);
        let glyph = Glyph {
            uv: [
                [ax / self.atlas_w as f32, ay / self.atlas_h as f32],
                [(ax + gw as f32) / self.atlas_w as f32, ay / self.atlas_h as f32],
                [(ax + gw as f32) / self.atlas_w as f32, (ay + gh as f32) / self.atlas_h as f32],
                [ax / self.atlas_w as f32, (ay + gh as f32) / self.atlas_h as f32],
            ],
            w: gw as f32,
            h: gh as f32,
            xmin: metrics.xmin as f32,
            // растеризатор: ymin — низ глифа над baseline (вверх), верх = ymin+height
            top: (metrics.ymin + metrics.height as i32) as f32,
            advance: metrics.advance_width,
        };
        self.cursor_x += gw + 1;
        self.row_h = self.row_h.max(gh);
        self.glyphs.insert((font, ch, px), glyph);
        glyph
    }

    /// Порт macroquad measure_text(text, font, font_size, scale).
    pub fn measure(
        &mut self,
        gfx: &mut Gfx,
        text: &str,
        font: FontId,
        font_size: u16,
        scale: f32,
    ) -> TextDimensions {
        let mut width = 0f32;
        let mut max_h = 0f32;
        for ch in text.chars() {
            let g = self.ensure_glyph(gfx, font, ch, font_size);
            width += g.advance * scale;
            max_h = max_h.max(g.h * scale);
        }
        TextDimensions {
            width,
            height: max_h.max(font_size as f32),
            offset_y: 0.,
        }
    }

    /// Порт draw_text/draw_text_ex: x — левый край, y — BASELINE (как в macroquad).
    #[allow(clippy::too_many_arguments)]
    pub fn draw_text(
        &mut self,
        gfx: &mut Gfx,
        text: &str,
        x: f32,
        y: f32,
        font: FontId,
        font_size: u16,
        scale: f32,
        color: [f32; 4],
    ) {
        let mut pen_x = x;
        for ch in text.chars() {
            if ch == '\n' {
                continue;
            }
            let g = self.ensure_glyph(gfx, font, ch, font_size);
            let left = pen_x + g.xmin * scale;
            let top = y - g.top * scale;
            let w = g.w * scale;
            let h = g.h * scale;
            let p = [
                [left, top],
                [left + w, top],
                [left + w, top + h],
                [left, top + h],
            ];
            gfx.draw_quad(self.atlas, p, g.uv, color);
            pen_x += g.advance * scale;
        }
    }

    /// Порт split_text из quad_ui (main.rs:607) — разбиение по словам с учётом
    /// ширины. Возвращает ((width, height), строка) по строкам.
    fn split_text(
        &mut self,
        gfx: &mut Gfx,
        text: &str,
        font: FontId,
        font_size: u16,
        max_width: f32,
    ) -> Vec<((f32, f32), String)> {
        let space_size = self.measure(gfx, " ", font, font_size, 1.).width;
        let mut out = Vec::new();
        for line in text.split('\n') {
            if line.is_empty() {
                out.push(((0., font_size as f32), String::new()));
                continue;
            }
            let mut lines: Vec<((f32, f32), String)> = Vec::new();
            let mut acc_width = 0f32;
            for word in line.split_whitespace() {
                let word_size = (
                    self.measure(gfx, word, font, font_size, 1.).width,
                    self.measure(gfx, word, font, font_size, 1.).height,
                );
                if acc_width + word_size.0 > max_width {
                    lines.push((
                        (word_size.0 + space_size, word_size.1),
                        format!("{word} "),
                    ));
                    acc_width = word_size.0 + space_size;
                } else if let Some(last) = lines.last_mut() {
                    last.0 = (
                        last.0 .0 + word_size.0 + space_size,
                        last.0 .1.max(word_size.1),
                    );
                    last.1.push_str(word);
                    last.1.push(' ');
                    acc_width += word_size.0 + space_size;
                } else {
                    lines.push((
                        (word_size.0 + space_size, word_size.1),
                        format!("{word} "),
                    ));
                    acc_width += word_size.0 + space_size;
                }
            }
            out.extend(lines);
        }
        out
    }

    /// Порт draw_multiline из quad_ui (main.rs:654): возвращает размер
    /// отрисованного блока (max_line_width, высота). centered — центрирование
    /// строк в пределах max_width.
    #[allow(clippy::too_many_arguments)]
    pub fn draw_multiline(
        &mut self,
        gfx: &mut Gfx,
        text: &str,
        pos: (f32, f32),
        max_width: f32,
        centered: bool,
        font: FontId,
        font_size: u16,
        color: [f32; 4],
    ) -> (f32, f32) {
        let lines = self.split_text(gfx, text, font, font_size, max_width);
        let mut cur_y_pos = pos.1;
        let mut max_line_width = 0f32;
        let font_line_distance = font_size as f32;
        for (num, (size, line)) in lines.iter().enumerate() {
            max_line_width = max_line_width.max(size.0);
            if num > 0 {
                cur_y_pos += font_line_distance;
            }
            let cur_x_pos = if centered {
                max_width / 2. - size.0 / 2. + pos.0
            } else {
                pos.0
            };
            self.draw_text(gfx, line, cur_x_pos, cur_y_pos, font, font_size, 1., color);
            if num == lines.len() - 1 {
                cur_y_pos += font_line_distance;
            }
        }
        (max_line_width, cur_y_pos - pos.1)
    }
}

fn upload_sub(
    gfx: &mut Gfx,
    atlas: TexId,
    pixels: &[u8],
    w: u32,
    h: u32,
    x: u32,
    y: u32,
    atlas_w: u32,
) {
    let tex = &gfx.textures_ref(atlas).texture;
    gfx.queue.write_texture(
        wgpu::TexelCopyTextureInfo {
            texture: tex,
            mip_level: 0,
            origin: wgpu::Origin3d { x, y, z: 0 },
            aspect: wgpu::TextureAspect::All,
        },
        pixels,
        wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(w * 4), rows_per_image: Some(h) },
        wgpu::Extent3d { width: w, height: h, depth_or_array_layers: 1 },
    );
    let _ = atlas_w;
}

/// Белая константа для совместимости вызовов.
pub const _WHITE: [f32; 4] = WHITE;
