//! Property 9: the wire format (feature `shrink_wrap`). A value encodes, its bytes fit the type's `MAX_SIZE` where it
//! has one, and decode to the same value; bytes from outside decode to an error or to a value that encodes and
//! decodes the same again, never a panic. A dimension that isn't canonical doesn't decode, a bounded `Fixed` outside
//! its bounds neither, a `Niched` number that is neither a value nor a niche code neither. Decoded units are walked (shown, their dimension and factor worked out) without a panic.
//! Byte buffers, so no Kani proof; `fuzz/` runs the same functions.
#![expect(
    clippy::arithmetic_side_effects,
    reason = "a test computes with the values it chose, which fit; an overflow fails the test"
)]
#![expect(clippy::panic, reason = "a property fails by panicking")]

use core::fmt::Debug;
use exact_units::niche::{J1939U16, Niched, Null, Reading};
use exact_units::origins::Celsius;
use exact_units::prefix::{Centi, Milli, One};
use exact_units::units::{Kelvin, Ratio, Sample, Volt};
use exact_units::{
    BaseExp, CompactError, Complex, Dim, DimWire, Exp, Fixed, FullScaleBits, Index, IndexFrom1,
    Magnitude, Offset, Point, Pow2Exp, Pow10Exp, Prefix, PrimePower, Range, SIExpr, SIExprOwned,
    Seq, Symbols, UnitDef, UnitDefOwned, UnitTable, tree,
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
    round_trip(&Fixed::<Ratio, One, f32>::bounded(f32::from(x) / 7.0));
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
    // a bounded index is the number on the wire too, read only within its bounds
    type Slot = Index<Sample, u32, Range<0, 1000>>;
    let mut buf = [0u8; 8];
    #[expect(clippy::expect_used, reason = "a property fails by panicking")]
    let bytes = y.to_ww_bytes(&mut buf).expect("encodes").to_vec();
    match Slot::try_new(y) {
        Ok(i) => {
            round_trip(&i);
            assert_eq!(Slot::from_ww_bytes(&bytes).ok(), Some(i));
        }
        Err(_) => assert!(
            Slot::from_ww_bytes(&bytes).is_err(),
            "index {y} decodes past its bounds"
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
pub(crate) fn magnitude(pow10: i8, pow2: i8, irr: [i8; 5], primes: &[(u32, i8)]) {
    let symbols = Symbols {
        pi: irr[0],
        ln2: irr[1],
        ln10: irr[2],
        mu0: irr[3],
        g: irr[4],
    };
    let parts = primes.iter().map(|&(prime, exp)| PrimePower { prime, exp });
    let Ok(m) = Magnitude::from_parts_x2(pow10, pow2, symbols, parts) else {
        return;
    };
    round_trip(&m);
    let _ = m.to_string();
    let _ = (m.checked_to_f64(), m.to_ratio());
    assert_eq!(m.checked_to_f64().is_none(), m.has_measured());
    let (p10, p2) = m.pow10_pow2_x2();
    assert_eq!(
        Magnitude::from_parts_x2(p10, p2, m.symbols_x2(), m.primes()),
        Ok(m)
    );
    for neg in [false, true] {
        round_trip(&Offset::new(neg, m));
    }
}

/// A prefix by a variant number and a parameter: each round-trips within its `MAX_SIZE`. `Unknown(tag)` with the tag of
/// a kind this version knows (the named prefixes and the three families) doesn't encode: a reader would take it for
/// that kind.
pub(crate) fn prefix(which: u8, param: i8, tag: u16) {
    let p = match which % 5 {
        0 => Prefix::ALL
            .get(usize::from(param.unsigned_abs()) % Prefix::ALL.len())
            .copied(),
        1 => Pow10Exp::new(param).map(Prefix::Pow10),
        2 => Pow2Exp::new(param).map(Prefix::Pow2),
        3 => FullScaleBits::new(param.unsigned_abs()).map(Prefix::FullScale),
        _ => {
            let known = u16::try_from(Prefix::ALL.len() + 3).unwrap_or(u16::MAX);
            let p = Prefix::Unknown(tag);
            let mut buf = [0u8; 16];
            assert_eq!(p.to_ww_bytes(&mut buf).is_ok(), tag >= known, "{p:?}");
            (tag >= known).then_some(p)
        }
    };
    if let Some(p) = p {
        round_trip(&p);
        let _ = p.to_string();
    }
}

/// A dimension as written into the fields of its wire shape, canonical or not: exponents in halves (any count) and
/// distinguished bases (any order, repeats, 0). It always encodes, and decodes as a [Dim] exactly when it is canonical
/// (7 exponents, the bases sorted by crate and name without repeats, no exponent 0), to the same exponents and bases;
/// what decodes writes the same bytes again.
#[expect(clippy::expect_used, reason = "a property fails by panicking")]
pub(crate) fn dim(exps: &[i8], bases: &[(String, i8)]) {
    let exps: Vec<Exp> = exps.iter().map(|h| Exp::from_halves(*h)).collect();
    let list: Vec<BaseExp<'_>> = bases
        .iter()
        .map(|(name, h)| BaseExp {
            base: exact_units::MarkerId {
                crate_name: "exact_units",
                name,
            },
            exp: Exp::from_halves(*h),
        })
        .collect();
    let w = DimWire {
        system: exact_units::SystemRef::SI,
        exps: RefVec::Slice { slice: &exps },
        bases: RefVec::Slice { slice: &list },
    };
    let mut buf = vec![0u8; BUF];
    let bytes = w.to_ww_bytes(&mut buf).expect("encodes").to_vec();
    let canonical = exps.len() == exact_units::MAX_TRUE_BASES
        && list.iter().all(|b| !b.exp.is_zero())
        && list
            .windows(2)
            .all(|p| matches!(p, [a, b] if a.base < b.base));
    match Dim::from_ww_bytes(&bytes) {
        Ok(back) => {
            assert!(canonical, "{exps:?} {list:?} isn't canonical, yet decodes");
            assert_eq!(back.exps().as_slice(), exps.as_slice());
            let mut seen = Vec::new();
            back.for_each_base(|b| seen.push(b));
            assert_eq!(seen, list);
            let mut buf2 = vec![0u8; BUF];
            assert_eq!(
                back.to_ww_bytes(&mut buf2).expect("encodes"),
                bytes.as_slice()
            );
            let owned = exact_units::DimOwned::from(back);
            assert_eq!(owned, back);
            let _ = (
                back.to_string(),
                owned.checked_mul(&owned),
                owned.checked_sqrt(),
            );
        }
        Err(_) => assert!(
            !canonical,
            "{exps:?} {list:?} is canonical, yet doesn't decode"
        ),
    }
    if let Ok(back) = exact_units::DimOwned::from_ww_bytes_owned(&bytes) {
        assert!(canonical);
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
    /// A reference to one of the crate's named units, by its place in `units::ALL`.
    KnownRef(u16),
    /// A reference no table lists: a crate and a name.
    Ref(String, String),
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
    /// The nodes on the longest path from the root to a named unit.
    fn depth(&self) -> usize {
        match self {
            Expr::Known(_) | Expr::Own(..) | Expr::KnownRef(_) | Expr::Ref(..) => 1,
            Expr::Mul(a, b) | Expr::Div(a, b) => 1 + a.depth().max(b.depth()),
            Expr::Pow(a, _) | Expr::Sqrt(a) => 1 + a.depth(),
        }
    }

    /// The unit as an owned `SIExpr`.
    fn build(&self) -> SIExprOwned {
        let known = |i: u16| {
            let all = exact_units::units::ALL;
            let Some(exact_units::UnitExpr::Named(def)) = all.get(usize::from(i) % all.len())
            else {
                panic!("ALL holds named units")
            };
            **def
        };
        match self {
            Expr::Known(i) => SIExprOwned::Def(known(*i).into()),
            Expr::KnownRef(i) => SIExprOwned::Ref(known(*i).id.into()),
            Expr::Ref(crate_name, name) => SIExprOwned::Ref(exact_units::MarkerIdOwned {
                crate_name: crate_name.clone(),
                name: name.clone(),
            }),
            Expr::Own(name, symbol, [s, m, kg], p10) => SIExprOwned::Def(UnitDefOwned {
                id: exact_units::MarkerIdOwned {
                    crate_name: "props".into(),
                    name: name.clone(),
                },
                names: exact_units::UnitNamesOwned {
                    symbol: symbol.clone(),
                    ascii: None,
                    long: String::new(),
                    plural: String::new(),
                    aliases: String::new(),
                    ucum: None,
                    prefixes: exact_units::PrefixPolicy::None,
                },
                dim: Dim::new(*s % 8, *m % 8, *kg % 8, 0, 0, 0, 0).into(),
                quantity: None,
                scale: Magnitude::checked_pow10(*p10).unwrap_or(Magnitude::ONE),
                provenance: exact_units::ProvenanceOwned::Defined,
            }),
            Expr::Mul(a, b) => SIExprOwned::Mul(Box::new(a.build()), Box::new(b.build())),
            Expr::Div(a, b) => SIExprOwned::Div(Box::new(a.build()), Box::new(b.build())),
            Expr::Pow(a, n) => SIExprOwned::Pow(Box::new(a.build()), *n),
            Expr::Sqrt(a) => SIExprOwned::Sqrt(Box::new(a.build())),
        }
    }
}

/// The nodes on a path an owned unit tree always decodes with: shrink_wrap's reader stops at
/// `BufReader::DEFAULT_MAX_DEPTH` (128) nested values, so a recursive type from untrusted data can't overflow the
/// stack, and a named unit's own fields (its dimension, a counted base) take up to 4 of them. Far past
/// `MAX_UNIT_DEPTH` (32), the depth the crate follows.
const DECODED_DEPTH: usize = 124;

/// A unit tree round-trips, owned and borrowed, and every walk over it (symbol, name, dimension, factor, magnitude,
/// known unit) ends without a panic, also past `MAX_UNIT_DEPTH`. The owned form decodes up to [DECODED_DEPTH] nodes
/// deep, and from there on may be refused as too deeply nested, never past 128; the borrowed one reads its top node
/// only.
#[expect(clippy::expect_used, reason = "a property fails by panicking")]
pub(crate) fn si_expr(e: &Expr) {
    let owned = e.build();
    let bytes = owned.to_ww_bytes_owned().expect("encodes");
    match SIExprOwned::from_ww_bytes_owned(&bytes) {
        Ok(back) => {
            assert!(e.depth() <= 128, "{} deep, decodes", e.depth());
            assert_eq!(back, owned);
            assert_eq!(back.to_ww_bytes_owned().expect("encodes"), bytes);
        }
        Err(err) => assert!(
            err == shrink_wrap::Error::NestingTooDeep && e.depth() > DECODED_DEPTH,
            "{} deep: {err:?}",
            e.depth()
        ),
    }
    let borrowed = SIExpr::from_ww_bytes(&bytes).expect("decodes");
    walk(&borrowed);
    walk_owned(&owned);
    compact(&owned, &borrowed, e.depth());
}

/// The nodes of a tree.
fn nodes(e: &SIExprOwned) -> usize {
    match e {
        SIExprOwned::Ref(_) | SIExprOwned::Def(_) => 1,
        SIExprOwned::Mul(a, b) | SIExprOwned::Div(a, b) => 1 + nodes(a) + nodes(b),
        SIExprOwned::Pow(a, _) | SIExprOwned::Sqrt(a) => 1 + nodes(a),
    }
}

/// A tree compacted against `units::TABLE` is the same unit: the same dimension, magnitude and symbol through the
/// table, and the same tree once resolved again; owned and borrowed alike. Nothing conflicts (the crate's own
/// definitions, and units of the crate `props`), so only a tree deeper than `MAX_UNIT_DEPTH` is refused.
#[expect(clippy::expect_used, reason = "a property fails by panicking")]
fn compact(owned: &SIExprOwned, borrowed: &SIExpr<'_>, depth: usize) {
    let table = &exact_units::units::TABLE;
    let within = depth <= exact_units::MAX_UNIT_DEPTH;
    let compacted = match owned.compact(table) {
        Ok(c) => c,
        Err(err) => {
            assert!(
                !within && err == CompactError::TooDeep,
                "{depth} deep: {err:?}"
            );
            return;
        }
    };
    assert!(within, "{depth} deep, compacts");
    assert_eq!(compacted.dim(table), owned.dim(table));
    assert_eq!(compacted.magnitude(table), owned.magnitude(table));
    assert_eq!(compacted.to_string(), owned.to_string());
    assert!(compacted.same(owned, table));
    assert_eq!(
        compacted.resolve(table).expect("resolves"),
        owned.resolve(table).expect("resolves")
    );
    // nothing left to compact
    assert_eq!(compacted.compact(table).expect("compacts"), compacted);

    let mut arena = vec![None; nodes(owned)];
    let small = borrowed.compact_in(table, &mut arena).expect("compacts");
    assert_eq!(SIExprOwned::from(&small), compacted);
    assert_eq!(small.dim(table), compacted.dim(table));
    let mut again = vec![None; nodes(owned)];
    let resolved = small.resolve_in(table, &mut again).expect("resolves");
    assert_eq!(
        SIExprOwned::from(&resolved),
        owned.resolve(table).expect("resolves")
    );
}

/// Every walk of a decoded unit; its owned form is cut at `MAX_UNIT_DEPTH` levels however deep the bytes.
fn walk(e: &SIExpr<'_>) {
    let table = &exact_units::units::TABLE;
    let _ = (
        e.to_string(),
        e.name().to_string(),
        e.symbol(table).to_string(),
    );
    let _ = (e.dim(table), e.factor(table), e.magnitude(table), e.known());
    let _ = (e.dim(&()), e.symbol(&()).to_string(), e.same(e, table));
    let _ = (tree::is_ratio(e), tree::same(e, e));
    let mut arena = [const { None }; 8];
    let _ = e.compact_in(table, &mut arena).map(|c| c.to_string());
    let mut arena = [const { None }; 8];
    let _ = e.resolve_in(table, &mut arena).map(|c| c.to_string());
    let owned = SIExprOwned::from(e);
    let _ = owned.to_string();
    let depth = owned_depth(&owned);
    assert!(
        depth <= exact_units::MAX_UNIT_DEPTH + 1,
        "{depth} levels owned"
    );
}

/// The nodes on the longest path from the root of an owned tree to a named unit.
fn owned_depth(e: &SIExprOwned) -> usize {
    match e {
        SIExprOwned::Ref(_) | SIExprOwned::Def(_) => 1,
        SIExprOwned::Mul(a, b) | SIExprOwned::Div(a, b) => 1 + owned_depth(a).max(owned_depth(b)),
        SIExprOwned::Pow(a, _) | SIExprOwned::Sqrt(a) => 1 + owned_depth(a),
    }
}

/// Every walk of an owned unit.
fn walk_owned(e: &SIExprOwned) {
    let table = &exact_units::units::TABLE;
    let _ = (e.to_string(), e.symbol(table).to_string());
    let _ = (tree::dim(&e), tree::factor(&e), tree::magnitude(&e));
    let _ = (e.dim(table), e.factor(table), e.magnitude(table), e.known());
    let _ = (e.compact(table), e.resolve(table));
}

/// Bytes from outside as a definition and a unit tree: the tree walks against a table that lists the definition
/// before the crate's units, and compacts and resolves through it without a panic; what compacts is the same unit
/// through that table (dimension, magnitude, `same`), and resolves back to what the tree resolves to.
pub(crate) fn tree_against_table(def_bytes: &[u8], expr_bytes: &[u8]) {
    let (Ok(def), Ok(e)) = (
        UnitDef::from_ww_bytes(def_bytes),
        SIExpr::from_ww_bytes(expr_bytes),
    ) else {
        return;
    };
    let tables: [&dyn UnitTable; 2] = [&def, &exact_units::units::TABLE];
    let _ = (e.symbol(&tables).to_string(), e.factor(&tables));
    let _ = (tables.unit(def.id), e.same(&e, &tables));
    let mut arena = [const { None }; 16];
    let Ok(compact) = e.compact_in(&tables, &mut arena) else {
        return;
    };
    assert_eq!(compact.dim(&tables), e.dim(&tables), "{e:?}");
    assert_eq!(compact.magnitude(&tables), e.magnitude(&tables), "{e:?}");
    assert!(compact.same(&e, &tables), "{e:?}");
    let (mut a, mut b) = ([const { None }; 16], [const { None }; 16]);
    if let (Ok(x), Ok(y)) = (
        compact.resolve_in(&tables, &mut a),
        e.resolve_in(&tables, &mut b),
    ) {
        assert_eq!(SIExprOwned::from(&x), SIExprOwned::from(&y), "{e:?}");
    }
}

/// Bytes from outside, as the type `which` picks: an error or a value that round-trips, and a decoded unit walks.
pub(crate) fn any_bytes(which: u8, bytes: &[u8]) {
    match which % 14 {
        0 => decode::<Fixed<Volt, Milli, i16>>(bytes),
        1 => decode::<Fixed<Volt, Milli, i16, Range<-1000, 1000>>>(bytes),
        2 => decode::<Complex<Volt, Milli, i16>>(bytes),
        3 => decode::<Magnitude>(bytes),
        4 => decode::<Prefix>(bytes),
        5 => decode::<Offset>(bytes),
        6 => decode::<IndexFrom1<Sample, u32>>(bytes),
        7 => {
            if let Ok(d) = Dim::from_ww_bytes(bytes) {
                let mut buf = vec![0u8; BUF];
                #[expect(clippy::expect_used, reason = "a property fails by panicking")]
                let again = d.to_ww_bytes(&mut buf).expect("encodes");
                assert_eq!(Dim::from_ww_bytes(again).ok(), Some(d));
                let _ = d.to_string();
            }
            if let Ok(d) = exact_units::DimOwned::from_ww_bytes_owned(bytes) {
                assert_eq!(Dim::from_ww_bytes(bytes).ok(), Some(d.as_dim()), "{d:?}");
                #[cfg(feature = "serde")]
                super::serde_input::dim(&d);
                let _ = d.as_dim().to_string();
            }
        }
        8 => {
            if let Ok(d) = UnitDef::from_ww_bytes(bytes) {
                let mut buf = vec![0u8; BUF];
                #[expect(clippy::expect_used, reason = "a property fails by panicking")]
                let again = d.to_ww_bytes(&mut buf).expect("encodes").to_vec();
                #[expect(clippy::expect_used, reason = "a property fails by panicking")]
                let back = UnitDef::from_ww_bytes(&again).expect("decodes");
                assert_eq!(back, d);
                let _ = (d.id.to_string(), d.scale.to_f64(), d.quantity_expr());
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
        11 => {
            if let Ok(s) = SeriesOwned::<Row>::from_ww_bytes_owned(bytes) {
                let _ = s.iter().count();
            }
        }
        12 => {
            // a value within 0..=0xFAFF or a J1939 code, never a code as a value
            if let Ok(n) =
                Niched::<Fixed<Ratio, One, u16, Range<0, 0xFAFF>>, J1939U16>::from_ww_bytes(bytes)
            {
                round_trip(&n);
                if let Reading::Value(v) = n.get() {
                    assert!(v.get() <= 0xFAFF, "{n:?}");
                }
            }
        }
        _ => {
            // -127..=20 or the code 127, nothing else
            if let Ok(n) =
                Niched::<Fixed<Ratio, One, i8, Range<-127, 20>>, Null<127>>::from_ww_bytes(bytes)
            {
                round_trip(&n);
                assert!(n.raw() == 127 || (-127..=20).contains(&n.raw()), "{n:?}");
            }
        }
    }
}

#[cfg(not(any(kani, fuzzing)))]
mod tests {
    use super::Expr;
    use crate::strategy;
    use exact_units::SIExprOwned;
    use proptest::collection::vec;
    use proptest::prelude::*;
    use shrink_wrap::prelude::*;

    fn expr() -> impl Strategy<Value = Expr> {
        expr_leaf().prop_recursive(8, 64, 2, |inner| {
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

    /// A named unit: one of the crate's, of its own, or a reference.
    fn expr_leaf() -> impl Strategy<Value = Expr> {
        prop_oneof![
            any::<u16>().prop_map(Expr::Known),
            any::<u16>().prop_map(Expr::KnownRef),
            ("[a-z_]{0,6}", "[A-Za-z]{0,6}").prop_map(|(k, n)| Expr::Ref(k, n)),
            (
                "[A-Za-zµ°Ω]{0,8}",
                "[A-Za-zµ°Ω/]{0,4}",
                any::<[i8; 3]>(),
                any::<i8>()
            )
                .prop_map(|(n, s, d, p)| Expr::Own(n, s, d, p)),
        ]
    }

    /// Bases from a few names, so equal and sorted ones come up.
    fn bases() -> impl Strategy<Value = Vec<(String, i8)>> {
        let one = (
            proptest::sample::select(&["Block", "Frame", "Item", "Sample", ""][..])
                .prop_map(String::from),
            prop_oneof![-3i8..=3, any::<i8>()],
        );
        vec(one, 0..6)
    }

    /// Chains at the reader's depth limit and past it.
    #[test]
    fn deep_units() {
        for depth in [32, 33, 124, 125, 126, 127, 128, 129, 200] {
            for leaf in [
                Expr::Known(0),
                Expr::KnownRef(3),
                Expr::Own("Rpm".into(), "rpm".into(), [1, -2, 3], -3),
            ] {
                let mut e = leaf.clone();
                for k in 1..depth {
                    e = if k % 2 == 0 {
                        Expr::Sqrt(Box::new(e))
                    } else {
                        Expr::Mul(Box::new(e), Box::new(leaf.clone()))
                    };
                }
                super::si_expr(&e);
            }
        }
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
            irr in any::<[i8; 5]>(),
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
        fn dims(exps in prop_oneof![
            8 => vec(any::<i8>(), 7..=7),
            1 => vec(any::<i8>(), 0..10),
        ], bases in bases()) {
            super::dim(&exps, &bases);
        }

        #[test]
        fn units(e in expr()) {
            super::si_expr(&e);
        }

        #[test]
        fn bytes(which: u8, bytes in vec(any::<u8>(), 0..64)) {
            super::any_bytes(which, &bytes);
        }

        /// A definition of its own or one of the crate's under a changed symbol, against a tree of such units.
        #[test]
        fn tables(def in expr_leaf(), symbol in "[a-z]{0,2}", e in expr()) {
            let def = match def.build() {
                SIExprOwned::Def(mut d) => {
                    if !symbol.is_empty() {
                        d.names.symbol = symbol;
                    }
                    d
                }
                _ => return Ok(()),
            };
            let def_bytes = def.to_ww_bytes_owned().expect("encodes");
            let expr_bytes = e.build().to_ww_bytes_owned().expect("encodes");
            super::tree_against_table(&def_bytes, &expr_bytes);
        }
    }
}
