---
level: advanced
---

# Time scales

A timestamp is a point: a moment against an epoch. Which epoch matters less than which **time scale** the epoch sits on, because scales count seconds differently:

- **TAI** counts every SI second. GPS time is TAI − 19 [[s]], and PTP counts TAI from 1970.
- **POSIX time** (`time_t`, `CLOCK_REALTIME`, NTP) counts UTC with every day 86 400 [[s]]. A leap second isn't
  counted, so POSIX time and TAI drift apart by one second at each leap second: 10 [[s]] in 1972, 37 [[s]] since 2017.
- **A device's monotonic clock** counts from its boot. Each device and each boot has its own zero, known only at run time.

Each scale is a `Reference`, and its epochs are origins on it. Points convert exactly within one scale. Between scales they convert only by an explicit step at run time.

| Reference | Origins | Aliases |
|---|---|---|
| `TaiScale` | `TaiEpoch1958` (1958-01-01 TAI), `GpsEpoch` (1980-01-06 00:00:19 TAI), `PtpEpoch` (1970-01-01 TAI) | `TaiSecs` .. `TaiNanos`, `GpsSecs` .. `GpsNanos`, `PtpNanos` |
| `PosixScale` | `UnixEpoch` (1970-01-01 UTC), `NtpEpoch` (1900-01-01 UTC) | `UnixSecs` .. `UnixNanos`, `NtpSecs` |
| `DeviceMonotonic` | `DeviceBoot` | `DeviceMillis`, `DeviceMicros`, `DeviceNanos` |

All of them are in `exact_units::origins` (feature `time`). The `T` of each alias defaults to `u64`.

## Within a scale: exact at compile time

GPS time to TAI or PTP time, or Unix to NTP time, is an offset of whole seconds. `to` works it out at compile time:

```rust
use exact_units::origins::{GpsSecs, NtpSecs, PtpEpoch, PtpNanos, UnixEpoch, UnixSecs, NtpEpoch};
use exact_units::prefix::{Nano, One};

// the GPS epoch is 315 964 819 s after PTP's: 3657 days and the 19 s GPS stays behind TAI
assert_eq!(
    GpsSecs::from_raw(0).to::<PtpEpoch, Nano, u64>(),
    Some(PtpNanos::from_raw(315_964_819_000_000_000))
);
// NTP's epoch is 2 208 988 800 s before Unix's
assert_eq!(UnixSecs::from_raw(0).to::<NtpEpoch, One, u64>(), Some(NtpSecs::from_raw(2_208_988_800)));
```

Points of two scales don't convert this way. POSIX time to GPS time needs the leap seconds, so it doesn't compile:

```rust,compile_fail
use exact_units::origins::{GpsEpoch, UnixSecs};
use exact_units::prefix::One;
let _ = UnixSecs::<u64>::from_raw(1_700_000_000).to::<GpsEpoch, One, u64>();
```

## Between POSIX time and TAI: the leap-second table

The feature `leap_seconds` adds `exact_units::leap`. It has a `LeapTable` trait, one table, `BULLETIN_C_72`, and `Point::across`, which converts a moment between `PosixScale` and `TaiScale` through a table at run time. The table is named after its source, IERS Bulletin C 72 (2026-07-06), and it expires on 2027-06-28, the earliest date a new leap second could come. `across` gives the exact moment, or it refuses with a reason:

- `Stale`: the moment is at or past the table's expiry.
- `InLeap`: the moment of TAI falls inside a leap second. POSIX time has no number for 23:59:60.
- `BeforeTable`: the moment is before 1972-01-01, when UTC was not yet whole seconds from TAI.
- `NotExact`: the result isn't a whole number of the target's steps, or it doesn't fit the target's storage.

A GPS receiver gives its week number modulo 1024 and the time of week. Together they make a `GpsMillis`. That converts at compile time to TAI, and then through the table to POSIX time:

```rust
use exact_units::leap::{AcrossError, BULLETIN_C_72};
use exact_units::origins::{GpsMillis, TaiEpoch1958, TaiNanos, UnixEpoch, UnixNanos};
use exact_units::prefix::{Milli, Nano, One};
use exact_units::units::{Second, Week};
use exact_units::{Fixed, Millis};

// week 370 of the third era (it began 2019-04-07), halfway through the week
let (week, tow) = (370_u16 & 0x3ff, Millis::<u32>::new(302_400_000));
let weeks = Fixed::<Week, One, u64>::new(2 * 1024 + u64::from(week));
let gps = GpsMillis::<u64>::from_raw(0)
    + weeks.convert::<Second, Milli, u64>().unwrap()
    + tow.to().unwrap();

let tai: TaiNanos = gps.to::<TaiEpoch1958, Nano, u64>().unwrap();
let unix: UnixNanos = tai.across::<UnixEpoch, Nano, u64, _>(&BULLETIN_C_72).unwrap();
// GPS time is 18 s ahead of UTC in 2026
let gps_s = 2418 * 604_800 + 302_400;
assert_eq!(unix.raw(), (315_964_800 + gps_s - 18) * 1_000_000_000);

// the last leap second, 2016-12-31 23:59:60 UTC, has no POSIX time
let new_year = TaiNanos::from_raw((1_483_228_800 + 37 + 378_691_200) * 1_000_000_000_u64);
let leap = TaiNanos::from_raw(new_year.raw() - 500_000_000);
assert_eq!(leap.across::<UnixEpoch, Nano, u64, _>(&BULLETIN_C_72), Err(AcrossError::InLeap));
```

The table sits behind its feature on purpose. A firmware that doesn't enable `leap_seconds` has neither the table nor `across`: they are not compiled, so nothing of them reaches its flash. A host that keeps a newer table (from a later Bulletin C, or IANA's `leap-seconds.list`) implements `LeapTable` for it and passes that instead.

## A device's clock: against a sync point

A device stamps its samples on its own monotonic clock. Where its boot lies on a host's scale is a run-time value: a clock sync measures it. `against` takes that value and gives the host's point:

```rust
use exact_units::origins::{DeviceMicros, TaiEpoch1958};
use exact_units::{Micros, Point};

let sample = DeviceMicros::from_raw(2_500_000);
// the sync says the device booted at this moment of TAI
let boot = Point::<TaiEpoch1958, Micros<u64>>::from_raw(2_000_000_000_000_000);
assert_eq!(sample.against(boot), Some(Point::from_raw(2_000_000_002_500_000)));
```

A host that talks to several devices declares a reference for each one, as `DeviceMonotonic` is declared. Then their points can't mix by mistake.

## A reference as data

`Reference::DEF` is a `ReferenceDef`: the reference's id, its name, and how its zero is known (`ReferenceHow`):

- `Exact`: the unit's own zero.
- `RunTime`: a zero known only at run time, like the ambient pressure.
- `TimeScale`: a time scale, with how it counts seconds (`TimeCounting::EverySecond` or `PosixDays`) and whether its zero is agreed or a device's own (`TimeZero`).

`OriginDef::reference` carries it, so a client that knows an origin only from the API model can tell whether two points convert, and how.
