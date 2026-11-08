// `assert_in!` failing at compile time: a rounded prefix shows the value's own digits and the prefix it has (integer,
// float, exact and point values, a negative one), any other range the stored values it holds, a level range the
// gains.
use exact_units::{
    Exact, Fixed, Point, assert_in,
    origins::Celsius,
    prefix::{Centi, Milli, One},
    q,
    units::{Kelvin, Per, Volt},
};

const MV: Fixed<Volt, Milli, i32> = Fixed::new(9_542);
const F: Fixed<Volt, One, f64> = Fixed::<Volt, One, f64>::bounded(0.1);
const THIRD: Exact<Volt> = Exact::new(1, 3);
const T: Point<Celsius, Fixed<Kelvin, Centi, i16>> = Point::from_raw(2_153);
const GAIN: Fixed<Per<Volt, Volt>, Milli, u16> = Fixed::new(1_000);

fn main() {
    let _ = assert_in!(MV, q!(9.55... V));
    let _ = assert_in!(Fixed::<Volt, Milli, i32>::new(-9_542), q!(-9.55... V));
    let _ = assert_in!(MV, q!(9.6 V ..= 10 V));
    let _ = assert_in!(F, q!(0.1000001... V));
    let _ = assert_in!(F, q!(0.2 V ..= 0.3 V, Inside));
    let _ = assert_in!(THIRD, q!(0.334... V));
    let _ = assert_in!(THIRD, q!(400 mV .. 500 mV));
    let _ = assert_in!(T, q!(21.54... degC));
    let _ = assert_in!(GAIN, q!(1 dB ..= 3 dB, Inside));
}
