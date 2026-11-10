//! Symbol units (NUM-35): `Ln2` and `Ln10` keep ln 2 and ln 10 exact in the unit of an `Exact`, so the decay constant,
//! activity and mean life of examples/15_nuclear.rs are exact values with ln 2 in their type.
#![cfg(all(feature = "dimensionless", feature = "time"))]

use exact_units::units::{Ln2, Ln10, Per, Ratio, Second};
use exact_units::{Exact, Magnitude, Unit};

/// ²²⁶Ra: T½ = 1600 y of NUBASE2020's year, in seconds.
const T_HALF: u128 = 50_491_081_728;

#[test]
fn scales_are_the_symbols() {
    assert_eq!(Ln2::EXPR.magnitude(), Some(Magnitude::LN2));
    assert_eq!(Ln10::EXPR.magnitude(), Some(Magnitude::LN10));
}

#[test]
fn decay_constant_keeps_ln2_in_the_type() {
    // λ = ln 2 / T½ holds exactly 1/T½ of (ln 2 per second)
    let lambda: Exact<Per<Ln2, Second>> =
        (Exact::<Ln2>::int(1) / Exact::<Second>::int(T_HALF.cast_signed())).unwrap();
    assert_eq!(lambda.get(), Exact::<Ratio>::new(1, T_HALF).get());
    // A = λ·N for N = 10²⁰ atoms stays exact, ln 2 still in the unit
    let n = Exact::<Ratio>::int(100_000_000_000_000_000_000);
    let activity = (lambda * n).unwrap();
    assert_eq!(
        activity.convert::<Per<Ln2, Second>>().unwrap().get(),
        Exact::<Ratio>::new(100_000_000_000_000_000_000, T_HALF).get()
    );
    // τ = T½ / ln 2, and λ·τ is exactly 1
    let tau = (Exact::<Second>::int(T_HALF.cast_signed()) / Exact::<Ln2>::int(1)).unwrap();
    let one = (lambda * tau).unwrap();
    assert_eq!(one.convert::<Ratio>(), Some(Exact::int(1)));
}
