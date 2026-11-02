---
level: advanced
---

# Niche codes and rails

Many protocols keep a few raw values for something that is not a number. J1939 sends 0xFF for "not available" and 0xFE for "error". SunSpec sends 0x8000 for a point the device doesn't implement. A BLE RSSI of 127 means "no reading", and Matter writes null as the top or bottom of the integer. Read as a number, such a code becomes a temperature of 215 °C.

`Niched<V, N>` keeps these codes apart from the values. It stores the raw number only, so the bytes in memory and on the wire stay the same. `get()` gives the value or the meaning of the code, never the code as a number.

## The J1939 coolant temperature

SPN 110 is one byte: 1 °C per bit from −40 °C ([Register formulas](../points/registers.md#bus-formulas)). 0 to 250 are temperatures, 251 to 253 are reserved, 0xFE is an error and 0xFF is not available.

<figure class="eu-figure" markdown="0">
--8<-- "figures/niche_codes.svg"
<figcaption>One <code>u8</code>: the bounds hold the values, the niche codes sit above them, and the two ends of the range are rails.</figcaption>
</figure>

```rust
use exact_units::{Fixed, Offset, Point, Range, origin, origins::Celsius, prefix::One, units::Kelvin};
use exact_units::niche::{J1939U8, Niched, Reading};

origin! {
    /// J1939 engine coolant temperature (SPN 110): 1 °C per bit, −40 °C offset.
    pub struct J1939Temp { above: Celsius, offset: Offset::int(-40) }
}

type Temp = Point<J1939Temp, Fixed<Kelvin, One, u8, Range<0, 250>>>;
type Coolant = Niched<Temp, J1939U8>;

let read = |raw| Coolant::try_from_raw(raw).map(Coolant::get);

match read(130) {
    Ok(Reading::Value(t)) => assert_eq!(t.to::<Celsius, One, i16>().map(|p| p.raw()), Some(90)),
    other => panic!("{other:?}"),
}
assert_eq!(read(0xFE), Ok(Reading::Niche(J1939U8::Error)));
assert_eq!(read(0xFF), Ok(Reading::Niche(J1939U8::NotAvailable)));
assert_eq!(read(252), Ok(Reading::Niche(J1939U8::Reserved(252))));

// writing a code back gives the same byte
assert_eq!(Coolant::niche(J1939U8::NotAvailable).map(Coolant::raw), Ok(0xFF));
assert_eq!(size_of::<Coolant>(), 1);
```

- `Range<0, 250>` holds the values. The codes must stay outside it.
- `try_from_raw` takes a value or a code. Anything else is `Err(OutOfBounds)`.
- `from_reading` and `niche` encode back. A code read is the code written, so a value passed through doesn't change its bytes.

## Declaring codes

`niches!` declares the meanings for one storage. A variant is one code, or a range of codes that keeps the code it was read from:

```rust
use exact_units::{Fixed, Range, niches, niche::{Niched, Reading}, prefix::One, units::Watt};

niches! {
    /// SunSpec `int16`: 0x8000 is a point the device doesn't implement.
    pub enum SunSpecI16: i16 {
        /// Not implemented by this device.
        NotImplemented = -32768,
    }
}

type Power = Niched<Fixed<Watt, One, i16, Range<-32767, 32767>>, SunSpecI16>;

assert_eq!(Power::try_from_raw(i16::MIN).map(Power::get), Ok(Reading::Niche(SunSpecI16::NotImplemented)));
assert_eq!(Power::try_from_raw(-1500).map(|p| p.value().map(|w| w.get())), Ok(Some(-1500)));
```

The crate ships J1939's sets: `J1939U8`, `J1939U16` and `J1939U32`. In the 2- and 4-byte sets every class is a range (`Error(0xFE00..=0xFEFF)`), so each variant holds its code.

| Set | Values | Codes |
|---|---|---|
| `J1939U8` | `0..=250` | `Reserved(251..=253)`, `Error` 0xFE, `NotAvailable` 0xFF |
| `J1939U16` | `0..=0xFAFF` | `Reserved(0xFB00..=0xFDFF)`, `Error(0xFE00..=0xFEFF)`, `NotAvailable(0xFF00..=0xFFFF)` |
| `J1939U32` | `0..=0xFAFF_FFFF` | the same classes in the top byte |
| `Null<CODE>` | the value's bounds | `CODE` alone |

## One code: an Option

`Null<CODE>` is the set with a single code. A `Niched` of it converts to and from `Option`, as Matter's nullable integers and the BLE RSSI do:

```rust
use exact_units::{Fixed, Range, niche::{Niched, Null}, prefix::One, units::Ratio};

type Rssi = Fixed<Ratio, One, i8, Range<-127, 20>>;
type HciRssi = Niched<Rssi, Null<127>>;

assert_eq!(Option::<Rssi>::from(HciRssi::try_from_raw(127).unwrap()), None);
assert_eq!(HciRssi::from(Some(Rssi::bounded(-60))).raw(), -60);
assert_eq!(HciRssi::from(None).raw(), 127);
assert!(HciRssi::try_from_raw(50).is_err()); // neither a value nor the code
```

## Checked at compile time

Every code must fit the storage, no code may be inside the bounds, and no two codes may overlap. A set that breaks one of these doesn't build where it is used:

```rust,compile_fail
use exact_units::{Fixed, Range, niche::{J1939U8, Niched}, prefix::One, units::Ratio};

// 0xFE is the error code, but Range<0, 254> takes it as a value
type Bad = Niched<Fixed<Ratio, One, u8, Range<0, 254>>, J1939U8>;
let _ = Bad::try_from_raw(3);
```

A value without bounds takes every number of its storage, so it leaves no room for a code: give it a `Range`.

## Rails: a value with a mark

A rail is not a code. A 12-bit ADC reads 4095 for 3.3 V and for anything above it. A Q15 after saturation holds its largest value for every larger result. The number is still a value, but it is a limit, not a measurement.

`Clipped<V>` marks the two ends of `V`'s range (its bounds, or the storage's ends without them). `get()` answers `Clip::AtMost` at the low end, `Clip::AtLeast` at the high end and `Clip::Exact` in between. `LowRail` or `HighRail` makes only one end a rail.

```rust
use exact_units::{Fixed, Range, niche::{Clip, Clipped}, prefix::Rational, units::Volt};

// a 12-bit ADC on 3.3 V
type Volts = Fixed<Volt, Rational<33, 40950>, u16, Range<0, 4095>>;
type Adc = Clipped<Volts>;

assert_eq!(Adc::try_from_raw(2048).map(Adc::get), Ok(Clip::Exact(Volts::bounded(2048))));
assert_eq!(Adc::try_from_raw(4095).map(Adc::get), Ok(Clip::AtLeast(Volts::bounded(4095))));
assert_eq!(Adc::try_from_raw(4095).unwrap().to_string(), "≥ 4095·11/13650 V");
```

The two combine. The coolant temperature above sits at its low rail at −40 °C, and its five top byte values are codes, so its full type is `Niched<Clipped<Temp>, J1939U8>`. That is still one byte.

## On the wire and in serde

The bytes are the raw number in both cases. Reading from the wire or through serde refuses a number that is neither a value within the bounds nor a code, just as a bounded `Fixed` refuses a number outside its range:

```text
150 is neither a value of 0..=100 nor a niche code (u8)
```
