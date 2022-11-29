use notan::app::Color;

#[derive(Copy, Clone)]
pub struct Tile {
    pub walkspeed: u32,
    color: Color
}
impl Tile {
    pub fn new(walkspeed: u32, color: Color) -> Self {
        Self {
            walkspeed,
            color
    }   }
    pub fn get(self) -> Color {
        self.color
}   }
