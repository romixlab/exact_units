// `q!`: a literal in a length doesn't compare with a speed.
use exact_units::{Fixed, prefix::Centi, q, units::Speed};

fn main() {
    let speed = Fixed::<Speed, Centi, i16>::new(9_999);
    let _ = speed < q!(100 m);
}
