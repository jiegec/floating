use crate::{extract, pack, FloatType};
use num_bigint::{BigUint, ToBigUint};

// round to nearest even with 3 bits: guard, round and sticky
fn round(man: &BigUint) -> BigUint {
    let one = 1.to_biguint().unwrap();

    let low_bits = man.to_u64_digits().pop().unwrap_or(0) & 0b111;
    let mut res: BigUint = man >> 3;
    if low_bits < 0b100 {
        // round down
    } else if low_bits > 0b100 {
        // round up
        res += &one;
    } else {
        // round to nearest even
        if res.bit(0) {
            // up
            res += &one;
        }
    }
    res
}

// right shift with the LSB sticky
// sticky bit: reduced OR of shifted-away bits
fn rshift_sticky(man: &BigUint, shift: u64) -> BigUint {
    let zero = 0.to_biguint().unwrap();
    let one = 1.to_biguint().unwrap();

    if (man & ((&one << shift) - &one)) != zero {
        (man >> shift) | one
    } else {
        man >> shift
    }
}

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
                man_c += two;
            }
            man_c >>= 1;
            man_c -= norm_bit;
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

            // pre left shift 3 bits for rounding
            norm_a <<= 3;
            norm_b <<= 3;

            let mut exp_c = if exp_a > exp_b {
                // exp_a > exp_b
                let exp_diff = (&exp_a - &exp_b).to_u64_digits().pop().unwrap_or(0);
                if exp_b != zero {
                    // add implicit 1.0
                    norm_b += &norm_bit << 3;
                }

                // align with sticky bit
                norm_b = rshift_sticky(&norm_b, exp_diff);
                exp_a
            } else {
                // exp_a < exp_b
                let exp_diff = (&exp_b - &exp_a).to_u64_digits().pop().unwrap_or(0);
                if exp_a != zero {
                    // add implicit 1.0
                    norm_a += &norm_bit << 3;
                }

                // align with sticky bit
                norm_a = rshift_sticky(&norm_a, exp_diff);
                exp_b
            };

            // the bigger one is always normal
            let mut man_c = norm_a + norm_b + (&norm_bit << 3);

            if man_c >= &norm_bit << 4 {
                exp_c += &one;
                man_c >>= 1;
            }

            // rounding and remove pre shifted bits
            man_c = round(&man_c);

            man_c -= &norm_bit;

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
                man_c <<= shift;
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
                man_c <<= shift;
                man_c -= norm_bit;
                (sign_c, exp_c, man_c)
            } else {
                // |a| == |b|
                // res = +0 if rounding mode is not roundTowardNegative
                let sign_c = zero.clone();
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
            // pre shift 3 bits for rounding
            let mut norm_a = if exp_a == zero {
                // subnormal
                man_a.clone()
            } else {
                // normal
                &man_a + &norm_bit
            };
            norm_a <<= 3;
            let mut norm_b = if exp_b == zero {
                // subnormal
                man_b.clone()
            } else {
                // normal
                &man_b + &norm_bit
            };
            norm_b <<= 3;

            if exp_a > exp_b {
                // |a| > |b|
                let sign_c = sign_a;

                // right shift with sticky bit
                let exp_diff = (&exp_a - &exp_b).to_u64_digits().pop().unwrap_or(0);
                let norm_b = rshift_sticky(&norm_b, exp_diff);
                let mut man_c = &norm_a - &norm_b;

                let man_diff = man_c.to_u64_digits()[0];
                // shift=1 when clz=9([63:55])
                let shift = man_diff.leading_zeros() + 3 - (64 - T::SIG) as u32;
                man_c <<= shift;
                let exp_c = &exp_a - shift;
                man_c -= &norm_bit << 3;

                // round pre shifted 3 bits
                man_c = round(&man_c);

                (sign_c, exp_c, man_c)
            } else {
                // |a| < |b|
                let sign_c = &one - sign_a;

                // right shift with sticky bit
                let exp_diff = (&exp_b - &exp_a).to_u64_digits().pop().unwrap_or(0);
                let norm_a = rshift_sticky(&norm_a, exp_diff);
                let mut man_c = &norm_b - &norm_a;

                let man_diff = man_c.to_u64_digits()[0];
                // shift=1 when clz=9([63:55])
                let shift = man_diff.leading_zeros() + 3 - (64 - T::SIG) as u32;
                man_c <<= shift;
                let exp_c = &exp_b - shift;
                man_c -= &norm_bit << 3;

                // round pre shifted 3 bits
                man_c = round(&man_c);

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
            (4503599627370496.0, 0.4),
            (4503599627370496.0, 0.5),
            (4503599627370496.0, 0.6),
            // (1.5E+308, 1.5E+308),
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

            let a_plus_b = a + b;
            let soft_a_plus_b = softfloat_add(a, b);
            println!(
                "a+b={}({})",
                a_plus_b,
                print_float::<f64>(&a_plus_b.to_biguint())
            );
            println!(
                "soft a+b={}({})",
                soft_a_plus_b,
                print_float::<f64>(&soft_a_plus_b.to_biguint())
            );
            assert_eq!(a_plus_b.to_bits(), soft_a_plus_b.to_bits());

            let b_plus_a = b + a;
            let soft_b_plus_a = softfloat_add(b, a);
            println!(
                "b+a={}({})",
                b_plus_a,
                print_float::<f64>(&b_plus_a.to_biguint())
            );
            println!(
                "soft b+a={}({})",
                soft_b_plus_a,
                print_float::<f64>(&soft_b_plus_a.to_biguint())
            );
            assert_eq!(b_plus_a.to_bits(), soft_b_plus_a.to_bits());

            let a_minus_b = a - b;
            let soft_a_minus_b = softfloat_sub(a, b);
            println!(
                "a-b={}({})",
                a_minus_b,
                print_float::<f64>(&a_minus_b.to_biguint())
            );
            println!(
                "soft a-b={}({})",
                soft_a_minus_b,
                print_float::<f64>(&soft_a_minus_b.to_biguint())
            );
            assert_eq!(a_minus_b.to_bits(), soft_a_minus_b.to_bits());

            let b_minus_a = b - a;
            let soft_b_minus_a = softfloat_sub(b, a);
            println!(
                "b-a={}({})",
                b_minus_a,
                print_float::<f64>(&b_minus_a.to_biguint())
            );
            println!(
                "soft b-a={}({})",
                soft_b_minus_a,
                print_float::<f64>(&soft_b_minus_a.to_biguint())
            );
            assert_eq!(b_minus_a.to_bits(), soft_b_minus_a.to_bits());
        }
    }
}
