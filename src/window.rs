//! [Window]: a closed range of stored values of one [Fixed] type; [Fit], which stored values stand for an end that is
//! none; and the tolerance of one stored value against another ([Fixed::window], [Fixed::within],
//! [Fixed::is_within]), decided in exact integer arithmetic.

use crate::{Bounds, Contains, Fixed, Storage};
#[cfg(feature = "dimensionless")]
use crate::{Exact, Fraction, OffTarget, Repr, Tolerance, decibel::exact::mul_wide, units::Ratio};
use core::fmt;

/// Which stored values stand for an end that is no stored value: there is no default, the caller names one.
///
/// A window of ±1 % around 333 steps runs from 329.67 to 336.33 steps: [Inside](Fit::Inside) gives 330 ..= 336,
/// every stored value in the real interval and no other; [Outside](Fit::Outside) gives 329 ..= 337, the smallest
/// window of stored values that covers it.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum Fit {
    /// The ends move inward, to the stored values within the real interval: everything in the window is in it.
    Inside,
    /// The ends move outward, to the stored values just past the real interval: everything in it is in the window.
    Outside,
}

/// A closed range of stored values of one [Fixed] type, both ends in: `4950 mV ..= 5050 mV`. Its ends are values of
/// the type, so a check is two compares of the storage, with no scale, rounding or wide arithmetic left for run time:
/// a control loop folds its window once from a setpoint ([Fixed::window]) and checks each reading against it.
///
/// NaN is in no window, also on a [MaybeNan](crate::MaybeNan) type.
///
/// ```
/// # #[cfg(feature = "electrical")] {
/// use exact_units::{Fixed, Window, prefix::Milli, units::Volt};
///
/// type MilliVolts = Fixed<Volt, Milli, u16>;
/// let w = Window::new(MilliVolts::new(4950), MilliVolts::new(5050)).unwrap();
/// assert!(w.contains(&MilliVolts::new(5000)));
/// assert!(MilliVolts::new(5050).is_in(w));
/// assert!(!w.contains(&MilliVolts::new(5051)));
/// assert_eq!(w.to_string(), "4950 mV ..= 5050 mV");
/// assert_eq!(format!("{w:#}"), "4.95 V ..= 5.05 V");
/// assert!(Window::new(MilliVolts::new(2), MilliVolts::new(1)).is_none());
/// # }
/// ```
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct Window<F> {
    /// The lowest value in.
    lo: F,
    /// The highest value in, at least `lo`.
    hi: F,
}

#[cfg(feature = "q")]
impl<F> Window<F> {
    /// `lo ..= hi`, trusting `lo <= hi` already: for a caller in the crate that just folded a `q!` range or
    /// tolerance literal and proved it (`refuse_range_empty` is the compile error otherwise), so no check is done
    /// or needed again at run time.
    pub(crate) const fn from_sorted(lo: F, hi: F) -> Self {
        Self { lo, hi }
    }
}

impl<U, P, T: Storage, B: Bounds<T>> Window<Fixed<U, P, T, B>> {
    /// `lo ..= hi`, `None` where `hi` is below `lo` or either is NaN: an empty window is no window.
    #[must_use]
    pub fn new(lo: Fixed<U, P, T, B>, hi: Fixed<U, P, T, B>) -> Option<Self> {
        (lo.get() <= hi.get()).then_some(Self { lo, hi })
    }

    /// The lowest value in the window.
    #[must_use]
    pub const fn lo(&self) -> Fixed<U, P, T, B> {
        self.lo
    }

    /// The highest value in the window.
    #[must_use]
    pub const fn hi(&self) -> Fixed<U, P, T, B> {
        self.hi
    }

    /// Whether `x` is in the window, its ends included: two compares of the storage, `false` for NaN.
    #[inline]
    #[must_use]
    pub fn contains(&self, x: &Fixed<U, P, T, B>) -> bool {
        let x = x.get();
        self.lo.get() <= x && x <= self.hi.get()
    }
}

impl<U, P, T: Storage, B: Bounds<T>> Contains<Fixed<U, P, T, B>> for Window<Fixed<U, P, T, B>> {
    #[inline]
    fn contains(&self, x: &Fixed<U, P, T, B>) -> bool {
        Window::contains(self, x)
    }
}

/// `4950 mV ..= 5050 mV`; `{:#}` writes each end as `{:#}` does, `4.95 V ..= 5.05 V`.
impl<F: fmt::Display> fmt::Display for Window<F> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.lo, f)?;
        f.write_str(" ..= ")?;
        fmt::Display::fmt(&self.hi, f)
    }
}

#[cfg(feature = "dimensionless")]
impl<U, P, T: Storage, B: Bounds<T>> Fixed<U, P, T, B> {
    /// The window of `tolerance` around this value: every stored value `x` with `|x − self| <= t · |self|`, where its
    /// real ends `self · (1 ± t)` fall between stored values moved as `fit` says, and cut to the type's range and
    /// bounds. The ends are worked out once, exactly, in integers; each check against the window is then two compares
    /// ([Window::contains]). A window around a value always holds that value; around 0 it is 0 alone. With
    /// [Fit::Inside], `x.is_in(target.window(tol, Fit::Inside))` is `x.is_within(target, tol)`. `tolerance` is
    /// anything a [Tolerance] comes from, a relative `q!` literal (`q!(+- 1 %)`) included: an absolute one
    /// (`q!(+- 50 mV)`) has its own `window` instead, needing no `fit` (its own `Inside`/`Outside` is that).
    ///
    /// Integer storages only: a float storage is a compile error.
    ///
    /// ```
    /// # #[cfg(feature = "electrical")] {
    /// use exact_units::{Fit, Fixed, Tolerance, prefix::Milli, units::Volt};
    ///
    /// let setpoint = Fixed::<Volt, Milli, i32>::new(3333);
    /// let inside = setpoint.window(Tolerance::percent(1), Fit::Inside);
    /// assert_eq!(inside.to_string(), "3300 mV ..= 3366 mV"); // 3299.67 ..= 3366.33 inward
    /// let outside = setpoint.window(Tolerance::percent(1), Fit::Outside);
    /// assert_eq!(outside.to_string(), "3299 mV ..= 3367 mV");
    /// assert!(Fixed::new(3366).is_in(inside));
    /// assert!(!Fixed::new(3367).is_in(inside));
    /// # }
    /// ```
    ///
    /// ```compile_fail
    /// # use exact_units::{Fit, Fixed, MaybeNan, Tolerance, prefix::Milli, units::Ratio};
    /// let _ = Fixed::<Ratio, Milli, f32, MaybeNan>::of(1.0).window(Tolerance::percent(1), Fit::Inside);
    /// ```
    #[must_use]
    pub fn window(self, tolerance: impl Into<Tolerance>, fit: Fit) -> Window<Self> {
        const {
            assert!(
                !T::IS_FLOAT,
                "a window from a tolerance is for integer storages"
            );
        };
        let tolerance = tolerance.into();
        let b = int(self.get());
        let (p, q) = parts(tolerance);
        let d = reach(b.unsigned_abs(), p, q, fit);
        let min = B::MIN.map_or(T::MIN_INT, |m| int(m).max(T::MIN_INT));
        let max = B::MAX.map_or(T::MAX_INT, |m| int(m).min(T::MAX_INT));
        // past the storage an end is cut to it, so a reach past i128 cuts the same
        let d = i128::try_from(d).unwrap_or(i128::MAX);
        let (lo, hi) = (b.saturating_sub(d).max(min), b.saturating_add(d).min(max));
        // a OneOf set's window ends on its values: b is one, so some are within
        let (lo, hi) = crate::fixed::set_inward(B::SET, lo, hi).unwrap_or((b, b));
        // min <= lo <= b <= hi <= max: both are values of T in the bounds, as b is
        let end = |x| Self::new_unchecked(T::from_int(x).unwrap_or(self.get()));
        Window {
            lo: end(lo),
            hi: end(hi),
        }
    }

    /// Whether this value is within `tolerance` of `target`: `|self − target| <= t · |target|`, decided exactly
    /// (`q · |self − target| <= p · |target|` for `t = p/q`), building no end. A value is always within any tolerance
    /// of itself, also of 0. The bool of [within](Self::within), for firmware: for a storage of up to 32 bits and a
    /// tolerance whose `p` and `q` fit 32 bits (`Tolerance::percent`, `ppm`, `ppb` of up to 4·10⁹), two 64-bit
    /// products; only past that, 128-bit ones.
    ///
    /// Integer storages only: a float storage is a compile error.
    ///
    /// ```
    /// # #[cfg(feature = "time")] {
    /// use exact_units::{Hz, Tolerance};
    ///
    /// let target = Hz::<u32>::new(80_000_000);
    /// assert!(Hz::new(80_000_080).is_within(target, Tolerance::ppm(1)));
    /// assert!(!Hz::new(80_000_081).is_within(target, Tolerance::ppm(1)));
    /// # }
    /// ```
    #[must_use]
    pub fn is_within(self, target: Self, tolerance: impl Into<Tolerance>) -> bool {
        const {
            assert!(
                !T::IS_FLOAT,
                "a tolerance between stored values is for integer storages"
            );
        };
        let (p, q) = parts(tolerance.into());
        if T::BITS <= 32
            && let (Ok(p), Ok(q)) = (u32::try_from(p), u32::try_from(q))
        {
            // exact: to_i64 of a storage of up to 32 bits
            let (a, b) = (self.get().to_i64(), target.get().to_i64());
            #[expect(
                clippy::arithmetic_side_effects,
                reason = "values of 32 bits are at most 2³² − 1 apart and at most 2³² − 1 from 0, times a u32: \
                          below 2⁶⁴"
            )]
            return u64::from(q) * a.abs_diff(b) <= u64::from(p) * b.unsigned_abs();
        }
        let (a, b) = (int(self.get()), int(target.get()));
        mul_wide(q, a.abs_diff(b)) <= mul_wide(p, b.unsigned_abs())
    }

    /// This value's relative error against `target`, `(self − target) / target` as an exact [Exact]`<Ratio>`, if it
    /// is within `tolerance`; else an [OffTarget] naming both values, the error and the tolerance. The scale cancels,
    /// so the error is that of the stored integers, exact for every value of every integer storage. A value equal to
    /// its target is within with an error of 0, also at 0; any other value against 0 has no relative error and is
    /// refused. `within(..).is_ok()` is [is_within](Self::is_within), which is for firmware.
    ///
    /// Integer storages only: a float storage is a compile error.
    ///
    /// ```
    /// # #[cfg(feature = "time")] {
    /// use exact_units::{Hz, Tolerance};
    ///
    /// let target = Hz::<u32>::new(80_000_000);
    /// assert_eq!(Hz::new(79_999_980).within(target, Tolerance::ppm(1)).unwrap().to_string(), "-0.25 ppm");
    /// let e = Hz::new(80_000_100).within(target, Tolerance::ppm(1)).unwrap_err();
    /// assert_eq!(e.to_string(), "80000100 Hz is 1.25 ppm off 80000000 Hz, outside ±1 ppm");
    /// # }
    /// ```
    pub fn within(
        self,
        target: Self,
        tolerance: impl Into<Tolerance>,
    ) -> Result<Exact<Ratio>, OffTarget<Self>> {
        const {
            assert!(
                !T::IS_FLOAT,
                "a tolerance between stored values is for integer storages"
            );
        };
        let tolerance = tolerance.into();
        let (a, b) = (int(self.get()), int(target.get()));
        // two values of 64 bits at most differ in i128; the sign goes on the numerator
        let off = if b < 0 {
            b.checked_sub(a)
        } else {
            a.checked_sub(b)
        };
        let error = off
            .and_then(|n| Fraction::checked_new(n, b.unsigned_abs()))
            .map(Exact::of);
        match error {
            _ if a == b => Ok(Exact::ZERO),
            Some(e) if tolerance.contains(e) => Ok(e),
            _ => Err(OffTarget {
                achieved: self,
                target,
                error,
                tolerance,
                zero_target: b == 0,
            }),
        }
    }
}

/// The integer a value of an integer storage is. Never reached for a float: each caller refuses a float storage at
/// compile time, and a float would be 0 here.
#[cfg(feature = "dimensionless")]
fn int<T: Storage>(x: T) -> i128 {
    match x.to_repr() {
        Repr::Int(i) => i,
        Repr::Float(_) => 0,
    }
}

/// `p` and `q` of `t = p/q`, in lowest terms, `q` at least 1.
#[cfg(feature = "dimensionless")]
fn parts(tolerance: Tolerance) -> (u128, u128) {
    let t = tolerance.get().get();
    (t.numer(), t.denom())
}

/// `of · p/q` rounded down ([Fit::Inside]) or up ([Fit::Outside]), `u128::MAX` past it: how far a window reaches from
/// a target of magnitude `of` (at most 2⁶⁴). A storage of up to 32 bits and `p`, `q` of 32 bits take one 64-bit
/// product and quotient.
#[cfg(feature = "dimensionless")]
fn reach(of: u128, p: u128, q: u128, fit: Fit) -> u128 {
    let up = fit == Fit::Outside;
    if let (Ok(of), Ok(p), Ok(q)) = (u32::try_from(of), u32::try_from(p), u32::try_from(q)) {
        #[expect(
            clippy::arithmetic_side_effects,
            reason = "a product of two u32s fits u64; q is a fraction's denominator, at least 1; a quotient of u64s \
                      plus 1 fits u128"
        )]
        let d = {
            let n = u64::from(of) * u64::from(p);
            u128::from(n / u64::from(q)) + u128::from(up && n % u64::from(q) != 0)
        };
        return d;
    }
    let (hi, lo) = mul_wide(of, p);
    if hi >= q {
        // a quotient of 2¹²⁸ or more
        return u128::MAX;
    }
    let (d, rem) = div_wide(hi, lo, q);
    d.saturating_add(u128::from(up && rem != 0))
}

/// `(hi · 2¹²⁸ + lo) / q` and its remainder, for `hi < q`: the quotient fits `u128`. Bit by bit, the remainder kept
/// below `q`.
#[cfg(feature = "dimensionless")]
#[expect(
    clippy::arithmetic_side_effects,
    reason = "q is at least 1 (hi < q); shifts by 1 and by i < 128; the quotient has 128 bits as hi < q"
)]
fn div_wide(hi: u128, lo: u128, q: u128) -> (u128, u128) {
    if hi == 0 {
        return (lo / q, lo % q);
    }
    let (mut rem, mut quot) = (hi, 0u128);
    for i in (0..128).rev() {
        // rem < q, so 2·rem + bit < 2q: past 2¹²⁸ the carry says so, and less q then fits
        let carry = rem >> 127 == 1;
        rem = (rem << 1) | ((lo >> i) & 1);
        quot <<= 1;
        if carry || rem >= q {
            rem = rem.wrapping_sub(q);
            quot |= 1;
        }
    }
    (quot, rem)
}

#[cfg(all(test, feature = "dimensionless"))]
mod tests {
    use super::div_wide;
    use std::string::ToString;

    #[test]
    fn against_a_target_of_zero() {
        use crate::{Exact, Fit, Fixed, Tolerance, prefix::Milli, units::Ratio};
        type F = Fixed<Ratio, Milli, i16>;
        let (zero, tol) = (F::new(0), Tolerance::percent(100));
        assert_eq!(zero.within(zero, tol), Ok(Exact::ZERO));
        assert!(zero.is_within(zero, tol));
        let e = F::new(1).within(zero, tol).unwrap_err();
        assert_eq!(e.error, None);
        assert_eq!(
            e.to_string(),
            "1 ‰ is off 0 ‰: no relative error to a target of 0"
        );
        let w = zero.window(tol, Fit::Outside);
        assert_eq!((w.lo().get(), w.hi().get()), (0, 0));
    }

    #[test]
    fn div_wide_of_wide_numbers() {
        // (2¹²⁸ + 5) / 3: 2¹²⁸ = 3 · 113427455640312821154458202477256070485 + 1
        let q = 113_427_455_640_312_821_154_458_202_477_256_070_485;
        assert_eq!(div_wide(1, 5, 3), (q + 2, 0));
        assert_eq!(div_wide(1, 0, 3), (q, 1));
        // (u128::MAX − 1) · 2¹²⁸ + u128::MAX, by u128::MAX: 2¹²⁸ − 1 rem u128::MAX − 1
        assert_eq!(
            div_wide(u128::MAX - 1, u128::MAX, u128::MAX),
            (u128::MAX, u128::MAX - 1)
        );
        assert_eq!(div_wide(0, 17, 5), (3, 2));
    }
}
