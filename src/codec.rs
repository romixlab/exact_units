//! [Coded]: a number given by a non-linear code instead of a linear step, beside [Fixed]: a μ-law or A-law byte
//! (telephony), a gamma-encoded pixel, a log-encoded illuminance (BLE). `Fixed`'s scale is always linear and stays
//! that way; a non-linear encoding is a wrapper instead, with the bytes of the raw code.
//!
//! - [Codec]: `type Raw` (the code, a [Storage]), `type Linear` (the value it means), [DEF](Codec::DEF) (the
//!   algorithm and the spare codes as data, a [CodecDef]), [decode](Codec::decode) (`None` exactly for a spare
//!   code, a code that means no value) and [encode](Codec::encode) (by a named [Rounding], refused only when the
//!   value is beyond every code's range, never a spare code).
//! - [CodecDef] names the [CodecShape] and the [spare](CodecDef::spare) codes: what a client without the trait
//!   runs to decode the same bytes, as [UnitDef](crate::UnitDef) is a unit's definition as data. [MuLaw] and
//!   [MuLawSingleZero] back [CodecShape::MuLaw], [CanFdDlc] (CAN FD's data length code) backs
//!   [CodecShape::Table], video's narrow-range quantizers ([video]) back [CodecShape::Offset]; the other shapes
//!   are the plan for later codecs (LVL-4's named conversions) and have none yet, a gap this module's docs don't
//!   hide.
//! - [Coded]`<C, V>` holds the bytes of `C::Raw`, never a spare code: [get](Coded::get) decodes,
//!   [try_new](Coded::try_new) encodes, [try_from_raw](Coded::try_from_raw) refuses a spare code ([SpareCode]).
//!   Its spare codes are where a [Niched](niche::Niched) puts its own: `Niched<Coded<MuLawSingleZero>,
//!   Null<0x7F>>` is an `Option` of a μ-law sample in one byte.
//!
//! ```
//! # #[cfg(feature = "dimensionless")] {
//! use exact_units::codec::{Coded, MuLaw};
//! use exact_units::Rounding;
//!
//! // byte 0x80 decodes exactly to the standard's largest positive 14-bit sample, 8031
//! let loudest = Coded::<MuLaw>::from_raw(0x80);
//! assert_eq!(loudest.get().get(), 8031);
//!
//! // and encodes back to the same byte, as every code does but G.711's negative zero, 0x7F (back as 0xFF)
//! assert_eq!(
//!     Coded::<MuLaw>::try_new(loudest.get(), Rounding::NearestEven),
//!     Ok(loudest)
//! );
//! # }
//! ```

#[cfg(all(feature = "std", feature = "shrink_wrap"))]
use crate::MarkerIdOwned;
#[cfg(any(feature = "dimensionless", feature = "information"))]
use crate::fixed::Fixed;
#[cfg(feature = "dimensionless")]
use crate::fixed::prefix::Pow2;
#[cfg(feature = "information")]
use crate::fixed::{OneOf, prefix::One};
use crate::fixed::{OutOfBounds, Repr, Storage, ValueSet, nearest_entry};
use crate::niche::{self, RawValue};
#[cfg(any(feature = "dimensionless", feature = "information"))]
use crate::system::def_list;
use crate::system::{DefList, for_each, slice_of};
use crate::{Magnitude, MarkerId, Rounding};
use core::fmt;
use core::hash::{Hash, Hasher};
use core::marker::PhantomData;
#[cfg(feature = "shrink_wrap")]
use shrink_wrap::prelude::*;
#[cfg(feature = "shrink_wrap")]
use shrink_wrap::refused::{Refused, Rule};

/// A non-linear encoding: `Raw` (the code on the wire) and `Linear` (the value it means). [decode](Self::decode)
/// refuses exactly the [spare](CodecDef::spare) codes [DEF](Self::DEF) names (none for a codec whose every code is
/// a value, [MuLaw]); [encode](Self::encode) gives `None` only for a `Linear` beyond every code's range, and
/// within it the nearest code by the given [Rounding], never a spare one. `DEF` is the same algorithm as data, for
/// a client without this trait, and the one place the spare codes are written: the typed path and the data path
/// read the same list.
///
/// A spare code is what a [Niched](niche::Niched) claims for a meaning: `Niched<Coded<C>, N>` takes `N`'s codes
/// from `C`'s spare ones. IEEE 11073-20601's SFLOAT (IO-3) is the case this is shaped for: its five codes around
/// `0x0800` (+∞, NaN, NRes, a reserved one, −∞) are no number, so its `DEF` names them spare and a [niches!](crate::niches!)
/// enum gives each its meaning. `Linear` here is a placeholder pair of mantissa and decimal exponent; IO-3 gives
/// SFLOAT its own exact type and [CodecShape]:
///
/// ```
/// use exact_units::codec::{Codec, CodecDef, CodecShape, CodeSpan, Coded};
/// use exact_units::niche::{Niched, Reading};
/// use exact_units::{MarkerId, Rounding, niches};
///
/// #[derive(Copy, Clone)]
/// struct Sfloat;
///
/// impl Codec for Sfloat {
///     type Raw = u16;
///     type Linear = (i16, i8); // mantissa · 10^exponent
///
///     const DEF: CodecDef<'static> = CodecDef {
///         id: MarkerId { crate_name: "doc", name: "Sfloat" },
///         shape: CodecShape::Table(exact_units::system::def_list(&[])), // IO-3 names SFLOAT's own shape
///         spare: exact_units::system::def_list(&[CodeSpan { low: 0x07FE, high: 0x0802 }]),
///     };
///
///     fn formula(raw: u16) -> (i16, i8) {
///         let exponent = (raw.cast_signed() >> 12) as i8; // the top 4 bits, sign-extended
///         let mantissa = ((raw << 4).cast_signed()) >> 4; // the low 12 bits, sign-extended
///         (mantissa, exponent)
///     }
///
///     fn encode(_: (i16, i8), _: Rounding) -> Option<u16> {
///         None // IO-3
///     }
/// }
///
/// niches! {
///     /// SFLOAT's special codes.
///     pub enum SfloatSpecial: u16 {
///         /// +∞.
///         PlusInfinity = 0x07FE,
///         /// Not a number.
///         Nan = 0x07FF,
///         /// Not at this resolution.
///         NRes = 0x0800,
///         /// Reserved for a future use.
///         Reserved = 0x0801,
///         /// −∞.
///         MinusInfinity = 0x0802,
///     }
/// }
///
/// type Reading11073 = Niched<Coded<Sfloat>, SfloatSpecial>;
///
/// // 0x07FF would be 2047 by the formula, but it is NaN: decode refuses it, the niche names it
/// assert_eq!(Sfloat::decode(0x07FF), None);
/// assert_eq!(Reading11073::try_from_raw(0x07FF).map(Reading11073::get), Ok(Reading::Niche(SfloatSpecial::Nan)));
/// // 0xF07D is 125 · 10⁻¹
/// let r = Reading11073::try_from_raw(0xF07D).map(|r| r.get().value().map(Coded::get));
/// assert_eq!(r, Ok(Some((125, -1))));
/// assert_eq!(size_of::<Reading11073>(), 2);
/// ```
pub trait Codec: Copy + 'static {
    /// The code: what is in memory and on the wire.
    type Raw: Storage;

    /// The value a code means.
    type Linear;

    /// Who this codec is, how it computes and which codes are spare, for introspection and a dynamic client.
    const DEF: CodecDef<'static>;

    /// What the codec's formula gives `raw`, spare or not: for a spare code a number that means nothing (SFLOAT's
    /// NaN code reads as 2047), so call [decode](Self::decode) unless the code is known not to be spare.
    /// [Coded::get] calls this, as a `Coded` never holds a spare code.
    fn formula(raw: Self::Raw) -> Self::Linear;

    /// The value `raw` means, `None` for a [spare](CodecDef::spare) code.
    #[inline]
    fn decode(raw: Self::Raw) -> Option<Self::Linear> {
        if in_spans(Coded::<Self>::SPARE, raw) {
            None
        } else {
            Some(Self::formula(raw))
        }
    }

    /// `value` by the nearest code, ties broken by `rounding`: never a [spare](CodecDef::spare) code.
    ///
    /// # Errors
    ///
    /// `None` where `value` is beyond every code's range.
    fn encode(value: Self::Linear, rounding: Rounding) -> Option<Self::Raw>;
}

/// Where a [Codec]'s algorithm is found, as data a client without the trait can run: an id ([MarkerId], as
/// [UnitDef](crate::UnitDef)'s is) and a [CodecShape]. [Codec::DEF] carries one for every codec in this crate.
///
/// On the wire (feature `shrink_wrap`): WireWeaver's `MarkerDef::Codec` is where a schema holds one; its walker,
/// `ww_self`, `ww_si` and the conformance corpus are their own, separate landing (filed, not done here).
#[cfg_attr(
    feature = "shrink_wrap",
    derive_shrink_wrap(
        borrowed,
        owned(feature = "std"),
        derive_owned(Clone, Debug, PartialEq),
        cfg_attr_owned(feature = "serde", derive(serde::Serialize, serde::Deserialize))
    )
)]
#[derive(Copy, Clone, Debug, PartialEq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct CodecDef<'i> {
    /// The crate and marker that declare it.
    pub id: MarkerId<'i>,
    /// What it computes.
    pub shape: CodecShape<'i>,
    /// The codes that mean no value: free for a [Niched](niche::Niched) to claim, refused by
    /// [Coded::try_from_raw] and [Codec::decode]. Empty for a codec whose every code is a value ([MuLaw]).
    pub spare: &'i [CodeSpan],
}

impl CodecDef<'_> {
    /// `raw` decoded by this definition alone, as [Codec::decode] does: `None` for a [spare](Self::spare) code,
    /// else what [CodecShape::decode] gives, the same algorithm the typed path runs. `None` too for a shape with no
    /// algorithm here yet.
    #[must_use]
    pub fn decode(&self, raw: i128) -> Option<i128> {
        if self.is_spare(raw) {
            return None;
        }
        self.shape.decode(raw)
    }

    /// `value` encoded by this definition alone, as [Codec::encode] does: [CodecShape::encode], never a spare code.
    ///
    /// # Errors
    ///
    /// `None` for a shape with no algorithm here yet, or where `value` is beyond every code's range.
    #[must_use]
    pub fn encode(&self, value: i128, rounding: Rounding) -> Option<i128> {
        self.shape
            .encode_where(value, rounding, |raw| !self.is_spare(raw))
            .filter(|&raw| !self.is_spare(raw))
    }

    /// Whether `raw` is one of the [spare](Self::spare) codes. `true` too where a span read from the wire doesn't
    /// decode: a code this definition can't vouch for is no value.
    #[must_use]
    pub fn is_spare(&self, raw: i128) -> bool {
        let mut hit = false;
        let whole = for_each(&self.spare, |s: CodeSpan| hit |= s.contains(raw));
        hit || !whole
    }
}

/// An inclusive range of codes, `low..=high`, as integers: a single code is `low == high`. [CodecDef::spare] lists
/// them.
#[cfg_attr(
    feature = "shrink_wrap",
    derive_shrink_wrap(borrowed, owned(feature = "std"), final_structure)
)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct CodeSpan {
    /// The lowest code.
    pub low: i128,
    /// The highest code.
    pub high: i128,
}

impl CodeSpan {
    /// The one code `code`.
    #[must_use]
    pub const fn one(code: i128) -> Self {
        CodeSpan {
            low: code,
            high: code,
        }
    }

    /// Whether `raw` is in `low..=high`.
    #[must_use]
    pub const fn contains(self, raw: i128) -> bool {
        self.low <= raw && raw <= self.high
    }
}

/// The spare codes of a `Codec`'s [DEF](Codec::DEF), checked: each span ascending and within `t_min..=t_max`, a
/// compile error where it isn't (this is only evaluated in a `const`).
#[expect(
    clippy::panic,
    reason = "const fn evaluated only in a const item: the panic is a compile error naming the broken definition"
)]
const fn checked_spare(def: &CodecDef<'static>, t_min: i128, t_max: i128) -> &'static [CodeSpan] {
    let Some(spare) = slice_of(&def.spare) else {
        panic!("a Codec's DEF.spare is const data, a slice");
    };
    let mut rest = spare;
    while let [s, tail @ ..] = rest {
        assert!(
            s.low <= s.high,
            "a spare span's low code is above its high code"
        );
        assert!(
            s.low >= t_min && s.high <= t_max,
            "a spare code doesn't fit the codec's Raw"
        );
        rest = tail;
    }
    spare
}

/// Whether `raw` is in one of `spare`'s spans.
#[inline(always)]
fn in_spans<T: Storage>(spare: &[CodeSpan], raw: T) -> bool {
    match raw.to_repr() {
        Repr::Int(x) => spare.iter().any(|s| s.contains(x)),
        Repr::Float(_) => false,
    }
}

/// What a [Codec] computes, as data: the family of algorithm, with its parameters where it has any. Every shape
/// here is exact (a table or an integer formula) or names where it isn't yet, never a float standing in for it.
#[cfg_attr(
    feature = "shrink_wrap",
    derive_shrink_wrap(
        borrowed,
        owned(feature = "std"),
        derive_owned(Clone, Debug, PartialEq),
        cfg_attr_owned(feature = "serde", derive(serde::Serialize, serde::Deserialize)),
        cfg_attr_owned(
            feature = "serde",
            expect(clippy::unsafe_derive_deserialize, reason = "the unsafe is shrink_wrap's, in the generated owned type")
        ),
        ww_repr = unib32
    )
)]
#[derive(Copy, Clone, Debug, PartialEq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum CodecShape<'i> {
    /// ITU-T G.711 μ-law: 8 segments of 16 steps, doubling in size. [MuLaw] is the worked [Codec].
    MuLaw,
    /// ITU-T G.711 A-law: as μ-law's segments, the even ones linear. No [Codec] in this crate yet (LVL-4).
    ALaw,
    /// A gamma curve, `v' = v^gamma`: sRGB's shape away from its linear segment near zero. No [Codec] in this
    /// crate yet (LVL-4); `gamma` is exact, never an `f64` standing in for it.
    Power {
        /// The exponent.
        gamma: Magnitude,
    },
    /// A gamma curve with a linear segment near zero (sRGB, L*): [Power](CodecShape::Power) plus the segment's own
    /// slope and breakpoint, not yet named here. No [Codec] in this crate yet (LVL-4).
    PiecewisePower,
    /// An exponential curve, `base^(v + offset)` (BLE illuminance's log scale). No [Codec] in this crate yet (LVL-4).
    Exp {
        /// The base.
        base: Magnitude,
        /// Added before raising to `base`.
        offset: Magnitude,
    },
    /// The linear value of every code, by raw code from 0: entry `i` is what code `i` means, a code past the end
    /// means no value. Decoding looks the code up; encoding takes the code of the value, or the one a named
    /// [Rounding] picks among the codes that aren't spare (the lowest code where two mean the same value). The
    /// entries need not be ascending. [CanFdDlc] is the worked [Codec]; IEEE 11073 SFLOAT (IO-3), whose codes
    /// outside a formula (NaN, ±∞, "not at this resolution") are the common case, is the next.
    Table(&'i [i128]),
    /// A code that is the `Linear`'s raw value plus a fixed `zero`: the `Linear` is `code − zero`, its scale in the
    /// `Linear`'s own [Fixed] (`Rational<1, 219>`). Affine, so exact both ways, and encoding needs no rounding.
    /// Video's narrow-range quantization is the case ([Narrow8](video::Narrow8): E′ = (D − 16)/219, ITU-R BT.709-6
    /// Part 2 item 3.4), where codes past the nominal range are values and the timing references are spare.
    Offset {
        /// The code of the `Linear`'s zero (16 for 8-bit narrow-range Y′, 128 for C′B and C′R).
        zero: i128,
    },
}

impl CodecShape<'_> {
    /// `raw` decoded by this shape, as its [Codec] would, but from the shape alone: a dynamic client's path, the
    /// same algorithm as the typed one, never a separate implementation of it. `None` for a shape with no
    /// algorithm here yet ([ALaw](Self::ALaw), [Power](Self::Power), [PiecewisePower](Self::PiecewisePower),
    /// [Exp](Self::Exp)), and for a [Table](Self::Table) code past its end (or an entry read from the wire that
    /// doesn't decode); never `None` for [MuLaw](Self::MuLaw), whose decode is total.
    #[must_use]
    pub fn decode(&self, raw: i128) -> Option<i128> {
        match self {
            CodecShape::MuLaw => {
                let byte = u8::try_from(raw).ok()?;
                Some(i128::from(mu_law_decode(byte)))
            }
            CodecShape::Table(table) => {
                let mut code = 0_i128;
                let mut hit = None;
                let whole = for_each(table, |v: i128| {
                    if code == raw {
                        hit = Some(v);
                    }
                    code = code.saturating_add(1);
                });
                hit.filter(|_| whole)
            }
            CodecShape::Offset { zero } => raw.checked_sub(*zero),
            // every other shape today (ALaw, Power, PiecewisePower, Exp), and (feature shrink_wrap) a
            // shape wire bytes named that this version doesn't know
            _ => None,
        }
    }

    /// `value` encoded by this shape, as its [Codec] would, but from the shape alone.
    ///
    /// # Errors
    ///
    /// `None` for a shape with no algorithm here yet, or (for [MuLaw](Self::MuLaw)) where `value` doesn't fit the
    /// `Linear`'s storage (`i16`); otherwise never refuses for [MuLaw](Self::MuLaw).
    #[must_use]
    pub fn encode(&self, value: i128, rounding: Rounding) -> Option<i128> {
        self.encode_where(value, rounding, |_| true)
    }

    /// [encode](Self::encode) among the codes `allowed` takes: a [Table](Self::Table) skips the others (a
    /// [CodecDef]'s spare codes), so a value rounds to the nearest code that is a value.
    fn encode_where(
        &self,
        value: i128,
        rounding: Rounding,
        allowed: impl Fn(i128) -> bool,
    ) -> Option<i128> {
        match self {
            CodecShape::MuLaw => {
                let value = i16::try_from(value).ok()?;
                Some(i128::from(mu_law_encode(value, rounding)))
            }
            CodecShape::Table(table) => table_encode(table, value, rounding, allowed),
            CodecShape::Offset { zero } => value.checked_add(*zero),
            _ => None,
        }
    }
}

/// `value` encoded by a [Table](CodecShape::Table) of entries `table`, among the codes `allowed` takes: the code of
/// `value`, or the one `rounding` picks. `None` for a table read from the wire with an entry that doesn't decode.
fn table_encode(
    table: &DefList<'_, i128>,
    value: i128,
    rounding: Rounding,
    allowed: impl Fn(i128) -> bool,
) -> Option<i128> {
    if !for_each(table, |_: i128| {}) {
        return None;
    }
    let entries = (0_i128..)
        .zip(table_values(table))
        .filter(|&(code, _)| allowed(code));
    nearest_entry(entries, value, rounding)
}

/// The entries of a table, each decoded (a caller checked that each does).
#[cfg(feature = "shrink_wrap")]
fn table_values<'a>(table: &'a DefList<'_, i128>) -> impl Iterator<Item = i128> + 'a {
    table.iter().filter_map(Result::ok)
}

/// The entries of a table.
#[cfg(not(feature = "shrink_wrap"))]
fn table_values<'a>(table: &'a DefList<'_, i128>) -> impl Iterator<Item = i128> + 'a {
    table.iter().copied()
}

/// A value given by a [Codec]'s code instead of a linear step: the bytes of `C::Raw`, nothing more (`size_of` is
/// `C::Raw`'s), and never a [spare](CodecDef::spare) code. [get](Self::get) decodes, which never refuses;
/// [try_new](Self::try_new) encodes, which refuses only a `Linear` beyond every code's range;
/// [try_from_raw](Self::try_from_raw) refuses a spare code, and [from_raw](Self::from_raw), which can't refuse,
/// compiles only for a codec without one.
///
/// `V` is `C::Linear` and `T` is `C::Raw`, both spelled out (as [Niched](crate::niche::Niched)'s `V` and `T` are) so
/// a `Coded` reads the value it holds and the bytes it is without following the trait: a G.711 μ-law byte is
/// `Coded<MuLaw, Fixed<Ratio, Pow2<-13>, i16>>`.
#[cfg_attr(
    feature = "shrink_wrap",
    derive_shrink_wrap(borrowed, owned(feature = "std"), transparent, validate = Self::validate)
)]
#[repr(transparent)]
pub struct Coded<
    C: Codec<Linear = V, Raw = T>,
    V = <C as Codec>::Linear,
    T: Storage = <C as Codec>::Raw,
> {
    /// The code, as stored and sent: never a spare one.
    raw: T,
    /// The codec, in the type only.
    _codec: PhantomData<C>,
    /// The value, in the type only (always `C::Linear`).
    _value: PhantomData<V>,
}

impl<C: Codec<Linear = V, Raw = T>, V, T: Storage> Coded<C, V, T> {
    /// `C::DEF`'s spare codes, checked against the storage: a compile error where a span is upside down or doesn't
    /// fit `T`.
    pub const SPARE: &'static [CodeSpan] = checked_spare(&C::DEF, T::MIN_INT, T::MAX_INT);

    /// `raw` as read from a register or a bus, for a codec whose every code is a value ([MuLaw]): a compile error
    /// for one with [spare](CodecDef::spare) codes, where [try_from_raw](Self::try_from_raw) is the way in.
    ///
    /// ```compile_fail
    /// # #[cfg(feature = "dimensionless")] {
    /// use exact_units::codec::{Coded, MuLawSingleZero};
    ///
    /// // 0x7F is spare here: from_raw can't refuse it, so it doesn't build
    /// let _ = Coded::<MuLawSingleZero>::from_raw(0x80);
    /// # }
    /// ```
    #[inline(always)]
    pub const fn from_raw(raw: T) -> Self {
        const {
            assert!(
                Self::SPARE.is_empty(),
                "this codec has spare codes: Coded::try_from_raw refuses them, from_raw can't"
            );
        }
        Self::from_code(raw)
    }

    /// `raw` as read from a register or a bus, if it isn't a [spare](CodecDef::spare) code.
    ///
    /// # Errors
    ///
    /// [SpareCode], holding `raw`, for a spare code.
    #[inline(always)]
    pub fn try_from_raw(raw: T) -> Result<Self, SpareCode<C, V, T>> {
        if in_spans(Self::SPARE, raw) {
            Err(SpareCode {
                raw,
                _codec: PhantomData,
            })
        } else {
            Ok(Self::from_code(raw))
        }
    }

    /// `raw`, known not to be a spare code.
    #[inline(always)]
    const fn from_code(raw: T) -> Self {
        Coded {
            raw,
            _codec: PhantomData,
            _value: PhantomData,
        }
    }

    /// `value` by the nearest code, ties broken by `rounding`.
    ///
    /// # Errors
    ///
    /// [OutOfBounds] where `value` is beyond every code's range.
    #[inline(always)]
    pub fn try_new(value: V, rounding: Rounding) -> Result<Self, OutOfBounds> {
        C::encode(value, rounding)
            .map(Self::from_code)
            .ok_or(OutOfBounds)
    }

    /// The code, as stored and sent.
    #[inline(always)]
    pub fn raw(self) -> T {
        self.raw
    }

    /// The value the code decodes to.
    #[inline(always)]
    pub fn get(self) -> V {
        // not a spare code: every constructor refused one
        C::formula(self.raw)
    }

    /// Run on every value read from the wire: a spare code is [shrink_wrap::Error::SubtypeOutOfRange], and the
    /// refusal names it as a code that means no value.
    #[cfg(feature = "shrink_wrap")]
    #[inline(always)]
    fn validate(&self) -> Result<(), Refused> {
        if in_spans(Self::SPARE, self.raw) {
            Err(Refused::new(
                crate::fixed::refused_num(self.raw),
                Rule::NoValue,
            ))
        } else {
            Ok(())
        }
    }
}

/// A code [Coded::try_from_raw] refused: one of the codec's [spare](CodecDef::spare) codes, which mean no value. It
/// holds the code, as [OutOfRange](crate::OutOfRange) holds a value; the codec is its type's, so it is as large as
/// `T`. `Display` names both, and `?` turns it into [OutOfBounds].
///
/// ```
/// # #[cfg(feature = "dimensionless")] {
/// use exact_units::codec::{Coded, MuLawSingleZero};
///
/// let e = Coded::<MuLawSingleZero>::try_from_raw(0x7F).unwrap_err();
/// assert_eq!(e.raw(), 0x7F);
/// assert_eq!(e.to_string(), "127 is a spare code of MuLawSingleZero, no value");
/// # }
/// ```
pub struct SpareCode<
    C: Codec<Linear = V, Raw = T>,
    V = <C as Codec>::Linear,
    T: Storage = <C as Codec>::Raw,
> {
    /// The code refused.
    raw: T,
    /// The codec and its value, in the type only.
    _codec: PhantomData<(C, V)>,
}

impl<C: Codec<Linear = V, Raw = T>, V, T: Storage> SpareCode<C, V, T> {
    /// The code refused.
    #[inline(always)]
    pub fn raw(&self) -> T {
        self.raw
    }
}

impl<C: Codec<Linear = V, Raw = T>, V, T: Storage> Clone for SpareCode<C, V, T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<C: Codec<Linear = V, Raw = T>, V, T: Storage> Copy for SpareCode<C, V, T> {}

/// The same code refused: the type says the rest.
impl<C: Codec<Linear = V, Raw = T>, V, T: Storage> PartialEq for SpareCode<C, V, T> {
    fn eq(&self, other: &Self) -> bool {
        self.raw == other.raw
    }
}

impl<C: Codec<Linear = V, Raw = T>, V, T: Storage + Eq> Eq for SpareCode<C, V, T> {}

impl<C: Codec<Linear = V, Raw = T>, V, T: Storage> fmt::Debug for SpareCode<C, V, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}

impl<C: Codec<Linear = V, Raw = T>, V, T: Storage> fmt::Display for SpareCode<C, V, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.raw.to_repr() {
            Repr::Int(x) => write!(f, "{x}")?,
            Repr::Float(x) => write!(f, "{x}")?,
        }
        write!(f, " is a spare code of {}, no value", C::DEF.id.name)
    }
}

impl<C: Codec<Linear = V, Raw = T>, V, T: Storage> core::error::Error for SpareCode<C, V, T> {}

#[cfg(feature = "defmt")]
impl<C: Codec<Linear = V, Raw = T>, V, T: Storage> defmt::Format for SpareCode<C, V, T> {
    fn format(&self, f: defmt::Formatter<'_>) {
        match self.raw.to_repr() {
            Repr::Int(x) => defmt::write!(f, "{=i128}", x),
            Repr::Float(x) => defmt::write!(f, "{=f64}", x),
        }
        defmt::write!(f, " is a spare code of {=str}, no value", C::DEF.id.name);
    }
}

impl<C: Codec<Linear = V, Raw = T>, V, T: Storage> From<SpareCode<C, V, T>> for OutOfBounds {
    #[inline(always)]
    fn from(_: SpareCode<C, V, T>) -> Self {
        OutOfBounds
    }
}

impl<C: Codec<Linear = V, Raw = T>, V, T: Storage> Clone for Coded<C, V, T> {
    #[inline(always)]
    fn clone(&self) -> Self {
        *self
    }
}

impl<C: Codec<Linear = V, Raw = T>, V, T: Storage> Copy for Coded<C, V, T> {}

impl<C: Codec<Linear = V, Raw = T>, V, T: Storage> PartialEq for Coded<C, V, T> {
    #[inline(always)]
    fn eq(&self, other: &Self) -> bool {
        self.raw == other.raw
    }
}

impl<C: Codec<Linear = V, Raw = T>, V, T: Storage + Eq> Eq for Coded<C, V, T> {}

impl<C: Codec<Linear = V, Raw = T>, V, T: Storage + Hash> Hash for Coded<C, V, T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.raw.hash(state);
    }
}

/// The decoded value, as it debugs.
impl<C: Codec<Linear = V, Raw = T>, V: fmt::Debug, T: Storage> fmt::Debug for Coded<C, V, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.get().fmt(f)
    }
}

impl<C: Codec<Linear = V, Raw = T>, V, T: Storage> niche::sealed::Sealed for Coded<C, V, T> {}

/// `Coded` composes with [Niched](niche::Niched), as [Fixed] and [Point](crate::Point) do: the codec's
/// [spare](CodecDef::spare) codes are the holes in `Coded`'s range ([RawValue::HOLES]) where a niche code may sit,
/// and every other code of `C::Raw` is a value. So `Niched<Coded<C, V, T>, N>` builds where each of `N`'s codes is
/// a spare code of `C`, and is a compile error where one is a value. [MuLawSingleZero] leaves `0x7F` spare, so
/// `Niched<Coded<MuLawSingleZero>, Null<0x7F>>` is an `Option` of a μ-law sample in one byte:
///
/// ```
/// # #[cfg(feature = "dimensionless")] {
/// use exact_units::codec::{Coded, MuLawSingleZero};
/// use exact_units::niche::{Niched, Null};
///
/// type Sample = Coded<MuLawSingleZero>;
/// type MaybeSample = Niched<Sample, Null<0x7F>>;
///
/// let silence = MaybeSample::try_from_raw(0x7F).map(MaybeSample::value);
/// assert_eq!(silence, Ok(None));
/// let loudest = MaybeSample::try_from_raw(0x80).map(|s| s.value().map(|c| c.get().get()));
/// assert_eq!(loudest, Ok(Some(8031)));
/// assert_eq!(MaybeSample::from(None::<Sample>).raw(), 0x7F);
/// assert_eq!(size_of::<MaybeSample>(), 1);
/// # }
/// ```
///
/// [MuLaw] is G.711 as written, where `0x7F` is a value (the code of a negative input between 0 and −1): no code is
/// spare, and a niche on it doesn't build.
///
/// ```compile_fail
/// # #[cfg(feature = "dimensionless")] {
/// use exact_units::codec::{Coded, MuLaw};
/// use exact_units::niche::{Niched, Null};
///
/// // 0x7F is G.711's negative zero, a value: nothing is left for a niche
/// let _ = Niched::<Coded<MuLaw>, Null<0x7F>>::try_from_raw(3);
/// # }
/// ```
impl<C: Codec<Linear = V, Raw = T>, V, T: Storage> RawValue for Coded<C, V, T> {
    type Raw = T;

    const RANGE: Option<(i128, i128)> = niche::int_range::<T, crate::fixed::Unbounded>();

    const HOLES: &'static [CodeSpan] = Self::SPARE;

    #[inline(always)]
    fn raw(self) -> T {
        self.raw
    }

    #[inline(always)]
    fn try_from_raw(raw: T) -> Result<Self, OutOfBounds> {
        Ok(Self::try_from_raw(raw)?)
    }

    #[inline(always)]
    fn from_raw_unchecked(raw: T, _: niche::sealed::Token) -> Self {
        Self::from_code(raw)
    }
}

/// ITU-T G.711 μ-law as the standard writes it: 8 segments of 16 steps each, the step doubling every segment (2, 4,
/// 8, .., 256), encoded in one byte. [decode](Codec::decode) is total and onto but not one-to-one: `0xFF` and `0x7F`
/// both mean 0, the standard's "double zero" (G.711, Tables 2a and 2b: `0x7F` is the character of every negative
/// input between decision values 0 and −1). And two adjacent segments leave a gap a few steps wide at the boundary
/// (segment 0's 16 steps of 2 reach 30, segment 1's 16 steps of 4 start at 33): both are the standard's own shape,
/// not a bug here.
///
/// `0x7F` is a value here, not a spare code: real streams carry it. ITU-T G.191's reference encoder
/// (`ulaw_compress`, one's complement on negative samples) gives it for the 16-bit inputs −1..=−4, and an
/// analogue codec for any slightly negative silence. A stream whose encoder never sends it, and that wants `0x7F`
/// for a niche, says so with [MuLawSingleZero]. [encode](Codec::encode) here never gives `0x7F` either: zero is
/// always `0xFF`.
///
/// [Linear](Codec::Linear) is `Fixed<Ratio, Pow2<-13>, i16>`, the 14-bit linear sample (13 magnitude bits and a
/// sign, the standard's native domain): the widely used 16-bit variant (BIAS 0x84, as telephony software converts
/// 16-bit PCM) is this value times 4, left for a client that wants it, not duplicated here.
#[cfg(feature = "dimensionless")]
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct MuLaw;

/// G.711 μ-law with one zero, `0xFF`: the negative zero `0x7F` is [spare](CodecDef::spare), free for a niche
/// (`Niched<Coded<MuLawSingleZero>, Null<0x7F>>`, an `Option` in one byte). Every other code decodes and encodes as
/// [MuLaw]'s, by the same [CodecShape::MuLaw].
///
/// A convention, so an opt-in (AGENTS.md's rule 5): right only for a stream whose encoder never sends `0x7F`, such
/// as Sun's widely copied `g711.c` (`linear2ulaw` folds every negative sample to a magnitude of at least 1) and this
/// crate's own [encode](Codec::encode). A G.711 stream from G.191's reference encoder or from an analogue codec does
/// carry `0x7F` as a value: read with this codec, such a sample is refused ([SpareCode]) or, in a `Niched`, taken
/// for the niche. Read those with [MuLaw].
#[cfg(feature = "dimensionless")]
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct MuLawSingleZero;

/// The bias a μ-law byte's magnitude is computed against, in the 14-bit domain: `(2·15 + 33) << 7 - 33 = 8031` is
/// the largest magnitude a byte decodes to ([CLIP]). Plain integer arithmetic, so unlike [MuLaw] itself this and
/// the functions below need no unit feature: [CodecShape::decode] and [CodecShape::encode] call them too, for a
/// dynamic client that runs the algorithm from [CodecDef] alone.
const BIAS: i32 = 33;

/// The largest magnitude a μ-law byte decodes to, and the clip [Codec::encode] applies before it computes a code.
const CLIP: i32 = 8031;

/// `byte` decoded by [MuLaw], exactly: the sign, 3-bit segment and 4-bit mantissa it carries (one's complemented on
/// the wire) give the 14-bit magnitude `((2·mantissa + 33) << segment) - 33`.
#[expect(
    clippy::as_conversions,
    reason = "const fn: From isn't const on stable; the final cast narrows a magnitude proven to be -8031..=8031"
)]
const fn mu_law_decode(byte: u8) -> i16 {
    let u = !byte;
    let sign = u & 0x80 != 0;
    let exponent = ((u >> 4) & 0x07) as u32;
    let mantissa = (u & 0x0F) as i32;
    #[expect(
        clippy::arithmetic_side_effects,
        reason = "mantissa is 0..=15, so 2*mantissa+33 is 33..=63; shifted left by at most 7 it is at most 8064, \
                  fitting i32 by a wide margin"
    )]
    let t = (2 * mantissa + BIAS) << exponent;
    #[expect(
        clippy::arithmetic_side_effects,
        reason = "t is (2*mantissa+33) << exponent with mantissa, exponent >= 0, so t >= 33 = BIAS: never underflows"
    )]
    let magnitude = t - BIAS;
    #[expect(
        clippy::arithmetic_side_effects,
        reason = "magnitude is 0..=8064-33=8031, far from i32::MIN: negating it never overflows"
    )]
    let signed = if sign { -magnitude } else { magnitude };
    #[expect(
        clippy::cast_possible_truncation,
        reason = "signed is -8031..=8031, well within i16's range"
    )]
    let narrowed = signed as i16;
    narrowed
}

/// `value` (a signed 14-bit magnitude) encoded as a μ-law byte, rounded to the nearest code by `r`: the inverse of
/// [mu_law_decode], clipped first to [CLIP]. Always some byte, never the negative zero `0x7F` (a value that rounds
/// to 0 is `0xFF`): encoding never refuses in this codec.
#[expect(
    clippy::as_conversions,
    reason = "const fn: From isn't const on stable; each narrowing cast is proven in its own `expect` below"
)]
const fn mu_law_encode(value: i16, r: Rounding) -> u8 {
    let x = value as i32;
    let sign = x < 0;
    #[expect(
        clippy::arithmetic_side_effects,
        reason = "x is an i16 widened to i32, so -32768..=32767: negating it never overflows i32"
    )]
    let mag = if sign { -x } else { x };
    let mag = if mag > CLIP { CLIP } else { mag };
    #[expect(
        clippy::arithmetic_side_effects,
        reason = "mag is 0..=CLIP (8031); mag + BIAS (33) is at most 8064, far from i32::MAX"
    )]
    let t = (mag + BIAS) as i128;
    // the largest segment (0..=7) whose minimum biased value (BIAS << e) is at most t; t >= BIAS = BIAS << 0, so
    // the loop always finds at least segment 0
    let mut e: u32 = 7;
    while e > 0 && t < ((BIAS as i128) << e) {
        #[expect(
            clippy::arithmetic_side_effects,
            reason = "the loop guard e > 0 keeps this from underflowing"
        )]
        {
            e -= 1;
        }
    }
    let base = (BIAS as i128) << e;
    let step = 2i128 << e;
    #[expect(
        clippy::arithmetic_side_effects,
        reason = "base = BIAS << e <= t by the loop above (e is the largest with BIAS << e <= t): never underflows"
    )]
    let diff = t - base;
    let m = crate::fixed::round_div_i128(diff, step, r);
    // rounding can push m past the segment's 15: t fell in the gap between this segment's top (base + 15*step)
    // and the next segment's bottom (BIAS << (e+1)), which (the standard's own shape) isn't base + 16*step. Only
    // for e < 7, since CLIP keeps segment 7's t at or below its own top.
    let (e, m) = if m >= 16 && e < 7 {
        #[expect(
            clippy::arithmetic_side_effects,
            reason = "base = BIAS << e, step = 2 << e: base + 15*step = 63 << e, at most 63 << 6, far from i128::MAX"
        )]
        let low_t = base + 15 * step;
        #[expect(
            clippy::arithmetic_side_effects,
            reason = "e < 7 here, so e + 1 is at most 7 and BIAS << (e + 1) at most BIAS << 7"
        )]
        let high_t = (BIAS as i128) << (e + 1);
        #[expect(
            clippy::arithmetic_side_effects,
            reason = "low_t <= t < high_t in the gap (m rounded up from a quotient below 16.5): neither subtraction \
                      underflows"
        )]
        if t - low_t <= high_t - t {
            (e, 15)
        } else {
            (e + 1, 0)
        }
    } else {
        (e, m)
    };
    let sign_bit: u8 = if sign { 0x80 } else { 0 };
    #[expect(
        clippy::cast_possible_truncation,
        reason = "e is 0..=7, fitting 3 bits, before the <<4"
    )]
    let exponent_bits = (e as u8) << 4;
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "m is 0..=15, fitting 4 bits and never negative"
    )]
    let mantissa_bits = m as u8;
    let byte = !(sign_bit | exponent_bits | mantissa_bits);
    // a negative value that rounds to 0 is still 0: the one zero, 0xFF, never the negative zero 0x7F
    if byte == 0x7F { 0xFF } else { byte }
}

#[cfg(feature = "dimensionless")]
impl Codec for MuLaw {
    type Raw = u8;
    type Linear = Fixed<crate::units::Ratio, Pow2<-13>, i16>;

    const DEF: CodecDef<'static> = CodecDef {
        id: MarkerId {
            crate_name: "exact_units",
            name: "MuLaw",
        },
        shape: CodecShape::MuLaw,
        spare: def_list(&[]),
    };

    #[inline]
    fn formula(raw: u8) -> Self::Linear {
        Fixed::new(mu_law_decode(raw))
    }

    #[inline]
    fn encode(value: Self::Linear, rounding: Rounding) -> Option<u8> {
        Some(mu_law_encode(value.get(), rounding))
    }
}

#[cfg(feature = "dimensionless")]
impl Codec for MuLawSingleZero {
    type Raw = u8;
    type Linear = Fixed<crate::units::Ratio, Pow2<-13>, i16>;

    const DEF: CodecDef<'static> = CodecDef {
        id: MarkerId {
            crate_name: "exact_units",
            name: "MuLawSingleZero",
        },
        shape: CodecShape::MuLaw,
        spare: def_list(&[CodeSpan::one(0x7F)]),
    };

    #[inline]
    fn formula(raw: u8) -> Self::Linear {
        Fixed::new(mu_law_decode(raw))
    }

    #[inline]
    fn encode(value: Self::Linear, rounding: Rounding) -> Option<u8> {
        Some(mu_law_encode(value.get(), rounding))
    }
}

/// The data lengths of a CAN FD frame, in bytes: 0..=8, 12, 16, 20, 24, 32, 48 and 64, the lengths its 4-bit
/// [data length code](CanFdDlc) can say (ISO 11898-1:2015; checked against ST RM0468 Rev 3, §61.4.1, Table 503). A
/// [ValueSet] for
/// [OneOf]: `Fixed<Byte, One, u8, OneOf<CanFdLengths>>` is a payload length that a frame can carry,
/// and nothing else.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct CanFdLengths;

impl ValueSet for CanFdLengths {
    const VALUES: &'static [i64] = &[0, 1, 2, 3, 4, 5, 6, 7, 8, 12, 16, 20, 24, 32, 48, 64];
}

/// The data length code (DLC) of a CAN FD frame: 4 bits that say how many data bytes follow, 0..=8 as themselves and
/// 9..=15 as 12, 16, 20, 24, 32, 48 and 64 bytes. ISO 11898-1:2015 defines the coding; the table is checked against
/// ST's RM0468 Rev 3 (STM32H72x/73x reference manual), §61.4.1, Table 503 "DLC coding in FDCAN", p. 2510, and Linux's
/// `can_fd_dlc2len` (`drivers/net/can/dev/length.c`) has the same. The first [CodecShape::Table] codec: its [DEF](Codec::DEF)
/// carries the table, and the typed path encodes through the same table and the same rounding as a client that
/// has only the data.
///
/// - [Raw](Codec::Raw) is `u8`, the field as a controller's register or a byte of a frame holds it: codes
///   16..=255 are [spare](CodecDef::spare), no DLC, refused by [Coded::try_from_raw] and free for a niche.
/// - [Linear](Codec::Linear) is `Fixed<Byte, One, u8, OneOf<CanFdLengths>>`: every value of it has its code, so
///   [encode](Codec::encode) is exact whatever [Rounding] it is given, and a length between two of the set
///   (13 bytes) is no `Linear` at all: [try_new](crate::Fixed::try_new) refuses it, and
///   [try_new_rounded](crate::Fixed::try_new_rounded) with [Rounding::Ceil] names the step up to 16, the frame
///   the 13 bytes are sent in (padded).
///
/// Only for an FD frame: a classic CAN frame reads the codes 9..=15 as 8 bytes, a different codec.
///
/// ```
/// use exact_units::codec::{CanFdDlc, CanFdLengths, Coded};
/// use exact_units::{Fixed, OneOf, Rounding, prefix::One, units::Byte};
///
/// type Len = Fixed<Byte, One, u8, OneOf<CanFdLengths>>;
///
/// // DLC 9 is 12 bytes, exactly
/// let dlc = Coded::<CanFdDlc>::try_from_raw(9).unwrap();
/// assert_eq!(dlc.get().get(), 12);
///
/// // 13 bytes is no FD length: refused, or rounded up by a rounding the code names
/// assert_eq!(Len::try_new(13).unwrap_err().to_string(), "13 B is out of {0..=8, 12, 16, 20, 24, 32, 48, 64} B (Byte, u8)");
/// let len = Len::try_new_rounded(13, Rounding::Ceil).unwrap();
/// assert_eq!(len.get(), 16);
/// assert_eq!(Coded::<CanFdDlc>::try_new(len, Rounding::Ceil).map(Coded::raw), Ok(10));
///
/// // 16 and above is no DLC
/// assert!(Coded::<CanFdDlc>::try_from_raw(16).is_err());
/// ```
#[cfg(feature = "information")]
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct CanFdDlc;

/// [CanFdDlc]'s table: the length of each code 0..=15, in bytes.
const CAN_FD_DLC: [u8; 16] = [0, 1, 2, 3, 4, 5, 6, 7, 8, 12, 16, 20, 24, 32, 48, 64];

/// [CAN_FD_DLC] as a [CodecShape::Table]'s entries, widened in `const` code: one table, two types.
#[cfg_attr(
    not(feature = "information"),
    expect(dead_code, reason = "CanFdDlc needs `information`")
)]
#[expect(
    clippy::as_conversions,
    clippy::indexing_slicing,
    reason = "a const item: From is not const on stable, a u8 widens into an i128 exactly; i < 16, both arrays' length"
)]
const CAN_FD_DLC_TABLE: [i128; 16] = {
    let mut t = [0_i128; 16];
    let mut i = 0;
    while i < 16 {
        t[i] = CAN_FD_DLC[i] as i128;
        i += 1;
    }
    t
};

#[cfg(feature = "information")]
impl Codec for CanFdDlc {
    type Raw = u8;
    type Linear = Fixed<crate::units::Byte, One, u8, OneOf<CanFdLengths>>;

    const DEF: CodecDef<'static> = CodecDef {
        id: MarkerId {
            crate_name: "exact_units",
            name: "CanFdDlc",
        },
        shape: CodecShape::Table(def_list(&CAN_FD_DLC_TABLE)),
        spare: def_list(&[CodeSpan { low: 16, high: 255 }]),
    };

    /// The length of the low 4 bits of `raw`, as a controller reads the field: a spare code (16 and above) means
    /// nothing, [decode](Codec::decode) refuses it.
    #[inline]
    fn formula(raw: u8) -> Self::Linear {
        let len = CAN_FD_DLC
            .get(usize::from(raw & 0x0F))
            .copied()
            .unwrap_or_default();
        // every entry of the table is one of CanFdLengths: the tests walk them
        Fixed::new_unchecked(len)
    }

    #[inline]
    fn encode(value: Self::Linear, rounding: Rounding) -> Option<u8> {
        let code = Self::DEF.encode(i128::from(value.get()), rounding)?;
        u8::try_from(code).ok()
    }
}

/// How a [Curve] computes: never a float standing in where an exact or tabled relationship holds (AGENTS.md
/// "Every lossy step is named").
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum CurveKind {
    /// A closed-form relationship, exact in both directions (mpg to L/100 km, an exact reciprocal).
    Exact,
    /// A table of named points, not a formula (AWG wire gauge, the Beaufort scale).
    Tabled,
    /// A floating-point approximation (Richter magnitude from amplitude): named so a client can always tell this
    /// curve apart from an [Exact](CurveKind::Exact) or [Tabled](CurveKind::Tabled) one.
    Float,
}

/// A named, non-linear conversion between two quantities: AWG, mpg ↔ L/100 km, Beaufort, Richter, shoe sizes
/// (LVL-4), marked by [CurveKind] so a client never has to guess which it is. Declared here, beside [Codec], as
/// the foundation; LVL-4 gives it a conversion (`convert` or similar, not fixed yet) and implements the first
/// curves. [Pq](video::Pq), the PQ transfer curve of HDR video, is the first, [CurveKind::Float].
pub trait Curve: Copy + 'static {
    /// Whether this curve is exact, tabled or a float approximation.
    const KIND: CurveKind;
}

#[cfg(test)]
mod tests;

#[cfg(feature = "dimensionless")]
pub mod video;
