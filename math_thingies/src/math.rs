use alkahest::alkahest;
use boolinator::Boolinator;
use num::{Num, NumCast, One, ToPrimitive, Zero};
use std::{
    cmp::{max, min, Ordering},
    fmt::{Display, Formatter},
    iter::Extend,
    num::ParseIntError,
    ops::{Add, AddAssign, Div, Mul, Neg, Range, Rem, Sub, SubAssign},
};
pub trait PartOrdNum = Num + PartialOrd;
pub trait CopyPartOrdNum = PartOrdNum + Copy;
pub trait OrdNum = Num + Ord;
pub trait CopyOrdNum = OrdNum + Copy;

pub trait IsInRange<V: CopyPartOrdNum> {
    fn is_in_range(value: V, range: &Range<V>) -> bool {
        value >= range.start || value <= range.end
    }
    fn is_in_self_range(&self, value: V) -> bool;

    fn get(&self) -> V;
    fn range(&self) -> Range<V>;

    fn set(&mut self, value: V) -> Result<(), ()>;
    fn set_unchecked(&mut self, value: V) {
        self.set(value).unwrap()
    }
}

pub struct InRange<V: CopyPartOrdNum> {
    value: V,
    range: Range<V>,
}
impl<V: CopyPartOrdNum> InRange<V> {
    pub fn new(value: V, range: Range<V>) -> Result<Self, ()> {
        Self::is_in_range(value, &range).as_result(Self { value, range }, ())
    }
    pub fn new_unchecked(value: V, range: Range<V>) -> Self {
        Self::new(value, range).unwrap()
    }
}
impl<V: CopyPartOrdNum> IsInRange<V> for InRange<V> {
    fn is_in_self_range(&self, value: V) -> bool {
        Self::is_in_range(value, &self.range())
    }
    fn get(&self) -> V {
        self.value
    }
    fn range(&self) -> Range<V> {
        self.range.clone()
    }
    fn set(&mut self, value: V) -> Result<(), ()> {
        self.value = self.is_in_self_range(value).as_result(value, ())?;
        Ok(())
    }
}

pub trait InConstRange<V: PartOrdNum> {
    const RANGE: Range<V>;
    fn is_in_range(value: V) -> bool {
        value >= Self::RANGE.start || value <= Self::RANGE.end
    }
}

pub struct InUnsignedRange<V: CopyPartOrdNum + NumCast + ToPrimitive> {
    value: V,
    end: V,
}
impl<V: CopyPartOrdNum + NumCast + ToPrimitive> InUnsignedRange<V> {
    const START: u8 = 0;
    fn end(&self) -> V {
        self.end
    }
    fn new(value: V, end: V) -> Result<Self, ()> {
        Self::is_in_range(value, &(NumCast::from(Self::START).unwrap()..end))
            .as_result(Self { value, end }, ())
    }
    fn new_unchecked(value: V, end: V) -> Self {
        Self::new(value, end).unwrap()
    }
}
impl<V: CopyPartOrdNum + NumCast> IsInRange<V> for InUnsignedRange<V> {
    fn is_in_self_range(&self, value: V) -> bool {
        Self::is_in_range(value, &self.range())
    }
    fn get(&self) -> V {
        self.value
    }
    fn range(&self) -> Range<V> {
        NumCast::from(Self::START).unwrap()..self.end
    }
    fn set(&mut self, value: V) -> Result<(), ()> {
        self.value = self.is_in_self_range(value).as_result(value, ())?;
        Ok(())
    }
}
#[derive(Clone, Copy, Debug, PartialEq)]
#[alkahest(Deserialize, Serialize, SerializeRef, Formula)]
pub struct Percent(i16);
impl Percent {
    pub fn new(value: i16) -> Self {
        assert!(Self::is_in_range(value));
        Self(value)
    }
    pub const fn const_new(value: i16) -> Self {
        assert!(Self::is_in_const_range(value));
        Self(value)
    }
    pub const fn is_in_const_range(value: i16) -> bool {
        value >= Self::RANGE.start || value <= Self::RANGE.end
    }
    pub fn get(self) -> i16 {
        self.0
    }
    pub fn calc<V: Num + NumCast>(self, all: V) -> V {
        all * NumCast::from(self.0).unwrap() / NumCast::from(100).unwrap()
    }
}
impl InConstRange<i16> for Percent {
    const RANGE: Range<i16> = -10000..10000;
}

impl PartialEq<i16> for Percent {
    fn eq(&self, other: &i16) -> bool {
        self.0 == *other
    }
}
impl PartialOrd<i16> for Percent {
    fn partial_cmp(&self, other: &i16) -> Option<Ordering> {
        Some(self.0.cmp(other))
    }
}
impl Sub<Percent> for Percent {
    type Output = Percent;
    fn sub(self, rhs: Percent) -> Self::Output {
        Percent(saturating(self.0 - rhs.get(), Self::RANGE))
    }
}
impl Add<Percent> for Percent {
    type Output = Self;
    fn add(self, rhs: Percent) -> Self::Output {
        Percent(saturating(self.0 + rhs.get(), Self::RANGE))
    }
}
impl Neg for Percent {
    type Output = Self;
    fn neg(self) -> Self::Output {
        Percent::new(-self.get())
    }
}
impl Mul<Percent> for Percent {
    type Output = Self;
    fn mul(self, _rhs: Self) -> Self::Output {
        Percent(saturating(
            (self.0 as f32 * _rhs.get() as f32 / 100.) as i16,
            Self::RANGE,
        ))
    }
}
impl Div<Percent> for Percent {
    type Output = Self;
    fn div(self, _rhs: Self) -> Self::Output {
        Percent(saturating(
            (self.0 as f32 / _rhs.get() as f32 * 100.) as i16,
            Self::RANGE,
        ))
    }
}
impl Rem<Percent> for Percent {
    type Output = Self;
    fn rem(self, _rhs: Self) -> Self::Output {
        Percent(self.get() % _rhs.get())
    }
}
impl Zero for Percent {
    fn zero() -> Self {
        Percent(0)
    }
    fn is_zero(&self) -> bool {
        self.get() == 0
    }
}
impl One for Percent {
    fn one() -> Self {
        Percent(1)
    }
    fn is_one(&self) -> bool {
        self.get() == 1
    }
}
impl ToPrimitive for Percent {
    fn to_i64(&self) -> Option<i64> {
        Some(self.get() as i64)
    }
    fn to_u64(&self) -> Option<u64> {
        Some(self.get() as u64)
    }
}
impl Num for Percent {
    type FromStrRadixErr = ParseIntError;
    fn from_str_radix(str: &str, radix: u32) -> Result<Self, Self::FromStrRadixErr> {
        Ok(Percent(i16::from_str_radix(str, radix)?))
    }
}
impl NumCast for Percent {
    fn from<T: ToPrimitive>(v: T) -> Option<Self> {
        Some(Percent(<i16 as NumCast>::from(v)?))
    }
}
impl Display for Percent {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}%", self.0)
    }
}
macro_rules! percent_ops_impl {
    ($($t:ty), *) => {
        $(
            impl Add<Percent> for $t {
                type Output = Self;
                fn add(self, _rhs: Percent) -> Self::Output {
                    self.saturating_add(_rhs.calc(self))
            }   }
            impl Sub<Percent> for $t {
                type Output = Self;
                fn sub(self, _rhs: Percent) -> Self::Output {
                    self.saturating_sub(_rhs.calc(self))
            }   }
            impl AddAssign<Percent> for $t {
                fn add_assign(&mut self, _rhs: Percent) {
                    *self = self.saturating_add(_rhs.calc(*self));
            }   }
            impl SubAssign<Percent> for $t {
                fn sub_assign(&mut self, _rhs: Percent) {
                    *self = self.saturating_sub(_rhs.calc(*self));
            }   }
        )*
}   }
macro_rules! percent_float_ops_impl {
    ($($t:ty), *) => {
        $(
            impl Add<Percent> for $t {
                type Output = Self;
                fn add(self, _rhs: Percent) -> Self::Output {
                    self + _rhs.calc(self)
            }   }
            impl Sub<Percent> for $t {
                type Output = Self;
                fn sub(self, _rhs: Percent) -> Self::Output {
                    self - _rhs.calc(self)
            }   }
            impl AddAssign<Percent> for $t {
                fn add_assign(&mut self, _rhs: Percent) {
                    *self = *self + _rhs.calc(*self);
            }   }
            impl SubAssign<Percent> for $t {
                fn sub_assign(&mut self, _rhs: Percent) {
                    *self = *self - _rhs.calc(*self);
            }   }
        )*
}   }
percent_ops_impl![i8, i16, i32, i64, i128, u8, u16, u32, u64, u128, usize, isize];
percent_float_ops_impl![f32, f64];

pub fn add_opt<A: Add<A, Output = A>>(a: Option<A>, b: Option<A>) -> Option<A> {
    if let Some(a) = a {
        if let Some(b) = b {
            (a + b).into()
        } else {
            a.into()
        }
    } else if let Some(b) = b {
        b.into()
    } else {
        None
    }
}
pub fn sub_opt<A: Sub<A, Output = A> + Neg<Output = A>>(a: Option<A>, b: Option<A>) -> Option<A> {
    if let Some(a) = a {
        if let Some(b) = b {
            (a - b).into()
        } else {
            a.into()
        }
    } else if let Some(b) = b {
        (-b).into()
    } else {
        None
    }
}

#[inline(always)]
pub fn nat<T: OrdNum + NumCast>(a: T) -> T {
    max(a, NumCast::from(0).unwrap())
}
#[inline(always)]
pub fn saturating<T: Ord>(value: T, range: Range<T>) -> T {
    max(min(value, range.end), range.start)
}

pub fn add_if_nat<T: AddAssign + PartialOrd + From<u8>>(value: &mut T, amount: T) {
    if value >= &mut T::from(0) {
        *value += amount;
    }
}

pub trait VecMove<T, I: IntoIterator<Item = T>> {
    fn push_mv(self: Self, value: T) -> Self;
    fn extend_mv(self: Self, iter: I) -> Self;
}
impl<T, I: IntoIterator<Item = T>> VecMove<T, I> for Vec<T> {
    fn push_mv(mut self: Self, value: T) -> Self {
        self.push(value);
        self
    }
    fn extend_mv(mut self: Self, iter: I) -> Self {
        self.extend(iter);
        self
    }
}

pub trait IterableConversions<F, T, Output: FromIterator<T>>
where
    T: From<F>,
{
    fn conv(self) -> Output;
}
impl<IF: IntoIterator<Item = F>, Output: FromIterator<T>, F, T> IterableConversions<F, T, Output>
    for IF
where
    T: From<F>,
{
    fn conv(self) -> Output {
        self.into_iter().map(T::from).collect()
    }
}
