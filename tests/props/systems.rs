//! Property 19: the bridges between SI and the other systems. For a random SI dimension (exponents in halves, an
//! angle and a bit beside), the bridge into Natural and into Gaussian by the rule is the reference's
//! (`reference::systems`: the image and the factor from ħ = c = 1 and 1 m = 100 cm, 1 kg = 1000 g, a product of half
//! powers exact per prime), `const` and at run time; the way back is its reciprocal, so the two multiply to exactly 1;
//! an image one half off is refused. A dimension without true exponents is the same in every system, `==` and
//! `Hash` agreeing; one with true exponents in two systems is not. A dimension of each system, and each built-in
//! [SystemDef], go through the wire and back to the same bytes.
//! Magnitudes with a dozen primes and allocating references: no Kani proof.
#![cfg(not(kani))]
#![expect(
    clippy::arithmetic_side_effects,
    reason = "a test computes with the values it chose, which fit; an overflow fails the test"
)]

use super::reference::systems::{Factor, Key, Sys, project};
use core::hash::{BuildHasher, BuildHasherDefault};
use exact_units::system::{BridgeError, Side, bridge, const_bridge};
use exact_units::systems::{Gaussian, Natural, Si};
use exact_units::{BaseExp, Dim, Exp, MAX_TRUE_BASES, Magnitude, MarkerId, QuantityExpr, System};
use std::hash::DefaultHasher;

/// The crate's magnitude as the reference's factor.
pub(crate) fn factor_of(m: Magnitude) -> Factor {
    let mut f = Factor::one();
    for p in m.primes() {
        f.add(Key::Prime(u64::from(p.prime)), i32::from(p.exp));
    }
    let (p10, p2) = m.pow10_pow2_x2();
    f.add(Key::Prime(2), i32::from(p10) + i32::from(p2));
    f.add(Key::Prime(5), i32::from(p10));
    let s = m.symbols_x2();
    assert_eq!((s.ln2, s.ln10, s.g), (0, 0, 0), "{m:?}");
    f.add(Key::Pi, i32::from(s.pi));
    f.add(Key::Mu0, i32::from(s.mu0));
    f
}

/// An angle and a bit at exponents in halves, as a list of distinguished bases (sorted, none 0).
fn bases(rad: i8, bit: i8) -> &'static [BaseExp<'static>] {
    let mut list: Vec<BaseExp<'static>> = [(MarkerId::BIT, bit), (MarkerId::RADIAN, rad)]
        .into_iter()
        .filter(|(_, e)| *e != 0)
        .map(|(base, e)| BaseExp {
            base,
            exp: Exp::from_halves(e),
        })
        .collect();
    list.sort_by_key(|b| b.base);
    Vec::leak(list)
}

/// A dimension of a system of the crate (`None`: SI) with exponents in halves, by symbol.
#[expect(clippy::expect_used, reason = "a property fails by panicking")]
fn dim_in(
    sys: Option<Sys>,
    exps: &[i32],
    list: &'static [BaseExp<'static>],
) -> Option<Dim<'static>> {
    let (id, symbols) = match sys {
        None => (Si::ID, Si::BASES),
        Some(Sys::Natural) => (Natural::ID, Natural::BASES),
        Some(Sys::Gaussian) => (Gaussian::ID, Gaussian::BASES),
    };
    let mut parts = Vec::new();
    for (s, e) in symbols.iter().zip(exps) {
        if *e != 0 {
            parts.push((*s, Exp::from_halves(i8::try_from(*e).ok()?)));
        }
    }
    let d = Dim::checked_in_system(id, symbols, &parts, list).expect("the system's own bases");
    Some(d)
}

/// A side of its dimension's own quantity.
fn side(dim: Dim<'static>) -> Side<'static> {
    Side {
        dim,
        quantity: QuantityExpr::Derived(dim),
        rows_only_leaf: false,
    }
}

/// The bridge both ways, `const` and at run time agreeing.
fn both(from: Side<'static>, to: Side<'static>) -> Result<Magnitude, BridgeError> {
    let c = const_bridge(from, to);
    assert_eq!(
        c,
        bridge(from, to, &()),
        "const and run time: {:?} to {:?}",
        from.dim,
        to.dim
    );
    c
}

/// The SI dimension of exponents `exps` (halves) and an angle and a bit crosses into `sys` by the reference's rule
/// and back by its reciprocal; a target one half off is refused.
#[expect(clippy::expect_used, reason = "a property fails by panicking")]
#[expect(clippy::panic, reason = "a property fails by panicking")]
pub(crate) fn round_trip(sys: Sys, exps: [i8; MAX_TRUE_BASES], rad: i8, bit: i8) {
    let list = bases(rad, bit);
    let si = dim_in(None, &exps.map(i32::from), list).expect("halves of an i8");
    let Some((image, want, held)) = project(sys, exps) else {
        // the ampere in Gaussian: by no rule, and a row of the SI dimension's quantity (a current, a charge) wants the
        // row's own quantity on the far side, not dimension one
        let to = dim_in(Some(sys), &[], list).expect("dimension one");
        let r = both(side(si), side(to));
        assert!(
            matches!(
                r,
                Err(BridgeError::Unmapped | BridgeError::RowMismatch | BridgeError::Dimension)
            ),
            "{si:?} into {sys:?}: {r:?}"
        );
        return;
    };
    let Some(x) = dim_in(Some(sys), &image, list) else {
        return;
    };
    let frequency = exps == [-2, 0, 0, 0, 0, 0, 0] && list.is_empty();
    let there = both(side(si), side(x));
    let back = both(side(x), side(si));
    if sys == Sys::Natural && frequency {
        // E = hf and E = ħω differ by 2π: s⁻¹ alone is a frequency, which Natural's rule doesn't take
        assert_eq!(there, Err(BridgeError::NoRule));
        return;
    }
    // a factor past what a Magnitude holds is refused, both ways; the way back also where only its reciprocal is
    if !held {
        assert_eq!(
            (there, back),
            (Err(BridgeError::Factor), Err(BridgeError::Factor)),
            "{si:?}"
        );
        return;
    }
    if !want.recip().representable() {
        assert_eq!(back, Err(BridgeError::Factor), "{x:?}");
        return;
    }
    // a dimension that is one of Gaussian's rows' (statC, G, ..): as its own quantity, no row takes it, the rule does
    let (there, back) = match (there, back) {
        (Ok(a), Ok(b)) => (a, b),
        other => panic!("{si:?} and {x:?} in {sys:?}: {other:?}"),
    };
    assert_eq!(factor_of(there), want, "{si:?} into {sys:?}: {there:?}");
    assert_eq!(factor_of(back), want.recip(), "{x:?} back: {back:?}");
    assert_eq!(there.mul(back), Magnitude::ONE);
    // an image one half off on any base is another dimension
    for i in 0..image.len() {
        let mut off = image.clone();
        if let Some(e) = off.get_mut(i) {
            *e += 1;
        }
        let Some(wrong) = dim_in(Some(sys), &off, list) else {
            continue;
        };
        assert_eq!(
            both(side(si), side(wrong)),
            Err(BridgeError::Dimension),
            "{si:?} into {wrong:?}"
        );
    }
    let _ = (Exp::ZERO, si.exps());
}

/// The hash of a value.
fn hash_of<T: core::hash::Hash>(x: &T) -> u64 {
    BuildHasherDefault::<DefaultHasher>::default().hash_one(x)
}

/// Distinguished bases alone are the same dimension in SI, Natural and Gaussian: `==`, `const_same` and `Hash`
/// agree. With a true exponent of `e` halves on each system's first base, the three differ.
#[expect(clippy::expect_used, reason = "a property fails by panicking")]
pub(crate) fn neutral(rad: i8, bit: i8, e: i8) {
    let list = bases(rad, bit);
    let dims = [None, Some(Sys::Natural), Some(Sys::Gaussian)]
        .map(|s| dim_in(s, &[], list).expect("no true exponent"));
    for a in dims {
        for b in dims {
            assert_eq!(a, b, "{a:?} {b:?}");
            assert_eq!(a.const_same(b), Some(true));
            assert_eq!(hash_of(&a), hash_of(&b), "{a:?} {b:?}");
            assert!(!a.has_true_exps());
        }
    }
    if e == 0 {
        return;
    }
    let first = [None, Some(Sys::Natural), Some(Sys::Gaussian)]
        .map(|s| dim_in(s, &[i32::from(e)], list).expect("an exponent of an i8"));
    for (i, a) in first.iter().enumerate() {
        for (j, b) in first.iter().enumerate() {
            assert_eq!(a == b, i == j, "{a:?} {b:?}");
            assert_eq!(a.const_same(*b), Some(i == j));
        }
        assert_eq!(a.exps().first().map(|x| x.halves()), Some(e));
    }
}

/// A dimension of each system goes through the wire and back, to the same bytes, and its owned mirror writes them too.
#[cfg(all(feature = "std", feature = "shrink_wrap"))]
#[expect(clippy::expect_used, reason = "a property fails by panicking")]
pub(crate) fn wire(sys: u8, exps: [i8; MAX_TRUE_BASES], rad: i8, bit: i8) {
    use shrink_wrap::prelude::*;
    let sys = match sys % 3 {
        0 => None,
        1 => Some(Sys::Natural),
        _ => Some(Sys::Gaussian),
    };
    let list = bases(rad, bit);
    let d = dim_in(sys, &exps.map(i32::from), list).expect("halves of an i8");
    let mut buf = vec![0u8; 1 << 12];
    let bytes = d.to_ww_bytes(&mut buf).expect("encodes").to_vec();
    let decoded = Dim::from_ww_bytes(&bytes).expect("decodes");
    assert_eq!(decoded, d);
    assert_eq!(decoded.system().id(), d.system().id());
    let mut buf2 = vec![0u8; 1 << 12];
    assert_eq!(
        decoded.to_ww_bytes(&mut buf2).expect("encodes"),
        bytes.as_slice()
    );
    let owned = exact_units::DimOwned::from(d);
    assert_eq!(owned.to_ww_bytes_owned().expect("encodes"), bytes);
    let back = exact_units::DimOwned::from_ww_bytes_owned(&bytes).expect("decodes");
    assert_eq!(back, owned);
    assert_eq!(back.as_dim(), d);
    assert_eq!(back.as_dim().system().id(), d.system().id());
}

#[cfg(all(test, feature = "std", feature = "shrink_wrap"))]
mod defs {
    use exact_units::systems::{Gaussian, Natural, Si};
    use exact_units::{System, SystemDef, SystemDefOwned};
    use shrink_wrap::prelude::*;

    /// A built-in system's definition goes through the wire, borrowed and owned, to the same bytes, and reads back
    /// the same bases, rule, rows (by id) and constants.
    #[expect(clippy::expect_used, reason = "a test fails by panicking")]
    fn def(d: &SystemDef<'static>) {
        let mut buf = vec![0u8; 1 << 14];
        let bytes = d.to_ww_bytes(&mut buf).expect("encodes").to_vec();
        let back = SystemDef::from_ww_bytes(&bytes).expect("decodes");
        let mut buf2 = vec![0u8; 1 << 14];
        assert_eq!(
            back.to_ww_bytes(&mut buf2).expect("encodes"),
            bytes.as_slice()
        );
        assert_eq!(back.id, d.id);
        // base_count is const and reads a list from bytes as MAX_TRUE_BASES; the list itself has its length
        let n = d.base_count();
        assert_eq!(back.bases.len(), n);
        for i in 0..n {
            assert_eq!(back.symbol(i), d.symbol(i));
        }
        let rules = |s: &SystemDef<'_>| {
            s.rules()
                .map(|r| {
                    r.map(|r| {
                        (
                            r.exps.iter().filter_map(Result::ok).collect::<Vec<_>>(),
                            r.factor,
                        )
                    })
                })
                .collect::<Vec<_>>()
        };
        assert_eq!(rules(&back), rules(d));
        let rows = |s: &SystemDef<'_>| {
            s.rows()
                .map(|r| (r.si.id().to_string(), r.here.id().to_string(), r.factor))
                .collect::<Vec<_>>()
        };
        assert_eq!(rows(&back), rows(d));
        let ones = |s: &SystemDef<'_>| {
            s.ones()
                .map(|o| (o.symbol.to_string(), o.si))
                .collect::<Vec<_>>()
        };
        assert_eq!(ones(&back), ones(d));
        assert_eq!(back.no_rule.len(), d.no_rule.len());
        let owned = SystemDefOwned::from_ww_bytes_owned(&bytes).expect("decodes");
        assert_eq!(owned.to_ww_bytes_owned().expect("encodes"), bytes);
    }

    #[test]
    fn builtin_defs() {
        def(Si::DEF);
        def(Natural::DEF);
        def(Gaussian::DEF);
    }
}

#[cfg(not(any(kani, fuzzing)))]
mod tests {
    use super::super::reference::systems::Sys;
    use super::MAX_TRUE_BASES;
    use proptest::prelude::*;

    /// Exponents in halves: mostly 0 so a dimension has a few bases, whole more often than half.
    fn halves() -> impl Strategy<Value = i8> {
        prop_oneof![6 => Just(0i8), 3 => (-2i8..=2).prop_map(|e| e * 2), 2 => -5i8..=5]
    }

    fn system() -> impl Strategy<Value = Sys> {
        prop_oneof![Just(Sys::Natural), Just(Sys::Gaussian)]
    }

    proptest! {
        #[test]
        fn round_trips(sys in system(), exps in proptest::array::uniform7(halves()), rad in -2i8..=2, bit in -2i8..=2) {
            super::round_trip(sys, exps, rad, bit);
        }

        #[test]
        fn neutral(rad in -3i8..=3, bit in -3i8..=3, e in -4i8..=4) {
            super::neutral(rad, bit, e);
        }
    }

    #[cfg(all(feature = "std", feature = "shrink_wrap"))]
    proptest! {
        #[test]
        fn wire(sys: u8, exps in proptest::array::uniform7(halves()), rad in -2i8..=2, bit in -2i8..=2) {
            super::wire(sys, exps, rad, bit);
        }
    }

    /// Every base alone at every exponent from −3 to 3 in halves, into both systems: the frequency, the ampere and
    /// the half powers each met at least once, whatever proptest draws.
    #[test]
    fn every_single_base() {
        for sys in [Sys::Natural, Sys::Gaussian] {
            for i in 0..MAX_TRUE_BASES {
                for e in -6i8..=6 {
                    let mut exps = [0i8; MAX_TRUE_BASES];
                    if let Some(x) = exps.get_mut(i) {
                        *x = e;
                    }
                    super::round_trip(sys, exps, 0, 0);
                    super::round_trip(sys, exps, 2, -1);
                }
            }
        }
    }
}
