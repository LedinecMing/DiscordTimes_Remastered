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