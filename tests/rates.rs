//! UNIT-13: the s⁻¹ kinds of a radioactive source (decay constant, count rate, reaction rate), fluence and fluence
//! rate, mean life and half-life, and the named steps that lead to an activity. What must not compile is in the
//! doc comments of `src/quantity/rates.rs` and `docs/recipes/nuclear.md`.
#![cfg(all(
    feature = "radiation",
    feature = "time",
    feature = "dimensionless",
    feature = "mechanical"
))]

use exact_units::prefix::{Micro, One, Pico};
use exact_units::quantities::{self, HalfLife, MeanLife, ReactionRate};
use exact_units::units::{
    Area, Becquerel, CountRate, Day, DecayConstant, Hour, Of, Per, PerSquareCentimeter,
    PerSquareCentimeterSecond, PerSquareMeter, PerSquareMeterSecond, Ratio, Second,
};
use exact_units::{Exact, Fixed, QuantitySpec, Unit};

/// Each new quantity is a kind of s⁻¹, m⁻² or m⁻²·s⁻¹, or a time under `Time`.
#[test]
fn the_quantities_and_their_dimensions() {
    for q in [
        quantities::DecayConstant::DEF,
        quantities::CountRate::DEF,
        ReactionRate::DEF,
    ] {
        assert!(q.is_kind);
        assert!(q.dim.same(quantities::Frequency::DEF.dim));
    }
    assert!(quantities::Fluence::DEF.is_kind);
    assert!(quantities::FluenceRate::DEF.is_kind);
    assert!(
        quantities::Fluence::DEF
            .dim
            .same(<Per<Ratio, Area> as Unit>::DIM)
    );
    assert!(quantities::Time::DEF.is_open_root());
    for q in [MeanLife::DEF, HalfLife::DEF] {
        assert!(!q.is_kind);
        assert_eq!(q.parent.unwrap().id().name, "Time");
    }
    assert_eq!(MeanLife::DEF.name().to_string(), "mean life");
}

/// The fluence units: cm⁻² is 10⁴ m⁻², each a fluence or a fluence rate.
#[test]
fn fluence_units() {
    let phi = Exact::<PerSquareCentimeter>::int(3);
    assert_eq!(phi.convert::<PerSquareMeter>(), Some(Exact::int(30_000)));
    let rate = Exact::<PerSquareCentimeterSecond>::int(7);
    assert_eq!(
        rate.convert::<PerSquareMeterSecond>(),
        Some(Exact::int(70_000))
    );
    assert_eq!(rate.to_string(), "7 cm⁻²·s⁻¹");
    // a fluence rate over a time is a derived m⁻², a fluence by the named step
    let t = Exact::<Second>::int(10);
    let fluence = (rate * t)
        .unwrap()
        .convert::<Per<Ratio, Area>>()
        .unwrap()
        .as_quantity::<quantities::Fluence>();
    assert_eq!(
        fluence.convert::<PerSquareMeter>(),
        Some(Exact::int(700_000))
    );
}

/// A = λ·N in a `Fixed`: the same unit and scale, `None` past the storage.
#[test]
fn times_nuclei_fixed() {
    let lambda = Fixed::<DecayConstant, Pico, u64>::new(13_700);
    let a = lambda.times_nuclei(1_000_000).unwrap();
    let a: Fixed<Becquerel, Micro, u64> = a.convert().unwrap();
    assert_eq!(a.get(), 13_700);
    assert_eq!(lambda.times_nuclei(u64::MAX), None);
    // λ per hour: the activity is per hour too, and converts to Bq by 1/3600
    let per_hour = Fixed::<Of<quantities::DecayConstant, Per<Ratio, Hour>>, One, u32>::new(36);
    let a: Fixed<Becquerel, Micro, u32> = per_hour.times_nuclei(100).unwrap().convert().unwrap();
    assert_eq!(a.get(), 1_000_000);
}

/// A = λ·N, τ = 1/λ and back, exactly; T½ and λ through ln 2, a float edge.
#[test]
fn decay_constant_exact() {
    let lambda = Exact::<DecayConstant>::new(1, 1_000_000);
    let a = lambda.times_nuclei(Exact::int(3_000_000)).unwrap();
    assert_eq!(a.convert::<Becquerel>(), Some(Exact::int(3)));
    let tau = lambda.mean_life::<Day>().unwrap();
    assert_eq!(tau.get(), Exact::<Ratio>::new(1_000_000, 86_400).get());
    assert_eq!(
        tau.decay_constant::<Per<Ratio, Second>>().unwrap().get(),
        lambda.get()
    );
    assert_eq!(Exact::<DecayConstant>::int(0).mean_life::<Second>(), None);
    // a mean life is a time: up to seconds with no step
    assert_eq!(tau.convert::<Second>(), Some(Exact::int(1_000_000)));
    let t_half = lambda.half_life_f64::<Second>().unwrap();
    assert!((t_half.get() - 693_147.180_559_945_3).abs() < 1e-6);
    let back = Exact::<Second>::new(6_931_471_805_599_453, 10_000_000_000)
        .as_quantity::<HalfLife>()
        .decay_constant_f64::<Per<Ratio, Second>>()
        .unwrap();
    assert!((back.get() - 1e-6).abs() < 1e-18);
}

/// A_sat = R, the same number, in a `Fixed` and an `Exact`.
#[test]
fn saturation_activity() {
    let r =
        Fixed::<Per<Ratio, Second>, One, u64>::new(100_000_000_000).as_quantity::<ReactionRate>();
    let a: Fixed<Becquerel, One, u64> = r.saturation_activity().convert().unwrap();
    assert_eq!(a.get(), 100_000_000_000);
    let r = Exact::<Per<Ratio, Second>>::int(5).as_quantity::<ReactionRate>();
    assert_eq!(
        r.saturation_activity().convert::<Becquerel>(),
        Some(Exact::int(5))
    );
}

/// A = R/ε, exact; ε ≤ 0 is refused.
#[test]
fn over_efficiency() {
    let r = Exact::<CountRate>::int(250);
    let a = r.over_efficiency(Exact::new(1, 4)).unwrap();
    assert_eq!(a.convert::<Becquerel>(), Some(Exact::int(1000)));
    assert_eq!(r.over_efficiency(Exact::int(0)), None);
    assert_eq!(r.over_efficiency(Exact::new(-1, 2)), None);
}
