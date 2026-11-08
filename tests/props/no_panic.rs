//! Property 5: conversions never panic and never give a wrong value, for any input: a float into an integer storage,
//! factors whose denominator is past `i64::MAX` or `i128::MAX` (typed, between points, offsets), bounds in decimal,
//! levels far below one step. Each of these once had a bug the cast lints found; the property would have caught it.
#![expect(
    clippy::as_conversions,
    reason = "a test casts the values it chose and the references it computes; the cast lints say why each fits"
)]
#![expect(
    clippy::arithmetic_side_effects,
    reason = "a test computes with the values it chose, which fit; an overflow fails the test"
)]

use super::rounding::Int;
use exact_units::units::{Kelvin, Ratio};
use exact_units::{Bounds, Fixed, FixedScale, Offset, Origin, Point, Rounding};

/// `T::from_f32(x)` is `Some` exactly for a whole `x` in `T`'s range, and then holds `x`.
pub(crate) fn from_f32<T: Int>(x: f32) {
    let got = T::from_f32(x).map(Into::into);
    // a whole f32 below 2¹⁰⁰ is an i128 exactly; anything from there on is past every storage
    #[expect(
        clippy::float_cmp,
        reason = "whole is an exact question: x equals its truncation or it doesn't"
    )]
    let whole = x == trunc(x);
    let want = if x.is_finite() && x.abs() < 1e30 && whole {
        #[expect(
            clippy::cast_possible_truncation,
            reason = "whole and below 2¹⁰⁰: exact"
        )]
        let i = x as i128;
        (T::MIN_INT..=T::MAX_INT).contains(&i).then_some(i)
    } else {
        None
    };
    assert_eq!(got, want, "{x:e} ({:#x})", x.to_bits());
}

/// `T::from_float(x)` is `Some` exactly for a whole `x` in `T`'s range, and then holds `x`.
pub(crate) fn from_f64<T: Int>(x: f64) {
    let got = T::from_float(x).map(Into::into);
    #[expect(
        clippy::float_cmp,
        reason = "whole is an exact question: x equals its truncation or it doesn't"
    )]
    let whole = x == x.trunc();
    // a whole f64 below 2¹⁰⁰ is an i128 exactly; anything from there on is past every storage
    let want = if x.is_finite() && x.abs() < 1e30 && whole {
        #[expect(
            clippy::cast_possible_truncation,
            reason = "whole and below 2¹⁰⁰: exact"
        )]
        let i = x as i128;
        (T::MIN_INT..=T::MAX_INT).contains(&i).then_some(i)
    } else {
        None
    };
    assert_eq!(got, want, "{x:e} ({:#x})", x.to_bits());
}

/// `i + d + f`: a whole float near an integer `i` of up to 64 bits (rounded to 53 bits), `f` the fraction.
#[expect(
    clippy::cast_precision_loss,
    reason = "the nearest float to a 64-bit integer is the point"
)]
pub(crate) fn near(i: i64, d: i32, f: f64) -> f64 {
    i as f64 + f64::from(d) + f
}

/// `x` rounded toward zero, without std: the bits below the binary point cleared.
fn trunc(x: f32) -> f32 {
    let bits = x.to_bits();
    let exp = i32::try_from((bits >> 23) & 0xff).unwrap_or(0) - 127;
    if exp >= 23 {
        x
    } else if exp < 0 {
        f32::from_bits(bits & 0x8000_0000)
    } else {
        f32::from_bits(bits & !((1u32 << (23 - exp)) - 1))
    }
}

/// The f32 bounds of `RangeDec<MIN, MAX, EXP>` are the floats just inside the decimal bounds, against the exact
/// decimals: `MIN` at or above `MIN · 10^EXP` and the float below it under it, `MAX` at or below `MAX · 10^EXP` and
/// the float above it over it. So a float compare against them is the exact compare against the decimals.
#[cfg(not(any(kani, fuzzing)))]
pub(crate) fn range_dec_f32<B: Bounds<f32>>(min: i64, max: i64, exp: i8) {
    let (lo, hi) = (B::MIN.expect("a bound"), B::MAX.expect("a bound"));
    range_dec_check(f64::from(lo), f64::from(lo.next_down()), min, exp, true);
    range_dec_check(f64::from(hi), f64::from(hi.next_up()), max, exp, false);
}

/// As [range_dec_f32], for `f64` bounds.
#[cfg(not(any(kani, fuzzing)))]
pub(crate) fn range_dec_f64<B: Bounds<f64>>(min: i64, max: i64, exp: i8) {
    let (lo, hi) = (B::MIN.expect("a bound"), B::MAX.expect("a bound"));
    range_dec_check(lo, lo.next_down(), min, exp, true);
    range_dec_check(hi, hi.next_up(), max, exp, false);
}

/// `bound` on the inner side of `m · 10^exp` (at or above it for a lower bound) and `beyond`, the next float out, on
/// the outer side, exactly.
#[cfg(not(any(kani, fuzzing)))]
fn range_dec_check(bound: f64, beyond: f64, m: i64, exp: i8, lower: bool) {
    use num_rational::BigRational;
    let ten = num_bigint::BigInt::from(10).pow(u32::from(exp.unsigned_abs()));
    let d = if exp >= 0 {
        BigRational::from_integer(num_bigint::BigInt::from(m) * ten)
    } else {
        BigRational::new(m.into(), ten)
    };
    let b = BigRational::from_float(bound).expect("a finite bound");
    let o = BigRational::from_float(beyond).expect("a finite float");
    if lower {
        assert!(b >= d && o < d, "MIN {m}e{exp}: {bound}");
    } else {
        assert!(b <= d && o > d, "MAX {m}e{exp}: {bound}");
    }
}

/// `x` of a scale `P` far below one step of `P2` (`|x| · P / P2 < 1/2` for every `x` of `T`), whatever the factor's
/// denominator: `to` is `Some(0)` for 0 and `None` otherwise.
pub(crate) fn only_zero_is_whole<P: FixedScale, P2: FixedScale, T: Int>(x: T) {
    let v = Fixed::<Ratio, P, T>::new(x);
    let exact = (x.into() == 0).then_some(0);
    assert_eq!(v.to::<P2, T>().map(|f| f.get().into()), exact, "{x:?}");
}

/// As [only_zero_is_whole], and rounded: Floor and Ceil give -1, 0 or 1 by the sign, the others 0. (A factor below
/// 2⁻¹²⁷ doesn't compile for the rounding forms.)
pub(crate) fn far_below_a_step<P: FixedScale, P2: FixedScale, T: Int>(x: T, r: Rounding) {
    only_zero_is_whole::<P, P2, T>(x);
    let v = Fixed::<Ratio, P, T>::new(x);
    let x = x.into();
    let want = match r {
        Rounding::Floor if x < 0 => -1,
        Rounding::Ceil if x > 0 => 1,
        _ => 0,
    };
    assert_eq!(
        v.to_rounded::<P2, T>(r).map(|f| f.get().into()),
        Some(want),
        "{x} {r:?}"
    );
}

/// A point of origin `O` in steps `P` into origin `O2` and steps `P2` is `(x · num + add) / den`, `Some` exactly
/// where that is whole and fits `T2`.
pub(crate) fn point_to<O, P: FixedScale, T: Int, O2, P2: FixedScale, T2: Int>(
    x: T,
    num: i128,
    add: i128,
    den: i128,
) where
    O: Origin<Unit = Kelvin>,
    O2: Origin<Unit = Kelvin, Reference = O::Reference>,
{
    let p = Point::<O, Fixed<Kelvin, P, T>>::from_raw(x);
    let got = p.to::<O2, P2, T2>().map(|q| q.raw().into());
    let n = x.into() * num + add;
    let want =
        (n % den == 0 && (T2::MIN_INT..=T2::MAX_INT).contains(&(n / den))).then_some(n / den);
    assert_eq!(got, want, "{x:?}");
}

/// `a + b` of two offsets, where it is `Some`, is the exact sum; past what the reference can add in `i128` it must be
/// `None` where an operand's denominator is past `i128` (the sum's can't fit 64 bits). `Display` shows the sign.
pub(crate) fn offset_add(a: Offset, b: Offset) {
    for o in [a, b] {
        let s = o.to_string();
        assert_eq!(s.starts_with('-'), o.is_negative(), "{o:?} shows as {s}");
        assert_eq!(o.to_f64() < 0.0, o.is_negative(), "{o:?}");
    }
    let got = a.checked_add(b);
    let (Some((an, ad)), Some((bn, bd))) = (a.to_ratio(), b.to_ratio()) else {
        assert_eq!(got, None, "{a:?} + {b:?}");
        return;
    };
    let exact = (|| {
        let ad = i128::try_from(ad).ok()?;
        let bd = i128::try_from(bd).ok()?;
        let n = an.checked_mul(bd)?.checked_add(bn.checked_mul(ad)?)?;
        let d = ad.checked_mul(bd)?;
        let g = gcd(n.unsigned_abs(), d.unsigned_abs());
        Some((n / i128::try_from(g).ok()?, d.unsigned_abs() / g))
    })();
    match (got, exact) {
        (Some(s), Some(want)) => assert_eq!(s.to_ratio(), Some(want), "{a:?} + {b:?}"),
        (got, None) => assert!(
            got.is_none(),
            "{a:?} + {b:?} is {got:?}, past what i128 adds"
        ),
        (None, Some(_)) => {}
    }
}

fn gcd(mut a: u128, mut b: u128) -> u128 {
    while b != 0 {
        (a, b) = (b, a % b);
    }
    a
}

exact_units::scale! {
    /// Steps of 3·2⁻⁶³: into kilo the factor's denominator is 2⁶⁶·125, into one it is 2⁶³, both past i64.
    pub(crate) struct Tiny = "", Exact(3 / 1, pow2 -63);
}

exact_units::scale! {
    /// Steps of 10⁻¹⁹·2⁻⁶³/3: into one the factor's denominator is past i128.
    pub(crate) struct Tinier = "", Exact(1 / 3, pow10 -19, pow2 -63);
}

/// A level of `num · 10^-decimals` dB, at or below 0, as a power gain in an u64: 1 above −3.0103 dB (half the power),
/// 0 below, however far.
pub(crate) fn level_at_most_0_db(num: i64, decimals: u8) {
    use exact_units::decibel::Decibels;
    use exact_units::prefix::One;
    use exact_units::units::{Per, Watt};
    let g = Fixed::<Per<Watt, Watt>, One, u64>::from_db_const_rounded(
        Decibels::new(num, decimals),
        Rounding::NearestEven,
    )
    .get();
    // −10·log10(2) dB = −3.01029995663981… dB, in units of 10⁻⁹ dB
    let nano = i128::from(num) * 10i128.pow(9 - u32::from(decimals));
    assert_eq!(g, u64::from(nano > -3_010_299_957), "{num}e-{decimals} dB");
}

/// A level of `num · 10^-decimals` Np, at or below 0, as a power gain in an u64: 1 above −ln(2)/2 Np = −0.34657359 Np
/// (half the power), 0 below, however far.
pub(crate) fn neper_at_most_0(num: i64, decimals: u8) {
    use exact_units::decibel::Nepers;
    use exact_units::prefix::One;
    use exact_units::units::{Per, Watt};
    let g = Fixed::<Per<Watt, Watt>, One, u64>::from_np_const_rounded(
        Nepers::new(num, decimals),
        Rounding::NearestEven,
    )
    .get();
    // −ln(2)/2 Np = −0.346573590279972…, in units of 10⁻⁹ Np
    let nano = i128::from(num) * 10i128.pow(9 - u32::from(decimals));
    assert_eq!(g, u64::from(nano >= -346_573_590), "{num}e-{decimals} Np");
}

#[cfg(not(kani))]
mod tests {
    use super::{Tinier, Tiny};
    use crate::strategy;
    use exact_units::origins::{AbsoluteZero, Celsius};
    use exact_units::prefix::{Centi, Kilo, Milli, One, Rational};
    use exact_units::{Magnitude, Offset, RangeDec};
    use proptest::prelude::*;

    /// Floats at the ends of the integer storages: `±2^k` and a few ulps around it.
    fn edges() -> Vec<f32> {
        let mut v = Vec::new();
        for k in 0..=66 {
            let bits: u32 = (127 + k) << 23;
            for ulps in -3i32..=3 {
                let x = f32::from_bits(bits.saturating_add_signed(ulps));
                v.extend([x, -x]);
            }
        }
        v.extend([
            0.5,
            -0.5,
            1.5,
            f32::NAN,
            f32::INFINITY,
            f32::NEG_INFINITY,
            f32::MAX,
            f32::MIN,
        ]);
        v
    }

    fn float() -> impl Strategy<Value = f32> {
        prop_oneof![
            any::<u32>().prop_map(f32::from_bits),
            proptest::sample::select(edges())
        ]
    }

    /// Every edge float into every integer storage.
    #[test]
    fn from_f32_at_the_ends() {
        for x in edges() {
            super::from_f32::<i8>(x);
            super::from_f32::<i16>(x);
            super::from_f32::<i32>(x);
            super::from_f32::<i64>(x);
            super::from_f32::<u8>(x);
            super::from_f32::<u16>(x);
            super::from_f32::<u32>(x);
            super::from_f32::<u64>(x);
        }
    }

    /// Every edge float, as an f64, into every integer storage.
    #[test]
    fn from_f64_at_the_ends() {
        let mut edges: Vec<f64> = edges().into_iter().map(f64::from).collect();
        // the f64 ends: 2^k and its neighbours, and the ends of the 64-bit storages
        for k in 0..=66 {
            let bits: u64 = (1023 + k) << 52;
            for ulps in -3i64..=3 {
                let x = f64::from_bits(bits.wrapping_add_signed(ulps));
                edges.extend([x, -x]);
            }
        }
        for x in edges {
            super::from_f64::<i8>(x);
            super::from_f64::<i16>(x);
            super::from_f64::<i32>(x);
            super::from_f64::<i64>(x);
            super::from_f64::<u8>(x);
            super::from_f64::<u16>(x);
            super::from_f64::<u32>(x);
            super::from_f64::<u64>(x);
        }
    }

    proptest! {
        #[test]
        fn from_f64_anywhere(x in any::<u64>().prop_map(f64::from_bits)) {
            super::from_f64::<i8>(x);
            super::from_f64::<i32>(x);
            super::from_f64::<i64>(x);
            super::from_f64::<u32>(x);
            super::from_f64::<u64>(x);
        }

        #[test]
        fn from_f64_near_whole(i in strategy::i64(), d in -2i32..=2, f in proptest::sample::select(&[0.0, 0.5, 0.25][..])) {
            let x = super::near(i, d, f);
            super::from_f64::<i8>(x);
            super::from_f64::<i16>(x);
            super::from_f64::<i32>(x);
            super::from_f64::<i64>(x);
            super::from_f64::<u64>(x);
        }
    }

    macro_rules! range_dec {
        ($($min:literal ..= $max:literal: $($exp:literal)*;)*) => {
            /// Decimal bounds at every exponent in reach of an f32's digits.
            #[test]
            fn range_dec_f32_bounds() {
                $($(super::range_dec_f32::<RangeDec<$min, $max, $exp>>($min, $max, $exp);)*)*
            }

            /// The same in f64.
            #[test]
            fn range_dec_f64_bounds() {
                $($(super::range_dec_f64::<RangeDec<$min, $max, $exp>>($min, $max, $exp);)*)*
            }
        };
    }

    range_dec! {
        -5 ..= 5: -12 -9 -6 -5 -4 -3 -2 -1 0 1 2 3 6 9 12;
        -7 ..= 13: -12 -9 -6 -5 -4 -3 -2 -1 0 1 2 3 6 9 12;
        0 ..= 25: -12 -9 -6 -5 -4 -3 -2 -1 0 1 2 3 6 9 12;
        -1 ..= 1: -30 -20 -10 -7 -1 0 1 7 10 20 30;
        -999 ..= 999: -15 -8 -7 -6 -4 -3 -2 -1 0 1 2 3 4;
        -33 ..= 3300: -9 -6 -4 -3 -2 0 3;
        -16_777_217 ..= 16_777_217: -9 -3 0 3;
        -123_456_789 ..= 987_654_321: -12 -9 -6 -3 0 3;
    }

    /// A Magnitude with a denominator from 1 to past i128: a ratio of u32s times powers of 2, 3 and 10.
    fn magnitude() -> impl Strategy<Value = Magnitude> {
        (
            1..=u32::MAX,
            1..=u32::MAX,
            -63i8..=63,
            -63i8..=63,
            0..=45u32,
        )
            .prop_filter_map("a magnitude", |(n, d, p2, p10, p3)| {
                Magnitude::checked_ratio(u64::from(n), u64::from(d))
                    .ok()?
                    .checked_mul(Magnitude::pow2(p2))
                    .ok()?
                    .checked_mul(Magnitude::pow10(p10))
                    .ok()?
                    .checked_mul(Magnitude::checked_ratio(1, 3u64.pow(p3.min(40))).ok()?)
                    .ok()
            })
    }

    fn offset() -> impl Strategy<Value = Offset> {
        prop_oneof![
            (any::<bool>(), magnitude()).prop_map(|(n, m)| Offset::new(n, m)),
            (-1000..=1000i64).prop_map(Offset::int),
        ]
    }

    /// 1/(2⁶³·3⁴¹): a denominator between i128::MAX and u128::MAX, the window the random magnitudes hit only now
    /// and then, alone, negated and with 1 added.
    #[test]
    fn an_offset_past_i128() {
        let m = Magnitude::pow2(-63)
            .checked_mul(Magnitude::checked_ratio(1, 3u64.pow(40)).expect("3⁴⁰ fits u64"))
            .and_then(|m| m.checked_mul(Magnitude::checked_ratio(1, 3).expect("1/3")))
            .expect("2⁻⁶³·3⁻⁴¹ is a magnitude");
        for neg in [false, true] {
            super::offset_add(Offset::new(neg, m), Offset::int(1));
            super::offset_add(Offset::new(neg, m), Offset::new(neg, m));
        }
    }

    proptest! {
        #[test]
        fn from_f32(x in float()) {
            super::from_f32::<i8>(x);
            super::from_f32::<i32>(x);
            super::from_f32::<i64>(x);
            super::from_f32::<u64>(x);
        }

        /// 3·2⁻⁶³ into kilo and one: denominators past i64; 10⁻¹⁹·2⁻⁶³/3 into one, past i128.
        #[test]
        fn tiny_steps(x in strategy::i32(), y in strategy::i64(), r in strategy::mode()) {
            super::far_below_a_step::<Tiny, Kilo, i32>(x, r);
            super::far_below_a_step::<Tiny, One, i32>(x, r);
            super::far_below_a_step::<Tiny, Kilo, i64>(y, r);
            super::only_zero_is_whole::<Tinier, One, i64>(y);
            super::only_zero_is_whole::<Tinier, One, i32>(x);
        }

        /// 0.01 °C into mK and back; 10⁻¹⁹ K steps into 2⁴⁵ K steps of one origin, a denominator past i64.
        #[test]
        fn points(c in strategy::i16(), mk: u32, tiny in strategy::i32()) {
            super::point_to::<Celsius, Centi, i16, AbsoluteZero, Milli, u32>(c, 10, 273_150, 1);
            super::point_to::<AbsoluteZero, Milli, u32, Celsius, Centi, i16>(mk, 1, -273_150, 10);
            super::point_to::<Celsius, Rational<1, 10_000_000_000_000_000_000>, i32, Celsius, Rational<{ 1 << 45 }, 1>, i32>(
                tiny, 1, 0, 10_000_000_000_000_000_000 << 45,
            );
        }

        #[test]
        fn offsets(a in offset(), b in offset()) {
            super::offset_add(a, b);
        }

        #[test]
        fn levels_at_most_0_db(num in i64::MIN..=0, decimals in 0..=9u8) {
            super::level_at_most_0_db(num, decimals);
        }

        #[test]
        fn nepers_at_most_0(num in i64::MIN..=0, decimals in 0..=9u8) {
            super::neper_at_most_0(num, decimals);
        }

        /// Levels where the power gain's exponent of 2 is a few hundred past a multiple of −2³²: an exponent cast
        /// to u32 wraps to a small shift there, in windows of about 1000 dB that uniform levels never hit.
        #[test]
        fn levels_near_a_wrap(k in 1..=700_000_000i64, s in -300..=300i64) {
            // a power of 2⁻ⁿ is −n · 10·log10(2) dB, to 19 digits: off by less than 10⁻⁶ in n, k up to 7·10⁸
            let n = i128::from(k) * (1 << 32) + i128::from(s);
            let num = i64::try_from(-n * 3_010_299_956_639_811_952 / 10i128.pow(18)).expect("k · 2³² · 3.01 fits i64");
            super::level_at_most_0_db(num, 0);
        }
    }
}

#[cfg(kani)]
mod proofs {
    use super::{Tiny, far_below_a_step, from_f32};
    use crate::reference::MODES;
    use exact_units::prefix::Kilo;

    /// Every f32 into an i64 (the 2⁶³ bug) and an i32.
    #[kani::proof]
    fn from_f32_i64() {
        from_f32::<i64>(kani::any());
    }

    /// Every i32 of 3·2⁻⁶³ into kilo: the denominator 2⁶⁶·125, past i64.
    #[kani::proof]
    fn tiny_to_kilo_i32() {
        let r = MODES[usize::from(kani::any_where(|i: &u8| *i < 5))];
        far_below_a_step::<Tiny, Kilo, i32>(kani::any(), r);
    }
}
