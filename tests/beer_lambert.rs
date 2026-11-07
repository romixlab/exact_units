//! Beer-Lambert (LVL-3): absorbance and optical depth as exact scales of the loss base, μ against α, ε·l·c.
#![cfg(all(
    feature = "levels",
    feature = "mechanical",
    feature = "chemical",
    feature = "electrical",
    feature = "radiometric"
))]
#![expect(clippy::unwrap_used, reason = "a test fails by panicking")]

use core::f64::consts::LN_10;
use exact_units::prefix::{Centi, Hecto, Micro, Milli, One};
use exact_units::units::{
    Absorbance, DecadicAttenuation, Decibel, LinearAttenuation, Loss, Meter, MolarAbsorptivity,
    MolarCentimeter, MolePerLitre, Neper, OpticalDepth, Per,
};
use exact_units::{Fixed, FixedScale, Magnitude, Rounding, Unit, fixed};

/// The exact scale of `U`.
fn magnitude<U: Unit>() -> Magnitude {
    U::EXPR.magnitude().unwrap()
}

/// τ = ln 10 · A, 1 AU = 10 dB and μ = 2α, as `Magnitude` identities: no float in the way.
#[test]
fn scales_are_exact() {
    assert_eq!(
        magnitude::<Absorbance>(),
        magnitude::<OpticalDepth>().mul(Magnitude::LN10)
    );
    assert_eq!(
        magnitude::<Absorbance>(),
        magnitude::<Loss<Decibel>>().mul(Magnitude::ratio(10, 1))
    );
    assert_eq!(
        magnitude::<Loss<Neper>>(),
        magnitude::<OpticalDepth>().mul(Magnitude::ratio(2, 1))
    );
    assert_eq!(
        magnitude::<Per<Loss<Neper>, Meter>>(),
        magnitude::<LinearAttenuation>().mul(Magnitude::ratio(2, 1))
    );
    assert_eq!(
        magnitude::<DecadicAttenuation>(),
        magnitude::<LinearAttenuation>().mul(Magnitude::LN10)
    );
    // L/(mol·cm) in units of the markers: 10⁻³ m³ / (mol · 10⁻² m)
    assert_eq!(
        magnitude::<MolarCentimeter>(),
        magnitude::<MolePerLitre>().mul(Centi::SCALE)
    );
}

/// Both are on the loss base, with what it means: 1 AU converts to dB of loss, not to a gain.
#[test]
fn on_the_loss_base() {
    assert_eq!(Absorbance::DIM, Loss::<Neper>::DIM);
    assert_eq!(OpticalDepth::DIM, Loss::<Neper>::DIM);
    let db = Fixed::<Absorbance, Milli, i16>::new(622)
        .convert::<Loss<Decibel>, Centi, i16>()
        .unwrap();
    assert_eq!(db.get(), 622);
    assert_eq!(db.as_gain(), Fixed::<Decibel, Centi, i16>::new(-622));
}

/// A = ε·l·c in integers, and what its numbers print as.
#[test]
fn molar_product() {
    let epsilon = Fixed::<MolarAbsorptivity, One, u32>::new(6220);
    let path = Fixed::<Meter, Milli, u32>::new(10);
    let c = Fixed::<MolePerLitre, Micro, u32>::new(100);
    let a = (epsilon * path * c)
        .convert::<Absorbance, Milli, u32>()
        .unwrap();
    assert_eq!(a.get(), 622);
    assert_eq!(a.to_string(), "622 mAU");
    assert_eq!(epsilon.to_string(), "6220 AU/(M·cm)");
    assert_eq!(
        Fixed::<OpticalDepth, Milli, u16>::new(1432).to_string(),
        "1432 mτ"
    );
}

/// μ·x is an optical depth, and the same path in α gives half as many nepers.
#[test]
fn coefficient_over_a_path() {
    let mu = Fixed::<LinearAttenuation, Milli, u32>::new(460);
    let x = Fixed::<Meter, One, u32>::new(5);
    let tau = (mu * x).convert::<OpticalDepth, Milli, u32>().unwrap();
    assert_eq!(tau.get(), 2300);
    let alpha = mu.convert::<Per<Loss<Neper>, Meter>, Milli, u32>().unwrap();
    assert_eq!(alpha.get(), 230);
    let np = (alpha * x).convert::<Loss<Neper>, Milli, u32>().unwrap();
    assert_eq!(np.get(), 1150);
    assert_eq!(tau.convert::<Loss<Neper>, Milli, u32>(), Some(np));
    // 2 AU/cm in AU/m and into μ
    let per_cm = Fixed::<DecadicAttenuation, Hecto, f64>::bounded(2.0);
    let mu = per_cm.convert::<LinearAttenuation, One, f64>().unwrap();
    assert!((mu.get() - 200.0 * LN_10).abs() < 1e-11);
}

/// A literal in AU lands in a depth's storage, correctly rounded at compile time.
#[test]
fn literal_in_absorbance() {
    const TAU: Fixed<OpticalDepth, Milli, u16> = fixed!(0.3 in Absorbance);
    assert_eq!(TAU.get(), 691);
    let r = Fixed::<Absorbance, Milli, u16>::new(300)
        .convert_rounded::<OpticalDepth, Milli, u16>(Rounding::NearestEven);
    assert_eq!(r, Some(TAU));
}

/// The transmittance edge: −lg T, −ln T and back, from powers, irradiances and field ratios.
#[cfg(any(feature = "std", feature = "libm"))]
#[test]
fn transmittance() {
    use exact_units::units::{Irradiance, Volt, Watt};

    let t = Fixed::<Per<Watt, Watt>, One, f64>::bounded(0.01);
    assert_eq!(
        t.to_level::<Absorbance, One, f64>(Rounding::NearestEven),
        Some(Fixed::<_, _, f64>::bounded(2.0)),
        "a whole number of decades stays exact"
    );
    let tau = t
        .to_level::<OpticalDepth, One, f64>(Rounding::NearestEven)
        .unwrap();
    assert!((tau.get() - 2.0 * LN_10).abs() < 1e-14);
    let back = tau.to_gain::<Per<Watt, Watt>, One, f64>().unwrap();
    assert!((back.get() - 0.01).abs() < 1e-15);

    let e = Fixed::<OpticalDepth, One, f64>::bounded(1.0);
    let t = e.to_gain::<Per<Watt, Watt>, One, f64>().unwrap();
    assert!((t.get() - (-1.0f64).exp()).abs() < 1e-15);

    let i = Fixed::<Per<Irradiance, Irradiance>, One, f64>::bounded(0.01);
    assert_eq!(
        i.to_level::<Absorbance, Milli, u16>(Rounding::NearestEven),
        Some(Fixed::new(2000))
    );
    // a field ratio of 0.1 is a power ratio of 0.01
    let v = Fixed::<Per<Volt, Volt>, One, f64>::bounded(0.1);
    assert_eq!(
        v.to_level::<Absorbance, One, f64>(Rounding::NearestEven),
        Some(Fixed::<_, _, f64>::bounded(2.0))
    );
}

/// The text `{}` writes reads back in the `with_unit` form, for each new symbol.
#[cfg(feature = "serde")]
#[test]
fn text_round_trip() {
    use exact_units::Unbounded;
    use serde::{Deserialize, Serialize};

    #[derive(Serialize, Deserialize)]
    #[serde(transparent)]
    struct Text<U: Unit, P: FixedScale>(
        #[serde(with = "exact_units::serde::with_unit")] Fixed<U, P, i64, Unbounded>,
    );

    fn round_trip<U: Unit, P: FixedScale>(text: &str) {
        let value = Text::<U, P>(Fixed::new(622));
        let json = serde_json::to_value(&value).unwrap();
        assert_eq!(json, serde_json::Value::String(text.into()));
        let back = serde_json::from_value::<Text<U, P>>(json).unwrap();
        assert_eq!(back.0.get(), 622);
    }

    round_trip::<Absorbance, Milli>("0.622 AU");
    round_trip::<OpticalDepth, Milli>("0.622 τ");
    round_trip::<MolarAbsorptivity, One>("622 AU/(M·cm)");
    round_trip::<LinearAttenuation, One>("622 τ/m");
}
