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
    #[cfg(all(feature = "time", feature = "dimensionless", feature = "nuclear"))]
    Uncertainty = "maths/uncertainty.md";
    Bounds = "maths/bounds.md";
    #[cfg(feature = "time")]
    RunTimeScales = "maths/run_time_scales.md";
    #[cfg(feature = "q")]
    Floats = "maths/floats.md";
    Rounding = "maths/rounding.md";
    Overflow = "maths/overflow.md";
    Quantities = "maths/quantities.md";
    #[cfg(feature = "radiation")]
    Doses = "maths/doses.md";
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
    #[cfg(all(
        feature = "radiation",
        feature = "nuclear",
        feature = "systems",
        feature = "chemical",
        feature = "dimensionless",
        feature = "mechanical",
        feature = "time"
    ))]
    Nuclear = "recipes/nuclear.md";
    #[cfg(feature = "time")]
    VideoTime = "recipes/video_time.md";
    #[cfg(all(feature = "dimensionless", feature = "information", feature = "time"))]
    ImageCodecs = "recipes/image_codecs.md";
    #[cfg(all(feature = "levels", feature = "photometric", feature = "time"))]
    Photography = "recipes/photography.md";
    RfRecipes = "recipes/rf/index.md";
    #[cfg(all(feature = "electrical", feature = "levels", feature = "time", feature = "mechanical", any(feature = "std", feature = "libm")))]
    RfMismatch = "recipes/rf/mismatch.md";
    #[cfg(all(feature = "electrical", feature = "levels", feature = "time", feature = "mechanical", any(feature = "std", feature = "libm")))]
    RfLinkBudget = "recipes/rf/link_budget.md";
    #[cfg(all(feature = "electrical", feature = "levels", feature = "time", feature = "mechanical", any(feature = "std", feature = "libm")))]
    RfNoiseCascade = "recipes/rf/noise_cascade.md";
    #[cfg(all(feature = "electrical", feature = "levels", feature = "time", feature = "mechanical", any(feature = "std", feature = "libm")))]
    RfFractionalN = "recipes/rf/fractional_n.md";
    #[cfg(all(feature = "electrical", feature = "levels", feature = "time", feature = "mechanical", any(feature = "std", feature = "libm")))]
    RfTwoPorts = "recipes/rf/two_ports.md";
    #[cfg(all(feature = "electrical", feature = "levels", feature = "time", feature = "mechanical", any(feature = "std", feature = "libm")))]
    RfFreeSpace = "recipes/rf/free_space.md";
    #[cfg(all(feature = "electrical", feature = "levels", feature = "time", feature = "mechanical", any(feature = "std", feature = "libm")))]
    RfSkinDepth = "recipes/rf/skin_depth.md";
    #[cfg(all(feature = "electrical", feature = "levels", feature = "time", feature = "mechanical", any(feature = "std", feature = "libm")))]
    RfPhaseNoise = "recipes/rf/phase_noise.md";
    Magnitude = "nomicon/magnitude.md";
    Constants = "nomicon/constants.md";
}
