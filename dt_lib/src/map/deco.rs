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
impl Ini<'_> for MapDeco {
	fn eat<'a>(input: &'a str, _additional: Self::Arg) -> Result<(&'a str, Self), advini::IniParseError> {
        match <(usize, usize, usize)>::eat(input, ()) {
            Ok(v) => Ok((
				v.0,
                Self {
                    index: v.1.0,
                    x: v.1.1,
                    y: v.1.2,
                }
            )),
            Err(err) => Err(err),
        }
    }
    fn vomit(&self, _additional: Self::Arg) -> String {
        (self.index, self.x, self.y).vomit(())
    }
}
