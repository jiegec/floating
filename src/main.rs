use anyhow;
use floating::{bit, print_float, range, FloatType};
use half::f16;
use num_bigint::{BigUint, ToBigUint};
use std::cmp::min;
use std::env::args;

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

fn print_hardfloat<T: FloatType>(bits: &BigUint) -> String {
    let sign = bit::<T>(bits, T::SIG + T::EXP);
    let exp = range::<T>(bits, T::SIG + T::EXP - 1, T::SIG - 1);
    let sig = range::<T>(bits, T::SIG - 2, 0);
    format!("sign={},exp={},sig={}", sign, exp, sig)
}

fn to_flopoco<T: FloatType>(num: &BigUint) -> BigUint {
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

fn print_flopoco<T: FloatType>(bits: &BigUint) -> String {
    let exn = range::<T>(bits, T::SIG + T::EXP + 1, T::SIG + T::EXP);
    let sign = bit::<T>(bits, T::SIG + T::EXP - 1);
    let exp = range::<T>(bits, T::SIG + T::EXP - 2, T::SIG - 1);
    let sig = range::<T>(bits, T::SIG - 2, 0);
    format!("exn={},sign={},exp={},sig={}", exn, sign, exp, sig)
}

fn float_to_hex_inner<T: FloatType>(num: T) {
    let bits = num.to_biguint();
    let hardfloat = to_hardfloat::<T>(&bits);
    let flopoco = to_flopoco::<T>(&bits);
    println!("    {}: {:#x}({})", T::NAME, bits, print_float::<T>(&bits));
    println!(
        "    h{}: {:#x}({})",
        T::NAME,
        hardfloat,
        print_hardfloat::<T>(&hardfloat)
    );
    println!(
        "    fpc{}: {:#x}({})",
        T::NAME,
        flopoco,
        print_flopoco::<T>(&flopoco)
    );
}

fn float_to_hex(num: f64) {
    println!("  float -> hex:");
    float_to_hex_inner::<f16>(f16::from_f64(num));
    float_to_hex_inner::<f32>(num as f32);
    float_to_hex_inner::<f64>(num as f64);
}

fn hex_to_float_inner<T: FloatType>(num: &BigUint) {
    let num_bits = num.bits() as usize;
    let mut offset = 0;
    let mut numbers = vec![];
    while offset < num_bits {
        numbers.push(T::from_biguint(&range::<T>(
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
    hex_to_float_inner::<f16>(&num);
    hex_to_float_inner::<f32>(&num);
    hex_to_float_inner::<f64>(&num);
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
