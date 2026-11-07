// `q!`: a symbol of a unit group that is off names the feature it needs (`in` needs `customary`).
use exact_units::{Fixed, prefix::Milli, q, units::Meter};

fn main() {
    let t = Fixed::<Meter, Milli, i16>::new(1);
    let _ = t < q!(5 in);
}
