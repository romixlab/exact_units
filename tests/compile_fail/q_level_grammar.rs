// `q!` levels: inside a unit `dB` reads two ways (a level of gain or of loss), `fixed!` is a number of its context's
// unit and no level, and an `Exact` gain has no compare with a level (10^(x/20) is no fraction).
use exact_units::{
    Exact, Fixed, fixed,
    prefix::Milli,
    q,
    units::{Per, Volt},
};

fn main() {
    let _ = q!(0.2 dB / km);
    let _ = q!(1 Np / s);
    let _: Fixed<Per<Volt, Volt>, Milli, u16> = fixed!(-3 dB);
    let _ = Exact::<Per<Volt, Volt>>::new(1, 2) < q!(-3 dB);
}
