use crate::{extract, pack, FloatType};
use num_bigint::ToBigUint;

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
            // case 1.2: inf/nan + inf/nan
            if man_a == zero {
                // inf
                (sign_b, exp_b, man_b)
            } else {
                // nan
                (sign_a, exp_a, man_a)
            }
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
    fn test_add() {
        for (a, b) in vec![
            // normal + normal
            (1.0, 1.1),
            (1.0, 2.0),
            (0.1, 0.2),
            // subnormal/zero + normal
            (0.0, 0.1),
            (1.0 / 1.5E+308, 0.1),
            // subnormal/zero + subnormal/zero
            (1.0 / 1.5E+308, 1.0 / 1.0E+308),
            (0.0, 1.0 / 1.0E+308),
            // inf/nan + inf/nan
            (f64::INFINITY, f64::NAN),
            (f64::NAN, f64::NAN),
            (f64::INFINITY, f64::INFINITY),
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
            assert_eq!(c.to_bits(), soft_c.to_bits());
        }
    }
}
