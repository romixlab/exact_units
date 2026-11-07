// A literal in another unit, or a level, whose value is no stored value doesn't compile without a rounding
// (P2401#3nov-0058b): the error names the two stored values around it in the storage's own unit, to write as
// `fixed!(..)`, and the call that names a rounding. A factor with π and most levels' gains are irrational: those
// always need one.
use exact_units::{
    Fixed, db, fixed, np,
    prefix::{Centi, Micro},
    units::{Degree, Per, Radian, Speed, Volt},
};

fn main() {
    let _: Fixed<Radian, Micro, i32> = fixed!(90 in Degree);
    let _: Fixed<Speed, Centi, i16> = fixed!(359.9 km/h);
    let _: Fixed<Per<Volt, Volt>, Micro, u32> = db!(6);
    let _: Fixed<Per<Volt, Volt>, Micro, u32> = np!(-1);
}
