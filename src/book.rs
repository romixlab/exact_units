//! The book's pages (docs/, the site `just docs-build` makes) as doctests: every Rust block on the site runs with
//! `cargo test --doc`, and every `rust,compile_fail` block fails to compile. A page that isn't listed here isn't
//! tested, so each page of zensical.toml's nav with Rust in it has a line.

macro_rules! pages {
    ($($(#[$cfg:meta])* $name:ident = $path:literal;)*) => {$(
        $(#[$cfg])*
        #[doc = include_str!(concat!("../docs/", $path))]
        pub struct $name;
    )*};
}

pages! {
    Index = "index.md";
    Why = "start/why.md";
    Exact = "start/exact.md";
    FirstSteps = "start/first_steps.md";
    Units = "start/units.md";
    Display = "start/display.md";
    Literals = "start/literals.md";
    #[cfg(all(feature = "q", feature = "electrical", feature = "time"))]
    Symbols = "start/symbols.md";
    Firmware = "start/firmware.md";
    Arithmetic = "maths/arithmetic.md";
    Conversions = "maths/conversions.md";
    #[cfg(all(feature = "time", feature = "dimensionless"))]
    Fractions = "maths/fractions.md";
    Bounds = "maths/bounds.md";
    #[cfg(feature = "q")]
    Floats = "maths/floats.md";
    Rounding = "maths/rounding.md";
    Overflow = "maths/overflow.md";
    Quantities = "maths/quantities.md";
    #[cfg(feature = "systems")]
    Systems = "maths/systems.md";
    Niches = "maths/niches.md";
    QFormats = "fixed_point/q_formats.md";
    Bam = "fixed_point/bam.md";
    Products = "fixed_point/products.md";
    #[cfg(feature = "dimensionless")]
    Codecs = "fixed_point/codecs.md";
    #[cfg(all(feature = "dimensionless", feature = "photometric", feature = "libm"))]
    Video = "fixed_point/video.md";
    Points = "points/points.md";
    Wrapping = "points/wrapping.md";
    Registers = "points/registers.md";
    Indices = "points/indices.md";
    #[cfg(feature = "leap_seconds")]
    TimeScales = "points/time.md";
    Decibels = "levels/decibels.md";
    Thresholds = "levels/thresholds.md";
    #[cfg(all(feature = "levels", feature = "time", feature = "angle", feature = "counting"))]
    Damping = "levels/damping.md";
    #[cfg(all(feature = "levels", feature = "mechanical", feature = "time", feature = "angle"))]
    Rates = "levels/rates.md";
    Complex = "complex/complex.md";
    Cordic = "complex/cordic.md";
    RfImpedance = "rf/impedance.md";
    RfReflection = "rf/reflection.md";
    RfPower = "rf/power.md";
    RfLevels = "rf/levels.md";
    RfIq = "rf/iq.md";
    RfMatching = "rf/matching.md";
    #[cfg(feature = "customary")]
    Customary = "customary.md";
    Declare = "units/declare.md";
    #[cfg(all(feature = "electrical", feature = "time", feature = "mechanical", feature = "information", feature = "levels"))]
    Names = "units/names.md";
    #[cfg(feature = "shrink_wrap")]
    Wire = "wire/shrink_wrap.md";
    Sensors = "recipes/sensors.md";
    Timing = "recipes/timing.md";
    Magnitude = "nomicon/magnitude.md";
    Constants = "nomicon/constants.md";
}
