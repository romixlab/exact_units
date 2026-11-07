// `q!`: °C is a point at the Celsius origin, never a plain amount of kelvin.
use exact_units::{Fixed, prefix::One, q, units::Kelvin};

fn main() {
    let t = Fixed::<Kelvin, One, i16>::new(300);
    let _ = t < q!(25 degC);
    let _ = t < q!(25 "°C");
    let _ = t < q!(300 K @ AbsoluteZero);
}
