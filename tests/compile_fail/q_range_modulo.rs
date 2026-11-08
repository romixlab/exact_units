// A `q!` range of points that wrap doesn't compile: they have no order, as `<` with one point literal has none.
use exact_units::{Fixed, Point, origins::Angle, prefix::Centi, q, units::Degree};

fn main() {
    let a = Point::<Angle<Degree>, Fixed<Degree, Centi, i16>>::from_raw(0);
    let _ = a.is_in(q!(0 @ Angle<Degree> ..= 90 @ Angle<Degree>));
}
