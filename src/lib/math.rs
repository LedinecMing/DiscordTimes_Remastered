use {
    std::ops::{Add, AddAssign, Div, Mul}
};

pub fn add_if_nat<T: AddAssign + PartialOrd + From<u8>>(value: &mut T, amount: T){
    if value > &mut T::from(0) {
        *value+=amount;
}   }
fn nat<T: PartialOrd + From<u8>>(a: T) -> T {
    if a >= T::from(0) {
        return a;
    }
    0.into()
}

pub fn percent<T>(value: &T, percent: T) -> T where T: Div<u64, Output=T> + Mul<Output=T> + Copy{
    *value / 100_u64 * percent
}

