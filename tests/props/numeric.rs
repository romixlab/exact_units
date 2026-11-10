//! Property 11: the numeric claims of the docs, each against `f64` or exact integers.
//!
//! - CORDIC (`src/cordic.rs`): a result is the correctly rounded value or, where that lies within a hundredth of a
//!   step of a tie, its neighbour; so within 0.51 of a step. `sin_cos_16` at every `Bam16`, `sin_cos_32_q15` at every
//!   `Bam16` and between, `arg` of every `i8` pair and of a point at every `Bam16` direction on circles of `i16`.
//! - Integer roots: `abs` of a complex `i8` (its `norm_sqr` a `u16`) is the root rounded by each mode, for every pair;
//!   `isqrt_u32` for every `u32` under Kani.
//! - `Mac` headroom: a part of one product of `i16`s lies in −2³¹ + 2¹⁵ ..= 2³¹ (Kani, every input), and `HEADROOM` of
//!   them fit the `i64`; the same algebra for `i32` into `i128`.
//! - `Mac::mean_at_least` is `Σ re ≥ threshold · n` read as a quotient, and `norm_sqr_at` the nearest |z|² to the level.
//! - `num::sqrt_f64` is `f64::sqrt` bit for bit.
#![expect(
    clippy::arithmetic_side_effects,
    reason = "a test computes with the values it chose, which fit; an overflow fails the test"
)]

#[cfg(not(kani))]
use core::f64::consts::TAU;
use exact_units::prefix::{Pow2, Product};
use exact_units::units::{Ratio, Times};
use exact_units::{Complex, Fixed, IqStorage, Mac, Rounding};

/// The step's share of a tie the docs allow a CORDIC result to miss: a hundredth.
#[cfg(not(kani))]
const NEAR_TIE: f64 = 0.01;

/// `got` is `exact` (in steps) rounded to the nearest, or a neighbour where `exact` is within [NEAR_TIE] of a tie;
/// `exact` clamped to `lo..=hi`, what the storage holds. `turn` is the steps of a full turn for an angle (the error
/// taken modulo it), 0 for a value.
#[cfg(not(kani))]
fn cordic_rounded(got: f64, exact: f64, lo: f64, hi: f64, turn: f64, what: &str) {
    let exact = exact.clamp(lo, hi);
    let mut e = got - exact;
    if turn > 0.0 {
        e = e.rem_euclid(turn);
        if e > turn / 2.0 {
            e -= turn;
        }
    }
    let tie = (exact - exact.floor() - 0.5).abs() < NEAR_TIE;
    // the nearest is within half a step; at the clamp (cos 0 is 1 - 2⁻¹⁵) the clamped value
    assert!(
        e.abs() <= 0.5 || (tie && e.abs() < 0.5 + NEAR_TIE),
        "{what}: {got}, exact {exact}, error {e}"
    );
}

/// `sin_cos_16(a)` against `f64` (std's `sin_cos`), in Q15.
#[cfg(not(kani))]
pub(crate) fn sin_cos_16(a: u16) {
    use exact_units::{Bam16, cordic};
    let (s, c) = cordic::sin_cos_16(Bam16::of(a));
    let (es, ec) = (f64::from(a) / 65536.0 * TAU).sin_cos();
    cordic_rounded(
        f64::from(s.get()),
        es * 32768.0,
        -32768.0,
        32767.0,
        0.0,
        &format!("sin {a}"),
    );
    cordic_rounded(
        f64::from(c.get()),
        ec * 32768.0,
        -32768.0,
        32767.0,
        0.0,
        &format!("cos {a}"),
    );
}

/// `sin_cos_32_q15(a)` against `f64`, in Q15.
#[cfg(not(kani))]
pub(crate) fn sin_cos_32_q15(a: u32) {
    use exact_units::{Bam32, cordic};
    let (s, c) = cordic::sin_cos_32_q15(Bam32::of(a));
    let (es, ec) = (f64::from(a) / 4_294_967_296.0 * TAU).sin_cos();
    cordic_rounded(
        f64::from(s.get()),
        es * 32768.0,
        -32768.0,
        32767.0,
        0.0,
        &format!("sin {a:#x}"),
    );
    cordic_rounded(
        f64::from(c.get()),
        ec * 32768.0,
        -32768.0,
        32767.0,
        0.0,
        &format!("cos {a:#x}"),
    );
}

/// `arg` of `x + jy` in `i16` parts against `f64::atan2`, in steps of 2⁻¹⁶ turn; (0, 0) gives 0.
#[cfg(not(kani))]
pub(crate) fn arg_16(x: i16, y: i16) {
    let z = Complex::<Ratio, Pow2<-15>, i16>::new(x, y);
    if (x, y) == (0, 0) {
        assert_eq!(z.arg().get(), 0, "arg 0");
        return;
    }
    let got = f64::from(z.arg().get());
    let exact = (f64::from(y).atan2(f64::from(x)) / TAU * 65536.0).rem_euclid(65536.0);
    cordic_rounded(got, exact, 0.0, 65536.0, 65536.0, &format!("arg({x}, {y})"));
}

/// `arg` of `x + jy` in `i8` parts, as [arg_16] (it widens them).
#[cfg(not(kani))]
pub(crate) fn arg_8(x: i8, y: i8) {
    let z = Complex::<Ratio, Pow2<-7>, i8>::new(x, y);
    arg_16(x.into(), y.into());
    let wide = Complex::<Ratio, Pow2<-15>, i16>::new(x.into(), y.into())
        .arg()
        .get();
    assert_eq!(z.arg().get(), wide, "arg({x}, {y}) of i8 against i16");
}

/// `abs_rounded(r)` of an `i8` pair is √(x² + y²) rounded by `r`: Floor and Truncate at or below, Ceil at or above,
/// each within a step; the nearest ones within half a step (a root of an integer is never a tie).
pub(crate) fn abs_8(x: i8, y: i8, r: Rounding) {
    let z = Complex::<Ratio, Pow2<-7>, i8>::new(x, y);
    let n = u32::from(z.abs_rounded(r).get());
    let sq = i32::from(x) * i32::from(x) + i32::from(y) * i32::from(y);
    let sq = u32::try_from(sq).unwrap_or(0);
    check_root(sq, n, r);
}

/// `n` is √`x` rounded by `r`, from the definition: n² against x, and (n ± ½)² = n² ± n + ¼ for the nearest.
fn check_root(x: u32, n: u32, r: Rounding) {
    let (x, n) = (u64::from(x), u64::from(n));
    let ok = match r {
        Rounding::Truncate | Rounding::Floor => n * n <= x && (n + 1) * (n + 1) > x,
        Rounding::Ceil => n * n >= x && (n == 0 || (n - 1) * (n - 1) < x),
        // n² - n < x ≤ n² + n
        Rounding::NearestEven | Rounding::NearestAway | Rounding::NearestCeil => {
            (n == 0 || n * n - n < x) && n * n + n >= x
        }
    };
    assert!(ok, "√{x} {r:?}: {n}");
}

/// `isqrt_u32(x, r)` is √x rounded by `r`.
pub(crate) fn isqrt_u32(x: u32, r: Rounding) {
    check_root(x, exact_units::cordic::isqrt_u32(x, r), r);
}

/// The two parts of the product `(a + jb)(c + jd)` and of `(a + jb)·conj(c + jd)`, as `IqStorage::mac` adds them to
/// an empty sum, are the exact integers and lie in −2²ⁿ⁻¹ + 2ⁿ⁻¹ ..= 2²ⁿ⁻¹ for `n` = 16: the bound the `Mac` docs
/// build `HEADROOM` on. `HEADROOM` products at either end fit the `i64`.
pub(crate) fn mac_part_16(a: i16, b: i16, c: i16, d: i16) {
    let (wa, wb, wc, wd) = (i64::from(a), i64::from(b), i64::from(c), i64::from(d));
    let parts = [
        (
            <i16 as IqStorage>::mac(0, (a, c), (b, d), true),
            wa * wc - wb * wd,
        ),
        (
            <i16 as IqStorage>::mac(0, (a, d), (b, c), false),
            wa * wd + wb * wc,
        ),
        (
            <i16 as IqStorage>::mac(0, (a, c), (b, d), false),
            wa * wc + wb * wd,
        ),
        (
            <i16 as IqStorage>::mac(0, (b, c), (a, d), true),
            wb * wc - wa * wd,
        ),
    ];
    for (got, exact) in parts {
        assert!(got == exact, "{a} {b} {c} {d}: {got} != {exact}");
        assert!(
            (-(1i64 << 31) + (1 << 15)..=1 << 31).contains(&got),
            "{a} {b} {c} {d}: {got}"
        );
    }
    headroom::<i16>(16);
}

/// `HEADROOM` products of the largest part, or of the most negative, sum within the accumulator of `T` (`n` bits).
pub(crate) fn headroom<T: IqStorage>(n: u32) {
    let h = i128::from(T::HEADROOM);
    let (top, bottom) = (
        1i128 << (2 * n - 1),
        -(1i128 << (2 * n - 1)) + (1 << (n - 1)),
    );
    // the accumulator has 4n bits
    let (min, max) = if 4 * n >= 128 {
        (i128::MIN, i128::MAX)
    } else {
        (-(1i128 << (4 * n - 1)), (1i128 << (4 * n - 1)) - 1)
    };
    let (hi, lo) = (h.checked_mul(top), h.checked_mul(bottom));
    assert!(hi.is_some_and(|s| s <= max), "{h} · {top} past {max}");
    assert!(lo.is_some_and(|s| s >= min), "{h} · {bottom} past {min}");
    // and one product more passes the top
    assert!(
        (h + 1).checked_mul(top).is_none_or(|s| s > max),
        "HEADROOM could be larger"
    );
}

/// The power sum of Q15 samples: `Σ re` in Q30.
type PowerMac = Mac<Times<Ratio, Ratio>, Product<Pow2<-15>, Pow2<-15>>, i16>;

/// A threshold in the sum's unit and scale.
type PowerQ30<T> = Fixed<Times<Ratio, Ratio>, Product<Pow2<-15>, Pow2<-15>>, T>;

/// `mean_at_least(t, n)` of a sum of `products` (each `a · conj(b)`) is whether the mean `Σ re / n` is at least `t`:
/// for an integer `t`, `⌊Σ re / n⌋ ≥ t`; `false` for `n = 0`. Thresholds in `i32`, `u32` and `i64`.
pub(crate) fn mean_at_least(products: &[(i16, i16, i16, i16)], t: i64, n: u32) {
    let mut acc = PowerMac::new();
    // at most 2³¹ a product: an i64 holds the sum of any slice a test makes
    let mut re = 0i64;
    for &(a, b, c, d) in products {
        acc.mac_conj(
            Complex::<Ratio, Pow2<-15>, i16>::new(a, b),
            Complex::new(c, d),
        );
        re += i64::from(a) * i64::from(c) + i64::from(b) * i64::from(d);
    }
    let n_usize = usize::try_from(n).unwrap_or(usize::MAX);
    let want = |t: i128| n != 0 && i128::from(re.div_euclid(i64::from(n))) >= t;
    assert_eq!(
        acc.mean_at_least(PowerQ30::<i64>::new(t), n_usize),
        want(t.into()),
        "{re} / {n} vs {t}"
    );
    if let Ok(t) = i32::try_from(t) {
        assert_eq!(
            acc.mean_at_least(PowerQ30::<i32>::new(t), n_usize),
            want(t.into()),
            "{re} / {n} vs {t}"
        );
    }
    if let Ok(t) = u32::try_from(t) {
        assert_eq!(
            acc.mean_at_least(PowerQ30::<u32>::new(t), n_usize),
            want(t.into()),
            "{re} / {n} vs {t}"
        );
    }
}

/// `norm_sqr_at_rounded(level, NearestEven)` of Q15 values is the nearest |z|² in Q30 to 10^(dB/10) (a root-power kind), so
/// `norm_sqr() >= norm_sqr_at_rounded(level, NearestEven)` is |z|² ≥ the level less half a step: the compare is exact up to that one
/// rounding.
#[cfg(not(kani))]
pub(crate) fn norm_sqr_at(num: i64, decimals: u8, x: i16, y: i16) {
    use exact_units::decibel::Decibels;
    use exact_units::units::{Per, Volt};
    type Gamma = Complex<Per<Volt, Volt>, Pow2<-15>, i16>;
    let db = Decibels::new(num, decimals);
    let level = Gamma::norm_sqr_at_rounded(db, Rounding::NearestEven).get();
    let exact = 10f64.powf(db.to_f64() / 10.0) * f64::from(1u32 << 30);
    assert!(
        (f64::from(level) - exact).abs() <= 0.5 + exact * 1e-12,
        "{} dB: {level}, exact {exact}",
        db.to_f64()
    );
    let z = Gamma::new(x, y);
    let n = z.norm_sqr().get();
    let above = n >= level;
    let margin = (f64::from(n) - (exact - 0.5)).abs();
    if margin > 1e-6 * exact.max(1.0) {
        assert_eq!(
            above,
            f64::from(n) > exact - 0.5,
            "|z|² {n} against {exact}"
        );
    }
}

/// `num::sqrt_f64(x)` (a `const fn`) is `f64::sqrt(x)`, bit for bit; any NaN for a NaN.
#[cfg(not(kani))]
pub(crate) fn sqrt_f64(bits: u64) {
    let x = f64::from_bits(bits);
    let (got, want) = (exact_units::num::sqrt_f64(x), x.sqrt());
    assert!(
        got.to_bits() == want.to_bits() || (got.is_nan() && want.is_nan()),
        "√{x:e} ({bits:#x}): {got:e} ({:#x}), std {want:e} ({:#x})",
        got.to_bits(),
        want.to_bits()
    );
}

#[cfg(not(any(kani, fuzzing)))]
mod tests {
    use crate::reference::MODES;
    use crate::strategy;
    use core::f64::consts::TAU;
    use proptest::collection::vec;
    use proptest::prelude::*;

    #[test]
    fn sin_cos_16_every_angle() {
        for a in 0..=u16::MAX {
            super::sin_cos_16(a);
        }
    }

    /// Every Bam16 angle as a Bam32, and halfway to the next and just below it.
    #[test]
    fn sin_cos_32_q15_every_bam16() {
        for a in 0..=u32::from(u16::MAX) {
            for low in [0, 0x8000, 0xffff] {
                super::sin_cos_32_q15(a << 16 | low);
            }
        }
    }

    /// Every pair of i8 parts.
    #[test]
    fn arg_every_i8_pair() {
        for x in i8::MIN..=i8::MAX {
            for y in i8::MIN..=i8::MAX {
                super::arg_8(x, y);
            }
        }
    }

    /// A point at every Bam16 direction, on circles from full scale down to a few steps.
    #[test]
    fn arg_every_direction() {
        for a in 0..=u16::MAX {
            let (s, c) = (f64::from(a) / 65536.0 * TAU).sin_cos();
            for r in [32767.0, 23170.0, 4096.0, 181.0, 12.0] {
                #[expect(clippy::cast_possible_truncation, reason = "at most 32767 in size")]
                let (x, y) = ((r * c).round() as i16, (r * s).round() as i16);
                super::arg_16(x, y);
            }
        }
    }

    /// Every i8 pair, every mode: the norm is a u16.
    #[test]
    fn abs_every_i8_pair() {
        for x in i8::MIN..=i8::MAX {
            for y in i8::MIN..=i8::MAX {
                for r in MODES {
                    super::abs_8(x, y, r);
                }
            }
        }
    }

    /// Every u16 as the norm, every mode.
    #[test]
    fn isqrt_every_u16() {
        for x in 0..=u32::from(u16::MAX) {
            for r in MODES {
                super::isqrt_u32(x, r);
            }
        }
    }

    #[test]
    fn mac_extremes() {
        let ends = [i16::MIN, i16::MIN + 1, -1, 0, 1, i16::MAX];
        for a in ends {
            for b in ends {
                for c in ends {
                    for d in ends {
                        super::mac_part_16(a, b, c, d);
                    }
                }
            }
        }
        super::headroom::<i32>(32);
    }

    #[test]
    fn sqrt_f64_edges() {
        let mut edges = vec![
            0,
            1,
            2,
            3,
            0x000f_ffff_ffff_ffff,
            0x0010_0000_0000_0000,
            0x3ff0_0000_0000_0000,
        ];
        edges.extend(
            [
                f64::MAX,
                f64::MIN_POSITIVE,
                f64::EPSILON,
                2.0,
                4.0,
                0.25,
                f64::INFINITY,
            ]
            .map(f64::to_bits),
        );
        edges.extend([-0.0f64, -1.0, f64::NEG_INFINITY, f64::NAN, -f64::NAN].map(f64::to_bits));
        for k in 1..=60 {
            let x = f64::from(k).powi(2);
            edges.extend([x.to_bits(), x.to_bits() - 1, x.to_bits() + 1]);
        }
        for b in edges {
            super::sqrt_f64(b);
        }
    }

    /// Bits weighted to the subnormals and to whole squares.
    fn f64_bits() -> impl Strategy<Value = u64> {
        prop_oneof![
            3 => any::<u64>(),
            1 => 0..0x0010_0000_0000_0000u64,
            1 => (0u32..(1 << 26)).prop_map(|n| f64::from(n).powi(2).to_bits()),
        ]
    }

    proptest! {
        #[test]
        fn sin_cos_32_q15(a: u32) {
            super::sin_cos_32_q15(a);
        }

        #[test]
        fn arg_16(x in strategy::i16(), y in strategy::i16()) {
            super::arg_16(x, y);
        }

        #[test]
        fn mac_parts(a in strategy::i16(), b in strategy::i16(), c in strategy::i16(), d in strategy::i16()) {
            super::mac_part_16(a, b, c, d);
        }

        #[test]
        fn mean_at_least(
            products in vec((strategy::i16(), strategy::i16(), strategy::i16(), strategy::i16()), 0..12),
            t in prop_oneof![any::<i64>(), -(1i64 << 33)..=(1 << 33)],
            n in prop_oneof![0u32..16, any::<u32>()],
        ) {
            super::mean_at_least(&products, t, n);
        }

        #[test]
        fn norm_sqr_at(num in -900i64..=0, decimals in 0..=2u8, x in strategy::i16(), y in strategy::i16()) {
            super::norm_sqr_at(num, decimals, x, y);
        }

        #[test]
        fn sqrt_f64(bits in f64_bits()) {
            super::sqrt_f64(bits);
        }
    }
}

#[cfg(kani)]
mod proofs {
    use crate::reference::MODES;

    /// Every u32 and mode: √x rounded.
    #[kani::proof]
    fn isqrt_u32() {
        let r = MODES[usize::from(kani::any_where(|i: &u8| usize::from(*i) < MODES.len()))];
        super::isqrt_u32(kani::any(), r);
    }

    /// Every i8 pair and mode: |z| rounded, through the u16 norm.
    #[kani::proof]
    fn abs_8() {
        let r = MODES[usize::from(kani::any_where(|i: &u8| usize::from(*i) < MODES.len()))];
        super::abs_8(kani::any(), kani::any(), r);
    }

    /// Every four i16: the parts of a product, exact and within the bound of `HEADROOM`.
    #[kani::proof]
    fn mac_part_16() {
        super::mac_part_16(kani::any(), kani::any(), kani::any(), kani::any());
    }

    /// A product of any four i16, any i16 threshold, a count of up to 8. The 128-bit multiply and divide of a wider
    /// threshold or count, or of two products, run past 40 minutes; the proptest covers those.
    #[kani::proof]
    fn mean_at_least() {
        let p = [(kani::any(), kani::any(), kani::any(), kani::any())];
        let n: u8 = kani::any_where(|n| *n <= 8);
        super::mean_at_least(&p, i64::from(kani::any::<i16>()), u32::from(n));
    }
}
