// A `pass()` fixture next to the `compile_fail()` ones below: trybuild runs `cargo build` (not `check`) for the
// whole batch only when at least one fixture must pass, and these refused conversions panic only post-monomorphization
// (`cargo check` alone never reaches it, see P2401#5nov-0954's ready note).
use exact_units::{Fixed, prefix::One, units::Joule};

fn main() {
    let j = Fixed::<Joule, One, i32>::new(5);
    assert_eq!(j.convert::<Joule, One, i32>(), Some(j));
}
