// `q!`: literals it can't read, each error at the token: no unit, a suffix, too many digits, a power out of range,
// symbols not joined, a digit for a power in a string, a binary prefix in a denominator, no number, -0.
use exact_units::{Fixed, prefix::One, q, units::Bit};

fn main() {
    let t = Fixed::<Bit, One, u16>::new(1);
    let _ = t < q!(100);
    let _ = t < q!(100u8 bit);
    let _ = t < q!(12345678901234567891 bit);
    let _ = t < q!(1 bit^0);
    let _ = t < q!(1 bit^10);
    let _ = t < q!(1 bit bit);
    let _ = t < q!("1 bit2");
    let _ = t < q!(1 bit/Kibit);
    let _ = t < q!(bit);
    let _ = t < q!(1e40000 bit);
    let _ = t < q!(-0 bit);
}
