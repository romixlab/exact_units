//! Property 9: the wire format (feature `shrink_wrap`). A value encodes, its bytes fit the type's `MAX_SIZE` where it
//! has one, and decode to the same value; bytes from outside decode to an error or to a value that encodes and
//! decodes the same again, never a panic. A dimension that isn't canonical doesn't decode, a bounded `Fixed` outside
//! its bounds neither. Decoded units are walked (shown, their dimension and factor worked out) without a panic.
//! Byte buffers, so no Kani proof; `fuzz/` runs the same functions.
#![expect(
    clippy::arithmetic_side_effects,
    reason = "a test computes with the values it chose, which fit; an overflow fails the test"
)]
#![expect(clippy::panic, reason = "a property fails by panicking")]

use core::fmt::Debug;
use exact_units::origins::Celsius;
use exact_units::prefix::{Centi, Milli, One};
use exact_units::units::{Kelvin, Ratio, Sample, Volt};
use exact_units::{
    Complex, Counted, Dim, Fixed, FullScaleBits, Index, IndexFrom1, Irrational, Magnitude, Offset,
    Point, Pow2Exp, Pow10Exp, Prefix, PrimePower, Range, SIExpr, SIExprOwned, Scale, Seq, tree,
};
use shrink_wrap::max_size::MaxSize;
use shrink_wrap::prelude::*;
use shrink_wrap::series::{Series, SeriesOwned};

/// Large enough for every value below: a unit tree of a few hundred names.
const BUF: usize = 1 << 16;

/// `x` encodes within its `MAX_SIZE` and decodes to itself, and encodes to the same bytes again.
#[expect(clippy::expect_used, reason = "a property fails by panicking")]
pub(crate) fn round_trip<T>(x: &T)
where
    T: SerializeShrinkWrap + for<'i> DeserializeShrinkWrap<'i> + PartialEq + Debug,
{
    let mut buf = vec![0u8; BUF];
    let bytes = x.to_ww_bytes(&mut buf).expect("encodes").to_vec();
    if let Some(max) = MaxSize::buffer_len_of(<T as SerializeShrinkWrap>::MAX_SIZE) {
        assert!(
            bytes.len() <= max,
            "{x:?}: {} bytes, MAX_SIZE {max}",
            bytes.len()
        );
    }
    let back = T::from_ww_bytes(&bytes).expect("decodes");
    assert_eq!(&back, x, "{bytes:02x?}");
    let mut buf2 = vec![0u8; BUF];
    assert_eq!(back.to_ww_bytes(&mut buf2).expect("encodes"), bytes);
}

/// `bytes` decode as `T` to an error or to a value that round-trips.
pub(crate) fn decode<T>(bytes: &[u8])
where
    T: SerializeShrinkWrap + for<'i> DeserializeShrinkWrap<'i> + PartialEq + Debug,
{
    if let Ok(x) = T::from_ww_bytes(bytes) {
        round_trip(&x);
    }
}

/// A `Fixed`, a bounded one, a complex value, a point and the indices of `x`'s bits: each round-trips, and the
/// bounded one decodes from the plain number's bytes only within its bounds.
pub(crate) fn numbers(x: i16, y: u32) {
    round_trip(&Fixed::<Volt, Milli, i16>::new(x));
    round_trip(&Fixed::<Volt, Milli, u32>::new(y));
    round_trip(&Fixed::<Ratio, One, f32>::new(f32::from(x) / 7.0));
    round_trip(&Complex::from_parts(
        Fixed::<Volt, Milli, i16>::new(x),
        Fixed::new(x.wrapping_neg()),
    ));
    round_trip(&Point::<Celsius, Fixed<Kelvin, Centi, i16>>::from_raw(x));
    round_trip(&Index::<Sample, u32>::new(y));
    round_trip(&Seq::<Sample, u32>::new(y));
    type Bounded = Fixed<Volt, Milli, i16, Range<-1000, 1000>>;
    let mut buf = [0u8; 8];
    let plain = Fixed::<Volt, Milli, i16>::new(x);
    #[expect(clippy::expect_used, reason = "a property fails by panicking")]
    let bytes = plain.to_ww_bytes(&mut buf).expect("encodes").to_vec();
    match Bounded::try_new(x) {
        Ok(b) => {
            round_trip(&b);
            assert_eq!(Bounded::from_ww_bytes(&bytes).ok(), Some(b));
        }
        Err(_) => assert!(
            Bounded::from_ww_bytes(&bytes).is_err(),
            "{x} decodes past its bounds"
        ),
    }
    let one = IndexFrom1::<Sample, u32>::try_new(y);
    if let Ok(i) = one {
        round_trip(&i);
    }
}

/// A `Series` of rows with `Fixed` fields round-trips and fits `Series::max_bytes`.
#[expect(clippy::expect_used, reason = "a property fails by panicking")]
pub(crate) fn series(rows: &[(u32, i16, u8)]) {
    let items: Vec<Row> = rows
        .iter()
        .map(|&(t, v, d)| Row {
            t: Fixed::new(t),
            v: Fixed::new(v),
            duty: Fixed::new(d),
        })
        .collect();
    let bytes = Series::new(&items).to_ww_bytes_owned().expect("encodes");
    let max = Series::<Row>::max_bytes(items.len()).expect("plain numbers have a bound");
    assert!(bytes.len() <= max, "{} bytes, max_bytes {max}", bytes.len());
    let back = SeriesOwned::<Row>::from_ww_bytes_owned(&bytes).expect("decodes");
    assert_eq!(back.iter().copied().collect::<Vec<_>>(), items);
}

/// A row of a [series].
#[derive_shrink_wrap(borrowed, owned, series, derive(Clone, Copy, Debug, PartialEq))]
pub(crate) struct Row {
    /// µs.
    t: Fixed<exact_units::units::Second, exact_units::prefix::Micro, u32>,
    /// mV.
    v: Fixed<Volt, Milli, i16>,
    /// %.
    duty: Fixed<Ratio, Centi, u8>,
}

/// A magnitude from its parts ([Magnitude::from_parts_x2], what serde and a hand-edited file go through) is an error
/// or a magnitude that round-trips, shows, and is rebuilt from its own parts.
pub(crate) fn magnitude(pow10: i8, pow2: i8, irr: [i8; 3], primes: &[(u32, i8)]) {
    let irrational = Irrational {
        pi: irr[0],
        ln2: irr[1],
        ln10: irr[2],
    };
    let parts = primes.iter().map(|&(prime, exp)| PrimePower { prime, exp });
    let Ok(m) = Magnitude::from_parts_x2(pow10, pow2, irrational, parts) else {
        return;
    };
    round_trip(&m);
    let _ = m.to_string();
    let _ = (m.to_f64(), m.to_ratio());
    let (p10, p2) = m.pow10_pow2_x2();
    assert_eq!(
        Magnitude::from_parts_x2(p10, p2, m.irrational_x2(), m.primes()),
        Ok(m)
    );
    for neg in [false, true] {
        round_trip(&Offset::new(neg, m));
    }
}

/// A prefix by a variant number and a parameter: each round-trips within its `MAX_SIZE`.
pub(crate) fn prefix(which: u8, param: i8, tag: u16) {
    let p = match which % 5 {
        0 => Prefix::ALL
            .get(usize::from(param.unsigned_abs()) % Prefix::ALL.len())
            .copied(),
        1 => Pow10Exp::new(param).map(Prefix::Pow10),
        2 => Pow2Exp::new(param).map(Prefix::Pow2),
        3 => FullScaleBits::new(param.unsigned_abs()).map(Prefix::FullScale),
        _ => Some(Prefix::Unknown(tag)),
    };
    if let Some(p) = p {
        round_trip(&p);
        let _ = p.to_string();
    }
}

/// A dimension as written into its fields, canonical or not. It encodes where every exponent fits 4 bits, and then
/// decodes, to itself, exactly when it is canonical (used counted slots first, sorted, no exponent 0, `root` only
/// with an odd exponent).
#[expect(clippy::expect_used, reason = "a property fails by panicking")]
pub(crate) fn dim(exps: [i8; 9], root: bool, counted: &[Option<(String, i8)>; 4]) {
    let [s, m, kg, a, k, mol, cd, rad, bit] = exps;
    let d = Dim {
        s,
        m,
        kg,
        a,
        k,
        mol,
        cd,
        rad,
        bit,
        root,
        counted: counted.each_ref().map(|c| {
            c.as_ref().map(|(name, exp)| Counted {
                crate_name: "exact_units",
                name,
                exp: *exp,
            })
        }),
    };
    let fits = |e: i8| (-8..=7).contains(&e);
    let mut buf = vec![0u8; BUF];
    let Ok(bytes) = d.to_ww_bytes(&mut buf) else {
        assert!(
            !exps
                .into_iter()
                .chain(d.counted.iter().flatten().map(|c| c.exp))
                .all(fits),
            "{d:?} doesn't encode"
        );
        return;
    };
    let bytes = bytes.to_vec();
    match Dim::from_ww_bytes(&bytes) {
        Ok(back) => {
            assert!(d.is_canonical(), "{d:?} isn't canonical, yet decodes");
            assert_eq!(back, d);
        }
        Err(_) => assert!(
            !d.is_canonical() || !exps.into_iter().all(fits),
            "{d:?} is canonical, yet doesn't decode"
        ),
    }
    let _ = d.to_string();
    let _ = d.checked_mul(d).map(|x| x.checked_div(d));
    let owned = exact_units::DimOwned::from(d);
    if let Ok(back) = exact_units::DimOwned::from_ww_bytes_owned(&bytes) {
        assert_eq!(back, owned);
        assert_eq!(back.to_ww_bytes_owned().expect("encodes"), bytes);
    }
}

/// A unit tree as plain data, for building an [SIExprOwned] from proptest's or the fuzzer's values.
#[derive(Clone, Debug)]
#[cfg_attr(fuzzing, derive(arbitrary::Arbitrary))]
pub(crate) enum Expr {
    /// One of the crate's named units, by its place in `units::ALL`.
    Known(u16),
    /// A named unit of its own: a name, a symbol, its exponents of s, m and kg, and a power of ten.
    Own(String, String, [i8; 3], i8),
    /// A product.
    Mul(Box<Expr>, Box<Expr>),
    /// A quotient.
    Div(Box<Expr>, Box<Expr>),
    /// A power.
    Pow(Box<Expr>, i8),
    /// A square root.
    Sqrt(Box<Expr>),
}

impl Expr {
    /// The unit as an owned `SIExpr`.
    fn build(&self) -> SIExprOwned {
        match self {
            Expr::Known(i) => {
                let all = exact_units::units::ALL;
                let Some(exact_units::UnitExpr::Named {
                    name,
                    symbol,
                    dim,
                    scale,
                    ..
                }) = all.get(usize::from(*i) % all.len())
                else {
                    panic!("ALL holds named units")
                };
                SIExprOwned::Named {
                    name: (*name).to_string(),
                    symbol: (*symbol).to_string(),
                    dim: (*dim).into(),
                    scale: *scale,
                }
            }
            Expr::Own(name, symbol, [s, m, kg], p10) => SIExprOwned::Named {
                name: name.clone(),
                symbol: symbol.clone(),
                dim: Dim::new(*s % 8, *m % 8, *kg % 8, 0, 0, 0, 0).into(),
                scale: Scale::Pow10(*p10),
            },
            Expr::Mul(a, b) => SIExprOwned::Mul(Box::new(a.build()), Box::new(b.build())),
            Expr::Div(a, b) => SIExprOwned::Div(Box::new(a.build()), Box::new(b.build())),
            Expr::Pow(a, n) => SIExprOwned::Pow(Box::new(a.build()), *n),
            Expr::Sqrt(a) => SIExprOwned::Sqrt(Box::new(a.build())),
        }
    }
}

/// A unit tree round-trips, owned and borrowed, and every walk over it (symbol, name, dimension, factor, magnitude,
/// known unit) ends without a panic, also past `MAX_UNIT_DEPTH`.
#[expect(clippy::expect_used, reason = "a property fails by panicking")]
pub(crate) fn si_expr(e: &Expr) {
    let owned = e.build();
    let bytes = owned.to_ww_bytes_owned().expect("encodes");
    let back = SIExprOwned::from_ww_bytes_owned(&bytes).expect("decodes");
    assert_eq!(back, owned);
    let borrowed = SIExpr::from_ww_bytes(&bytes).expect("decodes");
    walk(&borrowed);
    walk_owned(&owned);
    assert_eq!(back.to_ww_bytes_owned().expect("encodes"), bytes);
}

/// Every walk of a decoded unit.
fn walk(e: &SIExpr<'_>) {
    let _ = (e.to_string(), e.name().to_string(), e.symbol().to_string());
    let _ = (e.dim(), e.factor(), e.magnitude(), e.offset(), e.known());
    let _ = (tree::is_ratio(e), tree::same(e, e));
    let owned = SIExprOwned::from(e);
    let _ = owned.to_string();
}

/// Every walk of an owned unit.
fn walk_owned(e: &SIExprOwned) {
    let _ = e.to_string();
    let _ = (
        tree::dim(&e),
        tree::factor(&e),
        tree::magnitude(&e),
        tree::offset(&e),
    );
}

/// Bytes from outside, as the type `which` picks: an error or a value that round-trips, and a decoded unit walks.
pub(crate) fn any_bytes(which: u8, bytes: &[u8]) {
    match which % 12 {
        0 => decode::<Fixed<Volt, Milli, i16>>(bytes),
        1 => decode::<Fixed<Volt, Milli, i16, Range<-1000, 1000>>>(bytes),
        2 => decode::<Complex<Volt, Milli, i16>>(bytes),
        3 => decode::<Magnitude>(bytes),
        4 => decode::<Prefix>(bytes),
        5 => decode::<Offset>(bytes),
        6 => decode::<IndexFrom1<Sample, u32>>(bytes),
        7 => {
            if let Ok(d) = Dim::from_ww_bytes(bytes) {
                assert!(d.is_canonical(), "{d:?} decodes, not canonical");
                let mut buf = vec![0u8; BUF];
                #[expect(clippy::expect_used, reason = "a property fails by panicking")]
                let again = d.to_ww_bytes(&mut buf).expect("encodes");
                assert_eq!(Dim::from_ww_bytes(again).ok(), Some(d));
                let _ = d.to_string();
            }
            if let Ok(d) = exact_units::DimOwned::from_ww_bytes_owned(bytes) {
                let _ = d.as_dim().to_string();
            }
        }
        8 => {
            if let Ok(s) = Scale::from_ww_bytes(bytes) {
                let mut buf = vec![0u8; BUF];
                #[expect(clippy::expect_used, reason = "a property fails by panicking")]
                let again = s.to_ww_bytes(&mut buf).expect("encodes").to_vec();
                #[expect(clippy::expect_used, reason = "a property fails by panicking")]
                let back = Scale::from_ww_bytes(&again).expect("decodes");
                // the bytes, not ==: an Affine scale may hold a NaN
                assert_eq!(back.to_ww_bytes(&mut buf).ok(), Some(&again[..]));
                let _ = (s.factor(), s.magnitude(), s.offset());
            }
        }
        9 => {
            if let Ok(e) = SIExpr::from_ww_bytes(bytes) {
                walk(&e);
            }
        }
        10 => {
            if let Ok(e) = SIExprOwned::from_ww_bytes_owned(bytes) {
                walk_owned(&e);
                #[expect(clippy::expect_used, reason = "a property fails by panicking")]
                let again = e.to_ww_bytes_owned().expect("encodes");
                assert!(SIExprOwned::from_ww_bytes_owned(&again).is_ok());
            }
        }
        _ => {
            if let Ok(s) = SeriesOwned::<Row>::from_ww_bytes_owned(bytes) {
                let _ = s.iter().count();
            }
        }
    }
}

#[cfg(not(any(kani, fuzzing)))]
mod tests {
    use super::Expr;
    use crate::strategy;
    use proptest::collection::vec;
    use proptest::prelude::*;

    fn expr() -> impl Strategy<Value = Expr> {
        let leaf = prop_oneof![
            any::<u16>().prop_map(Expr::Known),
            (
                "[A-Za-zµ°Ω]{0,8}",
                "[A-Za-zµ°Ω/]{0,4}",
                any::<[i8; 3]>(),
                any::<i8>()
            )
                .prop_map(|(n, s, d, p)| Expr::Own(n, s, d, p)),
        ];
        leaf.prop_recursive(8, 64, 2, |inner| {
            prop_oneof![
                (inner.clone(), inner.clone())
                    .prop_map(|(a, b)| Expr::Mul(Box::new(a), Box::new(b))),
                (inner.clone(), inner.clone())
                    .prop_map(|(a, b)| Expr::Div(Box::new(a), Box::new(b))),
                (inner.clone(), any::<i8>()).prop_map(|(a, n)| Expr::Pow(Box::new(a), n)),
                inner.prop_map(|a| Expr::Sqrt(Box::new(a))),
            ]
        })
    }

    /// Counted bases from a few names, so equal and sorted ones come up.
    fn counted() -> impl Strategy<Value = [Option<(String, i8)>; 4]> {
        let slot = proptest::option::of((
            proptest::sample::select(&["Block", "Frame", "Item", "Sample", ""][..])
                .prop_map(String::from),
            -9i8..=9,
        ));
        [slot.clone(), slot.clone(), slot.clone(), slot]
    }

    proptest! {
        #[test]
        fn numbers(x in strategy::i16(), y: u32) {
            super::numbers(x, y);
        }

        #[test]
        fn series(rows in vec(any::<(u32, i16, u8)>(), 0..80)) {
            super::series(&rows);
        }

        #[test]
        fn magnitudes(
            pow10: i8,
            pow2: i8,
            irr in any::<[i8; 3]>(),
            primes in vec((prop_oneof![
                proptest::sample::select(&[3u32, 7, 11, 13, 2, 5, 9, 65_521, 4_294_967_291][..]),
                any::<u32>(),
            ], any::<i8>()), 0..6),
        ) {
            super::magnitude(pow10, pow2, irr, &primes);
        }

        #[test]
        fn prefixes(which: u8, param: i8, tag: u16) {
            super::prefix(which, param, tag);
        }

        #[test]
        fn dims(exps in any::<[i8; 9]>().prop_map(|e| e.map(|x| x % 9)), root: bool, counted in counted()) {
            super::dim(exps, root, &counted);
        }

        #[test]
        fn units(e in expr()) {
            super::si_expr(&e);
        }

        #[test]
        fn bytes(which: u8, bytes in vec(any::<u8>(), 0..64)) {
            super::any_bytes(which, &bytes);
        }
    }
}
