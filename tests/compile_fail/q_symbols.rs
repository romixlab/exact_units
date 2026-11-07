// `q!`: symbols it refuses, each error at the symbol. A symbol that reads two ways (min, gal), one it doesn't know
// (kmh), a prefix on a unit that takes none (kh) or a binary prefix on one that isn't bit or B (KiV), a prefix below
// kilo on the tonne (mt), the stem of qualified symbols (qt, cable: no bare default).
use exact_units::{Fixed, prefix::One, q, units::Second};

fn main() {
    let t = Fixed::<Second, One, i16>::new(1);
    let _ = t < q!(5 min);
    let _ = t < q!(5 gal);
    let _ = t < q!(5 kmh);
    let _ = t < q!(5 kh);
    let _ = t < q!(5 KiV);
    let _ = t < q!(5 "furlong");
    let _ = t < q!(5 mt);
    let _ = t < q!(5 qt);
    let _ = t < q!(5 cable);
}
