//! Property 21: a moment across the leap-second table, POSIX time to TAI and back, is the reference's
//! (`reference::posix_to_tai`, `tai_to_posix`, from IANA's `leap-seconds.list` of the same bulletin): the same moment
//! where the table covers it, `InLeap` inside a leap second, `Stale` from its expiry on, `BeforeTable` before 1972,
//! and `NotExact` where the result isn't a whole number of the target's steps. Every leap second is walked whole in
//! ms around its edges. Not a Kani proof: its products are of 64-bit moments.
#![cfg(not(kani))]
#![expect(
    clippy::as_conversions,
    clippy::cast_possible_truncation,
    reason = "a test casts the moments of the table, which are about 2^32 s and fit i64"
)]
#![expect(
    clippy::arithmetic_side_effects,
    reason = "a test computes with the values it chose, which fit; an overflow fails the test"
)]

use super::reference::{LEAPS_NTP, Leap, posix_to_tai, tai_to_posix};
use exact_units::leap::{AcrossError, BULLETIN_C_72};
use exact_units::origins::{
    GpsEpoch, GpsMillis, TaiEpoch1958, TaiMillis, TaiNanos, UnixEpoch, UnixMillis, UnixNanos,
};
use exact_units::prefix::{Milli, Nano, One};

/// The answer `across` gives as the reference writes it; `NotExact` as `At(i128::MIN)`, which no reference gives for a
/// moment that is whole ms on both scales.
fn leap<T: Into<i128>>(r: Result<T, AcrossError>) -> Leap {
    match r {
        Ok(v) => Leap::At(v.into()),
        Err(AcrossError::Stale) => Leap::Stale,
        Err(AcrossError::InLeap) => Leap::Inside,
        Err(AcrossError::BeforeTable) => Leap::Before,
        Err(AcrossError::NotExact) => Leap::At(i128::MIN),
    }
}

/// POSIX time in ms to TAI in ms, the reference's, and back to the same moment where it converts.
pub(crate) fn posix_ms(p: i64) {
    let got = UnixMillis::<i64>::from_raw(p).across::<TaiEpoch1958, Milli, i64, _>(&BULLETIN_C_72);
    let want = posix_to_tai(i128::from(p), 1000);
    assert_eq!(leap(got.map(exact_units::Point::raw)), want, "{p} ms POSIX");
    if let Ok(t) = got {
        assert_eq!(
            t.across::<UnixEpoch, Milli, i64, _>(&BULLETIN_C_72),
            Ok(UnixMillis::from_raw(p)),
            "{p} ms POSIX and back"
        );
    }
}

/// TAI in ms to POSIX time in ms, the reference's, and back to the same moment where it converts.
pub(crate) fn tai_ms(t: i64) {
    let got = TaiMillis::<i64>::from_raw(t).across::<UnixEpoch, Milli, i64, _>(&BULLETIN_C_72);
    let want = tai_to_posix(i128::from(t), 1000);
    assert_eq!(leap(got.map(exact_units::Point::raw)), want, "{t} ms TAI");
    if let Ok(p) = got {
        assert_eq!(
            p.across::<TaiEpoch1958, Milli, i64, _>(&BULLETIN_C_72),
            Ok(TaiMillis::from_raw(t)),
            "{t} ms TAI and back"
        );
    }
}

/// GPS time in ms to POSIX time in ns: GPS is TAI − 19 s from 1980-01-06 00:00:19 TAI.
pub(crate) fn gps_ms(g: u64) {
    let got = GpsMillis::<u64>::from_raw(g).across::<UnixEpoch, Nano, u64, _>(&BULLETIN_C_72);
    let tai_ms = i128::from(g) + (8040 * 86_400 + 19) * 1000;
    let want = match tai_to_posix(tai_ms, 1000) {
        Leap::At(p) => Leap::At(p * 1_000_000),
        other => other,
    };
    assert_eq!(leap(got.map(exact_units::Point::raw)), want, "{g} ms GPS");
}

/// A moment that isn't whole seconds of the target is refused, never rounded.
pub(crate) fn not_exact(p_ms: i64) {
    let got = UnixMillis::<i64>::from_raw(p_ms)
        .across::<TaiEpoch1958, One, i64, _>(&BULLETIN_C_72)
        .map(exact_units::Point::raw);
    match posix_to_tai(i128::from(p_ms), 1000) {
        Leap::At(t) if t % 1000 == 0 => {
            assert_eq!(got.map(i128::from), Ok(t / 1000), "{p_ms} ms to s");
        }
        Leap::At(_) => assert_eq!(got, Err(AcrossError::NotExact), "{p_ms} ms to s"),
        _ => assert!(got.is_err(), "{p_ms} ms to s"),
    }
}

/// Every ms within 3 s of each entry's edges, on both scales, and of the expiry.
#[test]
fn every_edge() {
    let (table, expires) = LEAPS_NTP;
    let to_posix = |ntp: i128| i64::try_from((ntp - 2_208_988_800) * 1000).unwrap();
    let tai_1970_ms = (12 * 365 + 3) * 86_400 * 1000;
    let mut edges: Vec<(i64, i64)> = table
        .iter()
        .map(|(from, delta)| {
            let p = to_posix(*from);
            (p, p + i64::try_from(delta * 1000 + tai_1970_ms).unwrap())
        })
        .collect();
    let e = to_posix(expires);
    edges.push((e, e + 37_000 + i64::try_from(tai_1970_ms).unwrap()));
    for (p, t) in edges {
        for d in -3000..=3000 {
            posix_ms(p + d);
            tai_ms(t + d);
            tai_ms(t - 1000 + d);
        }
    }
}

/// The ends of the table: before 1972, the first moment it covers, its expiry.
#[test]
fn ends() {
    let tai =
        |p: u64| UnixNanos::<u64>::from_raw(p).across::<TaiEpoch1958, Nano, u64, _>(&BULLETIN_C_72);
    assert_eq!(tai(63_071_999_999_999_999), Err(AcrossError::BeforeTable));
    assert_eq!(
        tai(63_072_000_000_000_000),
        Ok(TaiNanos::from_raw(
            (63_072_000 + 10 + 378_691_200) * 1_000_000_000
        ))
    );
    assert_eq!(
        tai(1_814_140_799_999_999_999).map(|t| t.raw() % 1_000_000_000),
        Ok(999_999_999)
    );
    assert_eq!(tai(1_814_140_800_000_000_000), Err(AcrossError::Stale));
    // a moment before the first leap second: GPS time 0 is 1980-01-06, 19 s behind TAI
    assert_eq!(
        GpsMillis::<u64>::from_raw(0).across::<UnixEpoch, Milli, u64, _>(&BULLETIN_C_72),
        Ok(UnixMillis::from_raw(315_964_800_000))
    );
    // a target that can't hold the moment
    assert_eq!(
        UnixMillis::<i64>::from_raw(1_700_000_000_000)
            .across::<GpsEpoch, Milli, u32, _>(&BULLETIN_C_72),
        Err(AcrossError::NotExact)
    );
    assert_eq!(BULLETIN_C_72.source(), "IERS Bulletin C 72 (2026-07-06)");
}

#[cfg(not(fuzzing))]
mod props {
    use super::*;
    use proptest::prelude::*;

    /// A moment of POSIX time in ms: anything from 1960 to 2100, or within 3 s of a leap second.
    fn moment() -> impl Strategy<Value = i64> {
        prop_oneof![
            3 => -315_619_200_000i64..4_102_444_800_000,
            2 => (proptest::sample::select(&LEAPS_NTP.0[..]), -3000i64..=3000)
                .prop_map(|((from, _), d)| (from - 2_208_988_800) as i64 * 1000 + d),
        ]
    }

    proptest! {
        #[test]
        fn posix_to_tai_and_back(p in moment()) {
            posix_ms(p);
        }

        #[test]
        fn tai_to_posix_and_back(p in moment()) {
            tai_ms(p + 378_691_200_000 + 37_000);
            tai_ms(p + 378_691_200_000);
        }

        #[test]
        fn gps_to_posix(g in 0u64..4_000_000_000_000) {
            gps_ms(g);
        }

        #[test]
        fn refuses_inexact(p in moment()) {
            not_exact(p);
        }
    }
}
