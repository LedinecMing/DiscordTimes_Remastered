use {
    derive_more::{Add, Sub, Mul, Neg, AddAssign, SubAssign}
};


#[derive(Add, AddAssign, Sub, SubAssign, Neg, Mul, Copy, Clone, Debug)]
pub struct Position(pub f32, pub f32);
#[derive(Add, AddAssign, Sub, SubAssign, Neg, Mul, Copy, Clone, Debug)]
pub struct Rect {
    pub pos: Position,
    pub size: Position
}
impl Rect {
    pub fn collides(&self, pos: Position) -> bool {
        pos.0 >= self.pos.0 && pos.0 <= self.pos.0 + self.size.0 &&
           pos.1 >= self.pos.1 && pos.1 <= self.pos.1 + self.size.1
}   }
impl Default for Rect {
    fn default() -> Self {
        Self { pos: Position::default(), size: Position::default() }
}   }
impl Into<(f32, f32)> for Position {
    fn into(self) -> (f32, f32) {
        (self.0, self.1)
}   }
impl From<(f32, f32)> for Position {
    fn from(values: (f32, f32)) -> Self {
        Self(values.0, values.1)
}   }
impl Default for Position {
    fn default() -> Self {
        Self(0., 0.)
}   }