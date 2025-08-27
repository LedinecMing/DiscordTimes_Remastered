use advini::Ini;
use alkahest::*;

#[derive(Copy, Clone, PartialEq, Debug)]
#[alkahest(Deserialize, Serialize, SerializeRef, Formula)]
pub struct MapDeco {
    pub index: usize,
    pub x: usize,
    pub y: usize,
}
impl MapDeco {
    pub fn new(index: usize, x: usize, y: usize) -> Self {
        Self { index, x, y }
    }
}
impl Ini for MapDeco {
    fn eat(chars: std::str::Chars) -> Result<(Self, std::str::Chars), advini::IniParseError> {
        match <(usize, usize, usize)>::eat(chars) {
            Ok(v) => Ok((
                Self {
                    index: v.0 .0,
                    x: v.0 .1,
                    y: v.0 .2,
                },
                v.1,
            )),
            Err(err) => Err(err),
        }
    }
    fn vomit(&self) -> String {
        (self.index, self.x, self.y).vomit()
    }
}
