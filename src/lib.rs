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
//! - [units]: zero-size unit markers (SI, angle, information, counted things) with their [Dim] and [Scale], and the
//!   products and quotients the compiler checks (`Volt · Ampere` is `Watt`).
//! - [Dim]: the dimension as data: SI base exponents, angle, information, up to [MAX_COUNTED] counted bases and a
//!   square root (V/√Hz). [Magnitude]: a factor of integers and powers of 10, 2 and π (`°` is π/180 rad).
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
//! - [decibel]: levels in dB of a gain, 20·lg or 10·lg by its [Kind]: [db!] works the stored value out at compile
//!   time, exactly; `to_db` / `from_db` at run time in `f64` and `f32` (feature `std` or `libm`). Nepers are the same
//!   with [np!] and `to_np` / `from_np`.
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
//! `serde` their serde, `libm` the run-time dB without `std`. Unit groups are cargo features (`time`, `electrical`, `mechanical`, `angle`, `thermal`, `chemical`, `photometric`,
//! `radiometric`, `radiation`, `fields`, `dimensionless`, `information`, `counting`, `levels`, `quantities` for all).
//!
//! Run-time quantities (a prefix and a number type chosen per value) are `ww_si`'s `Quantity` and `AnyQuantity`,
//! built on this crate.

// unit tests format with std also when the crate is built without it
#[cfg(all(test, not(feature = "std")))]
extern crate std;

// the README's examples run as doctests, so what it claims is checked
#[cfg(all(doctest, feature = "std"))]
#[doc = include_str!("../README.md")]
struct ReadmeDoctests;

// the book's pages (docs/) run as doctests too, so every example on the site compiles and does what it says
#[cfg(all(doctest, feature = "std"))]
mod book;

pub mod complex;
mod conv_error;
pub mod cordic;
pub mod decibel;
mod declare;
mod exact;
mod fixed;
#[cfg(feature = "counting")]
mod index;
pub mod kind;
#[cfg(feature = "levels")]
pub mod level;
pub mod literal;
#[cfg(feature = "q")]
#[doc(hidden)]
pub use exact_units_macros as __macros;
mod magnitude;
pub mod niche;
pub mod num;
mod point;
mod products;
#[cfg(feature = "serde")]
pub mod serde;
#[cfg(feature = "shrink_wrap")]
mod si_expr;
mod si_prefix;
mod text;
pub mod tree;
mod unit;
pub mod units;

#[cfg(all(feature = "angle", feature = "dimensionless"))]
pub use complex::Nco;
#[cfg(feature = "electrical")]
pub use complex::{Admittance, Impedance};
#[cfg(feature = "dimensionless")]
pub use complex::{CQ15, CQ31};
pub use complex::{Complex, ComplexStorage, Interleaved, IqStorage, Mac, NormStorage, Polar};
#[doc(hidden)]
pub use exact::Fraction8;
#[doc(hidden)]
pub use exact::order_parts as __order_parts;
pub use exact::{Exact, Fraction, InPrefix, ParseExactError, ParseFractionError};
#[cfg(feature = "dimensionless")]
pub use exact::{OffTarget, Tolerance};
#[cfg(feature = "angle")]
pub use fixed::{Bam16, Bam32, PhaseBam16, PhaseBam32};
pub use fixed::{
    Bounds, Count, DecimalPrefix, Fixed, FixedScale, MaybeNan, NanFree, OutOfBounds, Overflow,
    Range, RangeDec, RangeU64, Repr, Requantizer, Rounding, Saturating, Storage, StorageKind,
    Unbounded, Unlimited, UnsignedStorage, Wrapping, prefix, requantize,
};
#[cfg(feature = "information")]
pub use fixed::{Bytes, Gibibytes, Kibibytes, Mebibytes};
#[cfg(feature = "time")]
pub use fixed::{Days, Hours, Hz, Micros, Millis, Minutes, Nanos, Secs};
#[cfg(feature = "dimensionless")]
pub use fixed::{Duty16, Percent, Permille, Ppb, Ppm, Q15, Q31, UQ16_16};
#[cfg(feature = "counting")]
pub use index::{Index, IndexFrom1, Seq};
pub use kind::{Kind, LevelForm};
pub use magnitude::{
    Irrational, MAX_FACTORS, Magnitude, MagnitudeError, MagnitudeRest, PrimePower,
};
pub use point::{
    AmbientPressure, Linear, Modulo, Modulus, Offset, Origin, OriginDef, Point, PointArith,
    PointDiff, Reference, UnitZero, origins,
};
#[cfg(feature = "shrink_wrap")]
pub use si_expr::SIExpr;
#[cfg(all(feature = "std", feature = "shrink_wrap"))]
pub use si_expr::SIExprOwned;
pub use si_prefix::{FullScaleBits, Pow2Exp, Pow10Exp, Prefix, PrefixDecimal, PrefixRatio};
pub use tree::MAX_UNIT_DEPTH;
#[doc(hidden)]
pub use unit::exp as __dim_exp;
pub use unit::{Counted, Dim, MAX_COUNTED, Provenance, Scale, Unit, UnitDiv, UnitExpr, UnitMul};
#[cfg(all(feature = "std", feature = "shrink_wrap"))]
pub use unit::{CountedOwned, DimOwned, ProvenanceOwned};

/// The [Dim] of a counted unit marker `Name` of the crate this is written in: `Dim::counted(<this crate>, "Name")`,
/// with the crate as Rust names it (`CARGO_CRATE_NAME`). Two crates' markers of the same name are different bases.
///
/// ```
/// let bucket = exact_units::counted!(Bucket);
/// assert_eq!(bucket, exact_units::Dim::counted(env!("CARGO_CRATE_NAME"), "Bucket"));
/// assert_eq!(bucket.to_string(), "Bucket");
/// ```
#[macro_export]
macro_rules! counted {
    ($name:ident) => {
        $crate::Dim::counted(env!("CARGO_CRATE_NAME"), stringify!($name))
    };
}
