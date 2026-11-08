// `q!` ranges of points and of levels the const fold refuses: an end that is no stored value without `Inside` or
// `Outside` (68.01 °F against steps of 0.01 °C, the gain of −3 dB), a range every stored value is in, a level against
// a gain with no level form.
use exact_units::{
    Fixed, Point, q,
    origins::Celsius,
    prefix::{Centi, Milli},
    units::{Kelvin, Per, Ratio, Volt},
};

fn main() {
    let t = Point::<Celsius, Fixed<Kelvin, Centi, i16>>::from_raw(2000);
    let _ = t.is_in(q!(68.01 degF ..= 77 degF));
    let u = Point::<Celsius, Fixed<Kelvin, Centi, u8>>::from_raw(20);
    let _ = u.is_in(q!(0 degC ..= 5 degC));
    let g = Fixed::<Per<Volt, Volt>, Milli, u16>::new(1000);
    let _ = g.is_in(q!(-3 dB ..= 3 dB));
    let r = Fixed::<Ratio, Milli, u16>::new(1000);
    let _ = r.is_in(q!(-3 dB ..= 3 dB, Inside));
}
