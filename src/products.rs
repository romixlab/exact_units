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

/// `A * B = C;` gives `A · B`, `B · A`, `C / A` and `C / B`, and checks at compile time that the SI exponents of `A`
/// and `B` add up to those of `C` and that none of them has an offset scale (°C).
macro_rules! products {
    ($($(#[$meta:meta])* $a:ident * $b:ident = $o:ident;)*) => {
        $(
            $(#[$meta])*
            impl crate::UnitMul<crate::units::$b> for crate::units::$a {
                type Output = crate::units::$o;
            }
            $(#[$meta])*
            impl crate::UnitMul<crate::units::$a> for crate::units::$b {
                type Output = crate::units::$o;
            }
            $(#[$meta])*
            impl crate::UnitDiv<crate::units::$a> for crate::units::$o {
                type Output = crate::units::$b;
            }
            $(#[$meta])*
            impl crate::UnitDiv<crate::units::$b> for crate::units::$o {
                type Output = crate::units::$a;
            }
            $(#[$meta])*
            const _: () = {
                assert!(
                    <crate::units::$a as crate::Unit>::DIM
                        .mul(<crate::units::$b as crate::Unit>::DIM)
                        .same(<crate::units::$o as crate::Unit>::DIM),
                    concat!(stringify!($a * $b = $o), ": SI exponents differ")
                );
                assert!(
                    !<crate::units::$a as crate::Unit>::EXPR.has_offset()
                        && !<crate::units::$b as crate::Unit>::EXPR.has_offset()
                        && !<crate::units::$o as crate::Unit>::EXPR.has_offset(),
                    concat!(stringify!($a * $b = $o), ": a unit with an offset scale can't be in a product")
                );
            };
        )*
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
    #[cfg(all(feature = "angle", feature = "time"))]
    AngularVelocity * Second = Radian;
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
}
