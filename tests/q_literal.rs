//! PROTOTYPE: `q!(100 m/s)` against stored values, each compare checked against an exact reference over every
//! stored value of the storage.
#![cfg(all(
    feature = "mechanical",
    feature = "electrical",
    feature = "time",
    feature = "angle"
))]
#![expect(
    clippy::arithmetic_side_effects,
    reason = "references in i32 of i16 values times 10"
)]

use core::cmp::Ordering;
use exact_units::prefix::{Centi, Deci, Micro, Milli, One};
use exact_units::units::{self, Degree, Hertz, Meter, Ohm, Per, Radian, Second, Speed, Volt};
use exact_units::{Fixed, fixed, q};

/// Every operator of `t` against a literal agrees with `ord`, the exact order of `t` against it, both ways round.
macro_rules! agrees {
    ($t:expr, $lit:expr, $ord:expr) => {{
        let (t, lit, ord): (_, _, Ordering) = ($t, $lit, $ord);
        assert_eq!(t.partial_cmp(&lit), Some(ord), "{t:?}");
        assert_eq!(t < lit, ord.is_lt());
        assert_eq!(t <= lit, ord.is_le());
        assert_eq!(t > lit, ord.is_gt());
        assert_eq!(t >= lit, ord.is_ge());
        assert_eq!(t == lit, ord.is_eq());
        assert_eq!(lit.partial_cmp(&t), Some(ord.reverse()));
        assert_eq!(lit < t, ord.is_gt());
        assert_eq!(lit > t, ord.is_lt());
        assert_eq!(lit == t, ord.is_eq());
    }};
}

#[test]
fn the_ask() {
    let speed = Fixed::<Speed, Centi, i16>::new(9_999);
    assert!(speed < q!(100 m/s));
    assert!(speed < q!(100 Speed));
    assert!(speed < q!(100 units::Speed));
    assert!(speed < q!(100 ::exact_units::units::Speed));
    assert!(speed < q!(100 Per<Meter, Second>));
    assert_eq!(speed, q!(99.99 m/s));
    assert!(speed > q!(359.9 km/h)); // 99.97.. m/s
    assert!(speed < q!(360 km/h)); // 100 m/s
    assert_eq!(Fixed::<Speed, Centi, i16>::new(10_000), q!(360 km/h));
}

/// 0.155 m/s is 15.5 steps of cm/s: between two stored values, equal to neither.
#[test]
fn inexact_literal_compares_exactly() {
    for t in i16::MIN..=i16::MAX {
        let speed = Fixed::<Speed, Centi, i16>::new(t);
        // t · 1/100 against 155/1000: 10 t against 155
        agrees!(speed, q!(0.155 m/s), (i32::from(t) * 10).cmp(&155));
        agrees!(speed, q!(-0.155 m/s), (i32::from(t) * 10).cmp(&-155));
        // an exact literal
        agrees!(speed, q!(0.15 m/s), i32::from(t).cmp(&15));
    }
}

/// A literal past the storage's range: every stored value is below (or above) it, the compare folds to a constant.
#[test]
fn literal_past_range() {
    for t in i8::MIN..=i8::MAX {
        let x = Fixed::<Meter, One, i8>::new(t);
        agrees!(x, q!(1000 m), Ordering::Less);
        agrees!(x, q!(-1000 m), Ordering::Greater);
        agrees!(x, q!(127.5 m), Ordering::Less);
        agrees!(x, q!(-128.5 m), Ordering::Greater);
        agrees!(x, q!(-128 m), t.cmp(&-128));
        agrees!(x, q!(1 km), Ordering::Less);
    }
    for t in u8::MIN..=u8::MAX {
        let x = Fixed::<Meter, One, u8>::new(t);
        agrees!(x, q!(-0.5 m), Ordering::Greater);
        agrees!(x, q!(0 m), t.cmp(&0));
        agrees!(x, q!(0.255 km), t.cmp(&255));
        agrees!(x, q!(256 m), Ordering::Less);
    }
}

#[test]
fn prefixed_symbols() {
    for t in i16::MIN..=i16::MAX {
        let v = Fixed::<Volt, Milli, i16>::new(t);
        agrees!(v, q!(3.3 V), t.cmp(&3300));
        agrees!(v, q!(3300 mV), t.cmp(&3300));
        agrees!(v, q!(1250.5 mV), (i32::from(t) * 10).cmp(&12505));
        agrees!(v, q!(1500 μV), (i32::from(t) * 10).cmp(&15));
        agrees!(v, q!(1500 uV), (i32::from(t) * 10).cmp(&15));
        let f = Fixed::<Hertz, Deci, i16>::new(t);
        agrees!(f, q!(1.5 kHz), t.cmp(&15000));
        agrees!(f, q!(0.0015 MHz), t.cmp(&15000));
        let r = Fixed::<Ohm, One, i16>::new(t);
        agrees!(r, q!(4.7 kΩ), t.cmp(&4700));
        let s = Fixed::<Second, Micro, i16>::new(t);
        agrees!(s, q!(1.5 ms), t.cmp(&1500));
        agrees!(s, q!(1500 μs), t.cmp(&1500));
        agrees!(s, q!(1500000 ns), t.cmp(&1500));
    }
}

/// Degrees against steps of a rational scale (0.1°) and of one with π in it (mrad): no rounding in either.
#[test]
fn degrees() {
    for t in i16::MIN..=i16::MAX {
        let a = Fixed::<Degree, Deci, i16>::new(t);
        agrees!(a, q!(90 deg), t.cmp(&900));
        agrees!(a, q!(90.05 deg), (i32::from(t) * 10).cmp(&9005));
        // 90° is 1570.796.. mrad
        let r = Fixed::<Radian, Milli, i16>::new(t);
        let ord = if t <= 1570 {
            Ordering::Less
        } else {
            Ordering::Greater
        };
        agrees!(r, q!(90 deg), ord);
        let ord = if t <= -1571 {
            Ordering::Less
        } else {
            Ordering::Greater
        };
        agrees!(r, q!(-90 deg), ord);
    }
}

/// Quotients and products of symbols, left to right.
#[test]
fn compound() {
    let jerk = Fixed::<Per<Per<Speed, Second>, Second>, One, i16>::new(3);
    assert!(jerk < q!(3.5 m/s/s/s));
    let v = Fixed::<Volt, Milli, i16>::new(500);
    // A·Ω has a volt's dimension and kind
    assert_eq!(v, q!(0.5 A*Ω));
    let typed: Fixed<Speed, Centi, i16> = fixed!(100);
    assert!(Fixed::<Speed, Centi, i16>::new(9_999) < typed);
}
