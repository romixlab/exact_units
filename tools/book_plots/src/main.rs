//! The book's plots, each computed by exact_units itself, written as SVG into the folder given (`docs/plots`):
//!
//! - `rounding.svg`: the five `Rounding` modes as staircases, tenths rounded to whole numbers by `to_rounded`.
//! - `saturate_wrap.svg`: a Q15 sum past full scale, saturating (`Q15`) against wrapping (`Wrapping`).
//! - `cordic_error.svg`: the error of `cordic::sin_cos_16` against `f64` sine over a turn, in Q15 steps.
//! - `db_step.svg`: the step in dB between neighbouring stored gains of a `Fixed<Per<Volt, Volt>, Milli, u16>`.
//! - `db_scale.svg`: a dB ruler with amplitude and power ratios, converter noise floors and the steps of stored values.
//! - `neper_cable.svg`: a wave dying along an RG-58 cable, marked at 1, 2 and 3 Np, and 1 Np as one level of two ratios.
//!
//! `cargo run --manifest-path tools/book_plots/Cargo.toml -- docs/plots` (`just docs-build` does it).

mod db_scale;
mod neper_cable;
mod svg;

use std::{fs, path::Path, process::ExitCode};

use exact_units::{
    Bam16, Fixed, Q15, Rounding, Wrapping, cordic,
    prefix::{Deci, Milli, One, Pow2},
    units::{Per, Ratio, Volt},
};
use svg::{Panel, Svg};

fn main() -> ExitCode {
    let Some(dir) = std::env::args().nth(1) else {
        eprintln!("usage: book_plots OUT_DIR");
        return ExitCode::FAILURE;
    };
    let dir = Path::new(&dir);
    if let Err(e) = fs::create_dir_all(dir) {
        eprintln!("{}: {e}", dir.display());
        return ExitCode::FAILURE;
    }
    let plots = [
        ("rounding.svg", rounding()),
        ("saturate_wrap.svg", saturate_wrap()),
        ("cordic_error.svg", cordic_error()),
        ("db_step.svg", db_step()),
        ("db_scale.svg", db_scale::db_scale()),
        ("neper_cable.svg", neper_cable::neper_cable()),
    ];
    for (name, svg) in plots {
        let path = dir.join(name);
        if let Err(e) = fs::write(&path, svg) {
            eprintln!("{}: {e}", path.display());
            return ExitCode::FAILURE;
        }
    }
    ExitCode::SUCCESS
}

/// Five small panels, one per mode: x from -3 to 3 in tenths, the whole number `to_rounded` makes of it, with the
/// line y = x behind.
fn rounding() -> String {
    let modes = [
        (Rounding::Truncate, "Truncate"),
        (Rounding::Floor, "Floor"),
        (Rounding::Ceil, "Ceil"),
        (Rounding::NearestEven, "NearestEven"),
        (Rounding::NearestAway, "NearestAway"),
    ];
    let (pw, ph, gap, left, top) = (150.0, 150.0, 22.0, 34.0, 30.0);
    let width = left + 5.0 * pw + 4.0 * gap + 10.0;
    let mut s = Svg::new(
        width,
        top + ph + 34.0,
        "Staircases of the five rounding modes from -3 to 3",
    );
    for (i, (mode, name)) in modes.into_iter().enumerate() {
        let x0 = left + f64::from(u8::try_from(i).unwrap_or(0)) * (pw + gap);
        let p = Panel {
            area: (x0, top, pw, ph),
            x: (-3.0, 3.0),
            y: (-3.0, 3.0),
        };
        let ticks: [(f64, &str); 3] = [(-2.0, "−2"), (0.0, "0"), (2.0, "2")];
        s.axes(&p, &ticks, if i == 0 { &ticks } else { &[] }, "", "");
        s.text(x0 + pw / 2.0, top - 10.0, "middle", "mono", name);
        s.path(&p, &[(-3.0, -3.0), (3.0, 3.0)], "thin dash");
        let mut prev: Option<(f64, i16)> = None;
        for tenths in -30i16..=30 {
            let x = f64::from(tenths) / 10.0;
            let v: Fixed<Ratio, Deci, i16> = Fixed::new(tenths);
            let Some(r) = v.to_rounded::<One, i16>(mode) else {
                continue;
            };
            let y = f64::from(r.get());
            if let Some((px, pr)) = prev
                && pr == r.get()
            {
                s.path(&p, &[(px, y), (x, y)], "accent-line");
            }
            prev = Some((x, r.get()));
            // the halves are where the nearest modes differ: mark them
            if tenths % 10 == 5 || tenths % 10 == -5 {
                s.circle((p.px(x), p.py(y)), 3.0, "accent");
            }
        }
    }
    s.finish()
}

/// a + b for a fixed a = 0.75 and b from -1 to 1 in Q15: the saturating `Q15` and the wrapping one.
fn saturate_wrap() -> String {
    let (w, h) = (640.0, 300.0);
    let mut s = Svg::new(
        w,
        h,
        "A Q15 sum past full scale: saturating stays at the end, wrapping jumps to the other end",
    );
    let p = Panel {
        area: (50.0, 30.0, 560.0, 220.0),
        x: (-1.0, 1.0),
        y: (-1.0, 1.0),
    };
    s.axes(
        &p,
        &[
            (-1.0, "−1"),
            (-0.5, "−0.5"),
            (0.0, "0"),
            (0.5, "0.5"),
            (1.0, "1"),
        ],
        &[(-1.0, "−1"), (0.0, "0"), (1.0, "1")],
        "b (a = 0.75)",
        "a + b in Q15",
    );
    let a: i16 = 24576; // 0.75
    let mut sat = Vec::new();
    let mut wrap = Vec::new();
    for b in (-32768i32..=32767).step_by(256) {
        let Ok(b) = i16::try_from(b) else { continue };
        let x = f64::from(b) / 32768.0;
        sat.push((
            x,
            f64::from((Q15::<Ratio>::of(a) + Q15::of(b)).get()) / 32768.0,
        ));
        let wa: Fixed<Ratio, Pow2<-15>, i16, Wrapping> = Fixed::of(a);
        wrap.push((x, f64::from((wa + Fixed::of(b)).get()) / 32768.0));
    }
    // the wrapping line jumps from +1 to -1: draw it in two pieces
    let cut = wrap
        .windows(2)
        .position(|w| w[1].1 < w[0].1)
        .map_or(wrap.len(), |i| i + 1);
    s.path(&p, &wrap[..cut], "blue-line");
    s.path(&p, &wrap[cut..], "blue-line");
    s.path(&p, &sat, "accent-line");
    // under the flat end, right-aligned to the panel so it stays inside the figure
    s.text(
        p.px(1.0),
        p.py(0.88),
        "end",
        "accent",
        "Saturating: stays at 1 − 2⁻¹⁵",
    );
    // left of where the wrapped line comes back in at -1, clear of both curves
    s.text(
        p.px(-0.95),
        p.py(-0.82),
        "start",
        "blue",
        "Wrapping: modulo 2¹⁶ steps",
    );
    s.finish()
}

/// The largest error of `sin_cos_16`'s sine against the exact sine in each 1/512 turn, in steps of 2⁻¹⁵: a
/// correctly rounded sine is within ½ step, CORDIC's within 0.51.
fn cordic_error() -> String {
    let (w, h) = (640.0, 260.0);
    let mut s = Svg::new(
        w,
        h,
        "The largest error of the CORDIC sine over a turn, at most 0.51 of a step",
    );
    let p = Panel {
        area: (60.0, 30.0, 550.0, 180.0),
        x: (0.0, 1.0),
        y: (0.44, 0.52),
    };
    s.axes(
        &p,
        &[
            (0.0, "0"),
            (0.25, "¼"),
            (0.5, "½"),
            (0.75, "¾"),
            (1.0, "1 turn"),
        ],
        &[(0.45, "0.45"), (0.5, "0.50"), (0.51, "0.51")],
        "angle (Bam16)",
        "largest |error| in Q15 steps",
    );
    let mut worst = Vec::new();
    for bin in 0u32..512 {
        let mut max = 0.0f64;
        for k in 0..128u32 {
            let Ok(raw) = u16::try_from(bin * 128 + k) else {
                continue;
            };
            let (sin, _) = cordic::sin_cos_16(Bam16::of(raw));
            let exact = (f64::from(raw) / 65536.0 * core::f64::consts::TAU).sin() * 32768.0;
            max = max.max((f64::from(sin.get()) - exact.min(32767.0)).abs());
        }
        worst.push(((f64::from(bin) + 0.5) / 512.0, max.max(0.44)));
    }
    s.path(&p, &worst, "accent-line");
    s.finish()
}

/// For each stored gain n of a `Fixed<Per<Volt, Volt>, Milli, u16>`, how far apart n and n + 1 are in dB: the
/// resolution of a threshold written with `db!`, finest at the top of the range.
fn db_step() -> String {
    type Gain = Fixed<Per<Volt, Volt>, Milli, u16>;
    let (w, h) = (640.0, 280.0);
    let mut s = Svg::new(
        w,
        h,
        "The step between neighbouring stored gains in dB, against the level",
    );
    let p = Panel {
        area: (60.0, 30.0, 550.0, 190.0),
        x: (-60.0, 40.0),
        y: (-3.5, 1.0),
    };
    s.axes(
        &p,
        &[
            (-60.0, "−60"),
            (-40.0, "−40"),
            (-20.0, "−20"),
            (0.0, "0"),
            (20.0, "20"),
            (40.0, "40 dB"),
        ],
        &[
            (-3.0, "0.001"),
            (-2.0, "0.01"),
            (-1.0, "0.1"),
            (0.0, "1"),
            (1.0, "10 dB"),
        ],
        "level of the stored gain",
        "step to the next stored gain (log scale)",
    );
    let mut pts = Vec::new();
    for n in 1u16..u16::MAX {
        // every value at the low end, fewer where the curve is smooth
        if n > 200 && n % 16 != 0 {
            continue;
        }
        let level = Gain::new(n).to_db();
        let step = Gain::new(n + 1).to_db() - level;
        if level >= -60.0 && step.log10() >= -3.5 {
            pts.push((level, step.log10()));
        }
    }
    s.path(&p, &pts, "accent-line");
    s.finish()
}
