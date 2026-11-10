// `exact!` is an amount of the unit it reads, an `Exact`: an unknown or ambiguous symbol, a point, a level, a range
// and a bare number are refused at the token they are about, with what to write.
use exact_units::exact;

fn main() {
    let _ = exact!(3 furlongs);
    let _ = exact!(2 rd);
    let _ = exact!(25 degC);
    let _ = exact!(300 K @ AbsoluteZero);
    let _ = exact!(-3 dB);
    let _ = exact!(4.9 V ..= 5.1 V);
    let _ = exact!(5);
}
