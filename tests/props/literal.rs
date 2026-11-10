//! Property 7: a decimal as a stored value, against the exact quotient in `i128`. [Fixed::from_decimal] is the stored
//! value the decimal is, and panics where it is none; [Fixed::from_decimal_rounded] is the one each [Rounding] picks;
//! both panic where the decimal itself is past the storage, before any rounding. `fixed!(literal)` and
//! `fixed!(literal, NearestEven)` are the same values, worked out at compile time.
#![expect(
    clippy::as_conversions,
    reason = "a test casts the values it chose and the references it computes; the cast lints say why each fits"
)]
#![expect(
    clippy::arithmetic_side_effects,
    reason = "a test computes with the values it chose, which fit; an overflow fails the test"
)]

use super::reference::round_div;
use exact_units::literal::Decimal;
use exact_units::prefix::{Kilo, Milli, Pow2, Rational};
use exact_units::units::Ratio;
use exact_units::{Fixed, Rounding};

/// A [Fixed] read as integers: its step `STEP_NUM / STEP_DEN`, its storage's range and `from_decimal`'s stored value.
pub(crate) trait Lit {
    /// The step's numerator.
    const STEP_NUM: i128;

    /// The step's denominator.
    const STEP_DEN: i128;

    /// The storage's least value.
    const MIN: i128;

    /// The storage's greatest value.
    const MAX: i128;

    /// The stored value of `from_decimal(d)`: panics where `d` is no stored value.
    fn from_decimal(d: Decimal) -> i128;

    /// The stored value of `from_decimal_rounded(d, r)`.
    fn from_decimal_rounded(d: Decimal, r: Rounding) -> i128;
}

macro_rules! decimal_types {
    ($($name:ident = $p:ty, $t:ty, $snum:expr, $sden:expr;)*) => {$(
        pub(crate) type $name = Fixed<Ratio, $p, $t>;
        impl Lit for $name {
            const STEP_NUM: i128 = $snum;

            const STEP_DEN: i128 = $sden;

            const MIN: i128 = <$t>::MIN as i128;

            const MAX: i128 = <$t>::MAX as i128;

            fn from_decimal(d: Decimal) -> i128 {
                Self::from_decimal(d).get().into()
            }

            fn from_decimal_rounded(d: Decimal, r: Rounding) -> i128 {
                Self::from_decimal_rounded(d, r).get().into()
            }
        }
    )*};
}

decimal_types! {
    Q15 = Pow2<-15>, i16, 1, 1 << 15;
    Q31 = Pow2<-31>, i32, 1, 1 << 31;
    Q63 = Pow2<-63>, i64, 1, 1 << 63;
    MilliI32 = Milli, i32, 1, 1000;
    ThirdI16 = Rational<1, 3>, i16, 1, 3;
    ThirdU16 = Rational<1, 3>, u16, 1, 3;
    KiloI64 = Kilo, i64, 1000, 1;
}

/// Every rounding.
pub(crate) const MODES: [Rounding; 6] = [
    Rounding::Truncate,
    Rounding::Floor,
    Rounding::Ceil,
    Rounding::NearestEven,
    Rounding::NearestAway,
    Rounding::NearestCeil,
];

/// `num · 10^exp` in steps of `snum / sden`, rounded by `r`; `None` where the exact quotient needs more than `i128`.
pub(crate) fn rounded(num: i64, exp: i16, snum: i128, sden: i128, r: Rounding) -> Option<i128> {
    let ten = 10i128.checked_pow(u32::from(exp.unsigned_abs()))?;
    let (p, d) = if exp >= 0 {
        (i128::from(num).checked_mul(ten)?.checked_mul(sden)?, snum)
    } else {
        (i128::from(num).checked_mul(sden)?, ten.checked_mul(snum)?)
    };
    Some(round_div(p, d, r))
}

/// `num · 10^exp` in steps of `snum / sden`, to the nearest, ties to even.
pub(crate) fn nearest(num: i64, exp: i16, snum: i128, sden: i128) -> Option<i128> {
    rounded(num, exp, snum, sden, Rounding::NearestEven)
}

/// The decimal `num · 10^exp` lies within `L`'s storage: its floor and ceiling in steps are both stored values.
fn within<L: Lit>(num: i64, exp: i16) -> Option<bool> {
    let lo = rounded(num, exp, L::STEP_NUM, L::STEP_DEN, Rounding::Floor)?;
    let hi = rounded(num, exp, L::STEP_NUM, L::STEP_DEN, Rounding::Ceil)?;
    Some(L::MIN <= lo && hi <= L::MAX)
}

/// Where the decimal lies within `L`'s storage, `from_decimal_rounded` is the reference's rounding, for every mode,
/// and `from_decimal` is the decimal itself where it is a stored value.
pub(crate) fn from_decimal_fits<L: Lit>(num: i64, exp: i16) {
    if within::<L>(num, exp) != Some(true) {
        return;
    }
    let d = Decimal::new(num, exp);
    for r in MODES {
        let q = rounded(num, exp, L::STEP_NUM, L::STEP_DEN, r);
        assert_eq!(Some(L::from_decimal_rounded(d, r)), q, "{num}e{exp} {r:?}");
    }
    let lo = rounded(num, exp, L::STEP_NUM, L::STEP_DEN, Rounding::Floor);
    if lo == rounded(num, exp, L::STEP_NUM, L::STEP_DEN, Rounding::Ceil) {
        assert_eq!(Some(L::from_decimal(d)), lo, "{num}e{exp}");
    }
}

/// [from_decimal_fits], and a panic, never a wrong value, everywhere else: `from_decimal` of a decimal that is no
/// stored value, either of a decimal past the storage, or past what the exact quotient can hold.
#[cfg(not(kani))]
pub(crate) fn from_decimal_rounds<L: Lit>(num: i64, exp: i16) {
    from_decimal_fits::<L>(num, exp);
    let d = Decimal::new(num, exp);
    let fits = within::<L>(num, exp);
    for r in MODES {
        let got = std::panic::catch_unwind(|| L::from_decimal_rounded(d, r));
        match (fits, got) {
            (Some(true), got) => assert!(got.is_ok(), "{num}e{exp} {r:?} refused"),
            (Some(false), got) => {
                assert!(got.is_err(), "{num}e{exp} {r:?}: past the storage, taken");
            }
            (None, _) => {}
        }
    }
    let exact = rounded(num, exp, L::STEP_NUM, L::STEP_DEN, Rounding::Floor)
        .zip(rounded(num, exp, L::STEP_NUM, L::STEP_DEN, Rounding::Ceil))
        .map(|(lo, hi)| lo == hi);
    let got = std::panic::catch_unwind(|| L::from_decimal(d));
    match (fits, exact, got) {
        (Some(true), Some(true), got) => assert!(got.is_ok(), "{num}e{exp} refused"),
        (Some(_), Some(_), got) => assert!(got.is_err(), "{num}e{exp}: no stored value, taken"),
        _ => {}
    }
}

/// Whether the float `f` is the decimal `num · 10^exp` exactly, by integers: `None` where they need more than
/// `i128`.
fn float_is(num: i64, exp: i16, f: f64) -> Option<bool> {
    let bits = f.abs().to_bits();
    let field = i32::try_from(bits >> 52).ok()?;
    let m = i128::from(bits & ((1 << 52) - 1) | (1 << 52));
    let e = field - 1075;
    if (num < 0) != (f < 0.0) {
        return Some(false);
    }
    let ten = 10i128.checked_pow(u32::from(exp.unsigned_abs()))?;
    let two = 1i128
        .checked_shl(e.unsigned_abs())
        .filter(|_| e.unsigned_abs() < 127)?;
    // |num| · 10^exp == m · 2^e, each side with the negative powers moved across
    let mut left = i128::from(num.unsigned_abs());
    let mut right = m;
    if exp >= 0 {
        left = left.checked_mul(ten)?;
    } else {
        right = right.checked_mul(ten)?;
    }
    if e >= 0 {
        right = right.checked_mul(two)?;
    } else {
        left = left.checked_mul(two)?;
    }
    Some(left == right)
}

/// A float storage's `from_decimal_rounded(.., NearestEven)` is the nearest float, ties to even, as Rust's own
/// correctly rounded parser has it: wherever that is a normal number and the decimal is within the normal range.
/// `Floor` and `Ceil` are the floats around it, the same float or neighbours, `Truncate`, `NearestAway` and
/// `NearestCeil` one of them, the last two the same for a positive decimal; `from_decimal` is the decimal where it is a float, as integers tell, and refuses it elsewhere.
#[cfg(not(kani))]
pub(crate) fn float_from_decimal_is_nearest(num: i64, exp: i16) {
    // a literal whose whole value needs more than 128 bits is refused, as documented
    if exp > 0
        && 10u128
            .checked_pow(u32::from(exp.unsigned_abs()))
            .and_then(|ten| ten.checked_mul(u128::from(num.unsigned_abs())))
            .is_none()
    {
        return;
    }
    let text = format!("{num}e{exp}");
    let d = Decimal::new(num, exp);
    macro_rules! check {
        ($t:ty) => {{
            type F = Fixed<Ratio, exact_units::prefix::One, $t>;
            let want: $t = text.parse().expect("digits and an exponent");
            // a decimal next to the ends of the normal range may lie past them although its nearest float doesn't
            if num != 0 && want.is_normal() && want.abs() < <$t>::MAX && want.abs() > <$t>::MIN_POSITIVE {
                let got = |r| F::from_decimal_rounded(d, r).get();
                assert_eq!(got(Rounding::NearestEven).to_bits(), want.to_bits(), "{text} as {}", stringify!($t));
                let (lo, hi) = (got(Rounding::Floor), got(Rounding::Ceil));
                let same = lo.to_bits() == hi.to_bits();
                assert!(same || lo.next_up().to_bits() == hi.to_bits(), "{text}: {lo} and {hi}");
                assert!(lo <= want && want <= hi, "{text}");
                let toward_zero = if num < 0 { hi } else { lo };
                assert_eq!(got(Rounding::Truncate).to_bits(), toward_zero.to_bits(), "{text}");
                let away = got(Rounding::NearestAway);
                assert!(away.to_bits() == lo.to_bits() || away.to_bits() == hi.to_bits(), "{text}");
                let up = got(Rounding::NearestCeil);
                assert!(up.to_bits() == lo.to_bits() || up.to_bits() == hi.to_bits(), "{text}");
                assert!(num < 0 || up.to_bits() == away.to_bits(), "{text}: a positive tie toward +∞ is away from 0");
                let exact = std::panic::catch_unwind(|| F::from_decimal(d).get());
                assert_eq!(exact.is_ok(), same, "{text}: exact one way, not the other");
                if let Some(is) = float_is(num, exp, f64::from(want)) {
                    assert_eq!(is, same, "{text}: the integers disagree");
                }
            }
        }};
    }
    check!(f64);
    check!(f32);
}

/// A decimal written out parses back: `{num}e{exp}`, and with the point moved into the digits (`123.45e{exp + 2}`).
/// `i64::MIN` is left out: its digits don't fit the positive `i64` the parser builds them in.
#[cfg(not(kani))]
#[expect(clippy::expect_used, reason = "an i64 has at most 19 digits")]
pub(crate) fn parses(num: i64, exp: i16) {
    if num == i64::MIN {
        return;
    }
    assert_eq!(
        Decimal::parse(&format!("{num}e{exp}")),
        Decimal::new(num, exp)
    );
    let digits = num.unsigned_abs().to_string();
    let sign = if num < 0 { "-" } else { "" };
    for at in 1..digits.len() {
        let (int, frac) = digits.split_at(at);
        let e = i32::from(exp) + i32::try_from(frac.len()).expect("at most 19 digits");
        let s = format!("{sign}{int}.{frac}e{e}");
        assert_eq!(Decimal::parse(&s), Decimal::new(num, exp), "{s}");
    }
}

/// `fixed!(literal, NearestEven)` of each type is `from_decimal_rounded` of the literal parsed at run time, and the
/// nearest stored value.
macro_rules! literals {
    ($($ty:ident: $($lit:literal)*;)*) => {
        /// Every literal of the list.
        pub(crate) fn literals() {
            $($(
                let at: $ty = exact_units::fixed!($lit, NearestEven);
                let d = Decimal::parse(stringify!($lit));
                assert_eq!(at, $ty::from_decimal_rounded(d, Rounding::NearestEven), "{}", stringify!($lit));
                assert_eq!(
                    Some(at.get().into()),
                    nearest(d.num(), d.exp(), <$ty as Lit>::STEP_NUM, <$ty as Lit>::STEP_DEN),
                    "{}",
                    stringify!($lit),
                );
            )*)*
        }
    };
}

literals! {
    Q15: 0 0.1 -0.1 0.5 -0.5 -1 0.7071 0.99996948 1.52587890625e-5 -1.52587890625e-5 4.57763671875e-5
        0.999969482421875 -0.999969482421875 1e-10 -1e-10 0.333333333333333333;
    Q31: 0.1 -1 0.999999999 2.3283064365386962e-10 6.984919309616088e-10 -0.75 1e-18;
    Q63: 0.1 -1 0.5 -0.999999999999999999 1.0842021724855044e-19 0.333333333333333333;
    MilliI32: 0.0005 0.0015 0.0025 -0.0005 -0.0025 2147483.647 -2147483.648 1.0005e3 2.5e-4;
    ThirdI16: 2.5 3.5 -2.5 -3.5 0.5 -0.5 1e-1 10922.3333 -10922.6666 0.16666666666666667;
    ThirdU16: 2.5 3.5 0.5 0.1666 21845 0;
    KiloI64: 500 1500 2500 -500 -2500 1e18 -9.2233720368547758e21 0.4999e3 5e2;
}

/// `fixed!(literal)` without a rounding of a literal that is a stored value: its whole decimal, however long, reads
/// back as that value. The long ones are what the compile error of an inexact literal prints.
macro_rules! exact_literals {
    ($($ty:ident: $($lit:literal => $stored:expr),*;)*) => {
        /// Every literal of the list.
        pub(crate) fn exact_literals() {
            $($(
                let at: $ty = exact_units::fixed!($lit);
                assert_eq!(i128::from(at.get()), $stored, "{}", stringify!($lit));
            )*)*
        }
    };
}

exact_literals! {
    Q15: 0.5 => 16_384, -1 => -32_768, 3.0517578125e-5 => 1, 0.999969482421875 => 32_767,
        0.70709228515625 => 23_170, 0.707122802734375 => 23_171;
    Q31: 0.1000000000931322574615478515625 => 214_748_365, -0.75 => -1_610_612_736;
    Q63: 1.084202172485504434007452800869941711425781250e-19 => 1,
        0.100000000000000000021684043449710088680149056017398834228515625 => 922_337_203_685_477_581,
        -1 => i128::from(i64::MIN);
    MilliI32: 2147483.647 => 2_147_483_647, 2e-3 => 2;
    ThirdI16: 1e1 => 30;
    KiloI64: 1e18 => 1_000_000_000_000_000, -2e3 => -2;
}

#[cfg(not(kani))]
mod tests {
    use super::*;
    use crate::strategy;
    use proptest::prelude::*;

    /// Exponents where the quotient mostly fits `i128` and the storages, with a few past both.
    fn exp() -> impl Strategy<Value = i16> {
        prop_oneof![8 => -20i16..=4, 1 => -45i16..=40, 1 => any::<i16>()]
    }

    proptest! {
        #[test]
        fn from_decimal(num in strategy::i64(), exp in exp()) {
            from_decimal_rounds::<Q15>(num, exp);
            from_decimal_rounds::<Q31>(num, exp);
            from_decimal_rounds::<Q63>(num, exp);
            from_decimal_rounds::<MilliI32>(num, exp);
            from_decimal_rounds::<ThirdI16>(num, exp);
            from_decimal_rounds::<ThirdU16>(num, exp);
            from_decimal_rounds::<KiloI64>(num, exp);
        }

        #[test]
        fn float_from_decimal(num in strategy::i64(), exp in -38i16..=38) {
            float_from_decimal_is_nearest(num, exp);
        }

        /// Half an ulp below a power of two, where the nearest float is the power itself (a carry into the exponent):
        /// `2^k · 10^j - d` with the digits just short of it.
        #[test]
        fn float_from_decimal_below_a_power_of_two(k in 0u32..=62, j in 0u32..=18, d in 0i64..=80) {
            let scaled = 1i64.checked_shl(k).and_then(|p| p.checked_mul(10i64.pow(j)));
            if let Some(scaled) = scaled {
                float_from_decimal_is_nearest(scaled - d, -i16::try_from(j).unwrap());
            }
        }

        #[test]
        fn decimal_parses(num in strategy::i64(), exp in -9000i16..=9000) {
            parses(num, exp);
        }
    }

    #[test]
    fn every_literal() {
        literals();
        exact_literals();
    }
}

#[cfg(kani)]
mod proofs {
    use super::*;

    /// Every i8 of digits with a power of ten from 10⁻² to 10¹, into Q15: 4 minutes. The quotient's gcd loops in
    /// `u128` make i16 digits take 9 minutes at one power (10⁻³) and over 30 at 10⁻⁶..=10².
    #[kani::proof]
    #[kani::unwind(40)]
    fn from_decimal_q15() {
        let exp: i16 = kani::any_where(|e| (-2..=1).contains(e));
        from_decimal_fits::<Q15>(kani::any::<i8>().into(), exp);
    }
}
