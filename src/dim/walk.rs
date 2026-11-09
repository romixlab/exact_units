//! Walks over a [Bases] tree: the exponent of one base, the bases in canonical order, sameness. Each has a `const`
//! form (`c_*`), for the variants `const` data holds, and a run-time form for every variant.
//!
//! No walk collects: a base's exponent is a sum over the tree, and the canonical order is found by selection (the
//! smallest base above the last one), so there is no buffer and no count limit. O(n²) in the leaves, which are few.

use super::{BaseExp, Bases, Exp, cmp_ids};
use crate::unit::{MarkerId, UnitExpr};
use core::cmp::Ordering;

/// `==` on two names in `const` code.
pub(crate) const fn same_str(a: &str, b: &str) -> bool {
    matches!(cmp_names(a, b), Ordering::Equal)
}

/// Byte-wise order of two names, in `const` code.
#[expect(
    clippy::indexing_slicing,
    reason = "i < the shorter length, the loop bound"
)]
#[expect(
    clippy::arithmetic_side_effects,
    reason = "i < the shorter length, the loop bound, so i + 1 fits usize"
)]
pub(crate) const fn cmp_names(a: &str, b: &str) -> Ordering {
    let (a, b) = (a.as_bytes(), b.as_bytes());
    let mut i = 0;
    while i < a.len() && i < b.len() {
        if a[i] != b[i] {
            return if a[i] < b[i] {
                Ordering::Less
            } else {
                Ordering::Greater
            };
        }
        i += 1;
    }
    if a.len() < b.len() {
        Ordering::Less
    } else if a.len() > b.len() {
        Ordering::Greater
    } else {
        Ordering::Equal
    }
}

const fn same_id(a: MarkerId<'_>, b: MarkerId<'_>) -> bool {
    matches!(cmp_ids(a, b), Ordering::Equal)
}

/// Whether `x` is after `after` (anything is after `None`).
const fn is_after(x: MarkerId<'_>, after: Option<MarkerId<'_>>) -> bool {
    match after {
        None => true,
        Some(a) => matches!(cmp_ids(x, a), Ordering::Greater),
    }
}

/// The smaller of the best so far and a candidate that is after `after`.
const fn pick<'i>(
    best: Option<MarkerId<'i>>,
    x: MarkerId<'i>,
    after: Option<MarkerId<'_>>,
) -> Option<MarkerId<'i>> {
    if !is_after(x, after) {
        return best;
    }
    match best {
        Some(b) if !matches!(cmp_ids(x, b), Ordering::Less) => Some(b),
        _ => Some(x),
    }
}

/// The halves of `e` times `n`, `None` past an `i32`.
const fn times(e: Option<i32>, n: i32) -> Option<i32> {
    match e {
        Some(e) => e.checked_mul(n),
        None => None,
    }
}

const fn add(a: Option<i32>, b: Option<i32>, sign: i32) -> Option<i32> {
    match (a, times(b, sign)) {
        (Some(a), Some(b)) => a.checked_add(b),
        _ => None,
    }
}

/// The square root of halves: whole halves only from an even count.
const fn root(e: Option<i32>) -> Option<i32> {
    match e {
        Some(e) if e % 2 == 0 => Some(e / 2),
        _ => None,
    }
}

// ---- const walks: the variants const data holds; None for the run-time ones (or a sum that doesn't add up) ----

/// The exponent of `base` in halves.
pub(crate) const fn c_exp_of(b: &Bases<'_>, base: MarkerId<'_>) -> Option<i32> {
    match b {
        Bases::Empty => Some(0),
        Bases::Single(x) => Some(if same_id(x.base, base) {
            x.exp.wide()
        } else {
            0
        }),
        Bases::List(list) => match c_slice(list) {
            Some(slice) => c_exp_in_slice(slice, base),
            None => None,
        },
        Bases::Unit(e) => c_exp_in_expr(e, base),
        Bases::Mul(x, y) => add(c_exp_of(x, base), c_exp_of(y, base), 1),
        Bases::Div(x, y) => add(c_exp_of(x, base), c_exp_of(y, base), -1),
        Bases::Without(x, w) => {
            if same_id(*w, base) {
                Some(0)
            } else {
                c_exp_of(x, base)
            }
        }
        Bases::Of(_) => None,
        #[cfg(all(feature = "std", feature = "shrink_wrap"))]
        Bases::Owned(_) => None,
    }
}

/// The slice of a list in `const` data, `None` for one read from the wire.
#[cfg(feature = "shrink_wrap")]
const fn c_slice<'a, 'i>(
    list: &'a shrink_wrap::prelude::RefVec<'i, BaseExp<'i>>,
) -> Option<&'a [BaseExp<'i>]> {
    match list {
        shrink_wrap::prelude::RefVec::Slice { slice } => Some(slice),
        shrink_wrap::prelude::RefVec::Buf { .. } => None,
    }
}

#[cfg(not(feature = "shrink_wrap"))]
#[expect(
    clippy::unnecessary_wraps,
    reason = "the same signature as with shrink_wrap, where a list read from the wire has no slice"
)]
const fn c_slice<'a, 'i>(list: &'a &'i [BaseExp<'i>]) -> Option<&'a [BaseExp<'i>]> {
    Some(list)
}

#[expect(clippy::indexing_slicing, reason = "i < slice.len(), the loop bound")]
#[expect(clippy::arithmetic_side_effects, reason = "i < slice.len()")]
const fn c_exp_in_slice(slice: &[BaseExp<'_>], base: MarkerId<'_>) -> Option<i32> {
    let mut sum = Some(0);
    let mut i = 0;
    while i < slice.len() {
        if same_id(slice[i].base, base) {
            sum = add(sum, Some(slice[i].exp.wide()), 1);
        }
        i += 1;
    }
    sum
}

/// The exponent of `base` in a unit expression's named units.
pub(crate) const fn c_exp_in_expr(e: &UnitExpr, base: MarkerId<'_>) -> Option<i32> {
    match e {
        UnitExpr::Named(def) => c_exp_of(&def.dim.bases, base),
        UnitExpr::Mul(a, b) => add(c_exp_in_expr(a, base), c_exp_in_expr(b, base), 1),
        UnitExpr::Div(a, b) => add(c_exp_in_expr(a, base), c_exp_in_expr(b, base), -1),
        #[expect(
            clippy::as_conversions,
            reason = "const fn: From is not const on stable; i8 into i32 is exact"
        )]
        UnitExpr::Pow(a, n) => times(c_exp_in_expr(a, base), *n as i32),
        UnitExpr::Sqrt(a) => root(c_exp_in_expr(a, base)),
    }
}

/// The smallest base of the tree after `after`, the best so far given: the outer `None` for a run-time variant.
#[expect(
    clippy::option_option,
    reason = "the outer None: bases const code can't read; the inner None: past the last base"
)]
const fn c_next<'i>(
    b: &Bases<'i>,
    after: Option<MarkerId<'_>>,
    best: Option<MarkerId<'i>>,
) -> Option<Option<MarkerId<'i>>> {
    match b {
        Bases::Empty => Some(best),
        Bases::Single(x) => Some(pick(best, x.base, after)),
        Bases::List(list) => match c_slice(list) {
            Some(slice) => Some(c_next_in_slice(slice, after, best)),
            None => None,
        },
        Bases::Unit(e) => c_next_in_expr(e, after, best),
        Bases::Mul(x, y) | Bases::Div(x, y) => match c_next(x, after, best) {
            Some(best) => c_next(y, after, best),
            None => None,
        },
        Bases::Without(x, _) => c_next(x, after, best),
        Bases::Of(_) => None,
        #[cfg(all(feature = "std", feature = "shrink_wrap"))]
        Bases::Owned(_) => None,
    }
}

#[expect(clippy::indexing_slicing, reason = "i < slice.len(), the loop bound")]
#[expect(clippy::arithmetic_side_effects, reason = "i < slice.len()")]
const fn c_next_in_slice<'i>(
    slice: &[BaseExp<'i>],
    after: Option<MarkerId<'_>>,
    mut best: Option<MarkerId<'i>>,
) -> Option<MarkerId<'i>> {
    let mut i = 0;
    while i < slice.len() {
        best = pick(best, slice[i].base, after);
        i += 1;
    }
    best
}

#[expect(
    clippy::option_option,
    reason = "the outer None: bases const code can't read; the inner None: past the last base"
)]
const fn c_next_in_expr<'i>(
    e: &UnitExpr,
    after: Option<MarkerId<'_>>,
    best: Option<MarkerId<'i>>,
) -> Option<Option<MarkerId<'i>>> {
    match e {
        UnitExpr::Named(def) => c_next(&def.dim.bases, after, best),
        UnitExpr::Mul(a, b) | UnitExpr::Div(a, b) => match c_next_in_expr(a, after, best) {
            Some(best) => c_next_in_expr(b, after, best),
            None => None,
        },
        UnitExpr::Pow(a, _) | UnitExpr::Sqrt(a) => c_next_in_expr(a, after, best),
    }
}

/// The first base after `after` whose exponent isn't 0: the outer `None` for a run-time variant or a sum that
/// doesn't add up, the inner `None` past the last.
#[expect(
    clippy::option_option,
    reason = "the outer None: bases const code can't read; the inner None: past the last base"
)]
pub(crate) const fn c_next_nonzero<'i>(
    b: &Bases<'i>,
    mut after: Option<MarkerId<'i>>,
) -> Option<Option<BaseExp<'i>>> {
    loop {
        let x = match c_next(b, after, None) {
            Some(Some(x)) => x,
            Some(None) => return Some(None),
            None => return None,
        };
        let exp = match c_exp_of(b, x) {
            Some(e) => match Exp::of_i32(e) {
                Some(e) => e,
                None => return None,
            },
            None => return None,
        };
        if !exp.is_zero() {
            return Some(Some(BaseExp { base: x, exp }));
        }
        after = Some(x);
    }
}

/// Whether every base's exponent is whole halves within ±63½, in `const` code: `false` also where the bases are
/// read at run time.
pub(crate) const fn c_adds_up(b: &Bases<'_>) -> bool {
    let mut after = None;
    loop {
        match c_next_nonzero(b, after) {
            Some(Some(x)) => after = Some(x.base),
            Some(None) => return true,
            None => return false,
        }
    }
}

/// Whether two trees give every base the same exponent: the outer `None` like [c_next_nonzero].
pub(crate) const fn c_same(a: &Bases<'_>, b: &Bases<'_>) -> Option<bool> {
    // every base of either side, in order
    let mut after: Option<MarkerId<'_>> = None;
    loop {
        let x = match (c_next(a, after, None), c_next(b, after, None)) {
            (Some(None), Some(None)) => return Some(true),
            (Some(Some(x)), Some(None)) | (Some(None), Some(Some(x))) => x,
            (Some(Some(x)), Some(Some(y))) => {
                if matches!(cmp_ids(x, y), Ordering::Greater) {
                    y
                } else {
                    x
                }
            }
            _ => return None,
        };
        match (c_exp_of(a, x), c_exp_of(b, x)) {
            (Some(ea), Some(eb)) => {
                if ea != eb {
                    return Some(false);
                }
            }
            _ => return None,
        }
        after = Some(x);
    }
}

// ---- run-time walks: every variant ----

/// The exponent of `base` in halves, `None` where it doesn't add up or a list doesn't decode.
pub(crate) fn exp_of(b: &Bases<'_>, base: MarkerId<'_>) -> Option<i32> {
    match b {
        Bases::List(list) => {
            let mut sum = 0i32;
            for x in list_iter(list) {
                let x = x?;
                if x.base == base {
                    sum = sum.checked_add(x.exp.wide())?;
                }
            }
            Some(sum)
        }
        Bases::Unit(e) => exp_in_expr(e, base),
        Bases::Mul(x, y) => exp_of(x, base)?.checked_add(exp_of(y, base)?),
        Bases::Div(x, y) => exp_of(x, base)?.checked_sub(exp_of(y, base)?),
        Bases::Without(x, w) => {
            if *w == base {
                Some(0)
            } else {
                exp_of(x, base)
            }
        }
        Bases::Of(s) => s.exp_of(base),
        #[cfg(all(feature = "std", feature = "shrink_wrap"))]
        Bases::Owned(list) => Some(
            list.iter()
                .find(|x| x.base.as_id() == base)
                .map_or(0, |x| x.exp.wide()),
        ),
        Bases::Empty | Bases::Single(_) => c_exp_of(b, base),
    }
}

fn exp_in_expr(e: &UnitExpr, base: MarkerId<'_>) -> Option<i32> {
    match e {
        UnitExpr::Named(def) => exp_of(&def.dim.bases, base),
        UnitExpr::Mul(a, b) => exp_in_expr(a, base)?.checked_add(exp_in_expr(b, base)?),
        UnitExpr::Div(a, b) => exp_in_expr(a, base)?.checked_sub(exp_in_expr(b, base)?),
        UnitExpr::Pow(a, n) => exp_in_expr(a, base)?.checked_mul(i32::from(*n)),
        UnitExpr::Sqrt(a) => root(exp_in_expr(a, base)),
    }
}

/// The elements of a list, decoded on the way for one read from the wire.
#[cfg(feature = "shrink_wrap")]
fn list_iter<'a, 'i>(
    list: &'a shrink_wrap::prelude::RefVec<'i, BaseExp<'i>>,
) -> impl Iterator<Item = Option<BaseExp<'i>>> + 'a {
    list.iter().map(Result::ok)
}

#[cfg(not(feature = "shrink_wrap"))]
fn list_iter<'a, 'i>(
    list: &'a &'i [BaseExp<'i>],
) -> impl Iterator<Item = Option<BaseExp<'i>>> + 'a {
    list.iter().copied().map(Some)
}

/// Calls `f` with every base the tree names, as often as it occurs. `false` when a list doesn't decode.
pub(crate) fn each_leaf<'i>(b: &Bases<'i>, f: &mut dyn FnMut(MarkerId<'i>)) -> bool {
    match b {
        Bases::Empty => true,
        Bases::Single(x) => {
            f(x.base);
            true
        }
        Bases::List(list) => {
            for x in list_iter(list) {
                let Some(x) = x else { return false };
                f(x.base);
            }
            true
        }
        Bases::Unit(e) => each_leaf_in_expr(e, f),
        Bases::Mul(x, y) | Bases::Div(x, y) => each_leaf(x, f) && each_leaf(y, f),
        Bases::Without(x, w) => each_leaf(x, &mut |b| {
            if b != *w {
                f(b);
            }
        }),
        Bases::Of(s) => s.each_base(f),
        #[cfg(all(feature = "std", feature = "shrink_wrap"))]
        Bases::Owned(list) => {
            for x in *list {
                f(x.base.as_id());
            }
            true
        }
    }
}

fn each_leaf_in_expr<'i>(e: &UnitExpr, f: &mut dyn FnMut(MarkerId<'i>)) -> bool {
    match e {
        UnitExpr::Named(def) => each_leaf(&def.dim.bases, f),
        UnitExpr::Mul(a, b) | UnitExpr::Div(a, b) => {
            each_leaf_in_expr(a, f) && each_leaf_in_expr(b, f)
        }
        UnitExpr::Pow(a, _) | UnitExpr::Sqrt(a) => each_leaf_in_expr(a, f),
    }
}

/// The smallest base after `after`; `Err` when a list doesn't decode.
fn next<'i>(b: &Bases<'i>, after: Option<MarkerId<'_>>) -> Result<Option<MarkerId<'i>>, ()> {
    let mut best = None;
    if each_leaf(b, &mut |x| best = pick(best, x, after)) {
        Ok(best)
    } else {
        Err(())
    }
}

/// Calls `f` with every base whose exponent isn't 0, sorted by crate and name. `false` where the bases don't add up
/// (a list that doesn't decode, a root of an odd count of halves, an exponent past ±63½): `f` has then seen those
/// before the first that doesn't.
pub(crate) fn each_canonical<'i>(b: &Bases<'i>, f: &mut dyn FnMut(BaseExp<'i>)) -> bool {
    let mut after = None;
    loop {
        let Ok(next) = next(b, after) else {
            return false;
        };
        let Some(x) = next else { return true };
        let Some(exp) = exp_of(b, x).and_then(Exp::of_i32) else {
            return false;
        };
        if !exp.is_zero() {
            f(BaseExp { base: x, exp });
        }
        after = Some(x);
    }
}

/// Whether two trees give every base the same exponent; `false` when either doesn't add up.
pub(crate) fn same(a: &Bases<'_>, b: &Bases<'_>) -> bool {
    if let Some(same) = c_same(a, b) {
        return same;
    }
    let mut after: Option<MarkerId<'_>> = None;
    loop {
        let (Ok(x), Ok(y)) = (next(a, after), next(b, after)) else {
            return false;
        };
        let x = match (x, y) {
            (None, None) => return true,
            (Some(x), None) | (None, Some(x)) => x,
            (Some(x), Some(y)) => x.min(y),
        };
        match (exp_of(a, x), exp_of(b, x)) {
            (Some(ea), Some(eb)) if ea == eb => {}
            _ => return false,
        }
        after = Some(x);
    }
}
