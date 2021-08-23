use crate::{extract, pack, FloatType};
use num_bigint::{BigUint, ToBigUint};

fn effective_add<T: FloatType>(
    sign_a: BigUint,
    exp_a: BigUint,
    man_a: BigUint,
    sign_b: BigUint,
    exp_b: BigUint,
    man_b: BigUint,
) -> T {
    let zero = 0.to_biguint().unwrap();
    let one = 1.to_biguint().unwrap();
    let two = 2.to_biguint().unwrap();
    let three = 3.to_biguint().unwrap();
    let norm_bit = &one << (T::SIG - 1);

    // now exp_a >= exp_b
    let (sign_c, exp_c, man_c) = if exp_a == exp_b {
        // case 1: exponent equals
        if exp_a == zero {
            // case 1.1: subnormal/zero + subnormal/zero
            // sum up mantissa
            let sign_c = sign_a;
            let exp_c = zero;
            let man_c = &man_a + &man_b;
            (sign_c, exp_c, man_c)
        } else if exp_a == T::max_exp() {
            // case 1.2: inf/nan + inf/nan
            // propagate nan
            if man_a != zero {
                // nan
                (sign_a, T::max_exp(), man_a)
            } else if man_b != zero {
                // nan
                (sign_b, T::max_exp(), man_b)
            } else {
                // inf
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
            // if the lowest two bits are 0b11
            // it should be rounded up
            if (&man_c & &three) == three {
                man_c = man_c + two;
            }
            man_c = man_c >> 1;
            man_c = man_c - norm_bit;
            (sign_c, exp_c, man_c)
        }
    } else {
        // case: exponent differs
        if exp_a == T::max_exp() {
            // inf/nan
            (sign_a, exp_a, man_a)
        } else if exp_b == T::max_exp() {
            // inf/nan
            (sign_b, exp_b, man_b)
        } else {
            let mut norm_a = man_a;
            let mut norm_b = man_b;

            // pre left shift by one for rounding
            norm_a = norm_a << 1;
            norm_b = norm_b << 1;

            let mut exp_c = if exp_a > exp_b {
                // exp_a > exp_b
                let exp_diff = (&exp_a - &exp_b).to_u64_digits().pop().unwrap_or(0);
                if exp_b != zero {
                    // add implicit 1.0
                    norm_b += &norm_bit << 1;
                }
                // align
                norm_b >>= exp_diff;
                exp_a
            } else {
                // exp_a < exp_b
                let exp_diff = (&exp_b - &exp_a).to_u64_digits().pop().unwrap_or(0);
                if exp_a != zero {
                    // add implicit 1.0
                    norm_a += &norm_bit << 1;
                }
                // align
                norm_a >>= exp_diff;
                exp_b
            };

            // the bigger one is always normal
            let mut man_c = norm_a + norm_b + (&norm_bit << 1);

            if man_c >= &norm_bit << 2 {
                exp_c = exp_c + &one;
                man_c = man_c >> 1;
            }

            // round to nearest even
            // round up when ....1 1
            if (&man_c & &three) == three {
                man_c = man_c + two;
            }
            // remove pre shifted bit
            man_c = man_c >> 1;

            man_c = man_c - &norm_bit;

            let sign_c = sign_a;
            (sign_c, exp_c, man_c)
        }
    };
    T::from_biguint(&pack::<T>(&sign_c, &exp_c, &man_c))
}

fn effective_sub<T: FloatType>(
    sign_a: BigUint,
    exp_a: BigUint,
    man_a: BigUint,
    sign_b: BigUint,
    exp_b: BigUint,
    man_b: BigUint,
) -> T {
    let zero = 0.to_biguint().unwrap();
    let one = 1.to_biguint().unwrap();
    let norm_bit = &one << (T::SIG - 1);

    let (sign_c, exp_c, man_c) = if exp_a == exp_b {
        // case 1: exponent equals
        if exp_a == zero {
            // case 1.1: subnormal/zero - subnormal/zero
            let exp_c = zero.clone();
            if man_a > man_b {
                // |a| > |b|
                let sign_c = sign_a;
                let man_c = &man_a - &man_b;
                (sign_c, exp_c, man_c)
            } else if man_a < man_b {
                // |a| < |b|
                let sign_c = &one - sign_a;
                let man_c = &man_b - &man_a;
                (sign_c, exp_c, man_c)
            } else {
                // |a| == |b|
                // +0
                let sign_c = zero.clone();
                let man_c = zero;
                (sign_c, exp_c, man_c)
            }
        } else if exp_a == T::max_exp() {
            // case 1.2: inf/nan - inf/nan
            if man_a != zero {
                // nan
                (sign_a, exp_a, man_a)
            } else if man_b != zero {
                // nan
                (sign_b, exp_b, man_b)
            } else {
                // inf - inf = nan
                // signaling
                (zero, T::max_exp(), one << (T::SIG - 2))
            }
        } else {
            // case 1.3: normal - normal
            if man_a < man_b {
                // |a| < |b|
                let sign_c = one - sign_a;
                let mut man_c = man_b - man_a;
                let man_diff = man_c.to_u64_digits()[0];
                // shift=0 when clz=11([63:53])
                let shift = man_diff.leading_zeros() - (64 - T::SIG) as u32;
                let exp_c = exp_a - shift;
                man_c = man_c << shift;
                man_c -= norm_bit;
                (sign_c, exp_c, man_c)
            } else if man_a > man_b {
                // |a| > |b|
                let sign_c = sign_a;
                let mut man_c = man_a - man_b;
                let man_diff = man_c.to_u64_digits()[0];
                // shift=0 when clz=11([63:53])
                let shift = man_diff.leading_zeros() - (64 - T::SIG) as u32;
                let exp_c = exp_a - shift;
                man_c = man_c << shift;
                man_c -= norm_bit;
                (sign_c, exp_c, man_c)
            } else {
                // |a| == |b|
                let sign_c = sign_a;
                (sign_c, zero.clone(), zero.clone())
            }
        }
    } else {
        // case 2: exponent differs
        if exp_a == T::max_exp() {
            // inf/nan
            (sign_a, exp_a, man_a)
        } else if exp_b == T::max_exp() {
            // inf/nan
            (sign_b, exp_b, man_b)
        } else {
            // pre shift for rounding
            let mut norm_a = if exp_a == zero {
                // subnormal
                man_a.clone()
            } else {
                // normal
                &man_a + &norm_bit
            };
            norm_a <<= 1;
            let mut norm_b = if exp_b == zero {
                // subnormal
                man_b.clone()
            } else {
                // normal
                &man_b + &norm_bit
            };
            norm_b <<= 1;

            if exp_a > exp_b {
                // |a| > |b|
                let sign_c = sign_a;

                let exp_diff = (&exp_a - &exp_b).to_u64_digits().pop().unwrap_or(0);
                let mut man_c = norm_a - (norm_b >> exp_diff);

                let man_diff = man_c.to_u64_digits()[0];
                // shift=1 when clz=11([63:53])
                let shift = man_diff.leading_zeros() + 1 - (64 - T::SIG) as u32;
                man_c = man_c << shift;
                let exp_c = &exp_a - shift;
                man_c = man_c - (&norm_bit << 1);

                // remove pre shifted bit
                man_c = man_c >> 1;

                (sign_c, exp_c, man_c)
            } else {
                // |a| < |b|
                let sign_c = &one - sign_a;
                let exp_diff = (&exp_b - &exp_a).to_u64_digits().pop().unwrap_or(0);
                let mut man_c = norm_b - (norm_a >> exp_diff);

                let man_diff = man_c.to_u64_digits()[0];
                // shift=1 when clz=11([63:53])
                let shift = man_diff.leading_zeros() + 1 - (64 - T::SIG) as u32;
                man_c = man_c << shift;
                let exp_c = &exp_b - shift;
                man_c = man_c - (&norm_bit << 1);

                // remove pre shifted bit
                man_c = man_c >> 1;

                (sign_c, exp_c, man_c)
            }
        }
    };
    T::from_biguint(&pack::<T>(&sign_c, &exp_c, &man_c))
}

pub fn softfloat_add<T: FloatType>(a: T, b: T) -> T {
    let one = 1.to_biguint().unwrap();
    let num_a = a.to_biguint();
    let (sign_a, exp_a, man_a) = extract::<T>(&num_a);
    let num_b = b.to_biguint();
    let (sign_b, exp_b, man_b) = extract::<T>(&num_b);
    if (&sign_a ^ &sign_b) == one {
        // sub
        effective_sub(sign_a, exp_a, man_a, sign_b, exp_b, man_b)
    } else {
        // add
        effective_add(sign_a, exp_a, man_a, sign_b, exp_b, man_b)
    }
}

pub fn softfloat_sub<T: FloatType>(a: T, b: T) -> T {
    let one = 1.to_biguint().unwrap();
    let num_a = a.to_biguint();
    let (sign_a, exp_a, man_a) = extract::<T>(&num_a);
    let num_b = b.to_biguint();
    let (sign_b, exp_b, man_b) = extract::<T>(&num_b);
    if (&sign_a ^ &sign_b) == one {
        // add
        effective_add(sign_a, exp_a, man_a, sign_b, exp_b, man_b)
    } else {
        // sub
        effective_sub(sign_a, exp_a, man_a, sign_b, exp_b, man_b)
    }
}

#[cfg(test)]
mod tests {
    use crate::{print_float, softfloat_add, softfloat_sub, FloatType};

    #[test]
    fn test() {
        for (a, b) in vec![
            // normal + normal
            (1.0, 1.1),
            (1.0, 2.0),
            (0.1, 0.2),
            (0.1, -0.2),
            (0.1, -0.1),
            // subnormal/zero + normal
            (0.0, 0.1),
            (1.0 / 1.5E+308, 0.1),
            // subnormal/zero + subnormal/zero
            (1.0 / 1.5E+308, 1.0 / 1.0E+308),
            (0.0, 1.0 / 1.0E+308),
            (0.0, 0.0),
            (-0.0, 0.0),
            // inf/nan + inf/nan
            (f64::INFINITY, f64::NAN),
            (f64::INFINITY, -f64::NAN),
            (f64::NAN, f64::NAN),
            (-f64::NAN, f64::NAN),
            (f64::INFINITY, f64::INFINITY),
            (-f64::INFINITY, -f64::INFINITY),
            (-f64::INFINITY, f64::INFINITY),
        ] {
            println!("a={}({})", a, print_float::<f64>(&a.to_biguint()));
            println!("b={}({})", b, print_float::<f64>(&b.to_biguint()));

            let aplusb = a + b;
            let soft_aplusb = softfloat_add(a, b);
            println!(
                "a+b={}({})",
                aplusb,
                print_float::<f64>(&aplusb.to_biguint())
            );
            println!(
                "soft a+b={}({})",
                soft_aplusb,
                print_float::<f64>(&soft_aplusb.to_biguint())
            );
            assert_eq!(aplusb.to_bits(), soft_aplusb.to_bits());

            let aminusb = a - b;
            let soft_aminusb = softfloat_sub(a, b);
            println!(
                "a-b={}({})",
                aminusb,
                print_float::<f64>(&aminusb.to_biguint())
            );
            println!(
                "soft a-b={}({})",
                soft_aminusb,
                print_float::<f64>(&soft_aminusb.to_biguint())
            );
            assert_eq!(aminusb.to_bits(), soft_aminusb.to_bits());
        }
    }
}
