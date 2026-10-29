//! Kinds of quantity that share a dimension: a resistance is not a transimpedance, a voltage gain is not a duty cycle,
//! though each pair has one [Dim]. A unit's [Kind] is part of its type next to the dimension, so
//! [Fixed::convert](crate::Fixed::convert) refuses to cross kinds at compile time, at no cost at run time.
//!
//! The rules follow ISO 80000-1's quantity kinds as mp-units implements them (`kind_of`, the kind of a product or
//! quotient), simplified to what a type carries:
//!
//! - A unit is [Plain] (only its dimension) unless its `unit!` line names a kind (`kind "resistance"`).
//! - A quotient of two quantities of one dimension keeps what cancelled: V/V is a ratio of voltage
//!   ([RatioOf]), A/A one of current, W/W one of power; none of them is a plain [Ratio](crate::units::Ratio).
//! - A dimension-one factor or divisor keeps the other side's kind: V · 0.5 is V, Ω / (V/V) is Ω. Two ratios of
//!   one dimension stay one: V/V · V/V and (V/V) / (V/V) are voltage gains.
//! - Any other product or quotient is [Plain]: V/V · V is a plain V, Ω · A a plain V, and V/A a plain V/A, a
//!   transimpedance that is not a resistance.
//!
//! Crossing kinds is a named step (`as_resistance`, `as_ratio`, `to_hertz`), never a conversion.
//!
//! [Plain]: Kind::Plain
//! [RatioOf]: Kind::RatioOf

use crate::Dim;
use core::fmt;

/// The kind of quantity a unit measures, beyond its dimension. See the [module](crate::kind) for the rules.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum Kind<'i> {
    /// No kind beyond the dimension: two plain units of one dimension convert (N·m/s and W).
    Plain,
    /// A kind of its own, by its ISO 80000 name: `"resistance"` for [Ohm](crate::units::Ohm), `"conductance"` for
    /// [Siemens](crate::units::Siemens).
    Named(&'i str),
    /// Dimension one, a ratio of two quantities of this dimension: a voltage gain V/V is `RatioOf` the volt's
    /// dimension.
    RatioOf(Dim<'i>),
}

/// How a ratio of a kind is put in decibels: 10·lg for a ratio of powers, 20·lg for one of root-power quantities
/// (IEC 60027-3, ISO 80000-3 Annex C).
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum LevelForm {
    /// Power, energy, intensity: 10·lg(P/P₀) dB.
    Power,
    /// Voltage, current, pressure, field strength, whose square is proportional to a power: 20·lg(V/V₀) dB.
    RootPower,
}

impl LevelForm {
    /// Decibels per decade of the ratio: 10 or 20.
    pub const fn db_per_decade(self) -> f64 {
        match self {
            LevelForm::Power => 10.0,
            LevelForm::RootPower => 20.0,
        }
    }
}

/// Dimensions of power quantities: W, J, W/m².
const POWER: [Dim<'static>; 3] = [
    Dim::new(-3, 2, 1, 0, 0, 0, 0),
    Dim::new(-2, 2, 1, 0, 0, 0, 0),
    Dim::new(-3, 0, 1, 0, 0, 0, 0),
];

/// Dimensions of root-power quantities: V, A, Pa, V/m, A/m.
const ROOT_POWER: [Dim<'static>; 5] = [
    Dim::new(-3, 2, 1, -1, 0, 0, 0),
    Dim::new(0, 0, 0, 1, 0, 0, 0),
    Dim::new(-2, -1, 1, 0, 0, 0, 0),
    Dim::new(-3, 1, 1, -1, 0, 0, 0),
    Dim::new(0, -1, 0, 1, 0, 0, 0),
];

const fn any_same(dims: &[Dim<'static>], d: Dim<'_>) -> bool {
    let mut i = 0;
    while i < dims.len() {
        if dims[i].same(d) {
            return true;
        }
        i += 1;
    }
    false
}

/// `==` on two names in `const` code.
const fn same_str(a: &str, b: &str) -> bool {
    let (a, b) = (a.as_bytes(), b.as_bytes());
    if a.len() != b.len() {
        return false;
    }
    let mut i = 0;
    while i < a.len() {
        if a[i] != b[i] {
            return false;
        }
        i += 1;
    }
    true
}

impl<'i> Kind<'i> {
    /// `==` in `const` code.
    pub const fn same(self, o: Kind<'_>) -> bool {
        match (self, o) {
            (Kind::Plain, Kind::Plain) => true,
            (Kind::Named(a), Kind::Named(b)) => same_str(a, b),
            (Kind::RatioOf(a), Kind::RatioOf(b)) => a.same(b),
            _ => false,
        }
    }

    /// The kind of `a · b`, for quantities `a` of kind `self` and dimension `a_dim`, `b` of kind `o` and `b_dim`.
    pub const fn mul(self, a_dim: Dim<'i>, o: Kind<'i>, b_dim: Dim<'i>) -> Kind<'i> {
        Self::combine(self, a_dim, o, b_dim)
    }

    /// The kind of `a / b`, for quantities `a` of kind `self` and dimension `a_dim`, `b` of kind `o` and `b_dim`.
    pub const fn div(self, a_dim: Dim<'i>, o: Kind<'i>, b_dim: Dim<'i>) -> Kind<'i> {
        // what cancels is kept: V/V is a ratio of voltage
        if !a_dim.is_one() && a_dim.same(b_dim) {
            return Kind::RatioOf(a_dim);
        }
        Self::combine(self, a_dim, o, b_dim)
    }

    /// What a product and a quotient have in common: a plain dimension-one side keeps the other's kind, two ratios
    /// of one dimension stay one, a ratio with a quantity of a dimension gives the quantity's kind.
    const fn combine(a: Kind<'i>, a_dim: Dim<'i>, b: Kind<'i>, b_dim: Dim<'i>) -> Kind<'i> {
        match (a, b) {
            (Kind::RatioOf(x), Kind::RatioOf(y)) if x.same(y) => a,
            (Kind::RatioOf(_), Kind::RatioOf(_)) => Kind::Plain,
            (Kind::Plain | Kind::RatioOf(_), _) if a_dim.is_one() => b,
            (_, Kind::Plain | Kind::RatioOf(_)) if b_dim.is_one() => a,
            _ => Kind::Plain,
        }
    }

    /// The kind of `√a`: plain (`√Hz`).
    pub const fn sqrt(self) -> Kind<'i> {
        Kind::Plain
    }

    /// How a quantity of this kind is put in decibels: a ratio of a power or root-power dimension (W/W, V/V, A/A),
    /// `None` for any other kind (a plain [Ratio](crate::units::Ratio) has no level form: 0.5 duty is not −3 dB).
    pub const fn level_form(self) -> Option<LevelForm> {
        match self {
            Kind::RatioOf(d) if any_same(&POWER, d) => Some(LevelForm::Power),
            Kind::RatioOf(d) if any_same(&ROOT_POWER, d) => Some(LevelForm::RootPower),
            _ => None,
        }
    }
}

/// `resistance`, `ratio of kg·m²·s⁻³·A⁻¹`; nothing for a plain kind.
impl fmt::Display for Kind<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Kind::Plain => Ok(()),
            Kind::Named(n) => f.write_str(n),
            Kind::RatioOf(d) => write!(f, "ratio of {d}"),
        }
    }
}

/// The kinds a `Fixed` converts between and the ones it doesn't: a scale converts within a kind, a kind is crossed only
/// by a named step.
///
/// ```
/// # #[cfg(all(feature = "electrical", feature = "dimensionless"))] {
/// use exact_units::{Fixed, prefix::{Milli, One}, units::{Ampere, Ohm, Per, Ratio, Volt}};
///
/// // mV/V to V/V: the same kind, another scale
/// let gain = Fixed::<Per<Volt, Volt>, Milli, u32>::new(2500);
/// assert_eq!(gain.convert::<Per<Volt, Volt>, One, f32>(), Some(Fixed::new(2.5)));
/// // a voltage gain is a ratio, a transimpedance a resistance, when said so
/// let _: Fixed<Ratio, Milli, u32> = gain.as_ratio();
/// let r: Fixed<Ohm, One, u16> = Fixed::<Per<Volt, Ampere>, One, u16>::new(50).as_resistance();
/// assert_eq!(r.get(), 50);
/// # }
/// ```
///
/// A voltage gain is not a current gain:
///
/// ```compile_fail
/// use exact_units::{Fixed, prefix::One, units::{Ampere, Per, Volt}};
/// let _ = Fixed::<Per<Volt, Volt>, One, f32>::new(2.0).convert::<Per<Ampere, Ampere>, One, f32>();
/// ```
///
/// nor a power gain:
///
/// ```compile_fail
/// use exact_units::{Fixed, prefix::One, units::{Per, Volt, Watt}};
/// let _ = Fixed::<Per<Volt, Volt>, One, f32>::new(2.0).convert::<Per<Watt, Watt>, One, f32>();
/// ```
///
/// nor a plain ratio, a duty cycle:
///
/// ```compile_fail
/// use exact_units::{Fixed, prefix::One, units::{Per, Ratio, Volt}};
/// let _ = Fixed::<Per<Volt, Volt>, One, f32>::new(2.0).convert::<Ratio, One, f32>();
/// ```
///
/// and a duty cycle is no gain:
///
/// ```compile_fail
/// use exact_units::{Fixed, prefix::One, units::{Per, Ratio, Volt}};
/// let _ = Fixed::<Ratio, One, f32>::new(0.5).convert::<Per<Volt, Volt>, One, f32>();
/// ```
///
/// A transimpedance V/A is not a resistance, nor a resistance a transimpedance:
///
/// ```compile_fail
/// use exact_units::{Fixed, prefix::One, units::{Ampere, Ohm, Per, Volt}};
/// let _ = Fixed::<Per<Volt, Ampere>, One, f32>::new(50.0).convert::<Ohm, One, f32>();
/// ```
///
/// ```compile_fail
/// use exact_units::{Fixed, prefix::One, units::{Ampere, Ohm, Per, Volt}};
/// let _ = Fixed::<Ohm, One, f32>::new(50.0).convert::<Per<Volt, Ampere>, One, f32>();
/// ```
///
/// A transconductance A/V is not a conductance:
///
/// ```compile_fail
/// use exact_units::{Fixed, prefix::One, units::{Ampere, Per, Siemens, Volt}};
/// let _ = Fixed::<Per<Ampere, Volt>, One, f32>::new(0.02).convert::<Siemens, One, f32>();
/// ```
///
/// A gain times a voltage is a plain voltage, and converts like one:
///
/// ```
/// # #[cfg(feature = "electrical")] {
/// use exact_units::{Fixed, prefix::{Milli, One}, units::{Per, Times, Volt}};
/// let v = Fixed::<Times<Per<Volt, Volt>, Volt>, One, i32>::new(3);
/// assert_eq!(v.convert::<Volt, Milli, i32>(), Some(Fixed::new(3000)));
/// # }
/// ```
#[cfg(all(feature = "electrical", feature = "dimensionless"))]
#[allow(dead_code)]
struct KindsInConversions;

#[cfg(feature = "electrical")]
mod electrical {
    use crate::units::{Ampere, Ohm, Per, Siemens, Volt};
    use crate::{Bounds, Fixed, Storage};

    /// A transimpedance (an amplifier's V out per A in) is not a resistance: these are the named steps between them.
    impl<P, T: Storage, B: Bounds<T>> Fixed<Per<Volt, Ampere>, P, T, B> {
        /// The same number as a resistance in Ω.
        pub fn as_resistance(self) -> Fixed<Ohm, P, T, B> {
            Fixed::new_unchecked(self.get())
        }
    }

    impl<P, T: Storage, B: Bounds<T>> Fixed<Ohm, P, T, B> {
        /// The same number as a transimpedance in V/A.
        pub fn as_transimpedance(self) -> Fixed<Per<Volt, Ampere>, P, T, B> {
            Fixed::new_unchecked(self.get())
        }
    }

    /// A transconductance (A out per V in) is not a conductance: these are the named steps between them.
    impl<P, T: Storage, B: Bounds<T>> Fixed<Per<Ampere, Volt>, P, T, B> {
        /// The same number as a conductance in S.
        pub fn as_conductance(self) -> Fixed<Siemens, P, T, B> {
            Fixed::new_unchecked(self.get())
        }
    }

    impl<P, T: Storage, B: Bounds<T>> Fixed<Siemens, P, T, B> {
        /// The same number as a transconductance in A/V.
        pub fn as_transconductance(self) -> Fixed<Per<Ampere, Volt>, P, T, B> {
            Fixed::new_unchecked(self.get())
        }
    }
}

#[cfg(feature = "dimensionless")]
mod ratio {
    use crate::units::{Per, Ratio};
    use crate::{Bounds, Fixed, Storage, Unit};

    /// A gain `U/U` (V/V, A/A, W/W) is not a plain [Ratio], a duty cycle: these are the named steps between them.
    impl<U: Unit, P, T: Storage, B: Bounds<T>> Fixed<Per<U, U>, P, T, B> {
        /// The same number as a plain ratio: what the gain was a ratio of is gone.
        pub fn as_ratio(self) -> Fixed<Ratio, P, T, B> {
            Fixed::new_unchecked(self.get())
        }
    }

    impl<P, T: Storage, B: Bounds<T>> Fixed<Ratio, P, T, B> {
        /// The same number as a gain `U/U`: `as_gain::<Volt>()` is a voltage gain.
        pub fn as_gain<U: Unit>(self) -> Fixed<Per<U, U>, P, T, B> {
            Fixed::new_unchecked(self.get())
        }
    }
}

/// Levels in decibels of a gain, the form (10·lg or 20·lg) from its kind.
#[cfg(feature = "std")]
impl<U: crate::Unit, P: crate::FixedScale, T: crate::Storage, B: crate::Bounds<T>>
    crate::Fixed<U, P, T, B>
{
    /// The form a level of `U` takes, or a compile error for a kind without one.
    const LEVEL_FORM: LevelForm = match U::KIND.level_form() {
        Some(form) => form,
        None => panic!(
            "decibels are for a ratio of powers (W/W) or of root-power quantities (V/V, A/A, Pa/Pa): a plain \
             Ratio or another kind has no level form"
        ),
    };

    /// The gain in dB: 20·lg for a ratio of root-power quantities (V/V, A/A), 10·lg for one of powers (W/W). A
    /// kind without a level form (a plain [Ratio](crate::units::Ratio), a duty cycle) doesn't compile.
    ///
    /// ```
    /// # #[cfg(feature = "electrical")] {
    /// use exact_units::{Fixed, prefix::{Milli, One}, units::{Ampere, Per, Volt, Watt}};
    /// assert_eq!(Fixed::<Per<Volt, Volt>, One, f64>::new(10.0).to_db(), 20.0);
    /// assert_eq!(Fixed::<Per<Ampere, Ampere>, Milli, u32>::new(100).to_db(), -20.0);
    /// assert_eq!(Fixed::<Per<Watt, Watt>, One, f64>::new(100.0).to_db(), 20.0);
    /// # }
    /// ```
    ///
    /// ```compile_fail
    /// use exact_units::{Fixed, prefix::One, units::Ratio};
    /// let _ = Fixed::<Ratio, One, f64>::new(0.5).to_db();
    /// ```
    pub fn to_db(self) -> f64 {
        let form = const { Self::LEVEL_FORM };
        let x = match self.get().to_repr() {
            crate::Repr::Int(i) => i as f64,
            crate::Repr::Float(x) => x,
        };
        let x = x * const { P::SCALE.to_f64() * U::EXPR.factor() };
        form.db_per_decade() * x.log10()
    }

    /// The gain of a level in dB, by the form of [to_db](Self::to_db): `None` where it isn't a number of `T` (an
    /// integer storage takes whole numbers only) or not within `B`.
    ///
    /// ```
    /// # #[cfg(feature = "electrical")] {
    /// use exact_units::{Fixed, prefix::{Milli, One}, units::{Per, Volt, Watt}};
    /// assert_eq!(Fixed::<Per<Volt, Volt>, Milli, u32>::from_db(40.0), Some(Fixed::new(100_000)));
    /// assert_eq!(Fixed::<Per<Watt, Watt>, One, f64>::from_db(30.0), Some(Fixed::new(1000.0)));
    /// # }
    /// ```
    pub fn from_db(db: f64) -> Option<Self> {
        let form = const { Self::LEVEL_FORM };
        let x = 10f64.powf(db / form.db_per_decade());
        let x = x / const { P::SCALE.to_f64() * U::EXPR.factor() };
        Self::try_new(T::from_float(x)?).ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const V: Dim<'static> = Dim::new(-3, 2, 1, -1, 0, 0, 0);
    const A: Dim<'static> = Dim::new(0, 0, 0, 1, 0, 0, 0);
    const W: Dim<'static> = Dim::new(-3, 2, 1, 0, 0, 0, 0);
    const OHM: Dim<'static> = Dim::new(-3, 2, 1, -2, 0, 0, 0);
    const ONE: Dim<'static> = Dim::ONE;

    #[test]
    fn a_quotient_of_one_dimension_keeps_it() {
        let vv = Kind::Plain.div(V, Kind::Plain, V);
        assert_eq!(vv, Kind::RatioOf(V));
        assert!(!vv.same(Kind::Plain.div(A, Kind::Plain, A)));
        assert!(!vv.same(Kind::Plain));
        // also of two kinds of the dimension
        assert_eq!(
            Kind::Named("resistance").div(OHM, Kind::Plain, OHM),
            Kind::RatioOf(OHM)
        );
    }

    #[test]
    fn products_cancel_kinds() {
        let vv = Kind::RatioOf(V);
        // V/V · V is a plain V
        assert_eq!(vv.mul(ONE, Kind::Plain, V), Kind::Plain);
        // V/V · V/V and (V/V) / (V/V) are voltage gains
        assert_eq!(vv.mul(ONE, vv, ONE), vv);
        assert_eq!(vv.div(ONE, vv, ONE), vv);
        // A/A · V/V has no kind of its own
        assert_eq!(Kind::RatioOf(A).mul(ONE, vv, ONE), Kind::Plain);
        // a gain or a ratio scales a resistance, a resistance times a current is a plain V
        let r = Kind::Named("resistance");
        assert_eq!(vv.mul(ONE, r, OHM), r);
        assert_eq!(r.div(OHM, Kind::Plain, ONE), r);
        assert_eq!(r.mul(OHM, Kind::Plain, A), Kind::Plain);
        // V/A is no resistance
        assert_eq!(Kind::Plain.div(V, Kind::Plain, A), Kind::Plain);
    }

    #[test]
    fn level_forms() {
        assert_eq!(Kind::RatioOf(V).level_form(), Some(LevelForm::RootPower));
        assert_eq!(Kind::RatioOf(A).level_form(), Some(LevelForm::RootPower));
        assert_eq!(Kind::RatioOf(W).level_form(), Some(LevelForm::Power));
        assert_eq!(Kind::RatioOf(OHM).level_form(), None);
        assert_eq!(Kind::Plain.level_form(), None);
    }

    #[test]
    fn shown() {
        assert_eq!(std::format!("{}", Kind::Named("resistance")), "resistance");
        assert_eq!(std::format!("{}", Kind::Plain), "");
        assert!(std::format!("{}", Kind::RatioOf(A)).starts_with("ratio of "));
    }
}
