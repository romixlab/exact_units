//! A [Fixed] or an [Exact] field as text with its unit, `"1.5 ms"`: `#[serde(with =
//! "exact_units::serde::with_unit")]`, and [option] for an `Option` of one. No alloc: the text is written with
//! `collect_str` and read from the `&str` the format hands over.
//!
//! Written as `{:#}` writes it: the value in the unit itself, exact where the steps are a fraction of integers
//! whose decimal ends (`0.0015 s`, a [Q15](crate::Q15) of 0.8 as `0.79998779296875`), the shortest float form
//! otherwise (a [Duty16](crate::Duty16) of 1/65535 steps, a scale with π).
//!
//! Read: a decimal number (`1.5`, `-2e-3`, `1_000`), then the field's unit, with or without an SI prefix (`1.5 ms`,
//! `1500 µs` or `1500 us`, `0.0015 s`; a binary prefix too, `4 KiB`), the space optional. A ratio takes `%`, `‰`,
//! `ppm` and `ppb`, or nothing. Square and cubic metres read as `{}` writes them: `5 mm²` (10⁻⁶ m², and `5 m(m²)` is
//! 10⁻³ m²), `5 cm³/kg`, and litres on a volume and a volume flow, `5 mL/s` as well as `5 cm³/s`. A quotient reads with
//! a prefix on either side, as people write it, whichever spelling `{}` writes for the field: `5 µS/cm` and `5 uS/cm`,
//! `5 mg/L` and `5 g/m³`, `5 dB/km`, and the older `5 m(dB/m)`. The value goes into the field's scale and storage
//! exactly, else it is an error that says why:
//!
//! - not the field's unit: `"5 kg" is not in s (with or without an SI prefix)`;
//! - not a whole number of steps: `"1.2345 ms" is not a whole number of 1 µs steps`, with the nearest values the
//!   field can hold. Where `{:#}` writes a value of the field as a float (the decimal of its steps doesn't end), the
//!   text it writes reads back to that value, so every value round-trips;
//! - past the storage: `"70 s" doesn't fit u16 in 1 ms steps`;
//! - outside the bounds, as the plain number is: `6000 ms is out of 1..=5000 ms (Second, u16)`.
//!
//! A float storage takes the nearest float to the value, worked out in `f64`.
//!
//! An [Exact] field is written as `{}` writes it, with the prefix it picks: `"79.99998046875 MHz"`, `"333.(3) kHz"`,
//! `"1000/7919 kHz"`. It reads what a `Fixed` of its unit reads, a repetend in parentheses and a fraction too
//! (`"1000/3 kHz"`), always exactly: there are no steps to miss and no storage to pass, so the refusals are another
//! unit, not a number, and a number past what a fraction of `u128`s holds.
//!
//! ```
//! # #[cfg(feature = "time")] {
//! use exact_units::{Exact, units::Hertz};
//!
//! #[derive(serde::Serialize, serde::Deserialize)]
//! struct Clock {
//!     #[serde(with = "exact_units::serde::with_unit")]
//!     target: Exact<Hertz>,
//! }
//!
//! let c: Clock = serde_json::from_str(r#"{ "target": "79.99998046875 MHz" }"#).unwrap();
//! assert_eq!(c.target, Exact::new(2_559_999_375, 32));
//! let c: Clock = serde_json::from_str(r#"{ "target": "80000/3 kHz" }"#).unwrap();
//! assert_eq!(serde_json::to_string(&c).unwrap(), r#"{"target":"26.(6) MHz"}"#);
//! let e = serde_json::from_str::<Clock>(r#"{ "target": "24 MV" }"#).err().unwrap();
//! assert!(e.to_string().starts_with(r#""24 MV" is not in Hz (with or without an SI prefix)"#));
//! # }
//! ```
//!
//! ```
//! # #[cfg(feature = "electrical")] {
//! use exact_units::{Fixed, prefix::Milli, units::Volt};
//!
//! #[derive(serde::Serialize, serde::Deserialize)]
//! struct Rail {
//!     #[serde(with = "exact_units::serde::with_unit")]
//!     level: Fixed<Volt, Milli, u16>,
//!     #[serde(with = "exact_units::serde::with_unit::option", default)]
//!     trip: Option<Fixed<Volt, Milli, u16>>,
//! }
//!
//! let r: Rail = serde_json::from_str(r#"{ "level": "3.3 V" }"#).unwrap();
//! assert_eq!(r.level.get(), 3300);
//! assert_eq!(r.trip, None);
//! let r: Rail = serde_json::from_str(r#"{ "level": "3300 mV", "trip": "3.6V" }"#).unwrap();
//! assert_eq!(serde_json::to_string(&r).unwrap(), r#"{"level":"3.3 V","trip":"3.6 V"}"#);
//!
//! let e = serde_json::from_str::<Rail>(r#"{ "level": "3.3005 V" }"#).err().unwrap();
//! assert!(e.to_string().starts_with(r#""3.3005 V" is not a whole number of 1 mV steps; the nearest are 3.3 V and 3.301 V"#));
//! # }
//! ```

use crate::OutOfRange;
use crate::exact::read_text as read_exact;
use crate::fixed::write_prefix_unit;
use crate::text::unit::{TextPrefix, UnitText, prefix_of};
use crate::text::{Buf, Decimal};
use crate::tree::is_ratio;
use crate::{Bounds, Exact, ExactNum, Fixed, FixedScale, ParseExactError, Storage, Unit};
use ::serde::de::{self, Deserializer, Visitor};
use ::serde::ser::Serializer;
use core::fmt::{self, Write};
use core::marker::PhantomData;

/// A value [with_unit](super::with_unit) writes and reads as text: a [Fixed] (written as `{:#}`, read exactly into
/// its scale and storage) or an [Exact] (written as `{}`, with the prefix it picks, read exactly). Sealed.
pub trait Value: sealed::Sealed + Sized {
    #[doc(hidden)]
    fn write_text<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error>;

    #[doc(hidden)]
    fn read_text<E: de::Error>(s: &str) -> Result<Self, E>;

    #[doc(hidden)]
    fn expecting(f: &mut fmt::Formatter<'_>) -> fmt::Result;
}

mod sealed {
    pub trait Sealed {}

    impl<U, P, T: crate::Storage, B: crate::Bounds<T>> Sealed for crate::Fixed<U, P, T, B> {}

    impl<U, R: crate::ExactNum> Sealed for crate::Exact<U, R> {}
}

/// Writes `value` as text with its unit: a [Fixed] as `{:#}` writes it (`"1.5 ms"` is written `"0.0015 s"`), an
/// [Exact] as `{}` does (`"79.99998046875 MHz"`).
pub fn serialize<S: Serializer, V: Value>(value: &V, serializer: S) -> Result<S::Ok, S::Error> {
    value.write_text(serializer)
}

/// Reads text with a unit, exactly into the field's type, or fails saying why.
pub fn deserialize<'de, D: Deserializer<'de>, V: Value>(deserializer: D) -> Result<V, D::Error> {
    deserializer.deserialize_str(TextVisitor(PhantomData))
}

/// [with_unit](super::with_unit) for an `Option<Fixed<..>>` or an `Option<Exact<..>>`: `None` is serde's none
/// (`null`), `Some` the text. Add `#[serde(default)]` for a field that may be left out.
pub mod option {
    use super::{TextVisitor, Value, WithUnit};
    use ::serde::de::{self, Deserializer, Visitor};
    use ::serde::ser::Serializer;
    use core::fmt;
    use core::marker::PhantomData;

    /// `None` as none, `Some` as [serialize](super::serialize) writes it.
    pub fn serialize<S: Serializer, V: Value>(
        value: &Option<V>,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        match value {
            Some(v) => serializer.serialize_some(&WithUnit(v)),
            None => serializer.serialize_none(),
        }
    }

    /// None, or text read as [deserialize](super::deserialize) reads it.
    pub fn deserialize<'de, D: Deserializer<'de>, V: Value>(
        deserializer: D,
    ) -> Result<Option<V>, D::Error> {
        deserializer.deserialize_option(OptionVisitor(PhantomData))
    }

    struct OptionVisitor<V>(PhantomData<V>);

    impl<'de, V: Value> Visitor<'de> for OptionVisitor<V> {
        type Value = Option<V>;

        fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.write_str("none, or ")?;
            TextVisitor::<V>(PhantomData).expecting(f)
        }

        fn visit_none<E: de::Error>(self) -> Result<Self::Value, E> {
            Ok(None)
        }

        fn visit_unit<E: de::Error>(self) -> Result<Self::Value, E> {
            Ok(None)
        }

        fn visit_some<D: Deserializer<'de>>(
            self,
            deserializer: D,
        ) -> Result<Self::Value, D::Error> {
            super::deserialize(deserializer).map(Some)
        }
    }
}

/// A value written as text, for [option] and collections.
struct WithUnit<'a, V>(&'a V);

impl<V: Value> ::serde::Serialize for WithUnit<'_, V> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.0.write_text(serializer)
    }
}

struct TextVisitor<V>(PhantomData<V>);

impl<V: Value> Visitor<'_> for TextVisitor<V> {
    type Value = V;

    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        V::expecting(f)
    }

    fn visit_str<E: de::Error>(self, s: &str) -> Result<Self::Value, E> {
        V::read_text(s)
    }
}

impl<U: Unit, P: FixedScale, T: Storage, B: Bounds<T>> Value for Fixed<U, P, T, B> {
    fn write_text<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_str(&format_args!("{self:#}"))
    }

    fn read_text<E: de::Error>(s: &str) -> Result<Self, E> {
        read::<U, P, T, B>(s).map_err(|kind| {
            E::custom(TextError::<U, P, T, B> {
                input: s,
                kind,
                _type: PhantomData,
            })
        })
    }

    fn expecting(f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "a number with its unit, like \"1.5{}\"",
            UnitText::<U>(PhantomData)
        )
    }
}

impl<U: Unit, R: ExactNum> Value for Exact<U, R> {
    fn write_text<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_str(self)
    }

    fn read_text<E: de::Error>(s: &str) -> Result<Self, E> {
        read_exact::<U, R>(s)
            .map_err(|kind| E::custom(format_args!("{s:?} is {}", ParseExactError::<U>::new(kind))))
    }

    fn expecting(f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "an exact number with its unit, like \"1.5{0}\" or \"1000/3{0}\"",
            UnitText::<U>(PhantomData)
        )
    }
}

/// What went wrong reading `input`.
enum Kind<T> {
    /// Not a number followed by text.
    Form,
    /// The text after the number isn't the unit.
    Unit,
    /// Between two values the field can hold, `None` where that one is past the storage.
    NotWhole(Option<T>, Option<T>),
    /// Past the storage.
    Fit,
    /// Outside the bounds.
    Bounds(T),
}

struct TextError<'a, U, P, T, B> {
    input: &'a str,
    kind: Kind<T>,
    _type: PhantomData<(U, P, B)>,
}

impl<U: Unit, P: FixedScale, T: Storage, B: Bounds<T>> fmt::Display for TextError<'_, U, P, T, B> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let input = self.input;
        let unit = UnitText::<U>(PhantomData);
        match self.kind {
            Kind::Form => write!(
                f,
                "{input:?} is not a number with its unit, like \"1.5{unit}\""
            ),
            Kind::Unit if is_ratio(&U::EXPR) => {
                write!(
                    f,
                    "{input:?} is not a ratio: a number, or one with %, ‰, ppm or ppb"
                )
            }
            Kind::Unit => write!(
                f,
                "{input:?} is not in{unit} (with or without an SI prefix)"
            ),
            Kind::NotWhole(below, above) => {
                write!(f, "{input:?} is not a whole number of ")?;
                write_step::<U, P>(f)?;
                f.write_str(" steps")?;
                match (below, above) {
                    (Some(a), Some(b)) => write!(
                        f,
                        "; the nearest are {:#} and {:#}",
                        fixed::<U, P, T>(a),
                        fixed::<U, P, T>(b)
                    ),
                    (Some(a), None) | (None, Some(a)) => {
                        write!(f, "; the nearest is {:#}", fixed::<U, P, T>(a))
                    }
                    (None, None) => Ok(()),
                }
            }
            Kind::Fit => {
                write!(f, "{input:?} doesn't fit {} in ", T::NAME)?;
                write_step::<U, P>(f)?;
                f.write_str(" steps")
            }
            Kind::Bounds(v) => write!(f, "{}", OutOfRange::<U, P, T, B>::new(v)),
        }
    }
}

/// `1 ms`, `1·2⁻¹⁵`: one step of `P` in `U`.
fn write_step<U: Unit, P: FixedScale>(f: &mut fmt::Formatter<'_>) -> fmt::Result {
    f.write_str("1")?;
    write_prefix_unit::<P>(U::EXPR, f)
}

/// `v` for an error message only: it may be NaN, and is never handed out.
fn fixed<U, P, T: Storage>(v: T) -> Fixed<U, P, T> {
    Fixed::new_unchecked(v)
}

/// `s` as a value of the field.
fn read<U: Unit, P: FixedScale, T: Storage, B: Bounds<T>>(
    s: &str,
) -> Result<Fixed<U, P, T, B>, Kind<T>> {
    let s = s.trim();
    let (special, (dec, rest)) = match Decimal::parse(s) {
        Some(d) => (None, d),
        None if T::IS_FLOAT => match special_float(s) {
            Some((x, rest)) => (
                Some(x),
                (
                    Decimal {
                        neg: false,
                        m: 0,
                        e: 0,
                    },
                    rest,
                ),
            ),
            None => return Err(Kind::Form),
        },
        None => return Err(Kind::Form),
    };
    let prefix = prefix_of::<U>(rest.trim()).ok_or(Kind::Unit)?;
    let v = if T::IS_FLOAT {
        let x = match special {
            Some(x) => x,
            None => to_f64(s, rest).ok_or(Kind::Form)?,
        };
        let x = prefix.prefix.scale_f64(x) / P::SCALE.to_f64();
        T::from_float(x).ok_or(Kind::Fit)?
    } else {
        steps::<U, P, T>(dec, prefix)?
    };
    Fixed::try_new(v).map_err(|_| Kind::Bounds(v))
}

/// `inf`, `-infinity`, `NaN` at the start of `s` (any case), for a float storage.
fn special_float(s: &str) -> Option<(f64, &str)> {
    let (neg, t) = match s.as_bytes().first() {
        Some(b'-') => (true, s.get(1..)?),
        Some(b'+') => (false, s.get(1..)?),
        _ => (false, s),
    };
    for (word, x) in [
        ("infinity", f64::INFINITY),
        ("inf", f64::INFINITY),
        ("nan", f64::NAN),
    ] {
        let head = t.get(..word.len());
        if head.is_some_and(|h| h.eq_ignore_ascii_case(word)) {
            let rest = t.get(word.len()..)?;
            return Some((if neg { -x } else { x }, rest));
        }
    }
    None
}

/// The number at the start of `s` (before `rest`) as the nearest `f64`: core's parser, which rounds correctly, on
/// the digits without their `_` (copied for that, so up to 128 bytes; without `_` of any length).
fn to_f64(s: &str, rest: &str) -> Option<f64> {
    let number = s.get(..s.len().checked_sub(rest.len())?)?;
    if !number.contains('_') {
        return number.parse().ok();
    }
    let mut buf = Buf::new();
    for part in number.split('_') {
        buf.write_str(part).ok()?;
    }
    buf.as_str().parse().ok()
}

/// The exact number of steps of `P` in `dec` times `prefix`, as `T`; where that is not whole, the value whose `{:#}`
/// text is `dec` (a scale whose decimals don't end writes the float's shortest form).
fn steps<U: Unit, P: FixedScale, T: Storage>(
    dec: Decimal,
    prefix: TextPrefix,
) -> Result<T, Kind<T>> {
    let value = dec.times_pow10(prefix.pow10).ok_or(Kind::Fit)?;
    let Some((num, den)) = P::SCALE.to_ratio() else {
        // a scale with π or a root: the nearest values by f64, then the one that writes as the text
        let x = prefix.prefix.scale_f64(dec_to_f64(dec)) / P::SCALE.to_f64();
        #[expect(
            clippy::as_conversions,
            clippy::cast_possible_truncation,
            reason = "a float to the integer next to it, saturating at the ends of i128: only a candidate, checked below"
        )]
        let n = x as i128;
        return written_as::<U, P, T>(value, prefix, [n.saturating_sub(1), n, n.saturating_add(1)]);
    };
    // value · den / num, in lowest terms: the powers first, so the digits of a decimal that is exactly some binary
    // steps (a Q31's 31 digits) cancel before the scale multiplies; the scale first where that overflows
    let powers = |f: Frac| f.pow(10, value.e).and_then(|f| f.pow(2, prefix.pow2));
    let scale = |f: Frac| f.mul(den).and_then(|f| f.div(num));
    let m = Frac::of(dec.m);
    let f = powers(m)
        .and_then(scale)
        .or_else(|| scale(m).and_then(powers));
    let Some(f) = f else {
        // past u128: far too large for any storage, or a fraction of a step far too small to be one
        return Err(if value.e < 0 {
            Kind::NotWhole(None, None)
        } else {
            Kind::Fit
        });
    };
    let sign = |x: u128| {
        let x = i128::try_from(x).ok()?;
        if dec.neg { x.checked_neg() } else { Some(x) }
    };
    if f.q == 1 {
        let n = sign(f.p).ok_or(Kind::Fit)?;
        return T::from_int(n).ok_or(Kind::Fit);
    }
    // between floor and floor + 1 steps
    let floor = f.p.checked_div(f.q).ok_or(Kind::Fit)?;
    let (a, b) = (sign(floor), floor.checked_add(1).and_then(sign));
    let found = [a, b].into_iter().flatten();
    written_as::<U, P, T>(value, prefix, found).map_err(|_| {
        match (a.and_then(T::from_int), b.and_then(T::from_int)) {
            (None, None) => Kind::Fit,
            // the lower value first
            (a, b) if dec.neg => Kind::NotWhole(b, a),
            (a, b) => Kind::NotWhole(a, b),
        }
    })
}

/// The candidate whose `{:#}` text is `value` (in the unit, prefix applied), else the nearest two for the error.
fn written_as<U: Unit, P: FixedScale, T: Storage>(
    value: Decimal,
    prefix: TextPrefix,
    candidates: impl IntoIterator<Item = i128>,
) -> Result<T, Kind<T>> {
    let mut near: [Option<T>; 2] = [None, None];
    for n in candidates {
        let Some(v) = T::from_int(n) else { continue };
        if prefix.pow2 == 0 && writes_value::<U, P, T>(v, value) {
            return Ok(v);
        }
        near = match near {
            [None, _] => [Some(v), None],
            [a, None] => [a, Some(v)],
            [_, b] => [b, Some(v)],
        };
    }
    match near {
        [None, None] => Err(Kind::Fit),
        [a, b] => Err(Kind::NotWhole(a, b)),
    }
}

/// Whether `{:#}` of `v` writes the number `value`.
fn writes_value<U: Unit, P: FixedScale, T: Storage>(v: T, value: Decimal) -> bool {
    let mut buf = Buf::new();
    if write!(buf, "{:#}", fixed::<U, P, T>(v)).is_err() {
        return false;
    }
    Decimal::parse(buf.as_str()).is_some_and(|(d, _)| d == value)
}

/// The nearest `f64` to `dec`, near enough to pick candidates: rounded at each power of ten.
fn dec_to_f64(dec: Decimal) -> f64 {
    #[expect(
        clippy::as_conversions,
        clippy::cast_precision_loss,
        reason = "the nearest f64, a candidate only"
    )]
    let mut x = dec.m as f64;
    let (step, n) = if dec.e >= 0 {
        (10.0, dec.e)
    } else {
        (0.1, dec.e.saturating_neg())
    };
    for _ in 0..n.min(400) {
        x *= step;
    }
    if dec.neg { -x } else { x }
}

/// A fraction of `u128`s in lowest terms, `q >= 1`.
#[derive(Copy, Clone)]
struct Frac {
    p: u128,
    q: u128,
}

impl Frac {
    fn of(p: u128) -> Frac {
        Frac { p, q: 1 }
    }

    /// `self · n`, `n >= 1`; `None` past `u128`.
    fn mul(self, n: u128) -> Option<Frac> {
        let g = gcd(n, self.q);
        Some(Frac {
            p: self.p.checked_mul(n.checked_div(g)?)?,
            q: self.q.checked_div(g)?,
        })
    }

    /// `self / n`, `n >= 1`; `None` past `u128`.
    fn div(self, n: u128) -> Option<Frac> {
        let g = gcd(n, self.p);
        Some(Frac {
            p: self.p.checked_div(g)?,
            q: self.q.checked_mul(n.checked_div(g)?)?,
        })
    }

    /// `self · base^e`; `None` past `u128`. A zero stays zero for any `e`.
    fn pow(self, base: u128, e: i32) -> Option<Frac> {
        if self.p == 0 {
            return Some(self);
        }
        let mut f = self;
        for _ in 0..e.unsigned_abs() {
            f = if e > 0 { f.mul(base)? } else { f.div(base)? };
        }
        Some(f)
    }
}

/// The greatest common divisor, `gcd(0, n) = n`.
fn gcd(mut a: u128, mut b: u128) -> u128 {
    while let Some(r) = a.checked_rem(b) {
        (a, b) = (b, r);
    }
    a
}
