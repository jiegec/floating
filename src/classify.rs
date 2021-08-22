use num_bigint::ToBigUint;
use std::num::FpCategory;

use crate::{extract, FloatType};

pub fn softfloat_classify<T: FloatType>(a: T) -> FpCategory {
    let zero = 0.to_biguint().unwrap();
    let num_a = a.to_biguint();
    let (_sign_a, exp_a, man_a) = extract::<T>(&num_a);
    if exp_a == zero && man_a == zero {
        FpCategory::Zero
    } else if exp_a == zero && man_a != zero {
        FpCategory::Subnormal
    } else if exp_a == T::max_exp() && man_a == zero {
        FpCategory::Infinite
    } else if exp_a == T::max_exp() && man_a != zero {
        FpCategory::Nan
    } else {
        FpCategory::Normal
    }
}

#[cfg(test)]
mod tests {
    use crate::softfloat_classify;

    #[test]
    fn test() {
        for a in vec![
            1.0,
            0.1,
            0.2,
            1.0 / 1.5E+308,
            1.0 / 1.0E+308,
            f64::INFINITY,
            f64::NAN,
        ] {
            let cat = a.classify();
            let soft_cat = softfloat_classify(a);
            assert_eq!(cat, soft_cat);
        }
    }
}
