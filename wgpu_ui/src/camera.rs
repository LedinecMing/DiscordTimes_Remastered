// Порт семантики macroquad Camera2D::from_display_rect.
//
// Выведено из фактических использований в quad_ui (is_hovered, панорамирование
// через px = zoom * screen_w * 0.5, клампы 8e-6..0.02, запечка карты в RT):
//   screen = (world - target) * zoom * (viewport / 2) + viewport / 2   (компонентно)
//   world  = (screen - viewport / 2) / (zoom * (viewport / 2)) + target
// При этом viewport в матрице НЕ участвует: мировая ось X шириной display_rect.w
// заполняет клип [-1; 1] при любом размере вьюпорта ( aspect задаётся самим rect ).
// Ось Y направлена вниз: world y+ -> screen y+.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Camera {
    /// Точка мира в центре экрана.
    pub target: [f32; 2],
    /// macroquad-зум: базово (2 / w, 2 / h) от display rect; ось Y всегда положительная.
    pub zoom: [f32; 2],
    /// Поворот; в проекте всегда 0 (в коде только `camera.rotation = 0.`).
    pub rotation: f32,
}

pub type Vec2 = [f32; 2];

pub fn rect_center(x: f32, y: f32, w: f32, h: f32) -> Vec2 {
    [x + w / 2., y + h / 2.]
}

impl Camera {
    /// Порт `Camera2D::from_display_rect(Rect::new(x, y, w, h))`.
    /// В проекте h всегда отрицательный (Y-вниз); берём |h|.
    pub fn from_display_rect(x: f32, y: f32, w: f32, h: f32) -> Camera {
        Camera {
            target: rect_center(x, y, w, h),
            zoom: [2. / w.abs(), 2. / h.abs()],
            rotation: 0.,
        }
    }

    /// Пикселей на мировую единицу (по осям).
    pub fn pixels_per_world(&self, viewport: Vec2) -> Vec2 {
        [self.zoom[0] * viewport[0] * 0.5, self.zoom[1] * viewport[1] * 0.5]
    }

    pub fn world_to_screen(&self, p: Vec2, viewport: Vec2) -> Vec2 {
        let ppw = self.pixels_per_world(viewport);
        [
            (p[0] - self.target[0]) * ppw[0] + viewport[0] * 0.5,
            (p[1] - self.target[1]) * ppw[1] + viewport[1] * 0.5,
        ]
    }

    pub fn screen_to_world(&self, s: Vec2, viewport: Vec2) -> Vec2 {
        let ppw = self.pixels_per_world(viewport);
        [
            (s[0] - viewport[0] * 0.5) / ppw[0] + self.target[0],
            (s[1] - viewport[1] * 0.5) / ppw[1] + self.target[1],
        ]
    }

    /// Униформа камеры для шейдера: clip = (x * zx - cx, -(y * zy) + cy).
    /// Независима от вьюпорта (см. доку модуля).
    pub fn gpu_uniform(&self) -> [f32; 4] {
        let (zx, zy) = (self.zoom[0], self.zoom[1]);
        [zx, zy, self.target[0] * zx, self.target[1] * zy]
    }

    /// Кламп зума в границах по осям с сохранением их соотношения.
    ///
    /// Осевой кламп (порт quad_ui) ломал аспект неквадратного display rect:
    /// у карты zoom[0] = 2/(32·N) < zoom[1] = 2/(22·N), и при зуме оси
    /// достигали границ в разные моменты (Y упирался раньше X) — карту
    /// сплющивало. Здесь масштабируется пара целиком, пока все оси не
    /// войдут в границы; если часть осей выше max, рост запрещён.
    pub fn clamp_zoom_between(&mut self, min: [f32; 2], max: [f32; 2]) {
        let mut k: f32 = 1.;
        let mut over = false;
        for i in 0..2 {
            let z = self.zoom[i].abs();
            if z > max[i] {
                over = true;
                k = k.min(max[i] / z);
            } else if z < min[i] && !over {
                k = k.max(min[i] / z);
            }
        }
        if k != 1. {
            self.zoom[0] *= k;
            self.zoom[1] *= k;
        }
    }

    /// Кламп зума как в draw_map (порт quad_ui): модуль в [8e-6; 0.02],
    /// но с сохранением соотношения осей.
    pub fn clamp_zoom(&mut self) {
        self.clamp_zoom_between([0.000_008; 2], [0.02; 2]);
    }
}

/// Камера по умолчанию macroquad (set_default_camera): экранные пиксели, Y-вниз.
pub fn default_camera(viewport: Vec2) -> Camera {
    Camera::from_display_rect(0., viewport[1], viewport[0], -viewport[1])
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cam(zoom: [f32; 2]) -> Camera {
        Camera { target: [0., 0.], zoom, rotation: 0. }
    }

    /// База карты 50 тайлов (1600x1100): оси зума различаются.
    const BASE: [f32; 2] = [2. / 1600., 2. / 1100.];

    /// Приближение далеко за max: кламп упирает оси в max, сохраняя
    /// соотношение. Осевой кламп (старый) давал [0.02; 0.02] и сплющивал
    /// карту.
    #[test]
    fn clamp_in_preserves_ratio() {
        let max = [BASE[0] * 12., BASE[1] * 12.];
        // Соотношение осей = базовому: обе упираются одновременно.
        let mut c = cam([BASE[0] * 100., BASE[1] * 100.]);
        c.clamp_zoom_between([0.; 2], max);
        assert!((c.zoom[1] / c.zoom[0] - BASE[1] / BASE[0]).abs() < 1e-4);
        assert!((c.zoom[0] - max[0]).abs() < max[0] * 1e-5);
        assert!((c.zoom[1] - max[1]).abs() < max[1] * 1e-5);
        // Соотношение отличается: упирается только ось у своей границы.
        let mut c = cam([BASE[0] * 100., BASE[1] * 30.]);
        c.clamp_zoom_between([0.; 2], max);
        assert!((c.zoom[0] - max[0]).abs() < max[0] * 1e-5);
        assert!(c.zoom[1] < max[1]);
        assert!(
            (c.zoom[1] / c.zoom[0] - (BASE[1] * 30.) / (BASE[0] * 100.)).abs() < 1e-4
        );
    }

    /// Отдаление дальше «вся карта»: кламп подтягивает оси к min,
    /// соотношение сохранено.
    #[test]
    fn clamp_out_preserves_ratio() {
        let max = [BASE[0] * 12., BASE[1] * 12.];
        let mut c = cam([BASE[0] / 1000., BASE[1] / 1000.]);
        c.clamp_zoom_between(BASE, max);
        assert!((c.zoom[1] / c.zoom[0] - BASE[1] / BASE[0]).abs() < 1e-4);
        assert!((c.zoom[0] - BASE[0]).abs() < BASE[0] * 1e-5);
        assert!((c.zoom[1] - BASE[1]).abs() < BASE[1] * 1e-5);
    }

    /// Внутри границ кламп — no-op.
    #[test]
    fn clamp_noop_inside_bounds() {
        let min = [BASE[0] / 2., BASE[1] / 2.];
        let max = [BASE[0] * 8., BASE[1] * 8.];
        let mut c = cam(BASE);
        c.clamp_zoom_between(min, max);
        assert_eq!(c, cam(BASE));
    }

    /// Legacy-кламп: обе оси в [8e-6; 0.02], соотношение сохранено.
    #[test]
    fn legacy_clamp_keeps_ratio_and_bounds() {
        let mut c = cam([0.05, 0.06]);
        c.clamp_zoom();
        assert!((c.zoom[1] - 0.02).abs() < 1e-9);
        assert!((c.zoom[0] / c.zoom[1] - 0.05 / 0.06).abs() < 1e-4);
        let mut far = cam([1e-9, 2e-9]);
        far.clamp_zoom();
        assert!((far.zoom[0] - 8e-6).abs() < 1e-12);
        assert!((far.zoom[1] / far.zoom[0] - 2.).abs() < 1e-4);
    }

    /// Снап при включении лимитов: вид за пределами карты подтягивается
    /// к «вся карта» за один кламп.
    #[test]
    fn snap_to_map_bounds() {
        let max = [BASE[0] * 12., BASE[1] * 12.];
        let mut c = cam([BASE[0] * 0.5, BASE[1] * 0.5]); // вид шире карты
        c.clamp_zoom_between(BASE, max);
        assert!((c.zoom[0] - BASE[0]).abs() < 1e-9);
        assert!((c.zoom[1] / c.zoom[0] - BASE[1] / BASE[0]).abs() < 1e-4);
    }
}
