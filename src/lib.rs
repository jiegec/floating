use std::fmt::Display;

use num_bigint::{BigUint, ToBigUint};

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

pub fn softfloat_add<T: FloatType>(a: T, b: T) -> T {
    let zero = 0.to_biguint().unwrap();
    let one = 1.to_biguint().unwrap();
    let two = 2.to_biguint().unwrap();
    let three = 3.to_biguint().unwrap();
    let norm_bit = &one << (T::SIG - 1);

    let num_a = a.to_biguint();
    let (sign_a, exp_a, man_a) = extract::<T>(&num_a);
    let num_b = b.to_biguint();
    let (sign_b, exp_b, man_b) = extract::<T>(&num_b);

    if exp_a < exp_b {
        return softfloat_add(b, a);
    }

    // now exp_a >= exp_b
    let exp_diff = (&exp_a - &exp_b).to_u64_digits().pop().unwrap_or(0);
    let (sign_c, exp_c, man_c) = if exp_diff == 0 {
        // case 1: exponent equals
        if exp_a == zero {
            // case 1.1: subnormal/zero + subnormal/zero
            let sign_c = sign_a;
            let exp_c = zero;
            let man_c = &man_a + &man_b;
            (sign_c, exp_c, man_c)
        } else if exp_a == T::max_exp() {
            // case 1.2: Inf/NaN + Inf/NaN
            todo!()
        } else {
            // case 1.3: normal + normal
            // add implicit 1.0
            let norm_a = man_a + &norm_bit;
            let norm_b = man_b + &norm_bit;

            let sign_c = sign_a;
            let exp_c = exp_a + &one;
            let mut man_c = norm_a + norm_b;

            // normalize and rounding to nearest even
            if (&man_c & &three) == three {
                man_c = man_c + two;
            }
            man_c = man_c >> 1;
            man_c = man_c - norm_bit;
            (sign_c, exp_c, man_c)
        }
    } else {
        // case: exponent differs
        let mut norm_a = man_a + &norm_bit;
        let mut norm_b = man_b + &norm_bit;
        // pre left shift by one for rounding
        norm_a = norm_a << 1;
        norm_b = norm_b << 1;

        let mut exp_c = exp_a;
        let norm_b_shifted = norm_b >> exp_diff;
        let mut man_c = norm_a + norm_b_shifted;

        if man_c > &norm_bit << 2 {
            exp_c = exp_c + &one;
            man_c = man_c >> 1;
        }

        // round to nearest even
        // ....1 1
        // ->
        // ....1+1
        if (&man_c & &three) == three {
            man_c = man_c + two;
        }
        // remove pre shifted bit
        man_c = man_c >> 1;

        man_c = man_c - &norm_bit;

        let sign_c = sign_a;
        (sign_c, exp_c, man_c)
    };
    T::from_biguint(&pack::<T>(&sign_c, &exp_c, &man_c))
}
#[cfg(test)]
mod tests {
    use crate::{print_float, softfloat_add, FloatType};

    #[test]
    fn test() {
        for (a, b) in vec![
            (1.0, 1.1),
            (1.0, 2.0),
            (0.1, 0.2),
            (0.0, 0.1),
            (1.0 / 1.5E+308, 1.0 / 1.0E+308),
            (f64::INFINITY, f64::NAN),
        ] {
            let c = a + b;
            let soft_c = softfloat_add(a, b);
            println!("a={}({})", a, print_float::<f64>(&a.to_biguint()));
            println!("b={}({})", b, print_float::<f64>(&b.to_biguint()));
            println!("a+b={}({})", c, print_float::<f64>(&c.to_biguint()));
            println!(
                "soft a+b={}({})",
                soft_c,
                print_float::<f64>(&soft_c.to_biguint())
            );
            assert_eq!(c, soft_c);
        }
    }
}
