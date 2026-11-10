//! 16. Video time: stream time bases, RTP's wrapping 90 kHz clock, audio against video, drop-frame time codes.
//!
//! H.264 and H.265 give a stream's clock tick as tc = num_units_in_tick ÷ time_scale (H.264 C-1 with its semantics in
//! E.2.1, H.265 C-1 with E.3.1; AV1 6.4.3 the same with num_units_in_display_tick). Their own examples are exact
//! rationals: 30000/1001 Hz from time_scale 60 000 and num_units_in_tick 1001, and 25 Hz from a 27 MHz clock with
//! 1 080 000. Each is a scale marker here, `Rational<1001, 60000>`, and a tick count at that base is a `Fixed`. A
//! frame lasts 2·tc in H.264 (DeltaTfiDivisor 2 for a frame, Table E-6) but one elemental interval in H.265
//! (DeltaToDivisor 1, Table E.7): the two standards' own examples count a tick differently, so a frame period is
//! named per standard, never assumed.
//!
//! RTP for H.264 carries a 32-bit timestamp, and "A 90 kHz clock rate MUST be used" (RFC 6184 5.1). That is a point
//! on an origin of its own that wraps every 2³² ticks, 13 h 15 min 21.6 s: `+` wraps and `-` is the shortest way
//! round, signed. MPEG-2 systems count the same 90 kHz clock in 33 bits; that standard is not in the library, so only
//! the arithmetic is shown: a period of 2³³ ticks in a `u64`.
//!
//! Between two clocks a conversion is exact or names its rounding. A frame at 30000/1001 Hz is exactly 3003 ticks of
//! 90 kHz, a field (1001/60000 s) is 1501.5 and has no exact value. 48 kHz audio at 30000/1001 frames/s is 8008/5 =
//! 1601.6 samples per frame: the samples of each frame come from rounding the running total, and the rounding picks
//! the cadence (1602, 1601, 1602, 1601, 1602 with `NearestEven`, 1602, 1602, 1601, 1602, 1601 with `Ceil`, 1601, 1602,
//! 1601, 1602, 1602 with `Floor`). Each gives 8008 samples every 5 frames.
//!
//! A time code is a label, not a time. H.264 D.2.3 (eq. D-1) and H.265 D.3.27 (eq. D-26) turn one into a
//! clockTimestamp in ticks of time_scale, with tOffset carrying the difference to the true time. Drop-frame counting
//! (counting_type 4: H.264 Table D-3, H.265 Table D.11 and its NOTE 7, "NTSC drop-frame") skips the labels ;00 and
//! ;01 at the start of every minute not a multiple of ten. The drift of a label against true time is exact: 3/5000 s
//! short every ten minutes, 86.4 ms a day. The crate has no label codec; the example writes one with plain integers
//! and checks it whole.
//!
//! `cargo run --example 16_video_time --features time`

#![expect(
    clippy::print_stdout,
    clippy::unwrap_used,
    reason = "an example prints what it shows and unwraps values it knows are there"
)]
#![expect(
    clippy::arithmetic_side_effects,
    reason = "time code fields are below 100 and frame counts below 2^32: every sum and product fits u64"
)]

use exact_units::prefix::{FixedScale, Rational};
use exact_units::units::Second;
use exact_units::{Fixed, Modulo, Offset, Point, Reference, ReferenceHow, Rounding};
use exact_units::{TimeCounting, TimeScaleDef, TimeZero, origin};

/// H.264's and H.265's example tick for 30000/1001 Hz: num_units_in_tick 1001, time_scale 60 000 (H.264 E.2.1).
type Tick60k = Rational<1001, 60_000>;

/// An H.264 frame at that tick: 2·tc, DeltaTfiDivisor 2 (H.264 Table E-6), 1001/30000 s.
type Frame30k = Rational<1001, 30_000>;

/// A tick of the time_scale clock itself, 1/60000 s: the unit of clockTimestamp (H.264 D-1).
type Unit60k = Rational<1, 60_000>;

/// H.265's example tick for 25 Hz: num_units_in_tick 1 080 000 of a 27 MHz clock (H.265 E.3.1).
type Tick27M = Rational<1_080_000, 27_000_000>;

/// One tick of the 90 kHz clock of RFC 6184 5.1.
type Tick90k = Rational<1, 90_000>;

/// One sample of 48 kHz audio.
type Sample48k = Rational<1, 48_000>;

/// The RTP sender's media clock: its zero is the random initial timestamp of each stream, known only at run time,
/// so points of two streams don't mix.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
struct RtpMediaClock;

impl Reference for RtpMediaClock {
    const ID: exact_units::MarkerId<'static> = exact_units::MarkerId {
        crate_name: "video_time",
        name: "RtpMediaClock",
    };

    const NAME: &'static str = "RTP media clock";

    const HOW: ReferenceHow = ReferenceHow::TimeScale(TimeScaleDef {
        counting: TimeCounting::EverySecond,
        zero: TimeZero::RunTime,
    });
}

origin! {
    /// RFC 6184 5.1's timestamp: 32 bits of a 90 kHz clock, wrapping every 2³² ticks.
    struct Rtp90k { unit: Second, symbol: "s (RTP)", offset: Offset::ZERO, reference: RtpMediaClock,
        modulus: Modulo<Rational<4_294_967_296, 90_000>> }
}

origin! {
    /// The same clock counted in 33 bits, as MPEG-2 systems' PTS is: a period of 2³³ ticks in a `u64`.
    struct Pts33 { unit: Second, symbol: "s (PTS)", offset: Offset::ZERO, reference: RtpMediaClock,
        modulus: Modulo<Rational<8_589_934_592, 90_000>> }
}

origin! {
    /// The receiver's unwrapped count of the same clock, from the stream's first timestamp: it doesn't wrap.
    struct Rtp90kUnwrapped { unit: Second, symbol: "s (RTP, unwrapped)", offset: Offset::ZERO, reference: RtpMediaClock }
}

type RtpTime = Point<Rtp90k, Fixed<Second, Tick90k, u32>>;
type PtsTime = Point<Pts33, Fixed<Second, Tick90k, u64>>;
type RtpUnwrapped = Point<Rtp90kUnwrapped, Fixed<Second, Tick90k, u64>>;

/// A stream's time base, matched against the bases this firmware knows at build time. Any other pair is refused: a
/// base known only at run time has no type in the crate yet (the design call of PT-8).
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum KnownBase {
    /// 1001/60000 s: 30000/1001 frames/s in H.264.
    Ntsc,
    /// 1/25 s: 25 frames/s in H.265's example.
    Pal,
    /// 1/90000 s.
    Mpeg,
}

/// The base of a VUI's num_units_in_tick ÷ time_scale, any spelling of it (2002/120000 is 1001/60000).
fn known_base(num_units_in_tick: u32, time_scale: u32) -> Option<KnownBase> {
    let (n, d) = (u64::from(num_units_in_tick), u64::from(time_scale));
    // n/d = a/b exactly when n·b = a·d: no division, no rounding.
    let is = |a: u64, b: u64| d != 0 && n * b == a * d;
    if is(1001, 60_000) {
        Some(KnownBase::Ntsc)
    } else if is(1, 25) {
        Some(KnownBase::Pal)
    } else if is(1, 90_000) {
        Some(KnownBase::Mpeg)
    } else {
        None
    }
}

/// A time code label hh:mm:ss;ff at 30 labels a second.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
struct TimeCode {
    hours: u64,
    minutes: u64,
    seconds: u64,
    frames: u64,
}

impl core::fmt::Display for TimeCode {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let (h, m, s, ff) = (self.hours, self.minutes, self.seconds, self.frames);
        write!(f, "{h:02}:{m:02}:{s:02};{ff:02}")
    }
}

/// Labels in ten minutes of drop-frame counting: 18 000 less 2 at each of the 9 minutes not a multiple of ten.
const FRAMES_PER_10_MIN: u64 = 10 * 60 * 30 - 9 * 2;

/// Labels in a minute that drops two: 1800 − 2.
const FRAMES_PER_DROP_MIN: u64 = 60 * 30 - 2;

/// The drop-frame label of frame `n` (counting_type 4, H.264 Table D-3): the labels ;00 and ;01 are skipped at
/// ss = 0 of each minute that isn't a multiple of 10.
fn drop_frame_label(n: u64) -> TimeCode {
    let (tens, rest) = (n / FRAMES_PER_10_MIN, n % FRAMES_PER_10_MIN);
    // The first minute of ten keeps its 1800 labels; each later one starts 2 labels on.
    let dropped = 18 * tens
        + if rest < 1800 {
            0
        } else {
            2 * ((rest - 1800) / FRAMES_PER_DROP_MIN + 1)
        };
    let label = n + dropped;
    TimeCode {
        hours: label / (30 * 3600),
        minutes: label / (30 * 60) % 60,
        seconds: label / 30 % 60,
        frames: label % 30,
    }
}

/// The frame that carries a drop-frame label, `None` for a label drop-frame counting skips or a field out of range.
fn drop_frame_frame(tc: TimeCode) -> Option<u64> {
    let TimeCode {
        hours,
        minutes,
        seconds,
        frames,
    } = tc;
    if minutes >= 60 || seconds >= 60 || frames >= 30 {
        return None;
    }
    if seconds == 0 && frames < 2 && !minutes.is_multiple_of(10) {
        return None;
    }
    let all_minutes = hours * 60 + minutes;
    let label = (all_minutes * 60 + seconds) * 30 + frames;
    Some(label - 2 * (all_minutes - all_minutes / 10))
}

/// H.264 D-1's clockTimestamp of a label at time_scale 60 000, num_units_in_tick 1001, nuit_field_based_flag 1 and
/// tOffset 0: ((hH·60 + mM)·60 + sS)·time_scale + nFrames·(num_units_in_tick·2), in ticks of 1/60000 s.
fn clock_timestamp(tc: TimeCode) -> Fixed<Second, Unit60k, u64> {
    let seconds = (tc.hours * 60 + tc.minutes) * 60 + tc.seconds;
    Fixed::new(seconds * 60_000 + tc.frames * 1001 * 2)
}

/// Every claim about drop-frame labels, checked over a whole day against a counter that steps label by label.
fn walk_a_day() {
    let mut label = TimeCode {
        hours: 0,
        minutes: 0,
        seconds: 0,
        frames: 0,
    };
    let day = 24 * 6 * FRAMES_PER_10_MIN;
    for n in 0..day {
        assert_eq!(drop_frame_label(n), label, "frame {n}");
        assert_eq!(drop_frame_frame(label), Some(n), "label {label}");
        // the next label, as a time code generator steps it
        label.frames += 1;
        if label.frames == 30 {
            label.frames = 0;
            label.seconds += 1;
            if label.seconds == 60 {
                label.seconds = 0;
                label.minutes += 1;
                if label.minutes == 60 {
                    label.minutes = 0;
                    label.hours += 1;
                }
            }
            if label.seconds == 0 && !label.minutes.is_multiple_of(10) {
                label.frames = 2;
            }
        }
    }
    assert_eq!(
        label,
        TimeCode {
            hours: 24,
            minutes: 0,
            seconds: 0,
            frames: 0
        }
    );
}

fn main() {
    // Time bases: a scale is a Magnitude, so 1 080 000 / 27 000 000 is the same scale as 1/25.
    assert_eq!(Tick27M::SCALE, <Rational<1, 25>>::SCALE);
    assert_eq!(
        Frame30k::SCALE,
        Tick60k::SCALE.mul(exact_units::Magnitude::pow2(1))
    );
    let tick = Fixed::<Second, Tick60k, u32>::new(1);
    println!("tc = {tick}");
    assert_eq!(known_base(1001, 60_000), Some(KnownBase::Ntsc));
    assert_eq!(known_base(2002, 120_000), Some(KnownBase::Ntsc));
    assert_eq!(known_base(1_080_000, 27_000_000), Some(KnownBase::Pal));
    assert_eq!(known_base(1, 90_000), Some(KnownBase::Mpeg));
    assert_eq!(
        known_base(1, 15_360),
        None,
        "an MP4 timescale this firmware wasn't built for"
    );
    assert_eq!(known_base(1, 0), None);

    // Between clocks: a frame is exactly 3003 ticks of 90 kHz, an H.264 tick (a field) is 1501.5 and has to round.
    let frame = Fixed::<Second, Frame30k, u64>::new(1);
    assert_eq!(frame.to::<Tick90k, u64>().unwrap().get(), 3003);
    let field = Fixed::<Second, Tick60k, u64>::new(1);
    assert_eq!(field.to::<Tick90k, u64>(), None);
    assert_eq!(
        field
            .to_rounded::<Tick90k, u64>(Rounding::NearestEven)
            .unwrap()
            .get(),
        1502
    );
    assert_eq!(
        field
            .to_rounded::<Tick90k, u64>(Rounding::Floor)
            .unwrap()
            .get(),
        1501
    );
    // and back: 90 kHz ticks to frames, exact only on a frame boundary
    let rtp_delta = Fixed::<Second, Tick90k, u64>::new(3003 * 25 + 1);
    assert_eq!(rtp_delta.to::<Frame30k, u64>(), None);
    assert_eq!(
        rtp_delta
            .to_rounded::<Frame30k, u64>(Rounding::Floor)
            .unwrap()
            .get(),
        25
    );

    // RTP's 32-bit timestamp wraps every 2³² ticks; a difference across the wrap is the short way round.
    let before = RtpTime::from_raw(u32::MAX - 1000);
    let after = before + Fixed::new(3003);
    assert_eq!(after.raw(), 2002);
    assert_eq!((after - before).get(), 3003_i32);
    assert_eq!((before - after).get(), -3003_i32);
    println!(
        "RTP {} + 1 frame = {} (raw {})",
        before.raw(),
        after.raw(),
        after.raw()
    );
    // A receiver unwraps it into a u64 count by adding each signed step: the typed unwrapper is ww_si's.
    let mut full = RtpUnwrapped::from_raw(u64::from(before.raw()));
    let step = after - before;
    full = match u64::try_from(step.get()) {
        Ok(ahead) => full + Fixed::new(ahead),
        Err(_) => full - Fixed::new(u64::from(step.get().unsigned_abs())),
    };
    assert_eq!(full.raw(), u64::from(u32::MAX) + 2003);

    // A 33-bit PTS in a u64 wraps at 2³³ by rem_euclid; `-` is again the short way round.
    let pts = PtsTime::from_raw((1 << 33) - 1);
    let next = pts + Fixed::new(3003);
    assert_eq!(next.raw(), 3002);
    assert_eq!((next - pts).get(), 3003_i64);
    // A raw value past the period isn't reduced by from_raw: it compares unequal to its own residue (a gap of PT-8).
    assert_ne!(PtsTime::from_raw(1 << 33), PtsTime::from_raw(0));
    assert_eq!((PtsTime::from_raw(1 << 33) - PtsTime::from_raw(0)).get(), 0);

    // Audio at 48 kHz against 30000/1001 frames/s: 8008/5 samples a frame. The running total is rounded; the
    // rounding picks the cadence, every one gives 8008 samples in 5 frames.
    assert_eq!(frame.to::<Sample48k, u64>(), None);
    assert_eq!(
        Fixed::<Second, Frame30k, u64>::new(5)
            .to::<Sample48k, u64>()
            .unwrap()
            .get(),
        8008
    );
    for (r, want) in [
        (Rounding::NearestEven, [1602, 1601, 1602, 1601, 1602]),
        (Rounding::Ceil, [1602, 1602, 1601, 1602, 1601]),
        (Rounding::Floor, [1601, 1602, 1601, 1602, 1602]),
    ] {
        let total = |k: u64| {
            Fixed::<Second, Frame30k, u64>::new(k)
                .to_rounded::<Sample48k, u64>(r)
                .unwrap()
                .get()
        };
        let cadence = [1, 2, 3, 4, 5].map(|k| total(k) - total(k - 1));
        assert_eq!(cadence, want, "{r:?}");
        println!("{r:?}: {cadence:?}");
    }

    // Drop-frame labels against true time.
    walk_a_day();
    for n in [1799, 1800, 17_981, 17_982, 107_892] {
        let tc = drop_frame_label(n);
        // the true time of frame n, in ticks of 1/60000 s: exact, 2002 a frame
        let truth = Fixed::<Second, Frame30k, u64>::new(n)
            .to::<Unit60k, u64>()
            .unwrap();
        // D-1's tOffset is what makes clockTimestamp the true time
        let t_offset =
            truth.to::<Unit60k, i64>().unwrap() - clock_timestamp(tc).to::<Unit60k, i64>().unwrap();
        println!(
            "frame {n:>6}: {tc}, clockTimestamp {}, tOffset {t_offset}",
            clock_timestamp(tc)
        );
    }
    // Every ten minutes of labels are 17 982 frames, 3/5000 s short of ten minutes of true time.
    let ten_minutes = Fixed::<Second, Frame30k, u64>::new(FRAMES_PER_10_MIN)
        .to::<Unit60k, i64>()
        .unwrap();
    let label = clock_timestamp(drop_frame_label(FRAMES_PER_10_MIN))
        .to::<Unit60k, i64>()
        .unwrap();
    assert_eq!(label.get(), 600 * 60_000);
    assert_eq!(
        (ten_minutes - label).get(),
        -36,
        "36 ticks of 1/60000 s, 3/5000 s"
    );
    let day = Fixed::<Second, Frame30k, u64>::new(24 * 6 * FRAMES_PER_10_MIN)
        .to::<Unit60k, i64>()
        .unwrap();
    let day_label = Fixed::<Second, exact_units::prefix::One, i64>::new(86_400)
        .to::<Unit60k, i64>()
        .unwrap();
    let drift = (day - day_label)
        .to::<exact_units::prefix::Micro, i64>()
        .unwrap();
    assert_eq!(drift.get(), -86_400);
    println!("a day of drop-frame labels runs {drift} against true time");
}
