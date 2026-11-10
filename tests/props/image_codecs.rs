//! Properties: the integer arithmetic of image and video codecs, written with the crate's named roundings, against
//! the standards' formulas written out in `i128` (`reference::imaging`).
//!
//! - Lossless lifting: T.800's 5/3 wavelet (F.3.8.1, eq. F-5 and F-6, and its forward F.4.8.1), its reversible
//!   colour transform (G.2, eq. G-3 to G-8) and H.273's YCgCo-R (eq. 58 to 65) give back every input: each walked
//!   whole over 8-bit inputs, drawn by proptest over 16 bits, and a Kani proof over `i16`. T.800's forward F-9 as
//!   the 2002 edition prints it, ⌊·/4⌋, is not the inverse of F-6's ⌊·/2⌋: a test finds the inputs it loses.
//! - The rounding chain: T.81's two point transforms (A.4), every row of K.10's table, `Truncate` and `Floor` on
//!   every `i16` and every Pt; H.273's Round (eq. 8) is `NearestAway`; JFIF's Round (T.871 clause 7), AV1's Round2
//!   (4.7) and the add-half-then-shift of H.264 (eq. 8-337) and H.265 (eq. 8-309) are one rounding, a tie toward +∞,
//!   which none of the crate's five modes is yet; H.273's YCgCo (eq. 52, its NOTE 3) reaches 1 << BitDepthC.
#![expect(
    clippy::arithmetic_side_effects,
    reason = "a test computes with the values it chose, which fit; an overflow fails the test"
)]
#![expect(clippy::panic, reason = "a property fails by panicking")]

use super::reference::imaging as reference;
use exact_units::codec::{
    CodeSpan, Codec, CodecDef, CodecShape, Coded, HevcLevelScale, J2kStepSize,
};
#[cfg(all(feature = "information", feature = "time"))]
use exact_units::codec::{HrdBitRate, HrdCpbSize};
use exact_units::prefix::{One, Pow2, Pow10, Rational};
#[cfg(feature = "q")]
use exact_units::q;
use exact_units::system::def_list;
use exact_units::units::Ratio;
use exact_units::{Exact, Fixed, MarkerId, Overflow, Prefix, Rounding};

/// `x / 2`, rounded by `r`: `x >> 1` for `Floor`.
fn half(x: i32, r: Rounding) -> i32 {
    Fixed::<Ratio, Pow2<-1>, i32>::new(x)
        .into_rounded::<One, i32>(r)
        .get()
}

/// `x / 4`, rounded by `r`: `x >> 2` for `Floor`.
fn quarter(x: i32, r: Rounding) -> i32 {
    Fixed::<Ratio, Pow2<-2>, i32>::new(x)
        .into_rounded::<One, i32>(r)
        .get()
}

/// `x` at `i` of a signal extended both ways by T.800's periodic symmetric extension (eq. F-3, F-4), `N >= 2`.
fn ext<const N: usize>(x: &[i32; N], i: isize) -> i32 {
    let p = 2 * (N.cast_signed() - 1);
    let m = i.rem_euclid(p);
    x.get(m.min(p - m).unsigned_abs()).copied().unwrap_or(0)
}

/// T.800's forward 5/3 lifting (F.4.8.1, eq. F-9 and F-10, F-9 with the ⌊·/2⌋ its inverse F-6 undoes), `i0 = 0`.
pub(crate) fn fdwt53<const N: usize>(x: &[i32; N]) -> [i32; N] {
    let odd = |k: isize| ext(x, k) - half(ext(x, k - 1) + ext(x, k + 1), Rounding::Floor);
    core::array::from_fn(|k| {
        let k = k.cast_signed();
        if k % 2 == 1 {
            odd(k)
        } else {
            ext(x, k) + quarter(odd(k - 1) + odd(k + 1) + 2, Rounding::Floor)
        }
    })
}

/// T.800's inverse 5/3 lifting (F.3.8.1, eq. F-5 and F-6), `i0 = 0`.
pub(crate) fn idwt53<const N: usize>(y: &[i32; N]) -> [i32; N] {
    let even = |k: isize| ext(y, k) - quarter(ext(y, k - 1) + ext(y, k + 1) + 2, Rounding::Floor);
    core::array::from_fn(|k| {
        let k = k.cast_signed();
        if k % 2 == 1 {
            ext(y, k) + half(even(k - 1) + even(k + 1), Rounding::Floor)
        } else {
            even(k)
        }
    })
}

/// A signal of `N` samples, forward and back by the 5/3 lifting: the reference's coefficients, and the samples
/// again.
pub(crate) fn lifting_53_is_lossless<const N: usize>(x: [i16; N]) {
    let x = x.map(i32::from);
    let y = fdwt53(&x);
    #[cfg(not(kani))]
    {
        let wide = x.map(i128::from);
        assert_eq!(
            y.map(i128::from).to_vec(),
            reference::fdwt53(&wide, 2),
            "{x:?}"
        );
        assert_eq!(reference::idwt53(&reference::fdwt53(&wide, 2)), wide);
    }
    assert_eq!(idwt53(&y), x);
}

/// T.800's forward RCT (G.2.1, eq. G-3 to G-5): `[Y0, Y1, Y2]`.
pub(crate) fn rct(i: [i32; 3]) -> [i32; 3] {
    let [i0, i1, i2] = i;
    [quarter(i0 + 2 * i1 + i2, Rounding::Floor), i2 - i1, i0 - i1]
}

/// T.800's inverse RCT (G.2.2, eq. G-6 to G-8): `[I0, I1, I2]`.
pub(crate) fn rct_inverse(y: [i32; 3]) -> [i32; 3] {
    let [y0, y1, y2] = y;
    let i1 = y0 - quarter(y2 + y1, Rounding::Floor);
    [y2 + i1, i1, y1 + i1]
}

/// Three components, forward and back by the RCT.
pub(crate) fn rct_is_lossless(i: [i16; 3]) {
    let i = i.map(i32::from);
    let y = rct(i);
    let wide = i.map(i128::from);
    assert_eq!(y.map(i128::from), reference::rct(wide), "{i:?}");
    assert_eq!(reference::rct_inverse(reference::rct(wide)), wide);
    assert_eq!(rct_inverse(y), i, "{i:?}");
}

/// H.273's YCgCo-R (eq. 58 to 61) of `[R, G, B]` at `bits` (BitDepthY), chroma one bit wider (BitDepthC):
/// `[Y, Cg, Co]`.
pub(crate) fn ycgco_r(rgb: [i32; 3], bits: u32) -> [i32; 3] {
    let [r, g, b] = rgb;
    let mid = 1 << bits; // 1 << (BitDepthC − 1)
    let co = r - b + mid;
    let t = b + half(co - mid, Rounding::Floor);
    let cg = g - t + mid;
    [t + half(cg - mid, Rounding::Floor), cg, co]
}

/// H.273's inverse YCgCo-R (eq. 62 to 65), with its Clip3 to `0..=MaxValRGB`: `[R, G, B]`.
pub(crate) fn ycgco_r_inverse(ycc: [i32; 3], bits: u32) -> [i32; 3] {
    let [y, cg, co] = ycc;
    let mid = 1 << bits;
    let clip = |v: i32| v.clamp(0, (1 << bits) - 1);
    let t = y - half(cg - mid, Rounding::Floor);
    let g = clip(t + cg - mid);
    let b = clip(t - half(co - mid, Rounding::Floor));
    [clip(b + co - mid), g, b]
}

/// `[R, G, B]` of `bits` (8 to 16) forward and back by YCgCo-R: the reference's values, chroma within
/// BitDepthC = `bits + 1`, and the same samples again, the clip never cutting.
pub(crate) fn ycgco_r_is_lossless(rgb: [u16; 3], bits: u32) {
    let rgb = rgb.map(|v| i32::from(v) & ((1 << bits) - 1));
    let ycc = ycgco_r(rgb, bits);
    #[cfg(not(kani))]
    {
        let wide = rgb.map(i128::from);
        assert_eq!(
            ycc.map(i128::from),
            reference::ycgco_r(wide, bits + 1),
            "{rgb:?}"
        );
        assert_eq!(
            reference::ycgco_r_inverse(reference::ycgco_r(wide, bits + 1), bits + 1),
            wide
        );
    }
    let [y, cg, co] = ycc;
    assert!((0..1 << bits).contains(&y), "Y {y} of {rgb:?}");
    assert!((0..1 << (bits + 1)).contains(&cg) && (0..1 << (bits + 1)).contains(&co));
    assert_eq!(ycgco_r_inverse(ycc, bits), rgb, "{rgb:?}");
}

#[test]
fn lifting_53_every_i8_pair() {
    for a in i8::MIN..=i8::MAX {
        for b in i8::MIN..=i8::MAX {
            let (a, b) = (i16::from(a), i16::from(b));
            lifting_53_is_lossless([a, b]);
            lifting_53_is_lossless([a, b, a]);
            lifting_53_is_lossless([b, a, -a, b]);
        }
    }
}

/// F-9 as T.800 (08/2002) prints it, ⌊(X(2n) + X(2n + 2)) / 4⌋, against F-6's ⌊(X(2n) + X(2n + 2)) / 2⌋: the
/// inverse doesn't give back what that forward transform made, `[2, 0]` the first case.
#[test]
fn f9_as_printed_is_not_the_inverse_of_f6() {
    let lost = |x: &[i128]| reference::idwt53(&reference::fdwt53(x, 4)) != x;
    assert!(lost(&[2, 0]));
    assert_eq!(reference::idwt53(&reference::fdwt53(&[2, 0], 4)), [2, 1]);
    let mut kept = 0;
    let mut pairs = 0;
    for a in -128..=127 {
        for b in -128..=127 {
            pairs += 1;
            if !lost(&[a, b]) {
                kept += 1;
            }
        }
    }
    // only where X(0) + X(2) is ⌊·/4⌋ and ⌊·/2⌋ alike: X(0) = X(2) = a, so a ∈ {-1, 0}
    assert_eq!((kept, pairs), (2 * 256, 65_536));
}

#[test]
fn rct_every_i8_pair() {
    for a in i8::MIN..=i8::MAX {
        for b in i8::MIN..=i8::MAX {
            for c in [i8::MIN, -1, 0, 1, i8::MAX, a, b] {
                rct_is_lossless([a, b, c].map(i16::from));
                rct_is_lossless([c, a, b].map(i16::from));
            }
        }
    }
}

#[test]
fn ycgco_r_every_8_bit_colour() {
    for r in 0..=255 {
        for g in 0..=255 {
            for b in 0..=255 {
                ycgco_r_is_lossless([r, g, b], 8);
            }
        }
    }
}

/// `p / 2^k` rounded by `r`, with `k` known at run time: `p` in steps of 2⁻ᵏ rescaled to whole ones.
fn shift(p: i32, k: i8, r: Rounding) -> i32 {
    Prefix::from_pow2(-k)
        .and_then(|from| from.rescale(p, Prefix::Unit, r, Overflow::Checked))
        .unwrap_or_else(|| panic!("{p} / 2^{k} fits an i32"))
}

/// T.81's two point transforms (A.4) of `x` by `pt`: the integer divide of lossless samples and AC coefficients is
/// `Truncate`, the arithmetic shift of DC coefficients is `Floor`.
pub(crate) fn point_transforms(x: i16, pt: i8) {
    let x = i32::from(x);
    assert_eq!(
        shift(x, pt, Rounding::Truncate),
        x / (1 << pt),
        "{x} / 2^{pt}"
    );
    assert_eq!(shift(x, pt, Rounding::Floor), x >> pt, "{x} >> {pt}");
}

/// H.273's Round (eq. 8), Sign(x)·Floor(Abs(x) + 0.5), of `x = p / 2^k` is `NearestAway`.
pub(crate) fn h273_round_is_nearest_away(p: i16, k: i8) {
    let p = i32::from(p);
    let want = reference::round_h273(i128::from(p), 1 << k);
    assert_eq!(
        i128::from(shift(p, k, Rounding::NearestAway)),
        want,
        "Round({p} / 2^{k})"
    );
}

/// JFIF's Round(x) = ⌊x + 0.5⌋ (T.871 clause 7) of `x = p / 2^k`, AV1's Round2(p, k) (4.7) and the add-half-then-shift
/// of H.264 (eq. 8-337) and H.265 (eq. 8-309): `NearestCeil`, and `Floor` of `p` plus half a step, as they write it.
pub(crate) fn round_half_up_is_floor_of_half_more(p: i16, k: i8) {
    let p = i32::from(p);
    let want = reference::round_half_up(i128::from(p), 1 << k);
    let half_step = if k == 0 { 0 } else { 1 << (k - 1) };
    assert_eq!(
        i128::from(shift(p + half_step, k, Rounding::Floor)),
        want,
        "Round2({p}, {k})"
    );
    // AV1 prints it as (x + (1 << (n - 1))) >> n, for n > 0
    assert_eq!(i128::from((p + half_step) >> k), want);
}

/// K.10's table (T.81 Annex K): 8-bit DC values before the level shift, after it, divided by 2 and shifted right
/// by 1. The divide truncates the magnitude, the shift the LSB: from −1 down they part.
#[test]
fn k10_every_row() {
    const ROWS: [[i32; 4]; 9] = [
        [131, 3, 1, 1],
        [130, 2, 1, 1],
        [129, 1, 0, 0],
        [128, 0, 0, 0],
        [127, -1, 0, -1],
        [126, -2, -1, -1],
        [125, -3, -1, -2],
        [124, -4, -2, -2],
        [123, -5, -2, -3],
    ];
    for [before, shifted, divided, arithmetic] in ROWS {
        assert_eq!(before - 128, shifted, "the level shift 2^(P−1) (A.3.1)");
        assert_eq!(
            shift(shifted, 1, Rounding::Truncate),
            divided,
            "divide {shifted} by 2"
        );
        assert_eq!(
            shift(shifted, 1, Rounding::Floor),
            arithmetic,
            "{shifted} >> 1"
        );
        // A.4: the shift "is equivalent to dividing by 2^Pt before the level shift"
        assert_eq!((before >> 1) - (128 >> 1), arithmetic);
    }
}

/// A.4's claim in full: for P = 8 and 12 and every Pt below P, the arithmetic shift of a level-shifted sample is the
/// unsigned sample divided by 2^Pt, less the shifted level.
#[test]
fn shift_before_the_level_shift() {
    for p in [8, 12] {
        for pt in 0..p {
            for x in 0..1 << p {
                let level = 1 << (p - 1);
                assert_eq!(
                    shift(x - level, pt, Rounding::Floor),
                    (x >> pt) - (level >> pt)
                );
            }
        }
    }
}

#[test]
fn point_transforms_every_i16() {
    for x in i16::MIN..=i16::MAX {
        for pt in [0, 1, 2, 7, 15] {
            point_transforms(x, pt);
        }
    }
}

#[test]
fn h273_round_and_round_half_up_every_i16() {
    for p in i16::MIN..=i16::MAX {
        for k in [0, 1, 2, 12] {
            h273_round_is_nearest_away(p, k);
            round_half_up_is_floor_of_half_more(p, k);
        }
    }
}

/// JFIF's ⌊x + 0.5⌋ is `NearestCeil` and none of the other five roundings: each parts from it in quarters from −1.5
/// to 1.5 (`Ceil`, which agrees on every half, at 0.25). H.273's Round and JFIF's agree from 0 up and part on a
/// negative half.
#[test]
fn only_nearest_ceil_is_round_half_up() {
    for r in super::reference::MODES {
        if r == Rounding::NearestCeil {
            continue;
        }
        let differs = (-6..=6)
            .any(|p| i128::from(shift(p, 2, r)) != reference::round_half_up(i128::from(p), 4));
        assert!(differs, "{r:?} is ⌊x + 0.5⌋ on halves");
    }
    assert!((-6..=6).all(|p| {
        i128::from(shift(p, 2, Rounding::NearestCeil)) == reference::round_half_up(i128::from(p), 4)
    }));
    assert_eq!(reference::round_half_up(-1, 2), 0);
    assert_eq!(reference::round_h273(-1, 2), -1);
}

/// H.273's YCgCo (eq. 51 to 53), as its NOTE 3 warns: Cb and Cr reach 1 << BitDepthC, one past the chroma's
/// range, and only there: Cb = 256 at G = 255, R = B = 0 and Cr = 256 at R = 255, B = 0, on 8 bits.
#[test]
fn ycgco_reaches_one_past_the_chroma_range() {
    let (mut top_blue, mut top_red) = ((0, [0; 3]), (0, [0; 3]));
    for r in 0..=255 {
        for g in 0..=255 {
            for b in [0, 1, 127, 128, 254, 255, r] {
                let cb = quarter(2 * g - r - b, Rounding::NearestAway) + 128;
                let cr = half(r - b, Rounding::NearestAway) + 128;
                assert_eq!(
                    i128::from(cb - 128),
                    reference::round_h273(i128::from(2 * g - r - b), 4)
                );
                if cb > top_blue.0 {
                    top_blue = (cb, [r, g, b]);
                }
                if cr > top_red.0 {
                    top_red = (cr, [r, g, b]);
                }
            }
        }
    }
    assert_eq!(top_blue, (1 << 8, [0, 255, 0]));
    assert_eq!(top_red, (1 << 8, [255, 0, 0]));
    assert!(u8::try_from(top_blue.0).is_err());
    // a Floor, or NearestEven, would keep Cb within 8 bits: 255/2 is 127.5, which rounds to 128 away from zero only
    assert_eq!(quarter(2 * 255, Rounding::NearestEven) + 128, 1 << 8);
    assert_eq!(quarter(2 * 255, Rounding::Floor) + 128, 255);
}

/// T.800's 9/7 parameters (F.3.8.2.1) from their closed forms, Tables F.5 to F.7, each an interval of exact
/// rationals: √15 and the cube roots of Table F.7 by bisection, the rest by exact arithmetic on the ends. In the
/// order of the tables: A, B, x₁, ℜx₂, |x₂|², g₀ to g₄, r₀, r₁, s₀, t₀, α, β, γ, δ, K.
#[cfg(not(kani))]
pub(crate) fn taps_97() -> Vec<(&'static str, super::reference::interval::Interval)> {
    use super::reference::interval::Interval;
    let i = Interval::of;
    let div = |a: &Interval, b: &Interval| a.div(b).unwrap_or_else(|| panic!("{b:?} holds 0"));
    let sqrt15 = i(15, 1).root(2, 140);
    let den = i(1080, 1).mul(&sqrt15);
    let a = div(&i(63, 1).sub(&i(14, 1).mul(&sqrt15)), &den).root(3, 130);
    let b = div(&i(63, 1).add(&i(14, 1).mul(&sqrt15)), &den)
        .root(3, 130)
        .neg();
    let sum = a.add(&b);
    let x1 = sum.sub(&i(1, 6));
    let re_x2 = sum.mul(&i(-1, 2)).sub(&i(1, 6));
    let half_more = sum.mul(&i(1, 2)).add(&i(1, 6));
    let diff = a.sub(&b);
    let abs_x2 = half_more
        .mul(&half_more)
        .add(&i(3, 4).mul(&diff.mul(&diff)));
    let five_x1 = i(5, 1).mul(&x1);
    let g0 = five_x1
        .mul(
            &i(48, 1)
                .mul(&abs_x2)
                .sub(&i(16, 1).mul(&re_x2))
                .add(&i(3, 1)),
        )
        .mul(&i(1, 32));
    let g1 = five_x1
        .neg()
        .mul(&i(8, 1).mul(&abs_x2).sub(&re_x2))
        .mul(&i(1, 8));
    let g2 = five_x1
        .mul(&i(4, 1).mul(&abs_x2).add(&i(4, 1).mul(&re_x2)).sub(&i(1, 1)))
        .mul(&i(1, 16));
    let g3 = five_x1.neg().mul(&re_x2).mul(&i(1, 8));
    let g4 = five_x1.mul(&i(1, 64));
    let g1g4_g3 = div(&g1.mul(&g4), &g3);
    let r0 = g0.neg().add(&i(2, 1).mul(&g1g4_g3));
    let r1 = g2.neg().add(&g4).add(&g1g4_g3);
    let s0 = g1.sub(&g3).sub(&div(&g3.mul(&r0), &r1));
    let t0 = r0.sub(&i(2, 1).mul(&r1));
    let alpha = div(&g4.neg(), &g3);
    let beta = div(&g3, &r1);
    let gamma = div(&r1, &s0);
    let delta = div(&s0, &t0);
    let k = div(&i(1, 1), &t0);
    vec![
        ("A", a),
        ("B", b),
        ("x1", x1),
        ("Re x2", re_x2),
        ("|x2|^2", abs_x2),
        ("g0", g0),
        ("g1", g1),
        ("g2", g2),
        ("g3", g3),
        ("g4", g4),
        ("r0", r0),
        ("r1", r1),
        ("s0", s0),
        ("t0", t0),
        ("alpha", alpha),
        ("beta", beta),
        ("gamma", gamma),
        ("delta", delta),
        ("K", k),
    ]
}

/// `x` between the ends of `v`, widened to whole steps of 10⁻³⁰, as two [Exact]s.
#[cfg(not(kani))]
fn exact_ends(v: &super::reference::interval::Interval) -> [Exact<Ratio>; 2] {
    use num_bigint::BigInt;
    use num_rational::BigRational;
    let den = BigInt::from(10).pow(30);
    let scale = BigRational::from_integer(den.clone());
    let n = |x: BigRational| i128::try_from(x.to_integer()).unwrap_or_else(|_| panic!("fits"));
    let den = u128::try_from(den).unwrap_or_else(|_| panic!("10^30 fits"));
    [
        Exact::new(n((&v.lo * &scale).floor()), den),
        Exact::new(n((&v.hi * &scale).ceil()), den),
    ]
}

/// `v` rounded to 15 decimals by `r`, the same for every value in it: `None` where its ends round apart.
#[cfg(not(kani))]
fn decimals_15(v: &super::reference::interval::Interval, r: Rounding) -> Option<i64> {
    let [lo, hi] = exact_ends(v).map(|e| e.to_rounded::<Pow10<-15>, i64>(r).map(Fixed::get));
    lo.filter(|lo| Some(*lo) == hi)
}

/// Tables F.4 to F.7 print each parameter "including approximations up to 15 decimal points" (F.3.8.2.1). Each is
/// the closed form rounded to 15 decimals, but r₀ (Table F.6), 1.449513704087942|3…, printed …943, and γ (Table
/// F.4), 0.882911075530933|29…, printed …934: one in the last place each.
#[cfg(not(kani))]
#[test]
fn taps_97_are_their_closed_forms_rounded() {
    const PRINTED: [i64; 19] = [
        128_030_244_703_494,
        -303_747_672_895_197,
        -342_384_094_858_369,
        -78_807_952_570_815,
        146_034_820_982_800,
        -602_949_018_236_360,
        266_864_118_442_875,
        78_223_266_528_990,
        -16_864_118_442_875,
        -26_748_757_410_810,
        1_449_513_704_087_943,
        318_310_318_985_991,
        360_523_644_801_462,
        812_893_066_115_961,
        -1_586_134_342_059_924,
        -52_980_118_572_961,
        882_911_075_530_934,
        443_506_852_043_971,
        1_230_174_104_914_001,
    ];
    let taps = taps_97();
    assert_eq!(taps.len(), PRINTED.len());
    for ((name, v), printed) in taps.iter().zip(PRINTED) {
        let even = decimals_15(v, Rounding::NearestEven);
        // no parameter is within 10⁻³⁰ of a tie: every nearest rounding gives the same digits
        assert_eq!(even, decimals_15(v, Rounding::NearestAway), "{name}");
        let true_digits = even.unwrap_or_else(|| panic!("{name} is known to 15 decimals"));
        match *name {
            "r0" | "gamma" => assert_eq!(true_digits, printed - 1, "{name}"),
            _ => assert_eq!(true_digits, printed, "{name}"),
        }
    }
    // the misprinted two, with the digits they have, and as printed they are not
    #[cfg(feature = "q")]
    {
        let r0 = exact_ends(&taps[10].1);
        let gamma = exact_ends(&taps[16].1);
        assert!(r0.iter().all(|e| e.is_in(q!(1.4495137040879423...Ratio))));
        assert!(
            gamma
                .iter()
                .all(|e| e.is_in(q!(0.8829110755309332...Ratio)))
        );
        assert!(!gamma.iter().any(|e| e.is_in(q!(0.882911075530934...Ratio))));
    }
}

/// AV1's Cos128_Lookup (7.13.2), "4096 * cos( angle * pi / 128 ) rounded to the nearest integer" by its note: every
/// entry is that rounding of an interval holding the true value, which lies strictly between two halves, so the
/// unnamed tie never matters. sin128 is cos128(angle − 64) by the same note.
#[cfg(not(kani))]
#[test]
fn cos128_every_entry() {
    use super::reference::interval::{Interval, cos, pi};
    const COS128: [i32; 65] = [
        4096, 4095, 4091, 4085, 4076, 4065, 4052, 4036, 4017, 3996, 3973, 3948, 3920, 3889, 3857,
        3822, 3784, 3745, 3703, 3659, 3612, 3564, 3513, 3461, 3406, 3349, 3290, 3229, 3166, 3102,
        3035, 2967, 2896, 2824, 2751, 2675, 2598, 2520, 2440, 2359, 2276, 2191, 2106, 2019, 1931,
        1842, 1751, 1660, 1567, 1474, 1380, 1285, 1189, 1092, 995, 897, 799, 700, 601, 501, 401,
        301, 201, 101, 0,
    ];
    for (k, want) in (0_i64..).zip(COS128) {
        let v = Interval::of(4096, 1).mul(&cos(&pi().mul(&Interval::of(k, 128))));
        let [lo, hi] = exact_ends(&v);
        for r in [Rounding::NearestEven, Rounding::NearestAway] {
            let got = [lo, hi].map(|e| e.to_rounded::<One, i32>(r).map(Fixed::get));
            assert_eq!(got, [Some(want); 2], "cos128({k})");
        }
    }
}

/// H.264's sample aspect ratio indicator (Table E-1, H.265 Table E.1 the same): codes 1 to 16 are ratios, every
/// one a whole number of 1/198; 0 is Unspecified, 17 to 254 Reserved and 255 Extended_SAR (the ratio follows in
/// `sar_width` and `sar_height`), so all of them are spare.
#[derive(Copy, Clone, Debug)]
pub(crate) struct Sar;

/// [Sar]'s table, in 1/198: entry `i` is code `i`'s ratio, code 0's entry a placeholder for a spare code.
const SAR: [i128; 17] = [
    0, 198, 216, 180, 288, 240, 432, 360, 576, 480, 324, 270, 384, 320, 264, 297, 396,
];

/// Table E-1's ratios as printed, `width:height`, code 1 to 16.
const SAR_PRINTED: [(i128, u128); 16] = [
    (1, 1),
    (12, 11),
    (10, 11),
    (16, 11),
    (40, 33),
    (24, 11),
    (20, 11),
    (32, 11),
    (80, 33),
    (18, 11),
    (15, 11),
    (64, 33),
    (160, 99),
    (4, 3),
    (3, 2),
    (2, 1),
];

impl Codec for Sar {
    type Raw = u8;
    type Linear = Fixed<Ratio, Rational<1, 198>, u16>;

    const DEF: CodecDef<'static> = CodecDef {
        id: MarkerId {
            crate_name: "props",
            name: "Sar",
        },
        shape: CodecShape::Table(def_list(&SAR)),
        spare: def_list(&[
            CodeSpan::one(0),
            CodeSpan { low: 17, high: 254 },
            CodeSpan::one(255),
        ]),
        codes: CodeSpan::all::<u8>(),
        nominal: None,
        curve: None,
    };

    fn formula(raw: u8) -> Self::Linear {
        let ratio = SAR.get(usize::from(raw)).copied().unwrap_or_default();
        Fixed::new(u16::try_from(ratio).unwrap_or_default())
    }

    fn encode(value: Self::Linear, rounding: Rounding) -> Option<u8> {
        let code = Self::DEF.encode(i128::from(value.get()), rounding)?;
        u8::try_from(code).ok()
    }
}

/// Every code of [Sar], by the typed path and the data path: codes 1 to 16 are Table E-1's ratios exactly, and each
/// encodes back to its code by every rounding; every other code is spare.
#[test]
fn sar_every_code() {
    for raw in 0..=u8::MAX {
        let want = SAR_PRINTED
            .get(usize::from(raw).wrapping_sub(1))
            .map(|&(w, h)| Exact::<Ratio>::new(w, h).to::<Rational<1, 198>, u16>());
        let typed = Sar::decode(raw);
        assert_eq!(typed.map(Some), want, "code {raw}");
        assert_eq!(
            Sar::DEF.decode(i128::from(raw)),
            typed.map(|v| i128::from(v.get()))
        );
        assert_eq!(Coded::<Sar>::try_from_raw(raw).is_ok(), typed.is_some());
        if let Some(v) = typed {
            for r in super::reference::MODES {
                assert_eq!(Sar::encode(v, r), Some(raw), "code {raw} by {r:?}");
            }
        }
    }
    // between 40:33 and 4:3, a ratio no code has goes to the nearer, or up or down as named
    let between = Fixed::<Ratio, Rational<1, 198>, u16>::new(250);
    assert_eq!(Sar::encode(between, Rounding::NearestEven), Some(5));
    assert_eq!(Sar::encode(between, Rounding::Ceil), Some(14));
}

/// BT.601's luma weights, exact: K_R = 0.299, K_B = 0.114 (T.871 clause 7, T.800 G.3).
const KR: (i128, i128) = (299, 1000);
/// See [KR].
const KB: (i128, i128) = (114, 1000);

/// The Y′C_BC_R matrix of `K_R` and `K_B`, forward (rows Y, C_B, C_R over R, G, B) and inverse (rows R, G, B over
/// Y, C_B, C_R), every entry an exact rational.
#[cfg(not(kani))]
fn ycbcr_matrices() -> [[[num_rational::BigRational; 3]; 3]; 2] {
    use num_rational::BigRational;
    let q = |(n, d): (i128, i128)| BigRational::new(n.into(), d.into());
    let one = BigRational::from_integer(1.into());
    let half = BigRational::new(1.into(), 2.into());
    let (kr, kb) = (q(KR), q(KB));
    let kg = &one - &kr - &kb;
    let cb = (&one - &kb) * BigRational::from_integer(2.into()); // 1.772
    let cr = (&one - &kr) * BigRational::from_integer(2.into()); // 1.402
    let forward = [
        [kr.clone(), kg.clone(), kb.clone()],
        [-&kr / &cb, -&kg / &cb, half.clone()],
        [half, -&kg / &cr, -&kb / &cr],
    ];
    let zero = BigRational::default();
    let inverse = [
        [one.clone(), zero.clone(), cr.clone()],
        [one.clone(), -(&kb * &cb / &kg), -(&kr * &cr / &kg)],
        [one, cb, zero],
    ];
    [forward, inverse]
}

/// `a · b` of 3×3 matrices.
#[cfg(not(kani))]
fn product(
    a: &[[num_rational::BigRational; 3]; 3],
    b: &[[num_rational::BigRational; 3]; 3],
) -> [[num_rational::BigRational; 3]; 3] {
    let column = |j: usize| b.iter().filter_map(move |r| r.get(j));
    a.each_ref()
        .map(|row| core::array::from_fn(|j| row.iter().zip(column(j)).map(|(x, y)| x * y).sum()))
}

/// `x` rounded to `places` decimals as text, through [Exact::rounded].
#[cfg(not(kani))]
fn decimals(x: &num_rational::BigRational, places: u8) -> String {
    let n = i128::try_from(x.numer().clone()).unwrap_or_else(|_| panic!("fits"));
    let d = u128::try_from(x.denom().clone()).unwrap_or_else(|_| panic!("fits"));
    format!(
        "{:#}",
        Exact::<Ratio>::new(n, d).rounded(places, Rounding::NearestEven)
    )
}

/// The forward and inverse matrices from K_R and K_B are each other's inverse, exactly. JFIF prints them to four
/// decimals (T.871 clause 7), and every one is the exact entry rounded. T.800 prints them to five or six (G.3), with
/// three last digits off: G-10's 0,16875 and 0,331260 (the exact entries round to 0.16874 and 0.331264) and G-13's
/// 0,34413 (0.34414). G-9 prints Y0 = −0,299·I0 − 0,587·I1 + 0,114·I2: with those signs the inverse G-12 to G-14 does
/// not undo it, with plus signs it does.
#[cfg(not(kani))]
#[test]
fn ycbcr_matrices_from_kr_and_kb() {
    use num_rational::BigRational;
    let [forward, inverse] = ycbcr_matrices();
    let one = BigRational::from_integer(1.into());
    let identity: [[BigRational; 3]; 3] = core::array::from_fn(|i| {
        core::array::from_fn(|j| {
            if i == j {
                one.clone()
            } else {
                BigRational::default()
            }
        })
    });
    assert_eq!(product(&inverse, &forward), identity);
    assert_eq!(product(&forward, &inverse), identity);
    let mut printed_g9 = forward.clone();
    if let Some(y0) = printed_g9.first_mut() {
        for k in y0.iter_mut().take(2) {
            *k = -&*k;
        }
    }
    assert_ne!(product(&inverse, &printed_g9), identity);
    let [[_, _, _], [blue_r, blue_g, _], [_, red_g, red_b]] = &forward;
    let [_, [_, green_blue, green_red], _] = &inverse;
    // JFIF, four decimals: every entry
    let jfif = [
        (blue_r, "-0.1687"),
        (blue_g, "-0.3313"),
        (red_g, "-0.4187"),
        (red_b, "-0.0813"),
        (green_blue, "-0.3441"),
        (green_red, "-0.7141"),
    ];
    for (x, printed) in jfif {
        assert_eq!(decimals(x, 4), printed);
    }
    // T.800, as printed, and the exact entry to as many places
    let t800 = [
        (blue_r, "-0.16875", "-0.16874"),
        (blue_g, "-0.331260", "-0.331264"),
        (red_g, "-0.41869", "-0.41869"),
        (red_b, "-0.08131", "-0.08131"),
        (green_blue, "-0.34413", "-0.34414"),
        (green_red, "-0.71414", "-0.71414"),
    ];
    for (x, printed, exact) in t800 {
        let places = u8::try_from(printed.len() - 3).unwrap_or(0);
        assert_eq!(decimals(x, places), exact, "{printed}");
    }
}

/// H.265's levelScale (8.6.3, eq. 8-309).
const LEVEL_SCALE: [i64; 6] = [40, 45, 51, 57, 64, 72];

/// H.264's v for the 4×4 (eq. 8-315) and 8×8 (eq. 8-318) transforms: row m, column the coefficient's place.
const V4: [[i64; 3]; 6] = [
    [10, 16, 13],
    [11, 18, 14],
    [13, 20, 16],
    [14, 23, 18],
    [16, 25, 20],
    [18, 29, 23],
];
/// See [V4].
const V8: [[i64; 6]; 6] = [
    [20, 18, 32, 19, 25, 24],
    [22, 19, 35, 21, 28, 26],
    [26, 23, 42, 24, 33, 31],
    [28, 25, 45, 26, 35, 33],
    [32, 28, 51, 30, 40, 38],
    [36, 32, 58, 34, 46, 43],
];

/// The step of a scale table at `qp`: `table[qp % 6] << (qp / 6)`, as H.265's eq. 8-309 and H.264's eq. 8-336 form
/// it, through a run-time prefix 2^(qp / 6).
fn qp_step(table: [i64; 6], qp: u8) -> i64 {
    let mantissa = table.get(usize::from(qp % 6)).copied().unwrap_or(0);
    let exponent = i8::try_from(qp / 6).unwrap_or(0);
    Prefix::from_pow2(exponent)
        .and_then(|p| {
            p.rescale(
                mantissa,
                Prefix::Unit,
                Rounding::Truncate,
                Overflow::Checked,
            )
        })
        .unwrap_or_else(|| panic!("the step at {qp} fits an i64"))
}

/// H.264's NOTE 2 (8.5.8): the scale "doubles for every increment of 6 in QPY", "approximately 12 % for each
/// increase of 1". The doubling is exact in both standards and every step rises, but the steps between are the
/// tables' own ratios: none is 2^(1/6), H.265's are within 0.012 of it (45/40 = 1.125, 80/72 = 1.111…), H.264's
/// within 0.089 (19/18 = 1.0556 in the 8×8 table).
#[cfg(not(kani))]
#[test]
fn qp_doubles_every_six_and_no_step_is_the_sixth_root_of_two() {
    use num_bigint::BigInt;
    let mut tables: Vec<([i64; 6], i64)> = vec![(LEVEL_SCALE, 12)];
    for c in 0..3 {
        tables.push((V4.map(|row| row.get(c).copied().unwrap_or(0)), 89));
    }
    for c in 0..6 {
        tables.push((V8.map(|row| row.get(c).copied().unwrap_or(0)), 89));
    }
    for (table, within) in tables {
        for qp in 0..=51 {
            let (a, b) = (qp_step(table, qp), qp_step(table, qp + 1));
            assert_eq!(qp_step(table, qp + 6), 2 * a, "{table:?} at {qp}");
            assert!(b > a);
            // b/a against 2^(1/6), exactly: (b/a)^6 against 2, and b/a ∓ within/1000 on either side of it
            let (a, b) = (BigInt::from(a), BigInt::from(b));
            assert_ne!(b.pow(6), 2 * a.pow(6));
            let thousand = BigInt::from(1000);
            let lo: BigInt = &thousand * &b - within * &a;
            let hi: BigInt = &thousand * &b + within * &a;
            let two: BigInt = 2 * (&thousand * &a).pow(6);
            assert!(lo.pow(6) < two && two < hi.pow(6), "{table:?}: {a} to {b}");
        }
    }
}

/// Every code of a [CodecShape::MantissaExponent] codec in `codes`, by the typed path and the data path, against
/// `want` (the standard's formula in `i128`, `None` for a spare code); each value encodes back to `lowest` of it,
/// the lowest code with that value, by every rounding.
fn walk_mantissa_exponent<C: Codec>(
    codes: impl IntoIterator<Item = i128>,
    want: impl Fn(i128) -> Option<i128>,
    lowest: impl Fn(i128, i128) -> i128,
) where
    C::Raw: TryFrom<i128> + Into<i128> + core::fmt::Debug,
    C::Linear: Copy + LinearInt,
{
    for raw in codes {
        let Ok(code) = C::Raw::try_from(raw) else {
            panic!("{raw} fits the raw storage")
        };
        let w = want(raw);
        assert_eq!(
            C::decode(code).map(LinearInt::int),
            w,
            "{} code {raw}",
            C::DEF.id.name
        );
        assert_eq!(
            C::DEF.decode(raw),
            w,
            "{}'s DEF, code {raw}",
            C::DEF.id.name
        );
        if let (Some(v), Some(value)) = (w, C::decode(code)) {
            let first = lowest(v, raw);
            for r in super::reference::MODES {
                assert_eq!(
                    C::encode(value, r).map(Into::into),
                    Some(first),
                    "{} code {raw} by {r:?}",
                    C::DEF.id.name
                );
                assert_eq!(C::DEF.encode(v, r), Some(first));
            }
        }
    }
}

/// The raw value of a codec's `Linear`, an integer `Fixed`.
trait LinearInt {
    /// The stored integer.
    fn int(self) -> i128;
}

impl<U, P, T: Into<i128> + exact_units::Storage> LinearInt for Fixed<U, P, T> {
    fn int(self) -> i128 {
        self.get().into()
    }
}

#[test]
fn j2k_step_size_every_code() {
    walk_mantissa_exponent::<J2kStepSize>(
        0..=0xFFFF,
        |raw| Some(reference::j2k_step(raw)),
        |_, raw| raw,
    );
}

#[test]
fn hevc_level_scale_every_code() {
    walk_mantissa_exponent::<HevcLevelScale>(
        0..=255,
        |qp| (qp < 100).then(|| reference::hevc_level_scale(qp)),
        |_, raw| raw,
    );
}

/// The HRD's codes at every scale: the ends of `value_minus1`, the forbidden 2³² − 1, a run around 2³¹ and the
/// powers of two, where a rate has several codes; the codes past scale 15.
#[cfg(all(feature = "information", feature = "time"))]
#[test]
fn hrd_codes() {
    let mut values: Vec<i128> = vec![0, 1, 2, 3, 0xFFFF_FFFD, 0xFFFF_FFFE, 0xFFFF_FFFF];
    values.extend((0..32).map(|k| (1 << k) - 1));
    values.extend((0x7FFF_FFF0..=0x8000_0010).step_by(1));
    let codes = |extra: i128| {
        let values = values.clone();
        (0..=16)
            .flat_map(move |s| values.clone().into_iter().map(move |v| (s << 32) | v))
            .chain([extra])
    };
    // the lowest code of a value: the least scale whose mantissa is whole and in range
    let lowest = |zero: i128| {
        move |v: i128, _: i128| {
            (0..16)
                .find_map(|s| {
                    let step = 1 << (zero + s);
                    let m = v / step;
                    (v % step == 0 && (1..1 << 32).contains(&m)).then_some((s << 32) | (m - 1))
                })
                .unwrap_or_else(|| panic!("{v} has a code"))
        }
    };
    let want = |zero: i128| move |raw: i128| reference::hrd(raw >> 32, raw & 0xFFFF_FFFF, zero);
    walk_mantissa_exponent::<HrdBitRate>(codes(i128::from(u64::MAX)), want(6), lowest(6));
    walk_mantissa_exponent::<HrdCpbSize>(codes(i128::from(u64::MAX)), want(4), lowest(4));
}

/// `x` (a step of 2⁻⁴², or a level scale) encoded by `r` against every value the codec has, sorted: the closest on
/// the side `r` takes, the nearer for the nearest modes (a tie to the greater value for `NearestCeil` and
/// `NearestAway`, to the even code for `NearestEven`), `None` past the end on that side.
fn encodes_by_rounding(values: &[(i128, i128)], x: i128, r: Rounding, got: Option<i128>) {
    let below = values.iter().rev().find(|&&(v, _)| v <= x).copied();
    let above = values.iter().find(|&&(v, _)| v >= x).copied();
    let want = match (below, above) {
        (Some(b), _) if b.0 == x => Some(b),
        _ => match r {
            Rounding::Floor | Rounding::Truncate => below,
            Rounding::Ceil => above,
            _ => match (below, above) {
                (Some(b), Some(a)) => {
                    let (db, da) = (x - b.0, a.0 - x);
                    if db != da {
                        Some(if db < da { b } else { a })
                    } else if r == Rounding::NearestEven {
                        Some(if b.1 % 2 == 0 { b } else { a })
                    } else {
                        Some(a)
                    }
                }
                (one, other) => one.or(other),
            },
        },
    };
    assert_eq!(got, want.map(|(_, code)| code), "{x} by {r:?}");
}

/// Values with their codes, sorted by value.
type ByValue = Vec<(i128, i128)>;

/// Every value of [J2kStepSize] and [HevcLevelScale], with its code, sorted by value.
fn sorted_values() -> (ByValue, ByValue) {
    let mut j2k: Vec<_> = (0..=0xFFFF)
        .map(|raw| (reference::j2k_step(raw), raw))
        .collect();
    j2k.sort_unstable();
    let mut hevc: Vec<_> = (0..100)
        .map(|qp| (reference::hevc_level_scale(qp), qp))
        .collect();
    hevc.sort_unstable();
    (j2k, hevc)
}

/// A step between codes, encoded by each rounding, against every value of the codec.
pub(crate) fn mantissa_exponent_rounds(j2k_x: u64, hevc_x: u32, r: Rounding) {
    let (j2k, hevc) = sorted_values();
    let got = J2kStepSize::encode(Fixed::new(j2k_x), r).map(i128::from);
    encodes_by_rounding(&j2k, i128::from(j2k_x), r, got);
    let got = HevcLevelScale::encode(Fixed::new(hevc_x), r).map(i128::from);
    encodes_by_rounding(&hevc, i128::from(hevc_x), r, got);
}

#[cfg(not(any(kani, fuzzing)))]
mod props {
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn lifting_53(x in any::<[i16; 9]>(), y in any::<[i16; 4]>()) {
            super::lifting_53_is_lossless(x);
            super::lifting_53_is_lossless(y);
        }

        #[test]
        fn rct(i in any::<[i16; 3]>()) {
            super::rct_is_lossless(i);
        }

        #[test]
        fn ycgco_r(rgb in any::<[u16; 3]>(), bits in 8u32..=16) {
            super::ycgco_r_is_lossless(rgb, bits);
        }

        #[test]
        fn point_transforms(x: i16, pt in 0i8..=15) {
            super::point_transforms(x, pt);
        }

        #[test]
        fn mantissa_exponent_rounds(j2k in 0u64..1 << 44, hevc in 0u32..72 << 17, r in prop::sample::select(&super::super::reference::MODES[..])) {
            super::mantissa_exponent_rounds(j2k, hevc, r);
        }

        #[test]
        fn roundings(p: i16, k in 0i8..=15) {
            super::h273_round_is_nearest_away(p, k);
            super::round_half_up_is_floor_of_half_more(p, k);
        }
    }
}

#[cfg(kani)]
mod proofs {
    #[kani::proof]
    #[kani::unwind(6)]
    fn lifting_53() {
        super::lifting_53_is_lossless::<4>(kani::any());
    }

    #[kani::proof]
    fn rct() {
        super::rct_is_lossless(kani::any());
    }

    #[kani::proof]
    fn point_transforms() {
        let pt: i8 = kani::any();
        kani::assume((0..=15).contains(&pt));
        super::point_transforms(kani::any(), pt);
    }

    #[kani::proof]
    fn ycgco_r() {
        let bits: u32 = kani::any();
        kani::assume((8..=16).contains(&bits));
        super::ycgco_r_is_lossless(kani::any(), bits);
    }
}
