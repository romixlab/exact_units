---
level: advanced
---

# Run-time scales

A `Fixed` has its scale in the type. The compiler works out every conversion factor, and a conversion costs a multiply and a divide by constants. Some scales are only known when the program runs:

- A video stream states its own clock tick: $\mathrm{num\_units\_in\_tick} / \mathrm{time\_scale}$ in H.264 E.2.1 and H.265 E.3.1, $\mathrm{num\_units\_in\_display\_tick} / \mathrm{time\_scale}$ in AV1 6.4.3, and a timescale per track in MP4 (15360 is common).
- A JPEG file carries a quantization table: 64 steps, each a whole number from 1 to 65 535 (T.81 B.2.4.1).
- H.264's HRD states a bit rate as $(\mathrm{bit\_rate\_value\_minus1} + 1) \cdot 2^{6 + \mathrm{bit\_rate\_scale}}\,\unit{bit/s}$ (E.2.2, eq. E-81).

The unit is still known when the firmware is built: ticks are seconds, a rate is bits per second. `Scaled<U, T>` is such a number. `U` is the unit, in the type. `T` is a primitive integer. The scale is a `RunScale`, a value stored beside the number. `Ticks<T>` is `Scaled<Second, T>`.

```rust
use exact_units::{Fixed, Rounding, RunScale, Ticks, prefix::{Milli, Rational}};

// an MP4 track's timescale, read from the file
let base = RunScale::ratio(1, 15_360).unwrap();
let pts = Ticks::<u64>::new(154_112, base);
// to a 90 kHz clock: 154112/15360 s is 903 000 ticks, exact
assert_eq!(pts.to_fixed::<Rational<1, 90_000>, u64>(), Some(Fixed::new(903_000)));
// to milliseconds it isn't whole: refused, or rounded by name
assert_eq!(pts.to_fixed::<Milli, u32>(), None);
assert_eq!(pts.to_fixed_rounded::<Milli, u32>(Rounding::Floor), Some(Fixed::new(10_033)));
assert_eq!(pts.to_string(), "154112·1/15360 s");
```

A unit that is itself only known at run time is not this crate's: a value whose unit is data belongs to a dynamic units library.

## The scale

A `RunScale` is $\mathrm{num} / \mathrm{den} \cdot 2^{\mathrm{pow2}}$. `num` and `den` are odd `u32`s with no common factor, and `pow2` is within $\pm 127$. Each value has exactly one such form, so `==` and `<` compare values: $2002/120000$ is the same scale as $1001/60000$.

```rust
use exact_units::RunScale;

let tc = RunScale::ratio(1001, 60_000).unwrap();
assert_eq!(RunScale::ratio(2002, 120_000), Some(tc));
assert_eq!(tc.to_ratio(), Some((1001, 60_000)));
assert_eq!(tc.parts(), (1001, 1875, -5));
// a frame is two ticks in H.264 (Table E-6)
let frame = tc.checked_mul(RunScale::int(2).unwrap()).unwrap();
assert_eq!(frame.to_string(), "1001/30000");
assert!(frame > tc);
// a part of 0 is no scale
assert_eq!(RunScale::ratio(1, 0), None);
```

The constructors take the forms streams use:

- `ratio(num, den)` takes a pair of `u32` fields, which every time base in H.264, H.265, AV1 and MP4 is.
- `int(n)` takes a whole step, such as a JPEG quantization step.
- `pow2(e)` takes a power of two, such as an HRD rate.
- `new(num, den, e)` takes a mantissa at a power of two, such as T.800's step size $2^{R_b - \varepsilon_b} \cdot (1 + \mu_b/2^{11})$ (eq. E-3), down to $2^{-42}$ and beyond.
- `from_u64(num, den)` takes wider parts and refuses a fraction whose odd parts pass `u32` after it is reduced.
- `of::<P>()` and `of_magnitude(&m)` turn a compile-time scale into a run-time one. They give `None` for $\pi/180$ or $10^{-15}$, which have no such form.

A value without a form is refused when the scale is built. It is never rounded.

```rust
use exact_units::RunScale;

// T.800's step with Rb = 8, εb = 31, μb = 1: 2049 · 2⁻³⁴
let step = RunScale::new(2049, 1, 8 - 31 - 11).unwrap();
assert_eq!(step.to_ratio(), Some((2049, 1 << 34)));
// 5¹⁵ is past u32: 10⁻¹⁵ has no form
assert_eq!(RunScale::from_u64(1, 1_000_000_000_000_000), None);
assert_eq!(RunScale::new(3, 1, -100).unwrap().to_string(), "3·2⁻¹⁰⁰");
```

## Converting

Every conversion is exact or refused, unless it names a rounding:

| from | to | exact or `None` | by a named rounding |
|---|---|---|---|
| `Scaled` | `Fixed<U, P, T2>` | `to_fixed` | `to_fixed_rounded(r)`, `requantize_fixed(r, o)` |
| `Fixed` | `Scaled` | `from_fixed(f, scale)` | `from_fixed_rounded(f, scale, r)` |
| `Scaled` | another `RunScale` | `rescale(to)` | `rescale_rounded(to, r)`, `requantize(to, r, o)` |

`requantize` and `requantize_fixed` also settle a result that doesn't fit by a named `Overflow`, as `Fixed::from_prefix` does. `requantize_fixed` into a bounded `Fixed` clamps to its range with `Saturate` and refuses a value outside it otherwise.

```rust
use exact_units::{Rounding, RunScale, Ticks};

let field = RunScale::ratio(1001, 60_000).unwrap();
let rtp = RunScale::ratio(1, 90_000).unwrap();
// two fields are a frame, 3003 ticks of 90 kHz; one is 1501.5
assert_eq!(Ticks::<u32>::new(2, field).rescale::<u32>(rtp).map(|t| t.get()), Some(3003));
assert_eq!(Ticks::<u32>::new(1, field).rescale::<u32>(rtp), None);
let half = Ticks::<u32>::new(1, field).rescale_rounded::<u32>(rtp, Rounding::NearestEven);
assert_eq!(half.map(|t| t.get()), Some(1502));
```

The arithmetic is the crate's integer kernel, the same one `Fixed::from_prefix` uses. A count of 32 bits at a scale whose parts fit is one `u64` multiply and divide. Anything wider runs on 32-bit limbs with 64-bit products. There is no float and no 128-bit type, on a host or on a Cortex-M0+.

## Comparing and adding

Two `Scaled` values compare by their exact value, whatever their scales. Three ticks of $1/25\,\unit{s}$ equal six of $1/50\,\unit{s}$. `same_repr` asks whether they are the same number at the same scale.

```rust
use exact_units::{RunScale, Ticks};

let a = Ticks::<u32>::new(3, RunScale::ratio(1, 25).unwrap());
let b = Ticks::<u32>::new(6, RunScale::ratio(1, 50).unwrap());
assert_eq!(a, b);
assert!(!a.same_repr(&b));
assert!(Ticks::<u32>::new(3003, RunScale::ratio(1, 90_000).unwrap()) < Ticks::new(1, RunScale::ratio(1, 25).unwrap()));
```

`Scaled` has no `+` operator. `checked_add` and `checked_sub` add two values at the same scale. They refuse two different scales: a sum is never rescaled behind the caller's back. To add across scales, rescale one by name first. `RunScale::common(a, b)` gives the largest step both are whole multiples of, where every value of either converts exactly.

```rust
use exact_units::{RunScale, Ticks};

let rtp = RunScale::ratio(1, 90_000).unwrap();
let tc = RunScale::ratio(1001, 60_000).unwrap();
let (a, b) = (Ticks::<u64>::new(3003, rtp), Ticks::<u64>::new(1, tc));
assert_eq!(a.checked_add(b), None);
let c = RunScale::common(rtp, tc).unwrap();
assert_eq!(c, RunScale::ratio(1, 180_000).unwrap());
let sum = a.rescale::<u64>(c).unwrap().checked_add(b.rescale(c).unwrap()).unwrap();
assert_eq!(sum.get(), 6006 + 3003);
```

## Points on a run-time clock

`Point<O, Scaled<U, T>>` is a position on a clock whose tick is known at run time, such as a stream's presentation time. The origin must be `Linear`. Its number is read with `coord()` and `steps()`, because a `Fixed` point already has `get()` and `raw()`.

RTP's timestamp is 32 bits at a clock rate the session states (RFC 3550 5.1): 90 [[kHz]] for video, 48 [[kHz]] for most audio. It wraps every $2^{32}$ ticks, a period that is a number of ticks, not a compile-time number of seconds. So the timestamp is a `Linear` point known modulo $2^{32}$, with named methods that wrap:

- `wrapping_add` and `wrapping_sub` move the counter around its circle.
- `wrapping_diff` is the short way round, signed, with a half turn negative.
- `unwrap_near(reference)` gives the full `u64` count, nearest the last full point a receiver knows, whose low bits are the counter's.

```rust
use exact_units::{Point, RunScale, Ticks, origins::DeviceBoot};

let rate = RunScale::ratio(1, 48_000).unwrap();
let t = Point::<DeviceBoot, Ticks<u32>>::at(u32::MAX - 99, rate);
let next = t.wrapping_add(Ticks::new(960, rate)).unwrap();
assert_eq!(next.steps(), 860);
assert_eq!(next.wrapping_diff(t).map(|d| d.get()), Some(960));
// the receiver's full count across the wrap
let last = Point::<DeviceBoot, Ticks<u64>>::at(u64::from(u32::MAX) - 99, rate);
assert_eq!(next.unwrap_near(last).map(Point::steps), Some(u64::from(u32::MAX) + 861));
```

A `Modulo` origin has a period that is a compile-time number of its unit. At a run-time step that period is a whole number of steps only for some steps, so such an origin has no `Scaled` points.

## What each user holds

| What | Scale | Value |
|---|---|---|
| H.264 or H.265 VUI tick | `RunScale::ratio(num_units_in_tick, time_scale)` | `Ticks<u64>` |
| An MP4 track's time | `RunScale::ratio(1, timescale)` | `Point<O, Ticks<u64>>` |
| RTP at the session's rate | `RunScale::ratio(1, rate)` | `Point<O, Ticks<u32>>`, unwrapped near a `u64` |
| JPEG coefficient | `RunScale::int(q[k])`, one per element | `Scaled<U, i16>` |
| H.264 HRD bit rate | `RunScale::pow2(6 + bit_rate_scale)` | `Scaled<Per<Bit, Second>, u32>` |
| T.800 step size | `RunScale::new(2048 + μb, 1, Rb − εb − 11)` | `Scaled<U, i32>` |

Where the set of time bases is known when the firmware is built, a `Fixed` per base still does the job at no cost. The [video time recipe](../recipes/video_time.md) shows both.

## Cost on a microcontroller

`tests/scaled_size.rs` builds three images at a firmware's release profile. Each converts one count to 90 [[kHz]] ticks by a named rounding:

| Image | Cortex-M4F | Cortex-M0+ |
|---|---|---|
| `Fixed` at $1001/60000\,\unit{s}$, a compile-time scale | 88 B | 176 B |
| `Ticks<u32>`, the base read at run time | 4320 B | 5028 B |
| `Ticks<u64>` | 4248 B | 5056 B |

The run-time images hold the kernel's limbs, the core's `u64` division and `memcpy`. The test fails above 6 [[KiB]], and where an image links a soft-float or 128-bit routine.

## On the wire and in serde

On the wire a `RunScale` is `num` and `den` as `u32` and `pow2` as `i8`, nine bytes. A form that isn't the canonical one doesn't decode, so each scale has one encoding. A `Scaled` is its value, then its scale.

In serde a `RunScale` is `{ num, den, pow2 }`, written as a fraction of `u64`s where one holds it, so a config reads as the stream states it: `{ "num": 1001, "den": 60000, "pow2": 0 }`. Reading takes any fraction of nonzero parts and reduces it exactly, or refuses it. A `Scaled` is `{ value, scale }`.

## Limits

- `T` is an integer of up to 64 bits, the storages a `Fixed` has. There is no float `Scaled`. For a float, convert to a `Fixed` and use its named float edge.
- A `RunScale` is positive. A sign belongs to the value.
- There are no operators, only the `checked_` forms, because a sum at two scales has to be refused without a panic.
