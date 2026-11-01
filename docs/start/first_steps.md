---
level: basic
---

# First steps

## A value

A `Fixed` is made from its stored number:

```rust
use exact_units::{Fixed, prefix::Milli, units::Volt};

let v: Fixed<Volt, Milli, i16> = Fixed::new(1250); // 1250 mV
assert_eq!(v.get(), 1250);                         // the stored number
assert_eq!(v.to_string(), "1250 mV");
```

`get` gives the number back. Nothing else is stored.

## Aliases for the common ones

| Alias | Type |
|---|---|
| `Secs<T>`, `Millis<T>`, `Micros<T>`, `Nanos<T>` | `Fixed<Second, One / Milli / Micro / Nano, T>` |
| `Minutes<T>`, `Hours<T>`, `Days<T>` | `Fixed<Minute / Hour / Day, One, T>` |
| `Hz<T>` | `Fixed<Hertz, One, T>` |
| `Bytes<T>`, `Kibibytes<T>`, `Mebibytes<T>` | `Fixed<Byte, One / Kibi / Mebi, T>` |
| `Percent<T>`, `Permille<T>`, `Ppb<T>` | `Fixed<Ratio, Centi / Milli / Nano, T>` |
| `Duty16` | `Fixed<Ratio, Rational<1, 65535>, u16>`: 65535 is 100 % |
| `Q15<U>`, `Q31<U>` | fixed point, see [Q formats](../fixed_point/q_formats.md) |
| `Bam16`, `Bam32` | binary angles, see [Binary angles](../fixed_point/bam.md) |

```rust
use exact_units::{Fixed, Hz, Kibibytes, Millis, Percent};

let timeout: Millis<u16> = Fixed::new(250);
let rate: Hz<u32> = Fixed::new(48_000);
let buffer: Kibibytes<u16> = Fixed::new(4);
let duty: Percent<u8> = Fixed::new(50);
assert_eq!(timeout.to_string(), "250 ms");
assert_eq!(rate.to_string(), "48000 Hz");
assert_eq!(buffer.to_string(), "4 KiB");
assert_eq!(duty.to_string(), "50 %");
```

## Scales

The scale `P` says how big one step is. Every scale is an exact number, never a float.

| Scale | One step | Example |
|---|---|---|
| `One` | $1$ | `Fixed<Volt, One, i32>`: whole volts |
| `Milli`, `Micro`, `Kilo`, ... | $10^{-3}$, $10^{-6}$, $10^{3}$ | every SI prefix from `Quecto` to `Quetta` |
| `Kibi`, `Mebi`, `Gibi`, `Tebi` | $2^{10}$, $2^{20}$, ... | `Kibibytes<u16>` |
| `Pow10<E>` | $10^{E}$ | `Pow10<-4>`: tenths of a millisecond in s |
| `Pow2<E>` | $2^{E}$ | `Pow2<-15>`: the step of Q15 |
| `Rational<N, D>` | $N / D$ | `Rational<1, 65535>`: one step of a 16-bit duty |
| `FullScale<V, BITS>` | $V / 2^{BITS}$ | one step of an ADC |

Names are only spelling: `Milli`, `Pow10<-3>` and `Rational<1, 1000>` are the same scale.

```rust
use exact_units::{Fixed, prefix::{FullScale, Milli, Rational}, units::Volt};

// a 12-bit ADC over 3.3 V: one step is 3.3 V / 4096
type Adc = Fixed<Volt, FullScale<Rational<33, 10>, 12>, u16>;
assert_eq!(Adc::new(2048).to::<Milli, u16>(), Some(Fixed::new(1650)));
```

## Storages

`T` is any of `u8` .. `u64`, `i8` .. `i64`, `f32` and `f64`. The choice is yours: the type says what the bytes are.

```rust
use exact_units::{Fixed, prefix::Milli, units::Volt};

let small: Fixed<Volt, Milli, i16> = Fixed::new(-1250);
let float: Fixed<Volt, Milli, f32> = Fixed::new(-1250.0);
assert_eq!(small.to_string(), "-1250 mV");
assert_eq!(float.to_string(), "-1250 mV");
```

## Arithmetic, briefly

Values of one type add and subtract. A number of the storage multiplies and divides.

```rust
use exact_units::{Fixed, Millis};

let a: Millis<u32> = Fixed::new(250);
let b: Millis<u32> = Fixed::new(100);
assert_eq!(a + b, Fixed::new(350));
assert_eq!(a * 4, Fixed::new(1000));
assert!(b < a);
```

Values in different scales are different types. Convert one first: [Conversions](../maths/conversions.md).

## Standard library time

The time aliases convert to and from `core::time::Duration`:

```rust
use core::time::Duration;
use exact_units::{Fixed, Micros};

let t: Micros<u32> = Fixed::new(1500);
assert_eq!(Duration::from(t), Duration::from_micros(1500));
assert_eq!(Micros::<u32>::try_from(Duration::from_millis(3)), Ok(Fixed::new(3000)));
```
