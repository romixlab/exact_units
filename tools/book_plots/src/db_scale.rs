//! `db_scale.svg`: one linear dB ruler from +20 to −150 dB. On its left the amplitude and power ratios read off it, on
//! its right what lives down there: the noise floor of ideal converters and the smallest step of stored values, each
//! hanging from full scale. Every number is the crate's; the `assert!`s fail `just docs-build` if a number on the
//! figure stops matching the formula or the page.

use exact_units::{
    Fixed, Q15,
    Rounding::NearestEven,
    db,
    decibel::Decibels,
    prefix::{Micro, Milli, One},
    units::{Per, Volt, Watt},
};

use crate::svg::Svg;

type VoltGain = Fixed<Per<Volt, Volt>, One, f64>;
type PowerGain = Fixed<Per<Watt, Watt>, One, f64>;
/// The stored ratios: `from_db_const_rounded` rounds them to nearest, in integers.
type VoltMicro = Fixed<Per<Volt, Volt>, Micro, u32>;
type PowerMicro = Fixed<Per<Watt, Watt>, Micro, u32>;
type Level = Fixed<Per<Volt, Volt>, Milli, u16>;

const WIDTH: f64 = 520.0;
const HEIGHT: f64 = 850.0;
const TOP: f64 = 74.0;
const PX_PER_DB: f64 = 4.4;
const DB_MAX: i32 = 20;
const DB_MIN: i32 = -150;

// right edges of the columns, then the ruler and the bars
const AMPLITUDE: f64 = 84.0;
const POWER: f64 = 178.0;
const DECIBEL: f64 = 226.0;
const RULER: f64 = 252.0;
const BARS: f64 = 270.0;
const BAR_WIDTH: f64 = 18.0;
const BAR_STEP: f64 = 24.0;
/// The least distance between two landmark rows, in SVG units.
const ROW_PITCH: f64 = 18.0;

fn y(db: f64) -> f64 {
    TOP + (f64::from(DB_MAX) - db) * PX_PER_DB
}

/// A level as the figure prints it: a real minus, a plus sign above 0.
fn signed(db: i32) -> String {
    match db {
        0 => "0".into(),
        d if d < 0 => format!("−{}", -d),
        d => format!("+{d}"),
    }
}

/// A ratio to three decimals, without trailing zeros: 1.995, 0.501, 0.1, 100.
fn ratio(micro: u32) -> String {
    let s = format!("{:.3}", f64::from(micro) / 1e6);
    let s = s.trim_end_matches('0').trim_end_matches('.');
    format!("×{s}")
}

/// 10 to a whole power, the exponent raised as SVG text.
fn power_of_ten(exponent: i32) -> String {
    let e = signed(exponent);
    format!(r#"×10<tspan class="sup" dy="-6">{e}</tspan>"#)
}

/// The y of each landmark's row: on its true level, pushed apart from 0 outwards where rows would touch.
fn spread(levels: &[i32]) -> Vec<f64> {
    let mut rows: Vec<f64> = levels.iter().map(|&d| y(f64::from(d))).collect();
    let zero = levels.iter().position(|&d| d == 0).unwrap_or(0);
    for i in (0..zero).rev() {
        rows[i] = rows[i].min(rows[i + 1] - ROW_PITCH);
    }
    for i in zero + 1..rows.len() {
        rows[i] = rows[i].max(rows[i - 1] + ROW_PITCH);
    }
    rows
}

/// The landmarks: the amplitude and power ratio of each level, correctly rounded by the crate, with the two "×2" and
/// the two "×½" of the rules of thumb tinted.
fn landmarks(s: &mut Svg) {
    let levels = [20, 10, 6, 3, 0, -3, -6, -10, -20];
    let rows = spread(&levels);
    for (&level, &row) in levels.iter().zip(&rows) {
        let exact = Decibels::new(i64::from(level), 0);
        let amplitude = VoltMicro::from_db_const_rounded(exact, NearestEven).get();
        let power = PowerMicro::from_db_const_rounded(exact, NearestEven).get();
        // the stored ratios are the level again, within what a rounded micro can hold
        let back = VoltGain::bounded(f64::from(amplitude) / 1e6).to_db();
        assert!((back - f64::from(level)).abs() < 1e-4, "{level} dB, V/V");
        let back = PowerGain::bounded(f64::from(power) / 1e6).to_db();
        assert!((back - f64::from(level)).abs() < 1e-4, "{level} dB, W/W");

        if level.abs() == 6 {
            s.rect(
                (AMPLITUDE - 64.0, row - 11.0),
                (70.0, 21.0),
                4.0,
                "tint-blue",
            );
        }
        if level.abs() == 3 {
            s.rect((POWER - 64.0, row - 11.0), (70.0, 21.0), 4.0, "tint-accent");
        }
        let (a, p) = (ratio(amplitude), ratio(power));
        s.text(AMPLITUDE, row + 5.0, "end", "mono lg", &a);
        s.text(POWER, row + 5.0, "end", "mono lg", &p);
        let class = if level == 0 || level.abs() % 3 == 0 {
            "lg strong"
        } else {
            "mono lg"
        };
        s.text(DECIBEL, row + 5.0, "end", class, &signed(level));
        let tick = tick_length(level);
        s.line(
            (DECIBEL + 3.0, row),
            (RULER - tick, y(f64::from(level))),
            "thin",
        );
    }
}

/// Below the landmarks the ratios are whole powers of ten: power every 10 dB, amplitude every 20 dB.
fn decades(s: &mut Svg) {
    for level in (DB_MIN..=-30).step_by(10) {
        let row = y(f64::from(level));
        let exponent = level / 10;
        let back = PowerGain::bounded(10f64.powi(exponent)).to_db();
        assert!((back - f64::from(level)).abs() < 1e-9, "{level} dB, W/W");
        s.text(POWER, row + 5.0, "end", "mono lg", &power_of_ten(exponent));
        if level % 20 == 0 {
            let back = VoltGain::bounded(10f64.powi(level / 20)).to_db();
            assert!((back - f64::from(level)).abs() < 1e-9, "{level} dB, V/V");
            s.text(
                AMPLITUDE,
                row + 5.0,
                "end",
                "mono lg",
                &power_of_ten(level / 20),
            );
        }
        s.text(DECIBEL, row + 5.0, "end", "mono lg", &signed(level));
        s.line((DECIBEL + 3.0, row), (RULER - 12.0, row), "thin");
    }
}

fn tick_length(level: i32) -> f64 {
    match level {
        l if l % 10 == 0 => 12.0,
        l if l.abs() <= 20 && l % 3 == 0 => 12.0,
        l if l % 5 == 0 => 8.0,
        _ => 4.0,
    }
}

/// A ruler with a tick for every dB, its colour fading from full scale into the noise.
fn ruler(s: &mut Svg) {
    s.raw(
        r#"<defs><linearGradient id="eu-db-fade" x1="0" y1="0" x2="0" y2="1"><stop offset="0" style="stop-color:var(--eu-accent)"/><stop offset="1" style="stop-color:var(--eu-accent);stop-opacity:.2"/></linearGradient><linearGradient id="eu-db-green" x1="0" y1="0" x2="0" y2="1"><stop offset="0" style="stop-color:var(--eu-green);stop-opacity:.5"/><stop offset="1" style="stop-color:var(--eu-green);stop-opacity:.12"/></linearGradient><linearGradient id="eu-db-purple" x1="0" y1="0" x2="0" y2="1"><stop offset="0" style="stop-color:var(--eu-purple);stop-opacity:.5"/><stop offset="1" style="stop-color:var(--eu-purple);stop-opacity:.12"/></linearGradient></defs>"#,
    );
    for level in DB_MIN..=DB_MAX {
        let row = y(f64::from(level));
        s.line((RULER - tick_length(level), row), (RULER, row), "thin");
    }
    s.rect(
        (RULER, y(f64::from(DB_MAX))),
        (6.0, f64::from(DB_MAX - DB_MIN) * PX_PER_DB),
        1.0,
        "db-fade",
    );
}

/// One anchor: a bar from full scale down to its level, the level and its name under the bar's end, in the free
/// space the shorter bars leave on the right.
struct Anchor {
    level: f64,
    value: String,
    name: &'static str,
    /// the theme colour, for `db-green` / `db-purple`
    hue: &'static str,
    /// how much lower than usual the label sits, clear of the label of the bar beside it
    drop: f64,
}

fn adc_floor(bits: i32) -> Anchor {
    // a full-scale sine over quantisation noise: (A²/2) / (q²/12) with q = 2A/2ᴺ
    let snr = 1.5 * 4f64.powi(bits);
    let level = PowerGain::bounded(1.0 / snr).to_db();
    let rule_of_thumb = 6.02 * f64::from(bits) + 1.76;
    assert!((level + rule_of_thumb).abs() < 0.02, "{bits} bits");
    Anchor {
        level,
        value: dbfs(level),
        name: match bits {
            12 => "ideal 12-bit ADC",
            16 => "ideal 16-bit ADC",
            _ => "ideal 24-bit ADC",
        },
        hue: "green",
        drop: if bits == 16 { 8.0 } else { 0.0 },
    }
}

fn dbfs(level: f64) -> String {
    format!("−{:.1} dBFS", -level)
}

fn anchors(s: &mut Svg) {
    let q15_step = Q15::<Per<Volt, Volt>>::of(1).to_db();
    assert!((q15_step + 15.0 * VoltGain::bounded(2.0).to_db()).abs() < 1e-9);
    let level_step = Level::new(1).to_db();
    assert!((level_step + 60.0).abs() < 1e-9);

    // longest next to the ruler: each label then runs right, past the bars that ended above it
    let bars = [
        adc_floor(24),
        adc_floor(16),
        Anchor {
            level: q15_step,
            value: dbfs(q15_step),
            name: "Q15's smallest step",
            hue: "purple",
            drop: 0.0,
        },
        adc_floor(12),
        Anchor {
            level: level_step,
            value: dbfs(level_step),
            name: "1 mV/V in a u16",
            hue: "purple",
            drop: 0.0,
        },
    ];
    let full_scale = y(0.0);
    let mut x = BARS;
    for bar in &bars {
        let end = y(bar.level);
        // the guide back to the ruler goes first, so longer bars lie over it
        s.line((RULER + 6.0, end), (x, end), "thin dash");
        s.rect(
            (x, full_scale),
            (BAR_WIDTH, end - full_scale),
            0.0,
            &format!("db-{}", bar.hue),
        );
        s.line((x, end), (x + BAR_WIDTH, end), &format!("{}-line", bar.hue));
        s.text(x, end + bar.drop + 19.0, "start", "lg strong", &bar.value);
        s.text(x, end + bar.drop + 36.0, "start", "soft", bar.name);
        x += BAR_STEP;
    }
    s.line((RULER, full_scale), (WIDTH - 2.0, full_scale), "line");
    s.text(x + 2.0, full_scale + 19.0, "start", "lg strong", "0 dBFS");
    s.text(x + 2.0, full_scale + 36.0, "start", "soft", "full scale");
    // the legend, in the free space beside the shortest bar
    let legend = [
        ("db-green-key", "ideal ADC floor", "6.02·N + 1.76 dB", -21.0),
        ("db-purple-key", "smallest step", "of a stored value", -35.0),
    ];
    for (class, first, second, level) in legend {
        let row = y(level);
        s.rect((x + 2.0, row - 10.0), (10.0, 10.0), 2.0, class);
        s.text(x + 17.0, row, "start", "", first);
        s.text(x + 17.0, row + 16.0, "start", "soft", second);
    }
}

/// The error of the rule of thumb, tinted as the rows it is about.
fn thumb_note(s: &mut Svg) {
    let x = BARS;
    let power_of_2 = PowerGain::bounded(2.0).to_db();
    let amplitude_of_2 = VoltGain::bounded(2.0).to_db();
    let plus_3 = PowerMicro::from_db_const_rounded(Decibels::new(3, 0), NearestEven).get();
    let short = (1.0 - f64::from(plus_3) / 2e6) * 100.0;
    s.text(
        x,
        TOP + 6.0,
        "start",
        "lg strong",
        "≈ ×2 is close, not exact",
    );
    s.rect((x - 4.0, TOP + 13.0), (216.0, 20.0), 4.0, "tint-accent");
    let power = format!("×2 power = +{power_of_2:.4} dB");
    s.text(x, TOP + 27.0, "start", "mono", &power);
    s.rect((x - 4.0, TOP + 36.0), (216.0, 20.0), 4.0, "tint-blue");
    let amplitude = format!("×2 volts = +{amplitude_of_2:.4} dB");
    s.text(x, TOP + 50.0, "start", "mono", &amplitude);
    let error = format!("+3 dB is {short:.2} % short of ×2");
    s.text(x, TOP + 74.0, "start", "mono soft", &error);
}

fn headers(s: &mut Svg) {
    let t = TOP - 42.0;
    s.text(AMPLITUDE, t, "end", "blue strong", "amplitude");
    s.text(AMPLITUDE, t + 17.0, "end", "soft", "×10 per 20 dB");
    s.text(POWER, t, "end", "accent strong", "power");
    s.text(POWER, t + 17.0, "end", "soft", "×10 per 10 dB");
    s.text(DECIBEL, t, "end", "strong", "dB");
}

pub fn db_scale() -> String {
    // the page's own numbers: the figure can't drift from them
    const MINUS_6: Level = db!(-6, NearestEven);
    assert!(MINUS_6.get() == 501);
    let power: Fixed<Per<Watt, Watt>, Milli, u16> = db!(-6, NearestEven);
    assert!(power.get() == 251);

    let mut s = Svg::new(
        WIDTH,
        HEIGHT,
        "A dB ruler from +20 to −150 dB with the amplitude and power ratio at each level: +6 dB is ×1.995 in amplitude, \
         +3 dB is ×1.995 in power. Hanging from full scale down the ruler: the smallest step of a mV/V u16 at −60 dBFS, \
         the noise floor of ideal ADCs at −74.0 (12-bit), −98.1 (16-bit) and −146.3 dBFS (24-bit), and Q15's smallest \
         step at −90.3 dBFS",
    );
    headers(&mut s);
    ruler(&mut s);
    landmarks(&mut s);
    decades(&mut s);
    anchors(&mut s);
    thumb_note(&mut s);
    s.finish()
}
