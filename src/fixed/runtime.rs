//! The bridge between a `Fixed` and a number at a [Prefix] known only at run time (a `ww_si::Quantity`, a value
//! read with its prefix): exact integer arithmetic on the kernel, rounded and settled by name. No float code.

use super::{Bounds, Fixed, FixedScale, Overflow, Rounding, Storage};
use crate::Prefix;
use crate::magnitude::{Exact, whole};
use crate::num::Int;
use crate::num::limbs::{Both, scaled};
use crate::{Magnitude, Unit};
use core::marker::PhantomData;

/// `P`'s scale and its reciprocal as `'static` references, for the kernel to read in place; a compile error where the
/// scale isn't a fraction of integers: a `Fixed` of such a scale (π/180) and integer storage has no exact run-time
/// conversion to a prefix.
struct Scale<P>(PhantomData<P>);

impl<P: FixedScale> Scale<P> {
    const SCALE: &'static Magnitude = &P::SCALE;

    const RECIP: &'static Magnitude = &P::SCALE.recip();

    const WHOLE: () = assert!(
        whole(&P::SCALE),
        "the scale is not a fraction of integers: no exact conversion to a run-time prefix"
    );

    /// `m` for the kernel, after `WHOLE` has checked it.
    fn exact(m: &'static Magnitude) -> Option<Exact<'static>> {
        let () = Self::WHOLE;
        Exact::of(m)
    }
}

impl<U: Unit, P: FixedScale, T: Storage + Int, B: Bounds<T>> Fixed<U, P, T, B> {
    /// `x` in units of `prefix` (of `U`) as this type: rounded by `r` where it isn't a whole number of `P`, settled
    /// into `T` by `o`, and then within `B`: [Overflow::Saturate] clamps to `B`'s ends, otherwise a value outside `B`
    /// is `None`. `None` also for [Prefix::Unknown], and where `o` is [Overflow::Checked] and the value doesn't fit
    /// `T`. Exact for every prefix and every integer: one rounding of the exact product, no float.
    ///
    /// A scale `P` that isn't a fraction of integers (π/180) doesn't compile here:
    ///
    /// ```compile_fail
    /// # #[cfg(feature = "dimensionless")] {
    /// use exact_units::{Fixed, FixedScale, Magnitude, Overflow, Prefix, Rounding, units::Ratio};
    ///
    /// #[derive(Copy, Clone, Default)]
    /// struct Pi;
    /// impl FixedScale for Pi {
    ///     const SCALE: Magnitude = Magnitude::PI;
    ///     const SYMBOL: &'static str = "";
    ///     const NAME: &'static str = "Pi";
    ///     const RUNTIME: Option<Prefix> = None;
    /// }
    /// let _ = Fixed::<Ratio, Pi, i32>::from_prefix(1, Prefix::Unit, Rounding::Floor, Overflow::Checked);
    /// # }
    /// ```
    ///
    /// ```
    /// # #[cfg(feature = "electrical")] {
    /// use exact_units::{Fixed, Overflow, Prefix, Range, Rounding, prefix::{FullScale, Milli, Rational}, units::Volt};
    ///
    /// // a 12-bit ADC at 3.3 V: 1650 mV is code 2048, 1 V is 1241.21.., to nearest 1241
    /// type Adc = Fixed<Volt, FullScale<Rational<33, 10>, 12>, u16>;
    /// let r = Rounding::NearestEven;
    /// assert_eq!(Adc::from_prefix(1650, Prefix::Milli, r, Overflow::Checked), Some(Adc::new(2048)));
    /// assert_eq!(Adc::from_prefix(1, Prefix::Unit, r, Overflow::Checked), Some(Adc::new(1241)));
    /// // 100 V is past u16 at this scale
    /// assert_eq!(Adc::from_prefix(100, Prefix::Unit, r, Overflow::Checked), None);
    /// assert_eq!(Adc::from_prefix(100, Prefix::Unit, r, Overflow::Saturate), Some(Adc::new(u16::MAX)));
    /// // bounds: a rail of 0 to 3.3 V in mV, 5 V clamps to its top or is refused
    /// type Rail = Fixed<Volt, Milli, i16, Range<0, 3300>>;
    /// assert_eq!(Rail::from_prefix(5, Prefix::Unit, r, Overflow::Saturate).map(Rail::get), Some(3300));
    /// assert_eq!(Rail::from_prefix(5, Prefix::Unit, r, Overflow::Checked), None);
    /// // and back: code 2048 is 1650 mV
    /// assert_eq!(Adc::new(2048).to_prefix::<i32>(Prefix::Milli, r, Overflow::Checked), Some(1650));
    /// # }
    /// ```
    #[must_use]
    pub fn from_prefix<X: Int>(x: X, prefix: Prefix, r: Rounding, o: Overflow) -> Option<Self> {
        let (neg, mag, _) = if let Some(p) = P::RUNTIME {
            prefix.ratio_to(p)?.scaled(x, r)?
        } else {
            let k = Scale::<P>::exact(Scale::<P>::RECIP)?;
            scaled(&Both(&prefix.ratio_to(Prefix::Unit)?, &k), x, r)?
        };
        let v: T = mag.settle(neg, o)?;
        if o == Overflow::Saturate {
            Some(Self::clamped(v))
        } else {
            Self::try_new(v).ok()
        }
    }

    /// The value in units of `prefix` (of `U`) as an `X`: rounded by `r` where it isn't a whole number of them,
    /// settled into `X` by `o`. `None` for [Prefix::Unknown], and where `o` is [Overflow::Checked] and the value
    /// doesn't fit `X`. Exact as [from_prefix](Self::from_prefix) is.
    ///
    /// ```
    /// # #[cfg(feature = "time")] {
    /// use exact_units::{Fixed, Overflow, Prefix, Rounding, prefix::Micro, units::Second};
    ///
    /// // 1234567 µs in ms, to nearest; and into a u8, wrapped as a counter would
    /// let t = Fixed::<Second, Micro, u32>::new(1_234_567);
    /// assert_eq!(t.to_prefix::<u32>(Prefix::Milli, Rounding::NearestEven, Overflow::Checked), Some(1235));
    /// assert_eq!(t.to_prefix::<u8>(Prefix::Milli, Rounding::Truncate, Overflow::Wrap), Some((1234 % 256) as u8));
    /// # }
    /// ```
    #[must_use]
    pub fn to_prefix<X: Int>(self, prefix: Prefix, r: Rounding, o: Overflow) -> Option<X> {
        let (neg, mag, _) = if let Some(p) = P::RUNTIME {
            p.ratio_to(prefix)?.scaled(self.value, r)?
        } else {
            let k = Scale::<P>::exact(Scale::<P>::SCALE)?;
            scaled(&Both(&k, &Prefix::Unit.ratio_to(prefix)?), self.value, r)?
        };
        mag.settle(neg, o)
    }
}
