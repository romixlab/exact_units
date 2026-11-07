//! Property 8: a point converted to another origin and steps is `(x · step₁ + offset₁ - offset₂) / step₂`, exact in
//! `i128`, reduced into the period of a modulo origin (`[0, m)` unsigned, `[-m/2, m - m/2)` signed), and `None` where
//! it isn't whole or doesn't fit the storage. Floats are the same within the float's own error.
#![cfg(not(kani))]
#![expect(
    clippy::as_conversions,
    reason = "a test casts the values it chose and the references it computes; the cast lints say why each fits"
)]
#![expect(
    clippy::arithmetic_side_effects,
    reason = "a test computes with the values it chose, which fit; an overflow fails the test"
)]

use super::rounding::Int;
use exact_units::prefix::{Kilo, Milli, One, Pow2, Rational};
use exact_units::units::Ratio;
use exact_units::{Fixed, FixedScale, Modulo, Offset, Origin, Point, Storage};

macro_rules! origins {
    ($($name:ident: $offset:expr $(, $modulus:ty)?;)*) => {$(
        exact_units::origin! {
            /// An origin of the property.
            pub struct $name { unit: Ratio, symbol: "", offset: $offset $(, modulus: $modulus)? }
        }
    )*};
}

origins! {
    Zero: Offset::ZERO;
    Seven: Offset::int(7);
    Third: Offset::ratio(5, 3);
    Quarter: Offset::ratio(-11, 4);
    Degrees: Offset::ZERO, Modulo<Rational<360, 1>>;
    DegreesAt: Offset::ratio(1, 2), Modulo<Rational<360, 1>>;
    Sixteen: Offset::int(3), Modulo<Pow2<4>>;
}

/// An origin's offset and its steps, as the fractions `n / d` the case states.
#[derive(Copy, Clone)]
pub(crate) struct Side {
    off: (i128, i128),
    step: (i128, i128),
}

impl Side {
    pub(crate) const fn new(off: (i128, i128), step: (i128, i128)) -> Self {
        Side { off, step }
    }
}

/// `x` of `from` in `to`'s steps: the exact fraction `(num, den)` with `den > 0`.
fn fraction(x: i128, from: Side, to: Side) -> (i128, i128) {
    let ((a1, b1), (n1, d1)) = (from.off, from.step);
    let ((a2, b2), (n2, d2)) = (to.off, to.step);
    // y = (x · n1/d1 + a1/b1 - a2/b2) · d2/n2
    let num = (x * n1 * b1 * b2 + (a1 * b2 - a2 * b1) * d1) * d2;
    (num, d1 * b1 * b2 * n2)
}

/// `y` reduced into the period `(p_num, p_den)` of `to`'s unit, as `canonical` has it for a storage that is `signed` or not.
fn reduce(y: i128, period: Option<(i128, i128)>, to: Side, signed: bool) -> i128 {
    let Some((p_num, p_den)) = period else {
        return y;
    };
    let (n2, d2) = to.step;
    assert_eq!((p_num * d2) % (p_den * n2), 0, "the period is whole steps");
    let m = p_num * d2 / (p_den * n2);
    let r = y.rem_euclid(m);
    if signed && r >= m - m / 2 { r - m } else { r }
}

/// The conversion against the exact fraction, for every storage that can hold it.
pub(crate) fn converts<O1, P1, T1, O2, P2, T2>(
    x: T1,
    from: Side,
    to: Side,
    period: Option<(i128, i128)>,
) where
    O1: Origin<Unit = Ratio>,
    O2: Origin<Unit = Ratio, Reference = O1::Reference>,
    P1: FixedScale,
    P2: FixedScale,
    T1: Int,
    T2: Int,
{
    let p = Point::<O1, Fixed<Ratio, P1, T1>>::from_raw(x);
    let got = p.to::<O2, P2, T2>().map(|q| q.raw().into());
    let (num, den) = fraction(x.into(), from, to);
    let want = (num % den == 0)
        .then(|| reduce(num / den, period, to, T2::MIN_INT < 0))
        .filter(|y| (T2::MIN_INT..=T2::MAX_INT).contains(y));
    assert_eq!(got, want, "{x:?}: {num}/{den}");
}

/// The same conversion for floats: within the error of the arithmetic, reduced into `[0, m)` for a modulo origin.
#[expect(
    clippy::expect_used,
    reason = "a float point always converts to a float"
)]
pub(crate) fn converts_float<O1, P1, T1, O2, P2>(
    x: f64,
    from: Side,
    to: Side,
    period: Option<(i128, i128)>,
) where
    O1: Origin<Unit = Ratio>,
    O2: Origin<Unit = Ratio, Reference = O1::Reference>,
    P1: FixedScale,
    P2: FixedScale,
    T1: Storage + FromF64,
{
    let p = Point::<O1, Fixed<Ratio, P1, T1>>::new(
        Fixed::try_new(T1::from_f64(x)).expect("x is a number"),
    );
    let x = T1::to_f64(T1::from_f64(x));
    let got = p.to::<O2, P2, f64>().map(Point::raw);
    #[expect(
        clippy::cast_precision_loss,
        reason = "the fractions of the cases are small: exact in f64"
    )]
    let f = |(n, d): (i128, i128)| n as f64 / d as f64;
    let (a1, a2) = (f(from.off), f(to.off));
    let (s1, s2) = (f(from.step), f(to.step));
    let want = (x * s1 + a1 - a2) / s2;
    let got = got.expect("a float point converts");
    let tolerance = 1e-9 * (want.abs() + (a1 - a2).abs() / s2 + 1.0);
    match period {
        None => assert!((got - want).abs() <= tolerance, "{x}: {got} vs {want}"),
        Some((p_num, p_den)) => {
            let m = f((p_num, p_den)) / s2;
            assert!((0.0..m).contains(&got), "{x}: {got} not in 0..{m}");
            let off = (got - want).rem_euclid(m);
            assert!(
                off <= tolerance || m - off <= tolerance,
                "{x}: {got} vs {want} mod {m}"
            );
        }
    }
}

/// A storage a float case reads its input from.
pub(crate) trait FromF64: Copy {
    fn from_f64(x: f64) -> Self;
    fn to_f64(self) -> f64;
}

impl FromF64 for f64 {
    fn from_f64(x: f64) -> Self {
        x
    }

    fn to_f64(self) -> f64 {
        self
    }
}

impl FromF64 for i32 {
    #[expect(
        clippy::cast_possible_truncation,
        reason = "the input is drawn within i32"
    )]
    fn from_f64(x: f64) -> Self {
        x as i32
    }

    fn to_f64(self) -> f64 {
        f64::from(self)
    }
}

/// `n / d` as a pair.
const fn q(n: i128, d: i128) -> (i128, i128) {
    (n, d)
}

mod tests {
    use super::*;
    use crate::strategy;
    use proptest::prelude::*;

    const ONE: (i128, i128) = q(1, 1);
    const NO: (i128, i128) = q(0, 1);

    /// One proptest per case: the origins and steps, the storages, the reference's sides and period.
    macro_rules! cases {
        ($($name:ident: $o1:ty, $p1:ty, $t1:ident, $f1:expr => $o2:ty, $p2:ty, $t2:ident, $f2:expr $(, period $per:expr)?;)*) => {$(
            proptest! {
                #[test]
                fn $name(x in strategy::$t1()) {
                    converts::<$o1, $p1, $t1, $o2, $p2, $t2>(x, $f1, $f2, None $(.or(Some($per)))?);
                }
            }
        )*};
    }

    cases! {
        // only an offset added, narrow and wide
        add_i32: Zero, One, i32, Side::new(NO, ONE) => Seven, One, i32, Side::new(q(7, 1), ONE);
        sub_i32: Seven, One, i32, Side::new(q(7, 1), ONE) => Zero, One, i32, Side::new(NO, ONE);
        add_i64: Zero, One, i64, Side::new(NO, ONE) => Quarter, One, i64, Side::new(q(-11, 4), ONE);
        // a denominator from the steps, with and without an offset
        third_to_one: Zero, Rational<1, 3>, i32, Side::new(NO, q(1, 3)) => Zero, One, i32, Side::new(NO, ONE);
        milli_offset: Zero, Milli, i32, Side::new(NO, q(1, 1000)) => Seven, One, i32, Side::new(q(7, 1), ONE);
        milli_offset_i64: Zero, Milli, i64, Side::new(NO, q(1, 1000)) => Seven, One, i64, Side::new(q(7, 1), ONE);
        // a fraction in both offsets and both steps
        fractions_i32: Third, Rational<1, 3>, i32, Side::new(q(5, 3), q(1, 3)) => Quarter, Rational<7, 5>, i32, Side::new(q(-11, 4), q(7, 5));
        fractions_i16: Third, Rational<1, 3>, i16, Side::new(q(5, 3), q(1, 3)) => Quarter, Pow2<-3>, i16, Side::new(q(-11, 4), q(1, 8));
        fractions_i64: Third, Rational<1, 3>, i64, Side::new(q(5, 3), q(1, 3)) => Quarter, Rational<7, 5>, i64, Side::new(q(-11, 4), q(7, 5));
        fractions_i64_to_i32: Quarter, Pow2<-3>, i64, Side::new(q(-11, 4), q(1, 8)) => Third, Rational<1, 3>, i32, Side::new(q(5, 3), q(1, 3));
        // a factor above 1
        kilo_to_milli: Seven, Kilo, i32, Side::new(q(7, 1), q(1000, 1)) => Zero, Milli, i64, Side::new(NO, q(1, 1000));
        kilo_to_one: Quarter, Kilo, i64, Side::new(q(-11, 4), q(1000, 1)) => Third, One, i64, Side::new(q(5, 3), ONE);
        sixteen_to_pow2: Zero, Pow2<4>, i32, Side::new(NO, q(16, 1)) => Seven, Pow2<-3>, i64, Side::new(q(7, 1), q(1, 8));
        // modulo origins: signed and unsigned, steps of a third and a thousandth
        wrap_one_i16: Seven, One, i32, Side::new(q(7, 1), ONE) => Degrees, One, i16, Side::new(NO, ONE), period q(360, 1);
        wrap_one_u16: Seven, One, i32, Side::new(q(7, 1), ONE) => Degrees, One, u16, Side::new(NO, ONE), period q(360, 1);
        wrap_milli_i32: Zero, Milli, i32, Side::new(NO, q(1, 1000)) => Degrees, Milli, i32, Side::new(NO, q(1, 1000)), period q(360, 1);
        wrap_milli_u32: Zero, Milli, i32, Side::new(NO, q(1, 1000)) => Degrees, Milli, u32, Side::new(NO, q(1, 1000)), period q(360, 1);
        wrap_third_i64: Third, Rational<1, 3>, i64, Side::new(q(5, 3), q(1, 3)) => DegreesAt, Rational<1, 3>, i64, Side::new(q(1, 2), q(1, 3)), period q(360, 1);
        wrap_third_u64: Third, Rational<1, 3>, i64, Side::new(q(5, 3), q(1, 3)) => DegreesAt, Rational<1, 3>, u64, Side::new(q(1, 2), q(1, 3)), period q(360, 1);
        wrap_kilo: Quarter, Kilo, i32, Side::new(q(-11, 4), q(1000, 1)) => DegreesAt, One, i16, Side::new(q(1, 2), ONE), period q(360, 1);
        wrap_binary_u8: Zero, One, i64, Side::new(NO, ONE) => Sixteen, One, u8, Side::new(q(3, 1), ONE), period q(16, 1);
        wrap_binary_i8: Zero, One, i64, Side::new(NO, ONE) => Sixteen, One, i8, Side::new(q(3, 1), ONE), period q(16, 1);
    }

    /// Floats: the same conversion within rounding, reduced into the period.
    macro_rules! float_cases {
        ($($name:ident: $o1:ty, $p1:ty, $t1:ident, $f1:expr => $o2:ty, $p2:ty, $f2:expr $(, period $per:expr)?;)*) => {$(
            proptest! {
                #[test]
                fn $name(x in -1.0e6..1.0e6f64) {
                    converts_float::<$o1, $p1, $t1, $o2, $p2>(x, $f1, $f2, None $(.or(Some($per)))?);
                }
            }
        )*};
    }

    float_cases! {
        float_offset: Zero, One, f64, Side::new(NO, ONE) => Seven, One, Side::new(q(7, 1), ONE);
        float_fractions: Third, Rational<1, 3>, f64, Side::new(q(5, 3), q(1, 3)) => Quarter, Rational<7, 5>, Side::new(q(-11, 4), q(7, 5));
        float_from_int: Third, Rational<1, 3>, i32, Side::new(q(5, 3), q(1, 3)) => Quarter, Milli, Side::new(q(-11, 4), q(1, 1000));
        float_kilo: Seven, Kilo, f64, Side::new(q(7, 1), q(1000, 1)) => Zero, Milli, Side::new(NO, q(1, 1000));
        float_wrap: Seven, One, f64, Side::new(q(7, 1), ONE) => Degrees, One, Side::new(NO, ONE), period q(360, 1);
        float_wrap_fractions: Third, Rational<1, 3>, f64, Side::new(q(5, 3), q(1, 3)) => DegreesAt, Milli, Side::new(q(1, 2), q(1, 1000)), period q(360, 1);
        float_wrap_from_int: Quarter, Kilo, i32, Side::new(q(-11, 4), q(1000, 1)) => DegreesAt, One, Side::new(q(1, 2), ONE), period q(360, 1);
    }
}
