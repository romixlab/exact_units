//! [unit!](crate::unit) and [scale!](crate::scale): unit and scale markers declared as data, so that both the
//! compiler (through the `impl Unit` / `impl FixedScale` they expand to) and WireWeaver's walker (which reads the
//! same lines from source, INT-13) know what a marker stands for. A hand-written `impl Unit` still compiles, but an
//! API that uses such a marker is refused by the walker: a client would not know the unit.

/// Declares unit markers: a zero-size struct per line and its [Unit](crate::Unit) impl, from the unit's symbol,
/// dimension and scale. WireWeaver reads the same lines from source, so a client of an API that uses the marker
/// knows its definition without a table of its own.
///
/// ```
/// exact_units::unit! {
///     /// Revolutions per minute: 2π rad / 60 s, exactly.
///     pub struct Rpm = "rpm", [rad 1, s -1], Exact(1 / 30, pi 1);
///     /// A count of buckets of a histogram, a base of its own (named by this crate and `Bucket`).
///     pub struct Bucket = "bucket", counted;
/// }
/// use exact_units::{Fixed, Unit, prefix::One};
///
/// # use core::f64::consts::TAU;
/// let speed = Fixed::<Rpm, One, u8>::new(60);
/// assert_eq!(format!("{speed}"), "60 rpm");
/// assert!((Rpm::EXPR.factor() * 60.0 - TAU).abs() < 1e-12); // rad/s
/// assert_eq!(Bucket::DIM, exact_units::counted!(Bucket));
/// ```
///
/// Each line is `<attributes> <visibility> struct <Name> = "<symbol>" [(<names>)], <dimension>[, <scale>];`:
///
/// - The attributes are the marker's (docs, `#[cfg(..)]`); its derives are `Copy, Clone, Debug, Default, PartialEq,
///   Eq, Hash`. A `#[cfg(..)]` applies to its impl too.
/// - The dimension is a list of exponents of `s`, `m`, `kg`, `a`, `k`, `mol`, `cd`, `rad`, `bit` (`[kg 1, m 2, s -3,
///   a -1]` for a volt, `[]` for a ratio), or `counted`: a counted base of its own named by the crate and the marker
///   ([counted!](crate::counted)), or `counted(Other)`: the counted base of the marker `Other` of the same crate, a
///   scaled unit of it (the decibel of the neper).
/// - The scale, an exact [Magnitude](crate::Magnitude), one when left out: `Pow10(-3)`, `Exact(1 / 180, pi 1)`
///   (`num / den`, then optionally `pow10 e`, `pow2 e`, `pi e`, `ln2 e`, `ln10 e`, `mu0 e`, `g e` in that order, see
///   [Magnitude::new](crate::Magnitude::new); the decibel is `Exact(1 / 20, ln10 1)` of the neper), so every reader
///   computes the same value. The measured constants μ₀ and G stay symbols ([constants](crate::constants)), their
///   exponent whole or a half (`mu0 1 / 2`): an oersted is `Exact(1 / 1, pow10 -4, mu0 -1)` A/m. A unit has no offset: a scale with a zero of its own is an origin of points
///   ([origin!](crate::origin)), and a float factor isn't a unit's scale.
///
///   ```compile_fail
///   exact_units::unit! {
///       /// °C as a unit with an offset: refused, it is the origin `origins::Celsius`.
///       pub struct DegC = "°C", [k 1], Affine { factor: 1.0, offset: 273.15 };
///   }
///   ```
/// - The names in parentheses after the symbol ([UnitNames](crate::UnitNames)) say how text writes and reads the unit,
///   each optional, in any order, separated by commas: `ascii "Ohm"` where the transliteration of the symbol (`µ` as
///   `u`, `Ω` as `Ohm`, `²` as `^2`) isn't its ASCII form, `long "ohm"` (the plural adds an s) or `long "foot" plural
///   "feet"`, `aliases ["nit", "nt"]`, `ucum "Ohm"`, and `prefixes none`, `decimal`, `decimal_and_binary` or `only
///   [Milli, Kilo]` ([PrefixPolicy](crate::PrefixPolicy)). Without `prefixes`, a coherent unit whose symbol is one
///   word takes the SI prefixes and every other unit none (a scaled one, `min`; a counted one; a compound symbol,
///   whose parts take theirs).
///
///   ```
///   exact_units::unit! {
///       /// Shaft speed, rpm, with its names: `krpm` reads, `rpm` is written in ASCII as it is.
///       pub struct Rpm = "rpm" (long "revolution per minute" plural "revolutions per minute", prefixes only [Kilo]),
///           [rad 1, s -1], Exact(1 / 30, pi 1);
///   }
///   use exact_units::{Prefix, Unit};
///
///   let names = Rpm::EXPR.def().unwrap().names;
///   assert_eq!(names.plural, "revolutions per minute");
///   assert!(names.prefixes.allows(Prefix::Kilo) && !names.prefixes.allows(Prefix::Milli));
///   ```
///
///   A symbol with a character that has no ASCII form, and no `ascii`, doesn't compile:
///
///   ```compile_fail
///   exact_units::unit! {
///       /// Optical depth, τ: needs `("τ" (ascii "tau"), ..)`.
///       pub struct Tau = "τ", [];
///   }
///   ```
/// - The marker's identity is the crate that declares it (`CARGO_CRATE_NAME`) and its name
///   ([MarkerId](crate::MarkerId)): two crates' `Rpm` are two units.
/// - After the dimension, `kind "resistance"` names a kind of quantity of its own ([Kind::Named](crate::Kind)):
///   such a unit doesn't convert to or from another of its dimension, `Ohm` is no `Per<Volt, Ampere>`.
/// - After the scale, `conventional("NIST SP 811", 2008)` marks a unit fixed by a physical property (mercury at
///   32 °F, a calorie at 15 °C): its scale is the value that source prints, not a definition
///   ([Provenance::Conventional](crate::Provenance::Conventional)). Without it the scale is the unit's definition.
///
/// Literal values only: the walker reads them from source, it doesn't evaluate constants.
#[macro_export]
macro_rules! unit {
    // exact_units' own table: the markers and a static of all their `UnitExpr`s
    (
        @table $(#[$all_attr:meta])* $all_vis:vis static $all:ident;
        $(
            $(#[$($attr:tt)*])*
            $vis:vis struct $name:ident = $symbol:literal $(($($names:tt)*))?, $dim:tt $(($base:ident))? $(kind $qkind:literal)?
                $(, $kind:ident $args:tt $(, conventional($source:literal, $year:literal))?)?;
        )*
    ) => {
        $crate::unit! {
            $(
                $(#[$($attr)*])*
                $vis struct $name = $symbol $(($($names)*))?, $dim $(($base))? $(kind $qkind)? $(, $kind $args $(, conventional($source, $year))?)?;
            )*
        }

        $(#[$all_attr])*
        #[expect(clippy::allow_attributes, reason = "a table may have no documented line")]
        #[allow(unused_doc_comments, reason = "doc comments of the lines land on the expressions of the table")]
        $all_vis static $all: &[&$crate::UnitExpr] = &[
            $(
                $(#[$($attr)*])*
                <$name as $crate::Unit>::EXPR,
            )*
        ];
    };
    (
        $(
            $(#[$($attr:tt)*])*
            $vis:vis struct $name:ident = $symbol:literal $(($($names:tt)*))?, $dim:tt $(($base:ident))? $(kind $qkind:literal)?
                $(, $kind:ident $args:tt $(, conventional($source:literal, $year:literal))?)?;
        )*
    ) => {
        $(
            $(#[$($attr)*])*
            #[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
            $vis struct $name;

            $crate::__with_cfgs! {
                [] $(#[$($attr)*])* =>
                impl $crate::Unit for $name {
                    const DIM: $crate::Dim<'static> = $crate::__unit_dim!($name $dim $(($base))?);
                    const KIND: $crate::Kind<'static> = match Self::EXPR {
                        $crate::UnitExpr::Named(def) => def.kind(),
                        _ => $crate::Kind::Plain,
                    };
                    const EXPR: &'static $crate::UnitExpr = &$crate::UnitExpr::Named(&$crate::UnitDef {
                        id: $crate::MarkerId {
                            crate_name: env!("CARGO_CRATE_NAME"),
                            name: stringify!($name),
                        },
                        names: $crate::__unit_names!([$symbol] [$dim] [$($kind)?] $($($names)*)?),
                        dim: Self::DIM,
                        quantity: $crate::__unit_quantity!($($qkind)?),
                        scale: $crate::__unit_scale!($($kind $args)?),
                        provenance: $crate::__unit_provenance!($($($source $year)?)?),
                    });
                }
            }

            // the names are checked where the line is, not only where the unit is used
            $crate::__with_cfgs! {
                [] $(#[$($attr)*])* =>
                const _: &$crate::UnitExpr = <$name as $crate::Unit>::EXPR;
            }
        )*
    };
}

/// Declares scale markers, the `P` of a [Fixed](crate::Fixed): a zero-size struct per line and its
/// [FixedScale](crate::FixedScale) impl, from its symbol and exact factor. WireWeaver reads the same lines from
/// source, like [unit!](crate::unit)'s.
///
/// ```
/// exact_units::scale! {
///     /// One step of a gyro, 70 mdps per LSB: 70/1000 of the unit.
///     pub struct Mdps70 = "", Exact(70 / 1000);
///     /// A quarter, 2⁻².
///     pub struct Quarter = "", Pow2(-2);
/// }
/// use exact_units::{FixedScale, Magnitude, prefix::Milli};
///
/// assert_eq!(Mdps70::SCALE, Magnitude::ratio(7, 100));
/// assert_eq!(Quarter::SCALE.to_f64(), 0.25);
/// assert_eq!(Milli::SCALE, Magnitude::pow10(-3));
/// ```
///
/// Each line is `<attributes> <visibility> struct <Name> = "<symbol>", <factor>[, prefix <Prefix>];`: the symbol is
/// written before a unit's (`"m"`, empty for a factor without a prefix); the factor is `Pow10(e)`, `Pow2(e)` or
/// `Exact(num / den, ..)` as in [unit!](crate::unit); `prefix Milli` names the run-time [Prefix](crate::Prefix) of
/// the same scale, and the marker is a [DecimalPrefix](crate::DecimalPrefix) then.
#[macro_export]
macro_rules! scale {
    (
        $(
            $(#[$($attr:tt)*])*
            $vis:vis struct $name:ident = $symbol:literal, $kind:ident $args:tt $(, prefix $prefix:ident)?;
        )*
    ) => {
        $(
            $(#[$($attr)*])*
            #[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
            $vis struct $name;

            $crate::__with_cfgs! {
                [] $(#[$($attr)*])* =>
                impl $crate::FixedScale for $name {
                    const SCALE: $crate::Magnitude = $crate::__scale_magnitude!($kind $args);
                    const SYMBOL: &'static str = $symbol;
                    const NAME: &'static str = stringify!($name);
                    const RUNTIME: Option<$crate::Prefix> = $crate::__scale_runtime!($($prefix)?);
                    const ID: Option<$crate::MarkerId<'static>> = Some($crate::MarkerId {
                        crate_name: env!("CARGO_CRATE_NAME"),
                        name: stringify!($name),
                    });
                }
            }
            $crate::__scale_decimal! { [$(#[$($attr)*])*] $name $($prefix)? }
        )*
    };
}

/// `item` with only the `#[cfg(..)]` attributes of the ones before `=>`.
#[doc(hidden)]
#[macro_export]
macro_rules! __with_cfgs {
    ([$($cfg:tt)*] #[cfg $c:tt] $($rest:tt)*) => {
        $crate::__with_cfgs! { [$($cfg)* #[cfg $c]] $($rest)* }
    };
    ([$($cfg:tt)*] #[$($other:tt)*] $($rest:tt)*) => {
        $crate::__with_cfgs! { [$($cfg)*] $($rest)* }
    };
    ([$($cfg:tt)*] => $($item:tt)*) => {
        $($cfg)* $($item)*
    };
}

/// The dimension of a [unit!] line.
#[doc(hidden)]
#[macro_export]
macro_rules! __unit_dim {
    ($name:ident counted) => {
        $crate::counted!($name)
    };
    ($name:ident counted($base:ident)) => {
        $crate::counted!($base)
    };
    ($name:ident [$($d:ident $e:literal),* $(,)?]) => {{
        #[expect(clippy::allow_attributes, reason = "mutated only when the dimension has exponents")]
        #[allow(unused_mut, reason = "a dimension with no exponents is never changed")]
        let mut dim = $crate::Dim::ONE;
        $(dim.$d = $crate::__dim_exp($e);)*
        dim
    }};
}

/// The kind of quantity of a [unit!] line: `kind "<name>"` after the dimension, else none.
#[doc(hidden)]
#[macro_export]
macro_rules! __unit_quantity {
    () => {
        None
    };
    ($name:literal) => {
        Some($name)
    };
}

/// The names of a [unit!] line: the symbol, the dimension (a counted unit takes no prefix by default), the scale's
/// kind (none for a coherent unit, which takes the SI prefixes by default) and the items of its `( .. )`.
#[doc(hidden)]
#[macro_export]
macro_rules! __unit_names {
    ([$symbol:literal] [counted] [$($kind:ident)?] $($items:tt)*) => {
        $crate::__unit_names!(@munch [] $crate::UnitNames::new($symbol, $crate::PrefixPolicy::None); $($items)*)
    };
    ([$symbol:literal] [$dim:tt] [] $($items:tt)*) => {
        $crate::__unit_names!(@munch [] $crate::UnitNames::new($symbol, $crate::PrefixPolicy::Decimal); $($items)*)
    };
    ([$symbol:literal] [$dim:tt] [$kind:ident] $($items:tt)*) => {
        $crate::__unit_names!(@munch [] $crate::UnitNames::new($symbol, $crate::PrefixPolicy::None); $($items)*)
    };
    (@munch [$($done:tt)*] $base:expr; ) => {
        $crate::UnitNames { $($done)* ..$base }.checked()
    };
    (@munch [$($done:tt)*] $base:expr; , $($rest:tt)*) => {
        $crate::__unit_names!(@munch [$($done)*] $base; $($rest)*)
    };
    (@munch [$($done:tt)*] $base:expr; ascii $s:literal $($rest:tt)*) => {
        $crate::__unit_names!(@munch [$($done)* ascii: Some($s),] $base; $($rest)*)
    };
    (@munch [$($done:tt)*] $base:expr; long $s:literal plural $p:literal $($rest:tt)*) => {
        $crate::__unit_names!(@munch [$($done)* long: $s, plural: $p,] $base; $($rest)*)
    };
    (@munch [$($done:tt)*] $base:expr; long $s:literal $($rest:tt)*) => {
        $crate::__unit_names!(@munch [$($done)* long: $s, plural: concat!($s, "s"),] $base; $($rest)*)
    };
    (@munch [$($done:tt)*] $base:expr; aliases [$first:literal $(, $more:literal)* $(,)?] $($rest:tt)*) => {
        $crate::__unit_names!(@munch [$($done)* aliases: concat!($first $(, "|", $more)*),] $base; $($rest)*)
    };
    (@munch [$($done:tt)*] $base:expr; ucum $s:literal $($rest:tt)*) => {
        $crate::__unit_names!(@munch [$($done)* ucum: Some($s),] $base; $($rest)*)
    };
    (@munch [$($done:tt)*] $base:expr; prefixes none $($rest:tt)*) => {
        $crate::__unit_names!(@munch [$($done)* prefixes: $crate::PrefixPolicy::None,] $base; $($rest)*)
    };
    (@munch [$($done:tt)*] $base:expr; prefixes decimal $($rest:tt)*) => {
        $crate::__unit_names!(@munch [$($done)* prefixes: $crate::PrefixPolicy::Decimal,] $base; $($rest)*)
    };
    (@munch [$($done:tt)*] $base:expr; prefixes decimal_and_binary $($rest:tt)*) => {
        $crate::__unit_names!(
            @munch [$($done)* prefixes: $crate::PrefixPolicy::DecimalAndBinary,] $base; $($rest)*
        )
    };
    (@munch [$($done:tt)*] $base:expr; prefixes only [$($p:ident),* $(,)?] $($rest:tt)*) => {
        $crate::__unit_names!(
            @munch [$($done)* prefixes: $crate::PrefixPolicy::Only($crate::PrefixSet::of(&[$($crate::Prefix::$p),*])),]
            $base; $($rest)*
        )
    };
    (@munch [$($done:tt)*] $base:expr; $($other:tt)*) => {
        compile_error!(concat!(
            "unknown item in a unit's names: ", stringify!($($other)*),
            "; expected ascii \"..\", long \"..\" [plural \"..\"], aliases [\"..\", ..], ucum \"..\", \
             prefixes none | decimal | decimal_and_binary | only [Milli, ..]"
        ))
    };
}

/// The scale of a [unit!] line.
#[doc(hidden)]
#[macro_export]
macro_rules! __unit_scale {
    () => {
        $crate::Magnitude::ONE
    };
    (Pow10 ($e:literal)) => {
        $crate::Magnitude::pow10($e)
    };
    (Affine $($rest:tt)*) => {
        compile_error!(
            "a unit has no offset: a scale with a zero of its own is an origin of points (`origin!`, °C is \
             `origins::Celsius` of kelvins)"
        )
    };
    (Exact ($($m:tt)*)) => {
        $crate::__magnitude!($($m)*)
    };
}

/// The provenance of a [unit!] line: `conventional("<source>", <year>)` after the scale, else defined.
#[doc(hidden)]
#[macro_export]
macro_rules! __unit_provenance {
    () => {
        $crate::Provenance::Defined
    };
    ($source:literal $year:literal) => {
        $crate::Provenance::Conventional {
            source: $source,
            year: $year,
        }
    };
}

/// The factor of a [scale!] line.
#[doc(hidden)]
#[macro_export]
macro_rules! __scale_magnitude {
    (Pow10 ($e:literal)) => {
        $crate::Magnitude::pow10($e)
    };
    (Pow2 ($e:literal)) => {
        $crate::Magnitude::pow2($e)
    };
    (Exact ($($m:tt)*)) => {
        $crate::__magnitude!($($m)*)
    };
}

/// The [DecimalPrefix](crate::DecimalPrefix) impl of a [scale!] line with `prefix P`.
#[doc(hidden)]
#[macro_export]
macro_rules! __scale_decimal {
    ([$($attr:tt)*] $name:ident) => {};
    ([$($attr:tt)*] $name:ident $prefix:ident) => {
        $crate::__with_cfgs! {
            [] $($attr)* =>
            impl $crate::DecimalPrefix for $name {
                const PREFIX: $crate::Prefix = $crate::Prefix::$prefix;
            }
        }
    };
}

/// `Some(Prefix::P)` of a [scale!] line with `prefix P`.
#[doc(hidden)]
#[macro_export]
macro_rules! __scale_runtime {
    () => {
        None
    };
    ($prefix:ident) => {
        Some($crate::Prefix::$prefix)
    };
}

/// `num / den, pow10 a, pow2 b, pi c, ln2 d, ln10 e, mu0 f, g h` as a [Magnitude](crate::Magnitude).
#[doc(hidden)]
#[macro_export]
macro_rules! __magnitude {
    (
        $num:literal / $den:literal $(, pow10 $p10:literal)? $(, pow2 $p2:literal)? $(, pi $pi:literal)?
            $(, ln2 $ln2:literal)? $(, ln10 $ln10:literal)? $(, mu0 $mu0:literal $(/ $mu0d:literal)?)?
            $(, g $g:literal $(/ $gd:literal)?)? $(,)?
    ) => {
        $crate::Magnitude::new(
            $num,
            $den,
            $crate::__or_zero!($($p10)?),
            $crate::__or_zero!($($p2)?),
            $crate::__or_zero!($($pi)?),
        )
        .mul($crate::Magnitude::LN2.powi($crate::__or_zero!($($ln2)?)))
        .mul($crate::Magnitude::LN10.powi($crate::__or_zero!($($ln10)?)))
        .mul($crate::__symbol_pow!(MU0 $(, $mu0 $(/ $mu0d)?)?))
        .mul($crate::__symbol_pow!(G $(, $g $(/ $gd)?)?))
    };
}

/// A symbol of [Magnitude](crate::Magnitude) to a whole power `e` or a half one `e / 2`; 1 without an exponent.
#[doc(hidden)]
#[macro_export]
macro_rules! __symbol_pow {
    ($s:ident) => {
        $crate::Magnitude::ONE
    };
    ($s:ident, $e:literal) => {
        $crate::Magnitude::$s.powi($e)
    };
    ($s:ident, $e:literal / 2) => {
        $crate::Magnitude::$s.pow_x2($e)
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! __or_zero {
    () => {
        0
    };
    ($e:literal) => {
        $e
    };
}
