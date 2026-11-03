//! Exact integer arithmetic by a rational [Magnitude] at run time, on the kernel of [Limbs]: `x · m` exact, rounded
//! by a named [Rounding] and settled by a named [Overflow], and the comparison of `a · m` with `b`. No `f64` code and
//! no 128-bit arithmetic is linked unless a 128-bit type is used. An irrational magnitude (π, ln 2, ln 10, a half
//! exponent) has no such value and gives `None`, as does a product past the kernel's room (see each method).

use super::{MAX_FACTORS, Magnitude, Work};
use crate::PrefixRatio;
use crate::num::Int;
use crate::num::limbs::{Limbs, Mag, Rational, cmp_rescaled, scaled};
use crate::{Overflow, Rounding};
use core::cmp::Ordering;

/// A rational magnitude as the kernel applies it: `5^pow5 · 2^pow2 · ∏ primes^exps`, the exponents whole (half the
/// doubled ones a [Magnitude] stores). Built in `const` code, so a scale known at compile time is one at run time.
#[derive(Copy, Clone)]
pub(crate) struct Exact {
    pow5: i32,
    pow2: i32,
    primes: [u32; MAX_FACTORS],
    exps: [i32; MAX_FACTORS],
}

impl Exact {
    /// The magnitude's parts, `None` when it isn't a fraction of integers.
    #[expect(
        clippy::indexing_slicing,
        reason = "i < w.len <= MAX_FACTORS, the loop bound"
    )]
    #[expect(
        clippy::arithmetic_side_effects,
        reason = "i < w.len, the loop bound"
    )]
    pub(crate) const fn of(m: &Magnitude) -> Option<Exact> {
        let w = Work::of(m);
        if w.irr[0] != 0 || w.irr[1] != 0 || w.irr[2] != 0 || w.pow10 % 2 != 0 || w.pow2 % 2 != 0 {
            return None;
        }
        let mut x = Exact {
            pow5: w.pow10 / 2,
            pow2: i32::midpoint(w.pow10, w.pow2),
            primes: [1; MAX_FACTORS],
            exps: [0; MAX_FACTORS],
        };
        // a Magnitude holds at most MAX_FACTORS primes, so Work::of gives no more
        let mut i = 0;
        while i < w.len && i < MAX_FACTORS {
            if w.exps[i] % 2 != 0 {
                return None;
            }
            x.primes[i] = w.primes[i];
            x.exps[i] = w.exps[i] / 2;
            i += 1;
        }
        Some(x)
    }
}

/// `n · p^k` in place, by the largest powers of `p` a `u32` holds; `None` past the room of [Limbs].
fn mul_pow(n: &mut Limbs, p: u32, k: u32) -> Option<()> {
    let mut acc = 1u32;
    for _ in 0..k {
        if let Some(a) = acc.checked_mul(p) {
            acc = a;
        } else {
            n.mul_small(acc)?;
            acc = p;
        }
    }
    n.mul_small(acc)
}

/// `n / p^k` toward 0 in place, as [mul_pow]; whether the remainder was not 0.
fn div_pow(n: &mut Limbs, p: u32, k: u32) -> bool {
    let mut acc = 1u32;
    let mut lost = false;
    for _ in 0..k {
        if let Some(a) = acc.checked_mul(p) {
            acc = a;
        } else {
            lost |= n.div_small(acc);
            acc = p;
        }
    }
    lost | n.div_small(acc)
}

impl Rational for Exact {
    fn pow2(&self) -> i32 {
        self.pow2
    }

    fn mul_part(&self, n: &mut Limbs, den: bool) -> Option<()> {
        let side = |e: i32| if den { e < 0 } else { e > 0 };
        if side(self.pow5) {
            n.mul_pow5(self.pow5.unsigned_abs())?;
        }
        for (&p, &e) in self.primes.iter().zip(&self.exps) {
            if side(e) {
                mul_pow(n, p, e.unsigned_abs())?;
            }
        }
        Some(())
    }

    fn div_den(&self, n: &mut Limbs) -> bool {
        let mut lost = false;
        if self.pow5 < 0 {
            lost |= n.div_pow5(self.pow5.unsigned_abs());
        }
        for (&p, &e) in self.primes.iter().zip(&self.exps) {
            if e < 0 {
                lost |= div_pow(n, p, e.unsigned_abs());
            }
        }
        lost
    }

    fn den_is_one(&self) -> bool {
        self.pow5 >= 0 && self.exps.iter().all(|&e| e >= 0)
    }
}

impl Magnitude {
    /// `x · self`, exactly: `None` when that isn't whole, doesn't fit `U`, or the magnitude isn't a fraction of
    /// integers. Generic over the primitive integers ([Int]), unlike [apply_int](Self::apply_int) in `i128`.
    ///
    /// ```
    /// use exact_units::Magnitude;
    ///
    /// // inches in feet: 36 in is 3 ft, 30 in is not whole
    /// let k = Magnitude::ratio(1, 12);
    /// assert_eq!(k.apply::<u16, u8>(36), Some(3));
    /// assert_eq!(k.apply::<u16, u8>(30), None);
    /// ```
    #[must_use]
    pub fn apply<T: Int, U: Int>(&self, x: T) -> Option<U> {
        let (neg, mag, exact) = self.scaled(x, Rounding::Truncate)?;
        if exact {
            mag.settle(neg, Overflow::Checked)
        } else {
            None
        }
    }

    /// `x · self` rounded by `r`; `None` when the result doesn't fit `U` or the magnitude isn't a fraction of
    /// integers.
    ///
    /// ```
    /// use exact_units::{Magnitude, Rounding};
    ///
    /// // 30 in in feet: 2.5
    /// let k = Magnitude::ratio(1, 12);
    /// assert_eq!(k.apply_rounded::<i32, i32>(30, Rounding::NearestEven), Some(2));
    /// assert_eq!(k.apply_rounded::<i32, i32>(30, Rounding::NearestAway), Some(3));
    /// assert_eq!(k.apply_rounded::<i32, i32>(-30, Rounding::Floor), Some(-3));
    /// // π has no integer ratio
    /// assert_eq!(Magnitude::PI.apply_rounded::<i32, i32>(1, Rounding::Floor), None);
    /// ```
    #[must_use]
    pub fn apply_rounded<T: Int, U: Int>(&self, x: T, r: Rounding) -> Option<U> {
        self.requantize(x, r, Overflow::Checked)
    }

    /// `x · self` rounded by `r` and settled into `U` by `o`: `None` where `o` is [Overflow::Checked] and the
    /// result doesn't fit, or where the magnitude isn't a fraction of integers. Wrapping keeps the exact result's low
    /// bits, however large it is.
    #[must_use]
    pub fn requantize<T: Int, U: Int>(&self, x: T, r: Rounding, o: Overflow) -> Option<U> {
        let (neg, mag, _) = self.scaled(x, r)?;
        mag.settle(neg, o)
    }

    /// Compares `a · self` with `b`, exactly; `None` when the magnitude isn't a fraction of integers.
    ///
    /// ```
    /// use exact_units::Magnitude;
    /// use core::cmp::Ordering;
    ///
    /// // 3 ft against 37 in
    /// assert_eq!(Magnitude::ratio(12, 1).cmp_scaled(3, 37), Some(Ordering::Less));
    /// ```
    #[must_use]
    pub fn cmp_scaled<T: Int>(&self, a: T, b: T) -> Option<Ordering> {
        if let Some(r) = self.pow10_ratio() {
            return r.cmp_scaled(a, b);
        }
        let x = Exact::of(self)?;
        let (na, la) = a.to_parts();
        let (nb, lb) = b.to_parts();
        let sign = |neg: bool, l: [u32; 4]| match (l == [0; 4], neg) {
            (true, _) => 0i8,
            (false, true) => -1,
            (false, false) => 1,
        };
        let (sa, sb) = (sign(na, la), sign(nb, lb));
        if sa != sb || sa == 0 {
            return Some(sa.cmp(&sb));
        }
        let m = cmp_rescaled(&x, &mut Limbs::of(la), &mut Limbs::of(lb))?;
        Some(if sa < 0 { m.reverse() } else { m })
    }

    /// The magnitude as a [PrefixRatio] when it is a power of ten alone: the fast path in `u64`.
    fn pow10_ratio(&self) -> Option<PrefixRatio> {
        PrefixRatio::of_pow10(self.as_pow10()?)
    }

    /// `x · self` rounded by `r`: its sign, magnitude and whether it is exact. `None` for an irrational magnitude
    /// or past the room of [Limbs]: a magnitude of a few primes to small powers (every unit definition) fits with
    /// room to spare, one of twelve primes near 2³² to the 63rd power (6 · 10⁴ bits) doesn't.
    fn scaled<T: Int>(&self, x: T, r: Rounding) -> Option<(bool, Mag, bool)> {
        if let Some(p) = self.pow10_ratio() {
            return p.scaled(x, r);
        }
        scaled(&Exact::of(self)?, x, r)
    }
}
