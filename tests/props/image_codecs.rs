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
use exact_units::prefix::{One, Pow2};
use exact_units::units::Ratio;
use exact_units::{Fixed, Overflow, Prefix, Rounding};

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
        assert_eq!(y.map(i128::from).to_vec(), reference::fdwt53(&wide, 2), "{x:?}");
        assert_eq!(reference::idwt53(&reference::fdwt53(&wide, 2)), wide);
    }
    assert_eq!(idwt53(&y), x);
}

/// T.800's forward RCT (G.2.1, eq. G-3 to G-5): `[Y0, Y1, Y2]`.
pub(crate) fn rct(i: [i32; 3]) -> [i32; 3] {
    let [i0, i1, i2] = i;
    [
        quarter(i0 + 2 * i1 + i2, Rounding::Floor),
        i2 - i1,
        i0 - i1,
    ]
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
        assert_eq!(ycc.map(i128::from), reference::ycgco_r(wide, bits + 1), "{rgb:?}");
        assert_eq!(reference::ycgco_r_inverse(reference::ycgco_r(wide, bits + 1), bits + 1), wide);
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
    assert_eq!(shift(x, pt, Rounding::Truncate), x / (1 << pt), "{x} / 2^{pt}");
    assert_eq!(shift(x, pt, Rounding::Floor), x >> pt, "{x} >> {pt}");
}

/// H.273's Round (eq. 8), Sign(x)·Floor(Abs(x) + 0.5), of `x = p / 2^k` is `NearestAway`.
pub(crate) fn h273_round_is_nearest_away(p: i16, k: i8) {
    let p = i32::from(p);
    let want = reference::round_h273(i128::from(p), 1 << k);
    assert_eq!(i128::from(shift(p, k, Rounding::NearestAway)), want, "Round({p} / 2^{k})");
}

/// JFIF's Round(x) = ⌊x + 0.5⌋ (T.871 clause 7) of `x = p / 2^k`, AV1's Round2(p, k) (4.7) and the add-half-then-shift
/// of H.264 (eq. 8-337) and H.265 (eq. 8-309): `Floor` of `p` plus half a step, until `NearestCeil` names it.
pub(crate) fn round_half_up_is_floor_of_half_more(p: i16, k: i8) {
    let p = i32::from(p);
    let want = reference::round_half_up(i128::from(p), 1 << k);
    let half_step = if k == 0 { 0 } else { 1 << (k - 1) };
    assert_eq!(i128::from(shift(p + half_step, k, Rounding::Floor)), want, "Round2({p}, {k})");
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
        assert_eq!(shift(shifted, 1, Rounding::Truncate), divided, "divide {shifted} by 2");
        assert_eq!(shift(shifted, 1, Rounding::Floor), arithmetic, "{shifted} >> 1");
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
                assert_eq!(shift(x - level, pt, Rounding::Floor), (x >> pt) - (level >> pt));
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

/// None of today's five roundings is JFIF's ⌊x + 0.5⌋: each parts from it in quarters from −1.5 to 1.5 (`Ceil`,
/// which agrees on every half, at 0.25). H.273's Round and JFIF's agree from 0 up and part on a negative half.
#[test]
fn no_mode_is_round_half_up() {
    for r in super::reference::MODES {
        let differs = (-6..=6).any(|p| {
            i128::from(shift(p, 2, r)) != reference::round_half_up(i128::from(p), 4)
        });
        assert!(differs, "{r:?} is ⌊x + 0.5⌋ on halves");
    }
    assert_eq!(reference::round_half_up(-1, 2), 0);
    assert_eq!(reference::round_h273(-1, 2), -1);
}

/// H.273's YCgCo (eq. 51 to 53), as its NOTE 3 warns: Cb and Cr reach 1 << BitDepthC, one past the chroma's
/// range, and only there: Cb = 256 at G = 255, R = B = 0 and Cr = 256 at R = 255, B = 0, on 8 bits.
#[test]
fn ycgco_reaches_one_past_the_chroma_range() {
    let (mut cb_max, mut cr_max) = ((0, [0; 3]), (0, [0; 3]));
    for r in 0..=255 {
        for g in 0..=255 {
            for b in [0, 1, 127, 128, 254, 255, r] {
                let cb = quarter(2 * g - r - b, Rounding::NearestAway) + 128;
                let cr = half(r - b, Rounding::NearestAway) + 128;
                assert_eq!(
                    i128::from(cb - 128),
                    reference::round_h273(i128::from(2 * g - r - b), 4)
                );
                if cb > cb_max.0 {
                    cb_max = (cb, [r, g, b]);
                }
                if cr > cr_max.0 {
                    cr_max = (cr, [r, g, b]);
                }
            }
        }
    }
    assert_eq!(cb_max, (1 << 8, [0, 255, 0]));
    assert_eq!(cr_max, (1 << 8, [255, 0, 0]));
    assert!(u8::try_from(cb_max.0).is_err());
    // a Floor, or NearestEven, would keep Cb within 8 bits: 255/2 is 127.5, which rounds to 128 away from zero only
    assert_eq!(quarter(2 * 255, Rounding::NearestEven) + 128, 1 << 8);
    assert_eq!(quarter(2 * 255, Rounding::Floor) + 128, 255);
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
