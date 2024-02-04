use notan::prelude::*;
use notan_ui::*;

pub struct Movement {
	pub from: (f32, f32),
	pub to: (f32, f32)
}
pub struct Size {
	pub from: (f32, f32),
	pub to: (f32, f32)
}
pub enum Animations {
	Movement(Movement, DrawAnim<Animations>),
	ChangeSize(Size)
}

impl GetDrawFunc<GameAnimations> for GameAnimations {
	fn get_draw_func(&self) -> DrawAnim<GameAnimations> {
		let elapsed = now - anim.init;
		let (start, end) = (movement.from, movement.to);
		let diff = (end.0 - start.0, end.1 - start.1);
		let elapsed = anim.duration.div_duration_f64(elapsed);
		let new = (start.0 + diff.0 * elapsed, start.1 + diff.1 * elapsed);
		todo!()
	}
}
