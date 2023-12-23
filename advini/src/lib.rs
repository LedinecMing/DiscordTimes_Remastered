use std::{collections::{HashMap}, str::Chars, fmt::{Display, Debug}};
use ini_core::{Item, Parser};
use num::Num;
pub use advini_derive::*;

pub fn parse_for_sections(ini_doc: &str) -> Vec<(String, HashMap<String, String>)> {
    parse_for_sections_with(ini_doc, |(prop, v, _s)| (prop.to_lowercase(), v.to_string()), &mut 0)
}
pub fn parse_for_sections_with<'a, S>(ini_doc: &'a str, with: fn((&'a str, &'a str, &mut S)) -> (String, String), s: &mut S) -> Vec<(String, HashMap<String, String>)> {
    let mut result = Vec::new();
    let mut old_sec = "";
	let mut last_prop = "".into();
    let mut props: HashMap<String, String> = HashMap::new();
    let parser = Parser::new(&*ini_doc).auto_trim(true);
    for item in parser {
        match item {
            Item::Section(sec) => {
                if !old_sec.is_empty() {
                    result.push((old_sec.into(), props));
                    props = HashMap::new();
                    old_sec = sec;
                } else {
                    old_sec = sec
                }
            }
            Item::Property(k, v) => {
				let (prop, v) = with((k, v, s));
                props.insert(prop.clone(), v);
				last_prop = prop;
            }
            Item::Blank | Item::Comment(_) => {},
			Item::Action(v) => {
				if let Some(old) = props.get_mut(&last_prop) {
					old.push_str(v);
				};
			},
            Item::Error(err) => panic!("{}", err),
        }
    }
    result.push((old_sec.into(), props));
	result
}
pub const SEPARATOR: char = ',';
pub fn parse_string_from_string<'a>(mut chars: Chars<'a>) -> Result<(String, Chars<'a>), IniParseError> {
	let mut level = (0, false);
	let mut initial_level = 0;
	let mut result_string = String::new();
	loop {
		if let Some(chr) = chars.next() {
			match (level.1, chr) {
				(false, '"') => {
					level.0 += 1;
					initial_level = level.0;
					continue;
				},
				(false, _) => {
					level.1 = true;
				},
				(true, '"') => {
					level.0 -= 1;
					if let Some(chr) = chars.next() {
						if chr == '"' {
							if level.0 == 0 {
								result_string.push('"');
							}
							level.0 = 0.max(level.0 - 1);
						} else {
							if chr == SEPARATOR && level.0 == 0 {
								break
							} else {
								result_string.push('"');
								result_string.push(chr);
								level.0 = initial_level;
							} 
						}
						continue;
					}
					continue;
				},
				(true, SEPARATOR) => {
					if level.0 == 0 {
						break;
					}
				}
				(_, _) => {}
			}
			result_string.push(chr);
		} else {
			if result_string.is_empty() {
				return Err(IniParseError::Empty(chars));
			} else {
				break;
			};
		}
	}
	Ok((result_string, chars))
		
}

#[derive(Debug)]
pub enum IniParseError<'a> {
	Error(&'a str),
	Empty(Chars<'a>)
}
impl From<IniParseError<'static>> for &'static str {
	fn from(value: IniParseError<'static>) -> Self {
		match value {
			IniParseError::Error(string) => string,
			IniParseError::Empty(_) => "just no chars"
		}
	}
}
impl Display for IniParseError<'_> {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.write_str(match self {
			IniParseError::Error(string) => string,
			IniParseError::Empty(_) => "just no chars"
		})
	}
}
impl From<&'static str> for IniParseError<'_> {
	fn from(value: &'static str) -> Self {
		Self::Error(value)
	}
}
pub trait Sections where Self: Sized {
	fn from_section(sec: HashMap<String, String>) -> Result<(Self, HashMap<String, String>), &'static str>;
	fn to_section(&self) -> HashMap<String, String>;
}
pub trait Ini where Self: Sized {
	fn eat<'a>(chars: Chars<'a>) -> Result<(Self, Chars<'a>), IniParseError>;
	fn vomit(&self) -> String;
}
impl Ini for String {
	fn eat<'a>(chars: Chars<'a>) -> Result<(Self, Chars<'a>), IniParseError> {
		parse_string_from_string(chars)
	}
	fn vomit(&self) -> String {
		let amount = self
			.chars()
			.fold((0, 0, false), |acc, chr|
				  match (chr, acc.2) {
					  ('"', true) => {
						  (acc.0, acc.1 + 1, true)
					  },
					  (_, true) => {
						  (acc.1, 0, false)
					  },
					  ('"', false) => {
						  (acc.0, 1, true)
					  },
					  (_,_) => { acc }
				  }
			).0;
		let beginning = (0..=amount).map(|_| "\"").collect::<Vec<&str>>().concat();
		let mut res = beginning.clone();
		res.push_str(self.as_str());
		res.push_str(beginning.as_str());
		res
	}
}
impl Ini for bool {
	fn eat<'a>(mut chars: Chars<'a>) -> Result<(Self, Chars<'a>), IniParseError> {
		loop {
			if let Some(chr) = chars.next() {
				match chr {
					SEPARATOR => {
						break Err(IniParseError::Empty(chars))
					},
					't' | 'y' | '1' => {
						break Ok((true, chars))
					},
					'f' | 'n' | '0' => {
						break Ok((false, chars))
					},
					_ => continue
				}
			} else {
				break Err(IniParseError::Empty(chars))
			}
		}
	}
	fn vomit(&self) -> String {
		if *self {
			"true".into()
		} else {
			"false".into()
		}
	}
}
// impl<'b> Ini for &'b str {
// 	fn eat<'a>(chars: Chars<'a>) -> Result<(Self, Chars<'a>), IniParseError> {
// 		match parse_string_from_string(chars) {
// 			Ok(v) => Ok((v.0.as_str(), v.1)),
// 			Err(a) => Err(a)
// 		}
// 	}
// 	fn vomit(&self) -> String {
// 		let amount = self
// 			.chars()
// 			.fold((0, 0, false), |acc, chr|
// 				  match (chr, acc.2) {
// 					  ('"', true) => {
// 						  (acc.0, acc.1 + 1, true)
// 					  },
// 					  (_, true) => {
// 						  (acc.1, 0, false)
// 					  },
// 					  ('"', false) => {
// 						  (acc.0, 1, true)
// 					  },
// 					  (_,_) => { acc }
// 				  }
// 			).0;
// 		let beginning = (0..=amount).map(|_| "\"").collect::<Vec<&str>>().concat();
// 		let mut res = beginning.clone();
// 		res.push_str(self);
// 		res.push_str(beginning.as_str());
// 		res
// 	}
// }
impl Ini for char {
	fn eat<'a>(mut chars: Chars<'a>) -> Result<(Self, Chars<'a>), IniParseError> {
		Ok(
			({
				let chr = match chars.next() {
					Some(v) => v,
					None => return Err(IniParseError::Empty(chars))
 				};
				if chars.next() != Some(SEPARATOR) {
					return Err("problems with your char".into());
				}
				chr
			},
			chars)
		)
	}
	fn vomit(&self) -> String {
		self.to_string()
	}
}

macro_rules! impl_for_num {
	($ty:ty) => {
		impl Ini for $ty {
			fn eat<'a>(mut chars: Chars<'a>) -> Result<(Self, Chars<'a>), IniParseError> {
				let mut str_repr = String::new();
				loop {
					if let Some(chr) = chars.next() {
						if chr != SEPARATOR {
							str_repr.push(chr);
						} else {
							return Ok((Num::from_str_radix(&str_repr.trim(), 10).map_err(|_| "Parsing of num failed")?, chars));
						}
					} else {
						return if str_repr.is_empty() {
							Err(IniParseError::Empty(chars))
						} else {
							Ok((Num::from_str_radix(&str_repr.trim(), 10).map_err(|_| "Parsing of num failed")?, chars))
						}
					}
				}
			}
			fn vomit(&self) -> String {
				self.to_string()
			}
		}
	}
}
impl_for_num!(i128);
impl_for_num!(i64);
impl_for_num!(i32);
impl_for_num!(i16);
impl_for_num!(i8);
impl_for_num!(u128);
impl_for_num!(u64);
impl_for_num!(u32);
impl_for_num!(u16);
impl_for_num!(u8);
impl_for_num!(f32);
impl_for_num!(f64);
impl_for_num!(usize);
impl_for_num!(isize);

macro_rules! tuple_impls {
    () => {};
    (($idx:tt => $typ:ident), $( ($nidx:tt => $ntyp:ident), )*) => {
        tuple_impls!([($idx, $typ);] $( ($nidx => $ntyp), )*);
        tuple_impls!($( ($nidx => $ntyp), )*); // invoke macro on tail
    };
     ([$(($accIdx: tt, $accTyp: ident);)+]  ($idx:tt => $typ:ident), $( ($nidx:tt => $ntyp:ident), )*) => {
      tuple_impls!([($idx, $typ); $(($accIdx, $accTyp); )*] $( ($nidx => $ntyp), ) *);
    };

    ([($idx:tt, $typ:ident); $( ($nidx:tt, $ntyp:ident); )*]) => {
		impl<$typ : Ini, $( $ntyp : Ini),*> Ini for ($typ, $( $ntyp ),*) {
			fn eat<'a>(mut chars: Chars<'a>) -> Result<(Self, Chars<'a>), IniParseError> {
				let result = (
					{
						let res;
						(res, chars) = <$typ as Ini>::eat(chars)?;
						res
					},
					$(
						{
							let res;
							(res, chars) = <$ntyp as Ini>::eat(chars)?;
							res
						},
					)*
				);
				Ok((result, chars))
			}
			fn vomit(&self) -> String {
				[self.$idx.vomit(), $( self.$nidx.vomit() ), *].join(",")
			}
		}
	}
}
tuple_impls!(
    (9 => J),
    (8 => I),
    (7 => H),
    (6 => G),
    (5 => F),
    (4 => E),
    (3 => D),
    (2 => C),
    (1 => B),
    (0 => A),
);

impl<T: Ini> Ini for Vec<T> {
	fn eat<'a>(mut chars: Chars<'a>) -> Result<(Self, Chars<'a>), IniParseError> {
		let mut new = Vec::new();
		loop {
			let value;
			let res = T::eat(chars);
			(value, chars) = match res {
				Ok(v) => v,
				Err(IniParseError::Empty(chars)) => return Ok((new, chars)),
				err => return Err(err.err().unwrap()),
			};
			new.push(value);
		};
	}
	fn vomit(&self) -> String {
		self.iter().fold(String::new(), |acc, el| acc + &el.vomit() + ",")
	}
}
pub type Section = HashMap<String, String>;
pub type SectionError = &'static str;      

pub fn parse_for_props(ini_doc: &str) -> HashMap<String, String> {
    let mut props: HashMap<String, String> = HashMap::new();
    let parser = Parser::new(ini_doc).auto_trim(true);
	let mut last_prop = "".to_string();
    for item in parser {
        match item {
            Item::Section(_) => {}
            Item::Property(k, v) => {
                props.insert(k.to_lowercase().into(), v.into());
				last_prop = k.into();
            }
            Item::Blank | Item::Comment(_) => {}
			Item::Action(v) => {
				if let Some(old) = props.get_mut(&last_prop) {
					old.push_str(v);
				};
			},
            Item::Error(err) => panic!("{}", err),
        }
    }
    props
}
