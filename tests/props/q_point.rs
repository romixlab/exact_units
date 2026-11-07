//! Property 10: a `Point` against a point literal (`q!(25 degC)`, `q!(298.15 K @ AbsoluteZero)`) is the exact order
//! of the two positions, never a rounded one. The point is `t` steps of `sn / sd` K from its origin, `on / od` K
//! above absolute zero; the literal `num · 10^exp` of `an / ad` K from its origin, `ln / ld` K above it. Both sides
//! are multiplied out to integers in `i128`: `<`, `<=` and `==` agree with that order, for the fold a compare makes at
//! compile time (reached at run time through `literal_order_at`) and for `q!` itself, walked over every stored value.
//! For an origin that wraps, `==` is the literal taken into the period, against the number as stored.

use core::cmp::Ordering;
use exact_units::literal::{Decimal, Steps};
use exact_units::origins::{AbsoluteZero, Angle, Celsius, Fahrenheit, Rankine};
use exact_units::prefix::{Centi, Micro, Milli, Rational};
use exact_units::units::{Degree, Kelvin};
use exact_units::{Fixed, Point};

/// A fraction `(n, d)`, `d >= 1`.
type Frac = (i128, i128);

/// 0 K above absolute zero.
const ZERO: Frac = (0, 1);
/// 273.15 K: where °C starts.
const CELSIUS: Frac = (5463, 20);
/// 459.67 °R of 5/9 K: where °F starts.
const FAHRENHEIT: Frac = (45967, 180);
/// A step of 1 K.
const KELVIN: Frac = (1, 1);
/// A step of 5/9 K: °F and °R.
const DEGREE_F: Frac = (5, 9);

/// The point `t · step + origin` against the literal `num · 10^exp · unit + at`, exactly; `None` where the products
/// don't fit `i128`.
pub(crate) fn order_at(
    t: i128,
    step: Frac,
    origin: Frac,
    num: i64,
    exp: i16,
    unit: Frac,
    at: Frac,
) -> Option<Ordering> {
    let ten = 10i128.checked_pow(u32::from(exp.unsigned_abs()))?;
    let (up, down) = if exp >= 0 { (ten, 1) } else { (1, ten) };
    // over the common denominator step.1 · origin.1 · unit.1 · at.1 · down
    let left = t
        .checked_mul(step.0)?
        .checked_mul(origin.1)?
        .checked_mul(unit.1)?
        .checked_mul(at.1)?
        .checked_mul(down)?
        .checked_add(
            origin
                .0
                .checked_mul(step.1)?
                .checked_mul(unit.1)?
                .checked_mul(at.1)?
                .checked_mul(down)?,
        )?;
    let right = i128::from(num)
        .checked_mul(up)?
        .checked_mul(unit.0)?
        .checked_mul(step.1)?
        .checked_mul(origin.1)?
        .checked_mul(at.1)?
        .checked_add(
            at.0.checked_mul(step.1)?
                .checked_mul(origin.1)?
                .checked_mul(unit.1)?
                .checked_mul(down)?,
        )?;
    Some(left.cmp(&right))
}

/// `[<, <=, ==]` as an order has them.
fn ops(o: Ordering) -> [bool; 3] {
    [o.is_lt(), o.is_le(), o.is_eq()]
}

macro_rules! orders {
    ($($name:ident: $o:ty, $f:ty, $raw:ty, $step:expr, $origin:expr; [$($s:ty, $l:ty, $unit:expr, $at:expr;)*])*) => {$(
        /// The fold's `[<, <=, ==]` of the point `t` against `num · 10^exp` at each of the literal origins is the
        /// exact order, wherever the reference's products fit `i128`.
        pub(crate) fn $name(t: $raw, num: i64, exp: i16) {
            let p = Point::<$o, $f>::from_raw(t);
            $(
                if let Some(want) = order_at(t.into(), $step, $origin, num, exp, $unit, $at) {
                    let got = p.literal_order_at::<$s, $l>(Decimal::new(num, exp));
                    assert_eq!(got, ops(want), "{t} against {num}e{exp} {}", stringify!($l));
                }
            )*
        }
    )*};
}

orders! {
    celsius_centi_i16: Celsius, Fixed<Kelvin, Centi, i16>, i16, (1, 100), CELSIUS; [
        Steps, Celsius, KELVIN, CELSIUS;
        Kelvin, AbsoluteZero, KELVIN, ZERO;
        Steps, Fahrenheit, DEGREE_F, FAHRENHEIT; ]
    kelvin_centi_u16: AbsoluteZero, Fixed<Kelvin, Centi, u16>, u16, (1, 100), ZERO; [
        Steps, Celsius, KELVIN, CELSIUS;
        Steps, Fahrenheit, DEGREE_F, FAHRENHEIT;
        Steps, Rankine, DEGREE_F, ZERO; ]
    fahrenheit_tenths_i16: Fahrenheit, Fixed<Kelvin, Rational<1, 18>, i16>, i16, (1, 18), FAHRENHEIT; [
        Steps, Fahrenheit, DEGREE_F, FAHRENHEIT;
        Steps, Celsius, KELVIN, CELSIUS; ]
    celsius_u8: Celsius, Fixed<Kelvin, Rational<1, 2>, u8>, u8, (1, 2), CELSIUS; [
        Steps, Celsius, KELVIN, CELSIUS;
        Steps, Fahrenheit, DEGREE_F, FAHRENHEIT; ]
    kelvin_milli_i32: AbsoluteZero, Fixed<Kelvin, Milli, i32>, i32, (1, 1000), ZERO; [
        Steps, Celsius, KELVIN, CELSIUS;
        Steps, Fahrenheit, DEGREE_F, FAHRENHEIT;
        Kelvin, AbsoluteZero, KELVIN, ZERO; ]
    celsius_micro_i64: Celsius, Fixed<Kelvin, Micro, i64>, i64, (1, 1_000_000), CELSIUS; [
        Steps, Celsius, KELVIN, CELSIUS;
        Steps, Fahrenheit, DEGREE_F, FAHRENHEIT; ]
    // one origin each, small enough for a Kani proof
    celsius_i16_at_celsius: Celsius, Fixed<Kelvin, Centi, i16>, i16, (1, 100), CELSIUS; [
        Steps, Celsius, KELVIN, CELSIUS; ]
    celsius_i16_at_fahrenheit: Celsius, Fixed<Kelvin, Centi, i16>, i16, (1, 100), CELSIUS; [
        Steps, Fahrenheit, DEGREE_F, FAHRENHEIT; ]
}

/// `==` of an angle in tenths of a degree (period 3600 steps) against `num · 10^exp` degrees: the literal is a whole
/// number of steps that leaves the same remainder as `t` in the period, and `t` is in the period's range for its
/// storage (`[0, 3600)` unsigned, `[-1800, 1800)` signed), where points are kept.
#[expect(
    clippy::arithmetic_side_effects,
    reason = "d >= 1 (a power of ten); n / d and t are far inside i128, so their difference fits"
)]
fn angle_eq(t: i128, num: i64, exp: i16, signed: bool) -> Option<bool> {
    let ten = 10i128.checked_pow(u32::from(exp.unsigned_abs()))?;
    // the literal in tenths of a degree: num · 10^(exp + 1)
    let (n, d) = if exp >= 0 {
        (i128::from(num).checked_mul(ten)?.checked_mul(10)?, 1)
    } else {
        (i128::from(num).checked_mul(10)?, ten)
    };
    let kept = if signed {
        (-1800..1800).contains(&t)
    } else {
        (0..3600).contains(&t)
    };
    Some(n % d == 0 && kept && (n / d - t).rem_euclid(3600) == 0)
}

macro_rules! angle_eqs {
    ($($name:ident: $raw:ty, $signed:expr;)*) => {$(
        /// The fold's `==` of an angle point `t` in 0.1° against `num · 10^exp` degrees is [angle_eq].
        pub(crate) fn $name(t: $raw, num: i64, exp: i16) {
            if let Some(want) = angle_eq(t.into(), num, exp, $signed) {
                let p = Point::<Angle<Degree>, Fixed<Degree, Rational<1, 10>, $raw>>::from_raw(t);
                let got = p.literal_order_at::<Degree, Angle<Degree>>(Decimal::new(num, exp))[2];
                assert_eq!(got, want, "{t} against {num}e{exp}°");
            }
        }
    )*};
}

angle_eqs! {
    angle_i16: i16, true;
    angle_u16: u16, false;
}

#[cfg(not(any(kani, fuzzing)))]
mod tests {
    use super::*;
    use crate::strategy;
    use exact_units::q;
    use proptest::prelude::*;

    /// Exponents where the reference mostly fits, with a few past it.
    fn exp() -> impl Strategy<Value = i16> {
        prop_oneof![8 => -9i16..=4, 1 => -20i16..=12]
    }

    proptest! {
        #[test]
        fn narrow_storages(t in strategy::i16(), u in any::<u16>(), b in any::<u8>(), num in strategy::i64(), exp in exp()) {
            celsius_centi_i16(t, num, exp);
            kelvin_centi_u16(u, num, exp);
            fahrenheit_tenths_i16(t, num, exp);
            celsius_u8(b, num, exp);
            celsius_i16_at_celsius(t, num, exp);
            celsius_i16_at_fahrenheit(t, num, exp);
            angle_i16(t, num, exp);
            angle_u16(u, num, exp);
        }

        /// Literals near the stored value, in thousandths of the literal's unit, in each origin's frame: on it
        /// (`d` 0) and a little either side, where an off-by-one in the fold (a floor toward 0) would show.
        #[test]
        #[expect(
            clippy::arithmetic_side_effects,
            reason = "t is an i16, d and turns small: every literal is within 10^9, far inside i64"
        )]
        fn near_the_values(t in strategy::i16(), d in -12i64..=12, turns in -3i64..=3) {
            let t64 = i64::from(t);
            // t hundredths of a °C: 10·t m°C, 18·t + 32000 m°F, 10·t + 273150 mK
            celsius_centi_i16(t, 10 * t64 + d, -3);
            celsius_centi_i16(t, 18 * t64 + 32_000 + d, -3);
            celsius_centi_i16(t, 10 * t64 + 273_150 + d, -3);
            // t tenths of a °F: 100·t m°F, (100·t − 32000) · 5/9 m°C
            fahrenheit_tenths_i16(t, 100 * t64 + d, -3);
            fahrenheit_tenths_i16(t, (100 * t64 - 32_000) * 5 / 9 + d, -3);
            // t tenths of a degree, some turns away
            angle_i16(t, 100 * (t64 + 3600 * turns) + d, -3);
        }

        #[test]
        fn wide_storages(t32 in strategy::i32(), t64 in strategy::i64(), num in strategy::i64(), exp in exp()) {
            kelvin_milli_i32(t32, num, exp);
            celsius_micro_i64(t64, num, exp);
        }
    }

    /// Every operator of point `t` against a point literal, both ways round, is order `ord`.
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

    /// The reference order of point `t` against a literal.
    #[expect(
        clippy::panic,
        reason = "the walks choose literals whose reference fits i128; a panic fails the test"
    )]
    fn want(
        t: i16,
        step: Frac,
        origin: Frac,
        num: i64,
        exp: i16,
        unit: Frac,
        at: Frac,
    ) -> Ordering {
        order_at(t.into(), step, origin, num, exp, unit, at)
            .unwrap_or_else(|| panic!("{t} against {num}e{exp}"))
    }

    /// `q!` itself over every i16 in steps of 0.01 °C: every form of a point literal, at the same origin and others.
    #[test]
    fn every_i16_in_hundredths_of_celsius() {
        const C: Frac = (1, 100);
        for t in i16::MIN..=i16::MAX {
            let p = Point::<Celsius, Fixed<Kelvin, Centi, i16>>::from_raw(t);
            agrees!(p, q!(25 degC), want(t, C, CELSIUS, 25, 0, KELVIN, CELSIUS));
            agrees!(p, q!(25 "°C"), want(t, C, CELSIUS, 25, 0, KELVIN, CELSIUS));
            agrees!(p, q!("25 ℃"), want(t, C, CELSIUS, 25, 0, KELVIN, CELSIUS));
            agrees!(
                p,
                q!(-40.005 degC),
                want(t, C, CELSIUS, -40005, -3, KELVIN, CELSIUS)
            );
            agrees!(
                p,
                q!(25 @ Celsius),
                want(t, C, CELSIUS, 25, 0, KELVIN, CELSIUS)
            );
            agrees!(
                p,
                q!(250 mK @ Celsius),
                want(t, C, CELSIUS, 25, -2, KELVIN, CELSIUS)
            );
            agrees!(
                p,
                q!(298.15 K @ AbsoluteZero),
                want(t, C, CELSIUS, 29815, -2, KELVIN, ZERO)
            );
            agrees!(
                p,
                q!(0.29815 kK @ AbsoluteZero),
                want(t, C, CELSIUS, 29815, -2, KELVIN, ZERO)
            );
            agrees!(
                p,
                q!(77 degF),
                want(t, C, CELSIUS, 77, 0, DEGREE_F, FAHRENHEIT)
            );
            agrees!(
                p,
                q!(98.6 degF),
                want(t, C, CELSIUS, 986, -1, DEGREE_F, FAHRENHEIT)
            );
            agrees!(
                p,
                q!(-40 "°F"),
                want(t, C, CELSIUS, -40, 0, DEGREE_F, FAHRENHEIT)
            );
            // below the point's origin and not a whole step: the floor is toward −∞
            agrees!(
                p,
                q!(-5 degF),
                want(t, C, CELSIUS, -5, 0, DEGREE_F, FAHRENHEIT)
            );
            agrees!(
                p,
                q!(-0.01 degF),
                want(t, C, CELSIUS, -1, -2, DEGREE_F, FAHRENHEIT)
            );
            agrees!(
                p,
                q!(273.145 K @ AbsoluteZero),
                want(t, C, CELSIUS, 273_145, -3, KELVIN, ZERO)
            );
            agrees!(
                p,
                q!(77 @ Fahrenheit),
                want(t, C, CELSIUS, 77, 0, DEGREE_F, FAHRENHEIT)
            );
            agrees!(
                p,
                q!(536.67 degR),
                want(t, C, CELSIUS, 53667, -2, DEGREE_F, ZERO)
            );
            // past i16's range: a constant answer
            agrees!(p, q!(400 degC), Ordering::Less);
            agrees!(p, q!(-400 degC), Ordering::Greater);
        }
    }

    /// Every u16 in 0.01 K from absolute zero, and every u8 in half degrees Celsius: unsigned storages, literals below
    /// the origin and past the range.
    #[test]
    fn every_u16_and_u8() {
        for t in u16::MIN..=u16::MAX {
            let p = Point::<AbsoluteZero, Fixed<Kelvin, Centi, u16>>::from_raw(t);
            agrees!(p, q!(25 degC), t.cmp(&29815));
            agrees!(p, q!(-273.15 degC), t.cmp(&0));
            agrees!(p, q!(-273.16 degC), Ordering::Greater);
            agrees!(p, q!(32 degF), t.cmp(&27315));
            agrees!(p, q!(0 K @ AbsoluteZero), t.cmp(&0));
            agrees!(p, q!(400 degC), Ordering::Less);
        }
        for t in u8::MIN..=u8::MAX {
            let p = Point::<Celsius, Fixed<Kelvin, Rational<1, 2>, u8>>::from_raw(t);
            agrees!(p, q!(25 degC), t.cmp(&50));
            agrees!(
                p,
                q!(25.25 degC),
                if t <= 50 {
                    Ordering::Less
                } else {
                    Ordering::Greater
                }
            );
            agrees!(p, q!(-0.5 degC), Ordering::Greater);
            agrees!(p, q!(127.5 degC), t.cmp(&255));
            agrees!(p, q!(128 degC), Ordering::Less);
        }
    }

    /// `==` of angles that wrap: the literal is taken into the period, the point compared as stored.
    #[test]
    fn every_i16_and_u16_angle() {
        for t in i16::MIN..=i16::MAX {
            let p = Point::<Angle<Degree>, Fixed<Degree, Rational<1, 10>, i16>>::from_raw(t);
            assert_eq!(p == q!(10 deg @ Angle<Degree>), t == 100);
            assert_eq!(p == q!(370 deg @ Angle<Degree>), t == 100);
            assert_eq!(p == q!(-350 deg @ Angle<Degree>), t == 100);
            assert_eq!(p == q!(180 deg @ Angle<Degree>), t == -1800);
            assert_eq!(p == q!(-180 deg @ Angle<Degree>), t == -1800);
            assert_eq!(q!(190 "°" @ Angle<Degree>) == p, t == -1700);
            assert!(p != q!(10.05 deg @ Angle<Degree>));
        }
        for t in u16::MIN..=u16::MAX {
            let p = Point::<Angle<Degree>, Fixed<Degree, Rational<1, 10>, u16>>::from_raw(t);
            assert_eq!(p == q!(-10 deg @ Angle<Degree>), t == 3500);
            assert_eq!(p == q!(720 deg @ Angle<Degree>), t == 0);
        }
    }

    /// The fold `literal_order_at` is the one `q!` makes, over every i16.
    #[test]
    fn literal_order_at_is_what_q_compares() {
        for t in i16::MIN..=i16::MAX {
            let p = Point::<Celsius, Fixed<Kelvin, Centi, i16>>::from_raw(t);
            assert_eq!(
                [p < q!(98.6 degF), p <= q!(98.6 degF), p == q!(98.6 degF)],
                p.literal_order_at::<Steps, Fahrenheit>(Decimal::new(986, -1)),
                "{t}"
            );
            assert_eq!(
                [p < q!(-0.01 degC), p <= q!(-0.01 degC), p == q!(-0.01 degC)],
                p.literal_order_at::<Steps, Celsius>(Decimal::new(-1, -2)),
                "{t}"
            );
            assert_eq!(
                [
                    p < q!(300 K @ AbsoluteZero),
                    p <= q!(300 K @ AbsoluteZero),
                    p == q!(300 K @ AbsoluteZero)
                ],
                p.literal_order_at::<Kelvin, AbsoluteZero>(Decimal::new(3, 2)),
                "{t}"
            );
        }
    }
}

#[cfg(kani)]
mod proofs {
    use super::*;

    /// Every i16 in 0.01 °C against every i8 of digits with a power of ten from 10⁻² to 10¹ in °C: the same origin,
    /// the fold of `q!` against a `Fixed`.
    #[kani::proof]
    #[kani::unwind(40)]
    fn celsius_i16_at_its_origin() {
        let exp: i16 = kani::any_where(|e| (-2..=1).contains(e));
        let num: i8 = kani::any();
        celsius_i16_at_celsius(kani::any(), num.into(), exp);
    }

    /// The same in °F: another origin, its offset added in fractions of integers.
    #[kani::proof]
    #[kani::unwind(40)]
    fn celsius_i16_at_another_origin() {
        let exp: i16 = kani::any_where(|e| (-2..=1).contains(e));
        let num: i8 = kani::any();
        celsius_i16_at_fahrenheit(kani::any(), num.into(), exp);
    }

    /// Every i16 angle in 0.1° against the same literals in degrees: `==` taken into the period.
    #[kani::proof]
    #[kani::unwind(40)]
    fn angle_i16_in_its_period() {
        let exp: i16 = kani::any_where(|e| (-2..=1).contains(e));
        let num: i8 = kani::any();
        angle_i16(kani::any(), num.into(), exp);
    }
}
