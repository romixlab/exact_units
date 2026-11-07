// PROTOTYPE `q!`: a symbol that reads two ways is refused, the type names the choice.
use exact_units::{Fixed, prefix::One, q, units::Second};

fn main() {
    let t = Fixed::<Second, One, i16>::new(1);
    let _ = t < q!(5 min);
    let _ = t < q!(5 gal);
}
