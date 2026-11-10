//! Points whose number is a [Scaled]: a position on a clock whose tick is known only at run time, a stream's
//! presentation time at its own time base. Only for a [Linear] origin: a [Modulo](crate::Modulo) origin's period
//! is a compile-time number of its unit, and at a run-time step it is a whole number of steps only for some steps.
//! A counter that wraps at its storage (RTP's 32-bit timestamp at a clock rate the session states) is a
//! [Linear] point known modulo 2^BITS, with named wrapping methods and [unwrap_near](Point::unwrap_near).

use super::{Linear, Origin, Point};
use crate::num::Int;
use crate::{Fixed, FixedScale, Rounding, RunScale, Scaled, Storage, Unit};
use core::fmt;

impl<O: Origin<Unit = U>, U, T: Storage + Int> Point<O, Scaled<U, T>> {
    /// The point `value` steps of `scale` from the origin.
    #[must_use]
    pub const fn at(value: T, scale: RunScale) -> Self {
        Self::new(Scaled::new(value, scale))
    }

    /// The number as a [Scaled]: how far from the origin. Not `get`, which a point of a [Fixed] has: two
    /// inherent methods of one name would make `Point::get` ambiguous where the number's type isn't known yet.
    #[must_use]
    pub const fn coord(self) -> Scaled<U, T> {
        self.coord
    }

    /// The number of steps from the origin (a point of a [Fixed] says `raw`, see [coord](Self::coord)).
    #[must_use]
    pub const fn steps(self) -> T {
        self.coord.get()
    }

    /// The step.
    #[must_use]
    pub const fn scale(self) -> RunScale {
        self.coord.scale()
    }
}

impl<O, U, T> Point<O, Scaled<U, T>>
where
    O: Origin<Unit = U, Modulus = Linear>,
    T: Storage + Int,
{
    /// `self + v`: `None` where `v` is at another step or the sum doesn't fit `T`.
    ///
    /// ```
    /// # #[cfg(feature = "time")] {
    /// use exact_units::{Point, RunScale, Ticks, origins::DeviceBoot};
    ///
    /// // a stream's presentation time at its own base, 1/15360 s
    /// let base = RunScale::ratio(1, 15_360).unwrap();
    /// let pts = Point::<DeviceBoot, Ticks<u64>>::at(512, base);
    /// assert_eq!(pts.checked_add(Ticks::new(512, base)).map(Point::steps), Some(1024));
    /// // another step is refused, not rescaled behind the caller's back
    /// assert_eq!(pts.checked_add(Ticks::new(1, RunScale::ratio(1, 90_000).unwrap())), None);
    /// # }
    /// ```
    #[must_use]
    pub fn checked_add(self, v: Scaled<U, T>) -> Option<Self> {
        self.coord.checked_add(v).map(Self::new)
    }

    /// `self − v`: `None` where `v` is at another step or the difference doesn't fit `T`.
    #[must_use]
    pub fn checked_sub(self, v: Scaled<U, T>) -> Option<Self> {
        self.coord.checked_sub(v).map(Self::new)
    }

    /// The vector from `rhs` to `self`: `None` where the two are at different steps or it doesn't fit `T` (an
    /// unsigned `T` below `rhs`).
    #[must_use]
    pub fn checked_diff(self, rhs: Self) -> Option<Scaled<U, T>> {
        self.coord.checked_sub(rhs.coord)
    }

    /// The same point at the step `to`, exactly: `None` where it isn't a whole number of `to` or doesn't fit `T2`.
    #[must_use]
    pub fn rescale<T2: Storage + Int>(self, to: RunScale) -> Option<Point<O, Scaled<U, T2>>> {
        self.coord.rescale(to).map(Point::new)
    }

    /// The same point at the step `to`, rounded by `r`: `None` where it doesn't fit `T2`.
    #[must_use]
    pub fn rescale_rounded<T2: Storage + Int>(
        self,
        to: RunScale,
        r: Rounding,
    ) -> Option<Point<O, Scaled<U, T2>>> {
        self.coord.rescale_rounded(to, r).map(Point::new)
    }

    /// The same point on the same origin with a [Fixed] number in steps of `P`, exactly: `None` where it isn't a
    /// whole number of them or doesn't fit `T2`.
    ///
    /// ```
    /// # #[cfg(feature = "time")] {
    /// use exact_units::{Fixed, Point, RunScale, Ticks, origins::DeviceBoot, prefix::Rational, units::Second};
    ///
    /// let base = RunScale::ratio(1001, 60_000).unwrap();
    /// let t = Point::<DeviceBoot, Ticks<u32>>::at(4, base);
    /// let rtp: Point<DeviceBoot, Fixed<Second, Rational<1, 90_000>, u32>> = t.to_fixed().unwrap();
    /// assert_eq!(rtp.raw(), 6006);
    /// # }
    /// ```
    #[must_use]
    pub fn to_fixed<P: FixedScale, T2: Storage + Int>(self) -> Option<Point<O, Fixed<U, P, T2>>>
    where
        U: Unit,
    {
        self.coord.to_fixed().map(Point::new)
    }

    /// The same point with a [Fixed] number in steps of `P`, rounded by `r`: `None` where it doesn't fit `T2`.
    #[must_use]
    pub fn to_fixed_rounded<P: FixedScale, T2: Storage + Int>(
        self,
        r: Rounding,
    ) -> Option<Point<O, Fixed<U, P, T2>>>
    where
        U: Unit,
    {
        self.coord.to_fixed_rounded(r).map(Point::new)
    }

    /// A point with a [Fixed] number at the step `scale`, exactly: `None` where it isn't a whole number of `scale`
    /// or doesn't fit `T`.
    #[must_use]
    pub fn from_fixed<P: FixedScale, T2: Storage + Int>(
        p: Point<O, Fixed<U, P, T2>>,
        scale: RunScale,
    ) -> Option<Self> {
        Scaled::from_fixed(p.coord, scale).map(Self::new)
    }
}

/// The wrapping counter's methods for one unsigned storage and its signed twin.
macro_rules! wrapping {
    ($($t:ty => $s:ty),*) => {$(
        impl<O, U> Point<O, Scaled<U, $t>>
        where
            O: Origin<Unit = U, Modulus = Linear>,
        {
            #[doc = concat!("`self + v` modulo 2^", stringify!($t), "'s bits, like a counter that wraps: `None` only \
                             where `v` is at another step.")]
            ///
            /// ```
            /// # #[cfg(feature = "time")] {
            /// use exact_units::{Point, RunScale, Ticks, origins::DeviceBoot};
            ///
            /// // RTP's timestamp at 48 kHz (RFC 3550 5.1, the rate from the session): it wraps at 2³²
            /// let rate = RunScale::ratio(1, 48_000).unwrap();
            /// let t = Point::<DeviceBoot, Ticks<u32>>::at(u32::MAX - 99, rate);
            /// let next = t.wrapping_add(Ticks::new(960, rate)).unwrap();
            /// assert_eq!(next.steps(), 860);
            /// assert_eq!(next.wrapping_diff(t).map(|d| d.get()), Some(960));
            /// assert_eq!(t.wrapping_diff(next).map(|d| d.get()), Some(-960));
            /// # }
            /// ```
            #[must_use]
            pub fn wrapping_add(self, v: Scaled<U, $t>) -> Option<Self> {
                (self.scale() == v.scale()).then(|| Self::at(self.steps().wrapping_add(v.get()), self.scale()))
            }

            #[doc = concat!("`self − v` modulo 2^", stringify!($t), "'s bits: `None` only where `v` is at another \
                             step.")]
            #[must_use]
            pub fn wrapping_sub(self, v: Scaled<U, $t>) -> Option<Self> {
                (self.scale() == v.scale()).then(|| Self::at(self.steps().wrapping_sub(v.get()), self.scale()))
            }

            /// The vector from `earlier` to `self` the short way round the counter's circle: within
            #[doc = concat!("`", stringify!($s), "`, a half turn negative. `None` only where the two are at \
                             different steps.")]
            #[must_use]
            #[expect(
                clippy::cast_possible_wrap,
                reason = "the difference modulo 2^BITS read as two's complement is the short way round"
            )]
            pub fn wrapping_diff(self, earlier: Self) -> Option<Scaled<U, $s>> {
                (self.scale() == earlier.scale())
                    .then(|| Scaled::new(self.steps().wrapping_sub(earlier.steps()) as $s, self.scale()))
            }

            /// The full count whose low bits are this counter's, nearest to `reference` (the last full point a
            /// receiver knows): the step ahead or back from it the short way round, as a stateless unwrapper would
            /// take it. `None` where the two are at different steps, or where that point is below 0 or past
            /// `u64`.
            ///
            /// ```
            /// # #[cfg(feature = "time")] {
            /// use exact_units::{Point, RunScale, Ticks, origins::DeviceBoot};
            ///
            /// let rate = RunScale::ratio(1, 90_000).unwrap();
            /// let last = Point::<DeviceBoot, Ticks<u64>>::at(u64::from(u32::MAX) - 1000, rate);
            /// // the next timestamp, past the wrap of the 32-bit field
            /// let t = Point::<DeviceBoot, Ticks<u32>>::at(2002, rate);
            /// assert_eq!(t.unwrap_near(last).map(Point::steps), Some(u64::from(u32::MAX) + 2003));
            /// # }
            /// ```
            #[must_use]
            #[expect(
                clippy::cast_possible_truncation,
                reason = "the low bits of the reference are what is compared"
            )]
            pub fn unwrap_near(self, reference: Point<O, Scaled<U, u64>>) -> Option<Point<O, Scaled<U, u64>>> {
                if self.scale() != reference.scale() {
                    return None;
                }
                let low = Point::<O, Scaled<U, $t>>::at(reference.steps() as $t, self.scale());
                let step = i64::from(self.wrapping_diff(low)?.get());
                let full = reference.steps().checked_add_signed(step)?;
                Some(Point::at(full, self.scale()))
            }
        }
    )*};
}

wrapping!(u8 => i8, u16 => i16, u32 => i32);

/// `DeviceBoot(154112·1/15360 s, u64)`: the origin's name and the number as the [Scaled] formats it.
impl<O: Origin<Unit = U>, U: Unit, T: Storage + Int> fmt::Debug for Point<O, Scaled<U, T>> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}({}, {})", O::NAME, self.coord, T::NAME)
    }
}
