// `q!`: literals it can't read, each error at the token: no unit, a suffix, too many digits, a power out of range,
// symbols not joined, a digit for a power in a string, a binary prefix in a denominator, no number.
use exact_units::{Fixed, prefix::One, q, units::Byte};

fn main() {
    let t = Fixed::<Byte, One, u16>::new(1);
    let _ = t < q!(100);
    let _ = t < q!(100u8 B);
    let _ = t < q!(12345678901234567891 B);
    let _ = t < q!(1 B^0);
    let _ = t < q!(1 B^10);
    let _ = t < q!(1 B B);
    let _ = t < q!("1 B2");
    let _ = t < q!(1 B/KiB);
    let _ = t < q!(B);
    let _ = t < q!(1e40000 B);
}
