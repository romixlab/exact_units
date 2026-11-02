//! Levels (STD-97): dB ↔ Np in integers rounded against an exact reference (ln 10 to 30 digits), a literal in dB
//! into an Np storage, a loss negated into a gain and back by each overflow policy, and the gain edge in floats.
#![expect(
    clippy::arithmetic_side_effects,
    reason = "a test computes with the values it chose, which fit; an overflow fails the test"
)]

use super::reference::{LN10, LN10_DEN, round_div};
use exact_units::prefix::{Centi, Micro};
use exact_units::units::{Decibel, Loss, Neper};
use exact_units::{Fixed, Rounding, Saturating, Wrapping};

/// `x` hundredths of a dB into µNp, rounded by `r`: `x · ln10 · 500` exactly (1 dB is ln10/20 Np), and the same for
/// a loss.
pub(crate) fn db_to_np(x: i16, r: Rounding) {
    let want = round_div(i128::from(x) * 500 * LN10, LN10_DEN, r);
    let got = Fixed::<Decibel, Centi, i16>::new(x).convert_rounded::<Neper, Micro, i32>(r);
    assert_eq!(
        got.map(|f| i128::from(f.get())),
        Some(want),
        "{x} cdB {r:?}"
    );
    let loss =
        Fixed::<Loss<Decibel>, Centi, i16>::new(x).convert_rounded::<Loss<Neper>, Micro, i32>(r);
    assert_eq!(
        loss.map(|f| i128::from(f.get())),
        Some(want),
        "{x} cdB of loss {r:?}"
    );
}

/// `x` µNp into hundredths of a dB, rounded by `r`: `x · 2 · 10²⁶ / ln10` exactly; `None` only past `i16`.
pub(crate) fn np_to_db(x: i32, r: Rounding) {
    let want = round_div(i128::from(x) * 2 * 10_i128.pow(26), LN10, r);
    let got = Fixed::<Neper, Micro, i32>::new(x).convert_rounded::<Decibel, Centi, i16>(r);
    let fits = i16::try_from(want).ok();
    assert_eq!(got.map(Fixed::get), fits, "{x} µNp {r:?}");
}

/// A literal of `num` hundredths of a dB in an Np storage of µNp: the nearest stored value, ties to even, as
/// `fixed!(17.7 in Decibel)` lands it.
pub(crate) fn literal_in_db(num: i32) {
    let want = round_div(
        i128::from(num) * 500 * LN10,
        LN10_DEN,
        Rounding::NearestEven,
    );
    let got = Fixed::<Neper, Micro, i64>::from_decimal_in::<Decibel>(
        exact_units::literal::Decimal::new(i64::from(num), -2),
    );
    assert_eq!(i128::from(got.get()), want, "{num} cdB");
}

/// A gain negated into a loss and back is the same, by each policy; at `MIN` the loss saturates to `MAX` or wraps to
/// `MIN`.
pub(crate) fn loss_round_trip(x: i16) {
    let s = Fixed::<Decibel, Centi, i16, Saturating>::of(x);
    let loss = s.as_loss();
    assert_eq!(loss.get(), x.saturating_neg());
    if x != i16::MIN {
        assert_eq!(loss.as_gain(), s);
    }
    let w = Fixed::<Decibel, Centi, i16, Wrapping>::of(x);
    assert_eq!(w.as_loss().get(), x.wrapping_neg());
    assert_eq!(w.as_loss().as_gain(), w);
}

/// Every i16 of cdB through every mode, and of µNp back: what the proptests sample for i32, in full.
#[cfg(not(kani))]
#[test]
fn every_i16() {
    for x in i16::MIN..=i16::MAX {
        for r in super::reference::MODES {
            db_to_np(x, r);
            np_to_db(i32::from(x), r);
        }
        loss_round_trip(x);
        if x % 7 == 0 {
            literal_in_db(i32::from(x));
        }
    }
}

#[cfg(not(any(kani, fuzzing)))]
mod tests {
    use super::*;
    use crate::strategy;
    #[cfg(any(feature = "std", feature = "libm"))]
    use exact_units::{
        prefix::One,
        units::{Per, Volt, Watt},
    };
    use proptest::prelude::*;

    #[cfg(any(feature = "std", feature = "libm"))]
    /// A gain from 10⁻⁹ to 10⁹.
    fn gain() -> impl Strategy<Value = f64> {
        (-9.0f64..9.0).prop_map(|e| 10f64.powf(e))
    }

    proptest! {
        #[test]
        fn np_to_db_i32(x in strategy::i32(), r in strategy::mode()) {
            np_to_db(x, r);
        }

        /// µNp within a few steps of a half cdB, `(2k + 1) · ln10 / (4 · 10²⁶)`: where a rounding by a factor with
        /// ln 10 in it can go the wrong way, which random inputs rarely reach.
        #[test]
        fn np_to_db_near_half(k in -16_000i128..16_000, d in -3i128..=3, r in strategy::mode()) {
            let x = (2 * k + 1) * LN10 / (4 * 10_i128.pow(26)) + d;
            np_to_db(i32::try_from(x).unwrap(), r);
        }

        /// Literals up to ±10 kdB: the reference's product with ln 10 stays in `i128`.
        #[test]
        fn literal_in_db_wide(num in -1_000_000i32..=1_000_000) {
            literal_in_db(num);
        }

        /// A gain to a level and back is the gain within a few ulps, a loss is the gain's negation, and a power
        /// gain has the level of its root-power twin.
        #[cfg(any(feature = "std", feature = "libm"))]
        #[test]
        fn gain_edge(g in gain()) {
            let v = Fixed::<Per<Volt, Volt>, One, f64>::new(g);
            let db: Fixed<Decibel, One, f64> = v.to_level().unwrap();
            let back: Fixed<Per<Volt, Volt>, One, f64> = db.to_gain().unwrap();
            prop_assert!((back.get() / g - 1.0).abs() < 1e-13, "{g}: {}", back.get());
            let loss: Fixed<Loss<Neper>, One, f64> = v.to_level().unwrap();
            let np: Fixed<Neper, One, f64> = v.to_level().unwrap();
            prop_assert_eq!(loss.get().to_bits(), (-np.get()).to_bits());
            let p = Fixed::<Per<Watt, Watt>, One, f64>::new(g * g);
            let pdb: Fixed<Decibel, One, f64> = p.to_level().unwrap();
            prop_assert!((pdb.get() - db.get()).abs() <= 1e-12 * db.get().abs().max(1.0));
        }
    }
}

#[cfg(kani)]
mod proofs {
    use super::*;

    /// Every i16 gain into a loss and back, by both policies.
    #[kani::proof]
    fn loss_round_trip_i16() {
        loss_round_trip(kani::any());
    }
}
