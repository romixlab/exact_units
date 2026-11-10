//! Photographic exposure (LVL-12): levels in stops and Exif's APEX values against exact references.
//!
//! - A level in stops of every `u16` ratio, power (lx/lx) and root-power (V/V), by every rounding, against
//!   `k · log2 r` worked in big integers (`reference::stops_rounded`).
//! - Every whole stop back to its gain, exactly the power of two.
//! - APEX values from Exif rationals, exact or refused, against `6 · num / den` in `i128`.
//! - f-numbers and times exact for whole stops and `None` between them; their float edges equal to them where exact.
//! - Annex C's Sv against `8·S/25` as a power of two.
//! - Table 55's label codecs against the table written out here: every code, and every value of a window around the
//!   table by every rounding.
//!
//! The spaces are small and walked whole; the Exif rationals are drawn besides.
#![expect(
    clippy::arithmetic_side_effects,
    reason = "a test computes with the values it chose, which fit; an overflow fails the test"
)]
#![expect(clippy::unwrap_used, reason = "a test fails by panicking")]

#[cfg(not(kani))]
use exact_units::Magnitude;
use exact_units::codec::{Codec, Coded};
use exact_units::exposure::{ApertureLabel, Apex, ShutterLabel, apex_from_exif, sensitivity_value};
#[cfg(not(kani))]
use exact_units::exposure::{exposure_time, f_number};
#[cfg(all(not(kani), any(feature = "std", feature = "libm")))]
use exact_units::exposure::{exposure_time_f64, f_number_f64};
use exact_units::{Fixed, Rounding};

/// Av of each label of Table 55's aperture column, in whole stops, written out from CIPA DC-008-2026 Annex C: the
/// label "1" is Av 0, "32" Av 10.
const APERTURE_AV: [i16; 11] = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10];

/// Tv of each label of Table 55's shutter column, in whole stops: "30" is Tv −5, "1/2000" Tv 11.
const SHUTTER_TV: [i16; 17] = [-5, -4, -3, -2, -1, 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11];

/// `r` as a level in sixths of a stop, power (lx/lx) and root-power (V/V, in thirds: 2·log2 r stops, so the same
/// `6 · log2 r`), by `md`, against the reference; a ratio of 0 refused.
#[cfg(all(not(kani), any(feature = "std", feature = "libm")))]
pub(crate) fn level_in_stops(r: u16, md: Rounding) {
    use exact_units::prefix::{One, Rational};
    use exact_units::units::{Lux, Per, Stop, Volt};
    let lux = Fixed::<Per<Lux, Lux>, One, u16>::new(r).to_level::<Stop, Rational<1, 6>, i32>(md);
    let volt = Fixed::<Per<Volt, Volt>, One, u16>::new(r).to_level::<Stop, Rational<1, 3>, i32>(md);
    if r == 0 {
        assert_eq!((lux, volt), (None, None), "a ratio of 0");
        return;
    }
    let want = i32::try_from(crate::reference::stops_rounded(u64::from(r), 6, md)).unwrap();
    assert_eq!(lux.map(Fixed::get), Some(want), "{r} lx/lx {md:?}");
    assert_eq!(volt.map(Fixed::get), Some(want), "{r} V/V {md:?}");
}

/// `k` whole stops back to a gain: the power ratio is 2^k exactly, and the root-power one 2^(k/2) for an even `k`.
#[cfg(all(not(kani), any(feature = "std", feature = "libm")))]
pub(crate) fn whole_stop_gain(k: i8) {
    use exact_units::prefix::One;
    use exact_units::units::{Lux, Per, Stop, Volt};
    let level = Fixed::<Stop, One, i8>::new(k);
    let p = level.to_gain::<Per<Lux, Lux>, One, f64>().map(Fixed::get);
    assert_eq!(p, Some(2_f64.powi(i32::from(k))), "{k} stops of power");
    if k % 2 == 0 {
        let v = level.to_gain::<Per<Volt, Volt>, One, f64>().map(Fixed::get);
        assert_eq!(
            v,
            Some(2_f64.powi(i32::from(k / 2))),
            "{k} stops of a voltage"
        );
    }
}

/// An Exif RATIONAL `num / den` as an [Apex]: exactly `6 · num / den` sixths where that is a whole number that fits
/// `i16`, else `None`; a `den` of 0 refused.
pub(crate) fn exif_rational(num: i64, den: i64) {
    let got = apex_from_exif(num, den).map(Fixed::get);
    let want = if den == 0 {
        None
    } else {
        let (p, d) = (6 * i128::from(num), i128::from(den));
        (p % d == 0)
            .then(|| p / d)
            .and_then(|q| i16::try_from(q).ok())
    };
    assert_eq!(got, want, "{num}/{den}");
}

/// f-number and time of `sixths` of a stop: exact as a power of two (a half power for N) at a whole stop, `None`
/// between; the float edges give the same number where it is exact.
#[cfg(not(kani))]
pub(crate) fn whole_stops_exact(sixths: i16) {
    let v = Apex::new(sixths);
    let n = f_number(v);
    let t = exposure_time(v);
    if sixths % 6 != 0 {
        assert_eq!((n, t), (None, None), "{sixths}/6 stop");
        return;
    }
    let k = i8::try_from(sixths / 6).unwrap();
    // N = 2^q for Av 2q, 2^q·√2 for Av 2q + 1; t = 2^−Tv, refused past Magnitude's exponents (±63½)
    let q = Magnitude::pow2(k.div_euclid(2));
    let want_n = if k % 2 == 0 {
        q
    } else {
        q.mul(Magnitude::pow2(1).sqrt())
    };
    assert_eq!(n, Some(want_n), "N of Av {k}");
    let want_t = k
        .checked_neg()
        .and_then(|m| Magnitude::checked_pow2(m).ok());
    assert_eq!(t, want_t, "t of Tv {k}");
    #[cfg(any(feature = "std", feature = "libm"))]
    {
        if want_t.is_some() {
            assert_eq!(
                exposure_time_f64(v),
                2_f64.powi(-i32::from(k)),
                "t of Tv {k} in f64"
            );
        }
        if k % 2 == 0 {
            assert_eq!(
                f_number_f64(v),
                2_f64.powi(i32::from(k / 2)),
                "N of Av {k} in f64"
            );
        }
    }
}

/// Annex C's Sv = log2(S/3.125) of a speed `s`: whole stops exactly where `8·s/25` is a power of two, else `None`.
pub(crate) fn sv_of_speed(s: u32) {
    let got = sensitivity_value(s).map(Fixed::get);
    let q = u64::from(s) * 8;
    let want = (q % 25 == 0 && (q / 25).is_power_of_two())
        .then(|| i16::try_from((q / 25).trailing_zeros() * 6).unwrap());
    assert_eq!(got, want, "ISO {s}");
}

/// The code `x` sixths of a stop rounds to by `md` among labels of whole stops `stops` (code `i` is `stops[i]`):
/// the exact one, else the one below or above as the mode says, a tie to the even code (`NearestEven`) or the value
/// farther from zero (`NearestAway`); past an end, the nearest modes take the end and the others refuse.
fn label_rounded(stops: &[i16], x: i16, md: Rounding) -> Option<usize> {
    let sixth = |i: usize| stops.get(i).map(|s| s * 6);
    if let Some(i) = (0..stops.len()).find(|&i| sixth(i) == Some(x)) {
        return Some(i);
    }
    let below = (0..stops.len())
        .rev()
        .find(|&i| sixth(i).is_some_and(|v| v < x));
    let above = (0..stops.len()).find(|&i| sixth(i).is_some_and(|v| v > x));
    match md {
        Rounding::Floor => below,
        Rounding::Ceil => above,
        Rounding::Truncate => {
            if x > 0 {
                below
            } else {
                above.filter(|_| x < 0)
            }
        }
        Rounding::NearestEven | Rounding::NearestAway => match (below, above) {
            (Some(b), Some(a)) => {
                let (vb, va) = (sixth(b).unwrap(), sixth(a).unwrap());
                let (db, da) = (x - vb, va - x);
                Some(if db < da {
                    b
                } else if da < db {
                    a
                } else if md == Rounding::NearestEven {
                    if b % 2 == 0 { b } else { a }
                } else if vb.unsigned_abs() > va.unsigned_abs() {
                    b
                } else {
                    a
                })
            }
            (b, a) => b.or(a),
        },
    }
}

/// `x` sixths of a stop through both label codecs by `md`, against [label_rounded] over the tables written out here;
/// each code it gives decodes to its label's exact value.
pub(crate) fn labels_encode(x: i16, md: Rounding) {
    let got = ApertureLabel::encode(Apex::new(x), md);
    let want = label_rounded(&APERTURE_AV, x, md);
    assert_eq!(got.map(usize::from), want, "Av {x}/6 {md:?}");
    if let Some(c) = got {
        let back = Coded::<ApertureLabel>::try_from_raw(c).unwrap().get();
        assert_eq!(
            Some(back.get()),
            APERTURE_AV.get(usize::from(c)).map(|s| s * 6)
        );
    }
    let got = ShutterLabel::encode(Apex::new(x), md);
    let want = label_rounded(&SHUTTER_TV, x, md);
    assert_eq!(got.map(usize::from), want, "Tv {x}/6 {md:?}");
    if let Some(c) = got {
        let back = Coded::<ShutterLabel>::try_from_raw(c).unwrap().get();
        assert_eq!(
            Some(back.get()),
            SHUTTER_TV.get(usize::from(c)).map(|s| s * 6)
        );
    }
}

/// Every byte as a code of both label codecs: a code of the table decodes to its value and has a label, the others
/// are spare, refused, and have none.
pub(crate) fn label_codes(code: u8) {
    let i = usize::from(code);
    let a = Coded::<ApertureLabel>::try_from_raw(code)
        .ok()
        .map(|c| c.get().get());
    assert_eq!(a, APERTURE_AV.get(i).map(|s| s * 6), "aperture code {code}");
    assert_eq!(ApertureLabel::label(code).is_some(), a.is_some());
    let t = Coded::<ShutterLabel>::try_from_raw(code)
        .ok()
        .map(|c| c.get().get());
    assert_eq!(t, SHUTTER_TV.get(i).map(|s| s * 6), "shutter code {code}");
    assert_eq!(ShutterLabel::label(code).is_some(), t.is_some());
}

#[cfg(all(not(kani), any(feature = "std", feature = "libm")))]
#[test]
fn every_u16_ratio_in_stops() {
    for r in 0..=u16::MAX {
        for md in crate::reference::MODES {
            level_in_stops(r, md);
        }
    }
}

#[cfg(all(not(kani), any(feature = "std", feature = "libm")))]
#[test]
fn every_whole_stop_gain() {
    for k in i8::MIN..=i8::MAX {
        whole_stop_gain(k);
    }
}

#[cfg(not(kani))]
#[test]
fn every_sixth_stop_f_number_and_time() {
    // i8 whole stops: −768 to 762 sixths
    for x in -768..=762 {
        whole_stops_exact(x);
    }
}

#[cfg(not(kani))]
#[test]
fn every_u16_speed() {
    for s in 0..=u32::from(u16::MAX) {
        sv_of_speed(s);
    }
    for k in 0..32 {
        sv_of_speed(25 << k);
        sv_of_speed((25 << k) / 8);
    }
}

#[cfg(not(kani))]
#[test]
fn every_label_code_and_value() {
    for code in 0..=u8::MAX {
        label_codes(code);
    }
    for x in -60..=90 {
        for md in crate::reference::MODES {
            labels_encode(x, md);
        }
    }
}

#[cfg(not(kani))]
#[test]
fn small_exif_rationals() {
    for num in -400..=400 {
        for den in -13..=13 {
            exif_rational(num, den);
        }
    }
}

#[cfg(not(any(kani, fuzzing)))]
mod strategy {
    use proptest::prelude::*;

    /// An Exif RATIONAL or SRATIONAL's part: any 32-bit value, weighted to the ends and to small ones.
    pub(crate) fn part() -> impl Strategy<Value = i64> {
        prop_oneof![
            any::<u32>().prop_map(i64::from),
            any::<i32>().prop_map(i64::from),
            (-1000_i64..=1000),
            Just(i64::from(u32::MAX)),
            Just(i64::from(i32::MIN)),
        ]
    }
}

#[cfg(not(any(kani, fuzzing)))]
proptest::proptest! {
    #[test]
    fn exif_rationals(num in strategy::part(), den in strategy::part()) {
        exif_rational(num, den);
    }

    #[test]
    fn exif_rationals_of_sixths(sixths in proptest::prelude::any::<i16>(), den in 1_i64..=1_000_000) {
        // an APEX value of whole sixths times den / 6: exact where 6 divides it
        exif_rational(i64::from(sixths) * den, den * 6);
    }
}

#[cfg(kani)]
mod proofs {
    /// Every i16 value through both label codecs by every rounding.
    #[kani::proof]
    #[kani::unwind(20)]
    fn labels_any() {
        let md = crate::reference::MODES[usize::from(kani::any_where(|i: &u8| *i < 5))];
        super::labels_encode(kani::any(), md);
    }

    /// Every byte as a code of both label codecs.
    #[kani::proof]
    #[kani::unwind(20)]
    fn label_codes_any() {
        super::label_codes(kani::any());
    }

    /// Annex C's Sv of every u16 speed.
    #[kani::proof]
    fn sv_any() {
        super::sv_of_speed(u32::from(kani::any::<u16>()));
    }
}
