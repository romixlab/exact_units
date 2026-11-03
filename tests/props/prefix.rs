//! Properties of integer arithmetic across prefixes chosen at run time ([PrefixRatio]): conversions against the
//! exact quotient in `i128`, rounded and settled as the reference does, the floor bracketed by the exact comparison,
//! and the comparison against cross-multiplication. The kernel behind them has no float and no 128-bit type; these
//! say that it gives the exact answer anyway.
#![expect(
    clippy::arithmetic_side_effects,
    reason = "a test computes with the values it chose, which fit; an overflow fails the test"
)]

use super::reference::{round_div, settle};
use core::cmp::Ordering;
use exact_units::num::Int;
use exact_units::{FullScaleBits, Overflow, Pow2Exp, Pow10Exp, Prefix, PrefixRatio, Rounding};

/// The prefix of family `which` (0 named, 1 `Pow10`, 2 `Pow2`, 3 `FullScale`) with `param`, if there is one.
pub(crate) fn prefix(which: u8, param: i8) -> Option<Prefix> {
    match which % 4 {
        0 => Prefix::ALL
            .get(usize::from(param.unsigned_abs()) % Prefix::ALL.len())
            .copied(),
        1 => Pow10Exp::new(param).map(Prefix::Pow10),
        2 => Pow2Exp::new(param).map(Prefix::Pow2),
        _ => FullScaleBits::new(param.unsigned_abs()).map(Prefix::FullScale),
    }
}

/// A prefix's scale as `(num, den)` in `i128`, where both fit.
fn scale(p: Prefix) -> Option<(i128, i128)> {
    let pow = |b: i128, e: i8| {
        let m = b.checked_pow(u32::from(e.unsigned_abs()))?;
        Some(if e < 0 { (1, m) } else { (m, 1) })
    };
    match p {
        Prefix::Kibi => pow(2, 10),
        Prefix::Mebi => pow(2, 20),
        Prefix::Gibi => pow(2, 30),
        Prefix::Tebi => pow(2, 40),
        Prefix::Pow10(e) => pow(10, e.get()),
        Prefix::Pow2(e) => pow(2, e.get()),
        Prefix::FullScale(n) => Some((1, (1i128 << n.get()) - 1)),
        Prefix::Unknown(_) => None,
        p => pow(10, p.si_pow10()?),
    }
}

/// `to`'s scale over `from`'s as `(num, den)`, where it fits `i128` (not reduced).
fn ratio(from: Prefix, to: Prefix) -> Option<(i128, i128)> {
    let ((a, b), (c, d)) = (scale(from)?, scale(to)?);
    Some((a.checked_mul(d)?, b.checked_mul(c)?))
}

/// `x` in units of `from`, in units of `to` as a `U` rounded by `r` and settled by `o`, against `x · num / den`
/// rounded and settled by the reference, where that fits `i128`; `apply` is `Some` exactly where the quotient is
/// whole and fits.
#[expect(
    clippy::expect_used,
    reason = "a property fails by panicking: the prefixes drawn are known"
)]
pub(crate) fn rescales<U: Int + Into<i128>>(
    x: i64,
    from: Prefix,
    to: Prefix,
    r: Rounding,
    o: Overflow,
) {
    let ratio_of = from.ratio_to(to).expect("known prefixes");
    let Some((num, den)) = ratio(from, to) else {
        return;
    };
    let Some(p) = i128::from(x).checked_mul(num) else {
        return;
    };
    let (min, max) = min_max::<U>();
    let got = ratio_of.requantize::<i64, U>(x, r, o).map(Into::into);
    let q = round_div(p, den, r);
    assert_eq!(
        got,
        settle(q, min, max, o),
        "{x} {from:?} -> {to:?} {r:?} {o:?}"
    );
    let exact = (p % den == 0 && (min..=max).contains(&(p / den))).then_some(p / den);
    assert_eq!(
        ratio_of.apply::<i64, U>(x).map(Into::into),
        exact,
        "{x} {from:?} -> {to:?}"
    );
}

/// `U`'s ends.
pub(crate) fn min_max<U: Int + Into<i128>>() -> (i128, i128) {
    // the saturated ends of a value beyond every U
    let r = PrefixRatio::ONE;
    let min = r.requantize::<i128, U>(i128::MIN, Rounding::Truncate, Overflow::Saturate);
    let max = r.requantize::<i128, U>(i128::MAX, Rounding::Truncate, Overflow::Saturate);
    (min.map_or(0, Into::into), max.map_or(0, Into::into))
}

/// The floor `q` of `x` from `from` to `to` brackets the value, by the exact comparison (a multiplication only, not
/// the division the conversion does): `q · to <= x · from < (q + 1) · to`. For any two prefixes, also those whose
/// ratio is past `i128`.
#[expect(
    clippy::expect_used,
    reason = "a property fails by panicking: the prefixes drawn are known"
)]
pub(crate) fn brackets(x: i64, from: Prefix, to: Prefix) {
    let r = from.ratio_to(to).expect("known prefixes");
    let Some(q) = r.apply_rounded::<i64, i64>(x, Rounding::Floor) else {
        return;
    };
    assert_ne!(
        from.cmp_scaled(i128::from(x), to, i128::from(q)),
        Some(Ordering::Less),
        "{q} {to:?} <= {x} {from:?}"
    );
    assert_eq!(
        from.cmp_scaled(i128::from(x), to, i128::from(q) + 1),
        Some(Ordering::Less),
        "{x} {from:?} < {q} + 1 {to:?}"
    );
    let exact = from.cmp_scaled(i128::from(x), to, i128::from(q)) == Some(Ordering::Equal);
    assert_eq!(r.apply::<i64, i64>(x).is_some(), exact);
}

/// The comparison of `a` in units of `pa` with `b` in units of `pb` is the cross-multiplication's, where that fits
/// `i128`.
pub(crate) fn compares(a: i64, pa: Prefix, b: i64, pb: Prefix) {
    let (Some((na, da)), Some((nb, db))) = (scale(pa), scale(pb)) else {
        return;
    };
    let (Some(l), Some(r)) = (
        i128::from(a)
            .checked_mul(na)
            .and_then(|l| l.checked_mul(db)),
        i128::from(b)
            .checked_mul(nb)
            .and_then(|r| r.checked_mul(da)),
    ) else {
        return;
    };
    assert_eq!(
        pa.cmp_scaled(a, pb, b),
        Some(l.cmp(&r)),
        "{a} {pa:?} <=> {b} {pb:?}"
    );
}

#[cfg(not(any(kani, fuzzing)))]
mod run_time {
    use crate::strategy;
    use exact_units::Prefix;
    use proptest::prelude::*;

    /// Any prefix, weighted to the small exponents, whose ratios the `i128` reference holds.
    fn prefix() -> impl Strategy<Value = Prefix> {
        let param = prop_oneof![1 => any::<i8>(), 3 => -16i8..=16];
        (any::<u8>(), param).prop_filter_map("a prefix", |(w, p)| super::prefix(w, p))
    }

    proptest! {
        #[test]
        fn rescales(x in strategy::i64(), from in prefix(), to in prefix(), r in strategy::mode(), o in strategy::policy()) {
            super::rescales::<i8>(x, from, to, r, o);
            super::rescales::<u16>(x, from, to, r, o);
            super::rescales::<i32>(x, from, to, r, o);
            super::rescales::<u64>(x, from, to, r, o);
        }

        #[test]
        fn brackets(x in strategy::i64(), from in prefix(), to in prefix()) {
            super::brackets(x, from, to);
        }

        #[test]
        fn compares(a in strategy::i64(), pa in prefix(), b in strategy::i64(), pb in prefix()) {
            super::compares(a, pa, b, pb);
        }

        /// Ties: a value that is the exact half of a step of the target, and its neighbours.
        #[test]
        fn rescales_near_ties(k in 1i64..=1_000_000, d in -1i64..=1, e in 1i8..=12, r in strategy::mode(), o in strategy::policy()) {
            let x = (2 * k + 1) * 10i64.pow(u32::from(e.unsigned_abs())) / 2 + d;
            let to = super::prefix(1, e).unwrap_or(Prefix::Unit);
            super::rescales::<i32>(x, Prefix::Unit, to, r, o);
            super::rescales::<i32>(-x, Prefix::Unit, to, r, o);
        }
    }
}

#[cfg(kani)]
mod proofs {
    use super::super::reference::{MODES, POLICIES};
    use super::*;

    fn mode() -> Rounding {
        MODES[usize::from(kani::any_where(|i: &u8| *i < 5))]
    }

    fn policy() -> Overflow {
        POLICIES[usize::from(kani::any_where(|i: &u8| *i < 3))]
    }

    /// A prefix of a few that cover the kernel's paths: SI steps (the `u64` path), a binary prefix, a Q7 step and an
    /// 8-bit full scale (the limbs).
    fn some_prefix() -> Prefix {
        const SOME: [Prefix; 6] = [
            Prefix::Unit,
            Prefix::Milli,
            Prefix::Kilo,
            Prefix::Kibi,
            Prefix::Pow2(Pow2Exp::new(-7).unwrap()),
            Prefix::FullScale(FullScaleBits::new(8).unwrap()),
        ];
        SOME[usize::from(kani::any_where(|i: &u8| *i < 6))]
    }

    /// Every i8 between any two of the prefixes, every mode and policy, into an i8.
    #[kani::proof]
    #[kani::unwind(25)]
    fn rescales_i8() {
        rescales::<i8>(
            i64::from(kani::any::<i8>()),
            some_prefix(),
            some_prefix(),
            mode(),
            policy(),
        );
    }

    /// Every pair of i8 between any two of the prefixes.
    #[kani::proof]
    #[kani::unwind(25)]
    fn compares_i8() {
        let (a, b): (i8, i8) = (kani::any(), kani::any());
        compares(a.into(), some_prefix(), b.into(), some_prefix());
    }
}
