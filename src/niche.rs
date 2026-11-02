//! Niche codes next to [Bounds](crate::Bounds): raw values a protocol reserves for a meaning other than a number
//! (not available, error, not implemented, null), and rail marks for a value clipped at an end of its range.
//!
//! - [Niched]`<V, N>`: a value `V` ([Fixed](crate::Fixed) or [Point](crate::Point)) or one of the codes of `N`, in
//!   memory and on the wire the raw storage alone (`size_of` is the storage's). [get](Niched::get) answers a
//!   [Reading]: the value, or the meaning of the code, never the code as a value.
//! - [Niches]: the codes and their meanings, declared with [niches!](crate::niches): a single code or a code range
//!   each. [Null]`<CODE>` is the one-code set, [Niched] of it converts to and from an `Option`.
//! - [Clipped]`<V, R>`: a value whose ends are rails (an ADC at full scale, a Q15 after saturation): `get` marks a
//!   value at a rail [AtLeast](Clip::AtLeast) or [AtMost](Clip::AtMost). A rail is a value, a niche is not.
//!
//! The codes and the bounds are checked at compile time where the type is used: every code fits the storage, no code
//! is inside the bounds, no two codes overlap. Encoding a decoded reading gives back the same raw value, so the bytes
//! of a value read and written again don't change.
//!
//! ```
//! use exact_units::{Fixed, Range, niche::{J1939U8, Niched, Reading}, prefix::One, units::Ratio};
//!
//! // a J1939 1-byte parameter: 0..=250 a value, 251..=253 reserved, 0xFE error, 0xFF not available
//! type Percent = Fixed<Ratio, One, u8, Range<0, 250>>;
//! type Load = Niched<Percent, J1939U8>;
//!
//! assert_eq!(Load::try_from_raw(80).map(Load::get), Ok(Reading::Value(Percent::bounded(80))));
//! assert_eq!(Load::try_from_raw(0xFE).map(Load::get), Ok(Reading::Niche(J1939U8::Error)));
//! assert_eq!(Load::try_from_raw(252).map(Load::get), Ok(Reading::Niche(J1939U8::Reserved(252))));
//! assert_eq!(Load::niche(J1939U8::NotAvailable).map(Load::raw), Ok(0xFF));
//! assert_eq!(size_of::<Load>(), 1);
//! ```

use crate::fixed::{Bounds, Fixed, OutOfBounds, Repr, Storage};
use crate::point::{Origin, Point};
use core::fmt;
use core::hash::{Hash, Hasher};
use core::marker::PhantomData;
#[cfg(feature = "shrink_wrap")]
use shrink_wrap::prelude::*;

/// The niche codes of a storage `T` and their meanings: implemented by the meaning type itself, best declared with
/// [niches!](crate::niches), which writes [CODES](Self::CODES), [decode](Self::decode) and [encode](Self::encode) from
/// one list so they can't disagree.
///
/// A hand-written implementation keeps three rules: `decode` answers `Some` exactly for the codes in `CODES`,
/// `encode(decode(c)) == Some(c)` for each of them, and `encode` answers `None` for a meaning whose code is not in
/// `CODES`.
pub trait Niches<T: Storage>: Copy + Eq + fmt::Debug + 'static {
    /// The codes, each an inclusive range `(low, high)` as integers; a single code is `(c, c)`.
    const CODES: &'static [(i128, i128)];

    /// The meaning of `raw`, `None` when it is not a code.
    fn decode(raw: T) -> Option<Self>;

    /// The code of the meaning, `None` when it holds a code outside its range (`Reserved(5)`).
    fn encode(self) -> Option<T>;
}

/// The one-code set: `CODE` means null, no value. `Niched<V, Null<CODE>>` converts to and from `Option<V>`; Matter's
/// nullable integers are `Null<0xFF>` on an `u8` and `Null<-128>` on an `i8`.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct Null<const CODE: i128>;

impl<T: Storage, const CODE: i128> Niches<T> for Null<CODE> {
    const CODES: &'static [(i128, i128)] = &[(CODE, CODE)];

    #[inline(always)]
    fn decode(raw: T) -> Option<Self> {
        match raw.to_repr() {
            Repr::Int(x) if x == CODE => Some(Null),
            Repr::Int(_) | Repr::Float(_) => None,
        }
    }

    #[inline(always)]
    fn encode(self) -> Option<T> {
        T::from_int(CODE)
    }
}

/// Declares an enum of niche meanings for one storage and its [Niches] implementation: a variant is a single code
/// (`Error = 0xFE`, a unit variant) or a code range (`Reserved = 251..=253`, a variant holding the raw code, so the
/// code read is the code written back). Codes are integer literals without a suffix; one that doesn't fit the storage
/// doesn't compile.
///
/// ```
/// use exact_units::{Fixed, Range, niches, niche::{Niched, Reading}, prefix::One, units::Ratio};
///
/// niches! {
///     /// SunSpec's `int16` points: 0x8000 is a point the device doesn't implement.
///     pub enum SunSpecI16: i16 {
///         /// Not implemented by this device.
///         NotImplemented = -32768,
///     }
/// }
///
/// type PowerFactor = Niched<Fixed<Ratio, One, i16, Range<-32767, 32767>>, SunSpecI16>;
/// assert_eq!(PowerFactor::try_from_raw(i16::MIN).map(PowerFactor::get), Ok(Reading::Niche(SunSpecI16::NotImplemented)));
/// ```
///
/// A code that doesn't fit the storage doesn't build:
///
/// ```compile_fail
/// exact_units::niches! {
///     /// 0x100 is not a u8
///     pub enum Wide: u8 { Missing = 0x100 }
/// }
/// ```
#[macro_export]
macro_rules! niches {
    (
        $(#[$meta:meta])*
        $vis:vis enum $name:ident: $t:ty { $($body:tt)* }
    ) => {
        $crate::niches!(@munch [$(#[$meta])* $vis $name $t] [] [] [] []; $($body)*);
    };
    // a code range: a variant holding the raw code
    (@munch $head:tt [$($var:tt)*] [$($dec:tt)*] [$($enc:tt)*] [$($codes:tt)*];
        $(#[$vmeta:meta])* $v:ident = $lo:literal ..= $hi:literal $(, $($rest:tt)*)?
    ) => {
        $crate::niches!(@munch $head
            [$($var)* $(#[$vmeta])* $v(@t),]
            [$($dec)* c @ $lo..=$hi => ::core::option::Option::Some(Self::$v(c)),]
            [$($enc)* Self::$v(raw) => match raw {
                $lo..=$hi => ::core::option::Option::Some(raw),
                _ => ::core::option::Option::None,
            },]
            [$($codes)* ($lo, $hi),];
            $($($rest)*)?
        );
    };
    // a single code: a unit variant
    (@munch $head:tt [$($var:tt)*] [$($dec:tt)*] [$($enc:tt)*] [$($codes:tt)*];
        $(#[$vmeta:meta])* $v:ident = $code:literal $(, $($rest:tt)*)?
    ) => {
        $crate::niches!(@munch $head
            [$($var)* $(#[$vmeta])* $v,]
            [$($dec)* $code => ::core::option::Option::Some(Self::$v),]
            [$($enc)* Self::$v => ::core::option::Option::Some($code),]
            [$($codes)* ($code, $code),];
            $($($rest)*)?
        );
    };
    (@munch [$(#[$meta:meta])* $vis:vis $name:ident $t:ty] [$($var:tt)*] [$($dec:tt)*] [$($enc:tt)*]
        [$($codes:tt)*];
    ) => {
        $crate::niches!(@variants [$(#[$meta])* $vis $name $t] [] $($var)*);

        impl $crate::niche::Niches<$t> for $name {
            const CODES: &'static [(i128, i128)] = &[$($codes)*];

            #[inline]
            fn decode(raw: $t) -> ::core::option::Option<Self> {
                match raw {
                    $($dec)*
                    _ => ::core::option::Option::None,
                }
            }

            #[inline]
            fn encode(self) -> ::core::option::Option<$t> {
                match self {
                    $($enc)*
                }
            }
        }
    };
    // the variants with `@t` replaced by the storage
    (@variants $head:tt [$($done:tt)*] $(#[$vmeta:meta])* $v:ident(@t), $($rest:tt)*) => {
        $crate::niches!(@variants $head [$($done)* $(#[$vmeta])* $v(STORAGE),] $($rest)*);
    };
    (@variants $head:tt [$($done:tt)*] $(#[$vmeta:meta])* $v:ident, $($rest:tt)*) => {
        $crate::niches!(@variants $head [$($done)* $(#[$vmeta])* $v,] $($rest)*);
    };
    (@variants [$(#[$meta:meta])* $vis:vis $name:ident $t:ty] [$($done:tt)*]) => {
        $crate::niches!(@emit [$(#[$meta])* $vis $name $t] $($done)*);
    };
    (@emit [$(#[$meta:meta])* $vis:vis $name:ident $t:ty] $($variants:tt)*) => {
        $crate::__niches_enum!($t; $(#[$meta])* $vis $name; $($variants)*);
    };
}

/// The enum of [niches!], with each `STORAGE` replaced by the storage type.
#[doc(hidden)]
#[macro_export]
macro_rules! __niches_enum {
    ($t:ty; $(#[$meta:meta])* $vis:vis $name:ident; $($(#[$vmeta:meta])* $v:ident $(($s:ident))?,)*) => {
        $(#[$meta])*
        #[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
        $vis enum $name {
            $($(#[$vmeta])* $v $(($crate::__niches_storage!($s $t)))?,)*
        }
    };
}

/// `STORAGE` → the storage type.
#[doc(hidden)]
#[macro_export]
macro_rules! __niches_storage {
    (STORAGE $t:ty) => {
        $t
    };
}

crate::niches! {
    /// SAE J1939's 1-byte parameter ranges: 0..=250 (0x00..=0xFA) are values, 251..=253 are reserved, 0xFE is an
    /// error indicator and 0xFF not available. Give the value `Range<0, 250>`.
    pub enum J1939U8: u8 {
        /// 0xFB..=0xFD: reserved (0xFB for parameter-specific indicators), with the code read.
        Reserved = 251..=253,
        /// 0xFE: the sender has the parameter but can't measure it now (a sensor fault).
        Error = 0xFE,
        /// 0xFF: the sender doesn't provide the parameter.
        NotAvailable = 0xFF,
    }
}

crate::niches! {
    /// SAE J1939's 2-byte parameter ranges: 0x0000..=0xFAFF are values, 0xFB00..=0xFDFF reserved, 0xFE00..=0xFEFF
    /// error indicators and 0xFF00..=0xFFFF not available. Give the value `Range<0, 0xFAFF>`.
    pub enum J1939U16: u16 {
        /// 0xFB00..=0xFDFF: reserved (0xFBxx for parameter-specific indicators), with the code read.
        Reserved = 0xFB00..=0xFDFF,
        /// 0xFE00..=0xFEFF: an error indicator, with the code read.
        Error = 0xFE00..=0xFEFF,
        /// 0xFF00..=0xFFFF: not available, with the code read (senders use 0xFFFF).
        NotAvailable = 0xFF00..=0xFFFF,
    }
}

crate::niches! {
    /// SAE J1939's 4-byte parameter ranges: 0..=0xFAFF_FFFF are values, 0xFB00_0000..=0xFDFF_FFFF reserved,
    /// 0xFE00_0000..=0xFEFF_FFFF error indicators and 0xFF00_0000..=0xFFFF_FFFF not available. Give the value
    /// `Range<0, 0xFAFF_FFFF>`.
    pub enum J1939U32: u32 {
        /// 0xFB00_0000..=0xFDFF_FFFF: reserved, with the code read.
        Reserved = 0xFB00_0000..=0xFDFF_FFFF,
        /// 0xFE00_0000..=0xFEFF_FFFF: an error indicator, with the code read.
        Error = 0xFE00_0000..=0xFEFF_FFFF,
        /// 0xFF00_0000..=0xFFFF_FFFF: not available, with the code read.
        NotAvailable = 0xFF00_0000..=0xFFFF_FFFF,
    }
}

/// The values a [Niched] or a [Clipped] can wrap: a number with a raw storage and bounds, [Fixed] and
/// [Point]`<O, Fixed<..>>`. Sealed.
pub trait RawValue: Copy + sealed::Sealed {
    /// The storage, what is in memory and on the wire.
    type Raw: Storage;

    /// The values allowed, as integers: the bounds' `MIN ..= MAX` ([Bounds::INT_RANGE]), the whole storage without
    /// them, `None` for a float storage.
    const RANGE: Option<(i128, i128)>;

    /// The number as stored.
    fn raw(self) -> Self::Raw;

    /// The value of `raw`, if it is within the bounds.
    ///
    /// # Errors
    ///
    /// [OutOfBounds] outside them.
    fn try_from_raw(raw: Self::Raw) -> Result<Self, OutOfBounds>;

    /// The value of `raw` without the bounds check, for a raw value checked some other way. The token can only be
    /// made in this crate.
    #[doc(hidden)]
    fn from_raw_unchecked(raw: Self::Raw, token: sealed::Token) -> Self;
}

mod sealed {
    /// Only this crate implements [RawValue](super::RawValue).
    pub trait Sealed {}

    /// Proof that the caller is this crate: no constructor outside it.
    #[derive(Copy, Clone, Debug)]
    pub struct Token(pub(super) ());
}

/// The integer range of a storage `T` with bounds `B`, `None` for a float.
const fn int_range<T: Storage, B: Bounds<T>>() -> Option<(i128, i128)> {
    if T::IS_FLOAT {
        None
    } else {
        match B::INT_RANGE {
            Some(r) => Some(r),
            None => Some((T::MIN_INT, T::MAX_INT)),
        }
    }
}

impl<U, P, T: Storage, B: Bounds<T>> sealed::Sealed for Fixed<U, P, T, B> {}

impl<U, P, T: Storage, B: Bounds<T>> RawValue for Fixed<U, P, T, B> {
    type Raw = T;

    const RANGE: Option<(i128, i128)> = int_range::<T, B>();

    #[inline(always)]
    fn raw(self) -> T {
        self.get()
    }

    #[inline(always)]
    fn try_from_raw(raw: T) -> Result<Self, OutOfBounds> {
        Fixed::try_new(raw)
    }

    #[inline(always)]
    fn from_raw_unchecked(raw: T, _: sealed::Token) -> Self {
        Fixed::new_unchecked(raw)
    }
}

impl<O: Origin<Unit = U>, U, P, T: Storage, B: Bounds<T>> sealed::Sealed
    for Point<O, Fixed<U, P, T, B>>
{
}

impl<O: Origin<Unit = U>, U, P, T: Storage, B: Bounds<T>> RawValue for Point<O, Fixed<U, P, T, B>> {
    type Raw = T;

    const RANGE: Option<(i128, i128)> = int_range::<T, B>();

    #[inline(always)]
    fn raw(self) -> T {
        Point::raw(self)
    }

    #[inline(always)]
    fn try_from_raw(raw: T) -> Result<Self, OutOfBounds> {
        Point::try_from_raw(raw)
    }

    #[inline(always)]
    fn from_raw_unchecked(raw: T, _: sealed::Token) -> Self {
        Point::new(Fixed::new_unchecked(raw))
    }
}

/// Panics, a compile error where it is evaluated, unless the codes fit the storage, stay out of the bounds and don't
/// overlap: what [Niched] proves before its first use.
const fn check_codes(
    range: Option<(i128, i128)>,
    t_min: i128,
    t_max: i128,
    codes: &[(i128, i128)],
) {
    assert!(range.is_some(), "niche codes need an integer storage");
    let (min, max) = match range {
        Some(r) => r,
        None => (0, -1),
    };
    assert!(!codes.is_empty(), "a niche set without codes");
    let mut rest = codes;
    while let [(lo, hi), tail @ ..] = rest {
        assert!(
            *lo <= *hi,
            "a niche range's low code is above its high code"
        );
        assert!(
            *lo >= t_min && *hi <= t_max,
            "a niche code doesn't fit the storage"
        );
        assert!(
            *hi < min || *lo > max,
            "a niche code is inside the bounds: give the value a Range that leaves the codes out"
        );
        let mut later = tail;
        while let [(lo2, hi2), tail2 @ ..] = later {
            assert!(*hi < *lo2 || *lo > *hi2, "two niche codes overlap");
            later = tail2;
        }
        rest = tail;
    }
}

/// What a [Niched] holds: a value, or the meaning of a niche code.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum Reading<V, N> {
    /// A value within the bounds.
    Value(V),
    /// A niche code, by its meaning.
    Niche(N),
}

impl<V, N> Reading<V, N> {
    /// The value, `None` for a niche.
    #[inline(always)]
    pub fn value(self) -> Option<V> {
        match self {
            Reading::Value(v) => Some(v),
            Reading::Niche(_) => None,
        }
    }

    /// The niche's meaning, `None` for a value.
    #[inline(always)]
    pub fn niche(self) -> Option<N> {
        match self {
            Reading::Value(_) => None,
            Reading::Niche(n) => Some(n),
        }
    }
}

/// A value `V` or one of the niche codes of `N`, stored as the raw number: `size_of` and the bytes are the storage's.
/// [get](Self::get) answers the value or the code's meaning ([Reading]), so a code never passes for a number.
///
/// The bounds of `V` must leave the codes out, and every code must fit the storage: checked at compile time.
///
/// ```compile_fail
/// use exact_units::{Fixed, Range, niche::{J1939U8, Niched}, prefix::One, units::Ratio};
///
/// // 0xFE is an error code and inside 0..=254
/// type Bad = Niched<Fixed<Ratio, One, u8, Range<0, 254>>, J1939U8>;
/// let _ = Bad::try_from_raw(3);
/// ```
///
/// ```compile_fail
/// use exact_units::{Fixed, Range, niches, niche::Niched, prefix::One, units::Ratio};
///
/// niches! {
///     /// 3 is in both
///     pub enum Overlap: u8 { Low = 1..=3, High = 3 }
/// }
/// let _ = Niched::<Fixed<Ratio, One, u8, Range<10, 20>>, Overlap>::try_from_raw(12);
/// ```
///
/// ```compile_fail
/// use exact_units::{Fixed, niche::{Niched, Null}, prefix::One, units::Ratio};
///
/// // without bounds every u8 is a value, and 0xFF too
/// let _ = Niched::<Fixed<Ratio, One, u8>, Null<0xFF>>::try_from_raw(3);
/// ```
#[cfg_attr(
    feature = "shrink_wrap",
    derive_shrink_wrap(borrowed, owned(feature = "std"), transparent, validate = Self::validate)
)]
#[derive(Clone, Copy)]
#[repr(transparent)]
pub struct Niched<V: RawValue<Raw = T>, N: Niches<T>, T: Storage = <V as RawValue>::Raw> {
    /// A value within `V`'s bounds or a code of `N`.
    raw: T,
    /// The value type, in the type only.
    _value: PhantomData<V>,
    /// The niches, in the type only.
    _niches: PhantomData<N>,
}

impl<T: Storage, V: RawValue<Raw = T>, N: Niches<T>> Niched<V, N, T> {
    /// The proof that the codes fit the storage and stay out of the bounds, a compile error where it fails.
    const CHECKED: () = check_codes(V::RANGE, T::MIN_INT, T::MAX_INT, N::CODES);

    #[inline(always)]
    fn from_raw(raw: T) -> Self {
        let () = Self::CHECKED;
        Niched {
            raw,
            _value: PhantomData,
            _niches: PhantomData,
        }
    }

    /// The value.
    #[inline(always)]
    pub fn new(value: V) -> Self {
        Self::from_raw(value.raw())
    }

    /// The code of `meaning`.
    ///
    /// # Errors
    ///
    /// [OutOfBounds] for a meaning holding a code outside its range (`Reserved(5)`); never for a meaning
    /// [get](Self::get) answered.
    #[inline(always)]
    pub fn niche(meaning: N) -> Result<Self, OutOfBounds> {
        meaning.encode().map(Self::from_raw).ok_or(OutOfBounds)
    }

    /// The value or the niche of `reading`: the same raw number the reading was decoded from.
    ///
    /// # Errors
    ///
    /// As [niche](Self::niche).
    #[inline(always)]
    pub fn from_reading(reading: Reading<V, N>) -> Result<Self, OutOfBounds> {
        match reading {
            Reading::Value(v) => Ok(Self::new(v)),
            Reading::Niche(n) => Self::niche(n),
        }
    }

    /// `raw` as read from a register or a bus, if it is a value within the bounds or a niche code.
    ///
    /// # Errors
    ///
    /// [OutOfBounds] for anything else.
    #[inline(always)]
    pub fn try_from_raw(raw: T) -> Result<Self, OutOfBounds> {
        if Self::allows(raw) {
            Ok(Self::from_raw(raw))
        } else {
            Err(OutOfBounds)
        }
    }

    /// Whether `raw` is a value within the bounds or a niche code.
    #[inline(always)]
    fn allows(raw: T) -> bool {
        N::decode(raw).is_some() || V::try_from_raw(raw).is_ok()
    }

    /// The raw number, as stored and sent.
    #[inline(always)]
    pub fn raw(self) -> T {
        self.raw
    }

    /// The value, or the meaning of the niche code.
    #[inline(always)]
    pub fn get(self) -> Reading<V, N> {
        let () = Self::CHECKED;
        match N::decode(self.raw) {
            Some(n) => Reading::Niche(n),
            // not a code, so within the bounds: every constructor checked one or the other
            None => Reading::Value(V::from_raw_unchecked(self.raw, sealed::Token(()))),
        }
    }

    /// The value, `None` for a niche code.
    #[inline(always)]
    pub fn value(self) -> Option<V> {
        self.get().value()
    }

    /// The niche's meaning, `None` for a value.
    #[inline(always)]
    pub fn niche_meaning(self) -> Option<N> {
        N::decode(self.raw)
    }

    /// Run on every value read from the wire: neither a value within the bounds nor a code is
    /// [shrink_wrap::Error::SubtypeOutOfRange].
    #[cfg(feature = "shrink_wrap")]
    #[inline(always)]
    fn validate(&self) -> Result<(), ShrinkWrapError> {
        if Self::allows(self.raw) {
            Ok(())
        } else {
            Err(ShrinkWrapError::SubtypeOutOfRange)
        }
    }
}

impl<T: Storage, V: RawValue<Raw = T>, const CODE: i128> Niched<V, Null<CODE>, T>
where
    Null<CODE>: Niches<T>,
{
    /// The null code.
    #[inline(always)]
    #[must_use]
    pub fn null() -> Self {
        // the code fits the storage, CHECKED proves it, so this doesn't wrap
        Self::from_raw(T::from_int_wrapping(CODE))
    }

    /// `Some(v)` as the value, `None` as the null code.
    #[inline(always)]
    pub fn from_option(value: Option<V>) -> Self {
        value.map_or_else(Self::null, Self::new)
    }
}

impl<T: Storage, V: RawValue<Raw = T>, const CODE: i128> From<Option<V>>
    for Niched<V, Null<CODE>, T>
where
    Null<CODE>: Niches<T>,
{
    #[inline(always)]
    fn from(value: Option<V>) -> Self {
        Self::from_option(value)
    }
}

impl<T: Storage, V: RawValue<Raw = T>, const CODE: i128> From<Niched<V, Null<CODE>, T>>
    for Option<V>
where
    Null<CODE>: Niches<T>,
{
    #[inline(always)]
    fn from(n: Niched<V, Null<CODE>, T>) -> Self {
        n.value()
    }
}

impl<T: Storage, V: RawValue<Raw = T>, N: Niches<T>> From<V> for Niched<V, N, T> {
    #[inline(always)]
    fn from(value: V) -> Self {
        Self::new(value)
    }
}

impl<T: Storage, V: RawValue<Raw = T>, N: Niches<T>> TryFrom<Reading<V, N>> for Niched<V, N, T> {
    type Error = OutOfBounds;

    #[inline(always)]
    fn try_from(reading: Reading<V, N>) -> Result<Self, OutOfBounds> {
        Self::from_reading(reading)
    }
}

impl<T: Storage, V: RawValue<Raw = T>, N: Niches<T>> From<Niched<V, N, T>> for Reading<V, N> {
    #[inline(always)]
    fn from(n: Niched<V, N, T>) -> Self {
        n.get()
    }
}

impl<T: Storage, V: RawValue<Raw = T>, N: Niches<T>> PartialEq for Niched<V, N, T> {
    #[inline(always)]
    fn eq(&self, other: &Self) -> bool {
        self.raw == other.raw
    }
}

impl<T: Storage, V: RawValue<Raw = T>, N: Niches<T>> Eq for Niched<V, N, T> where T: Eq {}

impl<T: Storage, V: RawValue<Raw = T>, N: Niches<T>> Hash for Niched<V, N, T>
where
    T: Hash,
{
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.raw.hash(state);
    }
}

impl<T: Storage, V: RawValue<Raw = T> + fmt::Debug, N: Niches<T>> fmt::Debug for Niched<V, N, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.get() {
            Reading::Value(v) => v.fmt(f),
            Reading::Niche(n) => n.fmt(f),
        }
    }
}

/// The value as it displays, a niche by its meaning's name (`NotAvailable`, `Reserved(252)`).
impl<T: Storage, V: RawValue<Raw = T> + fmt::Display, N: Niches<T>> fmt::Display
    for Niched<V, N, T>
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.get() {
            Reading::Value(v) => v.fmt(f),
            Reading::Niche(n) => fmt::Debug::fmt(&n, f),
        }
    }
}

/// Which ends of a [Clipped] value's range are rails: [BothRails], [LowRail] or [HighRail].
pub trait Rails: Copy + Default + 'static {
    /// The lowest value allowed is a rail.
    const LOW: bool;

    /// The highest value allowed is a rail.
    const HIGH: bool;
}

/// Both ends are rails: an ADC reading 0 or full scale, a Q15 after saturation.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct BothRails;

/// Only the lowest value is a rail: a sensor that reads its lowest value for everything below it.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct LowRail;

/// Only the highest value is a rail: a counter or a reading that stops at its top.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct HighRail;

impl Rails for BothRails {
    const LOW: bool = true;

    const HIGH: bool = true;
}

impl Rails for LowRail {
    const LOW: bool = true;

    const HIGH: bool = false;
}

impl Rails for HighRail {
    const LOW: bool = false;

    const HIGH: bool = true;
}

/// A [Clipped] value with its rail mark.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum Clip<V> {
    /// Inside the range: the value as measured.
    Exact(V),
    /// At the low rail: the true value is this or lower.
    AtMost(V),
    /// At the high rail: the true value is this or higher.
    AtLeast(V),
}

impl<V> Clip<V> {
    /// The number, whatever its mark.
    #[inline(always)]
    pub fn value(self) -> V {
        match self {
            Clip::Exact(v) | Clip::AtMost(v) | Clip::AtLeast(v) => v,
        }
    }

    /// `true` at a rail.
    #[inline(always)]
    pub fn is_clipped(&self) -> bool {
        !matches!(self, Clip::Exact(_))
    }
}

/// A value whose lowest and highest values (the bounds of `V`, or its storage's ends without them) are rails, where
/// a reading stops when the true value is beyond it: an ADC at full scale, a Q15 after saturation, J1939 coolant at
/// −40 °C. [get](Self::get) marks such a value [AtLeast](Clip::AtLeast) or [AtMost](Clip::AtMost). The bytes are
/// `V`'s; an integer storage only.
///
/// A rail is a value with a mark, unlike a niche code, which is no value at all; the two combine:
/// `Niched<Clipped<V>, N>`.
///
/// ```
/// use exact_units::{Fixed, Range, niche::{Clip, Clipped}, prefix::Rational, units::Volt};
///
/// // a 12-bit ADC on 3.3 V
/// type Volts = Fixed<Volt, Rational<33, 40950>, u16, Range<0, 4095>>;
/// type Adc = Clipped<Volts>;
///
/// assert_eq!(Adc::try_from_raw(2048).map(Adc::get), Ok(Clip::Exact(Volts::bounded(2048))));
/// assert_eq!(Adc::try_from_raw(4095).map(Adc::get), Ok(Clip::AtLeast(Volts::bounded(4095))));
/// assert_eq!(Adc::try_from_raw(0).map(Adc::get), Ok(Clip::AtMost(Volts::bounded(0))));
/// ```
#[cfg_attr(
    feature = "shrink_wrap",
    derive_shrink_wrap(borrowed, owned(feature = "std"), transparent)
)]
#[repr(transparent)]
pub struct Clipped<V, R = BothRails> {
    /// The value.
    value: V,
    /// The rails, in the type only.
    _rails: PhantomData<R>,
}

impl<V: RawValue, R: Rails> Clipped<V, R> {
    /// The proof that the storage is an integer, a compile error where it fails.
    const RANGE: (i128, i128) = {
        assert!(V::RANGE.is_some(), "rails need an integer storage");
        match V::RANGE {
            Some(r) => r,
            None => (0, -1),
        }
    };

    /// The value, marked at a rail when it is at one.
    #[inline(always)]
    pub fn new(value: V) -> Self {
        let _ = Self::RANGE;
        Clipped {
            value,
            _rails: PhantomData,
        }
    }

    /// `raw` as read, if it is within the bounds.
    ///
    /// # Errors
    ///
    /// [OutOfBounds] outside them.
    #[inline(always)]
    pub fn try_from_raw(raw: V::Raw) -> Result<Self, OutOfBounds> {
        V::try_from_raw(raw).map(Self::new)
    }

    /// The raw number, as stored and sent.
    #[inline(always)]
    pub fn raw(self) -> V::Raw {
        self.value.raw()
    }

    /// The value, whatever its mark.
    #[inline(always)]
    pub fn value(self) -> V {
        self.value
    }

    /// The value with its mark: [AtMost](Clip::AtMost) at the low rail, [AtLeast](Clip::AtLeast) at the high one.
    #[inline(always)]
    pub fn get(self) -> Clip<V> {
        let (lo, hi) = Self::RANGE;
        match self.value.raw().to_repr() {
            Repr::Int(x) if R::LOW && x == lo => Clip::AtMost(self.value),
            Repr::Int(x) if R::HIGH && x == hi => Clip::AtLeast(self.value),
            Repr::Int(_) | Repr::Float(_) => Clip::Exact(self.value),
        }
    }

    /// The value of `clip`: the same raw number it was read from.
    ///
    /// # Errors
    ///
    /// [OutOfBounds] for [AtMost](Clip::AtMost) or [AtLeast](Clip::AtLeast) of a value not at that rail.
    #[inline(always)]
    pub fn from_clip(clip: Clip<V>) -> Result<Self, OutOfBounds> {
        let c = Self::new(clip.value());
        match (clip, c.get()) {
            (Clip::Exact(_), _)
            | (Clip::AtMost(_), Clip::AtMost(_))
            | (Clip::AtLeast(_), Clip::AtLeast(_)) => Ok(c),
            _ => Err(OutOfBounds),
        }
    }
}

impl<V: RawValue, R: Rails> From<V> for Clipped<V, R> {
    #[inline(always)]
    fn from(value: V) -> Self {
        Self::new(value)
    }
}

impl<V: Clone, R> Clone for Clipped<V, R> {
    #[inline(always)]
    fn clone(&self) -> Self {
        Clipped {
            value: self.value.clone(),
            _rails: PhantomData,
        }
    }
}

impl<V: Copy, R> Copy for Clipped<V, R> {}

impl<V: PartialEq, R> PartialEq for Clipped<V, R> {
    #[inline(always)]
    fn eq(&self, other: &Self) -> bool {
        self.value == other.value
    }
}

impl<V: Eq, R> Eq for Clipped<V, R> {}

impl<V: Hash, R> Hash for Clipped<V, R> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.value.hash(state);
    }
}

impl<V: RawValue + fmt::Debug, R: Rails> fmt::Debug for Clipped<V, R> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.get().fmt(f)
    }
}

/// The value as it displays, `≥ ` before it at the high rail and `≤ ` at the low one.
impl<V: RawValue + fmt::Display, R: Rails> fmt::Display for Clipped<V, R> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.get() {
            Clip::Exact(v) => v.fmt(f),
            Clip::AtMost(v) => write!(f, "≤ {v}"),
            Clip::AtLeast(v) => write!(f, "≥ {v}"),
        }
    }
}

impl<V: RawValue, R: Rails> sealed::Sealed for Clipped<V, R> {}

impl<V: RawValue, R: Rails> RawValue for Clipped<V, R> {
    type Raw = V::Raw;

    const RANGE: Option<(i128, i128)> = V::RANGE;

    #[inline(always)]
    fn raw(self) -> V::Raw {
        self.value.raw()
    }

    #[inline(always)]
    fn try_from_raw(raw: V::Raw) -> Result<Self, OutOfBounds> {
        V::try_from_raw(raw).map(Self::new)
    }

    #[inline(always)]
    fn from_raw_unchecked(raw: V::Raw, token: sealed::Token) -> Self {
        Self::new(V::from_raw_unchecked(raw, token))
    }
}

#[cfg(feature = "shrink_wrap")]
/// The error type the derive's `validate` returns.
type ShrinkWrapError = shrink_wrap::Error;

#[cfg(test)]
mod tests;
