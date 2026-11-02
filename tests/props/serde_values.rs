//! Property 11: serde of the numbers (feature `serde`). A value reads back as itself through JSON and postcard, and
//! a stored number reads exactly when construction takes it, else it is an error; the bytes are the plain number's.
//! The `with_unit` text reads back to the value it was written from, a decimal with a unit reads exactly as the
//! reference works it out in `i128`, and no text and no bytes at all panic. The text properties have no Kani proof
//! (strings); the bytes of a bounded `i16` have one.
#![expect(
    clippy::arithmetic_side_effects,
    reason = "a test computes with the values it chose, which fit; an overflow fails the test"
)]
#![expect(clippy::unwrap_used, reason = "a property fails by panicking")]
#![expect(clippy::panic, reason = "a property fails by panicking")]
#![expect(
    clippy::indexing_slicing,
    reason = "the bytes a buffer holds and the prefixes of a table the test sized; a wrong index fails the test"
)]

use core::fmt::Debug;
use exact_units::decibel::Decibels;
use exact_units::prefix::{Kibi, Milli, Pow2, Rational};
use exact_units::units::{Byte, Ratio, Second, Volt};
use exact_units::{
    Bounds, Complex, Fixed, FixedScale, Range, Saturating, Storage, Unbounded, Unit,
};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

// steps of π/2¹⁵ rad: a scale that isn't a fraction of integers, so `{:#}` writes a float
exact_units::scale! { struct PiQ15 = "", Exact(1 / 32768, pi 1); }

/// A bounded reading, the bounds away from the ends of `i16`.
pub(crate) type Reading = Fixed<Volt, Milli, i16, Range<-1000, 3300>>;

/// `x` written to postcard, `None` where it doesn't fit the buffer.
fn postcard<T: Serialize>(x: &T) -> Option<([u8; 32], usize)> {
    let mut buf = [0u8; 32];
    let n = postcard::to_slice(x, &mut buf).ok()?.len();
    Some((buf, n))
}

/// `raw` read as `F` from postcard (and, but under Kani, from JSON): a value exactly where `F::try_new` takes it,
/// written back to the same bytes as the plain number.
pub(crate) fn reads_iff_in_bounds<U: Unit, P: FixedScale, T, B: Bounds<T>>(raw: T)
where
    T: Storage + Serialize + DeserializeOwned + PartialEq + Debug,
{
    let want = Fixed::<U, P, T, B>::try_new(raw).ok();
    let (bytes, n) = postcard(&raw).unwrap();
    let got = postcard::from_bytes::<Fixed<U, P, T, B>>(&bytes[..n]).ok();
    assert_eq!(got.map(Fixed::get), want.map(Fixed::get), "{raw:?}");
    if let Some(v) = got {
        let (again, m) = postcard(&v).unwrap();
        assert_eq!(&again[..m], &bytes[..n]);
    }
    #[cfg(not(kani))]
    {
        let json = serde_json::to_string(&raw).unwrap();
        let got = serde_json::from_str::<Fixed<U, P, T, B>>(&json).ok();
        assert_eq!(got.map(Fixed::get), want.map(Fixed::get), "{json}");
        if let Some(v) = got {
            assert_eq!(serde_json::to_string(&v).unwrap(), json);
        }
    }
}

/// Any bytes read as a [Reading]: an error, or a value in its bounds that writes back to the bytes it was read from
/// (the first ones: postcard reads what it needs).
pub(crate) fn any_bytes(bytes: &[u8]) {
    if let Ok((v, rest)) = postcard::take_from_bytes::<Reading>(bytes) {
        assert!((-1000..=3300).contains(&v.get()), "{bytes:?}");
        let (again, n) = postcard(&v).unwrap();
        // a canonical varint: the bytes it took, unless they were an overlong form postcard also reads
        let used = bytes.len() - rest.len();
        assert!(n <= used, "{bytes:?}");
        assert_eq!(postcard::from_bytes::<Reading>(&again[..n]).unwrap(), v);
    }
}

/// Any bytes read as each serde type of this crate: an error or a value, never a panic, and a value writes and reads
/// back as itself.
#[cfg(not(kani))]
pub(crate) fn any_bytes_all(bytes: &[u8]) {
    fn check<T: Serialize + DeserializeOwned + PartialEq + Debug>(bytes: &[u8]) {
        if let Ok(v) = postcard::from_bytes::<T>(bytes) {
            let mut buf = [0u8; 64];
            let again = postcard::to_slice(&v, &mut buf).unwrap();
            assert_eq!(postcard::from_bytes::<T>(again).unwrap(), v, "{bytes:?}");
        }
    }
    any_bytes(bytes);
    check::<Fixed<Second, Milli, u32, Range<1, 5000>>>(bytes);
    check::<Fixed<Volt, Milli, f32, exact_units::RangeDec<-5, 25, -1>>>(bytes);
    check::<Complex<Volt, Pow2<-15>, i16, Saturating>>(bytes);
    check::<Decibels>(bytes);
    check::<super::niche::Word>(bytes);
    check::<super::niche::Rssi>(bytes);
    check::<exact_units::IndexFrom1<exact_units::units::Item, u16, Range<1, 300>>>(bytes);
    check::<exact_units::Index<exact_units::units::Item, u8, Range<0, 9>>>(bytes);
    check::<exact_units::Seq<exact_units::units::Item, u16>>(bytes);
}

/// A [Fixed] in the `with_unit` text form.
#[cfg(not(kani))]
#[derive(Serialize, Deserialize, Debug, PartialEq)]
#[serde(transparent)]
struct Text<U: Unit, P: FixedScale, T: Storage, B: Bounds<T>>(
    #[serde(with = "exact_units::serde::with_unit")] Fixed<U, P, T, B>,
);

/// `s` read with `with_unit`.
#[cfg(not(kani))]
fn read<U: Unit, P: FixedScale, T: Storage, B: Bounds<T>>(s: &str) -> Option<Fixed<U, P, T, B>> {
    serde_json::from_value::<Text<U, P, T, B>>(serde_json::Value::String(s.into()))
        .ok()
        .map(|t| t.0)
}

/// `v` as `with_unit` writes it.
#[cfg(not(kani))]
fn write<U: Unit, P: FixedScale, T: Storage, B: Bounds<T>>(v: Fixed<U, P, T, B>) -> String {
    match serde_json::to_value(Text(v)).unwrap() {
        serde_json::Value::String(s) => s,
        v => panic!("not text: {v}"),
    }
}

/// `v` written with `with_unit` reads back as `v` (bit for bit for a float).
#[cfg(not(kani))]
pub(crate) fn text_round_trip<U: Unit, P: FixedScale, T: Storage + Debug, B: Bounds<T>>(
    v: Fixed<U, P, T, B>,
) {
    let s = write(v);
    let back =
        read::<U, P, T, B>(&s).unwrap_or_else(|| panic!("{v:?} wrote {s:?}, which doesn't read"));
    assert!(
        same(back.get(), v.get()),
        "{v:?} wrote {s:?}, read {back:?}"
    );
}

/// Equal, or both NaN.
#[cfg(not(kani))]
fn same<T: Storage>(a: T, b: T) -> bool {
    match (a.to_repr(), b.to_repr()) {
        (exact_units::Repr::Float(a), exact_units::Repr::Float(b)) => {
            a.to_bits() == b.to_bits() || (a.is_nan() && b.is_nan())
        }
        _ => a == b,
    }
}

/// The text `"{m}e{e} {prefix}V"` read into mV of an `i32`: exactly `m · 10^(e + k + 3)` mV where that is a whole
/// number that fits `i32`, else an error. The reference in `i128`, without the crate's helpers.
#[cfg(not(kani))]
pub(crate) fn text_reads_exactly(m: i64, e: i8, prefix: usize) {
    let (sym, k) = [
        ("", 0),
        ("m", -3),
        ("µ", -6),
        ("u", -6),
        ("k", 3),
        ("M", 6),
        ("n", -9),
    ][prefix];
    let s = format!("{m}e{e} {sym}V");
    let exp = i32::from(e) + k + 3;
    let want = if m == 0 {
        // 0 · 10^anything is exactly 0, also where 10^exp is past i128
        Some(0)
    } else if exp >= 0 {
        10i128
            .checked_pow(exp.unsigned_abs())
            .and_then(|t| i128::from(m).checked_mul(t))
    } else {
        let t = 10i128.checked_pow(exp.unsigned_abs());
        match t {
            Some(t) if i128::from(m) % t == 0 => Some(i128::from(m) / t),
            // 10^-exp past i128: no m but 0 (handled above) is whole
            Some(_) | None => None,
        }
    };
    let want = want.and_then(|w| i32::try_from(w).ok());
    let got = read::<Volt, Milli, i32, Unbounded>(&s).map(Fixed::get);
    assert_eq!(got, want, "{s}");
}

/// Any text read with `with_unit` into fields of several scales and storages, and as a level in dB: an error or a
/// value, never a panic; a value is in its bounds and writes text that reads back as itself.
#[cfg(not(kani))]
pub(crate) fn any_text(s: &str) {
    fn check<U: Unit, P: FixedScale, T: Storage + Debug, B: Bounds<T>>(s: &str) {
        if let Some(v) = read::<U, P, T, B>(s) {
            assert!(B::contains(v.get()), "{s:?}: {v:?}");
            text_round_trip(v);
        }
    }
    check::<Volt, Milli, u16, Range<0, 5000>>(s);
    check::<Second, Milli, i64, Unbounded>(s);
    check::<Ratio, Pow2<-15>, i16, Saturating>(s);
    check::<Ratio, Rational<1, 65535>, u16, Unbounded>(s);
    check::<Ratio, Milli, i32, Unbounded>(s);
    check::<Byte, Kibi, u32, Unbounded>(s);
    check::<Volt, Milli, f32, Unbounded>(s);
    check::<exact_units::units::Radian, PiQ15, i16, Unbounded>(s);
    if let Ok(d) = serde_json::from_value::<Decibels>(serde_json::Value::String(s.into())) {
        let again = serde_json::to_string(&d).unwrap();
        assert_eq!(
            serde_json::from_str::<Decibels>(&again).unwrap(),
            d,
            "{s:?}"
        );
    }
}

#[cfg(not(kani))]
mod tests {
    use super::Reading;
    use crate::strategy;
    use exact_units::prefix::{Micro, Milli, Nano, Pow2};
    use exact_units::units::{Ratio, Second, Turn, Volt};
    use exact_units::{Fixed, Range, RangeU64, Unbounded, Wrapping};
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn i32_reads_iff_in_bounds(raw in strategy::i32()) {
            super::reads_iff_in_bounds::<Volt, Milli, i32, Range<-100_000, 100_000>>(raw);
        }

        #[test]
        fn i64_reads_iff_in_bounds(raw in strategy::i64()) {
            super::reads_iff_in_bounds::<Second, Nano, i64, Range<{ i64::MIN + 1 }, 0>>(raw);
        }

        #[test]
        fn u64_reads_iff_in_bounds(raw: u64) {
            super::reads_iff_in_bounds::<Second, Nano, u64, RangeU64<1, { u64::MAX - 1 }>>(raw);
        }

        #[test]
        fn f64_reads_iff_in_bounds(raw: f64) {
            super::reads_iff_in_bounds::<Volt, Milli, f64, exact_units::RangeDec<-5, 25, -1>>(raw);
        }

        #[test]
        fn any_bytes(bytes in proptest::collection::vec(any::<u8>(), 0..12)) {
            super::any_bytes_all(&bytes);
        }

        #[test]
        fn text_round_trip_i64_micro(v in strategy::i64()) {
            super::text_round_trip(Fixed::<Volt, Micro, i64>::new(v));
        }

        #[test]
        fn text_round_trip_q31(v in strategy::i32()) {
            super::text_round_trip(exact_units::Q31::<Ratio>::of(v));
        }

        #[test]
        fn text_round_trip_bam32(v: u32) {
            super::text_round_trip(Fixed::<Turn, Pow2<-32>, u32, Wrapping>::of(v));
        }

        #[test]
        fn text_round_trip_f32(v: f32) {
            super::text_round_trip(Fixed::<Volt, Milli, f32>::new(v));
        }

        #[test]
        fn text_round_trip_f64(v: f64) {
            super::text_round_trip(Fixed::<Second, exact_units::prefix::One, f64>::new(v));
        }

        #[test]
        fn text_reads_exactly(m in strategy::i64(), e in -40..=40i8, prefix in 0..7usize) {
            super::text_reads_exactly(m, e, prefix);
        }

        /// Numbers with units and prefixes in every shape, and anything at all.
        #[test]
        fn any_text(s in r"[ +-]?[0-9_.]{0,24}([eE][+-]?[0-9]{0,4})? ?(m|µ|u|k|M|Ki|da|%|‰|ppm|ppb)?(V|s|B|rad|dB)? ?|\PC{0,24}") {
            super::any_text(&s);
        }
    }

    /// A zero mantissa reads as 0 whatever its exponent, also where 10^exp is past i128 (`0e32 MV` failed the
    /// reference once, the parser was right).
    #[test]
    fn zero_with_any_exponent() {
        for prefix in 0..7 {
            for e in [-40, -32, -30, -1, 0, 1, 30, 32, 40] {
                super::text_reads_exactly(0, e, prefix);
            }
        }
    }

    /// Every i16 of a bounded reading: read exactly where it is in bounds.
    #[test]
    fn every_i16_reads_iff_in_bounds() {
        for raw in i16::MIN..=i16::MAX {
            super::reads_iff_in_bounds::<Volt, Milli, i16, Range<-1000, 3300>>(raw);
        }
    }

    /// Every pair of bytes, and every byte before the varints of an i16's ends.
    #[test]
    fn every_two_bytes() {
        for a in 0..=u8::MAX {
            for b in 0..=u8::MAX {
                super::any_bytes(&[a, b]);
                super::any_bytes(&[a, b, 0x01]);
                super::any_bytes(&[0xff, a, b]);
            }
        }
        let _ = Reading::try_new(0);
    }

    /// Every Q15 and every value of a scale with π: the text reads back as the value.
    #[test]
    fn every_i16_text_round_trips() {
        for v in i16::MIN..=i16::MAX {
            super::text_round_trip(Fixed::<Volt, Milli, i16>::new(v));
            super::text_round_trip(Fixed::<exact_units::units::Radian, super::PiQ15, i16>::new(
                v,
            ));
        }
    }

    /// The ends of u64 in ns: twenty digits.
    #[test]
    fn u64_ends_text_round_trip() {
        for v in [0, 1, u64::MAX - 1, u64::MAX] {
            super::text_round_trip(Fixed::<Second, Nano, u64, Unbounded>::new(v));
        }
    }
}

#[cfg(kani)]
mod proofs {
    use exact_units::prefix::Milli;
    use exact_units::units::Volt;
    use exact_units::{Range, Unbounded};

    /// Every i16: a bounded reading reads from postcard exactly where it is in bounds, and writes the same bytes.
    #[kani::proof]
    #[kani::unwind(4)]
    fn i16_reads_iff_in_bounds() {
        super::reads_iff_in_bounds::<Volt, Milli, i16, Range<-1000, 3300>>(kani::any());
    }

    /// Every three bytes read as a bounded reading: an error or a value in bounds, never a panic.
    #[kani::proof]
    #[kani::unwind(5)]
    fn any_three_bytes() {
        let bytes: [u8; 3] = kani::any();
        super::any_bytes(&bytes);
        let _ = postcard::from_bytes::<exact_units::Fixed<Volt, Milli, i16, Unbounded>>(&bytes);
    }
}
