//! NaN-free floats: a float `Fixed` without [MaybeNan] never holds NaN. Every way in refuses it (`try_new`, the
//! wire, a conversion; text and serde in `tests/serde.rs`). Arithmetic never checks and never panics: `+`, `-`,
//! `*` and `/` of floats give a [MaybeNan] whose bits are the IEEE 754 result's, and a [MaybeNan] compares only
//! through `gt_or` and its kin, which are the IEEE compare where neither side is NaN and the given fallback where
//! one is. Checked against the plain float operation, on every pair of the special values (±0, the smallest
//! subnormal, ±1, the largest finite, ±∞, NaN) and on drawn floats; construction on every `f32` whose low 16 bits
//! are 0.

use exact_units::prefix::One;
use exact_units::units::{Ampere, Volt};
use exact_units::{Finite, Fixed, MaybeNan, OutOfBounds};

type V = Fixed<Volt, One, f32>;
type M = Fixed<Volt, One, f32, MaybeNan>;
type F = Fixed<Volt, One, f32, Finite>;

/// The values where IEEE 754 makes NaN: ±0, ±∞ and their neighbours.
pub(crate) const SPECIAL: [f32; 13] = [
    0.0,
    -0.0,
    f32::from_bits(1),
    -f32::from_bits(1),
    1.0,
    -1.0,
    f32::MAX,
    f32::MIN,
    f32::MIN_POSITIVE,
    f32::INFINITY,
    f32::NEG_INFINITY,
    f32::NAN,
    -f32::NAN,
];

/// `a` and `b` are the same float: the same bits, or both NaN.
fn same(a: f32, b: f32) -> bool {
    a.to_bits() == b.to_bits() || (a.is_nan() && b.is_nan())
}

/// `try_new(x)` refuses exactly NaN, and keeps the bits of every other float; `MaybeNan` keeps every float.
pub(crate) fn constructs(x: f32) {
    match V::try_new(x) {
        Ok(v) => {
            assert!(!x.is_nan());
            assert_eq!(v.get().to_bits(), x.to_bits());
        }
        Err(OutOfBounds) => assert!(x.is_nan(), "{x:e} refused"),
    }
    let m = M::of(x);
    assert!(same(m.get(), x));
    assert_eq!(
        m.not_nan().map(|v| v.get().to_bits()),
        (!x.is_nan()).then_some(x.to_bits())
    );
}

/// One operator's [MaybeNan] result against the IEEE one: the same bits, NaN for NaN.
fn agrees(op: &str, a: f32, b: f32, ieee: f32, got: M) {
    assert!(
        same(got.get(), ieee),
        "{a:e} {op} {b:e}: {:e}, IEEE {ieee:e}",
        got.get()
    );
}

/// `+`, `-`, `*` and `/` (by a plain float and by a value), their assign forms on a [MaybeNan] and `sum` of
/// NaN-free `a` and `b` against IEEE 754, never a panic; `checked_*` is `None` exactly where the result isn't
/// finite. Then the [MaybeNan] operators and compares on `a` and `b` as they are, NaN included.
#[expect(
    clippy::arithmetic_side_effects,
    reason = "IEEE 754 float operators, the operators under test"
)]
pub(crate) fn operates(a: f32, b: f32) {
    if let (Ok(x), Ok(y)) = (V::try_new(a), V::try_new(b)) {
        agrees("+", a, b, a + b, x + y);
        agrees("-", a, b, a - b, x - y);
        agrees("*", a, b, a * b, x * b);
        agrees("/", a, b, a / b, x / b);
        let i = Fixed::<Ampere, One, f32>::bounded(b);
        assert!(same((x * i).get(), a * b), "{a:e} V * {b:e} A");
        assert!(same((x / i).get(), a / b), "{a:e} V / {b:e} A");
        // a MaybeNan and a NaN-free value of its unit, either way round
        agrees("+", a, b, a + b, x.maybe_nan() + y);
        agrees("+", a, b, a + b, x + y.maybe_nan());
        let mut z = x.maybe_nan();
        z += y;
        agrees("+=", a, b, a + b, z);
        // a sum folds from +0, as the storage's own `0.0 + a + b`: −0 + −0 sums to +0
        agrees("sum", a, b, 0.0 + a + b, [x, y].into_iter().sum::<M>());
        for (op, ieee, got) in [
            ("+", a + b, x.checked_add(y)),
            ("-", a - b, x.checked_sub(y)),
            ("*", a * b, x.checked_mul(b)),
            ("/", a / b, x.checked_div(b)),
        ] {
            match got {
                Some(v) => assert!(
                    ieee.is_finite() && same(v.get(), ieee),
                    "checked {a:e} {op} {b:e}"
                ),
                None => assert!(
                    !ieee.is_finite(),
                    "checked {a:e} {op} {b:e} refused {ieee:e}"
                ),
            }
        }
    }
    maybe_nan(a, b);
}

/// `MaybeNan`'s operators are IEEE 754, NaN in and out; its compares against a NaN-free `b` are IEEE's where `a` is
/// a number and the fallback where it is NaN.
#[expect(clippy::arithmetic_side_effects, reason = "IEEE 754 float operators")]
#[expect(
    clippy::float_cmp,
    reason = "the IEEE 754 compare is the reference, exactly"
)]
fn maybe_nan(a: f32, b: f32) {
    let (x, y) = (M::of(a), M::of(b));
    agrees("+", a, b, a + b, x + y);
    agrees("-", a, b, a - b, x - y);
    agrees("*", a, b, a * b, x * b);
    agrees("/", a, b, a / b, x / b);
    let mut z = x;
    z -= y;
    agrees("-=", a, b, a - b, z);
    let Ok(v) = V::try_new(b) else { return };
    for nan in [false, true] {
        let or = |ieee: bool| if a.is_nan() { nan } else { ieee };
        assert_eq!(x.gt_or(v, nan), or(a > b), "{a:e} > {b:e}");
        assert_eq!(x.ge_or(v, nan), or(a >= b), "{a:e} >= {b:e}");
        assert_eq!(x.lt_or(v, nan), or(a < b), "{a:e} < {b:e}");
        assert_eq!(x.le_or(v, nan), or(a <= b), "{a:e} <= {b:e}");
        assert_eq!(x.eq_or(v, nan), or(a == b), "{a:e} == {b:e}");
    }
}

/// [Finite] holds exactly the finite floats, with their bits: `try_new` and `to_bounds` refuse ±∞ and NaN and
/// nothing else, and `unbounded` gives the same bits back. For `f64` too, through the same float widened.
pub(crate) fn finite_constructs(x: f32) {
    match F::try_new(x) {
        Ok(f) => {
            assert!(x.is_finite(), "{x:e} taken");
            assert_eq!(f.get().to_bits(), x.to_bits());
            assert_eq!(f.unbounded().get().to_bits(), x.to_bits());
        }
        Err(OutOfBounds) => assert!(!x.is_finite(), "{x:e} refused"),
    }
    assert_eq!(
        M::of(x)
            .not_nan()
            .and_then(|v| v.to_bounds::<Finite>().ok()),
        F::try_new(x).ok()
    );
    let w = f64::from(x);
    assert_eq!(
        Fixed::<Volt, One, f64, Finite>::try_new(w).is_ok(),
        w.is_finite()
    );
}

/// `checked_*` of two [Finite] values is the IEEE 754 result where that is finite and `None` where it is ±∞ or NaN.
pub(crate) fn finite_operates(a: f32, b: f32) {
    let (Ok(x), Ok(y)) = (F::try_new(a), F::try_new(b)) else {
        return;
    };
    let ok = |op: &str, ieee: f32, got: Option<F>| {
        assert_eq!(
            got.map(|v| v.get().to_bits()),
            ieee.is_finite().then_some(ieee.to_bits()),
            "{a:e} {op} {b:e}"
        );
    };
    ok("+", a + b, x.checked_add(y));
    ok("-", a - b, x.checked_sub(y));
    ok("*", a * b, x.checked_mul(b));
    ok("/", a / b, x.checked_div(b));
}

/// The wire: the bytes of NaN are refused by a NaN-free type, read by a `MaybeNan` one; every other float reads back.
#[cfg(feature = "shrink_wrap")]
#[expect(clippy::expect_used, reason = "a property fails by panicking")]
pub(crate) fn wire(x: f32) {
    use shrink_wrap::prelude::*;
    let mut buf = [0u8; 16];
    let bytes = M::of(x).to_ww_bytes(&mut buf).expect("encodes").to_vec();
    match V::from_ww_bytes(&bytes) {
        Ok(v) => assert!(!x.is_nan() && same(v.get(), x), "{x:e} read back"),
        Err(_) => assert!(x.is_nan(), "{x:e} refused"),
    }
    assert!(M::from_ww_bytes(&bytes).is_ok_and(|m| same(m.get(), x)));
}

/// Bytes from outside, read as a float `Fixed` of every NaN-free bound and of [MaybeNan]: through the wire, through
/// JSON and as text with its unit (`with_unit`), in `V` and `mV`. A NaN-free type never holds NaN (a [Finite] one
/// never ±∞, a `RangeDec` one nothing outside), and what it holds writes and reads back to the same bits. A
/// [MaybeNan] reads every bit pattern of the wire and writes it back unchanged. Never a panic. `fuzz/` runs it as
/// the target `nan_free`.
#[cfg(any(feature = "shrink_wrap", all(feature = "serde", feature = "std")))]
pub(crate) fn input(data: &[u8]) {
    #[cfg(feature = "shrink_wrap")]
    {
        use exact_units::RangeDec;
        wire_input::<One, f32, exact_units::Unbounded>(data);
        wire_input::<One, f32, Finite>(data);
        wire_input::<One, f32, RangeDec<-5, 25, -1>>(data);
        wire_input::<One, f64, exact_units::Unbounded>(data);
        wire_input::<One, f64, Finite>(data);
        wire_input::<One, f64, RangeDec<-5, 25, -1>>(data);
        wire_maybe_nan(data);
    }
    #[cfg(all(feature = "serde", feature = "std"))]
    if let Ok(s) = core::str::from_utf8(data) {
        use exact_units::RangeDec;
        use exact_units::prefix::Milli;
        text_input::<One, f32, exact_units::Unbounded>(s);
        text_input::<Milli, f32, exact_units::Unbounded>(s);
        text_input::<One, f32, Finite>(s);
        text_input::<Milli, f32, RangeDec<-5, 25, -1>>(s);
        text_input::<One, f64, exact_units::Unbounded>(s);
        text_input::<Milli, f64, Finite>(s);
        text_input::<One, f32, MaybeNan>(s);
        text_input::<Milli, f64, MaybeNan>(s);
    }
}

/// `v` as a NaN-free bound `B` must hold it: not NaN, within `B`.
#[cfg(any(feature = "shrink_wrap", all(feature = "serde", feature = "std")))]
fn holds<T: exact_units::Storage + core::fmt::Debug, B: exact_units::Bounds<T>>(v: T, how: &str) {
    assert!(!v.is_nan() || B::ALLOWS_NAN, "{how}: NaN read");
    assert!(
        B::contains(v),
        "{how}: {v:?} outside {}",
        core::any::type_name::<B>()
    );
}

#[cfg(feature = "shrink_wrap")]
#[expect(clippy::expect_used, reason = "a property fails by panicking")]
fn wire_input<P, T, B>(data: &[u8])
where
    P: 'static,
    T: exact_units::Storage + core::fmt::Debug,
    B: exact_units::Bounds<T>,
    Fixed<Volt, P, T, B>: shrink_wrap::prelude::SerializeShrinkWrap
        + for<'i> shrink_wrap::prelude::DeserializeShrinkWrap<'i>,
{
    use shrink_wrap::prelude::*;
    let Ok(v) = Fixed::<Volt, P, T, B>::from_ww_bytes(data) else {
        return;
    };
    holds::<T, B>(v.get(), "wire");
    let mut buf = [0u8; 16];
    let bytes = v.to_ww_bytes(&mut buf).expect("encodes").to_vec();
    let back = Fixed::<Volt, P, T, B>::from_ww_bytes(&bytes).expect("reads back");
    assert!(back.get() == v.get(), "{bytes:02x?}");
}

/// A [MaybeNan] `f32` and `f64` read their bytes off the wire, NaN payloads and signs included, and write them back.
#[cfg(feature = "shrink_wrap")]
#[expect(clippy::expect_used, reason = "a property fails by panicking")]
fn wire_maybe_nan(data: &[u8]) {
    use shrink_wrap::prelude::*;
    let mut buf = [0u8; 16];
    if let Some(&b) = data.first_chunk::<4>() {
        let m = M::from_ww_bytes(data).expect("every 4 bytes are an f32");
        assert_eq!(m.get().to_bits(), u32::from_le_bytes(b), "{data:02x?}");
        assert_eq!(m.to_ww_bytes(&mut buf).expect("encodes"), &b);
    }
    if let Some(&b) = data.first_chunk::<8>() {
        let m = Fixed::<Volt, One, f64, MaybeNan>::from_ww_bytes(data)
            .expect("every 8 bytes are an f64");
        assert_eq!(m.get().to_bits(), u64::from_le_bytes(b), "{data:02x?}");
        assert_eq!(m.to_ww_bytes(&mut buf).expect("encodes"), &b);
    }
}

/// `s` as JSON (a number) and as `with_unit` text: a NaN-free type holds no NaN; every value read writes and reads
/// back, a NaN of a [MaybeNan] as a NaN.
#[cfg(all(feature = "serde", feature = "std"))]
#[expect(
    clippy::expect_used,
    clippy::panic,
    reason = "a property fails by panicking"
)]
fn text_input<P, T, B>(s: &str)
where
    P: exact_units::FixedScale,
    T: exact_units::Storage
        + core::fmt::Debug
        + serde::Serialize
        + for<'de> serde::Deserialize<'de>,
    B: exact_units::Bounds<T>,
{
    #[derive(serde::Serialize, serde::Deserialize)]
    #[serde(transparent)]
    struct Text<P: exact_units::FixedScale, T: exact_units::Storage, B: exact_units::Bounds<T>>(
        #[serde(with = "exact_units::serde::with_unit")] Fixed<Volt, P, T, B>,
    );

    let same = |a: T, b: T| a == b || (a.is_nan() && b.is_nan());
    if let Ok(v) = serde_json::from_str::<Fixed<Volt, P, T, B>>(s) {
        holds::<T, B>(v.get(), "JSON");
    }
    let Ok(Text(v)) = serde_json::from_value::<Text<P, T, B>>(serde_json::Value::String(s.into()))
    else {
        return;
    };
    holds::<T, B>(v.get(), "text");
    let written = serde_json::to_value(Text(v)).expect("writes");
    let back = serde_json::from_value::<Text<P, T, B>>(written.clone());
    if P::SCALE == exact_units::Magnitude::ONE {
        let Text(back) =
            back.unwrap_or_else(|e| panic!("{s:?} wrote {written}, which reads as {e}"));
        assert!(same(back.get(), v.get()), "{s:?} wrote {written}");
    } else if let Ok(Text(back)) = back {
        // a scaled float goes through f64 both ways and may come back another float (`52.65 mV` of an f64 writes
        // `0.05264999999999999 V`): P2401#3nov-0058h; NaN stays NaN and a number a number
        assert_eq!(
            back.get().is_nan(),
            v.get().is_nan(),
            "{s:?} wrote {written}"
        );
    }
}

#[cfg(not(any(kani, fuzzing)))]
mod tests {
    use super::*;
    use proptest::prelude::*;

    /// Every f32 whose low 16 bits are 0: every sign, exponent and the top of the mantissa, NaNs and infinities.
    #[test]
    fn every_f32_constructs() {
        for hi in 0..=u16::MAX {
            constructs(f32::from_bits(u32::from(hi) << 16));
        }
    }

    #[test]
    fn every_pair_of_specials_operates() {
        for a in SPECIAL {
            for b in SPECIAL {
                operates(a, b);
                finite_operates(a, b);
            }
        }
    }

    /// Every f32 whose low 16 bits are 0 against [Finite], ±∞ and the NaNs among them.
    #[test]
    fn every_f32_finite_constructs() {
        for hi in 0..=u16::MAX {
            finite_constructs(f32::from_bits(u32::from(hi) << 16));
        }
    }

    #[cfg(feature = "shrink_wrap")]
    #[test]
    fn specials_cross_the_wire() {
        for x in SPECIAL {
            wire(x);
        }
    }

    #[test]
    fn conversions_of_nan_are_none() {
        use exact_units::prefix::Milli;
        let nan = M::of(f32::NAN);
        assert_eq!(nan.to::<Milli, f32>(), None);
        assert_eq!(nan.to::<Milli, i32>(), None);
        assert_eq!(nan.not_nan(), None);
        assert_eq!(nan.to_bounds::<exact_units::Unbounded>(), Err(OutOfBounds));
        let inf = V::bounded(f32::INFINITY);
        assert_eq!(
            inf.to::<Milli, f32>(),
            Some(Fixed::<_, _, f32>::bounded(f32::INFINITY))
        );
    }

    /// Hand-picked input of the fuzz target `nan_free`: the bytes of NaN, ±∞ and the edges of `f32` and `f64`, and
    /// text and JSON for NaN, ±∞ and values whose scaling overflows.
    #[test]
    #[cfg(any(feature = "shrink_wrap", all(feature = "serde", feature = "std")))]
    fn hand_picked_input() {
        for x in SPECIAL {
            input(&x.to_le_bytes());
            input(&f64::from(x).to_le_bytes());
        }
        input(&0x7fc0_0001_u32.to_le_bytes());
        input(&0xfff8_0000_0000_0001_u64.to_le_bytes());
        for s in [
            "NaN V",
            "nan V",
            "-NaN V",
            "inf V",
            "-inf V",
            "Infinity mV",
            "1e39 V",
            "1e39 kV",
            "1e308 kV",
            "1e-46 mV",
            "-0 V",
            "2.5 V",
            "2.6 V",
            "NaN",
            "null",
            "1e39",
            "-1e309",
            "0",
        ] {
            input(s.as_bytes());
        }
    }

    proptest! {
        #[test]
        #[cfg(any(feature = "shrink_wrap", all(feature = "serde", feature = "std")))]
        fn drawn_input(data in proptest::collection::vec(any::<u8>(), 0..16)) {
            input(&data);
        }

        #[test]
        #[cfg(any(feature = "shrink_wrap", all(feature = "serde", feature = "std")))]
        fn drawn_text(s in "-?[0-9]{0,3}(\\.[0-9]{0,3})?(e-?[0-9]{1,3})? ?(m|k|M|G|µ)?V") {
            input(s.as_bytes());
        }

        #[test]
        fn drawn_floats_operate(a in any::<f32>(), b in any::<f32>()) {
            operates(a, b);
        }

        #[test]
        fn drawn_floats_construct(x in any::<f32>()) {
            constructs(x);
            finite_constructs(x);
        }

        #[test]
        fn drawn_finite_floats_operate(a in any::<f32>(), b in any::<f32>()) {
            finite_operates(a, b);
        }
    }
}
