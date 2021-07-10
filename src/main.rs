use anyhow;
use std::env::args;

fn float_to_hex(num: f64) {
    println!("  float -> hex:");
    println!("    f32: {:#x}", (num as f32).to_bits());
    println!("    f64: {:#x}", (num).to_bits());
}

fn hex_to_float(num: u64) {
    println!("  hex -> float:");
    println!("    hex: {:#x}", num);
    println!("    f64: {}", f64::from_bits(num));
    println!(
        "  f32: {}, {}",
        f32::from_bits((num >> 32) as u32),
        f32::from_bits(num as u32)
    );
}

fn main() -> anyhow::Result<()> {
    for arg in args().skip(1) {
        println!("{}:", arg);
        if arg.starts_with("0x") {
            let s = arg.trim_start_matches("0x");
            let num = u64::from_str_radix(s, 16)?;
            hex_to_float(num);
        } else {
            if let Ok(num) = arg.parse::<u64>() {
                hex_to_float(num);
                float_to_hex(num as f64);
            } else {
                let num = arg.parse::<f64>()?;
                float_to_hex(num);
            }
        };
    }
    Ok(())
}
