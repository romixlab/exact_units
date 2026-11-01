//! Property 7: a decimal as a stored value. [Fixed::from_decimal] is the nearest stored value, ties to even, against
//! the exact quotient in `i128`, and panics where that doesn't fit the storage; `fixed!(literal)` is the same value,
//! worked out at compile time.
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

    /// The stored value of `from_decimal(d)`.
    fn from_decimal(d: Decimal) -> i128;
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

/// `num · 10^exp` in steps of `snum / sden`, to the nearest, ties to even; `None` where the exact quotient needs more
/// than `i128`.
pub(crate) fn nearest(num: i64, exp: i16, snum: i128, sden: i128) -> Option<i128> {
    let ten = 10i128.checked_pow(u32::from(exp.unsigned_abs()))?;
    let (p, d) = if exp >= 0 {
        (i128::from(num).checked_mul(ten)?.checked_mul(sden)?, snum)
    } else {
        (i128::from(num).checked_mul(sden)?, ten.checked_mul(snum)?)
    };
    Some(round_div(p, d, Rounding::NearestEven))
}

/// Where the nearest stored value fits `L`'s storage, `from_decimal` is it.
pub(crate) fn from_decimal_fits<L: Lit>(num: i64, exp: i16) {
    if let Some(q) = nearest(num, exp, L::STEP_NUM, L::STEP_DEN)
        && (L::MIN..=L::MAX).contains(&q)
    {
        assert_eq!(L::from_decimal(Decimal::new(num, exp)), q, "{num}e{exp}");
    }
}

/// [from_decimal_fits], and a panic, never a wrong value, where the nearest stored value doesn't fit: past the
/// storage, or past what the exact quotient can hold.
#[cfg(not(kani))]
pub(crate) fn from_decimal_rounds<L: Lit>(num: i64, exp: i16) {
    from_decimal_fits::<L>(num, exp);
    let got = std::panic::catch_unwind(|| L::from_decimal(Decimal::new(num, exp)));
    match (nearest(num, exp, L::STEP_NUM, L::STEP_DEN), got) {
        (Some(q), Ok(v)) => assert_eq!(v, q, "{num}e{exp}"),
        (Some(q), Err(_)) => assert!(!(L::MIN..=L::MAX).contains(&q), "{num}e{exp}: {q} fits"),
        // the reference can't hold it: whatever the crate gives fits its storage, being one
        (None, _) => {}
    }
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

/// `fixed!(literal)` of each type is `from_decimal` of the literal parsed at run time, and the nearest stored value.
macro_rules! literals {
    ($($ty:ident: $($lit:literal)*;)*) => {
        /// Every literal of the list.
        pub(crate) fn literals() {
            $($(
                let at: $ty = exact_units::fixed!($lit);
                let d = Decimal::parse(stringify!($lit));
                assert_eq!(at, $ty::from_decimal(d), "{}", stringify!($lit));
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
    Q15: 0 0.1 -0.1 0.5 -0.5 -1 0.7071 0.99996948 0.999984741 1.52587890625e-5 -1.52587890625e-5 4.57763671875e-5
        0.999969482421875 -0.999969482421875 1e-10 -1e-10 0.333333333333333333;
    Q31: 0.1 -1 0.999999999 2.3283064365386962e-10 6.984919309616088e-10 -0.75 1e-18;
    Q63: 0.1 -1 0.5 -0.999999999999999999 1.0842021724855044e-19 0.333333333333333333;
    MilliI32: 0.0005 0.0015 0.0025 -0.0005 -0.0025 2147483.647 -2147483.648 1.0005e3 2.5e-4;
    ThirdI16: 2.5 3.5 -2.5 -3.5 0.5 -0.5 1e-1 10922.3333 -10922.6666 0.16666666666666667;
    ThirdU16: 2.5 3.5 0.5 0.1666 21845 0;
    KiloI64: 500 1500 2500 -500 -2500 1e18 -9.2233720368547758e21 0.4999e3 5e2;
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
        fn decimal_parses(num in strategy::i64(), exp in -9000i16..=9000) {
            parses(num, exp);
        }
    }

    #[test]
    fn every_literal() {
        literals();
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
