use {
    std::{
        cmp::Ordering,
        ops::{Add, AddAssign, Sub, SubAssign, Div, Mul, Range},
        fmt::{Display, Formatter}
    },
    num::{Num, NumCast}
};
pub struct InRange<V: Num + PartialOrd>{
    value: V,
    range: Range<V>
}
impl<V: Num + PartialOrd> InRange<V> {
    pub fn new(value: V, range: Range<V>) -> Result<Self, ()> {
        if !(value < range.start ||
            value > range.end) {
            Ok(Self {
                value,
                range
            })
        } else {
            Err(())
    }   }
    pub fn new_unchecked(value: V, range: Range<V>) -> Self {
        assert!(!(value < range.start ||
                value > range.end));
        Self {
            value,
            range
    }   }
    pub fn get(self) -> V {
        self.value
    }
    pub fn set(&mut self, value: V) -> Result<(), ()> {
        if !(value < self.range.start ||
            value > self.range.end) {
            self.value = value;
            Ok(())
        } else {
            Err(())
    }   }
    pub fn set_unchecked(&mut self, value: V) {
        assert!(!(value < self.range.start ||
            value > self.range.end));
        self.value = value;
}   }

#[derive(Clone, Copy, Debug)]
pub struct Percent(i16);
impl Percent {
    const RANGE: Range<i16> = -99..100;
    pub fn new(value: i16) -> Self {
        assert!(value > Self::RANGE.start ||
                value < Self::RANGE.end);
        Self(value)
    }
    pub const fn const_new(value: i16) -> Self {
        assert!(value > Self::RANGE.start ||
                value < Self::RANGE.end);
        Self(value)
    }
    pub fn get(self) -> i16 {
        self.0
    }
    pub fn calc<V: Num + NumCast>(self, all: V) -> V {
        all * NumCast::from(self.0).unwrap() / NumCast::from(100).unwrap()
}   }
impl PartialEq<i16> for Percent {
    fn eq(&self, other: &i16) -> bool {
        self.0 == *other
}   }
impl PartialOrd<i16> for Percent {
    fn partial_cmp(&self, other: &i16) -> Option<Ordering> {
        Some(self.0.cmp(other))
}   }
impl Sub<Percent> for Percent {
    type Output = Percent;
    fn sub(self, rhs: Percent) -> Self::Output {
        let mut result = self.0 - rhs.get();
        if result < Self::RANGE.start {
            result = Self::RANGE.start;
        } else if result > Self::RANGE.end {
            result = Self::RANGE.end;
        }
        Percent::new(result)
}   }
impl Add<Percent> for Percent {
    type Output = Percent;
    fn add(self, rhs: Percent) -> Self::Output {
        let mut result = self.0 + rhs.get();
        if result < Self::RANGE.start {
            result = Self::RANGE.start;
        } else if result > Self::RANGE.end {
            result = Self::RANGE.end;
        }
        Percent::new(result)
}   }
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
                    self - _rhs.calc(self)
            }   }
            impl Sub<Percent> for $t {
                type Output = Self;
                fn sub(self, _rhs: Percent) -> Self::Output {
                    self - _rhs.calc(self)
            }   }
            impl AddAssign<Percent> for $t {
                fn add_assign(&mut self, _rhs: Percent) {
                    *self = *self - _rhs.calc(*self);
            }   }
            impl SubAssign<Percent> for $t {
                fn sub_assign(&mut self, _rhs: Percent) {
                    *self = *self - _rhs.calc(*self);
            }   }
        )*
}   }
percent_ops_impl![i8, i16, i32, i64, i128, u8, u16, u32, u64, u128, usize, isize];
percent_float_ops_impl![f32, f64];


pub fn add_if_nat<T: AddAssign + PartialOrd + From<u8>>(value: &mut T, amount: T){
    *value+=nat(amount);
}
fn nat<T: PartialOrd + From<u8>>(a: T) -> T {
    if a >= T::from(0) {
        return a;
    }
    0.into()
}

pub fn percent<T>(value: &T, percent: T) -> T where T: Div<u64, Output=T> + Mul<Output=T> + Copy{
    *value / 100_u64 * percent
}

