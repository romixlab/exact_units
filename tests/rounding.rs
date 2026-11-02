//! Rounding conversions and overflow policies (STD-63): every mode against a reference written independently of the
//! crate's own (Euclidean division), the typed conversion against the run-time one, factors with π.
#![expect(
    clippy::as_conversions,
    reason = "a test casts the values it chose and the references it computes; the cast lints say why each fits"
)]
#![expect(
    clippy::arithmetic_side_effects,
    reason = "a test computes with the values it chose, which fit; an overflow fails the test"
)]

use exact_units::prefix::{Centi, Deci, Kilo, Micro, Milli, One, Pow2, Rational};
use exact_units::units::{Radian, Ratio, Second, Volt};
use exact_units::{
    Bam16, Fixed, FixedScale, Magnitude, Overflow, Q15, Range, Repr, Requantizer, Rounding,
    Storage, StorageKind, requantize,
};

const MODES: [Rounding; 5] = [
    Rounding::Truncate,
    Rounding::Floor,
    Rounding::Ceil,
    Rounding::NearestEven,
    Rounding::NearestAway,
];

/// `p / d` rounded by `r`, from the Euclidean floor.
fn reference(p: i128, d: i128, r: Rounding) -> i128 {
    let f = p.div_euclid(d);
    let rem = p - f * d; // 0 <= rem < d
    if rem == 0 {
        return f;
    }
    let c = f + 1;
    match r {
        Rounding::Floor => f,
        Rounding::Ceil => c,
        Rounding::Truncate => {
            if p < 0 {
                c
            } else {
                f
            }
        }
        Rounding::NearestEven | Rounding::NearestAway => match (2 * rem).cmp(&d) {
            core::cmp::Ordering::Less => f,
            core::cmp::Ordering::Greater => c,
            core::cmp::Ordering::Equal => match r {
                Rounding::NearestAway => {
                    if p < 0 {
                        f
                    } else {
                        c
                    }
                }
                _ => {
                    if f % 2 == 0 {
                        f
                    } else {
                        c
                    }
                }
            },
        },
    }
}

/// `x` into `min..=max` by `o`, wrapping modulo the span.
fn settle(x: i128, min: i128, max: i128, o: Overflow) -> Option<i128> {
    match o {
        Overflow::Checked => (min..=max).contains(&x).then_some(x),
        Overflow::Saturate => Some(x.clamp(min, max)),
        Overflow::Wrap => Some((x - min).rem_euclid(max - min + 1) + min),
    }
}

/// Every i16 from scale `P` to `P2` in `T2`, every mode and policy: the typed conversion, the reference and the
/// run-time one agree.
#[expect(
    clippy::indexing_slicing,
    reason = "ri and oi enumerate MODES and policies, which dynamic is built from"
)]
#[expect(
    clippy::unwrap_used,
    reason = "a test helper: a value that doesn't fit fails the test"
)]
fn exhaustive<P: FixedScale, P2: FixedScale, T2: Storage + Into<i128>>(num: i128, den: i128) {
    let factor = P::SCALE.div(P2::SCALE);
    #[expect(clippy::cast_sign_loss, reason = "the callers pass a positive ratio")]
    let ratio = (num as u128, den as u128);
    assert_eq!(factor.to_ratio(), Some(ratio));
    let to = StorageKind::of::<T2>();
    let policies = [Overflow::Checked, Overflow::Saturate, Overflow::Wrap];
    let dynamic: Vec<Vec<Requantizer>> = MODES
        .iter()
        .map(|&r| {
            policies
                .iter()
                .map(|&o| Requantizer::new(&factor, to, r, o))
                .collect()
        })
        .collect();
    for x in i16::MIN..=i16::MAX {
        let v: Fixed<Volt, P, i16> = Fixed::new(x);
        for (ri, r) in MODES.into_iter().enumerate() {
            let q = reference(i128::from(x) * num, den, r);
            for (oi, o) in policies.into_iter().enumerate() {
                let want = settle(q, T2::MIN_INT, T2::MAX_INT, o);
                let got = v.requantize::<P2, T2>(r, o).map(|f| f.get().into());
                assert_eq!(got, want, "{x} {r:?} {o:?}");
                let dynamic = dynamic[ri][oi].apply(Repr::Int(i128::from(x)));
                assert_eq!(dynamic, want.map(Repr::Int), "run time: {x} {r:?} {o:?}");
            }
            if x % 61 == 0 {
                // the named forms are requantize with one policy each
                assert_eq!(
                    v.to_rounded::<P2, T2>(r).map(|f| f.get().into()),
                    settle(q, T2::MIN_INT, T2::MAX_INT, Overflow::Checked)
                );
                assert_eq!(
                    v.to_saturating::<P2, T2>(r).get().into(),
                    settle(q, T2::MIN_INT, T2::MAX_INT, Overflow::Saturate).unwrap()
                );
                assert_eq!(
                    v.to_wrapping::<P2, T2>(r).get().into(),
                    settle(q, T2::MIN_INT, T2::MAX_INT, Overflow::Wrap).unwrap()
                );
            }
        }
    }
}

// every i16, one test per pair of scales so they run in parallel (each about 0.2 s in debug)

/// A division in i16's own width: mV to 0.1 V.
#[test]
fn every_i16_milli_to_deci() {
    exhaustive::<Milli, Deci, i8>(1, 100);
}

#[test]
fn every_i16_milli_to_one() {
    exhaustive::<Milli, One, i8>(1, 1000);
}

#[test]
fn every_i16_milli_to_centi_unsigned() {
    exhaustive::<Milli, Centi, u8>(1, 10);
}

/// A fraction: the multiply and divide in 64 bits.
#[test]
fn every_i16_fraction() {
    exhaustive::<Rational<7, 3>, One, i16>(7, 3);
}

#[test]
fn every_i16_power_of_two() {
    exhaustive::<Pow2<-15>, Pow2<-8>, i8>(1, 128);
}

/// Whole factors, past the target's range.
#[test]
fn every_i16_whole_factor() {
    exhaustive::<Milli, Micro, i16>(1000, 1);
    exhaustive::<Kilo, One, u16>(1000, 1);
}

#[test]
fn every_i16_duty() {
    exhaustive::<Rational<1, 65535>, Rational<1, 255>, u8>(1, 257);
}

#[test]
fn wide_storages() {
    // u64 µs to s: the division in u64, every mode
    let t: Fixed<Second, Micro, u64> = Fixed::new(u64::MAX);
    let s = |r| t.to_rounded::<One, u64>(r).unwrap().get();
    assert_eq!(s(Rounding::Truncate), 18_446_744_073_709);
    assert_eq!(s(Rounding::Ceil), 18_446_744_073_710);
    assert_eq!(s(Rounding::NearestEven), 18_446_744_073_710); // .551615
    // i64 by 7/3 in 128 bits
    let x: Fixed<Ratio, Rational<7, 3>, i64> = Fixed::new(i64::MIN);
    let want = reference(i128::from(i64::MIN) * 7, 3, Rounding::NearestEven);
    assert_eq!(
        x.to_saturating::<One, i64>(Rounding::NearestEven).get(),
        i64::MIN
    );
    assert_eq!(
        i128::from(x.to_wrapping::<One, i64>(Rounding::NearestEven).get()),
        settle(
            want,
            i128::from(i64::MIN),
            i128::from(i64::MAX),
            Overflow::Wrap
        )
        .unwrap()
    );
    assert_eq!(x.to_rounded::<One, i64>(Rounding::NearestEven), None);
}

#[test]
fn floats_round_once() {
    let v: Fixed<Volt, One, f32> = Fixed::new(2.5);
    let r = |r| v.to_rounded::<One, i16>(r).map(Fixed::get);
    assert_eq!(r(Rounding::NearestEven), Some(2));
    assert_eq!(r(Rounding::NearestAway), Some(3));
    assert_eq!(r(Rounding::Floor), Some(2));
    let v: Fixed<Volt, One, f64> = Fixed::new(-2.5);
    let r = |r| v.to_rounded::<One, i16>(r).map(Fixed::get);
    assert_eq!(r(Rounding::NearestEven), Some(-2));
    assert_eq!(r(Rounding::NearestAway), Some(-3));
    assert_eq!(r(Rounding::Truncate), Some(-2));
    assert_eq!(r(Rounding::Ceil), Some(-2));
    assert_eq!(r(Rounding::Floor), Some(-3));
    // mV in f32 to V in i8: 1499.9 mV is 1 V to nearest
    let v: Fixed<Volt, Milli, f32> = Fixed::new(1499.9);
    assert_eq!(
        v.to_rounded::<One, i8>(Rounding::NearestEven)
            .map(Fixed::get),
        Some(1)
    );
    // out of range and NaN by policy
    let big: Fixed<Volt, One, f64> = Fixed::new(1e10);
    assert_eq!(big.to_rounded::<One, i16>(Rounding::Truncate), None);
    assert_eq!(
        big.to_saturating::<One, i16>(Rounding::Truncate).get(),
        i16::MAX
    );
    #[expect(
        clippy::cast_possible_truncation,
        reason = "the expected value is the low 16 bits, as the wrapping policy keeps them"
    )]
    let wrapped = 10_000_000_000i64 as i16;
    assert_eq!(
        big.to_wrapping::<One, i16>(Rounding::Truncate).get(),
        wrapped
    );
    let nan: Fixed<Volt, One, f64> = Fixed::new(f64::NAN);
    assert_eq!(nan.to_rounded::<One, i16>(Rounding::Truncate), None);
    assert_eq!(nan.to_saturating::<One, i16>(Rounding::Truncate).get(), 0);
    assert_eq!(nan.to_wrapping::<One, u8>(Rounding::Truncate).get(), 0);
    let top: Fixed<Volt, One, f64> = Fixed::new(1.8446744073709552e19); // 2⁶⁴
    assert_eq!(
        top.to_saturating::<One, u64>(Rounding::Truncate).get(),
        u64::MAX
    );
    // integers into a float aren't rounded: the float's own rounding
    let n: Fixed<Volt, Milli, i32> = Fixed::new(1);
    assert_eq!(
        n.to_rounded::<One, f64>(Rounding::Floor).map(Fixed::get),
        Some(0.001)
    );
}

#[test]
fn float_sources_settle_by_policy() {
    let sat = |x: f64, r| {
        Fixed::<Volt, One, f64>::new(x)
            .to_saturating::<One, i16>(r)
            .get()
    };
    // in range: the rounded value, not an end
    assert_eq!(sat(2.5, Rounding::NearestEven), 2);
    assert_eq!(sat(-2.5, Rounding::NearestEven), -2);
    assert_eq!(sat(3.5, Rounding::NearestEven), 4);
    assert_eq!(sat(1234.0, Rounding::Floor), 1234);
    assert_eq!(sat(-1234.0, Rounding::Floor), -1234);
    assert_eq!(sat(0.0, Rounding::Floor), 0);
    // the ends and beyond
    assert_eq!(sat(32767.0, Rounding::Floor), i16::MAX);
    assert_eq!(sat(32766.0, Rounding::Floor), 32766);
    assert_eq!(sat(32767.4, Rounding::NearestEven), i16::MAX);
    assert_eq!(sat(-32768.0, Rounding::Floor), i16::MIN);
    assert_eq!(sat(-32767.0, Rounding::Floor), -32767);
    assert_eq!(sat(1e30, Rounding::Floor), i16::MAX);
    assert_eq!(sat(-1e30, Rounding::Floor), i16::MIN);
    assert_eq!(sat(f64::INFINITY, Rounding::Floor), i16::MAX);
    assert_eq!(sat(f64::NEG_INFINITY, Rounding::Floor), i16::MIN);
    let sat_u = |x: f64| {
        Fixed::<Volt, One, f64>::new(x)
            .to_saturating::<One, u64>(Rounding::Floor)
            .get()
    };
    assert_eq!(sat_u(-1.0), 0);
    assert_eq!(sat_u(1e19), 10_000_000_000_000_000_000);
    assert_eq!(sat_u(1.8446744073709552e19), u64::MAX);
    assert_eq!(sat_u(1e300), u64::MAX);
    // the same into 64-bit signed: in range from both sides of 2⁶³
    let sat_i = |x: f64| {
        Fixed::<Volt, One, f64>::new(x)
            .to_saturating::<One, i64>(Rounding::Floor)
            .get()
    };
    assert_eq!(sat_i(9.0e18), 9_000_000_000_000_000_000);
    assert_eq!(sat_i(-9.0e18), -9_000_000_000_000_000_000);
    assert_eq!(sat_i(9.3e18), i64::MAX);
    assert_eq!(sat_i(-9.3e18), i64::MIN);

    // wrapping: modulo 2^bits of the whole value, 0 for what has no bits (infinities, 2¹²⁷ and up)
    let wrap = |x: f64| {
        Fixed::<Volt, One, f64>::new(x)
            .to_wrapping::<One, i16>(Rounding::Floor)
            .get()
    };
    assert_eq!(wrap(2.5), 2);
    assert_eq!(wrap(-2.5), -3);
    assert_eq!(wrap(65_537.0), 1);
    assert_eq!(wrap(-65_537.0), -1);
    assert_eq!(wrap(f64::INFINITY), 0);
    assert_eq!(wrap(f64::NEG_INFINITY), 0);
    assert_eq!(wrap(f64::NAN), 0);
    assert_eq!(wrap(2f64.powi(127)), 0);
    assert_eq!(wrap(-(2f64.powi(127))), 0);
    assert_eq!(wrap(2f64.powi(126)), 0);
    assert_eq!(wrap(2f64.powi(126) + 2f64.powi(80)), 0);
    assert_eq!(wrap(2f64.powi(40) + 7.0), 7);
    assert_eq!(wrap(-(2f64.powi(40)) - 7.0), -7);
}

const KINDS: [StorageKind; 3] = [
    StorageKind::Int {
        bits: 16,
        signed: true,
    },
    StorageKind::F32,
    StorageKind::F64,
];

#[test]
fn requantizer_float_sources_and_targets() {
    let i16_kind = KINDS[0];
    let apply = |factor: &Magnitude, x: Repr, to, r, o| requantize(x, factor, to, r, o);
    let one = Magnitude::ONE;
    // a float into an integer: rounded once by the mode, then settled
    for (x, r, want) in [
        (2.5, Rounding::NearestEven, 2),
        (2.5, Rounding::NearestAway, 3),
        (2.5, Rounding::Floor, 2),
        (2.5, Rounding::Ceil, 3),
        (-2.5, Rounding::NearestEven, -2),
        (-2.5, Rounding::NearestAway, -3),
        (-2.5, Rounding::Truncate, -2),
        (-2.5, Rounding::Floor, -3),
        (3.5, Rounding::NearestEven, 4),
        (2.4999, Rounding::NearestAway, 2),
        (2.5001, Rounding::NearestEven, 3),
    ] {
        assert_eq!(
            apply(&one, Repr::Float(x), i16_kind, r, Overflow::Checked),
            Some(Repr::Int(want)),
            "{x} {r:?}"
        );
    }
    // out of range: checked refuses, saturating takes the end, wrapping the low bits
    let big = Repr::Float(1e10);
    let r = Rounding::NearestEven;
    assert_eq!(apply(&one, big, i16_kind, r, Overflow::Checked), None);
    assert_eq!(
        apply(&one, big, i16_kind, r, Overflow::Saturate),
        Some(Repr::Int(32767))
    );
    assert_eq!(
        apply(&one, Repr::Float(-1e10), i16_kind, r, Overflow::Saturate),
        Some(Repr::Int(-32768))
    );
    assert_eq!(
        apply(&one, big, i16_kind, r, Overflow::Wrap),
        Some(Repr::Int(i128::from(10_000_000_000i64 as i16)))
    );
    // a float that is not whole after the factor and rounding can't be, and NaN and infinities have no value
    for o in [Overflow::Checked, Overflow::Saturate, Overflow::Wrap] {
        let nan = apply(&one, Repr::Float(f64::NAN), i16_kind, r, o);
        assert_eq!(
            nan,
            (o != Overflow::Checked).then_some(Repr::Int(0)),
            "{o:?}"
        );
        let inf = apply(&one, Repr::Float(f64::INFINITY), i16_kind, r, o);
        let want = match o {
            Overflow::Checked => None,
            Overflow::Saturate => Some(Repr::Int(32767)),
            Overflow::Wrap => Some(Repr::Int(0)),
        };
        assert_eq!(inf, want, "{o:?}");
    }
    // a factor on a float: 25 · 0.1 is 2.5 (tie to even: 2)
    let tenth = Magnitude::pow10(-1);
    assert_eq!(
        apply(&tenth, Repr::Float(25.0), i16_kind, r, Overflow::Checked),
        Some(Repr::Int(2))
    );
    assert_eq!(
        apply(&tenth, Repr::Float(35.0), i16_kind, r, Overflow::Checked),
        Some(Repr::Int(4))
    );
    // unsigned: below 0 is refused, 0, or wrapped
    let u8_kind = StorageKind::Int {
        bits: 8,
        signed: false,
    };
    assert_eq!(
        apply(&one, Repr::Float(-1.0), u8_kind, r, Overflow::Checked),
        None
    );
    assert_eq!(
        apply(&one, Repr::Float(-1.0), u8_kind, r, Overflow::Saturate),
        Some(Repr::Int(0))
    );
    assert_eq!(
        apply(&one, Repr::Float(-1.0), u8_kind, r, Overflow::Wrap),
        Some(Repr::Int(255))
    );
    assert_eq!(
        apply(&one, Repr::Float(255.0), u8_kind, r, Overflow::Checked),
        Some(Repr::Int(255))
    );
    assert_eq!(
        apply(&one, Repr::Float(256.0), u8_kind, r, Overflow::Checked),
        None
    );
    // into floats: the factor applied to an integer or a float, f32 rounded once more
    let third = Magnitude::ratio(1, 3);
    assert_eq!(
        apply(&third, Repr::Int(3), KINDS[2], r, Overflow::Checked),
        Some(Repr::Float(1.0))
    );
    assert_eq!(
        apply(&third, Repr::Int(1), KINDS[2], r, Overflow::Checked),
        Some(Repr::Float(1.0 / 3.0))
    );
    assert_eq!(
        apply(&third, Repr::Int(1), KINDS[1], r, Overflow::Checked),
        Some(Repr::Float(f64::from(1.0f32 / 3.0)))
    );
    assert_eq!(
        apply(&third, Repr::Float(1.0), KINDS[1], r, Overflow::Checked),
        Some(Repr::Float(f64::from(1.0f32 / 3.0)))
    );
    assert_eq!(
        apply(
            &Magnitude::PI,
            Repr::Float(2.0),
            KINDS[2],
            r,
            Overflow::Checked
        ),
        Some(Repr::Float(2.0 * core::f64::consts::PI))
    );
    assert_eq!(
        apply(&Magnitude::PI, Repr::Int(2), KINDS[1], r, Overflow::Checked),
        Some(Repr::Float(f64::from(2.0 * core::f32::consts::PI)))
    );
}

#[test]
fn requantizer_fixed_point_factors() {
    let u64_kind = StorageKind::Int {
        bits: 64,
        signed: false,
    };
    let at = |factor: Magnitude, x: i128, r, o| {
        Requantizer::new(&factor, u64_kind, r, o).apply(Repr::Int(x))
    };
    let pi = |k: i8| Magnitude::PI.mul(Magnitude::pow2(k));
    // π · 2^-63 · 2^61: the mantissa of π is odd, so x = 2^61 at shift -62 is half a step above 0x6487ED5110B4611A
    let low = 0x6487_ED51_10B4_611Au128;
    let tie = |r| at(Magnitude::PI, 1 << 61, r, Overflow::Checked);
    assert_eq!(
        tie(Rounding::NearestEven),
        Some(Repr::Int(low.cast_signed()))
    );
    assert_eq!(
        tie(Rounding::NearestAway),
        Some(Repr::Int((low + 1).cast_signed()))
    );
    assert_eq!(tie(Rounding::Floor), Some(Repr::Int(low.cast_signed())));
    assert_eq!(tie(Rounding::Truncate), Some(Repr::Int(low.cast_signed())));
    assert_eq!(
        tie(Rounding::Ceil),
        Some(Repr::Int((low + 1).cast_signed()))
    );
    // 2^63 times π: past u64 for any x ≥ 1, saturated or wrapped or refused; below 1 it rounds to 0 or 1
    let big = pi(63);
    assert_eq!(at(big, 1, Rounding::Floor, Overflow::Checked), None);
    assert_eq!(
        at(big, 1, Rounding::Floor, Overflow::Saturate),
        Some(Repr::Int(i128::from(u64::MAX)))
    );
    assert_eq!(
        at(big, -1, Rounding::Floor, Overflow::Saturate),
        Some(Repr::Int(0))
    );
    // the low 64 bits of 0xC90FDAA22168C235 · 2 (shift 1)
    assert_eq!(
        at(big, 1, Rounding::Floor, Overflow::Wrap),
        Some(Repr::Int(i128::from(
            0xC90F_DAA2_2168_C235u64.wrapping_mul(2)
        )))
    );
    let tiny = pi(-64);
    assert_eq!(
        at(tiny, 1, Rounding::Ceil, Overflow::Checked),
        Some(Repr::Int(1))
    );
    assert_eq!(
        at(tiny, 1, Rounding::Floor, Overflow::Checked),
        Some(Repr::Int(0))
    );
    assert_eq!(
        at(tiny, 1, Rounding::NearestEven, Overflow::Checked),
        Some(Repr::Int(0))
    );
    assert_eq!(
        at(tiny, -1, Rounding::Floor, Overflow::Saturate),
        Some(Repr::Int(0))
    );
    assert_eq!(at(tiny, -1, Rounding::Floor, Overflow::Checked), None);
    // x of more than 64 bits has no fixed-point product
    assert_eq!(at(pi(0), 1 << 64, Rounding::Floor, Overflow::Checked), None);
    assert_eq!(
        at(pi(0), -(1 << 64), Rounding::Floor, Overflow::Saturate),
        None
    );
    assert_eq!(
        at(pi(0), (1 << 64) - 1, Rounding::Floor, Overflow::Saturate),
        Some(Repr::Int(i128::from(u64::MAX)))
    );
    assert_eq!(
        at(pi(0), 0, Rounding::Floor, Overflow::Checked),
        Some(Repr::Int(0))
    );
}

#[test]
fn requantizer_ratios_past_128_bits() {
    let i64_kind = StorageKind::Int {
        bits: 64,
        signed: true,
    };
    let at = |factor: Magnitude, x: i128, o| {
        requantize(Repr::Int(x), &factor, i64_kind, Rounding::Floor, o)
    };
    // 10^20 · 10^30 is past i128: a whole factor, so saturating and wrapping still know the answer
    let f = Magnitude::pow10(30);
    let x = 10i128.pow(20);
    assert_eq!(at(f, x, Overflow::Checked), None);
    assert_eq!(
        at(f, x, Overflow::Saturate),
        Some(Repr::Int(i128::from(i64::MAX)))
    );
    assert_eq!(
        at(f, -x, Overflow::Saturate),
        Some(Repr::Int(i128::from(i64::MIN)))
    );
    let low = 10u128.pow(20).wrapping_mul(10u128.pow(30)) as u64;
    assert_eq!(
        at(f, x, Overflow::Wrap),
        Some(Repr::Int(i128::from(low.cast_signed())))
    );
    let low = (-(10i128.pow(20))).wrapping_mul(10i128.pow(30)) as u64;
    assert_eq!(
        at(f, -x, Overflow::Wrap),
        Some(Repr::Int(i128::from(low.cast_signed())))
    );
    // a fraction whose product doesn't fit has no answer at all
    let frac = Magnitude::pow10(30).mul(Magnitude::ratio(1, 3));
    assert_eq!(at(frac, x, Overflow::Saturate), None);
    assert_eq!(at(frac, x, Overflow::Wrap), None);
    // x · num fits, den does: ordinary rounding
    assert_eq!(
        at(Magnitude::ratio(1, 3), i128::MAX, Overflow::Checked),
        None,
        "i128::MAX / 3 is past i64"
    );
    assert_eq!(
        at(Magnitude::ratio(1, 3), 10, Overflow::Checked),
        Some(Repr::Int(3))
    );
}

exact_units::scale! {
    /// Steps of 1/1000 °, over the unit radian: the scale is π/180000, not a fraction of integers.
    struct MilliDegree = "m°", Exact(1 / 180000, pi 1);
}

#[test]
fn fixed_points() {
    // the top 64 bits of each, rounded to nearest
    assert_eq!(Magnitude::PI.fixed_point(), (0xc90fdaa22168c235, -62));
    assert_eq!(Magnitude::LN2.fixed_point(), (0xb17217f7d1cf79ac, -64));
    assert_eq!(Magnitude::LN10.fixed_point(), (0x935d8dddaaa8ac17, -62));
    assert_eq!(
        Magnitude::pow2(1).sqrt().fixed_point(),
        (0xb504f333f9de6484, -63)
    );
    assert_eq!(
        Magnitude::ratio(1, 3).fixed_point(),
        (0xaaaaaaaaaaaaaaab, -65)
    );
    assert_eq!(Magnitude::ONE.fixed_point(), (1 << 63, -63));
    assert_eq!(Magnitude::pow10(-6).fixed_point().0, 0x8637bd05af6c69b6);
}

#[test]
fn degrees_to_radians() {
    use core::f64::consts::PI;
    // an integer of m° to µrad and back: a factor with π, rounded, never a compile error with to_rounded
    for x in (-360_000i32..=360_000).step_by(7) {
        let deg: Fixed<Radian, MilliDegree, i32> = Fixed::new(x);
        let urad = f64::from(x) * PI / 180_000.0 * 1e6;
        for r in MODES {
            let got = deg.to_rounded::<Micro, i32>(r).unwrap().get();
            let want = match r {
                Rounding::Truncate => urad.trunc(),
                Rounding::Floor => urad.floor(),
                Rounding::Ceil => urad.ceil(),
                Rounding::NearestEven => urad.round_ties_even(),
                Rounding::NearestAway => urad.round(),
            };
            // f64 is within 1e-9 of the truth here, nowhere near a rounding step for x != 0
            assert_eq!(f64::from(got), want, "{x} m° {r:?}");
        }
        let back: Fixed<Radian, MilliDegree, i32> = deg
            .to_rounded::<Micro, i32>(Rounding::NearestEven)
            .unwrap()
            .to_rounded(Rounding::NearestEven)
            .unwrap();
        // 1 µrad is 0.057 m°: back within one step
        assert!((back.get() - x).abs() <= 1, "{x}");
    }
    // the run-time path takes the same fixed point
    let factor = MilliDegree::SCALE.div(Micro::SCALE);
    for x in [-1_000_000i64, -1, 0, 1, 45_000, 90_000, i64::from(i32::MAX)] {
        for r in MODES {
            let typed = Fixed::<Radian, MilliDegree, i64>::new(x)
                .to_rounded::<Micro, i64>(r)
                .map(|f| Repr::Int(i128::from(f.get())));
            let dynamic = requantize(
                Repr::Int(i128::from(x)),
                &factor,
                StorageKind::of::<i64>(),
                r,
                Overflow::Checked,
            );
            assert_eq!(typed, dynamic, "{x} {r:?}");
        }
    }
}

exact_units::scale! {
    /// Steps of 1e-9 °: a radian is 5.7·10¹⁰ of them, a factor of more than 2³².
    struct NanoDegree = "n°", Exact(1 / 180000000000, pi 1);
}

/// `x` from `P` to `P2` in an i32 by every policy, through the 32-bit path and through the 64-bit one.
fn narrow_against_wide<P: FixedScale, P2: FixedScale>() {
    let xs = (i32::MIN..=i32::MAX)
        .step_by(1_048_583)
        .chain([i32::MIN, i32::MIN + 1, -1, 0, 1, i32::MAX - 1, i32::MAX])
        .chain(-5_000..=5_000);
    for x in xs {
        for r in MODES {
            for o in [Overflow::Checked, Overflow::Saturate, Overflow::Wrap] {
                let narrow = Fixed::<Radian, P, i32>::new(x)
                    .requantize::<P2, i32>(r, o)
                    .map(Fixed::get);
                let wide = Fixed::<Radian, P, i64>::new(i64::from(x))
                    .requantize::<P2, i32>(r, o)
                    .map(Fixed::get);
                assert_eq!(narrow, wide, "{x} {r:?} {o:?}");
            }
        }
    }
}

#[test]
fn narrow_fixed_point() {
    // storages of up to 32 bits settle in 64 bits: the same results as in 128
    narrow_against_wide::<MilliDegree, Micro>();
    narrow_against_wide::<Micro, MilliDegree>();
    // a factor past 2³²: the product's quotient doesn't fit the short path
    narrow_against_wide::<One, NanoDegree>();
    narrow_against_wide::<NanoDegree, One>();
}

#[test]
fn policies_by_kind() {
    // binary angles wrap: 3/4 turn + 1/2 turn is 1/4 turn
    let a = Bam16::of(49152) + Bam16::of(32768);
    assert_eq!(a.get(), 16384);
    let mut b = Bam16::of(0);
    b -= Bam16::of(1);
    assert_eq!(b.get(), u16::MAX);
    assert_eq!((Bam16::of(16384) * 5).get(), 16384);
    // Q formats saturate: 0.75 + 0.75 is the largest Q15, -1 / -1 too
    let q: Q15 = Fixed::of(24576);
    assert_eq!((q + q).get(), i16::MAX);
    let min: Q15 = Fixed::of(i16::MIN);
    assert_eq!((min / -1).get(), i16::MAX);
    assert_eq!([q, q, q].into_iter().sum::<Q15>().get(), i16::MAX);
    // the overflow policy is only in the type: a free conversion between them
    let plain = q.with_overflow::<exact_units::Unbounded>();
    assert_eq!(plain.checked_add(plain), None);
    assert_eq!(
        plain
            .with_overflow::<exact_units::Wrapping>()
            .wrapping_add(plain.with_overflow())
            .get(),
        -16384
    );
}

#[test]
fn bounds_prove_conversions() {
    // a 0..=3300 mV rail in µV fits an i32
    let v = Fixed::<Volt, Milli, i16, Range<0, 3300>>::bounded(3300);
    let uv: Fixed<Volt, Micro, i32> = v.into();
    assert_eq!(uv.get(), 3_300_000);
    // and rounds into a u8 of 0.1 V steps; every mode, the ends of the range
    for r in MODES {
        let lo = Fixed::<Volt, Milli, i16, Range<0, 3300>>::bounded(0);
        let dv: Fixed<Volt, Deci, u8> = lo.into_rounded(r);
        assert_eq!(dv.get(), 0);
        let dv: Fixed<Volt, Deci, u8> = v.into_rounded(r);
        assert_eq!(dv.get(), 33);
    }
    let mid = Fixed::<Volt, Milli, i16, Range<0, 3300>>::bounded(1250);
    let dv: Fixed<Volt, Deci, u8> = mid.into_rounded(Rounding::NearestEven);
    assert_eq!(dv.get(), 12);
    let dv: Fixed<Volt, Deci, u8> = mid.into_rounded(Rounding::NearestAway);
    assert_eq!(dv.get(), 13);
    // without bounds the storage is the range: any u8 of ms is a u32 of µs
    let ms: Fixed<Second, Milli, u8> = Fixed::new(255);
    let us: Fixed<Second, Micro, u32> = ms.into_rounded(Rounding::Truncate);
    assert_eq!(us.get(), 255_000);
}

exact_units::scale! {
    /// Steps of 3·2⁻⁶³: into kilo the factor's denominator is 2⁶⁶·125, into one it is 2⁶³, both past i64.
    struct Tiny = "", Exact(3 / 1, pow2 -63);
}

exact_units::scale! {
    /// Steps of 10⁻¹⁹·2⁻⁶³/3: into one the factor's denominator is past i128.
    struct Tinier = "", Exact(1 / 3, pow10 -19, pow2 -63);
}

#[test]
fn denominators_past_i64() {
    // an i32 takes the 64-bit path, which can't divide by these: the 128-bit one does
    let zero = Fixed::<Radian, Tiny, i32>::new(0);
    assert_eq!(zero.to::<Kilo, i32>().map(Fixed::get), Some(0));
    assert_eq!(zero.to::<One, i32>().map(Fixed::get), Some(0));
    assert_eq!(Fixed::<Radian, Tiny, i32>::new(1).to::<One, i32>(), None);
    for (x, r, want) in [
        (1, Rounding::Ceil, 1),
        (1, Rounding::Floor, 0),
        (-1, Rounding::Ceil, 0),
        (-1, Rounding::Floor, -1),
        (i32::MAX, Rounding::NearestEven, 0),
    ] {
        let got = Fixed::<Radian, Tiny, i32>::new(x).to_rounded::<One, i32>(r);
        assert_eq!(got.map(Fixed::get), Some(want), "{x} {r:?}");
    }
    // past i128 only 0 is whole
    assert_eq!(
        Fixed::<Radian, Tinier, i64>::new(0)
            .to::<One, i64>()
            .map(Fixed::get),
        Some(0)
    );
    assert_eq!(Fixed::<Radian, Tinier, i64>::new(1).to::<One, i64>(), None);
}
