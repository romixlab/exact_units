// `fixed!(value symbol-unit)` reads its unit through the same reader as `q!`, so the same refusals apply, each at
// its own token and named `fixed!: ..` rather than `q!: ..` (NUM-17). -0 is refused: a float storage would take it
// as +0.0.
use exact_units::{Fixed, fixed, prefix::One, units::Second};

fn main() {
    let _: Fixed<Second, One, i16> = fixed!(5 kmh);
    let _: Fixed<Second, One, i16> = fixed!(5 min);
    let _: Fixed<Second, One, f32> = fixed!(-0.0 s);
}
