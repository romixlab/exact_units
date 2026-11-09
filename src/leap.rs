//! Leap seconds (feature `leap_seconds`): the table that relates POSIX time ([PosixScale]) to TAI ([TaiScale]),
//! and [Point::across] between them.
//!
//! Within a time scale a point converts exactly at compile time ([Point::to]): GPS time to TAI, Unix to NTP time.
//! Between POSIX time and TAI the difference steps at each leap second, which the IERS announces about six months
//! ahead in its Bulletin C; no compile-time conversion can know it. [Point::across] looks the difference up in a
//! [LeapTable] at run time and refuses what the table doesn't cover:
//!
//! - [Stale](AcrossError::Stale) from the table's expiry on, where a leap second may have been added since;
//! - [InLeap](AcrossError::InLeap) for a moment of TAI inside a leap second (23:59:60 UTC has no POSIX time);
//! - [BeforeTable](AcrossError::BeforeTable) before 1972, when UTC was not yet whole seconds from TAI.
//!
//! The crate ships one table, [BULLETIN_C_72], named after its source. A build without the feature has neither the
//! table nor `across`: a firmware that doesn't ask for leap seconds carries none.
//!
//! ```
//! use exact_units::leap::BULLETIN_C_72;
//! use exact_units::origins::{GpsMillis, TaiEpoch1958, TaiNanos, UnixEpoch, UnixNanos};
//! use exact_units::prefix::{Milli, Nano, One};
//! use exact_units::units::{Second, Week};
//! use exact_units::{Fixed, Millis};
//!
//! // a receiver's week number modulo 1024 and its time of week; the third era began 2019-04-07
//! let (week, tow) = (370_u16 & 0x3ff, Millis::<u32>::new(302_400_000));
//! let weeks = Fixed::<Week, One, u64>::new(2 * 1024 + u64::from(week));
//! let gps = GpsMillis::<u64>::from_raw(0)
//!     + weeks.convert::<Second, Milli, u64>().unwrap()
//!     + tow.to().unwrap();
//! // GPS time to TAI: one scale, exact at compile time
//! let tai: TaiNanos = gps.to::<TaiEpoch1958, Nano, u64>().unwrap();
//! // TAI to POSIX time through the table: GPS is 18 s ahead of UTC in 2026
//! let unix: UnixNanos = tai.across::<UnixEpoch, Nano, u64, _>(&BULLETIN_C_72).unwrap();
//! assert_eq!(unix.raw(), (315_964_800 + 2418 * 604_800 + 302_400 - 18) * 1_000_000_000);
//! ```
//!
//! Without the table, POSIX time doesn't convert to GPS time:
//!
//! ```compile_fail
//! use exact_units::origins::{GpsEpoch, UnixSecs};
//! use exact_units::prefix::One;
//! let _ = UnixSecs::<u64>::from_raw(1_700_000_000).to::<GpsEpoch, One, u64>();
//! ```

use crate::origins::{PosixScale, TaiScale, UnixSecs};
use crate::units::Second;
use crate::{Bounds, Fixed, FixedScale, Linear, Origin, Point, Reference, Repr, Secs, Storage};
use core::fmt;

/// From one moment of POSIX time on, TAI − UTC: an entry of a [LeapTable].
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct LeapEntry {
    /// The first moment it holds, in POSIX time: 00:00:00 UTC of the day after a leap second.
    pub from: UnixSecs<i64>,
    /// TAI − UTC from then on.
    pub tai_minus_utc: Secs<i16>,
}

impl LeapEntry {
    /// From POSIX second `from` on, TAI − UTC is `tai_minus_utc` s.
    #[must_use]
    pub const fn new(from: i64, tai_minus_utc: i16) -> Self {
        LeapEntry {
            from: Point::from_raw(from),
            tai_minus_utc: Fixed::new(tai_minus_utc),
        }
    }
}

/// A table of leap seconds: TAI − UTC over time, and the moment it stops being known. [Leaps] is a table fixed at
/// compile time; a table read at run time (a newer Bulletin C a host fetched) implements it too.
pub trait LeapTable {
    /// The entries, oldest first: the first is 1972-01-01 (TAI − UTC = 10 s), each next one a leap second later.
    fn entries(&self) -> &[LeapEntry];

    /// The first moment the table doesn't cover: from then on a leap second may have been added since.
    fn expires(&self) -> UnixSecs<i64>;
}

/// A [LeapTable] fixed at compile time, with the source it was taken from.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Leaps {
    source: &'static str,
    entries: &'static [LeapEntry],
    expires: UnixSecs<i64>,
}

impl Leaps {
    /// A table from `source` (a Bulletin C, by number), its `entries` oldest first, valid until `expires`. In a
    /// `const`, entries out of order, a table without one or an expiry before its last entry don't compile.
    ///
    /// # Panics
    /// On such a table, which in a `const` is a compile error.
    #[must_use]
    #[expect(
        clippy::indexing_slicing,
        reason = "const fn: every index is below len"
    )]
    #[expect(
        clippy::arithmetic_side_effects,
        reason = "i + 1 < len, which fits usize"
    )]
    pub const fn new(source: &'static str, entries: &'static [LeapEntry], expires: i64) -> Self {
        assert!(
            !entries.is_empty(),
            "a leap table has at least its first entry, 1972-01-01"
        );
        let mut i = 0;
        while i + 1 < entries.len() {
            assert!(
                entries[i].from.raw() < entries[i + 1].from.raw(),
                "leap table entries go oldest first"
            );
            i += 1;
        }
        assert!(
            expires > entries[entries.len() - 1].from.raw(),
            "a leap table expires after its last entry"
        );
        Leaps {
            source,
            entries,
            expires: Point::from_raw(expires),
        }
    }

    /// Where the table comes from: `"IERS Bulletin C 72 (2026-07-06)"`.
    #[must_use]
    pub const fn source(&self) -> &'static str {
        self.source
    }
}

impl LeapTable for Leaps {
    fn entries(&self) -> &[LeapEntry] {
        self.entries
    }

    fn expires(&self) -> UnixSecs<i64> {
        self.expires
    }
}

/// The leap seconds as IERS Bulletin C 72 (Paris, 2026-07-06) has them: TAI − UTC = 37 s since 2017-01-01, no leap
/// second at the end of December 2026. It expires on 2027-06-28 00:00:00 UTC, as IANA's `leap-seconds.list` of the
/// same bulletin does: a leap second could come at the end of June 2027.
pub const BULLETIN_C_72: Leaps = Leaps::new(
    "IERS Bulletin C 72 (2026-07-06)",
    &[
        LeapEntry::new(63072000, 10),   // 1972-01-01
        LeapEntry::new(78796800, 11),   // 1972-07-01
        LeapEntry::new(94694400, 12),   // 1973-01-01
        LeapEntry::new(126230400, 13),  // 1974-01-01
        LeapEntry::new(157766400, 14),  // 1975-01-01
        LeapEntry::new(189302400, 15),  // 1976-01-01
        LeapEntry::new(220924800, 16),  // 1977-01-01
        LeapEntry::new(252460800, 17),  // 1978-01-01
        LeapEntry::new(283996800, 18),  // 1979-01-01
        LeapEntry::new(315532800, 19),  // 1980-01-01
        LeapEntry::new(362793600, 20),  // 1981-07-01
        LeapEntry::new(394329600, 21),  // 1982-07-01
        LeapEntry::new(425865600, 22),  // 1983-07-01
        LeapEntry::new(489024000, 23),  // 1985-07-01
        LeapEntry::new(567993600, 24),  // 1988-01-01
        LeapEntry::new(631152000, 25),  // 1990-01-01
        LeapEntry::new(662688000, 26),  // 1991-01-01
        LeapEntry::new(709948800, 27),  // 1992-07-01
        LeapEntry::new(741484800, 28),  // 1993-07-01
        LeapEntry::new(773020800, 29),  // 1994-07-01
        LeapEntry::new(820454400, 30),  // 1996-01-01
        LeapEntry::new(867715200, 31),  // 1997-07-01
        LeapEntry::new(915148800, 32),  // 1999-01-01
        LeapEntry::new(1136073600, 33), // 2006-01-01
        LeapEntry::new(1230768000, 34), // 2009-01-01
        LeapEntry::new(1341100800, 35), // 2012-07-01
        LeapEntry::new(1435708800, 36), // 2015-07-01
        LeapEntry::new(1483228800, 37), // 2017-01-01
    ],
    1_814_140_800, // 2027-06-28
);

/// Why [Point::across] refused.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum AcrossError {
    /// The moment is at or past the table's expiry: a leap second may have come since.
    Stale,
    /// The moment of TAI is inside a leap second, 23:59:60 UTC, which POSIX time can't count (or a POSIX second a
    /// negative leap second left out).
    InLeap,
    /// The moment is before the table's first entry, 1972-01-01, when UTC wasn't yet whole seconds from TAI.
    BeforeTable,
    /// The result isn't a whole number of the target's steps, or doesn't fit its storage.
    NotExact,
}

impl fmt::Display for AcrossError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            AcrossError::Stale => "past the leap table's expiry",
            AcrossError::InLeap => "inside a leap second",
            AcrossError::BeforeTable => "before the leap table's first entry",
            AcrossError::NotExact => {
                "not a whole number of the target's steps, or out of its range"
            }
        })
    }
}

impl core::error::Error for AcrossError {}

mod sealed {
    pub trait Sealed {}

    impl Sealed for crate::origins::PosixScale {}

    impl Sealed for crate::origins::TaiScale {}
}

/// A time scale whose points go [across](Point::across) to `R2`'s through a [LeapTable]: [PosixScale] to [TaiScale]
/// and back.
pub trait Across<R2: Reference>: Reference + sealed::Sealed {
    /// UTC (POSIX time) to TAI, or back.
    #[doc(hidden)]
    const UTC_TO_TAI: bool;
}

impl Across<TaiScale> for PosixScale {
    const UTC_TO_TAI: bool = true;
}

impl Across<PosixScale> for TaiScale {
    const UTC_TO_TAI: bool = false;
}

/// Seconds from TAI's zero, 1958-01-01 TAI, to 1970-01-01 on the same calendar: 4383 days. A moment of POSIX time `p`
/// after 1972 is `p + (TAI − UTC) + TAI_1970` s of TAI.
const TAI_1970: i128 = 378_691_200;

/// The steps and offsets of a conversion across, as integers: the source in units of 1/`D` s from its scale's
/// zero is `x · n1b1 + a1d1`; the target's number from `M` (units of 1/`D` s from its scale's zero) is
/// `(M · b2 − a2D) · d2 / den`.
#[derive(Copy, Clone)]
struct Map {
    n1b1: i128,
    a1d1: i128,
    d: i128,
    b2: i128,
    a2d: i128,
    d2: i128,
    den: i128,
}

impl Map {
    #[expect(
        clippy::panic,
        reason = "a compile error in const code, as documented: steps or offsets that aren't fractions of integers"
    )]
    #[expect(
        clippy::as_conversions,
        clippy::cast_possible_wrap,
        reason = "const fn: From is not const on stable; each part is checked to be at most i64::MAX first"
    )]
    const fn of<O: Origin, P: FixedScale, O2: Origin, P2: FixedScale>(int: bool) -> Self {
        assert!(
            int,
            "Point::across takes and gives integer storages only: a float is never exact"
        );
        let (Some((n1, d1)), Some((a1, b1)), Some((n2, d2)), Some((a2, b2))) = (
            P::SCALE.to_ratio(),
            O::OFFSET.to_ratio(),
            P2::SCALE.to_ratio(),
            O2::OFFSET.to_ratio(),
        ) else {
            panic!("Point::across: steps and offsets are fractions of integers");
        };
        let max = i64::MAX as u128;
        assert!(
            n1 <= max && d1 <= max && b1 <= max && n2 <= max && d2 <= max && b2 <= max,
            "Point::across: steps and offsets of at most 63 bits"
        );
        let (n1, d1, b1, n2, d2, b2) = (
            n1 as i128, d1 as i128, b1 as i128, n2 as i128, d2 as i128, b2 as i128,
        );
        let (Some(n1b1), Some(a1d1), Some(d)) =
            (n1.checked_mul(b1), a1.checked_mul(d1), d1.checked_mul(b1))
        else {
            panic!("Point::across: the source's steps and offset don't fit 128 bits");
        };
        let (Some(a2d), Some(den)) = (
            a2.checked_mul(d),
            match d.checked_mul(b2) {
                Some(x) => x.checked_mul(n2),
                None => None,
            },
        ) else {
            panic!("Point::across: the target's steps and offset don't fit 128 bits");
        };
        Map {
            n1b1,
            a1d1,
            d,
            b2,
            a2d,
            d2,
            den,
        }
    }
}

/// TAI − UTC for the moment `p` of POSIX time, in units of 1/`d` s.
pub(crate) fn utc_to_tai<L: LeapTable + ?Sized>(
    p: i128,
    d: i128,
    table: &L,
) -> Result<i128, AcrossError> {
    let at = |s: i64| i128::from(s).checked_mul(d).ok_or(AcrossError::NotExact);
    if p >= at(table.expires().raw())? {
        return Err(AcrossError::Stale);
    }
    let entries = table.entries();
    let mut found = None;
    for (i, e) in entries.iter().enumerate() {
        if p >= at(e.from.raw())? {
            found = Some(i);
        } else {
            break;
        }
    }
    let i = found.ok_or(AcrossError::BeforeTable)?;
    let (cur, next) = (entries.get(i), entries.get(i.wrapping_add(1)));
    let delta = cur.map_or(0, |e| i64::from(e.tai_minus_utc.get()));
    // a negative leap second leaves out the POSIX second before it
    if let Some(n) = next {
        let skip = i64::from(n.tai_minus_utc.get())
            .checked_sub(delta)
            .ok_or(AcrossError::NotExact)?;
        if skip < 0
            && p >= at(n
                .from
                .raw()
                .checked_add(skip)
                .ok_or(AcrossError::NotExact)?)?
        {
            return Err(AcrossError::InLeap);
        }
    }
    Ok(i128::from(delta))
}

/// TAI − UTC for the moment `l` of TAI, counted in units of 1/`d` s from 1970-01-01 on TAI's calendar.
pub(crate) fn tai_to_utc<L: LeapTable + ?Sized>(
    l: i128,
    d: i128,
    table: &L,
) -> Result<i128, AcrossError> {
    let entries = table.entries();
    let mut found = None;
    for (i, e) in entries.iter().enumerate() {
        let posix = l
            .checked_sub(
                i128::from(e.tai_minus_utc.get())
                    .checked_mul(d)
                    .ok_or(AcrossError::NotExact)?,
            )
            .ok_or(AcrossError::NotExact)?;
        if posix
            >= i128::from(e.from.raw())
                .checked_mul(d)
                .ok_or(AcrossError::NotExact)?
        {
            found = Some((i, e, posix));
        } else {
            break;
        }
    }
    let (i, e, posix) = found.ok_or(AcrossError::BeforeTable)?;
    let at = |s: i64| i128::from(s).checked_mul(d).ok_or(AcrossError::NotExact);
    if posix >= at(table.expires().raw())? {
        return Err(AcrossError::Stale);
    }
    // inside a leap second: past the next entry's start in the old difference, before it in the new one
    if let Some(n) = entries.get(i.wrapping_add(1))
        && posix >= at(n.from.raw())?
    {
        return Err(AcrossError::InLeap);
    }
    Ok(i128::from(e.tai_minus_utc.get()))
}

impl<O, P, T, B> Point<O, Fixed<Second, P, T, B>>
where
    O: Origin<Unit = Second, Modulus = Linear>,
    P: FixedScale,
    T: Storage,
    B: Bounds<T>,
{
    /// The same moment on the other time scale, POSIX time to TAI or back, through the leap seconds of `table`, in
    /// steps `P2` stored as `T2` from the origin `O2`. Refused when the table doesn't cover the moment
    /// ([Stale](AcrossError::Stale), [BeforeTable](AcrossError::BeforeTable)), inside a leap second
    /// ([InLeap](AcrossError::InLeap)) or when the result isn't a whole number of `P2` that fits `T2`
    /// ([NotExact](AcrossError::NotExact)). Integer storages only; within one scale, [to](Self::to).
    ///
    /// ```
    /// use exact_units::leap::{AcrossError, BULLETIN_C_72};
    /// use exact_units::origins::{GpsEpoch, TaiEpoch1958, TaiSecs, UnixEpoch, UnixSecs};
    /// use exact_units::prefix::One;
    ///
    /// // 2017-01-01 00:00:00 UTC, right after the last leap second
    /// let new_year = UnixSecs::<u64>::from_raw(1_483_228_800);
    /// let tai: TaiSecs = new_year.across::<TaiEpoch1958, One, u64, _>(&BULLETIN_C_72).unwrap();
    /// // the second before it in TAI is 2016-12-31 23:59:60 UTC
    /// let leap = TaiSecs::from_raw(tai.raw() - 1);
    /// assert_eq!(leap.across::<UnixEpoch, One, u64, _>(&BULLETIN_C_72), Err(AcrossError::InLeap));
    /// // to GPS time: 18 s ahead
    /// let gps = new_year.across::<GpsEpoch, One, u64, _>(&BULLETIN_C_72).unwrap();
    /// assert_eq!(gps.raw(), 1_483_228_800 - 315_964_800 + 18);
    /// // past the table's expiry, 2027-06-28
    /// let later = UnixSecs::<u64>::from_raw(1_814_140_800);
    /// assert_eq!(later.across::<TaiEpoch1958, One, u64, _>(&BULLETIN_C_72), Err(AcrossError::Stale));
    /// ```
    pub fn across<O2, P2, T2, L>(
        self,
        table: &L,
    ) -> Result<Point<O2, Fixed<Second, P2, T2>>, AcrossError>
    where
        O2: Origin<Unit = Second, Modulus = Linear>,
        O::Reference: Across<O2::Reference>,
        P2: FixedScale,
        T2: Storage,
        L: LeapTable + ?Sized,
    {
        let map = const { Map::of::<O, P, O2, P2>(!T::IS_FLOAT && !T2::IS_FLOAT) };
        let Repr::Int(x) = self.raw().to_repr() else {
            return Err(AcrossError::NotExact);
        };
        let e = AcrossError::NotExact;
        // the moment in units of 1/d s from the source scale's zero
        let n = x
            .checked_mul(map.n1b1)
            .and_then(|v| v.checked_add(map.a1d1))
            .ok_or(e)?;
        let tai_1970 = TAI_1970.checked_mul(map.d).ok_or(e)?;
        // the moment in units of 1/d s from the target scale's zero
        let m = if <O::Reference as Across<O2::Reference>>::UTC_TO_TAI {
            let delta = utc_to_tai(n, map.d, table)?.checked_mul(map.d).ok_or(e)?;
            n.checked_add(delta)
                .and_then(|v| v.checked_add(tai_1970))
                .ok_or(e)?
        } else {
            let l = n.checked_sub(tai_1970).ok_or(e)?;
            let delta = tai_to_utc(l, map.d, table)?.checked_mul(map.d).ok_or(e)?;
            l.checked_sub(delta).ok_or(e)?
        };
        let num = m
            .checked_mul(map.b2)
            .and_then(|v| v.checked_sub(map.a2d))
            .and_then(|v| v.checked_mul(map.d2))
            .ok_or(e)?;
        if num.checked_rem(map.den) != Some(0) {
            return Err(e);
        }
        let y = num.checked_div(map.den).and_then(T2::from_int).ok_or(e)?;
        Fixed::try_new(y).map(Point::new).map_err(|_| e)
    }
}
