// `q!` point literals: a reading on a scale is never an amount, points that wrap have no order, gauge and absolute
// pressure don't compare, and the reader refuses a second origin and an origin inside a unit.
use exact_units::{
    Bam16, Fixed, Point, q,
    origins::{AbsoluteZero, Angle, Gauge, Vacuum},
    prefix::{Hecto, One},
    units::{Kelvin, Pascal, Turn},
};

fn main() {
    let amount = Fixed::<Kelvin, One, i16>::new(300);
    let _ = amount < q!(25 degC);
    let t = Point::<AbsoluteZero, Fixed<Kelvin, One, i16>>::from_raw(300);
    let _ = t < q!(300 K);
    let heading = Point::<Angle<Turn>, Bam16>::from_raw(0);
    let _ = heading < q!(0.25 @ Angle<Turn>);
    let p = Point::<Vacuum, Fixed<Pascal, Hecto, i32>>::from_raw(1013);
    let _ = p == q!(2200 hPa @ Gauge);
    let _ = t == q!(25 degC / s);
    let _ = t == q!(25 degC @ AbsoluteZero);
    let _ = t == q!("300 K @ AbsoluteZero");
    let _ = t == q!(300 K @);
}
