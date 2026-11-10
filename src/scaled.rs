//! [Scaled]: a number at an exact scale known only at run time ([RunScale]), the unit still in the type. A stream's
//! own time base (H.264 and H.265 `num_units_in_tick ÷ time_scale`, an MP4 timescale), a JPEG quantization step, an
//! HRD rate of 2^(6 + bit_rate_scale) bit/s: the scale is read at run time, the unit is known when the firmware is
//! built. A [Fixed]'s scale is a type; a `Scaled`'s is a value beside the number.
//!
//! Every conversion is exact or refused, or rounds by a named [Rounding]: to and from a [Fixed], between two run-time
//! scales, and the comparison of two values at different scales. They run on the crate's integer kernel: a `u32`
//! count in `u64` arithmetic, wider ones in 32-bit limbs. No float, no allocation.

use crate::fixed::runtime::Scale;
use crate::num::Int;
use crate::num::limbs::{self, Both, Limbs, Mag, Rational, cmp_rescaled, round_up};
use crate::tree::write_unit;
use crate::unit::write_superscript;
use crate::{Bounds, Fixed, FixedScale, Magnitude, Overflow, Prefix, Rounding, Storage, Unit};
use core::cmp::Ordering;
use core::fmt;
use core::marker::PhantomData;
#[cfg(feature = "shrink_wrap")]
use shrink_wrap::prelude::*;

/// An exact positive scale known at run time: `num / den · 2^pow2`, `num` and `den` odd and coprime `u32`s, `pow2`
/// within ±127. One form per value, so `==` and `Ord` are those of the value: `RunScale::ratio(2002, 120_000)` is
/// `RunScale::ratio(1001, 60_000)`.
///
/// Every time base of H.264, H.265, AV1 and MP4 is a pair of `u32` fields and fits. The power of two holds the
/// base-2 mantissa-exponent forms of the codecs without a refusal: T.800's step 2^(Rb − εb)·(1 + μb/2¹¹) down to
/// 2^−42, H.264's HRD 2^(6 + bit_rate_scale). A value that has no such form (a numerator past `u32` after its
/// factors of two are taken out, 0) is refused at construction, never rounded.
///
/// ```
/// use exact_units::RunScale;
///
/// // H.264 E.2.1's example: num_units_in_tick 1001, time_scale 60 000
/// let tc = RunScale::ratio(1001, 60_000).unwrap();
/// assert_eq!(tc, RunScale::ratio(2002, 120_000).unwrap());
/// assert_eq!(tc.to_ratio(), Some((1001, 60_000)));
/// // a frame is two ticks in H.264 (Table E-6)
/// let frame = tc.checked_mul(RunScale::int(2).unwrap()).unwrap();
/// assert_eq!(frame.to_string(), "1001/30000");
/// assert!(frame > tc);
/// assert_eq!(RunScale::ratio(1, 0), None);
/// // T.800's step 2^(Rb − εb)·(1 + μb/2¹¹) with Rb = 8, εb = 31, μb = 1
/// let step = RunScale::new(2049, 1, 8 - 31 - 11).unwrap();
/// assert_eq!(step.parts(), (2049, 1, -34));
/// assert_eq!(step.to_ratio(), Some((2049, 1 << 34)));
/// // past u64 the power of two is written apart
/// assert_eq!(RunScale::new(3, 1, -100).unwrap().to_string(), "3·2⁻¹⁰⁰");
/// ```
///
/// On the wire (`final_structure`): `num` and `den` as `u32`, `pow2` as `i8`; a form that isn't the canonical one
/// (an even or zero part, a common factor, an exponent of −128) doesn't decode, so a value has one encoding.
#[cfg_attr(
    feature = "shrink_wrap",
    derive_shrink_wrap(borrowed, owned(feature = "std"), final_structure, validate = Self::validate)
)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct RunScale {
    /// The odd numerator, coprime to `den`.
    num: u32,
    /// The odd denominator.
    den: u32,
    /// The exponent of 2, within ±127.
    pow2: i8,
}

/// The greatest common divisor of two odd numbers, by subtraction and shifts: no division, which a Cortex-M0+ has
/// no instruction for.
#[expect(
    clippy::arithmetic_side_effects,
    reason = "a > b where a - b is taken, b > a where b - a is; trailing_zeros of a nonzero u64 is below 64"
)]
const fn gcd_odd(mut a: u64, mut b: u64) -> u64 {
    while a != b {
        if a > b {
            a -= b;
            a >>= a.trailing_zeros();
        } else {
            b -= a;
            b >>= b.trailing_zeros();
        }
    }
    a
}

/// `a · 2^k` against `b`, exactly, without a wider type.
#[expect(
    clippy::arithmetic_side_effects,
    reason = "k is a difference of two i8 exponents, so its negation fits i32"
)]
const fn cmp_shifted(a: u64, b: u64, k: i32) -> Ordering {
    if k >= 0 {
        cmp_shl(a, b, k.unsigned_abs())
    } else {
        cmp_shl(b, a, (-k).unsigned_abs()).reverse()
    }
}

/// `a · 2^k` against `b`: past `u64` it is the larger, as `a` is not 0 there.
const fn cmp_shl(a: u64, b: u64, k: u32) -> Ordering {
    if a == 0 {
        cmp_u64(0, b)
    } else if a.leading_zeros() < k {
        Ordering::Greater
    } else {
        cmp_u64(a << k, b)
    }
}

/// `a` against `b` in a const fn.
const fn cmp_u64(a: u64, b: u64) -> Ordering {
    if a < b {
        Ordering::Less
    } else if a > b {
        Ordering::Greater
    } else {
        Ordering::Equal
    }
}

impl RunScale {
    /// 1.
    pub const ONE: RunScale = RunScale {
        num: 1,
        den: 1,
        pow2: 0,
    };

    /// The largest exponent of 2 a scale holds, and its negation the smallest: ±127, so that
    /// [recip](Self::recip) always is one.
    pub const MAX_POW2: i8 = 127;

    /// `num / den · 2^pow2`, `None` for a part of 0 or where the reduced form has an exponent past ±127.
    #[must_use]
    #[expect(
        clippy::as_conversions,
        reason = "const fn: From is not const on stable"
    )]
    pub const fn new(num: u32, den: u32, pow2: i8) -> Option<RunScale> {
        Self::of_parts(num as u64, den as u64, pow2 as i32)
    }

    /// `num / den`, `None` for a part of 0: a VUI's `num_units_in_tick ÷ time_scale`.
    #[must_use]
    pub const fn ratio(num: u32, den: u32) -> Option<RunScale> {
        Self::new(num, den, 0)
    }

    /// The whole number `n`, `None` for 0: a JPEG quantization step (T.81 B.2.4.1, 1 to 65 535).
    #[must_use]
    pub const fn int(n: u32) -> Option<RunScale> {
        Self::new(n, 1, 0)
    }

    /// `2^e`, `None` for −128 (past ±127): H.264's HRD 2^(6 + bit_rate_scale) bit/s (E-81).
    #[must_use]
    pub const fn pow2(e: i8) -> Option<RunScale> {
        Self::new(1, 1, e)
    }

    /// `num / den` of wider parts: `None` for a part of 0, or where a part of the reduced form, its factors of two
    /// taken out, passes `u32`.
    #[must_use]
    pub const fn from_u64(num: u64, den: u64) -> Option<RunScale> {
        Self::of_parts(num, den, 0)
    }

    /// `num / den · 2^e` reduced to the canonical form.
    #[expect(
        clippy::as_conversions,
        clippy::cast_possible_truncation,
        clippy::cast_possible_wrap,
        reason = "const fn: From and TryFrom are not const on stable; the parts are checked to fit u32 and the \
                  exponent ±127 before each cast, and a trailing_zeros of a u64 is below 64"
    )]
    #[expect(
        clippy::arithmetic_side_effects,
        reason = "e is within ±255 (two i8 exponents or one) and each trailing_zeros below 64: the sums fit i32"
    )]
    const fn of_parts(num: u64, den: u64, e: i32) -> Option<RunScale> {
        if num == 0 || den == 0 {
            return None;
        }
        let (tn, td) = (num.trailing_zeros(), den.trailing_zeros());
        let (n, d) = (num >> tn, den >> td);
        let e = e + tn as i32 - td as i32;
        let g = gcd_odd(n, d);
        let (Some(n), Some(d)) = (n.checked_div(g), d.checked_div(g)) else {
            return None;
        };
        if n > u32::MAX as u64 || d > u32::MAX as u64 || e > 127 || e < -127 {
            return None;
        }
        Some(RunScale {
            num: n as u32,
            den: d as u32,
            pow2: e as i8,
        })
    }

    /// A compile-time scale as a run-time one: `None` where it isn't a fraction of integers (π/180) or has no
    /// form here (10⁻¹⁵: 5¹⁵ passes `u32`).
    ///
    /// ```
    /// use exact_units::{Magnitude, RunScale};
    ///
    /// assert_eq!(RunScale::of_magnitude(&Magnitude::ratio(1, 90_000)), RunScale::ratio(1, 90_000));
    /// assert_eq!(RunScale::of_magnitude(&Magnitude::PI), None);
    /// ```
    #[must_use]
    #[expect(
        clippy::as_conversions,
        clippy::cast_possible_truncation,
        clippy::cast_possible_wrap,
        reason = "const fn: TryFrom is not const on stable; the parts are checked to fit u64 before the casts, and \
                  a trailing_zeros of a u128 is below 128"
    )]
    #[expect(
        clippy::arithmetic_side_effects,
        reason = "each trailing_zeros is below 128, so their difference fits i32"
    )]
    pub const fn of_magnitude(m: &Magnitude) -> Option<RunScale> {
        let Some((n, d)) = m.to_ratio() else {
            return None;
        };
        if n == 0 || d == 0 {
            return None;
        }
        let (tn, td) = (n.trailing_zeros(), d.trailing_zeros());
        let (n, d) = (n >> tn, d >> td);
        if n > u64::MAX as u128 || d > u64::MAX as u128 {
            return None;
        }
        Self::of_parts(n as u64, d as u64, tn as i32 - td as i32)
    }

    /// `P`'s scale as a run-time one, as [of_magnitude](Self::of_magnitude) gives it.
    #[must_use]
    pub const fn of<P: FixedScale>() -> Option<RunScale> {
        Self::of_magnitude(&P::SCALE)
    }

    /// The odd numerator, the odd denominator and the exponent of 2: `(1, 5625, -4)` for 1/90000.
    #[must_use]
    pub const fn parts(self) -> (u32, u32, i8) {
        (self.num, self.den, self.pow2)
    }

    /// The scale as a reduced fraction of `u64`s, `None` where a part passes `u64` (2⁻⁷⁰).
    #[must_use]
    #[expect(
        clippy::as_conversions,
        reason = "const fn: From is not const on stable"
    )]
    pub const fn to_ratio(self) -> Option<(u64, u64)> {
        let (n, d) = (self.num as u64, self.den as u64);
        let k = self.pow2.unsigned_abs() as u32;
        let (wide, other) = if self.pow2 >= 0 { (n, d) } else { (d, n) };
        if wide.leading_zeros() < k {
            return None;
        }
        let wide = wide << k;
        Some(if self.pow2 >= 0 {
            (wide, other)
        } else {
            (other, wide)
        })
    }

    /// The scale as a [Magnitude], for the API model and the host; it factors both parts.
    #[must_use]
    pub fn to_magnitude(self) -> Magnitude {
        Magnitude::ratio(u64::from(self.num), u64::from(self.den)).mul(Magnitude::pow2(self.pow2))
    }

    /// `1 / self`.
    #[must_use]
    #[expect(
        clippy::arithmetic_side_effects,
        reason = "pow2 is within ±127, so its negation fits i8"
    )]
    pub const fn recip(self) -> RunScale {
        RunScale {
            num: self.den,
            den: self.num,
            pow2: -self.pow2,
        }
    }

    /// `self · rhs`, `None` where the product has no form here.
    #[must_use]
    #[expect(
        clippy::as_conversions,
        reason = "const fn: From is not const on stable"
    )]
    #[expect(
        clippy::arithmetic_side_effects,
        reason = "u32 · u32 fits u64; the exponents are i8, their sum fits i32"
    )]
    pub const fn checked_mul(self, rhs: RunScale) -> Option<RunScale> {
        Self::of_parts(
            self.num as u64 * rhs.num as u64,
            self.den as u64 * rhs.den as u64,
            self.pow2 as i32 + rhs.pow2 as i32,
        )
    }

    /// `self / rhs`, `None` where the quotient has no form here.
    #[must_use]
    pub const fn checked_div(self, rhs: RunScale) -> Option<RunScale> {
        self.checked_mul(rhs.recip())
    }

    /// The largest scale that both are whole multiples of: the step at which values of the two add exactly. `None`
    /// where it has no form here (the least common multiple of the denominators passes `u32`).
    ///
    /// ```
    /// use exact_units::RunScale;
    ///
    /// // 1/90000 s and 1001/60000 s meet at 1/180000 s
    /// let a = RunScale::ratio(1, 90_000).unwrap();
    /// let b = RunScale::ratio(1001, 60_000).unwrap();
    /// assert_eq!(RunScale::common(a, b), RunScale::ratio(1, 180_000));
    /// ```
    #[must_use]
    #[expect(
        clippy::as_conversions,
        reason = "const fn: From is not const on stable"
    )]
    #[expect(clippy::arithmetic_side_effects, reason = "u32 · u32 fits u64")]
    pub const fn common(a: RunScale, b: RunScale) -> Option<RunScale> {
        // gcd(n1/d1, n2/d2) = gcd(n1, n2) / lcm(d1, d2), the power of two the smaller one
        let gn = gcd_odd(a.num as u64, b.num as u64);
        let gd = gcd_odd(a.den as u64, b.den as u64);
        let Some(l) = (a.den as u64).checked_div(gd) else {
            return None;
        };
        let e = if a.pow2 < b.pow2 { a.pow2 } else { b.pow2 };
        Self::of_parts(gn, l * b.den as u64, e as i32)
    }

    /// `x · self` rounded by `r`: its sign, magnitude and whether it is exact. A magnitude of up to 32 bits in `u64`
    /// arithmetic, anything else on the kernel's limbs. `None` past the room of the limbs, which no scale here reaches
    /// for an integer up to 128 bits (128 + 32 + 127 bits, doubled for the rounding, against 736).
    #[inline]
    pub(crate) fn scaled<T: Int>(self, x: T, r: Rounding) -> Option<(bool, Mag, bool)> {
        let (neg, low) = x.to_parts();
        if let [m, 0, 0, 0] = low
            && let Some((q, exact)) = self.scaled_u32(m, neg, r)
        {
            return Some((neg, q, exact));
        }
        limbs::scaled_parts(&self, neg, low, r)
    }

    /// [scaled](Self::scaled) of a magnitude of 32 bits in `u64`: `None` where the shifted product or denominator
    /// passes it.
    #[expect(
        clippy::arithmetic_side_effects,
        reason = "u32 · u32 fits u64; the shifts are checked against leading_zeros or are at most 32 on a u32; rem < \
                  d, so d - rem doesn't wrap; d >= 2 where q is incremented, so q < 2^63"
    )]
    fn scaled_u32(self, m: u32, neg: bool, r: Rounding) -> Option<(Mag, bool)> {
        let p = u64::from(m) * u64::from(self.num);
        let k = u32::from(self.pow2.unsigned_abs());
        let (p, d) = if self.pow2 >= 0 {
            if p.leading_zeros() < k {
                return None;
            }
            (p << k, u64::from(self.den))
        } else {
            if k > 32 {
                return None;
            }
            (p, u64::from(self.den) << k)
        };
        if d == 1 {
            return Some((Mag::of_u64(p), true));
        }
        let (q, rem) = (p / d, p % d);
        let half = rem >= d - rem;
        let sticky = rem != 0 && rem != d - rem;
        let q = if round_up(r, neg, q & 1 == 1, half, sticky) {
            q + 1
        } else {
            q
        };
        Some((Mag::of_u64(q), rem == 0))
    }
}

#[cfg(feature = "shrink_wrap")]
impl RunScale {
    /// Refuses a form that isn't the canonical one: what [new](Self::new) makes of its own parts is itself.
    fn validate(&self) -> Result<(), shrink_wrap::Error> {
        if RunScale::new(self.num, self.den, self.pow2) == Some(*self) {
            Ok(())
        } else {
            Err(shrink_wrap::Error::SubtypeOutOfRange)
        }
    }
}

impl Rational for RunScale {
    fn pow2(&self) -> i32 {
        i32::from(self.pow2)
    }

    fn mul_part(&self, n: &mut Limbs, den: bool) -> Option<()> {
        n.mul_small(if den { self.den } else { self.num })
    }

    fn div_den(&self, n: &mut Limbs) -> bool {
        n.div_small(self.den)
    }

    fn den_is_one(&self) -> bool {
        self.den == 1
    }
}

impl PartialOrd for RunScale {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// By value: `n1 · d2 · 2^(k1 − k2)` against `n2 · d1`, in `u64`.
impl Ord for RunScale {
    #[expect(
        clippy::arithmetic_side_effects,
        reason = "u32 · u32 fits u64; the exponents are i8, their difference fits i32"
    )]
    fn cmp(&self, other: &Self) -> Ordering {
        cmp_shifted(
            u64::from(self.num) * u64::from(other.den),
            u64::from(other.num) * u64::from(self.den),
            i32::from(self.pow2) - i32::from(other.pow2),
        )
    }
}

/// `1001/60000`, `25`, and the power of two as a superscript where the fraction doesn't hold it in `u64`:
/// `2049·2⁻³⁴`.
impl fmt::Display for RunScale {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let (num, den, e) = match self.to_ratio() {
            Some((n, d)) => (n, d, 0),
            None => (u64::from(self.num), u64::from(self.den), self.pow2),
        };
        if den == 1 {
            write!(f, "{num}")?;
        } else {
            write!(f, "{num}/{den}")?;
        }
        if e != 0 {
            f.write_str("·2")?;
            write_superscript(f, i32::from(e))?;
        }
        Ok(())
    }
}

/// A number at an exact scale known only at run time: `value · scale` in units of `U`. The unit is a type, the scale
/// a [RunScale] beside the number, `T` a primitive integer. `Ticks<u64>` is a count at a stream's own time base.
///
/// Conversions to and from [Fixed] and between scales are exact or `None`, or round by a named [Rounding]. Two
/// values compare by their exact value, at any two scales: 3 at 1/25 s is 6 at 1/50 s. Arithmetic is at one scale:
/// [checked_add](Self::checked_add) refuses two different scales, never rescales one behind the caller's back.
///
/// ```
/// # #[cfg(feature = "time")] {
/// use exact_units::{Fixed, Rounding, RunScale, Ticks, prefix::Rational, units::Second};
///
/// // an MP4 track's timescale of 15360, read from the file
/// let base = RunScale::ratio(1, 15_360).unwrap();
/// let pts = Ticks::<u64>::new(15_360 * 10 + 512, base);
/// // to a 90 kHz clock: 512/15360 s is 3000 ticks, exact
/// assert_eq!(pts.to_fixed::<Rational<1, 90_000>, u64>(), Some(Fixed::new(903_000)));
/// // to milliseconds it isn't whole: refused, or rounded by name
/// assert_eq!(pts.to_fixed::<exact_units::prefix::Milli, u32>(), None);
/// assert_eq!(pts.to_fixed_rounded::<exact_units::prefix::Milli, u32>(Rounding::Floor), Some(Fixed::new(10_033)));
/// assert_eq!(pts.to_string(), "154112·1/15360 s");
/// # }
/// ```
///
/// On the wire (`final_structure`): the value as `T` is, then its [RunScale].
#[cfg_attr(
    feature = "shrink_wrap",
    derive_shrink_wrap(borrowed, owned(feature = "std"), final_structure)
)]
pub struct Scaled<U, T> {
    /// The number of steps.
    value: T,
    /// The step.
    scale: RunScale,
    /// The unit, in the type only.
    _unit: PhantomData<U>,
}

/// A count of ticks at a time base known at run time: `Ticks<u64>` at a VUI's `num_units_in_tick ÷ time_scale`.
#[cfg(feature = "time")]
pub type Ticks<T> = Scaled<crate::units::Second, T>;

/// `P`'s scale as a [RunScale], worked out at compile time.
struct RunOf<P>(PhantomData<P>);

impl<P: FixedScale> RunOf<P> {
    const RUN: Option<RunScale> = RunScale::of_magnitude(&P::SCALE);
}

impl<U, T: Storage + Int> Scaled<U, T> {
    /// `value` steps of `scale`.
    #[must_use]
    pub const fn new(value: T, scale: RunScale) -> Self {
        Scaled {
            value,
            scale,
            _unit: PhantomData,
        }
    }

    /// The number of steps.
    #[must_use]
    pub const fn get(self) -> T {
        self.value
    }

    /// The step.
    #[must_use]
    pub const fn scale(self) -> RunScale {
        self.scale
    }

    /// Whether both are the same number at the same step, not only the same value (`==` says that).
    #[must_use]
    pub fn same_repr(&self, other: &Self) -> bool {
        self.scale == other.scale && self.value.to_parts() == other.value.to_parts()
    }

    /// The value at the step `to`, exactly: `None` where it isn't a whole number of `to` or doesn't fit `T2`.
    ///
    /// ```
    /// # #[cfg(feature = "time")] {
    /// use exact_units::{Rounding, RunScale, Ticks};
    ///
    /// let field = RunScale::ratio(1001, 60_000).unwrap();
    /// let rtp = RunScale::ratio(1, 90_000).unwrap();
    /// // two fields are a frame, 3003 ticks of 90 kHz; one is 1501.5
    /// assert_eq!(Ticks::<u32>::new(2, field).rescale::<u32>(rtp).map(|t| t.get()), Some(3003));
    /// assert_eq!(Ticks::<u32>::new(1, field).rescale::<u32>(rtp), None);
    /// let half = Ticks::<u32>::new(1, field).rescale_rounded::<u32>(rtp, Rounding::NearestEven);
    /// assert_eq!(half.map(|t| t.get()), Some(1502));
    /// # }
    /// ```
    #[must_use]
    pub fn rescale<T2: Storage + Int>(self, to: RunScale) -> Option<Scaled<U, T2>> {
        let (neg, mag, exact) = by_ratio(self.value, self.scale, to, Rounding::Truncate)?;
        if !exact {
            return None;
        }
        Some(Scaled::new(mag.settle(neg, Overflow::Checked)?, to))
    }

    /// The value at the step `to`, rounded by `r`: `None` where it doesn't fit `T2`.
    #[must_use]
    pub fn rescale_rounded<T2: Storage + Int>(
        self,
        to: RunScale,
        r: Rounding,
    ) -> Option<Scaled<U, T2>> {
        self.requantize(to, r, Overflow::Checked)
    }

    /// The value at the step `to`, rounded by `r` and settled into `T2` by `o`: `None` only where `o` is
    /// [Overflow::Checked] and it doesn't fit.
    #[must_use]
    pub fn requantize<T2: Storage + Int>(
        self,
        to: RunScale,
        r: Rounding,
        o: Overflow,
    ) -> Option<Scaled<U, T2>> {
        let (neg, mag, _) = by_ratio(self.value, self.scale, to, r)?;
        Some(Scaled::new(mag.settle(neg, o)?, to))
    }

    /// The value as a [Fixed] in steps of `P`, exactly: `None` where it isn't a whole number of them or doesn't fit
    /// `T2`. A `P` that isn't a fraction of integers (π/180) doesn't compile.
    #[must_use]
    pub fn to_fixed<P: FixedScale, T2: Storage + Int>(self) -> Option<Fixed<U, P, T2>>
    where
        U: Unit,
    {
        let (neg, mag, exact) = self.in_steps_of::<P>(Rounding::Truncate)?;
        if !exact {
            return None;
        }
        Fixed::try_new(mag.settle(neg, Overflow::Checked)?).ok()
    }

    /// The value as a [Fixed] in steps of `P`, rounded by `r`: `None` where it doesn't fit `T2`. Into a bounded
    /// `Fixed`: [requantize_fixed](Self::requantize_fixed).
    #[must_use]
    pub fn to_fixed_rounded<P: FixedScale, T2: Storage + Int>(
        self,
        r: Rounding,
    ) -> Option<Fixed<U, P, T2>>
    where
        U: Unit,
    {
        self.requantize_fixed(r, Overflow::Checked)
    }

    /// The value as a [Fixed] in steps of `P`, rounded by `r`, settled into `T2` by `o` and then within `B`:
    /// [Overflow::Saturate] clamps to `B`'s ends, otherwise a value outside `B` is `None`, as
    /// [Fixed::from_prefix] does.
    #[must_use]
    pub fn requantize_fixed<P: FixedScale, T2: Storage + Int, B: Bounds<T2>>(
        self,
        r: Rounding,
        o: Overflow,
    ) -> Option<Fixed<U, P, T2, B>>
    where
        U: Unit,
    {
        let (neg, mag, _) = self.in_steps_of::<P>(r)?;
        let v: T2 = mag.settle(neg, o)?;
        if o == Overflow::Saturate {
            Some(Fixed::clamped(v))
        } else {
            Fixed::try_new(v).ok()
        }
    }

    /// `self` in steps of `P`: as [by_ratio] where `P` has a form as a [RunScale] (then no [Magnitude] code is
    /// linked), else on the limbs with `P`'s magnitude.
    fn in_steps_of<P: FixedScale>(self, r: Rounding) -> Option<(bool, Mag, bool)> {
        if let Some(p) = RunOf::<P>::RUN {
            return by_ratio(self.value, self.scale, p, r);
        }
        let k = Scale::<P>::exact(Scale::<P>::RECIP)?;
        limbs::scaled(&Both(&self.scale, &k), self.value, r)
    }

    /// A [Fixed] at the step `scale`, exactly: `None` where it isn't a whole number of `scale` or doesn't fit `T`.
    ///
    /// ```
    /// # #[cfg(feature = "time")] {
    /// use exact_units::{Fixed, RunScale, Ticks, prefix::Milli, units::Second};
    ///
    /// let base = RunScale::ratio(1, 48_000).unwrap();
    /// assert_eq!(Ticks::<u32>::from_fixed(Fixed::<Second, Milli, u16>::new(20), base).map(|t| t.get()), Some(960));
    /// # }
    /// ```
    #[must_use]
    pub fn from_fixed<P: FixedScale, T2: Storage + Int, B: Bounds<T2>>(
        f: Fixed<U, P, T2, B>,
        scale: RunScale,
    ) -> Option<Self> {
        let (neg, mag, exact) = from_steps_of::<P, T2>(f.get(), scale, Rounding::Truncate)?;
        if !exact {
            return None;
        }
        Some(Self::new(mag.settle(neg, Overflow::Checked)?, scale))
    }

    /// A [Fixed] at the step `scale`, rounded by `r`: `None` where it doesn't fit `T`.
    #[must_use]
    pub fn from_fixed_rounded<P: FixedScale, T2: Storage + Int, B: Bounds<T2>>(
        f: Fixed<U, P, T2, B>,
        scale: RunScale,
        r: Rounding,
    ) -> Option<Self> {
        let (neg, mag, _) = from_steps_of::<P, T2>(f.get(), scale, r)?;
        Some(Self::new(mag.settle(neg, Overflow::Checked)?, scale))
    }

    /// `self + rhs` at the same step: `None` where the steps differ or the sum doesn't fit `T`.
    ///
    /// ```
    /// # #[cfg(feature = "time")] {
    /// use exact_units::{RunScale, Ticks};
    ///
    /// let a = RunScale::ratio(1, 90_000).unwrap();
    /// let b = RunScale::ratio(1, 48_000).unwrap();
    /// assert_eq!(Ticks::<u32>::new(3003, a).checked_add(Ticks::new(3003, a)).map(|t| t.get()), Some(6006));
    /// // different steps: rescale first, by name
    /// assert_eq!(Ticks::<u32>::new(3003, a).checked_add(Ticks::new(1600, b)), None);
    /// # }
    /// ```
    #[must_use]
    pub fn checked_add(self, rhs: Self) -> Option<Self> {
        if self.scale != rhs.scale {
            return None;
        }
        Some(Self::new(self.value.checked_add(rhs.value)?, self.scale))
    }

    /// `self − rhs` at the same step: `None` where the steps differ or the difference doesn't fit `T`.
    #[must_use]
    pub fn checked_sub(self, rhs: Self) -> Option<Self> {
        if self.scale != rhs.scale {
            return None;
        }
        Some(Self::new(self.value.checked_sub(rhs.value)?, self.scale))
    }

    /// `self · n`, `None` where it doesn't fit `T`.
    #[must_use]
    pub fn checked_mul(self, n: T) -> Option<Self> {
        Some(Self::new(self.value.checked_mul(n)?, self.scale))
    }
}

/// `x` at the step `from` in steps of `to`: through `from / to` where it has a form here, else on the limbs.
fn by_ratio<T: Int>(x: T, from: RunScale, to: RunScale, r: Rounding) -> Option<(bool, Mag, bool)> {
    if let Some(k) = from.checked_div(to) {
        return k.scaled(x, r);
    }
    limbs::scaled(&Both(&from, &to.recip()), x, r)
}

/// `x` steps of `P` in steps of `to`, as [Scaled::in_steps_of] the other way.
fn from_steps_of<P: FixedScale, T: Int>(
    x: T,
    to: RunScale,
    r: Rounding,
) -> Option<(bool, Mag, bool)> {
    if let Some(p) = RunOf::<P>::RUN {
        return by_ratio(x, p, to, r);
    }
    let k = Scale::<P>::exact(Scale::<P>::SCALE)?;
    limbs::scaled(&Both(&k, &to.recip()), x, r)
}

/// By exact value, at any two steps: 3 at 1/25 is 6 at 1/50. Always `Some`: the kernel's room holds both sides for
/// every scale and integer here; an `Option` so that a bug there shows as `None`, never as a wrong order.
impl<U, T: Storage + Int> PartialOrd for Scaled<U, T> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        let (na, la) = self.value.to_parts();
        let (nb, lb) = other.value.to_parts();
        let (sa, sb) = (limbs::sign(na, la), limbs::sign(nb, lb));
        if sa != sb || sa == 0 {
            return Some(sa.cmp(&sb));
        }
        let m = cmp_magnitudes(la, self.scale, lb, other.scale)?;
        Some(if sa < 0 { m.reverse() } else { m })
    }
}

/// The magnitude `a · sa` against `b · sb`: in `u64` where both are of 32 bits, else on the limbs.
#[expect(
    clippy::arithmetic_side_effects,
    reason = "u32 · u32 fits u64 (a magnitude and an odd part); the exponents are i8, their difference fits i32"
)]
fn cmp_magnitudes(a: [u32; 4], sa: RunScale, b: [u32; 4], sb: RunScale) -> Option<Ordering> {
    if let ([a, 0, 0, 0], [b, 0, 0, 0]) = (a, b)
        && let Some(k) = sa.checked_div(sb)
        && let Some(l) = u64::from(a).checked_mul(u64::from(k.num))
    {
        // a · n / d · 2^e against b: a · n · 2^e against b · d
        return Some(cmp_shifted(
            l,
            u64::from(b) * u64::from(k.den),
            i32::from(k.pow2),
        ));
    }
    cmp_rescaled(
        &Both(&sa, &sb.recip()),
        &mut Limbs::of(a),
        &mut Limbs::of(b),
    )
}

impl<U, T: Copy> Clone for Scaled<U, T> {
    #[inline(always)]
    fn clone(&self) -> Self {
        *self
    }
}

impl<U, T: Copy> Copy for Scaled<U, T> {}

/// `Second(154112·1/15360 s u64)`, as a [Fixed]'s debug form with the step.
impl<U: Unit, T: Storage + Int> fmt::Debug for Scaled<U, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}(", U::EXPR.name())?;
        fmt::Display::fmt(self, f)?;
        write!(f, " {})", T::NAME)
    }
}

#[cfg(feature = "defmt")]
impl<U, T: Storage + Int + defmt::Format> defmt::Format for Scaled<U, T> {
    fn format(&self, f: defmt::Formatter<'_>) {
        defmt::write!(f, "{}·{}", self.value, self.scale);
    }
}

impl<U, T: Storage + Int> PartialEq for Scaled<U, T> {
    fn eq(&self, other: &Self) -> bool {
        self.partial_cmp(other) == Some(Ordering::Equal)
    }
}

/// The number as stored, its step after a `·` unless it is 1, and the unit: `154112·1/15360 s`, `3 s`.
impl<U: Unit, T: Storage + Int> fmt::Display for Scaled<U, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.value.fmt_number(f)?;
        if self.scale != RunScale::ONE {
            write!(f, "·{}", self.scale)?;
        }
        write_unit(Prefix::Unit, &U::EXPR, f)
    }
}

#[cfg(kani)]
mod proofs {
    use super::RunScale;
    use crate::{Overflow, Rounding};

    /// Every mode.
    const MODES: [Rounding; 6] = [
        Rounding::Truncate,
        Rounding::Floor,
        Rounding::Ceil,
        Rounding::NearestEven,
        Rounding::NearestAway,
        Rounding::NearestCeil,
    ];

    /// The kernel's `u32` path alone: every `u8` count of either sign at every scale of odd `u8` parts and an exponent
    /// within ±8 answers, and its answer `q` is `±m · num · 2^k / den` rounded as the mode promises, checked by
    /// multiplications in `i64` (`e = p − q·d` against the step `d`), and exact exactly where `e` is 0. The scale is
    /// built field by field, so no gcd runs: the arithmetic doesn't need the form reduced. The limbs and the gcd are
    /// covered by the properties of `tests/props/run_scale.rs`.
    #[kani::proof]
    fn scaled_u32() {
        let (m, num, den, k, neg): (u8, u8, u8, i8, bool) = (
            kani::any(),
            kani::any(),
            kani::any(),
            kani::any(),
            kani::any(),
        );
        let i: usize = kani::any();
        kani::assume(num % 2 == 1 && den % 2 == 1 && (-8..=8).contains(&k) && i < MODES.len());
        let Some(&r) = MODES.get(i) else { return };
        let s = RunScale {
            num: num.into(),
            den: den.into(),
            pow2: k,
        };
        let Some((mag, exact)) = s.scaled_u32(m.into(), neg, r) else {
            panic!("a u8 count at an 8-bit scale fits the u32 path");
        };
        let Some(q) = mag.settle::<i64>(neg, Overflow::Checked) else {
            panic!("the quotient fits i64");
        };
        let shift = u32::from(k.unsigned_abs());
        let mut p = i64::from(m) * i64::from(num);
        let mut d = i64::from(den);
        if k >= 0 {
            p <<= shift;
        } else {
            d <<= shift;
        }
        if neg {
            p = -p;
        }
        let e = p - q * d;
        assert_eq!(exact, e == 0);
        let off = e.abs();
        match r {
            Rounding::Floor => assert!((0..d).contains(&e)),
            Rounding::Ceil => assert!(e <= 0 && e > -d),
            Rounding::Truncate => assert!(off < d && (e == 0 || (e > 0) == (p > 0))),
            _ => {
                assert!(off <= d - off);
                if off == d - off {
                    match r {
                        Rounding::NearestEven => assert!(q % 2 == 0),
                        Rounding::NearestCeil => assert!(e < 0),
                        _ => assert!((e > 0) != (p > 0)),
                    }
                }
            }
        }
    }
}
