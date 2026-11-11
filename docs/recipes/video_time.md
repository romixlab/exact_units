---
level: basic
---

# Recipes: video time

A video plays at "29.97" frames a second. How long is one frame, in each of the clocks a stream carries, and why does its time code drift away from the clock on the wall? The frame lasts exactly $1001/30\,000\,\unit{s}$. That is 3003 ticks of RTP's 90 [[kHz]] clock and 1601.6 samples of 48 [[kHz]] audio, and a drop-frame time code still runs 86.4 [[ms]] a day ahead of true time. This page works up to those answers one step at a time.

Each step adds one idea: a frame period as an exact fraction, a conversion that is exact or names its rounding, a time base that a stream states at run time, a counter that wraps, and a label that is not a time. The example [`16_video_time`](examples.md) does all of it in one program. The sources are ITU-T H.264 (V16, 06/2026), ITU-T H.265 (V11, 01/2026), the AV1 bitstream specification (1.0.0 with errata) and IETF RFC 6184; [Going deeper](#going-deeper) has the summary and the details.

## A frame is an exact fraction of a second

"29.97" is a rounding. The rate is $30\,000/1001\,\unit{Hz}$, so a frame lasts $1001/30\,000\,\unit{s}$. As a scale marker that period is exact, and a count of frames is a `Fixed` at it. Converting a frame to ticks of 90 [[kHz]] gives a whole number:

```rust
use exact_units::{Fixed, prefix::Rational, units::Second};

let frame = Fixed::<Second, Rational<1001, 30_000>, u64>::new(1);
assert_eq!(frame.to::<Rational<1, 90_000>, u64>().map(Fixed::get), Some(3003));
```

A frame is 3003 ticks, exactly. Written as 29.97 frames/s the frame would be 3003.003 ticks, and the error would grow with every frame. Kept as a fraction, frame $n$ always starts at tick $3003\,n$.

## A tick that doesn't fit is refused, or rounded by name

Not every period fits another clock. An H.264 stream at this rate has a clock tick of $1001/60\,000\,\unit{s}$, half a frame, and that is 1501.5 ticks of 90 [[kHz]]. There is no exact answer, so `to` gives `None`. A rounding is a choice the caller writes:

```rust
use exact_units::{Fixed, Rounding, prefix::Rational, units::Second};

let tick = Fixed::<Second, Rational<1001, 60_000>, u64>::new(1);
assert_eq!(tick.to::<Rational<1, 90_000>, u64>(), None);
assert_eq!(tick.to_rounded::<Rational<1, 90_000>, u64>(Rounding::Floor).map(Fixed::get), Some(1501));
```

This is the rule for every conversion on this page: exact, or a rounding with a name. Two programs that name the same rounding agree on every tick.

## The stream names its tick

Where did $1001/60\,000\,\unit{s}$ come from? A stream says it. H.264 gives a clock tick as

$$
t_{\mathrm{c}} = \frac{\mathrm{num\_units\_in\_tick}}{\mathrm{time\_scale}}
$$

(equation C-1, semantics in E.2.1). H.265 has the same (C-1, E.3.1), and AV1 has $\mathrm{num\_units\_in\_display\_tick} / \mathrm{time\_scale}$ (6.4.3). The standards' own examples are exact rationals. $30\,000/1001\,\unit{Hz}$ comes from time_scale 60 000 and num_units_in_tick 1001 (H.264 E.2.1). 25 [[Hz]] comes from a 27 [[MHz]] clock and num_units_in_tick 1 080 000 (H.265 E.3.1). Each is a scale marker, and markers of the same factor are the same scale however they are spelled:

```rust
use exact_units::prefix::{FixedScale, Rational};

assert_eq!(<Rational<1_080_000, 27_000_000>>::SCALE, <Rational<1, 25>>::SCALE);
```

A frame is not the same number of ticks in both standards, though. In H.264 an ordinary frame lasts two ticks: DeltaTfiDivisor is 2 for a frame (Table E-6), which is why $30\,000/1001$ frames/s needs a tick of $1001/60\,000\,\unit{s}$. In H.265 a frame lasts one elemental interval: DeltaToDivisor is 1 for a frame (Table E.7), and the 25 [[Hz]] example has a tick of 0.04 [[s]], one per picture. So the frame period is named per standard (`Rational<1001, 30000>` for H.264's example), never derived from "the tick".

## A base known only at run time

A firmware can't list every base in advance: the stream states its own. A `RunScale` holds a base exactly at run time, and `Ticks<T>` is a count at it ([Run-time scales](../maths/run_time_scales.md)). It converts like a `Fixed` does, exact or by a named rounding:

```rust
use exact_units::{Fixed, Rounding, RunScale, Ticks, prefix::Rational};

let ntsc = RunScale::ratio(1001, 60_000).unwrap();
assert_eq!(RunScale::ratio(2002, 120_000), Some(ntsc));
// 512 ticks of 1/15360 s are 3000 ticks of 90 kHz
let mp4 = RunScale::ratio(1, 15_360).unwrap();
assert_eq!(Ticks::<u64>::new(512, mp4).to_fixed::<Rational<1, 90_000>, u64>(), Some(Fixed::new(3000)));
// a field of the NTSC stream has no exact value at 90 kHz
let field = Ticks::<u64>::new(1, ntsc);
assert_eq!(field.to_fixed::<Rational<1, 90_000>, u64>(), None);
let rounded = field.to_fixed_rounded::<Rational<1, 90_000>, u64>(Rounding::NearestEven);
assert_eq!(rounded, Some(Fixed::new(1502)));
// a zero is no base
assert_eq!(RunScale::ratio(1, 0), None);
```

$2002/120\,000$ is the same base as $1001/60\,000$, so two encoders that spell it differently agree. An MP4 timescale of 15 360, which no firmware lists in advance, converts like any other. A field rounds to 1502 under `NearestEven`, as 1501.5 is halfway and 1502 is even.

## Audio alongside: 1601.6 samples a frame

48 [[kHz]] audio runs beside the video. A frame lasts $48\,000 \cdot 1001/30\,000 = 8008/5 = 1601.6$ samples, so no frame holds a whole number of them. The way out is to round the running total, not each frame. The samples of frame $k$ are the total at frame $k + 1$ less the total at frame $k$, each total rounded:

```rust
use exact_units::{Fixed, Rounding, prefix::Rational, units::Second};

let total = |k: u64| {
    let t = Fixed::<Second, Rational<1001, 30_000>, u64>::new(k);
    t.to_rounded::<Rational<1, 48_000>, u64>(Rounding::NearestEven).unwrap().get()
};
assert_eq!([1, 2, 3, 4, 5].map(|k| total(k) - total(k - 1)), [1602, 1601, 1602, 1601, 1602]);
assert_eq!(total(5), 8008);
```

Every 5 frames hold exactly 8008 samples, and the audio never drifts from the video. The rounding decides the order of the 1601s and 1602s; [Going deeper](#going-deeper) has the cadence under each. The cadence is derived here, not cited: SMPTE ST 272, which fixes one for embedded audio, is not in the library. A sender and a receiver that agree on the rounding agree on every frame.

## RTP's 90 kHz clock wraps

RTP for H.264 stamps each packet with a 32-bit timestamp, and "A 90 kHz clock rate MUST be used" (RFC 6184 5.1). The counter wraps every $2^{32}$ ticks, 13 [[h]] 15 min 21.6 [[s]]. So a timestamp is a point on an origin of its own whose modulus is $2^{32}/90\,000\,\unit{s}$. On it `+` wraps, and `−` gives the shortest way round, signed:

```rust
use exact_units::{Fixed, Modulo, Offset, Point, origin, prefix::Rational, units::Second};

origin! {
    /// RFC 6184 5.1's timestamp: 32 bits of a 90 kHz clock.
    pub struct Rtp90k { unit: Second, symbol: "s (RTP)", offset: Offset::ZERO,
        modulus: Modulo<Rational<4_294_967_296, 90_000>> }
}
type RtpTime = Point<Rtp90k, Fixed<Second, Rational<1, 90_000>, u32>>;

let before = RtpTime::from_raw(u32::MAX - 1000);
let after = before + Fixed::new(3003); // one frame later
assert_eq!(after.raw(), 2002);
assert_eq!((after - before).get(), 3003);
assert_eq!((before - after).get(), -3003);
```

One frame after a timestamp near the top, the counter reads 2002. The difference is still one frame, 3003 ticks, and backwards it is −3003. A receiver never sees a jump of 13 hours.

A period of exactly $2^{32}$ steps wraps in the `u32`'s own arithmetic, with no division. RTP also starts each stream at a random timestamp, so the example declares the origin on a reference of its own, the sender's media clock, and points of two streams don't mix.

## A time code is a label, not a time

A time code such as `00:10:00;00` is what an editor reads. It looks like a time, but it is a label: hours, minutes, seconds and a frame number. H.264 turns one into a time with equation D-1 (D.2.3):

$$
\begin{aligned}
\mathrm{clockTimestamp} = {} & ((\mathrm{hH} \cdot 60 + \mathrm{mM}) \cdot 60 + \mathrm{sS}) \cdot \mathrm{time\_scale} \\
& + \mathrm{nFrames} \cdot (\mathrm{num\_units\_in\_tick} \cdot (1 + \mathrm{nuit\_field\_based\_flag})) \\
& + \mathrm{tOffset}
\end{aligned}
$$

H.265 has the same in D.3.27 (equation D-26). The result is in ticks of time_scale. tOffset is what makes it the true time.

At $30\,000/1001$ frames/s a label counts 30 frames a second, and true time runs $1000/1001$ as fast as the label. Drop-frame counting reduces the drift: counting_type 4 skips the labels `;00` and `;01` "when seconds_value is equal to 0 and minutes_value is not an integer multiple of 10" (H.264 Table D-3; H.265 Table D.11). H.265's NOTE 7 to Table D.11 calls it "NTSC drop-frame" and says it reduces the need for large tOffset; it does not remove it.

The drift that remains is exact. Ten minutes of labels are $18\,000 - 9 \cdot 2 = 17\,982$ frames, and `Exact` says how long they last:

```rust
use exact_units::{Exact, exact, units::Second};

let frames = 18_000 - 9 * 2;
let true_time = (Exact::<Second>::new(1001, 30_000) * frames).unwrap();
assert_eq!(true_time, exact!(599.9994 s));
// ten minutes of labels run 0.6 ms ahead; a day of them, 144 such blocks, 86.4 ms
let ahead = (exact!(600 s) - true_time).unwrap();
assert_eq!(ahead, exact!(0.6 ms));
assert_eq!((ahead * 144).unwrap(), exact!(86.4 ms));
```

Every ten minutes of labels run $3/5000\,\unit{s}$ ahead of true time, 36 ticks of $1/60\,000\,\unit{s}$. In those ticks the true time of frame $n$ is $2002\,n$, so tOffset is a whole number of ticks for every label: −36 at `00:10:00;00`, −216 at `01:00:00;00`, and +3540 just before the first dropped label at `00:00:59;29`.

The crate has no codec for such labels yet. The example writes one with integers (frame to label and back, refusing a label drop-frame counting skips) and checks it against a counter stepped label by label over a whole day.

## Going deeper

### What is exact where

MPEG-2 systems (H.222.0) is not in the library, so its 33-bit PTS is shown only as arithmetic.

| Step | Exact | Named rounding | Not in the crate yet |
|---|---|---|---|
| A stream's tick, $\mathrm{num\_units\_in\_tick} / \mathrm{time\_scale}$ | yes, as a scale marker or a `RunScale` | | |
| A frame at $30\,000/1001\,\unit{Hz}$ in 90 [[kHz]] ticks | 3003 | | |
| An H.264 tick (a field) in 90 [[kHz]] ticks | | 1501.5 | |
| 48 [[kHz]] samples per frame | every 5 frames, 8008 | each frame | |
| RTP's 32-bit timestamp, wrapping | `+` and `−` | | |
| A time code against true time | the drift | | a label codec |

### The audio cadence under each rounding

| Rounding | Samples of frames 1 to 5 |
|---|---|
| `NearestEven` | 1602, 1601, 1602, 1601, 1602 |
| `Ceil` | 1602, 1602, 1601, 1602, 1601 |
| `Floor` | 1601, 1602, 1601, 1602, 1602 |

### Fixed bases or a run-time base

Where every base is known when the firmware is built, a `Fixed` per base costs nothing at run time. Match the stream's pair against them without dividing: $n/d$ is $a/b$ exactly when $n b = a d$, and any other pair is refused. A run-time base costs about 4 KiB of flash on a Cortex-M4F for any number of bases.

### 33 bits, and counting past a wrap

A 33-bit count in a `u64`, as MPEG-2 systems keep its PTS, wraps the same way as RTP's with `modulus: Modulo<Rational<8_589_934_592, 90_000>>`, by `rem_euclid`.

A receiver that wants a count that doesn't wrap adds each signed step to a `u64` point. WireWeaver's `ww_si` has the typed unwrapper for device time; the example does the step by hand.

### What the crate doesn't do yet

- **`from_raw` doesn't reduce a modulo point.** `PtsTime::from_raw(1 << 33)` is not equal to `PtsTime::from_raw(0)`, although their difference is 0.
- **No label codec.** A time code (`hh:mm:ss;ff` with drop-frame counting) and its exact drift against a time point are written by hand.
- **No unwrapper for modulo points in this crate.** The typed one is `ww_si`'s, for device time. A counter at a run-time rate (RTP at the rate the session states) has a stateless step, `unwrap_near`, on a `Linear` point known modulo $2^{32}$ ([Run-time scales](../maths/run_time_scales.md#points-on-a-run-time-clock)).
