// `fixed!(value symbol-unit)`: a literal in a length doesn't compile into a speed either (NUM-17).
use exact_units::{Fixed, fixed, prefix::Centi, units::Speed};

fn main() {
    let _: Fixed<Speed, Centi, i16> = fixed!(100 m);
}
