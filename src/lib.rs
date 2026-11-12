#![cfg_attr(not(feature = "std"), no_std)]
#![doc(
    html_logo_url = "https://raw.githubusercontent.com/romixlab/exact_units/main/assets/logo/icon.svg",
    html_favicon_url = "https://raw.githubusercontent.com/romixlab/exact_units/main/assets/logo/icon-small.svg"
)]
//! Units of measure fixed at compile time, exact: a number with its unit, scale and storage in the type, on the wire
//! exactly the number.
//!
//! - [Fixed]`<U, P, T, B>`: a `T` in unit `U` and scale `P` (a prefix, a binary prefix, a power of ten, a fraction),
//!   within bounds `B`. `Fixed<Second, Micro, u32>` is a time in µs, the same bytes as the `u32` it replaces.
//!   Conversions between scales are exact or refused, never rounded through a float.
//! - [units]: zero-size unit markers (SI, angle, information, counted things) with their [UnitDef] (identity, [Dim],
//!   exact scale), and the products and quotients the compiler checks (`Volt · Ampere` is `Watt`).
//! - [Dim]: the dimension as data: SI's seven base exponents and a list of distinguished bases with no count limit
//!   (angle, level, information, counted things), every exponent in halves (V/√Hz). [Magnitude]: a factor of integers
//!   and powers of 10, 2 and π (`°` is π/180 rad).
//! - [Complex]`<U, P, T, B>`: a complex quantity, two `T` in unit `U` and scale `P`: an [Impedance], an
//!   [Admittance], a phasor, a Q15 IQ sample. Products and conversions by the rules of `Fixed`, part by part, and
//!   a complex multiply rounded once into a chosen scale. For IQ samples: [CQ15] and [CQ31], a multiply-accumulate
//!   in the wide type ([Mac]), interleaved buffers ([Interleaved]) and an [Nco] mixer.
//! - [Point]`<O, Fixed<..>>`: a position against an [Origin] (°C, °F, gauge pressure, an angle that wraps at a
//!   turn): point + vector = point, point − point = vector, conversions between origins exact or refused.
//! - [Index]`<Of, T, B>`, [IndexFrom1] and [Seq]`<Of, T>` (feature `counting`): a position in a table of `Of`, 0- or
//!   1-based, and a wrapping sequence number in serial number order; index − index is an amount, index + index
//!   doesn't compile.
//! - `SIExpr` (feature `shrink_wrap`): a unit as a tree on the wire, for a unit only known at run time.
//! - [decibel]: levels in dB of a gain, 20·lg or 10·lg by its [quantity](QuantityExpr): [db!] works the stored value
//!   out at compile time, exactly; `to_db` / `from_db` at run time in `f64` and `f32` (feature `std` or `libm`). Nepers
//!   are the same with [np!] and `to_np` / `from_np`.
//! - [level] (feature `levels`): a level as a quantity, the neper on a base of its own and the decibel a scaled
//!   unit of it, a loss on a second base (`Loss<Decibel>`, positive for a decay), absorbance and optical depth,
//!   rates like dB/m and Np/s, and the edge to a gain (`to_level`, `to_gain`), where 10·lg or 20·lg comes in.
//! - [literal]: [fixed!] turns a decimal literal into the stored value of the [Fixed] or [Complex] its context names
//!   at compile time, exactly rounded: `fixed!(0.1)` in Q30, no hand-computed `107_374_182`.
//! - `serde` (feature `serde`): every number as the number it stores, bounds checked on reading, and
//!   `serde::with_unit` for a field as text with its unit (`"1.5 ms"`), read back exactly; `no_std`, no alloc.
//!
//! `no_std`, no alloc, and no dependencies without features. `shrink_wrap` adds the wire format (`shrink_wrap`'s),
//! `std` every unit group and with `shrink_wrap` the owned forms of the wire types, `defmt` their `defmt::Format`,
//! `serde` their serde, `libm` the run-time dB without `std`, `bignum` an `Exact` of any size (`BigFraction`, with
//! `alloc`). Unit groups are cargo features (`time`, `electrical`,
//! `mechanical`, `angle`, `thermal`, `chemical`, `photometric`, `radiometric`, `radiation`, `fields`, `dimensionless`,
//! `information`, `counting`, `levels`, `quantities` for all).
//!
//! Run-time quantities (a prefix and a number type chosen per value) are `ww_si`'s `Quantity` and `AnyQuantity`,
//! built on this crate.

// unit tests format with std also when the crate is built without it
// BigFraction allocates: its integers grow as they need
#[cfg(feature = "bignum")]
extern crate alloc;
#[cfg(all(test, not(feature = "std")))]
extern crate std;

// the README's examples run as doctests, so what it claims is checked
#[cfg(all(doctest, feature = "std"))]
#[doc = include_str!("../README.md")]
struct ReadmeDoctests;

// the book's pages (docs/) run as doctests too, so every example on the site compiles and does what it says
#[cfg(all(doctest, feature = "std"))]
mod book;

pub mod codec;
pub mod complex;
pub mod constants;
mod contains;
mod conv_error;
pub mod cordic;
pub mod decibel;
mod declare;
mod dim;
mod exact;
#[cfg(feature = "levels")]
pub mod exposure;
mod fixed;
#[cfg(feature = "counting")]
mod index;
mod interval;
#[cfg(feature = "leap_seconds")]
pub mod leap;
#[cfg(feature = "levels")]
pub mod level;
pub mod literal;
mod names;
#[cfg(feature = "q")]
#[doc(hidden)]
pub use exact_units_macros as __macros;
mod magnitude;
pub mod niche;
pub mod notation;
pub mod num;
mod point;
mod products;
pub mod quantities;
pub mod quantity;
#[cfg(feature = "radiation")]
pub mod radiation;
pub mod registry;
mod scaled;
#[cfg(feature = "serde")]
pub mod serde;
#[cfg(feature = "shrink_wrap")]
mod si_expr;
mod si_prefix;
pub mod system;
pub mod systems;
mod text;
pub mod tree;
mod unit;
pub mod units;
mod window;

#[cfg(all(feature = "angle", feature = "dimensionless"))]
pub use complex::Nco;
#[cfg(all(feature = "electrical", feature = "dimensionless"))]
pub use complex::Undefined;
#[cfg(feature = "electrical")]
pub use complex::{Admittance, Impedance, Z0};
#[cfg(feature = "dimensionless")]
pub use complex::{CQ15, CQ31};
pub use complex::{
    Complex, ComplexStorage, Extended, Interleaved, IqStorage, Mac, NormStorage, Polar,
};
pub use contains::Contains;
/// dashu's integers, of a [BigFraction]'s [numer](BigFraction::numer) and [denom](BigFraction::denom).
#[cfg(feature = "bignum")]
pub use dashu_int;
#[doc(hidden)]
pub use declare::{__dim_exp, __dim_set};
#[cfg(feature = "shrink_wrap")]
pub use dim::DimWire;
pub use dim::{BaseDef, BaseExp, BaseSource, Bases, Dim, Exp, MAX_TRUE_BASES};
#[cfg(all(feature = "std", feature = "shrink_wrap"))]
pub use dim::{BaseDefOwned, BaseExpOwned, DimOwned, DimWireOwned, NotCanonical};
#[doc(inline)]
pub use exact::Crossing as ExactCrossing;
#[doc(hidden)]
pub use exact::Fraction8;
#[doc(hidden)]
pub use exact::order_parts as __order_parts;
#[cfg(feature = "bignum")]
pub use exact::{BigExact, BigFraction, NotAFraction};
pub use exact::{
    Exact, ExactNum, Fraction, InPrefix, NearTie, ParseExactError, ParseFractionError,
    ParseUncertainError, PlusMinus, Refused, Rounded, Uncertain,
};
#[cfg(feature = "dimensionless")]
pub use exact::{OffTarget, Tolerance};
#[cfg(feature = "angle")]
pub use fixed::{Bam16, Bam32, PhaseBam16, PhaseBam32};
pub use fixed::{
    Bounds, Count, Crossing, DecimalPrefix, Finite, Fixed, FixedScale, MaybeNan, NanFree, OneOf,
    OutOfBounds, OutOfRange, Overflow, Range, RangeDec, RangeU64, Repr, Requantizer, Rounding,
    Saturating, ScaleDef, Storage, StorageKind, StorageOrd, Unbounded, Unlimited, UnsignedStorage,
    ValueSet, Wrapping, prefix, requantize,
};
#[cfg(feature = "information")]
pub use fixed::{Bytes, Gibibytes, Kibibytes, Mebibytes};
#[cfg(feature = "time")]
pub use fixed::{Days, Hours, Hz, Micros, Millis, Minutes, Nanos, Secs};
#[cfg(feature = "dimensionless")]
pub use fixed::{Duty16, Percent, Permille, Ppb, Ppm, Q15, Q31, UQ16_16};
#[cfg(feature = "counting")]
pub use index::{Index, IndexFrom1, Seq};
pub use magnitude::{MAX_FACTORS, Magnitude, MagnitudeError, MagnitudeRest, PrimePower, Symbols};
pub use notation::Notation;
pub use point::{
    AmbientPressure, Linear, Modulo, Modulus, Offset, Origin, OriginDef, Point, PointArith,
    PointDiff, Reference, ReferenceDef, ReferenceHow, TimeCounting, TimeScaleDef, TimeZero,
    UnitZero, origins,
};
pub use quantity::{Character, LevelForm, QuantityDef, QuantityExpr, QuantityRef, QuantitySpec};
#[cfg(all(feature = "std", feature = "shrink_wrap"))]
pub use quantity::{QuantityDefOwned, QuantityRefOwned};
pub use registry::Registry;
#[cfg(feature = "time")]
pub use scaled::Ticks;
pub use scaled::{RunScale, Scaled};
#[cfg(all(feature = "std", feature = "shrink_wrap"))]
pub use si_expr::SIExprOwned;
#[cfg(feature = "shrink_wrap")]
pub use si_expr::{CompactError, SIExpr};
pub use si_prefix::{FullScaleBits, Pow2Exp, Pow10Exp, Prefix, PrefixDecimal, PrefixRatio};
pub use system::{BaseRule, BridgeDef, BridgeError, SetToOne, System, SystemDef, SystemRef};
#[cfg(all(feature = "std", feature = "shrink_wrap"))]
pub use system::{BaseRuleOwned, BridgeDefOwned, SetToOneOwned, SystemDefOwned, SystemRefOwned};
pub use tree::MAX_UNIT_DEPTH;
pub use unit::{
    MarkerId, PrefixPolicy, PrefixSet, Provenance, Unit, UnitDef, UnitDiv, UnitExpr, UnitMul,
    UnitNames, UnitTable,
};
#[cfg(all(feature = "std", feature = "shrink_wrap"))]
pub use unit::{MarkerIdOwned, ProvenanceOwned, UnitDefOwned, UnitNamesOwned};
pub use window::{Fit, Window};

/// The [MarkerId] of the marker `Name` of the crate this is written in, with the crate as Rust names it
/// (`CARGO_CRATE_NAME`): a distinguished base a crate declares with `base` on a [unit!](crate::unit) line is
/// `Dim::base(base!(Name))`. Two crates' markers of the same name are different bases.
///
/// ```
/// let pixel = exact_units::Dim::base(exact_units::base!(Pixel));
/// assert_eq!(exact_units::base!(Pixel).crate_name, env!("CARGO_CRATE_NAME"));
/// assert_eq!(pixel.to_string(), "Pixel");
/// ```
#[macro_export]
macro_rules! base {
    ($name:ident) => {
        $crate::MarkerId {
            crate_name: env!("CARGO_CRATE_NAME"),
            name: stringify!($name),
        }
    };
}
