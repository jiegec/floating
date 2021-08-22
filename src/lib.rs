use std::fmt::Display;

use num_bigint::{BigUint, ToBigUint};

pub trait FloatType: Display {
    const EXP: usize;
    const SIG: usize;
    const WIDTH: usize = Self::EXP + Self::SIG;
    const NAME: &'static str;
    fn to_bits(self) -> BigUint;
    fn from_bits(num: &BigUint) -> Self;
}

impl FloatType for f32 {
    const EXP: usize = 8;
    const SIG: usize = 24;
    const NAME: &'static str = "f32";
    fn to_bits(self) -> BigUint {
        self.to_bits().to_biguint().unwrap()
    }
    fn from_bits(num: &BigUint) -> Self {
        f32::from_bits(num.iter_u32_digits().next().unwrap())
    }
}

impl FloatType for f64 {
    const EXP: usize = 11;
    const SIG: usize = 53;
    const NAME: &'static str = "f64";
    fn to_bits(self) -> BigUint {
        self.to_bits().to_biguint().unwrap()
    }
    fn from_bits(num: &BigUint) -> Self {
        f64::from_bits(num.iter_u64_digits().next().unwrap())
    }
}
