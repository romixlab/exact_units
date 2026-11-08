//! `neper_cable.svg`: a wave dying along a coax cable, marked where its amplitude has fallen by 1, 2 and 3 Np, and
//! under it the one level 1 Np is for an amplitude and for a power. Every number comes from the datasheet's own
//! 5.4 dB/100 ft, typed as `Per<Loss<Decibel>, Feet100>` and `convert`ed, through the crate's `to_db` and
//! `from_db_const_rounded`; the `assert!`s fail `just docs-build` if a number on the figure stops matching the formula or
//! the page.

use core::f64::consts::{E, TAU};

use exact_units::{
    Fixed,
    Rounding::NearestEven,
    decibel::Decibels,
    prefix::{Deci, Micro, One},
    units::{Decibel, Feet100, Loss, Meter, Metres100, Neper, Per, Volt, Watt},
};

use crate::svg::{Panel, Svg};

type VoltGain = Fixed<Per<Volt, Volt>, One, f64>;
type PowerGain = Fixed<Per<Watt, Watt>, One, f64>;
/// The stored ratios: `from_db_const_rounded` rounds them to nearest, in integers.
type VoltMicro = Fixed<Per<Volt, Volt>, Micro, u32>;
type PowerMicro = Fixed<Per<Watt, Watt>, Micro, u32>;

/// Belden 8259 (RG-58), its product page: 5.4 dB/100 ft at 100 MHz, nominal, velocity of propagation 66 %, as the
/// datasheet writes it.
fn datasheet_loss() -> Fixed<Per<Loss<Decibel>, Feet100>, Deci, i32> {
    Fixed::new(54)
}
const VELOCITY_FACTOR: f64 = 0.66;
const FREQUENCY_HZ: f64 = 100e6;
const LIGHT_M_PER_S: f64 = 299_792_458.0;
/// Whole wave periods the drawing shows per neper, so that a crest falls on each mark.
const CYCLES_PER_NP: f64 = 4.0;

const WIDTH: f64 = 520.0;
const HEIGHT: f64 = 620.0;
const LEFT: f64 = 8.0;
/// Where the cable starts, and how much of it one neper takes.
const CABLE_X: f64 = 62.0;
const PX_PER_NP: f64 = 126.0;
const CABLE_NP: f64 = 3.5;
const CABLE_TOP: f64 = 92.0;
const CABLE_HEIGHT: f64 = 150.0;
const ROWS_TOP: f64 = CABLE_TOP + CABLE_HEIGHT + 30.0;
const ROW_PITCH: f64 = 24.0;
/// Where the amplitude and the power of the second panel start.
const LEVELS_TOP: f64 = 438.0;

/// The attenuation constant in Np/m, the datasheet's dB/100 ft converted through the crate's own types.
fn alpha() -> f64 {
    let per_100ft = datasheet_loss();
    let np_per_m: Fixed<Per<Loss<Neper>, Meter>, One, f64> = per_100ft.convert().unwrap();
    let alpha = np_per_m.get();
    // back through the crate: the loss over one "100 ft" span is the datasheet's own number
    let span = Fixed::<Feet100, One, f64>::bounded(1.0);
    let loss: Fixed<Loss<Decibel>, One, f64> = per_100ft.over(span, NearestEven).unwrap();
    assert!(
        (loss.get() - 5.4).abs() < 1e-9,
        "{} dB over 100 ft",
        loss.get()
    );
    alpha
}

/// One neper mark: the level and the two ratios it stands for.
struct Mark {
    /// the level in dB of the amplitude, negative
    db: f64,
    amplitude: u32,
    power: u32,
}

/// What `n` Np is: `to_db` of e⁻ⁿ V/V and of e⁻²ⁿ W/W are one level, and that level as `Decibels` gives the two ratios
/// back, correctly rounded by the crate.
fn mark(n: i32, db_in_np: f64) -> Mark {
    let db = VoltGain::bounded(E.powi(-n)).to_db().unwrap().get();
    let of_power = PowerGain::bounded(E.powi(-2 * n)).to_db().unwrap().get();
    assert!((db - of_power).abs() < 1e-9, "{n} Np, V/V against W/W");
    assert!((db * db_in_np + f64::from(n)).abs() < 1e-12, "{n} Np");
    #[expect(
        clippy::cast_possible_truncation,
        reason = "a few tens of dB in nanobels fit an i64"
    )]
    let level = Decibels::new((db * 1e9).round() as i64, 9);
    let amplitude = VoltMicro::from_db_const_rounded(level, NearestEven).get();
    let power = PowerMicro::from_db_const_rounded(level, NearestEven).get();
    assert!(
        (f64::from(amplitude) / 1e6 - E.powi(-n)).abs() < 1e-6,
        "{n} Np, V/V"
    );
    assert!(
        (f64::from(power) / 1e6 - E.powi(-2 * n)).abs() < 1e-6,
        "{n} Np, W/W"
    );
    Mark {
        db,
        amplitude,
        power,
    }
}

fn panel(np_per_m: f64) -> Panel {
    Panel {
        area: (CABLE_X, CABLE_TOP, CABLE_NP * PX_PER_NP, CABLE_HEIGHT),
        x: (0.0, CABLE_NP / np_per_m),
        y: (-1.14, 1.14),
    }
}

/// The coax: jacket, the braid just inside it, the centre conductor coming out to the source at the left.
fn cable(s: &mut Svg, p: &Panel) {
    let (l, t, w, h) = p.area;
    s.rect((l, t), (w, h), 6.0, "faint");
    s.rect((l, t), (w, h), 6.0, "thin");
    s.line((l, t + 5.0), (l + w, t + 5.0), "thin");
    s.line((l, t + h - 5.0), (l + w, t + h - 5.0), "thin");
    let centre = t + h / 2.0;
    s.line((l - 26.0, centre), (l + w, centre), "thin");
    s.circle((l - 34.0, centre), 8.0, "accent");
    s.text(LEFT, centre - 16.0, "start", "soft lg", "source");
}

/// The wave, its envelope and the crest on each mark. The cosine's crests are `CYCLES_PER_NP` to a neper.
fn wave(s: &mut Svg, p: &Panel, np_per_m: f64) {
    let (_, end) = p.x;
    let drawn_wavelength = 1.0 / np_per_m / CYCLES_PER_NP;
    let steps = 1400_i32;
    let at = |i: i32| end * f64::from(i) / f64::from(steps);
    let envelope = |x: f64| (-np_per_m * x).exp();
    let points: Vec<_> = (0..=steps)
        .map(&at)
        .map(|x| (x, envelope(x) * (TAU * x / drawn_wavelength).cos()))
        .collect();
    let upper: Vec<_> = (0..=steps)
        .step_by(20)
        .map(|i| (at(i), envelope(at(i))))
        .collect();
    let lower: Vec<_> = upper.iter().map(|&(x, y)| (x, -y)).collect();
    s.path(p, &upper, "blue-line dash");
    s.path(p, &lower, "blue-line dash");
    s.path(p, &points, "accent-line");
    // in the free corner above the falling envelope
    s.text(
        p.px(0.44 / np_per_m),
        p.py(0.83),
        "start",
        "blue lg strong",
        r#"V₀·e<tspan class="sup" dy="-6">−αx</tspan>"#,
    );
}

/// The heading: the datasheet's number, the same in dB per 100 m, and in Np/m.
fn heading(s: &mut Svg, np_per_m: f64) {
    let per_100m: Fixed<Per<Loss<Decibel>, Metres100>, One, f64> =
        datasheet_loss().convert().unwrap();
    s.text(LEFT, 22.0, "start", "lg strong", "RG-58 at 100 MHz");
    let loss = format!("5.4 dB/100 ft = {:.1} dB/100 m", per_100m.get());
    s.text(LEFT, 44.0, "start", "mono lg", &loss);
    let alpha = format!("α = {np_per_m:.4} Np/m: 1 Np every {:.1} m", 1.0 / np_per_m);
    s.text(LEFT, 66.0, "start", "mono lg", &alpha);
    s.text(WIDTH - LEFT, 82.0, "end", "soft lg", "the wave travels →");
}

/// The dashed mark of each neper through the cable, then the rows under it, one column a mark.
fn marks(s: &mut Svg, p: &Panel, np_per_m: f64, db_in_np: f64) {
    let titles = [
        "along the cable",
        "attenuation",
        "in dB",
        "amplitude",
        "power",
    ];
    for (row, title) in (0_i32..).zip(titles) {
        let y = ROWS_TOP + f64::from(row) * ROW_PITCH;
        s.text(LEFT, y, "start", "soft lg", title);
    }
    let (_, t, _, h) = p.area;
    let power_of_e = [("", "²"), ("²", "⁴"), ("³", "⁶")];
    for (n, (amplitude_e, power_e)) in (1_i32..).zip(power_of_e) {
        let m = mark(n, db_in_np);
        let metres = f64::from(n) / np_per_m;
        let x = p.px(metres);
        s.line((x, t - 6.0), (x, t + h + 10.0), "thin dash");
        s.circle((x, p.py(E.powi(-n))), 4.0, "accent");
        let rows = [
            (format!("{metres:.0} m"), "mono lg"),
            (format!("{n} Np"), "lg strong"),
            (format!("{:.3} dB", -m.db), "mono lg"),
            (
                format!("1/e{amplitude_e}={:.4}", f64::from(m.amplitude) / 1e6),
                "mono lg blue",
            ),
            (
                format!("1/e{power_e}={:.4}", f64::from(m.power) / 1e6),
                "mono lg accent",
            ),
        ];
        for (row, (text, class)) in (0_i32..).zip(rows) {
            let y = ROWS_TOP + f64::from(row) * ROW_PITCH;
            s.text(x, y, "middle", class, &text);
        }
    }
    // the true wavelength, against the one that can be drawn
    let wavelength = VELOCITY_FACTOR * LIGHT_M_PER_S / FREQUENCY_HZ;
    let stretch = 1.0 / np_per_m / CYCLES_PER_NP / wavelength;
    assert!((wavelength - 1.98).abs() < 0.01 && (stretch - 6.2).abs() < 0.05);
    let note = format!(
        "wave drawn {stretch:.0}× too long: at 66 % of c its wavelength is {wavelength:.2} m"
    );
    let y = ROWS_TOP + 5.0 * ROW_PITCH + 6.0;
    s.text(LEFT, y, "start", "soft lg", &note);
}

/// One bar of the second panel, its length the ratio left after 1 Np, and the two logarithms that give one level.
fn level_row(s: &mut Svg, top: f64, row: (&str, &str, u32), formulas: (&str, &str)) {
    let (name, hue, micro) = row;
    let (bar, width) = (96.0, 300.0);
    let ratio = f64::from(micro) / 1e6;
    s.text(LEFT, top + 13.0, "start", &format!("{hue} lg strong"), name);
    s.rect((bar, top), (width, 16.0), 2.0, &format!("tint-{hue}"));
    s.rect((bar, top), (width * ratio, 16.0), 2.0, hue);
    let value = format!("×{ratio:.4}");
    s.text(WIDTH - LEFT, top + 13.0, "end", "mono lg", &value);
    let (natural, decadic) = formulas;
    s.text(LEFT, top + 38.0, "start", "mono lg", natural);
    s.text(WIDTH - LEFT, top + 38.0, "end", "mono lg", decadic);
}

fn levels(s: &mut Svg, db_in_np: f64) {
    let one = mark(1, db_in_np);
    s.line(
        (LEFT, LEVELS_TOP - 8.0),
        (WIDTH - LEFT, LEVELS_TOP - 8.0),
        "thin",
    );
    s.text(
        LEFT,
        LEVELS_TOP + 16.0,
        "start",
        "lg strong",
        "1 Np down, one level, two ratios",
    );
    let db = -one.db;
    level_row(
        s,
        LEVELS_TOP + 30.0,
        ("amplitude", "blue", one.amplitude),
        ("ln(A₁/A₂) = 1 Np", &format!("20·lg(A₁/A₂) = {db:.3} dB")),
    );
    level_row(
        s,
        LEVELS_TOP + 88.0,
        ("power", "accent", one.power),
        ("½·ln(P₁/P₂) = 1 Np", &format!("10·lg(P₁/P₂) = {db:.3} dB")),
    );
    let factors = format!("1 Np = {db:.3} dB, 1 dB = ln 10/20 Np = {db_in_np:.4} Np");
    s.text(LEFT, LEVELS_TOP + 158.0, "start", "mono lg", &factors);
}

pub fn neper_cable() -> String {
    // 1 dB in Np, through the crate's own conversion: the page's own numbers can't drift from it
    let db_in_np = Fixed::<Decibel, One, f64>::bounded(1.0)
        .convert::<Neper, One, f64>()
        .unwrap()
        .get();
    assert!((db_in_np - core::f64::consts::LN_10 / 20.0).abs() < 1e-15);
    let np_per_m = alpha();
    assert!((np_per_m - 0.0204).abs() < 5e-5 && (1.0 / np_per_m - 49.0).abs() < 0.05);

    let mut s = Svg::new(
        WIDTH,
        HEIGHT,
        "A sine wave decaying along an RG-58 cable at 100 MHz, 5.4 dB per 100 ft or 0.0204 Np/m: after 49 m it has lost \
         1 Np, 8.686 dB, and its amplitude is 1/e; after 98 m 2 Np, 17.372 dB, 1/e squared; after 147 m 3 Np, \
         26.058 dB, 1/e cubed. Below, 1 Np as one level: amplitude down to 0.3679, power down to 0.1353, both 8.686 dB",
    );
    let p = panel(np_per_m);
    heading(&mut s, np_per_m);
    cable(&mut s, &p);
    marks(&mut s, &p, np_per_m, db_in_np);
    wave(&mut s, &p, np_per_m);
    levels(&mut s, db_in_np);
    s.finish()
}
