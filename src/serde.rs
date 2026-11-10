//! serde for the numbers of this crate (feature `serde`): `no_std`, no alloc.
//!
//! Every value is written as the number it stores, as on the wire: the unit, scale and bounds are in the type, so a
//! `Fixed<Second, Milli, u16>` of 1500 ms is `1500`, in JSON and in postcard alike.
//!
//! - [Fixed], [Point], [Index](crate::Index), [IndexFrom1](crate::IndexFrom1) and
//!   [Seq](crate::Seq): the stored number alone. The binary angles and phases ([Bam16](crate::Bam16),
//!   [PhaseBam16](crate::PhaseBam16)) are `Fixed`, so a `u16`.
//! - [Complex]: `{ re, im }`, a struct of its two parts. In a binary format (postcard, bincode)
//!   that is the two numbers one after the other, as a tuple would be; in a text format the parts are named, so a
//!   config can't swap them, and `[re, im]` reads too.
//! - [Polar]: `{ abs, arg }`, each part as above.
//! - [Niched]: the raw number, a value or a niche code alike; reading refuses a number that is neither.
//!   [Clipped]: its value.
//! - [Decibels]: in a human-readable format (JSON, TOML, RON) the decimal as text,
//!   `"-3.01"`, which reads back exactly (a JSON number reads too: `-3.01`, `6`); in a binary one its two stored
//!   numbers, `(num, decimals)`.
//!
//! - [Exact](crate::Exact) and [Fraction](crate::Fraction): in a human-readable format the exact number as text, in
//!   the unit itself (`"333.(3)"`, `"80000000"`, `"1000/7919"`; an integer reads too, a float is refused, its digits
//!   already rounded), in a binary one `(negative, numerator, denominator)` in lowest terms.
//!
//! Reading checks what construction checks and never builds a value that breaks it: a [Fixed] or an
//! index outside its [Bounds], an [IndexFrom1](crate::IndexFrom1) of 0, a level with more than
//! [MAX_DECIMALS] decimals. The error names the value and the bound:
//! `6000 ms is out of 1..=5000 ms (Second, u16)`.
//!
//! [with_unit] is the human form for a field: `"1.5 ms"`, read back exactly into the field's scale and storage.
//!
//! ```
//! # #[cfg(feature = "time")] {
//! use exact_units::{Fixed, Millis, Range, prefix::Milli, units::Second};
//!
//! #[derive(serde::Serialize, serde::Deserialize, Debug, PartialEq)]
//! struct Config {
//!     period: Millis<u32>,
//!     timeout: Fixed<Second, Milli, u16, Range<1, 5000>>,
//!     #[serde(with = "exact_units::serde::with_unit")]
//!     debounce: Millis<u16>,
//! }
//!
//! let c: Config = serde_json::from_str(r#"{ "period": 20, "timeout": 250, "debounce": "0.015 s" }"#).unwrap();
//! assert_eq!(c.debounce.get(), 15);
//! assert_eq!(serde_json::to_string(&c).unwrap(), r#"{"period":20,"timeout":250,"debounce":"0.015 s"}"#);
//!
//! let e = serde_json::from_str::<Config>(r#"{ "period": 20, "timeout": 6000, "debounce": "15 ms" }"#);
//! assert!(e.unwrap_err().to_string().starts_with("6000 ms is out of 1..=5000 ms (Second, u16)"));
//! # }
//! ```

mod exact;
pub mod with_unit;

use crate::complex::{Complex, ComplexStorage, Polar};
use crate::decibel::{Decibels, MAX_DECIMALS};
use crate::niche::{Clipped, Niched, Niches, Rails, RawValue};
use crate::text;
use crate::{Bounds, Fixed, FixedScale, Origin, Point, Storage, Unit, Unlimited};
use ::serde::de::{self, Deserialize, Deserializer};
use ::serde::ser::{Serialize, Serializer};
use core::fmt;

impl<U, P, T: Storage + Serialize, B: Bounds<T>> Serialize for Fixed<U, P, T, B> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.get().serialize(serializer)
    }
}

impl<'de, U: Unit, P: FixedScale, T: Storage + Deserialize<'de>, B: Bounds<T>> Deserialize<'de>
    for Fixed<U, P, T, B>
{
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let value = T::deserialize(deserializer)?;
        Self::try_new(value).map_err(de::Error::custom)
    }
}

impl<O: Origin<Unit = U>, U, P, T: Storage + Serialize, B: Bounds<T>> Serialize
    for Point<O, Fixed<U, P, T, B>>
{
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.get().serialize(serializer)
    }
}

impl<'de, O: Origin<Unit = U>, U: Unit, P: FixedScale, T: Storage + Deserialize<'de>, B: Bounds<T>>
    Deserialize<'de> for Point<O, Fixed<U, P, T, B>>
{
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Fixed::deserialize(deserializer).map(Point::new)
    }
}

impl<T: Storage + Serialize, V: RawValue<Raw = T>, N: Niches<T>> Serialize for Niched<V, N, T> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.raw().serialize(serializer)
    }
}

impl<'de, T: Storage + Deserialize<'de>, V: RawValue<Raw = T>, N: Niches<T>> Deserialize<'de>
    for Niched<V, N, T>
{
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let raw = T::deserialize(deserializer)?;
        Self::try_from_raw(raw).map_err(|_| de::Error::custom(NotACode::<V> { raw }))
    }
}

/// `150 is neither a value of 0..=100 nor a niche code (u8)`: a number read for a [Niched] that is neither.
struct NotACode<V: RawValue> {
    raw: V::Raw,
}

impl<V: RawValue> fmt::Display for NotACode<V> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.raw.fmt_number(f)?;
        f.write_str(" is neither a value")?;
        if let Some((min, max)) = V::RANGE {
            write!(f, " of {min}..={max}")?;
        }
        write!(f, " nor a niche code ({})", <V::Raw as Storage>::NAME)
    }
}

impl<V: RawValue + Serialize, R: Rails> Serialize for Clipped<V, R> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.value().serialize(serializer)
    }
}

impl<'de, V: RawValue + Deserialize<'de>, R: Rails> Deserialize<'de> for Clipped<V, R> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        V::deserialize(deserializer).map(Clipped::new)
    }
}

/// A [Complex] or a [Polar] as serde writes it: a struct of its named parts.
#[derive(::serde::Serialize, ::serde::Deserialize)]
#[serde(rename = "Complex")]
struct ComplexParts<T> {
    re: T,
    im: T,
}

impl<U, P, T: ComplexStorage + Serialize, B: Unlimited<T>> Serialize for Complex<U, P, T, B> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let (re, im) = self.get();
        ComplexParts { re, im }.serialize(serializer)
    }
}

impl<'de, U, P, T: ComplexStorage + Deserialize<'de>, B: Unlimited<T>> Deserialize<'de>
    for Complex<U, P, T, B>
{
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let ComplexParts { re, im } = ComplexParts::deserialize(deserializer)?;
        Complex::try_new(re, im)
            .map_err(|_| de::Error::custom("a part of the complex value is NaN"))
    }
}

#[derive(::serde::Serialize, ::serde::Deserialize)]
#[serde(rename = "Polar")]
struct PolarParts<F, A> {
    abs: F,
    arg: A,
}

impl<U, P, T: Storage + Serialize, A: Copy + Serialize> Serialize for Polar<U, P, T, A> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        PolarParts {
            abs: self.abs(),
            arg: self.arg(),
        }
        .serialize(serializer)
    }
}

impl<'de, U: Unit, P: FixedScale, T: Storage + Deserialize<'de>, A: Copy + Deserialize<'de>>
    Deserialize<'de> for Polar<U, P, T, A>
{
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let PolarParts { abs, arg } = PolarParts::deserialize(deserializer)?;
        Ok(Polar::new(abs, arg))
    }
}

#[cfg(feature = "counting")]
mod index {
    use crate::fixed::write_range;
    use crate::{Index, IndexFrom1, Seq, UnsignedStorage};
    use ::serde::de::{self, Deserialize, Deserializer};
    use ::serde::ser::{Serialize, Serializer};
    use core::fmt;
    use core::marker::PhantomData;

    impl<Of, T: UnsignedStorage + Serialize, B: crate::Bounds<T>> Serialize for Index<Of, T, B> {
        fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
            self.get().serialize(serializer)
        }
    }

    impl<'de, Of, T: UnsignedStorage + Deserialize<'de>, B: crate::Bounds<T>> Deserialize<'de>
        for Index<Of, T, B>
    {
        fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
            let index = T::deserialize(deserializer)?;
            Self::try_new(index).map_err(|_| {
                de::Error::custom(IndexOutOfRange::<T, B> {
                    from1: false,
                    index,
                    _bounds: PhantomData,
                })
            })
        }
    }

    impl<Of, T: UnsignedStorage + Serialize, B: crate::Bounds<T>> Serialize for IndexFrom1<Of, T, B> {
        fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
            self.get().serialize(serializer)
        }
    }

    impl<'de, Of, T: UnsignedStorage + Deserialize<'de>, B: crate::Bounds<T>> Deserialize<'de>
        for IndexFrom1<Of, T, B>
    {
        fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
            let number = T::deserialize(deserializer)?;
            Self::try_new(number).map_err(|_| {
                de::Error::custom(IndexOutOfRange::<T, B> {
                    from1: true,
                    index: number,
                    _bounds: PhantomData,
                })
            })
        }
    }

    /// `index 7 is out of 0..=3`, `index 0 of a table counted from 1`.
    struct IndexOutOfRange<T, B> {
        from1: bool,
        index: T,
        _bounds: PhantomData<B>,
    }

    impl<T: UnsignedStorage, B: crate::Bounds<T>> fmt::Display for IndexOutOfRange<T, B> {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            if self.from1 && self.index == T::ZERO {
                return f.write_str("index 0 of a table counted from 1");
            }
            f.write_str("index ")?;
            self.index.fmt_number(f)?;
            f.write_str(" is out of ")?;
            write_range::<T, B>(f)
        }
    }

    impl<Of, T: UnsignedStorage + Serialize> Serialize for Seq<Of, T> {
        fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
            self.get().serialize(serializer)
        }
    }

    impl<'de, Of, T: UnsignedStorage + Deserialize<'de>> Deserialize<'de> for Seq<Of, T> {
        fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
            T::deserialize(deserializer).map(Seq::new)
        }
    }
}

/// `-3.01`: a level as text, `-` and the digits with the point `decimals` from the end.
struct DecibelsText(Decibels);

impl fmt::Display for DecibelsText {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let (num, decimals) = (self.0.num(), self.0.decimals());
        if num < 0 {
            f.write_str("-")?;
        }
        let ten = 10u64.pow(u32::from(decimals));
        let abs = num.unsigned_abs();
        #[expect(
            clippy::arithmetic_side_effects,
            reason = "ten is 10^decimals >= 1, decimals <= MAX_DECIMALS (9), so it fits u64 and isn't 0"
        )]
        let (int, frac) = (abs / ten, abs % ten);
        write!(f, "{int}")?;
        if decimals > 0 {
            write!(f, ".{frac:0width$}", width = usize::from(decimals))?;
        }
        Ok(())
    }
}

impl Serialize for Decibels {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        if serializer.is_human_readable() {
            serializer.collect_str(&DecibelsText(*self))
        } else {
            (self.num(), self.decimals()).serialize(serializer)
        }
    }
}

impl<'de> Deserialize<'de> for Decibels {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        if deserializer.is_human_readable() {
            deserializer.deserialize_any(DecibelsVisitor)
        } else {
            let (num, decimals) = <(i64, u8)>::deserialize(deserializer)?;
            decibels(num, decimals)
        }
    }
}

fn decibels<E: de::Error>(num: i64, decimals: u8) -> Result<Decibels, E> {
    if decimals > MAX_DECIMALS {
        return Err(E::custom(format_args!(
            "a level in dB takes up to {MAX_DECIMALS} decimals, not {decimals}"
        )));
    }
    Ok(Decibels::new(num, decimals))
}

/// A level from text (`"-3.01"`, `"6 dB"`), an integer or a float (its shortest decimal form).
struct DecibelsVisitor;

impl de::Visitor<'_> for DecibelsVisitor {
    type Value = Decibels;

    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("a level in dB as a decimal, like \"-3.01\"")
    }

    fn visit_str<E: de::Error>(self, s: &str) -> Result<Decibels, E> {
        let s = s.trim();
        let s = s.strip_suffix("dB").map_or(s, str::trim_end);
        let Some((dec, "")) = text::Decimal::parse(s) else {
            return Err(E::invalid_value(de::Unexpected::Str(s), &self));
        };
        dec.to_decibels().ok_or_else(|| {
            E::custom(format_args!(
                "{s} dB has more than 18 digits or {MAX_DECIMALS} decimals"
            ))
        })
    }

    fn visit_i64<E: de::Error>(self, v: i64) -> Result<Decibels, E> {
        Ok(Decibels::new(v, 0))
    }

    fn visit_u64<E: de::Error>(self, v: u64) -> Result<Decibels, E> {
        i64::try_from(v)
            .map(|v| Decibels::new(v, 0))
            .map_err(|_| E::invalid_value(de::Unexpected::Unsigned(v), &self))
    }

    fn visit_f64<E: de::Error>(self, v: f64) -> Result<Decibels, E> {
        let mut buf = text::Buf::new();
        if fmt::Write::write_fmt(&mut buf, format_args!("{v}")).is_err() {
            return Err(E::invalid_value(de::Unexpected::Float(v), &self));
        }
        self.visit_str(buf.as_str())
    }
}
