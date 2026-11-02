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
    FirstSteps = "start/first_steps.md";
    Units = "start/units.md";
    Display = "start/display.md";
    Literals = "start/literals.md";
    Firmware = "start/firmware.md";
    Arithmetic = "maths/arithmetic.md";
    Conversions = "maths/conversions.md";
    Bounds = "maths/bounds.md";
    Rounding = "maths/rounding.md";
    Overflow = "maths/overflow.md";
    Kinds = "maths/kinds.md";
    Niches = "maths/niches.md";
    QFormats = "fixed_point/q_formats.md";
    Bam = "fixed_point/bam.md";
    Products = "fixed_point/products.md";
    Points = "points/points.md";
    Wrapping = "points/wrapping.md";
    Registers = "points/registers.md";
    Indices = "points/indices.md";
    Decibels = "levels/decibels.md";
    Thresholds = "levels/thresholds.md";
    #[cfg(all(feature = "levels", feature = "time", feature = "angle", feature = "counting"))]
    Damping = "levels/damping.md";
    Complex = "complex/complex.md";
    Iq = "complex/iq.md";
    Cordic = "complex/cordic.md";
    #[cfg(feature = "customary")]
    Customary = "customary.md";
    Declare = "units/declare.md";
    #[cfg(feature = "shrink_wrap")]
    Wire = "wire/shrink_wrap.md";
    Sensors = "recipes/sensors.md";
    Timing = "recipes/timing.md";
    Magnitude = "nomicon/magnitude.md";
}
