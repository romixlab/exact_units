// `q!` against an `Exact`: the literal is a fraction of the `Exact`'s unit, so a unit of another dimension and a
// factor with π (degrees against radians) don't compile.
use exact_units::{
    Exact, q,
    units::{Hertz, Radian},
};

fn main() {
    let f = Exact::<Hertz>::new(80_000_000, 3);
    let _ = f < q!(1 m);
    let a = Exact::<Radian>::new(3, 2);
    let _ = a < q!(90 deg);
    let _ = q!(90 deg) == a;
}
