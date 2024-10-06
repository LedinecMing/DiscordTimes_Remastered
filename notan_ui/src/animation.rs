use notan::{
    app::{assets::Asset, Graphics},
    draw::{Draw, DrawBuilder, DrawImages, DrawTransform, Image},
    graphics::Texture,
};
use parking_lot::MappedRwLockReadGuard;
use std::time::{Duration, Instant, SystemTime};

type TextureFunc<D> = fn(&D) -> Texture; // fn(&D) -> MappedRwLockReadGuard<'_, Texture>,
#[derive(Clone, Debug)]
pub struct AnimationTime<D> {
    pub data: Animation,
    texture_func: TextureFunc<D>,
    init: Instant,
    duration: Duration,
}
impl<D> AnimationTime<D> {
    pub fn get_timings(&self) -> (Instant, Duration) {
        (self.init, self.duration)
    }
    pub fn is_over(&self, now: Instant) -> bool {
        self.init + self.duration < now + Duration::from_millis(250)
    }
    pub fn draw(&self, data: &D, draw: &mut Draw, now: Instant) {
        draw.image(&(self.texture_func)(data))
            .apply_animation(&self.data);
    }
    pub fn new(data: Animation, texture_func: TextureFunc<D>, duration: Duration) -> Self {
        Self {
            data,
            texture_func,
            init: Instant::now(),
            duration,
        }
    }
}

#[derive(Clone, Debug)]
pub struct MovementChange {
    pub from: (f32, f32),
    pub to: (f32, f32),
}
impl MovementChange {
    pub fn new(from: (f32, f32), to: (f32, f32)) -> Self {
        Self { from, to }
    }
}
#[derive(Clone, Debug)]
pub struct SizeChange {
    pub from: (f32, f32),
    pub to: (f32, f32),
}
impl SizeChange {
    pub fn new(from: (f32, f32), to: (f32, f32)) -> Self {
        Self { from, to }
    }
}

#[derive(Clone, Debug)]
pub struct Animation {
    pos: MovementChange,
    size: SizeChange,
    time: (Instant, Duration),
    flip: (bool, bool),
}
impl<'a> Animation {
    pub fn new(
        pos: MovementChange,
        size: SizeChange,
        time: (Instant, Duration),
        flip: (bool, bool),
    ) -> Self {
        Self {
            pos,
            size,
            time,
            flip,
        }
    }
    fn draw_process(&self, draw: &mut DrawBuilder<Image<'a>>) {
        let now = Instant::now();
        let elapsed = now - self.time.0;
        let (start, end) = (self.pos.from, self.pos.to);
        let diff = (end.0 - start.0, end.1 - start.1);
        let elapsed = elapsed.div_duration_f32(self.time.1).clamp(0., 1.);
        let new = (start.0 + diff.0 * elapsed, start.1 + diff.1 * elapsed);
        let (size_f, size_t) = (self.size.from, self.size.to);
        let diff_size = (size_t.0 - size_f.0, size_t.1 - size_f.1);
        let size = (
            size_f.0 + diff_size.0 * elapsed,
            size_f.1 + diff_size.1 * elapsed,
        );
        draw.position(new.0, new.1)
            .flip_x(self.flip.0)
            .flip_y(self.flip.1)
            .size(size.0, size.1);
    }
}

pub trait Animate {
    fn apply_animation(self, anim: &Animation) -> Self;
}
impl Animate for DrawBuilder<'_, Image<'_>> {
    fn apply_animation(mut self, anim: &Animation) -> Self {
        anim.draw_process(&mut self);
        self
    }
}

pub fn load_asset(gfx: &mut Graphics, path: &str) -> Result<Asset<Texture>, String> {
    Ok(Asset::from_data(
        &*path,
        gfx.create_texture()
            .from_image(&std::fs::read(path).unwrap())
            .build()
            .unwrap(),
    ))
}
