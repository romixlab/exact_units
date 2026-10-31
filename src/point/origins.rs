//! The origins of this crate: temperature scales (K, °C, °F, °R), absolute and gauge pressure, angles that wrap at a
//! turn, the Unix epoch. Each is a zero-size marker declared with [origin!](crate::origin); a user crate declares its own the same
//! way (a register formula, a vehicle bus' offset).

#[allow(unused_imports)]
use crate::{Magnitude, Offset, Origin};

#[cfg(feature = "time")]
mod time {
    use super::*;
    use crate::units::Second;
    use crate::{Micros, Millis, Nanos, Point, Secs};

    crate::origin! {
        /// The Unix epoch, 1970-01-01 00:00:00 UTC: a point from it is a moment in POSIX time, UTC with leap seconds
        /// not counted, as `CLOCK_REALTIME`, `time_t`, journald's `__REALTIME_TIMESTAMP` and most log formats count
        /// it. Every day is 86400 s on this scale: a leap second repeats one (or the clock smears it), so the
        /// difference of two such points is the elapsed time only up to the leap seconds between them (27 from 1972
        /// to 2017).
        ///
        /// Clocks that count every SI second (TAI, GPS = TAI − 19 s) and UTC with its leap-second table come with
        /// WireWeaver's STD-65 as clocks of their own; this origin stays POSIX time, and a conversion to them goes
        /// through the table, never at compile time.
        pub struct UnixEpoch { unit: Second, symbol: "s UTC", offset: Offset::ZERO }
    }

    /// A moment in POSIX time in whole seconds since the [UnixEpoch]: `time_t`, the bytes of a `T`.
    pub type UnixSecs<T = u64> = Point<UnixEpoch, Secs<T>>;

    /// A moment in POSIX time in ms since the [UnixEpoch], the bytes of a `T`.
    pub type UnixMillis<T = u64> = Point<UnixEpoch, Millis<T>>;

    /// A moment in POSIX time in µs since the [UnixEpoch] (journald's `__REALTIME_TIMESTAMP`), the bytes of a `T`.
    pub type UnixMicros<T = u64> = Point<UnixEpoch, Micros<T>>;

    /// A moment in POSIX time in ns since the [UnixEpoch], the bytes of a `T`; a `u64` holds them until 2554.
    pub type UnixNanos<T = u64> = Point<UnixEpoch, Nanos<T>>;
}

#[cfg(feature = "time")]
pub use time::*;

#[cfg(feature = "thermal")]
mod thermal {
    use super::*;
    use crate::units::Kelvin;

    crate::origin! {
        /// Thermodynamic temperature, from absolute zero: `Point<AbsoluteZero, Fixed<Kelvin, Milli, u32>>` is a
        /// temperature in mK.
        pub struct AbsoluteZero { unit: Kelvin, symbol: "K", offset: Offset::ZERO }
    }

    crate::origin! {
        /// Degrees Celsius: 0 °C is 273.15 K, a degree is 1 K (SI Brochure 2.3.4).
        pub struct Celsius { unit: Kelvin, symbol: "°C", offset: Offset::decimal(27315, -2) }
    }

    crate::origin! {
        /// Degrees Rankine: from absolute zero, a degree is 5/9 K (NIST SP 811 B.8). Numbers in steps of °R are
        /// `Fixed<Kelvin, Rational<5, 9>, T>`.
        pub struct Rankine { unit: Kelvin, symbol: "°R", offset: Offset::ZERO, step: Magnitude::ratio(5, 9) }
    }

    crate::origin! {
        /// Degrees Fahrenheit: 0 °F is 459.67 °R, so 45967/180 K (255.372.. K, NIST SP 811 B.8); a degree is 5/9 K.
        /// Numbers in steps of °F are `Fixed<Kelvin, Rational<5, 9>, T>`, tenths `Rational<1, 18>`.
        pub struct Fahrenheit {
            unit: Kelvin,
            symbol: "°F",
            offset: Offset::ratio(45967, 180),
            step: Magnitude::ratio(5, 9)
        }
    }
}

#[cfg(feature = "thermal")]
pub use thermal::*;

#[cfg(feature = "mechanical")]
mod pressure {
    use super::*;
    use crate::point::AmbientPressure;
    use crate::units::Pascal;

    crate::origin! {
        /// Absolute pressure, from vacuum.
        pub struct Vacuum { unit: Pascal, symbol: "Pa", offset: Offset::ZERO }
    }

    crate::origin! {
        /// Gauge pressure, from the ambient pressure (a tyre, a compressor): converts to [Vacuum] only with the
        /// ambient pressure at run time ([Point::against](crate::Point::against)), never at compile time.
        pub struct Gauge {
            unit: Pascal,
            symbol: "Pa (gauge)",
            offset: Offset::ZERO,
            step: Magnitude::ONE,
            reference: AmbientPressure
        }
    }

    crate::origin! {
        /// Pressure above one standard atmosphere, 101325 Pa (ISO 2533): gauge pressure with its reference stated,
        /// so it converts to [Vacuum] at compile time.
        pub struct AboveStandardAtmosphere { unit: Pascal, symbol: "Pa (above 1 atm)", offset: Offset::int(101325) }
    }
}

#[cfg(feature = "mechanical")]
pub use pressure::*;

#[cfg(feature = "angle")]
mod angle {
    use super::*;
    use crate::point::{Modulo, UnitZero};
    use crate::prefix::{One, Rational};
    use crate::units::{Degree, Radian, Turn};
    use core::marker::PhantomData;

    /// An angle from the unit's zero that wraps every turn: `Point<Angle<Turn>, Bam16>` is a binary angle whose
    /// differences are signed and the shortest way round. The modulus is one turn in `U`: 1 rev, 360 °, 2π rad (only
    /// a float storage holds a period of 2π rad exactly in steps). A heading from north or an encoder's index is an
    /// origin of its own ([origin!](crate::origin) with `modulus: Modulo<..>`).
    #[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
    pub struct Angle<U>(PhantomData<U>);

    crate::scale! {
        /// 2π: a turn in radians.
        pub struct TwoPi = "", Exact(2 / 1, pi 1);
    }

    macro_rules! angle {
        ($($unit:ty, $turn:ty, $symbol:literal;)*) => {$(
            impl Origin for Angle<$unit> {
                type Unit = $unit;
                type Reference = UnitZero;
                type Modulus = Modulo<$turn>;
                const OFFSET: Offset = Offset::ZERO;
                const NAME: &'static str = "Angle";
                const SYMBOL: &'static str = $symbol;
                const STEP: Magnitude = Magnitude::ONE;
                const SHOWN: Offset = Offset::ZERO;
            }
        )*};
    }

    angle! {
        Turn, One, "rev";
        Degree, Rational<360, 1>, "°";
        Radian, TwoPi, "rad";
    }
}

#[cfg(feature = "angle")]
pub use angle::*;
