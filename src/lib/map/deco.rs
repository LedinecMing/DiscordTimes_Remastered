#[derive(Copy, Clone, Debug)]
pub struct MapDeco {
    pub can_be_passed: bool,
    symbol: char
}
impl MapDeco {
    pub fn new(can_be_passed: bool, symbol: char) -> Self {
        Self {
            can_be_passed,
            symbol
    }    }
    pub fn get_symbol(self) -> char {
        self.symbol
}   }