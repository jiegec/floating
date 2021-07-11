use anyhow;
use num_bigint::{BigUint, ToBigUint};
use std::cmp::min;
use std::env::args;
use std::fmt::Display;

trait FloatType: Display {
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

fn range<T: FloatType>(num: &BigUint, upper: usize, lower: usize) -> BigUint {
    assert!(upper >= lower);
    (num >> lower) & ((1.to_biguint().unwrap() << (upper - lower + 1)) - 1u32)
}

fn bit<T: FloatType>(num: &BigUint, idx: usize) -> BigUint {
    (num >> idx) & 1.to_biguint().unwrap()
}

fn to_hardfloat<T: FloatType>(num: &BigUint) -> BigUint {
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

    let is_zero_exp_in = exp_in == 0.to_biguint().unwrap();
    let is_zero_sig_in = sig_in == 0.to_biguint().unwrap();

    let k = T::EXP - 1;
    let pow2k = (1 << k).to_biguint().unwrap();
    let (exp, sig) = if is_zero_exp_in && is_zero_sig_in {
        // zeros
        (f0.clone(), f0.clone())
    } else if is_zero_exp_in && !is_zero_sig_in {
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

fn print_float<T: FloatType>(bits: &BigUint) -> String {
    let sign = bit::<T>(bits, T::SIG + T::EXP - 1);
    let exp = range::<T>(bits, T::SIG + T::EXP - 2, T::SIG - 1);
    let sig = range::<T>(bits, T::SIG - 2, 0);
    format!("sign={},exp={},sig={}", sign, exp, sig)
}

fn print_hardfloat<T: FloatType>(bits: &BigUint) -> String {
    let sign = bit::<T>(bits, T::SIG + T::EXP);
    let exp = range::<T>(bits, T::SIG + T::EXP - 1, T::SIG - 1);
    let sig = range::<T>(bits, T::SIG - 2, 0);
    format!("sign={},exp={},sig={}", sign, exp, sig)
}

fn float_to_hex_inner<T: FloatType>(num: T) {
    let bits = num.to_bits();
    let hardfloat = to_hardfloat::<T>(&bits);
    println!("    {}: {:#x}({})", T::NAME, bits, print_float::<T>(&bits));
    println!(
        "    h{}: {:#x}({})",
        T::NAME,
        hardfloat,
        print_hardfloat::<T>(&hardfloat)
    );
}

fn float_to_hex(num: f64) {
    println!("  float -> hex:");
    float_to_hex_inner::<f32>(num as f32);
    float_to_hex_inner::<f64>(num as f64);
}

fn hex_to_float_inner<T: FloatType>(num: &BigUint) {
    let num_bits = num.bits() as usize;
    let mut offset = 0;
    let mut numbers = vec![];
    while offset < num_bits {
        numbers.push(T::from_bits(&range::<T>(
            &num,
            min(offset + T::WIDTH, num_bits - 1),
            offset,
        )));
        offset += T::WIDTH;
    }

    print!("    {}:", T::NAME);
    for num in numbers.iter().rev() {
        print!(" {}", num);
    }
    println!("");
}

fn hex_to_float(num: &BigUint) {
    println!("  hex -> float:");
    println!("    hex: {:#x}", num);
    hex_to_float_inner::<f64>(&num);
    hex_to_float_inner::<f32>(&num);
}

fn main() -> anyhow::Result<()> {
    for arg in args().skip(1) {
        println!("{}:", arg);
        if arg.starts_with("0x") {
            let s = arg.trim_start_matches("0x");
            if let Some(num) = BigUint::parse_bytes(s.as_bytes(), 16) {
                hex_to_float(&num);
            }
        } else {
            if let Ok(num) = arg.parse::<u64>() {
                if let Some(num) = BigUint::parse_bytes(arg.as_bytes(), 10) {
                    hex_to_float(&num);
                }
                float_to_hex(num as f64);
            } else {
                let num = arg.parse::<f64>()?;
                float_to_hex(num);
            }
        };
    }
    Ok(())
}