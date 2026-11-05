// An ohm (a resistance) is not a transimpedance V/A, though both are kg·m²·s⁻³·A⁻²: a kind, not a dimension,
// tells them apart (P2401#5nov-0954).
use exact_units::{Fixed, prefix::One, units::{Ampere, Ohm, Per, Volt}};

fn main() {
    let _ = Fixed::<Ohm, One, i32>::new(50).convert::<Per<Volt, Ampere>, One, i32>();
}
