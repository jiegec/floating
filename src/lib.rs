use num_bigint::{BigUint, ToBigUint};
use std::fmt::Display;

mod add;
mod classify;

pub use add::*;
pub use classify::*;

pub trait FloatType: Display + Copy + Clone {
    const EXP: usize;
    const SIG: usize;
    const WIDTH: usize = Self::EXP + Self::SIG;
    const NAME: &'static str;
    fn to_biguint(self) -> BigUint;
    fn from_biguint(num: &BigUint) -> Self;
    fn bias() -> BigUint {
        (1.to_biguint().unwrap() << (Self::EXP - 1)) - 1.to_biguint().unwrap()
    }
    fn max_exp() -> BigUint {
        (1.to_biguint().unwrap() << (Self::EXP)) - 1.to_biguint().unwrap()
    }
}

impl FloatType for f32 {
    const EXP: usize = 8;
    const SIG: usize = 24;
    const NAME: &'static str = "f32";
    fn to_biguint(self) -> BigUint {
        self.to_bits().to_biguint().unwrap()
    }
    fn from_biguint(num: &BigUint) -> Self {
        f32::from_bits(num.iter_u32_digits().next().unwrap())
    }
}

impl FloatType for f64 {
    const EXP: usize = 11;
    const SIG: usize = 53;
    const NAME: &'static str = "f64";
    fn to_biguint(self) -> BigUint {
        self.to_bits().to_biguint().unwrap()
    }
    fn from_biguint(num: &BigUint) -> Self {
        f64::from_bits(num.iter_u64_digits().next().unwrap())
    }
}

pub fn range<T: FloatType>(num: &BigUint, upper: usize, lower: usize) -> BigUint {
    assert!(upper >= lower);
    (num >> lower) & ((1.to_biguint().unwrap() << (upper - lower + 1)) - 1u32)
}

pub fn bit<T: FloatType>(num: &BigUint, idx: usize) -> BigUint {
    (num >> idx) & 1.to_biguint().unwrap()
}

// extract (sign, exponent, mantissa)
pub fn extract<T: FloatType>(num: &BigUint) -> (BigUint, BigUint, BigUint) {
    (
        bit::<T>(&num, T::WIDTH - 1),
        range::<T>(&num, T::WIDTH - 2, T::SIG - 1),
        range::<T>(&num, T::SIG - 2, 0),
    )
}

pub fn pack<T: FloatType>(sign: &BigUint, exp: &BigUint, man: &BigUint) -> BigUint {
    // validate
    let one = 1.to_biguint().unwrap();
    assert!(sign < &(&one << 1));
    assert!(exp < &(&one << T::EXP));
    assert!(man < &(&one << (T::SIG - 1)));
    (sign << (T::WIDTH - 1)) + (exp << (T::SIG - 1)) + man
}

pub fn print_float<T: FloatType>(bits: &BigUint) -> String {
    let (sign, exp, man) = extract::<T>(bits);
    format!(
        "sign={},exp={},man={:0width$b}",
        sign,
        exp,
        man,
        width = T::SIG - 1
    )
}
