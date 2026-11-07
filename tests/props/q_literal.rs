//! Property 9: a `Fixed` against a `q!` literal is the exact order of the two, never a rounded one. `t` steps of
//! `num_s / den_s` (the step in units of the literal) against `num · 10^exp`, compared in `i128` after multiplying out
//! the denominators: `<`, `<=` and `==` all agree with that order, for the fold a compare makes at compile time
//! (reached at run time through `literal_order`) and for `q!` itself, walked over every stored value.

use core::cmp::Ordering;
use exact_units::Fixed;
use exact_units::literal::Decimal;
use exact_units::prefix::{Centi, Kilo, Micro, Milli, One, Rational};
use exact_units::units::{Kilometre, Meter};

/// `t` steps of `num_s / den_s` against `num · 10^exp`, exactly; `None` where the products don't fit `i128`.
pub(crate) fn order(t: i128, num_s: i128, den_s: i128, num: i64, exp: i16) -> Option<Ordering> {
    let ten = 10i128.checked_pow(u32::from(exp.unsigned_abs()))?;
    let (left, right) = if exp >= 0 {
        (
            t.checked_mul(num_s)?,
            i128::from(num).checked_mul(ten)?.checked_mul(den_s)?,
        )
    } else {
        (
            t.checked_mul(num_s)?.checked_mul(ten)?,
            i128::from(num).checked_mul(den_s)?,
        )
    };
    Some(left.cmp(&right))
}

/// `[<, <=, ==]` as an order has them.
fn ops(o: Ordering) -> [bool; 3] {
    [o.is_lt(), o.is_le(), o.is_eq()]
}

macro_rules! orders {
    ($($name:ident: $ty:ty, $raw:ty, $s:ty, $num_s:expr, $den_s:expr;)*) => {$(
        /// The fold's `[<, <=, ==]` of stored value `t` against `num · 10^exp` of the literal's unit is the exact
        /// order, wherever the reference's products fit `i128`.
        pub(crate) fn $name(t: $raw, num: i64, exp: i16) {
            if let Some(want) = order(t.into(), $num_s, $den_s, num, exp) {
                let got = <$ty>::new(t).literal_order::<$s>(Decimal::new(num, exp));
                assert_eq!(got, ops(want), "{t} against {num}e{exp}");
            }
        }
    )*};
}

orders! {
    centi_i16: Fixed<Meter, Centi, i16>, i16, Meter, 1, 100;
    third_i16: Fixed<Meter, Rational<1, 3>, i16>, i16, Meter, 1, 3;
    kilo_u8: Fixed<Meter, Kilo, u8>, u8, Meter, 1000, 1;
    km_in_cm_i16: Fixed<Meter, Centi, i16>, i16, Kilometre, 1, 100_000;
    milli_i32: Fixed<Meter, Milli, i32>, i32, Meter, 1, 1000;
    micro_i64: Fixed<Meter, Micro, i64>, i64, Meter, 1, 1_000_000;
    one_u32: Fixed<Meter, One, u32>, u32, Meter, 1, 1;
}

#[cfg(not(any(kani, fuzzing)))]
mod tests {
    use super::*;
    use crate::strategy;
    use exact_units::q;
    use exact_units::units::{Degree, Hertz, Minute, Ohm, Per, Radian, Second, Speed, Times, Volt};
    use proptest::prelude::*;

    /// Exponents where the reference mostly fits, with a few past it.
    fn exp() -> impl Strategy<Value = i16> {
        prop_oneof![8 => -12i16..=6, 1 => -30i16..=20]
    }

    proptest! {
        #[test]
        fn wide_storages(t32 in strategy::i32(), t64 in strategy::i64(), u in any::<u32>(), num in strategy::i64(), exp in exp()) {
            milli_i32(t32, num, exp);
            micro_i64(t64, num, exp);
            one_u32(u, num, exp);
        }

        #[test]
        fn narrow_storages(t in strategy::i16(), u in any::<u8>(), num in strategy::i64(), exp in exp()) {
            centi_i16(t, num, exp);
            third_i16(t, num, exp);
            km_in_cm_i16(t, num, exp);
            kilo_u8(u, num, exp);
        }
    }

    /// Every operator of stored value `t` against a `q!` literal, both ways round, is order `ord`.
    macro_rules! agrees {
        ($t:expr, $lit:expr, $ord:expr) => {{
            let (t, lit, ord): (_, _, Ordering) = ($t, $lit, $ord);
            assert_eq!(t.partial_cmp(&lit), Some(ord), "{t:?}");
            assert_eq!(t < lit, ord.is_lt());
            assert_eq!(t <= lit, ord.is_le());
            assert_eq!(t > lit, ord.is_gt());
            assert_eq!(t >= lit, ord.is_ge());
            assert_eq!(t == lit, ord.is_eq());
            assert_eq!(t != lit, ord.is_ne());
            assert_eq!(lit.partial_cmp(&t), Some(ord.reverse()));
            assert_eq!(lit < t, ord.is_gt());
            assert_eq!(lit <= t, ord.is_ge());
            assert_eq!(lit > t, ord.is_lt());
            assert_eq!(lit >= t, ord.is_le());
            assert_eq!(lit == t, ord.is_eq());
        }};
    }

    /// The reference order of `t` steps of `num_s / den_s` against `num · 10^exp`.
    #[expect(
        clippy::panic,
        reason = "the walks choose literals whose reference fits i128; a panic fails the test"
    )]
    fn want(t: i16, num_s: i128, den_s: i128, num: i64, exp: i16) -> Ordering {
        order(t.into(), num_s, den_s, num, exp).unwrap_or_else(|| panic!("{t} against {num}e{exp}"))
    }

    /// `q!` itself over every i16 in steps of 1 cm/s, exact and inexact literals, prefixes, types and strings.
    #[test]
    fn every_i16_in_centimetres_per_second() {
        for t in i16::MIN..=i16::MAX {
            let speed = Fixed::<Speed, Centi, i16>::new(t);
            agrees!(speed, q!(100 m/s), want(t, 1, 100, 100, 0));
            agrees!(speed, q!(99.99 m/s), want(t, 1, 100, 9999, -2));
            agrees!(speed, q!(0.155 m/s), want(t, 1, 100, 155, -3));
            agrees!(speed, q!(-0.155 m/s), want(t, 1, 100, -155, -3));
            agrees!(speed, q!(99.995 m/s), want(t, 1, 100, 99995, -3));
            // 360 km/h is 100 m/s, 359.9 km/h 99.97.. m/s: in m/s, times 1000/3600
            agrees!(speed, q!(360 km/h), want(t, 36, 1000, 360, 0));
            agrees!(speed, q!(359.9 km/h), want(t, 36, 1000, 3599, -1));
            agrees!(speed, q!(100 Speed), want(t, 1, 100, 100, 0));
            agrees!(speed, q!(100 Per<Meter, Second>), want(t, 1, 100, 100, 0));
            agrees!(speed, q!("100 m/s"), want(t, 1, 100, 100, 0));
            agrees!(speed, q!(100 "m/s"), want(t, 1, 100, 100, 0));
            agrees!(speed, q!(6 m/Minute), want(t, 60, 100, 6, 0));
            // past i16's range: a constant answer
            agrees!(speed, q!(1000 m/s), Ordering::Less);
            agrees!(speed, q!(-1000 m/s), Ordering::Greater);
        }
    }

    #[test]
    fn every_i16_with_prefixes() {
        for t in i16::MIN..=i16::MAX {
            let v = Fixed::<Volt, Milli, i16>::new(t);
            agrees!(v, q!(3.3 V), want(t, 1, 1000, 33, -1));
            agrees!(v, q!(3300 mV), want(t, 1, 1000, 3300, -3));
            agrees!(v, q!(1250.5 mV), want(t, 1, 1000, 12505, -4));
            agrees!(v, q!(1500 μV), want(t, 1, 1000, 1500, -6));
            agrees!(v, q!(1500 uV), want(t, 1, 1000, 1500, -6));
            agrees!(v, q!("1500 µV"), want(t, 1, 1000, 1500, -6));
            agrees!(v, q!(0.5 A*Ω), want(t, 1, 1000, 5, -1));
            agrees!(v, q!(-12 V), want(t, 1, 1000, -12, 0));
            let f = Fixed::<Hertz, One, i16>::new(t);
            agrees!(f, q!(1.5 kHz), want(t, 1, 1, 15, 2));
            agrees!(f, q!(0.0015 MHz), want(t, 1, 1, 15, 2));
            agrees!(f, q!(1500 s^-1), want(t, 1, 1, 15, 2));
            agrees!(f, q!("1500 s⁻¹"), want(t, 1, 1, 15, 2));
            agrees!(f, q!(1.5 / ms), want(t, 1, 1, 15, 2));
            let r = Fixed::<Ohm, One, i16>::new(t);
            agrees!(r, q!(4.7 kΩ), want(t, 1, 1, 47, 2));
            let s = Fixed::<Second, Micro, i16>::new(t);
            agrees!(s, q!(1.5 ms), want(t, 1, 1_000_000, 15, -4));
            agrees!(s, q!(1500000 ns), want(t, 1, 1_000_000, 15, -4));
        }
    }

    #[test]
    fn every_i16_in_powers() {
        for t in i16::MIN..=i16::MAX {
            // m² in steps of 1 cm² (10⁻⁴ m²)
            let a = Fixed::<Times<Meter, Meter>, Rational<1, 10_000>, i16>::new(t);
            agrees!(a, q!(1.5 m^2), want(t, 1, 10_000, 15, -1));
            agrees!(a, q!(150 cm^2), want(t, 1, 10_000, 150, -4));
            agrees!(a, q!("150 cm²"), want(t, 1, 10_000, 150, -4));
            agrees!(a, q!(150 cm*cm), want(t, 1, 10_000, 150, -4));
            // m/s² in steps of 1 mm/s²
            let g = Fixed::<Per<Speed, Second>, Milli, i16>::new(t);
            agrees!(g, q!(9.81 m/s^2), want(t, 1, 1000, 981, -2));
            agrees!(g, q!(9.81 m/s/s), want(t, 1, 1000, 981, -2));
            agrees!(g, q!(9.81 m/(s*s)), want(t, 1, 1000, 981, -2));
            agrees!(g, q!("9.81 m/s²"), want(t, 1, 1000, 981, -2));
        }
    }

    /// Degrees against steps of a rational scale (0.1°) and of one with π in it (mrad): no rounding in either.
    #[test]
    fn every_i16_in_angles() {
        for t in i16::MIN..=i16::MAX {
            let a = Fixed::<Degree, Rational<1, 10>, i16>::new(t);
            agrees!(a, q!(90 deg), want(t, 1, 10, 90, 0));
            agrees!(a, q!(90 "°"), want(t, 1, 10, 90, 0));
            agrees!(a, q!(90.05 deg), want(t, 1, 10, 9005, -2));
            // 90° is 1570.796.. mrad: never equal, below from 1571 on
            let r = Fixed::<Radian, Milli, i16>::new(t);
            let ord = if t <= 1570 {
                Ordering::Less
            } else {
                Ordering::Greater
            };
            agrees!(r, q!(90 deg), ord);
            let ord = if t <= -1571 {
                Ordering::Less
            } else {
                Ordering::Greater
            };
            agrees!(r, q!(-90 deg), ord);
        }
    }

    /// Literals past the range of `i8` and `u8`: every stored value is below (or above) them.
    #[test]
    fn every_i8_and_u8_past_the_range() {
        for t in i8::MIN..=i8::MAX {
            let x = Fixed::<Meter, One, i8>::new(t);
            agrees!(x, q!(1000 m), Ordering::Less);
            agrees!(x, q!(-1000 m), Ordering::Greater);
            agrees!(x, q!(127.5 m), Ordering::Less);
            agrees!(x, q!(-128.5 m), Ordering::Greater);
            agrees!(x, q!(-128 m), t.cmp(&-128));
            agrees!(x, q!(1 km), Ordering::Less);
        }
        for t in u8::MIN..=u8::MAX {
            let x = Fixed::<Meter, One, u8>::new(t);
            agrees!(x, q!(-0.5 m), Ordering::Greater);
            agrees!(x, q!(0 m), t.cmp(&0));
            agrees!(x, q!(0.255 km), t.cmp(&255));
            agrees!(x, q!(256 m), Ordering::Less);
        }
    }

    /// Kilobytes and kibibytes against bytes.
    #[test]
    fn every_u16_in_bytes() {
        use exact_units::units::Byte;
        for t in u16::MIN..=u16::MAX {
            let b = Fixed::<Byte, One, u16>::new(t);
            agrees!(b, q!(4 KiB), t.cmp(&4096));
            agrees!(b, q!(4 kB), t.cmp(&4000));
            agrees!(b, q!(1.5 KiB), t.cmp(&1536));
            agrees!(b, q!(64 KiB), Ordering::Less);
        }
    }

    /// The fold `literal_order` is the one `q!` makes, over every i16, for literals of the property's types.
    #[test]
    fn literal_order_is_what_q_compares() {
        for t in i16::MIN..=i16::MAX {
            let x = Fixed::<Meter, Centi, i16>::new(t);
            for (lit, d) in [
                (
                    [x < q!(0.155 m), x <= q!(0.155 m), x == q!(0.155 m)],
                    Decimal::new(155, -3),
                ),
                (
                    [x < q!(-327.68 m), x <= q!(-327.68 m), x == q!(-327.68 m)],
                    Decimal::new(-32768, -2),
                ),
                (
                    [x < q!(2 km), x <= q!(2 km), x == q!(2 km)],
                    Decimal::new(2000, 0),
                ),
            ] {
                assert_eq!(lit, x.literal_order::<Meter>(d), "{t}");
            }
        }
    }
}

#[cfg(kani)]
mod proofs {
    use super::*;

    /// Every i16 stored in steps of 1 cm and of 1/3 m against every i8 of digits with a power of ten from 10⁻² to
    /// 10¹. The fold's gcd loops in `u128` bound the size, as for property 7.
    #[kani::proof]
    #[kani::unwind(40)]
    fn centi_and_third_i16() {
        let exp: i16 = kani::any_where(|e| (-2..=1).contains(e));
        let num: i8 = kani::any();
        let t: i16 = kani::any();
        centi_i16(t, num.into(), exp);
        third_i16(t, num.into(), exp);
    }
}
