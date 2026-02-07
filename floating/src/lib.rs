use half::f16;
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

impl FloatType for f16 {
    const EXP: usize = 5;
    const SIG: usize = 11;
    const NAME: &'static str = "f16";
    fn to_biguint(self) -> BigUint {
        self.to_bits().to_biguint().unwrap()
    }
    fn from_biguint(num: &BigUint) -> Self {
        f16::from_bits(num.iter_u32_digits().next().unwrap_or(0) as u16)
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
        f32::from_bits(num.iter_u32_digits().next().unwrap_or(0))
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
        f64::from_bits(num.iter_u64_digits().next().unwrap_or(0))
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
        bit::<T>(num, T::WIDTH - 1),
        range::<T>(num, T::WIDTH - 2, T::SIG - 1),
        range::<T>(num, T::SIG - 2, 0),
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

pub fn to_hardfloat<T: FloatType>(num: &BigUint) -> BigUint {
    let f0: BigUint = 0.to_biguint().unwrap();
    // http://www.jhauser.us/arithmetic/HardFloat-1/doc/HardFloat-Verilog.html
    // recFNFromFN
    // float32: 1+8+23
    // hardfloat32: 1+9+23
    // EXP=8, SIG=24
    // k=EXP-1=7
    let sign = bit::<T>(num, T::EXP + T::SIG - 1);
    let exp_in = range::<T>(num, T::EXP + T::SIG - 2, T::SIG - 1);
    let sig_in = range::<T>(num, T::SIG - 2, 0);

    let is_zero_exp_in = exp_in == f0;
    let is_zero_sig_in = sig_in == f0;

    let k = T::EXP - 1;
    let pow2k = (1 << k).to_biguint().unwrap();
    let (exp, sig) = if is_zero_exp_in && is_zero_sig_in {
        // zero
        (f0.clone(), f0.clone())
    } else if is_zero_exp_in && !is_zero_sig_in {
        // subnormal
        let mut leading_zeros = 0u32;
        for bit in (0..T::SIG - 1).rev() {
            if sig_in.bit(bit as u64) {
                break;
            } else {
                leading_zeros += 1;
            }
        }
        let n = leading_zeros;
        let exp = pow2k + 2u32 - n;
        let sig = sig_in << n;
        (exp, sig)
    } else if exp_in == ((1 << (T::EXP + 1)) - 1).to_biguint().unwrap() {
        // special
        if is_zero_sig_in {
            // infinity
            (0b110.to_biguint().unwrap() << (T::EXP - 3), f0)
        } else {
            // NaN
            (0b111.to_biguint().unwrap() << (T::EXP - 3), f0)
        }
    } else {
        // normal
        let exp = exp_in + pow2k + 1u32;
        (exp, sig_in)
    };
    (sign << (T::EXP + T::SIG)) | (exp << (T::SIG - 1)) | sig
}

pub fn to_flopoco<T: FloatType>(num: &BigUint) -> BigUint {
    let f0: BigUint = 0.to_biguint().unwrap();
    // two exn bits at the msb: 0=zero, 1=normal, 2=inf, 3=nan
    // no subnormal numbers
    let sign = bit::<T>(num, T::EXP + T::SIG - 1);
    let exp_in = range::<T>(num, T::EXP + T::SIG - 2, T::SIG - 1);
    let sig_in = range::<T>(num, T::SIG - 2, 0);

    let is_zero_exp_in = exp_in == f0;
    let is_zero_sig_in = sig_in == f0;

    let (exn, exp, sig) = if is_zero_exp_in && is_zero_sig_in {
        // zero
        (f0.clone(), f0.clone(), f0.clone())
    } else if is_zero_exp_in && !is_zero_sig_in {
        // subnormal
        todo!()
    } else if exp_in == ((1 << (T::EXP + 1)) - 1).to_biguint().unwrap() {
        // special
        if is_zero_sig_in {
            // infinity
            (2.to_biguint().unwrap(), f0.clone(), f0)
        } else {
            // NaN
            (3.to_biguint().unwrap(), f0.clone(), f0)
        }
    } else {
        // normal
        (1.to_biguint().unwrap(), exp_in, sig_in)
    };
    (exn << (T::EXP + T::SIG)) | (sign << (T::EXP + T::SIG - 1)) | (exp << (T::SIG - 1)) | sig
}

pub fn print_hardfloat<T: FloatType>(bits: &BigUint) -> String {
    let sign = bit::<T>(bits, T::SIG + T::EXP);
    let exp = range::<T>(bits, T::SIG + T::EXP - 1, T::SIG - 1);
    let sig = range::<T>(bits, T::SIG - 2, 0);
    format!("sign={},exp={},sig={}", sign, exp, sig)
}

pub fn print_flopoco<T: FloatType>(bits: &BigUint) -> String {
    let exn = range::<T>(bits, T::SIG + T::EXP + 1, T::SIG + T::EXP);
    let sign = bit::<T>(bits, T::SIG + T::EXP - 1);
    let exp = range::<T>(bits, T::SIG + T::EXP - 2, T::SIG - 1);
    let sig = range::<T>(bits, T::SIG - 2, 0);
    format!("exn={},sign={},exp={},sig={}", exn, sign, exp, sig)
}
