//! NaN-free floats: a float `Fixed` without [MaybeNan] never holds NaN. Every way in refuses it (`try_new`, the
//! wire, a conversion; text and serde in `tests/serde.rs`). Arithmetic never checks and never panics: `+`, `-`,
//! `*` and `/` of floats give a [MaybeNan] whose bits are the IEEE 754 result's, and a [MaybeNan] compares only
//! through `gt_or` and its kin, which are the IEEE compare where neither side is NaN and the given fallback where
//! one is. Checked against the plain float operation, on every pair of the special values (±0, the smallest
//! subnormal, ±1, the largest finite, ±∞, NaN) and on drawn floats; construction on every `f32` whose low 16 bits
//! are 0.

use exact_units::prefix::One;
use exact_units::units::{Ampere, Volt};
use exact_units::{Fixed, MaybeNan, OutOfBounds};

type V = Fixed<Volt, One, f32>;
type M = Fixed<Volt, One, f32, MaybeNan>;

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
            }
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

    proptest! {
        #[test]
        fn drawn_floats_operate(a in any::<f32>(), b in any::<f32>()) {
            operates(a, b);
        }

        #[test]
        fn drawn_floats_construct(x in any::<f32>()) {
            constructs(x);
        }
    }
}
