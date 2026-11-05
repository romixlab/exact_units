// `fixed!(.. in S)` takes a literal in another unit `S` of the same dimension and kind as the context's: a torque
// is not a joule here either (P2401#5nov-0954).
use exact_units::{Fixed, fixed, prefix::One, units::{Joule, Torque}};

fn main() {
    let _: Fixed<Joule, One, i32> = fixed!(5 in Torque);
}
