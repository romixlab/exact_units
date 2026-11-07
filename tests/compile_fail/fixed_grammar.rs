// `fixed!(value symbol-unit)` reads its unit through the same reader as `q!`, so the same refusals apply, each at
// its own token and named `fixed!: ..` rather than `q!: ..` (NUM-17).
use exact_units::{Fixed, fixed, prefix::One, units::Second};

fn main() {
    let _: Fixed<Second, One, i16> = fixed!(5 kmh);
    let _: Fixed<Second, One, i16> = fixed!(5 min);
}
