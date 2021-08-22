use std::fmt::Display;

use num_bigint::{BigUint, ToBigUint};

pub trait FloatType: Display + Copy + Clone {
    const EXP: usize;
    const SIG: usize;
    const WIDTH: usize = Self::EXP + Self::SIG;
    const NAME: &'static str;
    fn to_bits(self) -> BigUint;
    fn from_bits(num: &BigUint) -> Self;
    fn bias() -> BigUint {
        (1.to_biguint().unwrap() << (Self::EXP - 1)) - 1.to_biguint().unwrap()
    }
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
    (sign << (T::WIDTH - 1)) + (exp << (T::SIG - 1)) + man
}

pub fn softfloat_add<T: FloatType>(a: T, b: T) -> T {
    let one = 1.to_biguint().unwrap();
    let num_a = a.to_bits();
    let (sign_a, exp_a, man_a) = extract::<T>(&num_a);
    let num_b = b.to_bits();
    let (sign_b, exp_b, man_b) = extract::<T>(&num_b);

    if exp_a < exp_b {
        return softfloat_add(b, a);
    }
    // now exp_a >= exp_b

    // assume normalized
    let norm_bit = &one << (T::SIG - 1);
    let norm_a = man_a + &norm_bit;
    let norm_b = man_b + &norm_bit;

    let exp_diff = (&exp_a - exp_b).to_u64_digits()[0];
    let mut exp_c = exp_a;
    let norm_b_shifted = norm_b >> exp_diff;
    let mut man_c = norm_a + norm_b_shifted;
    if man_c > &norm_bit << 1 {
        exp_c = exp_c + &one;
        man_c = man_c >> 1;
    }
    man_c = man_c - &norm_bit;

    // TODO
    assert!(sign_a == sign_b);
    let sign_c = &sign_a;

    T::from_bits(&pack::<T>(sign_c, &exp_c, &man_c))
}

#[cfg(test)]
mod tests {
    use crate::softfloat_add;

    #[test]
    fn test() {
        assert_eq!(3.0, softfloat_add(1.0, 2.0));
    }
}
