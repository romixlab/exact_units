//! 18. Photographic exposure from Exif 3.1: APEX values as exact stops, the labels a camera marks, conventions.
//!
//! Exif (CIPA DC-008-2026) records exposure in APEX (Annex C): Av = 2·log2 N of the f-number, Tv = −log2 t of the
//! time, Bv of the brightness, Sv = log2(S/3.125) of the speed, and Ev = Av + Tv = Bv + Sv. Each is a level in
//! stops, `units::Stop`, (ln 2)/2 Np: in exposure the logarithm is the exact quantity and the linear value the
//! edge, the reverse of a level in dB. A whole stop's f-number is √2^Av, a `Magnitude` half power; a third stop's,
//! 2^(1/6) per step, is irrational and only an `f64`.
//!
//! A camera marks labels, not values: f/11 is 8√2 ≈ 11.31, "1/125" is 1/128 s. Exif's own sample file (Table 45)
//! records ExposureTime 1/60 s beside ShutterSpeedValue 6, which is 1/64 s: the label and the value, side by side.
//!
//! The meter's calibration constants K (reflected) and C (incident) are set by ISO 2720, which is paid and was not
//! read for this example: the values below are the ones cameras are widely said to use, marked unverified, and each
//! is the caller's named choice, as is which ISO speed a number is (Exif's SensitivityType) and whether "EV" means
//! a camera setting or a light value at ISO 100.
//!
//! `cargo run --example 18_photo_exposure --features levels,photometric,time,libm`

#![expect(
    clippy::print_stdout,
    clippy::unwrap_used,
    reason = "an example prints what it shows and unwraps values it knows are there"
)]
#![expect(
    clippy::arithmetic_side_effects,
    reason = "APEX values here are a few stops, far from i16's ends; a stop count is below 128"
)]

use exact_units::codec::{Codec, Coded};
use exact_units::exposure::{
    ApertureLabel, Apex, SensitivityType, ShutterLabel, apex_from_exif, exposure_time,
    exposure_time_f64, f_number, f_number_f64, sensitivity_value,
};
use exact_units::prefix::{One, Rational};
use exact_units::units::{Decibel, Lux, Nit, Per, Second, Stop, Times};
use exact_units::{Exact, Fixed, Fraction, Magnitude, Rounding};

/// A photographic exposure H, lx·s: the illuminance on the sensor times the time.
type Exposure = Times<Lux, Second>;

/// The reflected-light meter constant K, in cd·s/m² (N²/t = L·S/K). No default: the caller names the one their
/// meter uses. 12.5 and 14 are the values cameras are widely said to use; ISO 2720 sets the range and is paid, so
/// both are unverified here.
#[derive(Copy, Clone)]
struct ReflectedK(Exact<Times<Nit, Second>>);

/// The incident-light meter constant C, in lx·s (N²/t = E·S/C), the same: 250 for a flat receptor and 320 for a
/// dome are the values usually cited, unverified against ISO 2720.
#[derive(Copy, Clone)]
struct IncidentC(Exact<Exposure>);

/// "EV" as a camera setting, Av + Tv: the same number at any ISO.
#[derive(Copy, Clone, Debug, PartialEq)]
struct CameraEv(Apex);

/// A light value: the camera EV that meters a scene at ISO 100 (Sv 5), Bv + 5. People write both as "EV"; here
/// each has its own name and the step between them names the ISO.
#[derive(Copy, Clone, Debug, PartialEq)]
struct LightValue(Apex);

impl LightValue {
    /// The camera setting that exposes this scene at `sv`: Ev = Bv + Sv = LV − 5 + Sv.
    fn camera_ev_at(self, sv: Apex) -> CameraEv {
        CameraEv(self.0 - sensitivity_value(100).unwrap() + sv)
    }
}

/// A RATIONAL or SRATIONAL from little-endian Exif bytes (an "II" file), as Exif's Table 45 prints them.
fn rational_le(bytes: [u8; 8]) -> (i64, i64) {
    let [n0, n1, n2, n3, d0, d1, d2, d3] = bytes;
    (
        i64::from(u32::from_le_bytes([n0, n1, n2, n3])),
        i64::from(u32::from_le_bytes([d0, d1, d2, d3])),
    )
}

/// The whole number of stops k of a ratio that is 2^k exactly, `None` for any other: log2 of a rational is rational
/// only for a power of two. The crate has no exact logarithm of an `Exact`; this is the case APEX needs.
fn whole_stops(x: Fraction) -> Option<i16> {
    let (num, den) = (x.numer()?, x.denom()?);
    if num.is_power_of_two() && den.is_power_of_two() {
        i16::try_from(i64::from(num.trailing_zeros()) - i64::from(den.trailing_zeros())).ok()
    } else {
        None
    }
}

/// 2^k as a fraction.
fn pow2(k: i16) -> Fraction {
    Fraction::pow2(i32::from(k)).unwrap()
}

fn main() {
    let (av, tv) = exif_sample();
    meters(av, tv);
    stops_and_decibels();
    labels();
    third_stops();
    conventions();
}

/// Exif's sample file: its APEX values and the labels beside them. Gives Av and Tv.
fn exif_sample() -> (Apex, Apex) {
    // 1. Exif's sample file, Table 45: ExposureTime 1/60, FNumber 4/1, ShutterSpeedValue 6/1, ApertureValue 4/1,
    //    BrightnessValue 0/1, each a RATIONAL or SRATIONAL in little-endian bytes
    let exposure_time_tag = rational_le([0x01, 0, 0, 0, 0x3C, 0, 0, 0]);
    let f_number_tag = rational_le([0x04, 0, 0, 0, 0x01, 0, 0, 0]);
    let tv = apex_from_exif(rational_le([0x06, 0, 0, 0, 0x01, 0, 0, 0]).0, 1).unwrap();
    let av = apex_from_exif(rational_le([0x04, 0, 0, 0, 0x01, 0, 0, 0]).0, 1).unwrap();
    let bv = apex_from_exif(0, 1).unwrap();
    assert_eq!((exposure_time_tag, f_number_tag), ((1, 60), (4, 1)));

    // Av 4 is f/4 exactly, the FNumber tag; Tv 6 is 1/64 s, and the ExposureTime tag says 1/60, the dial's label
    assert_eq!(f_number(av), Some(Magnitude::ratio(4, 1)));
    assert_eq!(exposure_time(tv), Some(Magnitude::ratio(1, 64)));
    let shutter = Coded::<ShutterLabel>::try_new(tv, Rounding::NearestEven).unwrap();
    assert_eq!(ShutterLabel::label(shutter.raw()), Some("1/60"));
    assert_eq!(
        ShutterLabel::MARKED.get(usize::from(shutter.raw())),
        Some(&(1, 60))
    );

    // 1/60 s against 1/64 s: the exposure ratio 64/60 in stops, 0.093, is a float edge (a log of a rational)
    let off = Fixed::<Per<Exposure, Exposure>, One, f64>::bounded(64.0 / 60.0)
        .to_level::<Stop, One, f64>(Rounding::NearestEven)
        .unwrap();
    println!(
        "ExposureTime 1/60 s is {:.3} stop more light than Tv 6 (1/64 s)",
        off.get()
    );

    // 2. Ev = Av + Tv, exact in sixths of a stop
    let ev = CameraEv(av + tv);
    assert_eq!(ev, CameraEv(Apex::new(60)));
    println!(
        "Av {} + Tv {} = Ev {} (Bv {} in the file)",
        av.get() / 6,
        tv.get() / 6,
        ev.0.get() / 6,
        bv.get()
    );
    (av, tv)
}

/// The meter equations, exact for whole stops, under each named K and C.
fn meters(av: Apex, tv: Apex) {
    let ev = CameraEv(av + tv);
    // 3. the meter equation N²/t = L·S/K, exact for whole stops: Av 4, Tv 6 at ISO 100 (Sv 5)
    let s = 100;
    let sv = sensitivity_value(s).unwrap();
    for (name, k) in [
        ("K = 12.5", ReflectedK(Exact::new(25, 2))),
        ("K = 14", ReflectedK(Exact::int(14))),
    ] {
        let t = Exact::<Second>::of(Fraction::from_magnitude(&exposure_time(tv).unwrap()).unwrap());
        // N² = 2^Av, the light lost to the aperture
        let n2_over_s = pow2(av.get() / 6)
            .checked_div(Fraction::int(s.into()))
            .unwrap();
        let l: Exact<Nit> =
            k.0.checked_per(t)
                .unwrap()
                .convert::<Nit>()
                .unwrap()
                .checked_scale(n2_over_s)
                .unwrap();
        // Bv = log2(L/(N·K)) with Annex C's N = 1/3.125 (Sv = log2(S·N)): the same Bv for either K
        let n_k = k.0.get().checked_mul(Fraction::new(8, 25)).unwrap();
        let bv = whole_stops(l.get().checked_div(n_k).unwrap()).unwrap();
        assert_eq!(Apex::new(bv * 6) + sv, ev.0); // Ev = Bv + Sv
        println!(
            "{name} cd·s/m²: f/4 at 1/64 s, ISO 100 meters L = {} cd/m², Bv {bv}",
            l.get().to_f64()
        );
    }

    // the incident meter: N²/t = E·S/C, the illuminance on the subject
    for (name, c) in [
        ("C = 250", IncidentC(Exact::int(250))),
        ("C = 320", IncidentC(Exact::int(320))),
    ] {
        let t = Exact::<Second>::of(Fraction::from_magnitude(&exposure_time(tv).unwrap()).unwrap());
        let n2_over_s = pow2(av.get() / 6)
            .checked_div(Fraction::int(s.into()))
            .unwrap();
        let e: Exact<Lux> =
            c.0.checked_per(t)
                .unwrap()
                .convert::<Lux>()
                .unwrap()
                .checked_scale(n2_over_s)
                .unwrap();
        println!(
            "{name} lx·s: f/4 at 1/64 s, ISO 100 meters E = {} lx on the subject",
            e.get().to_f64()
        );
    }
}

/// A stop of exposure in stops and in dB.
fn stops_and_decibels() {
    // 4. a stop is a factor of 2 in light, 3.0103 dB of power: exposures double, stops add
    let h2 = Fixed::<Per<Exposure, Exposure>, One, u32>::new(2);
    assert_eq!(
        h2.to_level::<Stop, One, f64>(Rounding::NearestEven)
            .unwrap()
            .get(),
        1.0
    );
    let db = h2
        .to_level::<Decibel, Rational<1, 10000>, i32>(Rounding::NearestEven)
        .unwrap();
    assert_eq!(db.get(), 30103);
    let h = Fixed::<Per<Exposure, Exposure>, One, u32>::new(1 << 10);
    assert_eq!(
        h.to_level::<Stop, One, i8>(Rounding::Floor),
        Some(Fixed::new(10))
    );
}

/// The labels of Exif's Table 55 against their exact values.
fn labels() {
    // 5. labels: f/11 is Av 7, N = 8√2; f/5.6 is 4√2; the label is never the value
    let f11 = Coded::<ApertureLabel>::try_from_raw(7).unwrap();
    assert_eq!(ApertureLabel::label(f11.raw()), Some("11"));
    assert_eq!(
        f_number(f11.get()),
        Some(Magnitude::pow2(3).mul(Magnitude::pow2(1).sqrt()))
    );
    println!(
        "f/11 is N = {:.4}, f/5.6 is {:.4}",
        f_number_f64(f11.get()),
        f_number_f64(Apex::new(30))
    );
    // a half stop, Av 4.5, is a tie between f/4 and f/5.6: the rounding is named
    let half = Apex::new(27);
    assert_eq!(ApertureLabel::encode(half, Rounding::Floor), Some(4));
    assert_eq!(ApertureLabel::encode(half, Rounding::Ceil), Some(5));
    assert_eq!(ApertureLabel::encode(half, Rounding::NearestEven), Some(4)); // the even code, f/4
    // past f/32 the nearest label is f/32, and there is none above it
    assert_eq!(
        ApertureLabel::encode(Apex::new(66), Rounding::NearestEven),
        Some(10)
    );
    assert_eq!(ApertureLabel::encode(Apex::new(66), Rounding::Ceil), None);
}

/// Third stops: exact in stops, a float edge in linear values.
fn third_stops() {
    // 6. third stops: the stop is exact, the linear value the float edge
    let third = Apex::new(26); // Av 4⅓
    assert_eq!(f_number(third), None); // 2^(13/6): no Magnitude
    println!("Av 4⅓ is f/{:.3} (marked 4.5)", f_number_f64(third));
    println!(
        "Tv 7⅓ is {:.6} s (marked 1/160)",
        exposure_time_f64(Apex::new(44))
    );
    // ISO 125 and 160 are labels for 100·2^(1/3) and 100·2^(2/3): their Sv is no exact value, 125 is refused
    assert_eq!(sensitivity_value(125), None);
    let sv_third = sensitivity_value(100).unwrap() + Apex::new(2);
    let iso_third = 3.125
        * Fixed::<Stop, Rational<1, 6>, i16>::new(sv_third.get())
            .to_gain::<Per<Exposure, Exposure>, One, f64>()
            .unwrap()
            .get();
    println!("Sv 5⅓ is ISO {iso_third:.2}, marked 125");
}

/// The conventions the crate leaves to the caller: LV against EV, which ISO.
fn conventions() {
    // 7. EV against LV: a scene of LV 15 (sunlight, Bv 10) at ISO 400 is a camera EV of 17
    let lv = LightValue(Apex::new(90));
    assert_eq!(
        lv.camera_ev_at(sensitivity_value(400).unwrap()),
        CameraEv(Apex::new(102))
    );

    // 8. which ISO: tag 34864 says; 0 is unknown, never taken for ISO speed
    let kind = SensitivityType::from_tag(2).unwrap();
    assert!(kind.includes_rei() && !kind.includes_iso_speed());
    assert_eq!(SensitivityType::from_tag(0), Some(SensitivityType::Unknown));

    // An f-number is a ratio of lengths, which has no level form: Av is 2·log2 N by definition, not the level of N.
    // let _ = Fixed::<Per<Meter, Meter>, One, f64>::bounded(4.0).to_level::<Stop, One, f64>(Rounding::NearestEven);
    // error[E0080]: evaluation panicked: decibels are for a ratio of powers (W/W) or of root-power quantities
    //               (V/V, A/A, Pa/Pa): a plain Ratio or another kind has no level form
}
