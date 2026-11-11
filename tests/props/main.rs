//! Properties of the crate, each against an exact reference, checked two ways from one function: a proptest draws
//! its inputs (`just test`), a Kani proof takes them symbolic, all of them at once (`just verify`). A property is a
//! plain `pub fn` that takes its inputs and asserts; the proptests and proofs beside it only call it. Where an input
//! space is small and cheap (every i8, every i16), a test walks it whole. Properties of input from outside (text, wire
//! bytes, serde) take strings and bytes, which Kani can't reason about: a proptest draws them here, and the fuzz targets
//! of `fuzz/` (`just fuzz TARGET SECONDS`) call the same functions, coverage-guided.
#![cfg(feature = "quantities")]
#![expect(
    clippy::as_conversions,
    reason = "a test casts the values it chose and the references it computes; the cast lints say why each fits"
)]

mod arith;
#[cfg(all(feature = "bignum", not(any(kani, fuzzing))))]
mod big_fraction;
#[cfg(feature = "information")]
mod can_fd;
#[cfg(feature = "dimensionless")]
mod codec;
#[cfg(all(feature = "dimensionless", feature = "libm"))]
mod curve;
#[cfg(feature = "levels")]
mod damping;
mod dim;
mod exact;
#[cfg(not(kani))]
mod exact_text;
#[cfg(feature = "levels")]
mod exposure;
#[cfg(not(any(kani, fuzzing)))]
mod float_conv;
mod format;
#[cfg(feature = "dimensionless")]
mod image_codecs;
#[cfg(feature = "leap_seconds")]
mod leap;
#[cfg(feature = "levels")]
mod level;
mod literal;
mod magnitude;
mod masses;
#[cfg(not(kani))]
mod nan;
mod niche;
mod no_panic;
mod numeric;
#[cfg(not(kani))]
mod panics;
mod point;
mod prefix;
#[cfg(all(feature = "q", not(kani)))]
mod q_exact;
#[cfg(all(feature = "q", not(kani)))]
mod q_float;
#[cfg(all(feature = "q", feature = "electrical", not(kani)))]
mod q_level;
#[cfg(feature = "q")]
mod q_literal;
#[cfg(all(feature = "q", feature = "thermal", feature = "angle"))]
mod q_point;
#[cfg(all(feature = "q", feature = "electrical"))]
mod q_range;
#[cfg(all(feature = "q", feature = "electrical", not(kani)))]
mod q_range_level;
#[cfg(all(feature = "q", feature = "thermal", feature = "angle", not(kani)))]
mod q_range_point;
#[cfg(all(feature = "q", not(kani)))]
mod q_text;
#[cfg(feature = "radiation")]
mod radiation;
mod reference;
#[cfg(not(kani))]
mod registry;
#[cfg(all(feature = "electrical", feature = "levels", feature = "time"))]
mod rf;
mod rounding;
#[cfg(feature = "time")]
mod run_scale;
#[cfg(all(not(kani), feature = "serde", feature = "std", feature = "shrink_wrap"))]
mod serde_input;
#[cfg(feature = "serde")]
mod serde_values;
mod symbols;
#[cfg(feature = "systems")]
mod systems;
#[cfg(not(kani))]
mod text;
#[cfg(all(not(kani), feature = "serde"))]
mod unit_text;
#[cfg(feature = "dimensionless")]
mod video;
#[cfg(feature = "time")]
mod video_time;
mod window;
#[cfg(all(not(kani), feature = "std", feature = "shrink_wrap"))]
mod wire;

/// Inputs for the proptests: integers weighted to the ends and to 0, where conversions overflow and round.
#[cfg(not(any(kani, fuzzing)))]
mod strategy {
    use super::reference::{MODES, POLICIES};
    use exact_units::{Overflow, Rounding};
    use proptest::prelude::*;
    use proptest::sample::select;

    macro_rules! ints {
        ($($t:ident)*) => {$(
            /// Anything, the ends and a step in from them, or near 0.
            pub(crate) fn $t() -> impl Strategy<Value = $t> {
                prop_oneof![
                    4 => any::<$t>(),
                    1 => select(&[$t::MIN, $t::MIN + 1, $t::MAX - 1, $t::MAX][..]),
                    2 => -100..=100 as $t,
                ]
            }
        )*};
    }

    ints!(i8 i16 i32 i64);

    /// A magnitude (as an `i64`) at the edge of the kernel's `u32` path: about `u32::MAX` and `2^32`, and whole steps,
    /// halves and their neighbours of `10^9` and `10^10` up to past `2^32`.
    #[expect(
        clippy::arithmetic_side_effects,
        reason = "the values are within 6·10¹⁰, far inside i64"
    )]
    pub(crate) fn u32_edge() -> impl Strategy<Value = i64> {
        let edge = i64::from(u32::MAX);
        prop_oneof![
            3 => (edge - 1000)..=(edge + 1000),
            2 => (0i64..=5, 0i64..=1, -2i64..=2, 9u32..=10)
                .prop_map(|(k, half, d, e)| k * 10i64.pow(e) + half * 5 * 10i64.pow(e - 1) + d),
            1 => 0i64..=100,
        ]
    }

    /// Anything, the ends, near 0, and powers of 2 and 10 (the denominators of prefixes and binary fractions).
    pub(crate) fn u128s() -> impl Strategy<Value = u128> {
        prop_oneof![
            3 => any::<u128>(),
            2 => any::<u64>().prop_map(u128::from),
            1 => select(&[0, 1, 2, u128::MAX - 1, u128::MAX, 1 << 127][..]),
            2 => 0..=100u128,
            2 => (0u32..=127).prop_map(|e| 1u128 << e),
            2 => (0u32..=38).prop_map(|e| 10u128.pow(e)),
        ]
    }

    /// A fraction of [u128s] times a power of ten of [exp10s], either sign; one with no form is 0.
    pub(crate) fn fraction() -> impl Strategy<Value = exact_units::Fraction> {
        (
            any::<bool>(),
            u128s(),
            u128s().prop_filter("a denominator", |d| *d != 0),
            exp10s(),
        )
            .prop_map(|(neg, num, den, exp)| {
                exact_units::Fraction::from_parts_exp10(neg, num, den, exp).unwrap_or_default()
            })
    }

    /// Powers of ten: mostly 0 (the values of before), small ones, atomic scales, those near where a sum aligns past
    /// four limbs (about 77), and past what an `f64` holds. The ends of `i32` are [exp10_ends]: a `BigRational` of
    /// `10^2³¹` is past any reference.
    pub(crate) fn exp10s() -> impl Strategy<Value = i32> {
        prop_oneof![
            4 => Just(0),
            3 => -40i32..=40,
            2 => prop_oneof![-60i32..=-25, 25i32..=60],
            1 => 70i32..=85,
            1 => -85i32..=-70,
            1 => prop_oneof![-500i32..=-300, 300i32..=500],
        ]
    }

    /// A shift of the exponent near either end of `i32`, leaving room for [exp10s] and the steps of an operation.
    pub(crate) fn exp10_ends() -> impl Strategy<Value = i32> {
        prop_oneof![
            (i32::MAX - 2000)..=(i32::MAX - 1200),
            (i32::MIN + 1200)..=(i32::MIN + 2000),
        ]
    }

    /// Fractions where the exponent is clamped: a numerator of many 5s over a large denominator, or many 2s over a
    /// small one, so the value's power of 5 is out of the range where the mantissa fits, and the exponent is that
    /// range's end; at every exponent, the ends of `i32` included.
    pub(crate) fn clamped() -> impl Strategy<Value = exact_units::Fraction> {
        (
            any::<bool>(),
            0u32..=55,
            1u128..=1000,
            prop_oneof![(1u128 << 100)..=u128::MAX, 1u128..=1000],
            0u32..=127,
            exp10s(),
        )
            .prop_map(|(neg, fives, m, den, twos, exp)| {
                let num = 5u128.pow(fives).checked_mul(m).unwrap_or(m);
                let num = num
                    .checked_shl(twos)
                    .filter(|n| n >> twos == num)
                    .unwrap_or(num);
                exact_units::Fraction::from_parts_exp10(neg, num, den | 1, exp).unwrap_or_default()
            })
    }

    pub(crate) fn mode() -> impl Strategy<Value = Rounding> {
        select(&MODES[..])
    }

    pub(crate) fn policy() -> impl Strategy<Value = Overflow> {
        select(&POLICIES[..])
    }
}
