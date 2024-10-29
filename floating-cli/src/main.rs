use anyhow;
use floating::*;
use half::f16;
use num_bigint::BigUint;
use std::cmp::min;
use std::env::args;

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
