//! `ultrasound.svg`: one-way loss against depth for medical ultrasound at three frequencies, soft tissue's 0.5
//! dB/(cm·MHz). Every slope comes from the crate's own `times_into`, `convert` and `over`, as the book's worked
//! example does; the `assert!`s fail `just docs-build` if a number on the figure stops matching the page.

use exact_units::{
    Fixed,
    Rounding::NearestEven,
    prefix::{Centi, Deci, Mega, One},
    units::{CentimetreMegahertz, Decibel, Hertz, Loss, Meter, Per},
};

use crate::svg::{Panel, Svg};

const WIDTH: f64 = 560.0;
const HEIGHT: f64 = 340.0;
const DEPTH_MAX: f64 = 6.0;
const DB_MAX: f64 = 30.0;

/// Soft tissue's own rate, 0.5 dB/(cm·MHz).
fn alpha0() -> Fixed<Per<Loss<Decibel>, CentimetreMegahertz>, Deci, i32> {
    Fixed::new(5)
}

/// α(f) in dB/m, through the crate's `times_into` and `convert`: cm·MHz divided by hertz settles to a length.
fn alpha_per_metre(mhz: i32) -> Fixed<Per<Loss<Decibel>, Meter>, One, i32> {
    let f = Fixed::<Hertz, Mega, i32>::new(mhz);
    let product = alpha0()
        .times_into::<Hertz, Mega, _, Deci, i32>(f, NearestEven)
        .unwrap();
    product.convert().unwrap()
}

/// The one-way loss at `depth_cm` for the given frequency, in dB, through `Fixed::over`.
fn loss_db(mhz: i32, depth_cm: f64) -> f64 {
    let alpha: Fixed<Per<Loss<Decibel>, Meter>, One, f64> =
        Fixed::new(f64::from(alpha_per_metre(mhz).get()));
    let depth = Fixed::<Meter, Centi, f64>::new(depth_cm);
    let loss: Fixed<Loss<Decibel>, One, f64> = alpha.over(depth, NearestEven).unwrap();
    loss.get()
}

/// One frequency's line, and its label at the free end.
fn line(s: &mut Svg, p: &Panel, mhz: i32, hue: &str) {
    let points: Vec<_> = (0..=120)
        .map(|i| f64::from(i) * DEPTH_MAX / 120.0)
        .map(|depth| (depth, loss_db(mhz, depth)))
        .take_while(|&(_, db)| db <= DB_MAX)
        .collect();
    s.path(p, &points, &format!("{hue}-line"));
    if let Some(&(depth, db)) = points.last() {
        s.text(
            p.px(depth) + 6.0,
            p.py(db) + 4.0,
            "start",
            &format!("{hue} strong"),
            &format!("{mhz} MHz"),
        );
    }
}

pub fn ultrasound() -> String {
    // the book's own worked example: 5 MHz through 4 cm is 10.0 dB one way
    let alpha = alpha_per_metre(5);
    assert_eq!(alpha.get(), 250, "2.5 dB/cm at 5 MHz");
    let ten = loss_db(5, 4.0);
    assert!((ten - 10.0).abs() < 1e-9, "5 MHz at 4 cm: {ten} dB");

    let mut s = Svg::new(
        WIDTH,
        HEIGHT,
        "One-way ultrasound loss against depth at 2, 5 and 8 MHz, soft tissue's 0.5 dB per centimetre-megahertz: \
         5 MHz reaches 10 dB of loss at 4 cm, the book's worked example",
    );
    let p = Panel {
        area: (56.0, 30.0, 480.0, 250.0),
        x: (0.0, DEPTH_MAX),
        y: (0.0, DB_MAX),
    };
    s.axes(
        &p,
        &[(0.0, "0"), (2.0, "2"), (4.0, "4"), (6.0, "6 cm")],
        &[(0.0, "0"), (10.0, "10"), (20.0, "20"), (30.0, "30 dB")],
        "depth",
        "one-way loss",
    );
    line(&mut s, &p, 2, "blue");
    line(&mut s, &p, 5, "accent");
    line(&mut s, &p, 8, "purple");

    // the worked example, 5 MHz at 4 cm
    let (x, y) = (p.px(4.0), p.py(10.0));
    s.line((x, p.py(0.0)), (x, y), "thin dash");
    s.line((p.px(0.0), y), (x, y), "thin dash");
    s.circle((x, y), 4.0, "accent");
    s.text(x + 6.0, y - 10.0, "start", "mono soft", "4 cm, 10 dB");
    s.finish()
}
