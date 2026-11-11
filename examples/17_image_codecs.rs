//! 17. The integer maths of image and video codecs: JPEG, JPEG 2000, H.264, H.265, H.273 and AV1.
//!
//! Codecs compute in integers on purpose: a decoder must give the same bits on every machine. Their standards name
//! a rounding in each formula, or forget to, and they print irrational constants as decimals. This example walks
//! through what is exact in them, which rounding each step is, where a formula was printed wrong, and where a
//! constant's printed digits round differently from the exact value.
//!
//! - **Four roundings in one chain.** JPEG quantizes "to the nearest integer" with no tie named (ITU-T T.81 A.3.4).
//!   Its point transform truncates for AC coefficients and floors for DC ones (A.4, K.10). JFIF rounds a half up
//!   (T.871 clause 7), as AV1's Round2 and the add-half-then-shift of H.264 and H.265 do: `NearestCeil`. H.273
//!   rounds a half away from zero (eq. 8), `NearestAway`. Each is a named [Rounding] here.
//! - **Lossless transforms.** JPEG 2000's 5/3 wavelet and reversible colour transform (T.800 F.3.8.1, G.2) and
//!   H.273's YCgCo-R (eq. 58 to 65) are integer lifting: every input comes back. `tests/props/image_codecs.rs` walks
//!   them whole and proves them with Kani.
//! - **Mantissa times a power of two.** JPEG 2000's step size, H.265's `levelScale << (qP / 6)` and the HRD's bit
//!   rate are one codec shape, `CodecShape::MantissaExponent`: [J2kStepSize], [HevcLevelScale], [HrdBitRate].
//! - **Irrational constants.** The 9/7 wavelet's taps (T.800 Table F.4) and AV1's cosines (7.13.2) have closed
//!   forms with roots and π. They are float edges here; the properties certify every printed digit.
//! - **Tables and matrices.** H.264's sample aspect ratios are a table codec over 1/198. JFIF's and JPEG 2000's
//!   colour matrices come exactly from K_R = 0.299 and K_B = 0.114. JFIF's density byte picks the unit.
//!
//! Every number names its clause. Where the crate has no place for something, the comment says so.
//!
//! `cargo run --example 17_image_codecs --features dimensionless,customary,q,information,time`

#![expect(
    clippy::print_stdout,
    clippy::unwrap_used,
    reason = "an example prints what it shows and unwraps values it knows are there"
)]
#![expect(
    clippy::arithmetic_side_effects,
    reason = "samples are 8 to 16 bits in an i32; an Exact's arithmetic gives an Option, unwrapped where it fits"
)]

use core::f64::consts::PI;

use exact_units::codec::{
    CodeSpan, Codec, CodecDef, CodecShape, Coded, HevcLevelScale, HrdBitRate, J2kStepSize,
};
use exact_units::niche::{Niched, Reading};
use exact_units::prefix::{One, Pow2, Rational};
use exact_units::system::def_list;
use exact_units::units::{Inch, Meter, Per, Ratio};
use exact_units::{Exact, Fixed, MarkerId, Overflow, Prefix, Rounding, niches, q};

/// Every rounding, for the tables below.
const ROUNDINGS: [Rounding; 6] = [
    Rounding::Truncate,
    Rounding::Floor,
    Rounding::Ceil,
    Rounding::NearestEven,
    Rounding::NearestAway,
    Rounding::NearestCeil,
];

/// `p / 2^k` rounded by `r`, with `k` known only at run time: `p` in steps of 2⁻ᵏ, rescaled to whole ones.
fn shift(p: i32, k: i8, r: Rounding) -> i32 {
    Prefix::from_pow2(-k)
        .and_then(|from| from.rescale(p, Prefix::Unit, r, Overflow::Checked))
        .unwrap()
}

/// `x / 2` and `x / 4` with the shift's rounding, `Floor`, as a compile-time scale.
fn half_floor(x: i32) -> i32 {
    Fixed::<Ratio, Pow2<-1>, i32>::new(x)
        .into_rounded::<One, i32>(Rounding::Floor)
        .get()
}

/// See [half_floor].
fn quarter_floor(x: i32) -> i32 {
    Fixed::<Ratio, Pow2<-2>, i32>::new(x)
        .into_rounded::<One, i32>(Rounding::Floor)
        .get()
}

fn jpeg_roundings() {
    println!("JPEG: quantization rounds 'to the nearest integer' (T.81 A.3.4), the tie unnamed");
    // Sq = round(S / Q): Q is one of 64 run-time table entries (B.2.4.1), so the step is an `Exact`, not a scale
    for (s, q) in [(20, 8), (-20, 8), (12, 8), (13, 8)] {
        let quotient = Exact::<Ratio>::new(s, q);
        let rounded = ROUNDINGS.map(|r| quotient.to_rounded::<One, i16>(r).unwrap().get());
        println!(
            "  S = {s:>3}, Q = {q}: S/Q = {quotient}, Truncate/Floor/Ceil/NearestEven/NearestAway/NearestCeil {rounded:?}"
        );
        // dequantization, Rvu = Sqvu × Qvu, is normative and exact
        let r = rounded[3];
        println!(
            "    dequantized from NearestEven: {}",
            i128::from(r) * q.cast_signed()
        );
    }
    println!(
        "  20/8 = 2.5: NearestEven gives 2, NearestAway 3. T.81 leaves that to the encoder; the crate makes it named."
    );

    println!("JPEG: the point transform (A.4) divides by 2^Pt two ways; K.10's table, Pt = 1:");
    println!("  before level shift | after | divide (Truncate) | shift right (Floor)");
    for before in (123..=131).rev() {
        let x = before - 128;
        let divided = shift(x, 1, Rounding::Truncate);
        let shifted = shift(x, 1, Rounding::Floor);
        println!("  {before:>18} | {x:>5} | {divided:>17} | {shifted:>19}");
    }
    // A.4: the shift "is equivalent to dividing by 2^Pt before the level shift"
    assert!((0..256).all(|x| shift(x - 128, 3, Rounding::Floor) == (x >> 3) - (128 >> 3)));

    println!("JFIF: Round(x) = floor(x + 0.5) (T.871 clause 7): a half goes up, toward +infinity");
    println!(
        "  AV1's Round2(x, n) = (x + 2^(n-1)) >> n (4.7), H.265's eq. 8-309 and H.264's eq. 8-337 are the same"
    );
    for p in [-3, -1, 1, 3] {
        // the standards write it as floor of x plus half a step; it is NearestCeil, and no other rounding
        let up = shift(p + 1, 1, Rounding::Floor);
        assert_eq!(shift(p, 1, Rounding::NearestCeil), up);
        let near = ROUNDINGS.map(|r| shift(p, 1, r));
        println!(
            "  {p}/2: half up {up}, Truncate/Floor/Ceil/NearestEven/NearestAway/NearestCeil {near:?}"
        );
    }

    println!(
        "H.273: Round(x) = Sign(x) * Floor(Abs(x) + 0.5) (eq. 8), a half away from zero: NearestAway"
    );
    // YCgCo's Cb (eq. 52) = Round(0.5 G - 0.25 (R + B)) + (1 << (BitDepthC - 1)), on 8 bits
    let cb = |r: i32, g: i32, b: i32| {
        Fixed::<Ratio, Pow2<-2>, i32>::new(2 * g - r - b)
            .into_rounded::<One, i32>(Rounding::NearestAway)
            .get()
            + 128
    };
    let green = cb(0, 255, 0);
    println!(
        "  YCgCo Cb of pure green (0, 255, 0): {green}, one past 8 bits, as H.273's NOTE 3 warns"
    );
    // a bounded type refuses it instead of wrapping it to 0
    assert_eq!(Fixed::<Ratio, One, i32>::new(green).to::<One, u8>(), None);
}

/// T.800's forward 5/3 (F.4.8.1, eq. F-9 and F-10) on a signal of even length, the ends mirrored (F.3.7).
fn fdwt53(x: &[i32]) -> Vec<i32> {
    let n = x.len().cast_signed();
    let ext = |i: isize| {
        let m = i.rem_euclid(2 * (n - 1));
        x.get(m.min(2 * (n - 1) - m).unsigned_abs())
            .copied()
            .unwrap()
    };
    let odd = |k: isize| ext(k) - half_floor(ext(k - 1) + ext(k + 1));
    (0..n)
        .map(|k| {
            if k % 2 == 1 {
                odd(k)
            } else {
                ext(k) + quarter_floor(odd(k - 1) + odd(k + 1) + 2)
            }
        })
        .collect()
}

/// T.800's inverse 5/3 (F.3.8.1, eq. F-5 and F-6).
fn idwt53(y: &[i32]) -> Vec<i32> {
    let n = y.len().cast_signed();
    let ext = |i: isize| {
        let m = i.rem_euclid(2 * (n - 1));
        y.get(m.min(2 * (n - 1) - m).unsigned_abs())
            .copied()
            .unwrap()
    };
    let even = |k: isize| ext(k) - quarter_floor(ext(k - 1) + ext(k + 1) + 2);
    (0..n)
        .map(|k| {
            if k % 2 == 1 {
                ext(k) + half_floor(even(k - 1) + even(k + 1))
            } else {
                even(k)
            }
        })
        .collect()
}

fn lossless() {
    println!("JPEG 2000's 5/3 wavelet (T.800 F.3.8.1): lifting with floors, every sample back");
    let x = [-128, 5, 127, 0, -1, 64, 3, -77];
    let y = fdwt53(&x);
    println!("  {x:?} -> {y:?}");
    assert_eq!(idwt53(&y), x);
    // F-9 as T.800 (08/2002) prints it divides by 4, where F-6 inverts a divide by 2: [2, 0] comes back [2, 1].
    // Technical Corrigendum 4 (10/2014) corrects the divisor to 2; the 06/2019 edition prints it right.
    println!(
        "  F-9 is printed with /4 in 08/2002; F-6 undoes /2. With /4, [2, 0] would come back as [2, 1]."
    );
    println!("  Corrigendum 4 (10/2014) fixes it to /2; 06/2019 prints it right.");

    println!("JPEG 2000's reversible colour transform (G.2, eq. G-3 to G-8)");
    let (i0, i1, i2) = (200 - 128, 30 - 128, 255 - 128);
    let y = [quarter_floor(i0 + 2 * i1 + i2), i2 - i1, i0 - i1];
    let back1 = y[0] - quarter_floor(y[2] + y[1]);
    println!(
        "  ({i0}, {i1}, {i2}) -> {y:?} -> ({}, {back1}, {})",
        y[2] + back1,
        y[1] + back1
    );

    println!("H.273's YCgCo-R (eq. 58 to 65), 8-bit RGB, chroma on 9 bits");
    let (r, g, b) = (200, 30, 255);
    let co = r - b + 256;
    let t = b + half_floor(co - 256);
    let cg = g - t + 256;
    let luma = t + half_floor(cg - 256);
    let t2 = luma - half_floor(cg - 256);
    let (g2, b2) = (t2 + cg - 256, t2 - half_floor(co - 256));
    println!(
        "  ({r}, {g}, {b}) -> Y {luma}, Cg {cg}, Co {co} -> ({}, {g2}, {b2})",
        b2 + co - 256
    );
    assert_eq!((b2 + co - 256, g2, b2), (r, g, b));
}

fn mantissa_exponent() {
    println!(
        "JPEG 2000's step size (T.800 E.1.1.1, eq. E-3): 5 bits of exponent, 11 of mantissa, a 1 implied"
    );
    // εb = 8, μb = 1024: Δb = 2^(Rb − 8) · (1 + 1024/2048), so Δb / 2^Rb = 3/512
    let raw = (8 << 11) | 1024;
    let step = J2kStepSize::decode(raw).unwrap();
    println!(
        "  code {raw:#06x}: Δb = {:#} · 2^Rb",
        Exact::from_fixed(step).unwrap()
    );
    // every code is a different step, so a step comes back to its code; between two, a named rounding picks
    assert_eq!(J2kStepSize::encode(step, Rounding::NearestEven), Some(raw));

    println!("H.265's dequantization scale: levelScale[qP % 6] << (qP / 6) (8.6.3, eq. 8-309)");
    for qp in [20, 21, 26, 32] {
        println!("  qP {qp}: {}", HevcLevelScale::decode(qp).unwrap().get());
    }
    // H.264's NOTE 2 (8.5.8): "doubles for every increment of 6", "approximately 12 % for each increase of 1".
    // The doubling is exact; the steps between are the table's own ratios, not 2^(1/6) = 1.1225…
    let at = |qp| i128::from(HevcLevelScale::decode(qp).unwrap().get());
    for qp in 0..5 {
        println!(
            "  qP {} / qP {qp} = {}",
            qp + 1,
            Exact::<Ratio>::new(at(qp + 1), at(qp).unsigned_abs())
        );
    }
    println!(
        "  2^(1/6) is no fraction of integers: it appears in neither standard, only in that NOTE's words"
    );

    println!(
        "H.264's HRD: BitRate = (bit_rate_value_minus1 + 1) * 2^(6 + bit_rate_scale) (E.2.2, eq. E-81)"
    );
    // the two fields as one code, scale << 32 | value_minus1
    let rate = HrdBitRate::decode((4 << 32) | 0x3D08).unwrap(); // 15 624
    println!("  value_minus1 15624, scale 4: {} bit/s", rate.get());
    // the same rate at scale 0: encoding gives that code, the lowest one with the value
    println!(
        "  its lowest code: {:?}",
        HrdBitRate::encode(rate, Rounding::NearestEven)
    );
    // the largest: 2^32 - 1 times 2^21 bit/s, about 9.0e15: a u64 holds it, a u32 doesn't
    println!(
        "  the largest a stream can say: {} bit/s",
        HrdBitRate::decode((15 << 32) | 0xFFFF_FFFE).unwrap().get()
    );
}

fn irrational() {
    println!(
        "JPEG 2000's 9/7 taps (T.800 Table F.4) from their closed forms (Tables F.5 to F.7), in f64"
    );
    // the closed forms take √15 and two cube roots: algebraic, not a `Magnitude`, so a float edge by necessity
    let s15 = 15f64.sqrt();
    let a = ((63.0 - 14.0 * s15) / (1080.0 * s15)).cbrt();
    let b = -((63.0 + 14.0 * s15) / (1080.0 * s15)).cbrt();
    let x1 = a + b - 1.0 / 6.0;
    let re_x2 = -(a + b) / 2.0 - 1.0 / 6.0;
    let g3 = -5.0 * x1 * re_x2 / 8.0;
    let g4 = 5.0 * x1 / 64.0;
    let alpha = Fixed::<Ratio, One, f64>::bounded(-g4 / g3);
    println!("  alpha = {}", alpha.get());
    // eight digits, far from an f64's error; tests/props certifies all fifteen by exact intervals, and finds
    // two printed one off in the last place: r0 (Table F.6) and gamma (Table F.4)
    assert!(alpha.is_in(q!(-1.58613434...Ratio)));

    println!("AV1's Cos128_Lookup (7.13.2): 4096 cos(k pi / 128) 'rounded to the nearest integer'");
    let table = [4096, 4095, 4091, 4085, 4076, 4065, 4052, 4036, 4017];
    for (k, want) in (0..).zip(table) {
        let v = Fixed::<Ratio, One, f64>::bounded(4096.0 * (f64::from(k) * PI / 128.0).cos());
        let even = v
            .to_rounded::<One, i16>(Rounding::NearestEven)
            .unwrap()
            .get();
        let away = v
            .to_rounded::<One, i16>(Rounding::NearestAway)
            .unwrap()
            .get();
        assert_eq!((even, away), (want, want));
    }
    println!(
        "  the first nine entries by NearestEven and NearestAway alike: no entry is near a tie (all 65 certified)"
    );
}

/// H.264's sample aspect ratio indicator, Table E-1 (H.265 Table E.1 the same): 16 ratios, each a whole number of
/// 1/198. Code 0 is Unspecified, 17 to 254 Reserved, 255 Extended_SAR (the ratio follows in two 16-bit fields).
#[derive(Copy, Clone, Debug)]
struct Sar;

/// [Sar]'s table, in 1/198, by code; code 0's entry stands for a spare code.
const SAR: [i128; 17] = [
    0, 198, 216, 180, 288, 240, 432, 360, 576, 480, 324, 270, 384, 320, 264, 297, 396,
];

impl Codec for Sar {
    type Raw = u8;
    type Linear = Fixed<Ratio, Rational<1, 198>, u16>;

    const DEF: CodecDef<'static> = CodecDef {
        id: MarkerId {
            crate_name: "image_codecs",
            name: "Sar",
        },
        shape: CodecShape::Table(def_list(&SAR)),
        spare: def_list(&[
            CodeSpan::one(0),
            CodeSpan { low: 17, high: 254 },
            CodeSpan::one(255),
        ]),
        codes: CodeSpan::all::<u8>(),
        nominal: None,
        curve: None,
    };

    fn formula(raw: u8) -> Self::Linear {
        Fixed::new(
            SAR.get(usize::from(raw))
                .map_or(0, |&v| u16::try_from(v).unwrap()),
        )
    }

    fn encode(value: Self::Linear, rounding: Rounding) -> Option<u8> {
        u8::try_from(Self::DEF.encode(value.get().into(), rounding)?).ok()
    }
}

niches! {
    /// The codes of Table E-1 that are no ratio here.
    enum SarOther: u8 {
        /// Code 0, Unspecified.
        Unspecified = 0,
        /// Code 255, Extended_SAR: the ratio is `sar_width : sar_height`, fields of their own. A code that means
        /// "the value is elsewhere" has no name of its own in the crate yet.
        Extended = 255,
    }
}

/// What JFIF's density fields mean, by the `units` byte (T.871 10.1): one code picks among three kinds of value,
/// which a [Codec] (one `Linear`) can't say.
enum Density {
    /// 0: no unit, the densities give the pixel aspect ratio only, width : height = Vdensity : Hdensity.
    Aspect(Exact<Ratio>),
    /// 1 and 2: dots per inch or per cm, both as dots per metre here. There is no counted dot or pixel unit yet.
    PerMetre(Exact<Per<Ratio, Meter>>, Exact<Per<Ratio, Meter>>),
}

fn density(units: u8, h: u16, v: u16) -> Option<Density> {
    let per =
        |n: u16, to_metres: Exact<Per<Ratio, Meter>>| to_metres.scaled(Exact::new(n.into(), 1));
    match units {
        0 => Some(Density::Aspect(Exact::new(v.into(), h.into()))),
        1 => {
            // "dots per 2.54 cm": the inch of the `customary` group, 0.0254 m exactly
            let one = Exact::<Per<Ratio, Inch>>::new(1, 1).convert::<Per<Ratio, Meter>>()?;
            Some(Density::PerMetre(per(h, one)?, per(v, one)?))
        }
        2 => {
            let one = Exact::<Per<Ratio, Meter>>::new(100, 1);
            Some(Density::PerMetre(per(h, one)?, per(v, one)?))
        }
        _ => None,
    }
}

fn tables_and_matrices() {
    println!("H.264's sample aspect ratios (Table E-1) as a table codec over 1/198");
    type Aspect = Niched<Coded<Sar>, SarOther>;
    for raw in [1, 2, 5, 13, 0, 255] {
        match Aspect::try_from_raw(raw).map(Aspect::get) {
            Ok(Reading::Value(v)) => {
                let x = v.get();
                println!(
                    "  code {raw}: {}/198 = {}",
                    x.get(),
                    Exact::from_fixed(x).unwrap()
                );
            }
            Ok(Reading::Niche(n)) => println!("  code {raw}: {n:?}"),
            Err(_) => println!("  code {raw}: reserved"),
        }
    }
    let between = Fixed::<Ratio, Rational<1, 198>, u16>::new(250);
    println!(
        "  250/198, between 40:33 and 4:3, by NearestEven: code {:?}; by Ceil: code {:?}",
        Sar::encode(between, Rounding::NearestEven),
        Sar::encode(between, Rounding::Ceil)
    );

    println!("JFIF's and JPEG 2000's colour matrices, exactly, from K_R = 0.299 and K_B = 0.114");
    let (kr, kb) = (
        Exact::<Ratio>::new(299, 1000),
        Exact::<Ratio>::new(114, 1000),
    );
    let kg = Exact::<Ratio>::int(1)
        .checked_sub(kr)
        .unwrap()
        .checked_sub(kb)
        .unwrap();
    let two = Exact::<Ratio>::int(2);
    let blue_div = two.scaled(Exact::int(1).checked_sub(kb).unwrap()).unwrap(); // 1.772
    let red_div = two.scaled(Exact::int(1).checked_sub(kr).unwrap()).unwrap(); // 1.402
    let entries = [
        (
            "Cb from R, -K_R / 1.772",
            kr.scaled(blue_div.recip().unwrap()).unwrap(),
            "0.1687",
            "0,16875",
        ),
        (
            "Cb from G, -K_G / 1.772",
            kg.scaled(blue_div.recip().unwrap()).unwrap(),
            "0.3313",
            "0,331260",
        ),
        (
            "Cr from G, -K_G / 1.402",
            kg.scaled(red_div.recip().unwrap()).unwrap(),
            "0.4187",
            "0,41869",
        ),
        (
            "G from Cb, -K_B 1.772 / K_G",
            kb.scaled(blue_div)
                .unwrap()
                .scaled(kg.recip().unwrap())
                .unwrap(),
            "0.3441",
            "0,34413",
        ),
    ];
    for (name, x, jfif, t800) in entries {
        let places = u8::try_from(t800.len() - 2).unwrap();
        println!(
            "  {name} = -{x:#}: to 4 places {:#} (JFIF prints {jfif}), to {places} {:#} (T.800's 08/2002 edition prints {t800})",
            x.rounded(4, Rounding::NearestEven),
            x.rounded(places, Rounding::NearestEven),
        );
    }
    println!(
        "  T.800's 08/2002 edition prints -0,299 and -0,587 for G-9's Y0; its inverse G-12 to G-14 is the matrix with plus signs"
    );
    println!(
        "  Corrigendum 3 (10/2014) corrects G-9 to the plus signs; the 06/2019 edition prints it right"
    );

    println!("JFIF's density (T.871 10.1): the units byte picks the unit");
    for (units, h, v) in [(0, 2, 1), (1, 72, 72), (2, 120, 120)] {
        match density(units, h, v) {
            Some(Density::Aspect(a)) => {
                println!("  units {units}, {h} x {v}: pixel aspect ratio {a:#}");
            }
            Some(Density::PerMetre(x, y)) => println!("  units {units}, {h} x {v}: {x} by {y}"),
            None => println!("  units {units}: no such code"),
        }
    }
}

fn main() {
    jpeg_roundings();
    lossless();
    mantissa_exponent();
    irrational();
    tables_and_matrices();
}
