// `q!`: "rd" with both `nuclear` and `customary` on reads two ways, the rad (UNIT-14) and the international rod
// (`customary::RodInternational`); "rad" itself stays the radian's, unqualified.
use exact_units::{Fixed, prefix::One, q, units::Second};

fn main() {
    let t = Fixed::<Second, One, i16>::new(1);
    let _ = t < q!(5 rd);
}
