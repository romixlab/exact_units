//! A crate with its own `Rpm`, for exact_units' two-crate test (`tests/two_crates.rs`): rpm_a and rpm_b declare the
//! same line, so their units print the same and have the same definition, under two identities.
#![no_std]

exact_units::unit! {
    @table
    /// The units of this crate, as a `UnitTable`.
    pub static UNITS;

    /// Shaft speed in revolutions per minute: 2π rad / 60 s.
    pub struct Rpm = "rpm", [rad 1, s -1], Exact(1 / 30, pi 1);
}
