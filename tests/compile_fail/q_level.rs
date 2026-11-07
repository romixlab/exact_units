// `q!` levels: `dB` and `Np` compare with a gain whose kind has a level form, `==` only where a stored value is the
// gain, and a value stored as a level asks for its unit (a level of gain and one of loss differ in sign).
use exact_units::{
    Fixed,
    prefix::{Milli, One},
    q,
    units::{Decibel, Loss, Per, Ratio, Volt},
};

fn main() {
    let gain = Fixed::<Per<Volt, Volt>, Milli, u16>::new(707);
    let _ = gain == q!(-3 dB);
    let fgain = Fixed::<Per<Volt, Volt>, One, f32>::new(0.5);
    let _ = fgain != q!(-6 dB);
    let ratio = Fixed::<Ratio, Milli, u16>::new(707);
    let _ = ratio < q!(-3 dB);
    let level = Fixed::<Decibel, Milli, i16>::new(-3000);
    let _ = level < q!(-3 dB);
    let loss = Fixed::<Loss<Decibel>, Milli, i16>::new(3000);
    let _ = q!(0.5 Np) > loss;
}
