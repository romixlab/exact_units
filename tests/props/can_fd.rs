//! Property: a set bound ([OneOf]) and the CAN FD data length code ([CanFdDlc]), the first `Table` codec, against
//! the lengths written out here from ST RM0468 Rev 3, §61.4.1, Table 503 "DLC coding in FDCAN" (ISO 11898-1:2015's
//! coding; the same table as Linux's `can_fd_dlc2len`). A `u8` length is taken by `try_new` exactly when it is one of the set, refused with the set
//! named otherwise, and rounded into the set by every [Rounding] as a brute-force reference picks; every byte as a
//! DLC decodes to the reference's length (codes 16..=255 are spare), the data path (`CodecDef`) decodes and encodes
//! as the typed one does, and every length and every rounding encodes to the code the reference finds. 256 values
//! and 5 roundings: walked whole.

use exact_units::codec::{CanFdDlc, CanFdLengths, Codec, Coded};
use exact_units::prefix::One;
use exact_units::units::Byte;
use exact_units::{Fixed, OneOf, Rounding, ValueSet};

/// A CAN FD payload length.
type Len = Fixed<Byte, One, u8, OneOf<CanFdLengths>>;

/// The length of each DLC 0..=15 of an FD frame, written out from RM0468's Table 503.
const DLC_LENGTHS: [i128; 16] = [0, 1, 2, 3, 4, 5, 6, 7, 8, 12, 16, 20, 24, 32, 48, 64];

/// Every rounding.
const ROUNDINGS: [Rounding; 5] = [
    Rounding::Truncate,
    Rounding::Floor,
    Rounding::Ceil,
    Rounding::NearestEven,
    Rounding::NearestAway,
];

/// The DLC whose length is `x`, if one is.
fn dlc_of(x: i128) -> Option<usize> {
    DLC_LENGTHS.iter().position(|&l| l == x)
}

/// The DLC `x` rounds to by `r`, worked out by brute force over the 16 lengths: the exact one, else the largest
/// below (`Floor`, `Truncate` of a positive `x`; of a negative one the smallest above), the smallest above (`Ceil`), or the closest, a tie to the even DLC
/// (`NearestEven`) or to the larger length (`NearestAway`: every length is positive).
fn dlc_rounded(x: i128, r: Rounding) -> Option<usize> {
    if let Some(d) = dlc_of(x) {
        return Some(d);
    }
    let below = (0..16)
        .rev()
        .find(|&d| DLC_LENGTHS.get(d).is_some_and(|&l| l < x));
    let above = (0..16).find(|&d| DLC_LENGTHS.get(d).is_some_and(|&l| l > x));
    let len = |d: usize| DLC_LENGTHS.get(d).copied().unwrap_or_default();
    match r {
        Rounding::Floor => below,
        Rounding::Ceil => above,
        Rounding::Truncate => match x.cmp(&0) {
            core::cmp::Ordering::Greater => below,
            core::cmp::Ordering::Less => above,
            core::cmp::Ordering::Equal => None,
        },
        Rounding::NearestEven | Rounding::NearestAway => match (below, above) {
            (Some(b), Some(a)) => {
                let (db, da) = (x.abs_diff(len(b)), len(a).abs_diff(x));
                Some(if db < da {
                    b
                } else if da < db {
                    a
                } else if r == Rounding::NearestEven {
                    if b % 2 == 0 { b } else { a }
                } else {
                    a
                })
            }
            (b, a) => b.or(a),
        },
    }
}

/// The set is the table's lengths, in order: one list, the standard's.
pub(crate) fn set_is_the_table() {
    let set: Vec<i128> = CanFdLengths::VALUES
        .iter()
        .map(|&v| i128::from(v))
        .collect();
    assert_eq!(set, DLC_LENGTHS);
}

/// `x` is taken exactly when it is one of the lengths, and a refusal names the set.
pub(crate) fn length_is_taken_only_from_the_set(x: u8) {
    let ok = dlc_of(i128::from(x)).is_some();
    match Len::try_new(x) {
        Ok(len) => {
            assert!(ok, "{x} taken");
            assert_eq!(len.get(), x);
        }
        Err(e) => {
            assert!(!ok, "{x} refused");
            assert_eq!(e.value(), x);
            assert_eq!(
                e.to_string(),
                format!("{x} B is out of {{0..=8, 12, 16, 20, 24, 32, 48, 64}} B (Byte, u8)")
            );
        }
    }
}

/// `x` rounds into the set as the reference does, by every rounding.
pub(crate) fn length_rounds_into_the_set(x: u8, r: Rounding) {
    let got = Len::try_new_rounded(x, r).map(|l| i128::from(l.get()));
    let want = dlc_rounded(i128::from(x), r).and_then(|d| DLC_LENGTHS.get(d).copied());
    assert_eq!(got, want, "{x} by {r:?}");
}

/// Every byte as a DLC: 0..=15 decode to the reference's length by both paths, 16..=255 are spare.
pub(crate) fn dlc_decodes(raw: u8) {
    let want = DLC_LENGTHS.get(usize::from(raw)).copied();
    let typed = CanFdDlc::decode(raw).map(|l| i128::from(l.get()));
    assert_eq!(typed, want, "{raw}");
    assert_eq!(CanFdDlc::DEF.decode(i128::from(raw)), want, "{raw}");
    assert_eq!(
        CanFdDlc::DEF.is_spare(i128::from(raw)),
        want.is_none(),
        "{raw}"
    );
    match Coded::<CanFdDlc>::try_from_raw(raw) {
        Ok(c) => {
            assert_eq!(Some(i128::from(c.get().get())), want, "{raw}");
            assert_eq!(c.raw(), raw);
        }
        Err(e) => {
            assert!(want.is_none(), "{raw} refused");
            assert_eq!(e.raw(), raw);
        }
    }
}

/// `x` by `r`: the data path's code is the reference's, and the typed path (`x` rounded into the set by the same
/// rounding, then encoded) gives the same code. A length of the set encodes to its own DLC whatever the rounding.
pub(crate) fn dlc_encodes(x: u8, r: Rounding) {
    let want = dlc_rounded(i128::from(x), r).map(|d| i128::try_from(d).unwrap_or(i128::MAX));
    assert_eq!(CanFdDlc::DEF.encode(i128::from(x), r), want, "{x} by {r:?}");
    let typed = Len::try_new_rounded(x, r)
        .and_then(|l| Coded::<CanFdDlc>::try_new(l, r).ok())
        .map(|c| i128::from(c.raw()));
    assert_eq!(typed, want, "{x} by {r:?}");
    if let Ok(len) = Len::try_new(x) {
        // the typed encode is a position in u8, the data path's a table walk in i128: the same code
        let exact = dlc_of(i128::from(x)).map(|d| i128::try_from(d).unwrap_or(i128::MAX));
        let typed = CanFdDlc::encode(len, r).map(i128::from);
        assert_eq!(typed, exact, "{x} by {r:?}");
        assert_eq!(
            typed,
            CanFdDlc::DEF.encode(i128::from(x), r),
            "{x} by {r:?}"
        );
    }
}

/// A signed set, its ends at `i16`'s: the binary search of `OneOf` off the CAN case, and gaps wider than `i16`
/// holds (−32768 to −300), so a rounding that took their difference in `i16` would overflow.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
pub(crate) struct Signed;

impl ValueSet<i16> for Signed {
    const VALUES: &'static [i16] = &[-32768, -300, -1, 0, 7, 8, 1000, 32767];
}

/// The value of `set` `x` rounds to by `r`, by brute force in `i128`: the exact one, else the largest below or the
/// smallest above as `r` says, the nearer of the two, a tie to the even position (`NearestEven`) or the value
/// farther from zero (`NearestAway`; the one above where both are as far).
fn set_rounded(set: &[i128], x: i128, r: Rounding) -> Option<i128> {
    if set.contains(&x) {
        return Some(x);
    }
    let below = set.iter().rposition(|&v| v < x);
    let above = set.iter().position(|&v| v > x);
    let at = |i: usize| set.get(i).copied().unwrap_or_default();
    let pick = match r {
        Rounding::Floor => below,
        Rounding::Ceil => above,
        Rounding::Truncate => match x.cmp(&0) {
            core::cmp::Ordering::Greater => below,
            core::cmp::Ordering::Less => above,
            core::cmp::Ordering::Equal => None,
        },
        Rounding::NearestEven | Rounding::NearestAway => match (below, above) {
            (Some(b), Some(a)) => {
                let (db, da) = (x.abs_diff(at(b)), at(a).abs_diff(x));
                Some(if db < da {
                    b
                } else if da < db {
                    a
                } else if r == Rounding::NearestEven {
                    if b % 2 == 0 { b } else { a }
                } else if at(b).unsigned_abs() > at(a).unsigned_abs() {
                    b
                } else {
                    a
                })
            }
            (b, a) => b.or(a),
        },
    };
    pick.map(at)
}

/// `x` is taken by a `OneOf<Signed>` exactly when the list holds it, and rounds into it by every rounding as the
/// reference does.
pub(crate) fn signed_set_contains(x: i16) {
    type S = Fixed<exact_units::units::Ratio, One, i16, OneOf<Signed>>;
    let listed = Signed::VALUES.contains(&x);
    assert_eq!(S::try_new(x).is_ok(), listed, "{x}");
    let set: Vec<i128> = Signed::VALUES.iter().map(|&v| i128::from(v)).collect();
    for r in ROUNDINGS {
        assert_eq!(
            S::try_new_rounded(x, r).map(|v| i128::from(v.get())),
            set_rounded(&set, i128::from(x), r),
            "{x} by {r:?}"
        );
    }
}

#[cfg(not(any(kani, fuzzing)))]
mod tests {
    use super::{Len, ROUNDINGS};

    #[test]
    fn every_length_and_dlc() {
        super::set_is_the_table();
        for x in 0..=u8::MAX {
            super::length_is_taken_only_from_the_set(x);
            super::dlc_decodes(x);
            for r in ROUNDINGS {
                super::length_rounds_into_the_set(x, r);
                super::dlc_encodes(x, r);
            }
        }
    }

    #[test]
    fn every_i16_against_a_signed_set() {
        for x in i16::MIN..=i16::MAX {
            super::signed_set_contains(x);
        }
    }

    /// The data path beyond a `u8`: below 0 and above 64 only the side with a length rounds.
    #[test]
    fn data_path_past_the_lengths() {
        use super::{CanFdDlc, Codec, Rounding};
        assert_eq!(CanFdDlc::DEF.encode(-1, Rounding::Ceil), Some(0));
        assert_eq!(CanFdDlc::DEF.encode(-1, Rounding::Floor), None);
        assert_eq!(CanFdDlc::DEF.encode(-1, Rounding::Truncate), Some(0)); // toward zero
        assert_eq!(CanFdDlc::DEF.encode(1000, Rounding::NearestEven), Some(15));
        assert_eq!(CanFdDlc::DEF.encode(1000, Rounding::Ceil), None);
        assert_eq!(CanFdDlc::DEF.decode(-1), None);
        assert_eq!(CanFdDlc::DEF.decode(16), None);
    }

    /// A window around a length ends on lengths: ±25 % of 16 is 12..=20, ±10 % of 16 only 16 (12 and 20 are out).
    #[cfg(feature = "dimensionless")]
    #[test]
    fn a_window_ends_on_lengths() {
        use exact_units::{Exact, Fit, Fraction, Tolerance};
        let at = |p, q, fit| {
            let tol = Tolerance::of(Exact::of(Fraction::new(p, q)));
            let w = Len::bounded(16).window(tol, fit);
            (w.lo().get(), w.hi().get())
        };
        assert_eq!(at(1, 4, Fit::Inside), (12, 20));
        assert_eq!(at(1, 10, Fit::Inside), (16, 16));
        // 14.4 ..= 17.6 cut outward is 14 ..= 18, which ends on no length: inward to 16
        assert_eq!(at(1, 10, Fit::Outside), (16, 16));
        assert_eq!(at(3, 10, Fit::Outside), (12, 20)); // 11 ..= 21
        assert_eq!(at(1, 2, Fit::Inside), (8, 24));
    }

    /// `Debug` of a set-bounded value and of its refusal name the set.
    #[test]
    fn debug_names_the_set() {
        assert_eq!(
            format!("{:?}", Len::bounded(12)),
            "Byte(12 B u8, {0..=8, 12, 16, 20, 24, 32, 48, 64})"
        );
        assert_eq!(
            format!("{:?}", Len::try_new(13).unwrap_err()),
            "OutOfRange(13, {0..=8, 12, 16, 20, 24, 32, 48, 64})"
        );
        assert_eq!(
            Len::bounded(8).checked_add(Len::bounded(4)),
            Some(Len::bounded(12))
        );
        assert_eq!(Len::bounded(8).checked_add(Len::bounded(1)), None);
    }

    /// The wire refuses a length off the set: a gap value as no value, one past the ends as out of the range.
    #[cfg(all(feature = "shrink_wrap", feature = "std"))]
    #[test]
    fn the_wire_refuses_a_length_off_the_set() {
        use super::{CanFdDlc, Coded};
        use shrink_wrap::prelude::*;
        use shrink_wrap::refused::{Refused, RefusedNum, Rule};

        let refused = |b: u8| {
            <Len as DeserializeShrinkWrapOwned>::from_ww_bytes_explained(&[b])
                .err()
                .and_then(|e| e.refused)
        };
        assert_eq!(
            <Len as DeserializeShrinkWrapOwned>::from_ww_bytes_owned(&[12]).ok(),
            Some(Len::bounded(12))
        );
        assert_eq!(
            refused(13),
            Some(Refused::new(RefusedNum::Int(13), Rule::NoValue))
        );
        assert_eq!(
            refused(65),
            Some(Refused::out_of(
                RefusedNum::Int(65),
                RefusedNum::Int(0),
                RefusedNum::Int(64)
            ))
        );
        let dlc = |b: u8| {
            <Coded<CanFdDlc> as DeserializeShrinkWrapOwned>::from_ww_bytes_explained(&[b])
                .map_err(|e| e.refused)
        };
        assert_eq!(dlc(9).map(|c| c.get().get()), Ok(12));
        assert_eq!(
            dlc(16),
            Err(Some(Refused::new(RefusedNum::Int(16), Rule::NoValue)))
        );
    }

    /// serde reads a length of the set and refuses the rest, naming the set.
    #[cfg(feature = "serde")]
    #[test]
    fn serde_refuses_a_length_off_the_set() {
        assert_eq!(
            serde_json::from_str::<Len>("48").ok(),
            Some(Len::bounded(48))
        );
        let e = serde_json::from_str::<Len>("13")
            .err()
            .map(|e| e.to_string());
        assert_eq!(
            e.as_deref(),
            Some("13 B is out of {0..=8, 12, 16, 20, 24, 32, 48, 64} B (Byte, u8)")
        );
    }
}

#[cfg(kani)]
mod proofs {
    /// Every byte decodes as a DLC by both paths, 16..=255 refused as spare.
    #[kani::proof]
    fn length_and_dlc() {
        let x: u8 = kani::any();
        super::dlc_decodes(x);
    }
}
