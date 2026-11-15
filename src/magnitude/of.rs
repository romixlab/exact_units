//! [MagnitudeOf]: a [Magnitude] with its unit in the type, a clock's rate or resolution known at run time.

use super::Magnitude;
use crate::exact::{magnitude, recip_magnitude};
use crate::{Exact, Fraction, Ratio64, Unit};
use core::fmt;
use core::hash::{Hash, Hasher};
use core::marker::PhantomData;

#[cfg(feature = "shrink_wrap")]
use shrink_wrap::prelude::*;

/// An exact positive value of `U` known at run time, held as a [Magnitude]: a clock's rate
/// `MagnitudeOf<Per<Tick, Second>>` (60000/1001 tick/s) and its resolution `MagnitudeOf<Second>` (1/32768 s). The
/// unit is in the type, so a rate is never passed as a resolution; the value is canonical, so two spellings of one
/// rate are `==`.
///
/// Free to and from a [Magnitude] ([new](Self::new), [magnitude](Self::magnitude), `From` both ways), and on the
/// wire exactly the magnitude's bytes (`transparent`): a field declared as a plain `Magnitude` reads the same bytes.
/// `{}` writes the exact value in its unit as an [Exact] does (`59.[940059] tick/s`, `30.517578125 µs`), a value with
/// π or a root as the magnitude's factor (`1·π/180 rad`).
///
/// ```
/// # #[cfg(all(feature = "time", feature = "counting"))] {
/// use exact_units::{Magnitude, MagnitudeOf, units::{Per, Second, Tick}};
///
/// // NTSC's frame clock and a 32 kHz watch crystal
/// let rate = MagnitudeOf::<Per<Tick, Second>>::new(Magnitude::ratio(60_000, 1001));
/// let resolution = MagnitudeOf::<Second>::new(Magnitude::ratio(1, 32_768));
/// assert_eq!(rate.to_string(), "59.[940059] tick/s");
/// assert_eq!(resolution.to_string(), "30.517578125 µs");
/// // the tick's length is the rate's reciprocal
/// let tick = rate.recip::<Per<Second, Tick>>().unwrap();
/// assert_eq!(format!("{tick:#}"), "0.01668[3] s/tick");
/// assert_eq!(rate.magnitude(), Magnitude::ratio(60_000, 1001));
/// # }
/// ```
///
/// A rate passed as a resolution doesn't compile:
///
/// ```compile_fail
/// use exact_units::{Magnitude, MagnitudeOf, units::{Per, Second, Tick}};
/// let rate = MagnitudeOf::<Per<Tick, Second>>::new(Magnitude::ratio(60_000, 1001));
/// let _: MagnitudeOf<Second> = rate;
/// ```
#[cfg_attr(
    feature = "shrink_wrap",
    derive_shrink_wrap(borrowed, owned(feature = "std"), transparent)
)]
#[repr(transparent)]
pub struct MagnitudeOf<U> {
    /// The value in `U`.
    value: Magnitude,
    /// The unit, in the type only.
    _unit: PhantomData<U>,
}

impl<U> MagnitudeOf<U> {
    /// `m` of `U`.
    #[must_use]
    pub const fn new(m: Magnitude) -> Self {
        Self {
            value: m,
            _unit: PhantomData,
        }
    }

    /// `num / den` of `U`, `None` for a part of 0 or one with more primes than a [Magnitude] holds.
    #[must_use]
    pub const fn checked_ratio(num: u64, den: u64) -> Option<Self> {
        match Magnitude::checked_ratio(num, den) {
            Ok(m) => Some(Self::new(m)),
            Err(_) => None,
        }
    }

    /// The value in `U`, a [Magnitude].
    #[must_use]
    pub const fn magnitude(self) -> Magnitude {
        self.value
    }

    /// The value as a fraction of `u64`s, `None` where it has none (π, a root, a part past `u64`).
    #[must_use]
    pub const fn to_ratio64(self) -> Option<Ratio64> {
        Ratio64::of_magnitude(&self.value)
    }
}

impl<U: Unit> MagnitudeOf<U> {
    /// The value as an [Exact] of `U`, `None` for one with π, ln 2, ln 10 or a root, or past `u128`.
    #[must_use]
    pub const fn to_exact(self) -> Option<Exact<U>> {
        Exact::from_scaled(Fraction::ONE, &self.value)
    }

    /// The same value in `U2`, a unit of the same dimension and kind: a rate in tick/s as tick/ms. `None` where the
    /// product leaves what a [Magnitude] holds; units of other dimensions or kinds, or with an offset, don't compile.
    #[must_use]
    pub const fn convert<U2: Unit>(self) -> Option<MagnitudeOf<U2>> {
        match self.value.checked_mul(const { magnitude::<U, U2>() }) {
            Ok(m) => Some(MagnitudeOf::new(m)),
            Err(_) => None,
        }
    }

    /// `1 / self` in `U2`, a unit of the reciprocal dimension: a clock's period from its rate. `None` where the
    /// product leaves what a [Magnitude] holds; a `U2` of another dimension doesn't compile.
    #[must_use]
    pub const fn recip<U2: Unit>(self) -> Option<MagnitudeOf<U2>> {
        match self
            .value
            .recip()
            .checked_mul(const { recip_magnitude::<U, U2>() })
        {
            Ok(m) => Some(MagnitudeOf::new(m)),
            Err(_) => None,
        }
    }
}

impl<U> From<Magnitude> for MagnitudeOf<U> {
    fn from(m: Magnitude) -> Self {
        Self::new(m)
    }
}

impl<U> From<MagnitudeOf<U>> for Magnitude {
    fn from(m: MagnitudeOf<U>) -> Self {
        m.value
    }
}

impl<U> Clone for MagnitudeOf<U> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<U> Copy for MagnitudeOf<U> {}

impl<U> PartialEq for MagnitudeOf<U> {
    fn eq(&self, other: &Self) -> bool {
        self.value == other.value
    }
}

impl<U> Eq for MagnitudeOf<U> {}

impl<U> Hash for MagnitudeOf<U> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.value.hash(state);
    }
}

/// The exact value in its unit, as [Exact] writes it, the format's flags passed on; a value without a fraction
/// as `1` and the magnitude's factor.
impl<U: Unit> fmt::Display for MagnitudeOf<U> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(e) = self.to_exact() {
            return fmt::Display::fmt(&e, f);
        }
        write!(f, "1{}", self.value)?;
        crate::tree::write_unit(crate::Prefix::Unit, &U::EXPR, f)
    }
}

impl<U: Unit> fmt::Debug for MagnitudeOf<U> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "MagnitudeOf({self:#})")
    }
}

#[cfg(all(feature = "serde", feature = "std"))]
impl<U> serde::Serialize for MagnitudeOf<U> {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.value.serialize(serializer)
    }
}

#[cfg(all(feature = "serde", feature = "std"))]
impl<'de, U> serde::Deserialize<'de> for MagnitudeOf<U> {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Magnitude::deserialize(deserializer).map(Self::new)
    }
}

#[cfg(all(test, feature = "time", feature = "counting", feature = "angle"))]
mod tests {
    use super::MagnitudeOf;
    use crate::units::{Degree, Hertz, Per, Radian, Second, Tick};
    use crate::{Exact, Magnitude, Ratio64};

    type Rate = MagnitudeOf<Per<Tick, Second>>;

    #[test]
    fn converts_and_inverts() {
        let rate = Rate::new(Magnitude::ratio(60_000, 1001));
        let hz: MagnitudeOf<Per<Tick, Second>> = rate.convert().unwrap();
        assert_eq!(hz, rate);
        let period = rate.recip::<Per<Second, Tick>>().unwrap();
        assert_eq!(period.magnitude(), Magnitude::ratio(1001, 60_000));
        assert_eq!(period.recip::<Per<Tick, Second>>(), Some(rate));
        let f = MagnitudeOf::<Hertz>::new(Magnitude::ratio(32_768, 1));
        assert_eq!(
            f.recip::<Second>().map(MagnitudeOf::magnitude),
            Some(Magnitude::ratio(1, 32_768))
        );
        assert_eq!(rate.to_exact(), Some(Exact::new(60_000, 1001)));
        assert_eq!(rate.to_ratio64(), Ratio64::new(60_000, 1001));
        assert_eq!(Rate::checked_ratio(60_000, 0), None);
        assert_eq!(Rate::checked_ratio(120_000, 2002), Some(rate));
        assert_eq!(Magnitude::from(rate), rate.magnitude());
        assert_eq!(Rate::from(rate.magnitude()), rate);
    }

    #[test]
    fn writes_what_has_no_fraction() {
        let deg = MagnitudeOf::<Degree>::new(Magnitude::ratio(1, 1));
        let rad = deg.convert::<Radian>().unwrap();
        assert_eq!(rad.to_exact(), None);
        assert_eq!(rad.to_string(), "1·π/180 rad");
        assert_eq!(format!("{rad:?}"), "MagnitudeOf(1·π/180 rad)");
        let rate = Rate::new(Magnitude::ratio(60_000, 1001));
        assert_eq!(format!("{rate:?}"), "MagnitudeOf(59.[940059] tick/s)");
    }

    #[cfg(all(feature = "serde", feature = "std"))]
    #[test]
    fn serde_as_its_magnitude() {
        let rate = Rate::new(Magnitude::ratio(60_000, 1001));
        let text = serde_json::to_string(&rate).unwrap();
        assert_eq!(text, serde_json::to_string(&rate.magnitude()).unwrap());
        assert_eq!(serde_json::from_str::<Rate>(&text).unwrap(), rate);
    }
}
