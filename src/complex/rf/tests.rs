use super::*;
use crate::prefix::Milli;
use crate::{Admittance, Impedance};

fn z(re: f64, im: f64) -> Impedance<One, f64> {
    Complex::try_new(re, im).unwrap()
}

fn z32(re: f32, im: f32) -> Impedance<One, f32> {
    Complex::try_new(re, im).unwrap()
}

fn y(re: f64, im: f64) -> Admittance<One, f64> {
    Complex::try_new(re, im).unwrap()
}

fn y32(re: f32, im: f32) -> Admittance<One, f32> {
    Complex::try_new(re, im).unwrap()
}

fn z0(re: f64, im: f64) -> Z0<One, f64> {
    Z0::<One, f64>::try_new(re, im).unwrap()
}

fn gamma(re: f64, im: f64) -> Extended<Gamma<f64>> {
    Extended::Finite(Complex::try_new(re, im).unwrap())
}

const INF: f64 = f64::INFINITY;

#[test]
fn z0_is_finite_and_not_zero() {
    for (re, im) in [
        (0.0, 0.0),
        (-0.0, 0.0),
        (INF, 0.0),
        (1.0, -INF),
        (f64::NAN, 1.0),
    ] {
        assert_eq!(
            Z0::<One, f64>::try_new(re, im),
            Err(OutOfBounds),
            "{re} {im}"
        );
    }
    assert_eq!(
        Z0::<One, f32>::try_new(0.0, f32::INFINITY),
        Err(OutOfBounds)
    );
    assert_eq!(z0(50.0, 0.0).get(), z(50.0, 0.0));
    assert_eq!(z0(0.0, -1.0).get(), z(0.0, -1.0));
    assert_eq!(
        Z0::try_from(z(5e-324, 0.0)).map(Z0::get),
        Ok(z(5e-324, 0.0))
    );
}

#[test]
fn reflection_of_the_special_points() {
    let line = z0(50.0, 10.0);
    // matched, a short, an open (plain and the point), −Z₀
    assert_eq!(z(50.0, 10.0).reflection(line), gamma(0.0, 0.0));
    let short = z(0.0, 0.0).reflection(line).finite().unwrap();
    assert!((short.re().get() + 1.0).abs() < 1e-15 && short.im().get().abs() < 1e-15);
    for open in [z(INF, 0.0), z(-INF, 3.0), z(1.0, INF), z(INF, -INF)] {
        assert_eq!(open.reflection(line), gamma(1.0, 0.0), "{open:?}");
        assert_eq!(Extended::Finite(open).reflection(line), gamma(1.0, 0.0));
    }
    assert_eq!(
        Impedance::<One, f64>::OPEN.reflection(line),
        gamma(1.0, 0.0)
    );
    assert_eq!(z(-50.0, -10.0).reflection(line), Extended::Infinity);
    assert_eq!(
        z32(-75.0, 0.0).reflection(Z0::<One, f32>::bounded(75.0, 0.0)),
        Extended::Infinity
    );
    // one step of the float away from −Z₀, Γ is a large number
    let near = z(-50.0f64.next_up(), -10.0)
        .reflection(line)
        .finite()
        .unwrap();
    assert!(near.re().get().abs() > 1e14);
    // a purely reactive Z₀
    assert_eq!(z(0.0, 1.0).reflection(z0(0.0, 1.0)), gamma(0.0, 0.0));
    assert_eq!(z(0.0, -1.0).reflection(z0(0.0, 1.0)), Extended::Infinity);
}

#[test]
fn reflection_is_one_to_one_on_the_special_points() {
    let line = z0(50.0, 0.0);
    let points = [
        Extended::Finite(z(50.0, 0.0)),
        Extended::Finite(z(0.0, 0.0)),
        Impedance::OPEN,
        Extended::Finite(z(-50.0, 0.0)),
        Extended::Finite(z(0.0, 50.0)),
        Extended::Finite(z(0.0, -50.0)),
    ];
    let images = points.map(|p| p.reflection(line));
    assert_eq!(
        images,
        [
            gamma(0.0, 0.0),
            gamma(-1.0, 0.0),
            gamma(1.0, 0.0),
            Extended::Infinity,
            gamma(0.0, 1.0),
            gamma(0.0, -1.0),
        ]
    );
}

#[test]
fn reflection_of_extreme_finite_values() {
    // Z = 3·Z₀ is Γ = 1/2 at every size, where (Z + Z₀)² overflows or underflows the float
    for s in [f64::MAX / 4.0, 1e300, 1e-300, f64::MIN_POSITIVE, 5e-324] {
        let g = z(3.0 * s, 0.0).reflection(z0(s, 0.0)).finite().unwrap();
        assert!(
            (g.re().get() - 0.5).abs() < 1e-15 && g.im().get() == 0.0,
            "{s}"
        );
    }
    for s in [f32::MAX / 4.0, 1e30, 1e-30, f32::MIN_POSITIVE] {
        let g = z32(0.0, 3.0 * s)
            .reflection(Z0::<One, f32>::try_new(0.0, s).unwrap())
            .finite()
            .unwrap();
        assert!(
            (g.re().get() - 0.5).abs() < 1e-6 && g.im().get() == 0.0,
            "{s}"
        );
    }
}

#[test]
fn reflection_where_the_scaled_sum_goes_below_the_float() {
    // Z + Z₀ = 5e-324·j, which is 0 once scaled by 1/8 for the large parts: Γ is past the float, but a number
    let g = z(-1e308, 5e-324)
        .reflection(z0(1e308, 0.0))
        .finite()
        .unwrap();
    assert_eq!(g.im().get(), INF);
    assert!(!g.re().get().is_nan());
    // the same at a size that fits: Γ = −2·10³⁰⁰ / (j·10⁻³⁰⁰)
    let g = z(-f64::MAX / 2.0, 1e-300)
        .reflection(z0(f64::MAX / 2.0, 0.0))
        .finite()
        .unwrap();
    assert!(g.im().get().is_infinite() && !g.re().get().is_nan());
}

#[test]
fn admittance_of_a_short_and_an_open() {
    assert_eq!(z(0.0, 0.0).to_admittance::<One>(), Admittance::SHORT);
    assert_eq!(z32(-0.0, 0.0).to_admittance::<Milli>(), Admittance::SHORT);
    assert_eq!(y(0.0, -0.0).to_impedance::<One>(), Impedance::OPEN);
    assert_eq!(y32(0.0, 0.0).to_impedance::<Milli>(), Impedance::OPEN);
    // an open conducts nothing, a short has no impedance
    assert_eq!(
        z(INF, 1.0).to_admittance::<One>(),
        Extended::Finite(y(0.0, 0.0))
    );
    assert_eq!(
        y(1.0, f64::NEG_INFINITY).to_impedance::<One>(),
        Extended::Finite(z(0.0, 0.0))
    );
    // and the points themselves
    assert_eq!(
        Impedance::<One, f64>::OPEN.to_admittance::<Milli>(),
        Extended::Finite(Complex::try_new(0.0, 0.0).unwrap())
    );
    assert_eq!(
        Admittance::<One, f32>::SHORT.to_impedance::<One>(),
        Extended::Finite(z32(0.0, 0.0))
    );
}

#[test]
fn z_and_y_round_trip_through_the_points() {
    // short → ∞ → short, open → 0 → open
    let short = Extended::Finite(z(0.0, 0.0));
    let y = short.to_admittance::<One>();
    assert_eq!(y, Admittance::SHORT);
    assert_eq!(y.to_impedance::<One>(), short);
    let y = Impedance::<One, f64>::OPEN.to_admittance::<One>();
    assert!(y.is_open());
    assert_eq!(y.to_impedance::<One>(), Impedance::OPEN);
    // a value both ways
    let z = Extended::Finite(z(30.0, 40.0));
    assert_eq!(z.to_admittance::<Milli>().to_impedance::<One>(), z);
}

#[test]
fn short_and_open_by_name() {
    assert!(z(0.0, -0.0).is_short() && !z(0.0, -0.0).is_open());
    assert!(z(INF, 0.0).is_open() && !z(INF, 0.0).is_short());
    assert!(!z(1.0, 0.0).is_short() && !z(1.0, 0.0).is_open());
    assert!(y(0.0, 0.0).is_open() && !y(0.0, 0.0).is_short());
    assert!(y(0.0, -INF).is_short() && !y(0.0, -INF).is_open());
    assert!(Impedance::<One, f64>::OPEN.is_open() && !Impedance::<One, f64>::OPEN.is_short());
    assert!(Admittance::<One, f64>::SHORT.is_short() && !Admittance::<One, f64>::SHORT.is_open());
    assert!(Extended::Finite(z(0.0, 0.0)).is_short());
    assert!(Extended::Finite(z(INF, 0.0)).is_open());
    assert!(Extended::Finite(y(0.0, 0.0)).is_open());
    assert!(Extended::Finite(y(INF, 0.0)).is_short());
    // integers have no infinity, but a zero
    let zi = Impedance::<Milli, i32>::new(0, 0);
    assert!(zi.is_short() && !zi.is_open());
    assert!(!Impedance::<Milli, i32>::new(1, 0).is_short());
    // in an if and a match
    let y = z(0.0, 0.0).to_admittance::<One>();
    let said = if y == Admittance::SHORT {
        "a short"
    } else {
        "a value"
    };
    assert_eq!(said, "a short");
    let what = match y {
        Extended::Finite(_) => "a value",
        Extended::Infinity => "a short",
    };
    assert_eq!(what, "a short");
}

#[test]
fn admittance_of_extreme_finite_values() {
    // |Z|² overflows or underflows where 1 / Z still fits: 3 + 4j at every size is (3 − 4j) / 25 of its reciprocal
    for s in [1e300, 1e-300, 1e-305] {
        let got = z(3.0 * s, 4.0 * s).to_admittance::<One>().finite().unwrap();
        let (re, im) = (got.re().get() * s, got.im().get() * s);
        assert!(
            (re - 0.12).abs() < 1e-15 && (im + 0.16).abs() < 1e-15,
            "{s}: {got:?}"
        );
    }
    // past the float, 1 / Z is a value with an infinite part, never NaN and not the point
    let got = z(5e-324, 0.0).to_admittance::<One>().finite().unwrap();
    assert_eq!(got.re().get(), INF);
}

#[test]
#[cfg(feature = "std")]
fn display() {
    assert_eq!(Admittance::<One, f64>::SHORT.to_string(), "∞");
    assert_eq!(
        z(30.0, 40.0).to_admittance::<Milli>().to_string(),
        "12-16j mS"
    );
    assert_eq!(z0(50.0, 0.0).to_string(), "50+0j Ω");
    assert_eq!(
        format!("{:?}", z0(50.0, 0.0)),
        "Z0(Complex<Ohm>(50+0j Ω f64))"
    );
}
