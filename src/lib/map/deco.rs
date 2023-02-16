#[derive(Copy, Clone, Debug)]
pub struct MapDeco {
    pub index: usize,
}
impl MapDeco {
    pub fn new(index: usize, name: String) -> Self {
        Self { index }
    }
}
