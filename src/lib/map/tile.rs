use once_cell::sync::Lazy;

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

pub static TILES: Lazy<[Tile; 10]> = Lazy::new(|| {
    [
        Tile::new(2, "Land.png", false),
        Tile::new(1, "Badground.png", false),
        Tile::new(4, "Road.png", false),
        Tile::new(2, "Snow.png", false),
        Tile::new(2, "Plain.png", false),
        Tile::new(2, "LowLand.png", false),
        Tile::new(4, "Water.png", true),
        Tile::new(4, "Shallow.png", true),
        Tile::new(0, "DeepWater.png", false),
        Tile::new(2, "Desert.png", false),
    ]
});
