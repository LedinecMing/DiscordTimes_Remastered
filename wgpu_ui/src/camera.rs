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

    /// Кламп зума как в draw_map: сохраняет оси, модуль в [8e-6; 0.02].
    pub fn clamp_zoom(&mut self) {
        for z in self.zoom.iter_mut() {
            *z = z.abs().clamp(0.000_008, 0.02);
        }
    }
}

/// Камера по умолчанию macroquad (set_default_camera): экранные пиксели, Y-вниз.
pub fn default_camera(viewport: Vec2) -> Camera {
    Camera::from_display_rect(0., viewport[1], viewport[0], -viewport[1])
}
