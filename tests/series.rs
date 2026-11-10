//! `Fixed` as a field of a `Series` item (SW-60): the columns of plain numbers.
#![cfg(all(feature = "std", feature = "quantities"))]
#![expect(
    clippy::as_conversions,
    reason = "a test casts the values it chose and the references it computes; the cast lints say why each fits"
)]

use shrink_wrap::prelude::*;
use shrink_wrap::series::{Series, SeriesOwned};

/// `bytes` are within the bound of a `Series` of `n` items of `T` (SW-62).
#[expect(clippy::expect_used, reason = "a test helper: no bound fails the test")]
fn within_bound<T: for<'i> shrink_wrap::series::SeriesColumns<'i>>(n: usize, bytes: &[u8]) {
    let max = Series::<T>::max_bytes(n).expect("fixed numbers have a bound");
    assert!(bytes.len() <= max, "{} bytes, bound {max}", bytes.len());
}

#[derive_shrink_wrap(borrowed, owned, series, derive(Clone, Copy, Debug, PartialEq))]
struct PlainSample {
    /// µs
    t_us: u32,
    /// mV
    v_mv: i16,
    /// %
    duty_pct: u8,
}

#[derive_shrink_wrap(borrowed, owned, series, derive(Clone, Copy, Debug, PartialEq))]
struct FixedSample {
    /// The same µs.
    t: exact_units::Micros<u32>,
    /// The same mV.
    v: exact_units::Fixed<exact_units::units::Volt, exact_units::prefix::Milli, i16>,
    /// The same %.
    duty: exact_units::Fixed<exact_units::units::Ratio, exact_units::prefix::Centi, u8>,
}

#[derive_shrink_wrap(borrowed, owned, series, derive(Clone, Copy, Debug, PartialEq))]
struct PlainT {
    /// The same µs since boot as [PointSample]'s.
    t_us: u32,
}

#[derive_shrink_wrap(borrowed, owned, series, derive(Clone, Copy, Debug, PartialEq))]
struct PointSample {
    /// A device clock point in µs since boot (PT-7): the same bytes as the plain `t_us` of [PlainT].
    t: exact_units::Point<exact_units::origins::DeviceBoot, exact_units::Micros<u32>>,
}

/// A `Series` of a [Point] field (PT-7) has the bytes of the same series of its plain number, and reads back the
/// same values.
#[test]
fn point_field_is_a_plain_column() {
    use exact_units::Point;
    let plain: Vec<PlainT> = (0..64u32)
        .map(|k| PlainT {
            t_us: 1_000_000 + k * 40,
        })
        .collect();
    let points: Vec<PointSample> = plain
        .iter()
        .map(|p| PointSample {
            t: Point::from_raw(p.t_us),
        })
        .collect();
    let plain_bytes = Series::new(&plain).to_ww_bytes_owned().unwrap();
    let point_bytes = Series::new(&points).to_ww_bytes_owned().unwrap();
    assert_eq!(point_bytes, plain_bytes);
    within_bound::<PointSample>(points.len(), &point_bytes);
    let back = SeriesOwned::<PointSample>::from_ww_bytes_owned(&point_bytes).unwrap();
    assert_eq!(back.iter().copied().collect::<Vec<_>>(), points);
}

/// A `Series` of `Fixed` fields has the bytes of the same series of plain numbers, and reads back the same values.
#[test]
fn fixed_fields_are_plain_columns() {
    use exact_units::Fixed;
    let plain: Vec<PlainSample> = (0..64u32)
        .map(|k| PlainSample {
            t_us: 1_000_000 + k * 40,
            v_mv: 3300 + (k % 3) as i16 - 1,
            duty_pct: (k % 101) as u8,
        })
        .collect();
    let fixed: Vec<FixedSample> = plain
        .iter()
        .map(|p| FixedSample {
            t: Fixed::new(p.t_us),
            v: Fixed::new(p.v_mv),
            duty: Fixed::new(p.duty_pct),
        })
        .collect();
    let plain_bytes = Series::new(&plain).to_ww_bytes_owned().unwrap();
    let fixed_bytes = Series::new(&fixed).to_ww_bytes_owned().unwrap();
    assert_eq!(fixed_bytes, plain_bytes);
    within_bound::<FixedSample>(fixed.len(), &fixed_bytes);
    let back = SeriesOwned::<FixedSample>::from_ww_bytes_owned(&fixed_bytes).unwrap();
    assert_eq!(back.iter().copied().collect::<Vec<_>>(), fixed);
}
