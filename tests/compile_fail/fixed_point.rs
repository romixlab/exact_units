// `fixed!` is an amount, a `Fixed`: a literal at an origin (`degC`, `@ AbsoluteZero`) is refused at the origin's token
// instead of being read as a number of kelvin.
use exact_units::{Fixed, fixed, prefix::One, units::Kelvin};

fn main() {
    let _: Fixed<Kelvin, One, i16> = fixed!(25 degC);
    let _: Fixed<Kelvin, One, i16> = fixed!(300 K @ AbsoluteZero);
    let _: Fixed<Kelvin, One, i16> = fixed!(25 "°C");
}
