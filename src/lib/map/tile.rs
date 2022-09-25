#[derive(Copy, Clone)]
pub struct Tile {
    pub walkspeed: u32,
    symbol: char
}
impl Tile {
    pub fn new(walkspeed: u32, symbol: char) -> Self {
        Self {
            walkspeed,
            symbol
    }   }
    pub fn get_symbol(self) -> char {
        self.symbol
}   }
