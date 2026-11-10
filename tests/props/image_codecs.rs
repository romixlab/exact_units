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

use super::reference::imaging as reference;
use exact_units::prefix::{One, Pow2};
use exact_units::units::Ratio;
use exact_units::{Fixed, Rounding};

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
    fn ycgco_r() {
        let bits: u32 = kani::any();
        kani::assume((8..=16).contains(&bits));
        super::ycgco_r_is_lossless(kani::any(), bits);
    }
}
