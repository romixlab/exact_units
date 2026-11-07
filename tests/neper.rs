//! Levels in Np (LVL-2): the gains `from_np_const` works out at compile time against an 80-digit reference and
//! libm's `exp`, `np!`, and the run-time `to_np` / `from_np` in `f64` and `f32`, with 1 Np = 20/ln 10 dB.
#![cfg(feature = "electrical")]
#![expect(
    clippy::as_conversions,
    reason = "a test casts the values it chose and the references it computes; the cast lints say why each fits"
)]
#![expect(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    reason = "a test casts the values it chose: levels of a few tens of Np, gains below 2^53, and f32 comparisons that allow for the rounding"
)]

use core::f64::consts::{E, LN_10};
use exact_units::decibel::{Decibels, Nepers};
use exact_units::prefix::{Micro, Milli, Nano, One};
use exact_units::units::{Ampere, Per, Volt, Watt};
use exact_units::{Fixed, Rounding, np};

type VoltGain<P, T> = Fixed<Per<Volt, Volt>, P, T>;
type PowerGain<P, T> = Fixed<Per<Watt, Watt>, P, T>;

/// Levels from −40 to +20 Np in odd steps of up to three decimals.
fn levels() -> impl Iterator<Item = Nepers> {
    (-40_000i64..=20_000)
        .step_by(137)
        .map(|n| Nepers::new(n, 3))
}

/// How far an `f64` reference may be from the exact gain, relative: libm's own ulp and the rounding of its input
/// `y`, which `e^y` turns into `|y|` times as much.
fn tolerance(y: f64) -> f64 {
    (2.0 + y.abs()) * f64::EPSILON
}

fn ulps(a: f64, b: f64) -> u64 {
    a.to_bits().abs_diff(b.to_bits())
}

/// `(Np digits, decimals, e^Np, e^(2·Np))`, from Python's `decimal` at 40 digits.
const GOLDEN: &[(i64, u8, &str, &str)] = &[
    (
        -1,
        0,
        "0.3678794411714423215955237701614608674458",
        "0.1353352832366126918939994949724844034076",
    ),
    (
        -5,
        1,
        "0.6065306597126334236037995349911804534419",
        "0.3678794411714423215955237701614608674458",
    ),
    (
        1,
        0,
        "2.718281828459045235360287471352662497757",
        "7.389056098930650227230427460575007813180",
    ),
    (
        25,
        1,
        "12.18249396070347343807017595116796618318",
        "148.4131591025766034211155800405522796235",
    ),
];

#[test]
fn golden_values_are_exact() {
    for &(n, d, v, p) in GOLDEN {
        let np = Nepers::new(n, d);
        assert_eq!(
            VoltGain::<One, f64>::from_np_const_rounded(np, Rounding::NearestEven).get(),
            v.parse::<f64>().unwrap(),
            "{np:?}"
        );
        assert_eq!(
            PowerGain::<One, f64>::from_np_const_rounded(np, Rounding::NearestEven).get(),
            p.parse::<f64>().unwrap(),
            "{np:?}"
        );
        assert_eq!(
            VoltGain::<One, f32>::from_np_const_rounded(np, Rounding::NearestEven).get(),
            v.parse::<f32>().unwrap(),
            "{np:?}"
        );
        assert_eq!(
            PowerGain::<One, f32>::from_np_const_rounded(np, Rounding::NearestEven).get(),
            p.parse::<f32>().unwrap(),
            "{np:?}"
        );
    }
}

#[test]
fn macro_and_const_fn() {
    const A: VoltGain<Micro, u32> = np!(-1, NearestEven);
    const B: PowerGain<Micro, u32> = np!(-0.5, NearestEven);
    const C: VoltGain<One, f64> = np!(1, NearestEven);
    const D: PowerGain<Nano, u64> = np!(0.25, NearestEven);
    assert_eq!(A.get(), 367_879);
    assert_eq!(B.get(), 367_879);
    assert_eq!(C.get(), E);
    // e^0.5 = 1.6487212707
    assert_eq!(D.get(), 1_648_721_271);
    assert_eq!(Nepers::parse("-0.5"), Nepers::new(-5, 1));
    assert_eq!(Nepers::parse("+ 2"), Nepers::new(2, 0));
    assert_eq!(Nepers::new(-5, 1).to_f64(), -0.5);
    let z: Fixed<Per<Ampere, Ampere>, Milli, u16> = np!(0);
    assert_eq!(z.get(), 1000);
}

#[test]
fn matches_libm() {
    // the compile-time gain is correctly rounded, libm's exp is within an ulp or so
    for np in levels() {
        let x = np.to_f64();
        let v = VoltGain::<One, f64>::from_np_const_rounded(np, Rounding::NearestEven).get();
        let p = PowerGain::<One, f64>::from_np_const_rounded(np, Rounding::NearestEven).get();
        assert!(
            (v / libm::exp(x) - 1.0).abs() <= tolerance(x),
            "{np:?}: {v}"
        );
        assert!(
            (p / libm::exp(2.0 * x) - 1.0).abs() <= tolerance(2.0 * x),
            "{np:?}: {p}"
        );
    }
}

#[test]
fn integers_are_nearest() {
    let mut checked = 0;
    for np in levels() {
        let x = np.to_f64();
        let want = libm::exp(x) * 1e6;
        // micro-units up to 10^9 µV/V, where the reference is good to a millionth of a step
        if (want.fract() - 0.5).abs() > 1e-4 && want < 1e9 {
            let g = VoltGain::<Micro, u64>::from_np_const_rounded(np, Rounding::NearestEven).get();
            assert_eq!(g as f64, want.round(), "{np:?}");
            checked += 1;
        }
    }
    assert!(checked > 300, "{checked}");
}

#[test]
fn edges() {
    // the largest and smallest u8: e^5.54 = 254.7 -> 255, e^5.55 = 257 is past
    let g: VoltGain<One, u8> = np!(5.54, NearestEven);
    assert_eq!(g.get(), 255);
    // far below one step rounds to 0, however far
    for n in [-100i64, -10_000_000_000, i64::MIN / 2, i64::MIN] {
        assert_eq!(
            PowerGain::<One, u64>::from_np_const_rounded(Nepers::new(n, 0), Rounding::NearestEven)
                .get(),
            0,
            "{n} Np"
        );
    }
    // near the ends of f64's range (e^709.78 is f64::MAX)
    let top: VoltGain<One, f64> = np!(709, NearestEven);
    assert!(ulps(top.get(), libm::exp(709.0)) <= 800);
    let low: VoltGain<One, f64> = np!(-708, NearestEven);
    assert!(ulps(low.get(), libm::exp(-708.0)) <= 800);
}

#[test]
#[should_panic(expected = "this value is too large for its storage")]
fn too_large_for_the_storage() {
    let _ = VoltGain::<One, u8>::from_np_const_rounded(Nepers::new(556, 2), Rounding::NearestEven);
}

#[test]
#[should_panic(expected = "this value is too large for its storage")]
fn far_too_large_for_the_storage() {
    let _ = PowerGain::<One, u64>::from_np_const_rounded(
        Nepers::new(i64::MAX, 0),
        Rounding::NearestEven,
    );
}

#[test]
#[should_panic(expected = "this value is past the range of its float storage")]
fn past_the_float_range() {
    let _ = VoltGain::<One, f64>::from_np_const_rounded(Nepers::new(710, 0), Rounding::NearestEven);
}

#[test]
#[should_panic(expected = "a level in Np takes up to 9 decimals")]
fn ten_decimals_refused() {
    let _ = Nepers::new(1, 10);
}

/// 1 Np = 20/ln 10 dB, the same for both forms: the compile-time gains at a level in dB and in Np agree.
#[test]
fn np_and_db_are_one_level() {
    // 20/ln 10 dB per Np = 8.685889638065035 dB
    const DB_PER_NP: f64 = 20.0 / LN_10;
    for np in levels() {
        let x = np.to_f64();
        let n = VoltGain::<One, f64>::from_np_const_rounded(np, Rounding::NearestEven).get();
        // the same level in dB as a decimal with 9 decimals, 5·10^-10 dB off at most
        let db_nano = (x * DB_PER_NP * 1e9).round() as i64;
        let d = VoltGain::<One, f64>::from_db_const_rounded(
            Decibels::new(db_nano, 9),
            Rounding::NearestEven,
        )
        .get();
        assert!(
            (n / d - 1.0).abs() < 1e-9 * (1.0 + x.abs()),
            "{x} Np: {n} {d}"
        );
        let p = PowerGain::<One, f64>::from_np_const_rounded(np, Rounding::NearestEven).get();
        let d = PowerGain::<One, f64>::from_db_const_rounded(
            Decibels::new(db_nano, 9),
            Rounding::NearestEven,
        )
        .get();
        assert!(
            (p / d - 1.0).abs() < 1e-9 * (1.0 + x.abs()),
            "{x} Np: {p} {d}"
        );
    }
}

#[cfg(any(feature = "std", feature = "libm"))]
mod run_time {
    use super::*;

    #[test]
    fn forms() {
        let e = E;
        assert_eq!(VoltGain::<One, f64>::bounded(e).to_np(), 1.0);
        assert!((PowerGain::<One, f64>::bounded(e * e).to_np() - 1.0).abs() < 1e-15);
        assert_eq!(VoltGain::<One, f64>::bounded(1.0).to_np(), 0.0);
        // 100 mV/V of an amplitude is ln 0.1 = -2.302585 Np; 100 mW/W of a power is half of it
        let a = VoltGain::<Milli, u32>::new(100).to_np();
        let b = PowerGain::<Milli, u32>::new(100).to_np();
        assert!((a + LN_10).abs() < 1e-15);
        assert!((b * 2.0 - a).abs() < 1e-15);
        assert_eq!(VoltGain::<Milli, u32>::from_np(0.0), Some(Fixed::new(1000)));
        assert_eq!(VoltGain::<One, u16>::from_np(0.5), None); // 1.6487.. is no whole number
    }

    /// `to_np` against `to_db`: Np · 20/ln 10 is the dB, a few ulps off.
    #[test]
    fn identity_with_db() {
        for np in levels() {
            let x = np.to_f64();
            let v = VoltGain::<One, f64>::from_np(x).unwrap();
            let p = PowerGain::<One, f64>::from_np(x).unwrap();
            for (n, db) in [(v.to_np(), v.to_db()), (p.to_np(), p.to_db())] {
                let want = n * (20.0 / LN_10);
                assert!(
                    (want - db).abs() <= 1e-13 * (1.0 + db.abs()),
                    "{x} Np: {want} dB vs {db} dB"
                );
            }
            assert!((v.to_np() - x).abs() < 1e-12 * (1.0 + x.abs()), "{x}");
            assert!((p.to_np() - x).abs() < 1e-12 * (1.0 + x.abs()), "{x}");
        }
    }

    #[test]
    fn f32_and_f64_agree() {
        for np in levels() {
            let x = np.to_f64();
            let g = VoltGain::<One, f64>::from_np(x).unwrap();
            let g32 = VoltGain::<One, f32>::from_np_f32(x as f32).unwrap();
            assert!(
                (f64::from(g32.get()) / g.get() - 1.0).abs() < 1e-5 * (1.0 + x.abs()),
                "{x} Np"
            );
            assert!((g.to_np() - x).abs() < 1e-12 * (1.0 + x.abs()), "{x}");
            assert!(
                (f64::from(g32.to_np_f32()) - x).abs() < 1e-3 + x.abs() * 1e-5,
                "{x} Np: {}",
                g32.to_np_f32()
            );
        }
        let c: VoltGain<Micro, u32> = np!(-1, NearestEven);
        let r = VoltGain::<Micro, f32>::from_np_f32(-1.0).unwrap();
        assert!((c.get() as f32 - r.get()).abs() <= 1.0);
        assert_eq!(
            PowerGain::<Milli, u32>::from_np_f32(0.0),
            Some(Fixed::new(1000))
        );
        assert_eq!(VoltGain::<Milli, u16>::new(1000).to_np_f32(), 0.0);
    }
}
