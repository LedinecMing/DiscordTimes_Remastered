#![feature(associated_type_defaults)]
use nom::{Err, Finish, IResult, Parser as NomParser, branch::alt, bytes::complete::{is_a, is_not, tag, tag_no_case, take_till, take_until, take_while, take_while1}, character::complete::digit1, combinator::{consumed, map_res, not, opt, recognize, rest, value}, error::{Error, ErrorKind, ParseError}, number::{self, complete::{double, float}}, sequence::{preceded, terminated}};
pub use advini_derive::*;
use ini_core::{Item, Parser};
use num::{Num, Zero};
use std::{
	error::Error as ErrorTrait, fmt::{Debug, Display}
};
use indexmap::IndexMap;
pub fn parse_for_sections(ini_doc: &str) -> Vec<(String, IndexMap<String, String>)> {
    parse_for_sections_with(
        ini_doc,
        |(prop, v, _s)| (prop.to_lowercase(), v.to_string()),
        &mut 0,
    )
}
pub fn parse_for_sections_with<'a, S>(
    ini_doc: &'a str,
    with: fn((&'a str, &'a str, &mut S)) -> (String, String),
    s: &mut S,
) -> Vec<(String, IndexMap<String, String>)> {
    let mut result = Vec::new();
    let mut old_sec = "";
    let mut last_prop = "".into();
    let mut props: IndexMap<String, String> = IndexMap::new();
    let parser = Parser::new(&*ini_doc).auto_trim(true);
    for item in parser {
        match item {
            Item::Section(sec) => {
                if !old_sec.is_empty() {
                    result.push((old_sec.into(), props));
                    props = IndexMap::new();
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
            Item::Blank | Item::Comment(_) => {}
            Item::Action(v) => {
                if let Some(old) = props.get_mut(&last_prop) {
                    old.push_str(v);
                };
            }
            Item::Error(err) => panic!("{}", err),
        }
    }
    result.push((old_sec.into(), props));
    result
}

pub fn trim_separator(input: &str) -> IResult<&str, (), Error<&str>> {
	if let Ok((rest, _)) = tag::<_, _, Error<&str>>(",")(input) {
		Ok((rest, ()))
	} else { Err(nom::Err::Error(Error::new(input, ErrorKind::Tag))) }
}

fn parse_string_from_string(input: &str) -> IResult<&str, &str, Error<&str>> {
	if let Ok((rest, _)) = trim_separator(input) {
		return Ok((rest, ""));
	}
	let mut opening = consumed(
		opt( alt( [is_a("\""), is_a("\'")] ) )
	);
	let (inner, (opening, _)) = opening.parse(input)?;
	let (opening, ends_with_separator) = match opening {
		"" => (",", true),
		x => (x, false)
	};
	if ends_with_separator {
		terminated(
			take_until(opening).or(rest),
			opt(tag(",")))
			.parse(inner)
	} else {
		terminated(
			take_until(opening).or(rest),
			opt(take_while(|ch| matches!(ch, '\"' | '\'')))
		).parse(inner)
 	}
}

pub const SEPARATOR: char = ',';

#[derive(Debug, PartialEq)]
pub enum IniParseError {
    Error(String),
    Empty,
}
impl From<Err<Error<&str>>> for IniParseError {
	fn from(value: Err<Error<&str>>) -> Self {
		IniParseError::Error(value.to_string())
	}
}
impl ErrorTrait for IniParseError {}
impl Display for IniParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            IniParseError::Error(string) => string,
            IniParseError::Empty => "just no chars",
        })
    }
}
impl From<&'static str> for IniParseError {
    fn from(value: &'static str) -> Self {
        Self::Error(value.to_owned())
    }
}
pub trait Sections<'z>
where
    Self: Sized,
{
	type Arg;
    fn from_section<'a>(
        sec: IndexMap<String, String>,
		_additional: Self::Arg
    ) -> Result<(Self, IndexMap<String, String>), String>;
    fn to_section<'a>(&self, _addtional: Self::Arg) -> IndexMap<String, String>;
}
impl<'z, T: Sections<'z>> Sections<'z> for Option<T> {
	type Arg = <T as Sections<'z>>::Arg;
	fn from_section<'a>(
        sec: IndexMap<String, String>,
		_additional: Self::Arg
    ) -> Result<(Self, IndexMap<String, String>), String> {
        T::from_section(sec, _additional).and_then(|res| Ok((Some(res.0), res.1)))
    }
    fn to_section(&self, _addtional: Self::Arg) -> IndexMap<String, String> {
        let Some(data) = self else {
            return IndexMap::new();
        };
        data.to_section(_addtional)
    }
}
pub trait Ini<'b>
where
    Self: Sized,
{
	type Arg = ();
    fn eat<'a>(input: &'a str, _additional: Self::Arg) -> Result<(&'a str, Self), IniParseError>;
    fn vomit(&self, _additional: Self::Arg) -> String;
}
impl Ini<'_> for String {
	fn eat<'a>(input: &'a str, _: Self::Arg) -> Result<(&'a str, Self), IniParseError> {
        parse_string_from_string(input).map(|x| (x.0, x.1.to_owned()))
			.map_err(Into::into)
    }
    fn vomit(&self, _: Self::Arg) -> String {
        let amount = self
            .chars()
            .fold((0, 0, false), |acc, chr| match (chr, acc.2) {
                ('"', true) => (acc.0, acc.1 + 1, true),
                (_, true) => (acc.1, 0, false),
                ('"', false) => (acc.0, 1, true),
                (_, _) => acc,
            })
            .0;
        let beginning = (0..=amount).map(|_| "\"").collect::<Vec<&str>>().concat();
        let mut res = beginning.clone();
        res.push_str(self.as_str());
        res.push_str(beginning.as_str());
        res
    }
}
impl Ini<'_> for bool {
    fn eat<'a>(input: &'a str, _additional: Self::Arg) -> Result<(&'a str, Self), IniParseError> {
		if let Ok((rest, _)) = trim_separator(input) {
			return Ok((rest, false));
		}
		
		terminated(
			alt([
				value(true, alt([
					tag_no_case("y"),
					tag_no_case("t"),
					tag_no_case("1")
				])),
				value(false, alt([
					tag_no_case("f"),
					tag_no_case("n"),
					tag_no_case("0")
				])),
			]),
			take_till(|ch| ch == ',')
		)
			.parse(input).map_err(Into::into)
		}
    fn vomit(&self, _additional: Self::Arg) -> String {
        if *self {
            "true".into()
        } else {
            "false".into()
        }
    }
}
impl<'b, T: Ini<'b>> Ini<'b> for Option<T> {
	type Arg = T::Arg;
	fn eat<'a>(input: &'a str, _additional: Self::Arg) -> Result<(&'a str, Self), IniParseError> {
		if let Ok((rest, _)) = trim_separator(input) {
			return Ok((rest, None));
		};
		let res = T::eat(input, _additional)?;
		Ok((res.0, Some(res.1)))
    }
	fn vomit(&self, _additional: Self::Arg) -> String {
        match self {
            Some(v) => v.vomit(_additional),
            None => "".to_string(),
        }
    }
}

macro_rules! impl_for_num {
    ($ty:ty) => {
        impl Ini<'_> for $ty {
            fn eat<'a>(input: &'a str, _: Self::Arg) -> Result<(&'a str, Self), IniParseError> {
				if let Ok((rest, _)) = trim_separator(input) {
					return Ok((rest, <Self as Zero>::zero()));
				}
				let (rest, res) = digit1(input)?;
                Ok((rest, Num::from_str_radix(res, 10).map_err(|_| IniParseError::Empty)?))
            }
            fn vomit(&self, _: Self::Arg) -> String {
                self.to_string()
            }
        }
    };
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
impl_for_num!(usize);
impl_for_num!(isize);

impl Ini<'_> for f32 {
	fn eat<'a>(input: &'a str, _additional: Self::Arg) -> Result<(&'a str, Self), IniParseError> {
		if let Ok((rest, _)) = trim_separator(input) {
			return Ok((rest, <Self as Zero>::zero()));
		};
		Ok(float(input)?)
	}
	fn vomit(&self, _additional: Self::Arg) -> String {
		self.to_string()
	}
}
impl Ini<'_> for f64 {
	fn eat<'a>(input: &'a str, _additional: Self::Arg) -> Result<(&'a str, Self), IniParseError> {
		if let Ok((rest, _)) = trim_separator(input) {
			return Ok((rest, <Self as Zero>::zero()));
		};
		Ok(double(input)?)
	}
	fn vomit(&self, _additional: Self::Arg) -> String {
		self.to_string()
	}
}


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
		impl<'z, T: 'z + Copy, $typ : Ini<'z, Arg=T>, $( $ntyp : Ini<'z, Arg=T>),*> Ini<'z> for ($typ, $( $ntyp ),*) {
			type Arg = T;
			fn eat<'a>(input: &'a str, _additional: Self::Arg) -> Result<(&'a str, Self), IniParseError> {
				let mut rest = input;
				let result = (
					{
						let res;
						(rest, res) = <$typ as Ini>::eat(rest, _additional)?;
						(rest, _) = tag(",").parse(rest)?; 
						res
					},
					$(
						{
							let res;
							(rest, res) = <$ntyp as Ini>::eat(rest, _additional)?;
							(rest, _) = tag(",").parse(rest)?;	
							res
						},
					)*
				);
				Ok((rest, result))
			}
			fn vomit(&self, _additional: Self::Arg) -> String {
				[self.$idx.vomit(_additional), $( self.$nidx.vomit(_additional) ), *].join(",")
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

impl<'z, T: Ini<'z>> Ini<'z> for Vec<T> where T::Arg: Copy {
	type Arg = T::Arg;
    fn eat<'a>(mut input: &'a str, _additional: Self::Arg) -> Result<(&'a str, Self), IniParseError> {
        let mut new = Vec::new();
		loop {
			if input.is_empty() {
				break;
			}
			let (rest, value) = T::eat(input, _additional)?;
			input = tag(",")(rest)?.0;
			new.push(value);
		}
		Ok((input, new))
		
    }
    fn vomit(&self, _additional: Self::Arg) -> String {
        self.iter()
            .fold(String::new(), |acc, el| acc + &el.vomit(_additional) + ",")
    }
}
pub type Section = IndexMap<String, String>;
pub type SectionError = String;

pub fn parse_for_props(ini_doc: &str) -> Vec<(String, String)> {
    let mut props: Vec<(String, String)> = Vec::new();
    let parser = Parser::new(ini_doc).auto_trim(true);
    for item in parser {
        match item {
            Item::Section(_) => {}
            Item::Property(k, v) => {
                props.push((k.to_lowercase(), v.into()));
            }
            Item::Blank | Item::Comment(_) => {}
            Item::Action(v) => {
                if let Some(old) = props.last_mut() {
                    old.1.push_str(v);
                };
            }
            Item::Error(err) => panic!("{}", err),
        }
    }
    props
}


#[test]
pub fn test_string_parsing() {
	for quote in ["'", "\""] {
		for i in 0..=10 {
			let term = quote.repeat(i);
			let input = &format!("{term}хихишки{term}");
			assert_eq!(String::eat(input, ()), Ok(("", "хихишки".to_owned())));
		}
	}
	for i in 1..=10 {
		let input = &",тест".repeat(i);
		let output: &str = &input[1..];
		assert_eq!(String::eat(input, ()), Ok((output, String::new())));
	}
	assert_eq!(String::eat("тест", ()), Ok(("", "тест".to_owned())));
	assert_eq!(String::eat("тест,", ()), Ok(("", "тест".to_owned())));
	assert_eq!(String::eat(",", ()), Ok(("", "".to_owned())));
}

#[test]
pub fn test_vec() {
	let v = vec![1; 10];
	let output = "1,".repeat(10);
	assert_eq!(v.vomit(()), output);
}
