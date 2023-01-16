#[derive(Copy, Clone)]
pub struct Tile {
    pub walkspeed: u32,
    sprite_id: &'static str,
    need_transport: bool,
}
impl Tile {
    pub fn new(walkspeed: u32, sprite_id: &'static str, need_transport: bool) -> Self {
        Self {
            walkspeed,
            sprite_id,
            need_transport,
        }
    }
    pub fn sprite(&self) -> &'static str {
        self.sprite_id
    }
    pub fn need_transport(&self) -> bool {
        self.need_transport
    }
}
