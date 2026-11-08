// `q!` ranges the const fold refuses: an end that is no stored value without `Inside` or `Outside` (integer and
// float storages), a range no stored value is in, one every value is in.
use exact_units::{
    Fixed,
    prefix::{Deci, Milli, One},
    q,
    units::Volt,
};

fn main() {
    let v = Fixed::<Volt, Milli, i16>::new(5000);
    let _ = v.is_in(q!(4.9005 V ..= 5 V));
    let _ = v.is_in(q!(4.9 V ..= 5.0005 V));
    let d = Fixed::<Volt, Deci, i16>::new(49);
    let _ = d.is_in(q!(4.91 V ..= 4.99 V, Inside));
    let b = Fixed::<Volt, Milli, u8>::new(100);
    let _ = b.is_in(q!(0 V ..= 5 V));
    let f = Fixed::<Volt, One, f32>::bounded(0.1);
    let _ = f.is_in(q!(0.1 V ..= 0.2 V));
}
