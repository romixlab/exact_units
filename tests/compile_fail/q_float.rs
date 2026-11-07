// `q!`: a float storage checks the literal's unit as an integer one does, and a literal of 0 has its unit checked
// too. `==` and `!=` compile only against a literal that is a float of the storage: not 0.1, not one past the
// float's range, not one in degrees against radians.
use exact_units::{
    Fixed,
    prefix::{Centi, One},
    q,
    units::{Radian, Speed, Volt},
};

fn main() {
    let speed = Fixed::<Speed, Centi, f32>::bounded(99.99);
    let _ = speed < q!(100 m);
    let speed = Fixed::<Speed, Centi, i16>::new(9_999);
    let _ = speed < q!(0 m);
    let v = Fixed::<Volt, One, f32>::bounded(0.1);
    let _ = v == q!(0.1 V);
    let _ = v != q!(3.4028235e38 V);
    let _ = q!(-3.4028235e38 V) == v;
    let a = Fixed::<Radian, One, f64>::bounded(1.5707963267948966);
    let _ = a == q!(90 deg);
}
