// `q!`: a float storage checks the literal's unit as an integer one does, and a literal of 0 has its unit checked
// too.
use exact_units::{Fixed, prefix::Centi, q, units::Speed};

fn main() {
    let speed = Fixed::<Speed, Centi, f32>::new(99.99);
    let _ = speed < q!(100 m);
    let speed = Fixed::<Speed, Centi, i16>::new(9_999);
    let _ = speed < q!(0 m);
}
