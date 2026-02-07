use floating::*;
use half::f16;
use num_bigint::BigUint;
use std::cmp::min;

fn float_to_hex_inner<T: FloatType, W: std::io::Write>(w: &mut W, num: T) -> anyhow::Result<()> {
    let bits = num.to_biguint();
    let hardfloat = to_hardfloat::<T>(&bits);
    let flopoco = to_flopoco::<T>(&bits);
    writeln!(
        w,
        "    {}: {:#x}({})",
        T::NAME,
        bits,
        print_float::<T>(&bits)
    )?;
    writeln!(
        w,
        "    h{}: {:#x}({})",
        T::NAME,
        hardfloat,
        print_hardfloat::<T>(&hardfloat)
    )?;
    writeln!(
        w,
        "    fpc{}: {:#x}({})",
        T::NAME,
        flopoco,
        print_flopoco::<T>(&flopoco)
    )?;
    Ok(())
}

fn float_to_hex<W: std::io::Write>(w: &mut W, num: f64) -> anyhow::Result<()> {
    writeln!(w, "  float -> hex:")?;
    float_to_hex_inner::<f16, W>(w, f16::from_f64(num))?;
    float_to_hex_inner::<f32, W>(w, num as f32)?;
    float_to_hex_inner::<f64, W>(w, num)?;
    Ok(())
}

fn hex_to_float_inner<T: FloatType, W: std::io::Write>(
    w: &mut W,
    num: &BigUint,
) -> anyhow::Result<()> {
    let num_bits = num.bits() as usize;
    let mut offset = 0;
    let mut numbers = vec![];
    while offset < num_bits {
        numbers.push(T::from_biguint(&range::<T>(
            num,
            min(offset + T::WIDTH, num_bits - 1),
            offset,
        )));
        offset += T::WIDTH;
    }

    write!(w, "    {}:", T::NAME)?;
    for num in numbers.iter().rev() {
        write!(w, " {}", num)?;
    }
    writeln!(w)?;
    Ok(())
}

fn hex_to_float<T: std::io::Write>(w: &mut T, num: &BigUint) -> anyhow::Result<()> {
    writeln!(w, "  hex -> float:")?;
    writeln!(w, "    hex: {:#x}", num)?;
    hex_to_float_inner::<f16, T>(w, num)?;
    hex_to_float_inner::<f32, T>(w, num)?;
    hex_to_float_inner::<f64, T>(w, num)?;
    Ok(())
}

pub fn process_arg<T: std::io::Write>(w: &mut T, arg: &str) -> anyhow::Result<()> {
    writeln!(w, "{}:", arg)?;
    if arg.starts_with("0x") {
        let s = arg.trim_start_matches("0x");
        if let Some(num) = BigUint::parse_bytes(s.as_bytes(), 16) {
            hex_to_float(w, &num)?;
        }
    } else if let Ok(num) = arg.parse::<u64>() {
        if let Some(num) = BigUint::parse_bytes(arg.as_bytes(), 10) {
            hex_to_float(w, &num)?;
        }
        float_to_hex(w, num as f64)?;
    } else {
        let num = arg.parse::<f64>()?;
        float_to_hex(w, num)?;
    };
    Ok(())
}
