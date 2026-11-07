// A literal that is no stored value doesn't compile without a rounding (P2401#3nov-0058b): the error names the two
// stored values around it, their exact decimals ready to paste, and the call that names a rounding. Integer steps,
// negative ones, the floats of a float storage, and steps whose values aren't finite decimals.
use exact_units::{
    Fixed, fixed,
    prefix::{One, Pow2, Rational},
    units::Ratio,
};

fn main() {
    let _: Fixed<Ratio, Pow2<-30>, i64> = fixed!(0.1);
    let _: Fixed<Ratio, Pow2<-15>, i16> = fixed!(-0.7071);
    let _: Fixed<Ratio, One, f32> = fixed!(0.1);
    let _: Fixed<Ratio, Rational<1, 3>, u8> = fixed!(2.5);
}
