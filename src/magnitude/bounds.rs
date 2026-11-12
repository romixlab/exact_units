//! A [Magnitude]'s value as an interval with 128-bit ends, rounded outward at every step: what the certified edge of
//! an `Exact` (NUM-38) multiplies by where the factor between two units isn't a fraction (π, ln 2, ln 10, a root).

use super::{Magnitude, approx::SYMBOLS_128};
use crate::interval::{Interval, Num};

/// `base^(e_x2 / 2)`, `base > 0`: a whole power by squaring, a half one as the square root of `base^|e_x2|`, a
/// negative one as `1 /` the positive. Every step rounds its lower end down and its upper end up.
#[expect(
    clippy::as_conversions,
    reason = "const fn: From is not const on stable; |e_x2| <= 128 fits u64"
)]
const fn pow_x2(base: Interval, e_x2: i8) -> Interval {
    if e_x2 == 0 {
        return Interval::exact(1);
    }
    let k = e_x2.unsigned_abs() as u64;
    let p = if k.is_multiple_of(2) {
        base.pow(k / 2)
    } else {
        let p = base.pow(k);
        let lo = p.lo.root_down(2, 1);
        Interval {
            lo,
            hi: p.hi.root_up(2, lo, 1),
        }
    };
    if e_x2 < 0 {
        Interval::exact(1).div(p)
    } else {
        p
    }
}

/// One of [SYMBOLS_128]: its 128-bit mantissa one unit down and one up.
#[expect(
    clippy::arithmetic_side_effects,
    reason = "the mantissas are of π, ln 2 and ln 10, neither 2^127 nor u128::MAX, so m - 1 and m + 1 fit"
)]
const fn symbol(i: usize) -> Interval {
    #[expect(clippy::indexing_slicing, reason = "i < 3, the callers' constants")]
    let (m, e) = SYMBOLS_128[i];
    #[expect(
        clippy::as_conversions,
        reason = "const fn: From is not const on stable; i32 into i64"
    )]
    let e = e as i64;
    Interval {
        lo: Num { m: m - 1, e },
        hi: Num { m: m + 1, e },
    }
}

impl Magnitude {
    /// `[lo, hi]` holding this magnitude's value, its ends 128-bit mantissas rounded outward at every step: about
    /// 2⁻¹²⁰ wide relative for a few factors. `None` where μ₀, G or m_u is left in it: those have no value without a
    /// set ([resolve](Magnitude::resolve)).
    #[expect(clippy::indexing_slicing, reason = "i < MAX_FACTORS, the loop bound")]
    #[expect(
        clippy::arithmetic_side_effects,
        reason = "i < MAX_FACTORS, the loop bound, so i + 1 fits"
    )]
    pub(crate) const fn bounds(&self) -> Option<Interval> {
        if self.has_measured() {
            return None;
        }
        let mut acc = pow_x2(Interval::exact(10), self.pow10);
        if let Some(rest) = &self.rest {
            acc = acc.mul(pow_x2(Interval::exact(2), rest.pow2));
            let mut i = 0;
            while i < super::MAX_FACTORS {
                if let Some(p) = rest.factors[i] {
                    // a prime below 2^64 is a u128 above 0
                    #[expect(
                        clippy::as_conversions,
                        reason = "const fn: From is not const on stable; u64 into u128"
                    )]
                    let base = Interval::exact(p.prime as u128);
                    acc = acc.mul(pow_x2(base, p.exp));
                }
                i += 1;
            }
            if let Some(s) = rest.symbols {
                acc = acc.mul(pow_x2(symbol(0), s.pi));
                acc = acc.mul(pow_x2(symbol(1), s.ln2));
                acc = acc.mul(pow_x2(symbol(2), s.ln10));
            }
        }
        Some(acc)
    }
}
