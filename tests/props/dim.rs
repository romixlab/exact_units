//! Property 18: a dimension's distinguished bases, read through the `const` walk, add up as integers do. A tree of
//! products, quotients, strips and lists (bases repeated, in any order, with 0) gives every base the exponent a plain
//! sum gives it (`const_exp_of`, `exp_of`), is one exactly when every sum is 0, is the same as another tree exactly
//! when the sums agree (`const_same`, `same`, `==`), is the `DimOwned` its arithmetic builds step by step, and goes
//! through the wire to the same bytes and back. A step whose sum leaves an exponent (±63½) is refused by both.
//! A pair of single bases is also proved for every exponent by Kani; the trees allocate and compare names, which it
//! can't take in reasonable time.
#![expect(
    clippy::arithmetic_side_effects,
    reason = "a test computes with the values it chose, which fit; an overflow fails the test"
)]

use exact_units::{BaseExp, Dim, Exp, MAX_TRUE_BASES, MarkerId};

/// The bases the trees draw from: the two of this crate with reserved spellings, one name in two crates (sorted by
/// crate first), and one more.
pub(crate) const NAMES: [MarkerId<'static>; 5] = [
    MarkerId::RADIAN,
    MarkerId::BIT,
    MarkerId {
        crate_name: "exact_units",
        name: "Sample",
    },
    MarkerId {
        crate_name: "my_api",
        name: "Sample",
    },
    MarkerId {
        crate_name: "a",
        name: "Z",
    },
];

fn name(i: u8) -> MarkerId<'static> {
    NAMES
        .get(usize::from(i) % NAMES.len())
        .copied()
        .unwrap_or(MarkerId::RADIAN)
}

/// Two single bases `a` and `b` (indices into [NAMES], exponents in halves) multiplied or divided, in `const` code:
/// the result has the exponent `ea ± eb` of a base both name, `ea` and `±eb` of two bases, refused exactly when one of
/// those leaves an `i8` of halves; its true exponents are `exps`, untouched.
#[expect(clippy::expect_used, reason = "a property fails by panicking")]
pub(crate) fn pair(a: (u8, i8), b: (u8, i8), exps: [i8; MAX_TRUE_BASES], div: bool) {
    let (na, nb) = (name(a.0), name(b.0));
    let base = |n, h| {
        Dim::checked_with_list(
            exps.map(Exp::from_halves),
            core::slice::from_ref(Box::leak(Box::new(BaseExp {
                base: n,
                exp: Exp::from_halves(h),
            }))),
        )
        .expect("one base always fits")
    };
    let (da, db) = (base(na, a.1), base(nb, b.1));
    let sign = if div { -1 } else { 1 };
    let true_sum: Option<Vec<i8>> = exps
        .iter()
        .map(|e| i8::try_from(i32::from(*e) * (1 + sign)).ok())
        .collect();
    let (sa, sb) = (i32::from(a.1), sign * i32::from(b.1));
    let base_sums = if na == nb {
        vec![(na, sa + sb)]
    } else {
        vec![(na, sa), (nb, sb)]
    };
    let fits = true_sum.is_some() && base_sums.iter().all(|(_, s)| i8::try_from(*s).is_ok());
    let d = if div {
        da.const_div(&db)
    } else {
        da.const_mul(&db)
    };
    let Some(d) = d else {
        assert!(!fits, "{a:?} {b:?} {exps:?} {div}: refused, yet fits");
        return;
    };
    assert!(fits, "{a:?} {b:?} {exps:?} {div}: fits, yet refused");
    for (n, s) in base_sums {
        let want = i8::try_from(s).map(Exp::from_halves).ok();
        assert_eq!(d.const_exp_of(n), want, "{n}");
    }
    assert_eq!(
        d.const_is_one(),
        Some(d.exps().iter().all(|e| e.is_zero()) && d.has_no_base())
    );
}

/// A tree of bases as plain data, for building a [Dim] from proptest's values.
#[derive(Clone, Debug)]
pub(crate) enum Tree {
    /// A list in `const` data: true exponents and bases (an index into [NAMES]) in halves, repeats and 0 allowed.
    Leaf([i8; MAX_TRUE_BASES], Vec<(u8, i8)>),
    /// A product.
    Mul(Box<Tree>, Box<Tree>),
    /// A quotient.
    Div(Box<Tree>, Box<Tree>),
    /// Without one base.
    Strip(Box<Tree>, u8),
}

/// The exponents of a dimension by plain sums in `i32`: the true ones, then each base with a sum that isn't 0.
type Sums = (
    [i32; MAX_TRUE_BASES],
    std::collections::BTreeMap<(&'static str, &'static str), i32>,
);

/// What a tree adds up to, `None` when a step leaves an `i8` of halves (each step is checked, as each is built).
fn reference(t: &Tree) -> Option<Sums> {
    let fits = |s: &Sums| {
        s.0.iter()
            .chain(s.1.values())
            .all(|e| i8::try_from(*e).is_ok())
    };
    let combine = |a: Sums, b: Sums, sign: i32| {
        let mut out = a;
        for (x, y) in out.0.iter_mut().zip(b.0) {
            *x += sign * y;
        }
        for (k, v) in b.1 {
            *out.1.entry(k).or_insert(0) += sign * v;
        }
        out.1.retain(|_, v| *v != 0);
        out
    };
    let s = match t {
        Tree::Leaf(exps, list) => {
            let mut bases = std::collections::BTreeMap::new();
            for (i, h) in list {
                let n = name(*i);
                *bases.entry((n.crate_name, n.name)).or_insert(0) += i32::from(*h);
            }
            bases.retain(|_, v| *v != 0);
            (exps.map(i32::from), bases)
        }
        Tree::Mul(a, b) => combine(reference(a)?, reference(b)?, 1),
        Tree::Div(a, b) => combine(reference(a)?, reference(b)?, -1),
        Tree::Strip(a, i) => {
            let mut s = reference(a)?;
            let n = name(*i);
            s.1.remove(&(n.crate_name, n.name));
            s
        }
    };
    fits(&s).then_some(s)
}

/// The tree built with the `const` API, each node leaked so that the next borrows it for `'static`; `None` where a
/// step is refused.
fn build(t: &Tree) -> Option<&'static Dim<'static>> {
    let leak = |d: Dim<'static>| -> &'static Dim<'static> { Box::leak(Box::new(d)) };
    Some(leak(match t {
        Tree::Leaf(exps, list) => {
            let list: Vec<BaseExp<'static>> = list
                .iter()
                .map(|(i, h)| BaseExp {
                    base: name(*i),
                    exp: Exp::from_halves(*h),
                })
                .collect();
            Dim::checked_with_list(exps.map(Exp::from_halves), list.leak())?
        }
        Tree::Mul(a, b) => build(a)?.const_mul(build(b)?)?,
        Tree::Div(a, b) => build(a)?.const_div(build(b)?)?,
        Tree::Strip(a, i) => build(a)?.strip(name(*i)),
    }))
}

/// The tree's [DimOwned](exact_units::DimOwned), built by its run-time arithmetic step by step.
#[cfg(all(feature = "std", feature = "shrink_wrap"))]
fn owned(t: &Tree) -> Option<exact_units::DimOwned> {
    use exact_units::DimOwned;
    Some(match t {
        Tree::Leaf(..) => DimOwned::from(*build(t)?),
        Tree::Mul(a, b) => owned(a)?.checked_mul(&owned(b)?)?,
        Tree::Div(a, b) => owned(a)?.checked_div(&owned(b)?)?,
        Tree::Strip(a, i) => owned(a)?.strip(name(*i)),
    })
}

/// A tree adds up as its plain sums do, through every reader, and is the same as another exactly when the sums agree.
#[expect(clippy::expect_used, reason = "a property fails by panicking")]
pub(crate) fn tree(t: &Tree, other: &Tree) {
    let want = reference(t);
    let Some(d) = build(t) else {
        assert_eq!(want, None, "{t:?} refused, yet adds up");
        return;
    };
    let want = want.expect("built, so it adds up");
    assert_eq!(d.exps().map(|e| i32::from(e.halves())), want.0, "{t:?}");
    for n in NAMES {
        let w = want.1.get(&(n.crate_name, n.name)).copied().unwrap_or(0);
        let w = Exp::from_halves(i8::try_from(w).expect("fits"));
        assert_eq!(d.const_exp_of(n), Some(w), "{t:?} {n}");
        assert_eq!(d.exp_of(n), w, "{t:?} {n}");
    }
    let mut seen = Vec::new();
    d.for_each_base(|b| seen.push(((b.base.crate_name, b.base.name), i32::from(b.exp.halves()))));
    let sorted: Vec<_> = want.1.iter().map(|(k, v)| (*k, *v)).collect();
    assert_eq!(seen, sorted, "{t:?}: the bases in canonical order");
    let is_one = want.0.iter().all(|e| *e == 0) && want.1.is_empty();
    assert_eq!(d.const_is_one(), Some(is_one), "{t:?}");
    assert_eq!(d.is_one(), is_one, "{t:?}");

    if let (Some(o), Some(ow)) = (build(other), reference(other)) {
        let same = ow == want;
        assert_eq!(d.const_same(*o), Some(same), "{t:?} vs {other:?}");
        assert_eq!(d.same(*o), same, "{t:?} vs {other:?}");
        assert_eq!(*d == *o, same, "{t:?} vs {other:?}");
    }

    #[cfg(all(feature = "std", feature = "shrink_wrap"))]
    {
        use exact_units::DimOwned;
        use shrink_wrap::prelude::*;
        let o = owned(t).expect("its steps are the same as the tree's");
        assert_eq!(o, *d, "{t:?}");
        assert_eq!(o.to_string(), d.to_string(), "{t:?}");
        let mut buf = [0u8; 512];
        let bytes = d.to_ww_bytes(&mut buf).expect("encodes").to_vec();
        assert_eq!(o.to_ww_bytes_owned().expect("encodes"), bytes, "{t:?}");
        assert_eq!(Dim::from_ww_bytes(&bytes).expect("decodes"), *d, "{t:?}");
        assert_eq!(
            DimOwned::from_ww_bytes_owned(&bytes).expect("decodes"),
            o,
            "{t:?}"
        );
    }
}

#[cfg(not(any(kani, fuzzing)))]
mod tests {
    use super::{MAX_TRUE_BASES, Tree};
    use proptest::prelude::*;

    /// Exponents in halves, mostly small, sometimes at the ends where a sum leaves an `i8`.
    fn halves() -> impl Strategy<Value = i8> {
        prop_oneof![8 => -4i8..=4, 1 => any::<i8>()]
    }

    fn leaf() -> impl Strategy<Value = Tree> {
        (
            prop_oneof![
                3 => Just([0i8; MAX_TRUE_BASES]),
                1 => proptest::array::uniform7(halves()),
            ],
            proptest::collection::vec((0u8..5, halves()), 0..5),
        )
            .prop_map(|(e, l)| Tree::Leaf(e, l))
    }

    fn tree() -> impl Strategy<Value = Tree> {
        leaf().prop_recursive(4, 16, 2, |inner| {
            prop_oneof![
                (inner.clone(), inner.clone())
                    .prop_map(|(a, b)| Tree::Mul(Box::new(a), Box::new(b))),
                (inner.clone(), inner.clone())
                    .prop_map(|(a, b)| Tree::Div(Box::new(a), Box::new(b))),
                (inner, 0u8..5).prop_map(|(a, i)| Tree::Strip(Box::new(a), i)),
            ]
        })
    }

    proptest! {
        #[test]
        fn trees(t in tree(), other in tree()) {
            super::tree(&t, &other);
        }

        /// A tree and the same factors in another order and grouping: the same dimension.
        #[test]
        fn reordered(a in tree(), b in tree(), c in tree()) {
            let left = Tree::Mul(Box::new(Tree::Mul(Box::new(a.clone()), Box::new(b.clone()))), Box::new(c.clone()));
            let right = Tree::Mul(Box::new(c), Box::new(Tree::Mul(Box::new(b), Box::new(a))));
            super::tree(&left, &right);
        }

        #[test]
        fn pairs(a in (0u8..5, any::<i8>()), b in (0u8..5, any::<i8>()), e in proptest::array::uniform7(any::<i8>()), div: bool) {
            super::pair(a, b, e, div);
        }
    }
}

#[cfg(kani)]
mod proofs {
    /// Every pair of single bases of three names, every exponent, both operations.
    #[kani::proof]
    #[kani::unwind(13)]
    fn pairs() {
        let a = (kani::any_where(|i: &u8| *i < 3), kani::any());
        let b = (kani::any_where(|i: &u8| *i < 3), kani::any());
        super::pair(a, b, [0; super::MAX_TRUE_BASES], kani::any());
    }
}
