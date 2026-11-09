//! The product table: which units multiply and divide into which (`Volt · Ampere = Watt`), as [UnitMul] and
//! [UnitDiv] impls. Each line is checked against the dimensions of its units at compile time, so the table can't be
//! wrong.

/// A ratio keeps the unit: 48 V · 0.5 duty = 24 V.
#[cfg(feature = "dimensionless")]
impl<U: crate::Unit> crate::UnitMul<crate::units::Ratio> for U {
    type Output = U;
}

/// A ratio keeps the unit: 24 V / 0.5 = 48 V.
#[cfg(feature = "dimensionless")]
impl<U: crate::Unit> crate::UnitDiv<crate::units::Ratio> for U {
    type Output = U;
}

/// One side of a [ProductEntry]: a unit's real [Dim](crate::Dim) and [Kind](crate::Kind), the two things
/// [Fixed::convert](crate::Fixed::convert) itself checks.
pub(crate) type Side = (crate::Dim<'static>, crate::Kind<'static>);

/// One line of the product table (`A * B = O`), its three sides' real `Dim` and `Kind` read straight off the unit
/// types: what [TABLE] holds, so [known_angle_step](crate::conv_error::known_angle_step) can scan the table
/// itself for an angle-crossing step instead of keeping a separate list that could drift from it.
pub(crate) struct ProductEntry {
    pub(crate) a: Side,
    pub(crate) b: Side,
    pub(crate) o: Side,
}

/// `A * B = C;` gives `A · B`, `B · A`, `C / A` and `C / B`, and checks at compile time that the SI exponents of `A`
/// and `B` add up to those of `C` and that none of them has an offset scale (°C). Each line's `#[cfg(..)]` must
/// come before any doc comment on it (a macro_rules limitation: a repeated `#[doc = ..]` ahead of another meta is
/// ambiguous), and also gates that line's row of [TABLE].
macro_rules! products {
    ($(#[$cfg:meta] $(#[doc = $doc:literal])* $a:ident * $b:ident = $o:ident;)*) => {
        $(
            #[$cfg]
            $(#[doc = $doc])*
            impl crate::UnitMul<crate::units::$b> for crate::units::$a {
                type Output = crate::units::$o;
            }
            #[$cfg]
            $(#[doc = $doc])*
            impl crate::UnitMul<crate::units::$a> for crate::units::$b {
                type Output = crate::units::$o;
            }
            #[$cfg]
            $(#[doc = $doc])*
            impl crate::UnitDiv<crate::units::$a> for crate::units::$o {
                type Output = crate::units::$b;
            }
            #[$cfg]
            $(#[doc = $doc])*
            impl crate::UnitDiv<crate::units::$b> for crate::units::$o {
                type Output = crate::units::$a;
            }
            #[$cfg]
            $(#[doc = $doc])*
            const _: () = {
                assert!(
                    <crate::units::Times<crate::units::$a, crate::units::$b> as crate::Unit>::DIM
                        .same_typed(<crate::units::$o as crate::Unit>::DIM),
                    concat!(stringify!($a * $b = $o), ": SI exponents differ")
                );
            };
        )*

        /// Every line of the macro call below, as a [ProductEntry] each, in the same order, each still behind its
        /// own `#[cfg(..)]`.
        pub(crate) const TABLE: &[ProductEntry] = &[
            $(
                #[$cfg]
                ProductEntry {
                    a: (
                        <crate::units::$a as crate::Unit>::DIM,
                        <crate::units::$a as crate::Unit>::KIND,
                    ),
                    b: (
                        <crate::units::$b as crate::Unit>::DIM,
                        <crate::units::$b as crate::Unit>::KIND,
                    ),
                    o: (
                        <crate::units::$o as crate::Unit>::DIM,
                        <crate::units::$o as crate::Unit>::KIND,
                    ),
                },
            )*
        ];
    };
}

products! {
    #[cfg(feature = "electrical")]
    Volt * Ampere = Watt;
    #[cfg(feature = "electrical")]
    Ampere * Ohm = Volt;
    #[cfg(feature = "electrical")]
    Henry * Ampere = Weber;
    #[cfg(feature = "electrical")]
    Farad * Volt = Coulomb;
    #[cfg(all(feature = "electrical", feature = "time"))]
    Ampere * Second = Coulomb;
    #[cfg(all(feature = "electrical", feature = "time"))]
    Volt * Second = Weber;
    #[cfg(all(feature = "electrical", feature = "time", feature = "mechanical"))]
    Watt * Second = Joule;
    #[cfg(all(feature = "electrical", feature = "mechanical", feature = "angle"))]
    Torque * AngularVelocity = Watt;
    #[cfg(all(feature = "mechanical", feature = "angle"))]
    Torque * Radian = Joule;
    #[cfg(all(feature = "mechanical", feature = "angle"))]
    MomentOfInertia * AngularVelocity = AngularMomentum;
    #[cfg(all(feature = "mechanical", feature = "angle"))]
    MomentOfInertia * AngularAcceleration = Torque;
    #[cfg(all(feature = "mechanical", feature = "angle", feature = "time"))]
    Torque * Second = AngularMomentum;
    #[cfg(all(feature = "angle", feature = "time"))]
    AngularVelocity * Second = Radian;
    #[cfg(all(feature = "time", feature = "angle"))]
    /// ```
    /// # #[cfg(all(feature = "time", feature = "angle"))] {
    /// use exact_units::{Fixed, prefix::One, units::{AngularVelocity, Hertz, Radian}};
    ///
    /// let f = Fixed::<Hertz, One, i32>::new(50);
    /// let one_rad = Fixed::<Radian, One, i32>::new(1);
    /// // 50 Hz · 1 rad is 50 of Hz·rad, a named step away from rad/s
    /// assert_eq!((f * one_rad).convert::<AngularVelocity, One, i32>(), Some(Fixed::new(50)));
    /// # }
    /// ```
    Hertz * Radian = AngularVelocity;
    #[cfg(all(feature = "time", feature = "angle"))]
    HertzPerSecond * Radian = AngularAcceleration;
    #[cfg(all(feature = "mechanical", feature = "time"))]
    Speed * Second = Meter;
    #[cfg(all(feature = "mechanical", feature = "time"))]
    Acceleration * Second = Speed;
    #[cfg(feature = "mechanical")]
    Gram * Acceleration = Newton;
    #[cfg(feature = "mechanical")]
    Newton * Meter = Joule;
    #[cfg(all(feature = "time", feature = "dimensionless"))]
    Hertz * Second = Ratio;
    #[cfg(feature = "time")]
    HertzPerSecond * Second = Hertz;
    #[cfg(all(feature = "photometric", feature = "angle"))]
    /// ```
    /// # #[cfg(all(feature = "photometric", feature = "angle"))] {
    /// use exact_units::{Fixed, prefix::One, units::{Candela, Lumen, Steradian}};
    ///
    /// let i = Fixed::<Candela, One, i32>::new(120);
    /// let sr = Fixed::<Steradian, One, i32>::new(2);
    /// // a luminous flux is its intensity times the solid angle it fills
    /// assert_eq!((i * sr).convert::<Lumen, One, i32>(), Some(Fixed::new(240)));
    /// # }
    /// ```
    Candela * Steradian = Lumen;
    #[cfg(all(feature = "radiometric", feature = "angle", feature = "electrical"))]
    RadiantIntensity * Steradian = Watt;
}
