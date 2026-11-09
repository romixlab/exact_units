//! The origins of this crate: temperature scales (K, °C, °F, °R), absolute and gauge pressure, angles that wrap at a
//! turn, the time scales (TAI with the GPS and PTP epochs, POSIX time with the Unix and NTP epochs, a device's
//! monotonic clock). Each is a zero-size marker declared with [origin!](crate::origin); a user crate declares its own the same
//! way (a register formula, a vehicle bus' offset).

#[cfg(feature = "time")]
mod time {
    use crate::point::{Reference, ReferenceHow, TimeCounting, TimeScaleDef, TimeZero};
    use crate::units::Second;
    use crate::{Micros, Millis, Nanos, Offset, Point, Secs};

    /// International Atomic Time, the time scale that counts every SI second: its zero is 1958-01-01 00:00:00 TAI.
    /// GPS time (TAI − 19 s) and PTP's are origins on it ([GpsEpoch], [PtpEpoch]), so points convert between them
    /// exactly at compile time. To and from [PosixScale] only through a leap-second table at run time
    /// ([Point::across], feature `leap_seconds`).
    #[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
    pub struct TaiScale;

    impl Reference for TaiScale {
        const ID: crate::MarkerId<'static> = crate::MarkerId {
            crate_name: "exact_units",
            name: "TaiScale",
        };

        const NAME: &'static str = "TAI";

        const HOW: ReferenceHow = ReferenceHow::TimeScale(TimeScaleDef {
            counting: TimeCounting::EverySecond,
            zero: TimeZero::Agreed,
        });
    }

    /// UTC as POSIX time counts it, every day 86400 s, a leap second not counted: its zero is 1970-01-01 00:00:00
    /// UTC ([UnixEpoch]). `time_t`, `CLOCK_REALTIME`, NTP ([NtpEpoch]) and most log formats count it. A difference
    /// of two of its points is the elapsed time only up to the leap seconds between them; to and from [TaiScale]
    /// only through a leap-second table at run time ([Point::across], feature `leap_seconds`).
    #[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
    pub struct PosixScale;

    impl Reference for PosixScale {
        const ID: crate::MarkerId<'static> = crate::MarkerId {
            crate_name: "exact_units",
            name: "PosixScale",
        };

        const NAME: &'static str = "POSIX time";

        const HOW: ReferenceHow = ReferenceHow::TimeScale(TimeScaleDef {
            counting: TimeCounting::PosixDays,
            zero: TimeZero::Agreed,
        });
    }

    /// A device's monotonic clock (a timer since boot): a time scale of its own, whose zero is known only at run time
    /// and differs per device and per boot, and whose rate is its crystal's. Its points reach a host's scale only
    /// through [Point::against] with where the device's zero lies there, what a clock sync measures. A host that
    /// talks to several devices declares a reference per device the same way, so their points can't mix.
    #[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
    pub struct DeviceMonotonic;

    impl Reference for DeviceMonotonic {
        const ID: crate::MarkerId<'static> = crate::MarkerId {
            crate_name: "exact_units",
            name: "DeviceMonotonic",
        };

        const NAME: &'static str = "device monotonic clock";

        const HOW: ReferenceHow = ReferenceHow::TimeScale(TimeScaleDef {
            counting: TimeCounting::EverySecond,
            zero: TimeZero::RunTime,
        });
    }

    crate::origin! {
        /// The Unix epoch, 1970-01-01 00:00:00 UTC, on [PosixScale]: a point from it is a moment in POSIX time, as
        /// `CLOCK_REALTIME`, `time_t`, journald's `__REALTIME_TIMESTAMP` and most log formats count it. Every day is
        /// 86400 s on this scale: a leap second repeats one (or the clock smears it), so the difference of two such
        /// points is the elapsed time only up to the leap seconds between them (27 from 1972 to 2017). To TAI or GPS
        /// time only through a leap-second table ([Point::across]), never at compile time.
        pub struct UnixEpoch { unit: Second, symbol: "s UTC", offset: Offset::ZERO, reference: PosixScale }
    }

    crate::origin! {
        /// NTP's epoch, 1900-01-01 00:00:00 UTC, on [PosixScale]: 2 208 988 800 s before the [UnixEpoch], exactly
        /// (NTP counts every day as 86400 s too, RFC 5905).
        pub struct NtpEpoch { unit: Second, symbol: "s UTC", offset: Offset::int(-2_208_988_800), reference: PosixScale }
    }

    crate::origin! {
        /// TAI's zero, 1958-01-01 00:00:00 TAI, on [TaiScale].
        pub struct TaiEpoch1958 { unit: Second, symbol: "s TAI", offset: Offset::ZERO, reference: TaiScale }
    }

    crate::origin! {
        /// The GPS epoch, 1980-01-06 00:00:00 GPS time, which is 1980-01-06 00:00:19 TAI: GPS time counts every SI
        /// second and stays 19 s behind TAI, so it is an origin on [TaiScale], 694 656 019 s after
        /// [TaiEpoch1958] (8040 days and 19 s).
        pub struct GpsEpoch { unit: Second, symbol: "s GPS", offset: Offset::int(694_656_019), reference: TaiScale }
    }

    crate::origin! {
        /// PTP's epoch (IEEE 1588), 1970-01-01 00:00:00 TAI, on [TaiScale]: 378 691 200 s (4383 days) after
        /// [TaiEpoch1958]. A PTP timestamp counts TAI seconds from it, 37 s ahead of POSIX time since 2017.
        pub struct PtpEpoch { unit: Second, symbol: "s PTP", offset: Offset::int(378_691_200), reference: TaiScale }
    }

    crate::origin! {
        /// A device's boot, the zero of its monotonic clock, on [DeviceMonotonic].
        pub struct DeviceBoot { unit: Second, symbol: "s since boot", offset: Offset::ZERO, reference: DeviceMonotonic }
    }

    /// A moment in POSIX time in whole seconds since the [UnixEpoch]: `time_t`, the bytes of a `T`.
    pub type UnixSecs<T = u64> = Point<UnixEpoch, Secs<T>>;

    /// A moment in POSIX time in ms since the [UnixEpoch], the bytes of a `T`.
    pub type UnixMillis<T = u64> = Point<UnixEpoch, Millis<T>>;

    /// A moment in POSIX time in µs since the [UnixEpoch] (journald's `__REALTIME_TIMESTAMP`), the bytes of a `T`.
    pub type UnixMicros<T = u64> = Point<UnixEpoch, Micros<T>>;

    /// A moment in POSIX time in ns since the [UnixEpoch], the bytes of a `T`; a `u64` holds them until 2554.
    pub type UnixNanos<T = u64> = Point<UnixEpoch, Nanos<T>>;

    /// A moment in POSIX time in whole seconds since the [NtpEpoch], the bytes of a `T` (NTP's era seconds in a `u32`).
    pub type NtpSecs<T = u64> = Point<NtpEpoch, Secs<T>>;

    /// A moment in TAI in whole seconds since [TaiEpoch1958], the bytes of a `T`.
    pub type TaiSecs<T = u64> = Point<TaiEpoch1958, Secs<T>>;

    /// A moment in TAI in ms since [TaiEpoch1958], the bytes of a `T`.
    pub type TaiMillis<T = u64> = Point<TaiEpoch1958, Millis<T>>;

    /// A moment in TAI in µs since [TaiEpoch1958], the bytes of a `T`.
    pub type TaiMicros<T = u64> = Point<TaiEpoch1958, Micros<T>>;

    /// A moment in TAI in ns since [TaiEpoch1958], the bytes of a `T`; a `u64` holds them until 2542.
    pub type TaiNanos<T = u64> = Point<TaiEpoch1958, Nanos<T>>;

    /// A moment in GPS time in whole seconds since the [GpsEpoch], the bytes of a `T`.
    pub type GpsSecs<T = u64> = Point<GpsEpoch, Secs<T>>;

    /// A moment in GPS time in ms since the [GpsEpoch], the bytes of a `T`: a receiver's week times 604 800 000 plus
    /// its time of week.
    pub type GpsMillis<T = u64> = Point<GpsEpoch, Millis<T>>;

    /// A moment in GPS time in µs since the [GpsEpoch], the bytes of a `T`.
    pub type GpsMicros<T = u64> = Point<GpsEpoch, Micros<T>>;

    /// A moment in GPS time in ns since the [GpsEpoch], the bytes of a `T`.
    pub type GpsNanos<T = u64> = Point<GpsEpoch, Nanos<T>>;

    /// A PTP timestamp in ns since the [PtpEpoch], the bytes of a `T`.
    pub type PtpNanos<T = u64> = Point<PtpEpoch, Nanos<T>>;

    /// A moment on a device's monotonic clock in ms since its [DeviceBoot], the bytes of a `T`.
    pub type DeviceMillis<T = u64> = Point<DeviceBoot, Millis<T>>;

    /// A moment on a device's monotonic clock in µs since its [DeviceBoot], the bytes of a `T`.
    pub type DeviceMicros<T = u64> = Point<DeviceBoot, Micros<T>>;

    /// A moment on a device's monotonic clock in ns since its [DeviceBoot], the bytes of a `T`.
    pub type DeviceNanos<T = u64> = Point<DeviceBoot, Nanos<T>>;
}

#[cfg(feature = "time")]
pub use time::*;

#[cfg(feature = "thermal")]
mod thermal {
    use crate::units::Kelvin;
    use crate::{Magnitude, Offset};

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
    use crate::point::AmbientPressure;
    use crate::units::Pascal;
    use crate::{Magnitude, Offset};

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
    use crate::point::{Modulo, UnitZero};
    use crate::prefix::{One, Rational};
    use crate::units::{Degree, Radian, Turn};
    use crate::{Magnitude, Offset, Origin};
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

                const ID: crate::MarkerId<'static> = crate::MarkerId {
                    crate_name: "exact_units",
                    name: "Angle",
                };

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
