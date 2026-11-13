//! Photographic exposure: APEX values as exact levels in [Stop]s, and the nominal labels a camera marks against
//! the values they stand for.
//!
//! Exif (CIPA DC-008-2026, Exif 3.1) records exposure in APEX, the Additive System of Photographic Exposure: every
//! quantity a base-2 logarithm (Annex C), so a factor of 2 in light is one step.
//!
//! - ApertureValue Av = 2·log2 N of the f-number N (4.6.6.7.14), ShutterSpeedValue Tv = −log2 t of the exposure
//!   time in seconds (4.6.6.7.13), BrightnessValue Bv (4.6.6.7.15), ExposureBiasValue (4.6.6.7.16), and the film
//!   sensitivity Sv = log2(ASA/3.125) of Annex C. Ev = Av + Tv = Bv + Sv.
//! - Each is a level in [Stop]s here: an [Apex], in sixths of a stop, so whole, half and third stops are all exact.
//!   Exif writes them as a RATIONAL or SRATIONAL, which [apex_from_exif] reads exactly or refuses.
//!
//! The level is the exact quantity and the linear value the edge, the reverse of a level in dB. f/8 is Av 6 exactly,
//! and N = √2^Av is exact as a [Magnitude] half power for whole stops ([f_number]); a third stop's N, 2^(1/6), is
//! irrational and no `Magnitude` at all, so it is only an `f64` ([f_number_f64]). The same for times: 2^−7 s is
//! exact ([exposure_time]), 2^(−22/3) s is not.
//!
//! A lens or a dial is marked with labels, not values. Exif's Table 55 lists them for whole stops: f/11 is the label
//! of Av 7, N = 8√2 ≈ 11.31, and "1/125" that of Tv 7, t = 1/128 s. [ApertureLabel] and [ShutterLabel] are
//! [Codec]s from the label's position in that table to its exact APEX value: decoding is exact, encoding a value
//! between two labels takes a named [Rounding] (an Exif reader showing Tv 6.5 as a whole-stop label picks one).
//!
//! ```
//! use exact_units::codec::{Coded, Codec};
//! use exact_units::exposure::{Apex, ApertureLabel, ShutterLabel, apex_from_exif};
//! use exact_units::Rounding;
//!
//! // Exif's ApertureValue 4/1 and ShutterSpeedValue 6/1: f/4 at 1/64 s, which the dial marks "1/60"
//! let av = apex_from_exif(4, 1).unwrap();
//! let tv = apex_from_exif(6, 1).unwrap();
//! assert_eq!(av + tv, Apex::new(60)); // Ev 10, in sixths
//! let f = Coded::<ApertureLabel>::try_new(av, Rounding::NearestEven).unwrap();
//! assert_eq!(ApertureLabel::label(f.raw()), Some("4"));
//! let t = Coded::<ShutterLabel>::try_new(tv, Rounding::NearestEven).unwrap();
//! assert_eq!(ShutterLabel::label(t.raw()), Some("1/60"));
//! ```
//!
//! What this module doesn't pick, because people pick differently: the meter constants K and C, which ISO speed a
//! value is ([SensitivityType] says which, Exif's tag), and "EV" as a camera setting against a light value LV at
//! ISO 100. The book's [Photography](https://exact.vhrd.tech/recipes/photography/) recipe writes each as the
//! caller's named choice.

use crate::codec::{CodeSpan, Codec, CodecDef, CodecShape};
use crate::exact::{Exact, Fraction};
use crate::fixed::Fixed;
use crate::fixed::prefix::Rational;
#[cfg(any(feature = "std", feature = "libm"))]
use crate::num::math;
use crate::system::def_list;
use crate::units::Stop;
use crate::{Magnitude, MarkerId, Rounding};

/// An APEX value (Av, Tv, Bv, Sv, Ev) in sixths of a [Stop]: whole, half and third stops exact, ±5461 stops.
pub type Apex = Fixed<Stop, Rational<1, 6>, i16>;

/// Sixths of a stop in one stop.
const SIXTHS: i16 = 6;

/// An APEX value as Exif records it, a RATIONAL or SRATIONAL `num / den` (4.6.6.7.13 to 4.6.6.7.16), as an [Apex]:
/// exact, or `None` where it is no whole number of sixths of a stop, doesn't fit, or `den` is 0. BrightnessValue's
/// "unknown" (a numerator of `FFFFFFFF.H`, 4.6.6.7.15) is the caller's to check first: as an SRATIONAL it reads
/// −1/den, a value.
///
/// ```
/// use exact_units::exposure::{Apex, apex_from_exif};
/// assert_eq!(apex_from_exif(13, 2), Some(Apex::new(39))); // Av 6.5
/// assert_eq!(apex_from_exif(497, 100), None); // 4.97 is no sixth of a stop: refused, never the nearest
/// assert_eq!(apex_from_exif(1, 0), None);
/// ```
#[must_use]
pub fn apex_from_exif(num: i64, den: i64) -> Option<Apex> {
    let (num, den) = if den < 0 {
        (i128::from(num).checked_neg()?, den.unsigned_abs())
    } else {
        (i128::from(num), den.unsigned_abs())
    };
    Exact::<Stop>::checked_new(num, u128::from(den))?.to()
}

/// The f-number N = √2^Av of an aperture value of whole stops, exactly: a [Magnitude] half power of 2 (Av 7 is
/// 8√2, the lens's f/11). `None` for a value between whole stops, whose N is irrational and no `Magnitude`
/// ([f_number_f64] is its float edge), or one past `Magnitude`'s exponent range.
///
/// ```
/// use exact_units::exposure::{Apex, f_number};
/// use exact_units::Magnitude;
/// assert_eq!(f_number(Apex::new(42)), Some(Magnitude::pow2(3).mul(Magnitude::pow2(1).sqrt()))); // 8√2
/// assert_eq!(f_number(Apex::new(36)), Some(Magnitude::pow2(3))); // f/8
/// assert_eq!(f_number(Apex::new(44)), None); // a third stop past f/11
/// ```
#[must_use]
pub fn f_number(av: Apex) -> Option<Magnitude> {
    let stops = whole_stops(av)?;
    Magnitude::pow2(1).checked_pow_x2(stops).ok()
}

/// The exposure time t = 2^−Tv seconds of a shutter-speed value of whole stops, exactly: Tv 7 is 1/128 s, which a
/// dial marks "1/125". `None` for a value between whole stops (irrational, [exposure_time_f64] is its float edge)
/// or one past `Magnitude`'s exponent range.
///
/// ```
/// use exact_units::exposure::{Apex, exposure_time};
/// use exact_units::Magnitude;
/// assert_eq!(exposure_time(Apex::new(42)), Some(Magnitude::ratio(1, 128)));
/// assert_eq!(exposure_time(Apex::new(-30)), Some(Magnitude::ratio(32, 1))); // Tv −5, marked "30"
/// ```
#[must_use]
pub fn exposure_time(tv: Apex) -> Option<Magnitude> {
    let stops = whole_stops(tv)?.checked_neg()?;
    Magnitude::checked_pow2(stops).ok()
}

/// The f-number √2^Av of any aperture value, an `f64` by `exp2`: the float edge for a value between whole stops.
/// A whole stop of an even Av gives its power of two exactly. Feature `std` or `libm`.
#[cfg(any(feature = "std", feature = "libm"))]
#[must_use]
pub fn f_number_f64(av: Apex) -> f64 {
    math::exp2(f64::from(av.get()) / 12.0)
}

/// The exposure time 2^−Tv in seconds of any shutter-speed value, an `f64` by `exp2`: the float edge for a value
/// between whole stops; a whole stop gives its power of two exactly. Feature `std` or `libm`.
#[cfg(any(feature = "std", feature = "libm"))]
#[must_use]
pub fn exposure_time_f64(tv: Apex) -> f64 {
    math::exp2(-f64::from(tv.get()) / 6.0)
}

/// The film sensitivity Sv = log2(S/3.125) of an arithmetic speed S (Annex C), exactly: S = 100 is Sv 5. `None`
/// for a speed whose S/3.125 is no power of two, as every third-stop marking is (ISO 125 stands for 100·2^(1/3),
/// whose S/3.125 is irrational; 125 itself gives log2 40): its Sv is no exact value, and this refuses it rather
/// than give the nearest.
///
/// ```
/// use exact_units::exposure::{Apex, sensitivity_value};
/// assert_eq!(sensitivity_value(100), Some(Apex::new(30)));
/// assert_eq!(sensitivity_value(3200), Some(Apex::new(60)));
/// assert_eq!(sensitivity_value(125), None);
/// ```
#[must_use]
pub fn sensitivity_value(s: u32) -> Option<Apex> {
    // S / 3.125 = 8·S / 25, a power of two or no exact Sv
    let stops = crate::exact::whole_log_of(
        &Fraction::new(i128::from(s), 25).checked_mul(Fraction::int(8))?,
        true,
    )?;
    Some(Apex::new(i16::try_from(stops).ok()?.checked_mul(SIXTHS)?))
}

/// `v` in whole stops, `None` between them.
fn whole_stops(v: Apex) -> Option<i8> {
    let sixths = v.get();
    if sixths % SIXTHS != 0 {
        return None;
    }
    i8::try_from(sixths / SIXTHS).ok()
}

/// The aperture labels of Exif's Table 55 (Annex C), f/1 to f/32 by whole stops: code `i` is the label of Av `i`.
/// [Codec::decode] is the exact Av of a label, [Codec::encode] the label a named [Rounding] picks for an Av between
/// two (a tie to the even code: f/1, f/2, f/4, ...). Av below 0 or past 10 is no label here.
#[derive(Copy, Clone, Debug)]
pub struct ApertureLabel;

/// Av of each aperture label, in sixths of a stop.
const APERTURE_TABLE: [i128; 11] = [0, 6, 12, 18, 24, 30, 36, 42, 48, 54, 60];

impl ApertureLabel {
    /// The labels as Table 55 prints them, by code.
    pub const LABELS: [&'static str; 11] = [
        "1", "1.4", "2", "2.8", "4", "5.6", "8", "11", "16", "22", "32",
    ];

    /// The f-number each label writes, in tenths: 110 for "11", whose value is 8√2 ≈ 11.31.
    pub const MARKED_TENTHS: [u16; 11] = [10, 14, 20, 28, 40, 56, 80, 110, 160, 220, 320];

    /// The label of `code`, `None` past the table.
    #[must_use]
    pub fn label(code: u8) -> Option<&'static str> {
        Self::LABELS.get(usize::from(code)).copied()
    }
}

impl Codec for ApertureLabel {
    type Raw = u8;
    type Linear = Apex;

    const DEF: CodecDef<'static> = CodecDef {
        id: MarkerId {
            crate_name: "exact_units",
            name: "ApertureLabel",
        },
        shape: CodecShape::Table(def_list(&APERTURE_TABLE)),
        spare: def_list(&[CodeSpan { low: 11, high: 255 }]),
        codes: CodeSpan::all::<u8>(),
        nominal: None,
        curve: None,
    };

    /// Av `raw` whole stops: the table is Av 0 to 10 by code, so the code is the stop.
    #[expect(
        clippy::arithmetic_side_effects,
        reason = "a code is at most 255, and 255 · 6 fits i16"
    )]
    fn formula(raw: u8) -> Apex {
        Apex::new(i16::from(raw) * SIXTHS)
    }

    fn encode(value: Apex, rounding: Rounding) -> Option<u8> {
        let code = Self::DEF.encode(i128::from(value.get()), rounding)?;
        u8::try_from(code).ok()
    }
}

/// The shutter-speed labels of Exif's Table 55 (Annex C), "30" (Tv −5) to "1/2000" (Tv 11) by whole stops: code
/// `i` is the label of Tv `i − 5`. The labels are round numbers, not the values: "30" is 32 s, "15" 16 s, "1/15"
/// 1/16 s, "1/125" 1/128 s. [Codec::decode] is the exact Tv of a label, [Codec::encode] the label a named
/// [Rounding] picks for a Tv between two (a tie to the even code).
#[derive(Copy, Clone, Debug)]
pub struct ShutterLabel;

/// Tv of each shutter label, in sixths of a stop.
const SHUTTER_TABLE: [i128; 17] = [
    -30, -24, -18, -12, -6, 0, 6, 12, 18, 24, 30, 36, 42, 48, 54, 60, 66,
];

/// Tv of code 0, in whole stops.
const SHUTTER_FIRST: i16 = -5;

impl ShutterLabel {
    /// The labels as Table 55 prints them, by code.
    pub const LABELS: [&'static str; 17] = [
        "30", "15", "8", "4", "2", "1", "1/2", "1/4", "1/8", "1/15", "1/30", "1/60", "1/125",
        "1/250", "1/500", "1/1000", "1/2000",
    ];

    /// The time each label writes, in seconds, as `(numerator, denominator)`: `(1, 125)` for "1/125", whose value is
    /// 1/128 s.
    pub const MARKED: [(u16, u16); 17] = [
        (30, 1),
        (15, 1),
        (8, 1),
        (4, 1),
        (2, 1),
        (1, 1),
        (1, 2),
        (1, 4),
        (1, 8),
        (1, 15),
        (1, 30),
        (1, 60),
        (1, 125),
        (1, 250),
        (1, 500),
        (1, 1000),
        (1, 2000),
    ];

    /// The label of `code`, `None` past the table.
    #[must_use]
    pub fn label(code: u8) -> Option<&'static str> {
        Self::LABELS.get(usize::from(code)).copied()
    }
}

impl Codec for ShutterLabel {
    type Raw = u8;
    type Linear = Apex;

    const DEF: CodecDef<'static> = CodecDef {
        id: MarkerId {
            crate_name: "exact_units",
            name: "ShutterLabel",
        },
        shape: CodecShape::Table(def_list(&SHUTTER_TABLE)),
        spare: def_list(&[CodeSpan { low: 17, high: 255 }]),
        codes: CodeSpan::all::<u8>(),
        nominal: None,
        curve: None,
    };

    /// Tv `raw − 5` whole stops.
    #[expect(
        clippy::arithmetic_side_effects,
        reason = "a code is at most 255, and (255 − 5) · 6 fits i16"
    )]
    fn formula(raw: u8) -> Apex {
        Apex::new((i16::from(raw) + SHUTTER_FIRST) * SIXTHS)
    }

    fn encode(value: Apex, rounding: Rounding) -> Option<u8> {
        let code = Self::DEF.encode(i128::from(value.get()), rounding)?;
        u8::try_from(code).ok()
    }
}

/// Which parameter of ISO 12232 an Exif PhotographicSensitivity is: Exif's SensitivityType tag (34864,
/// 4.6.6.7.7). Standard output sensitivity, recommended exposure index and ISO speed are three different
/// measurements that a camera may mark with the same number; a value without its type is [Unknown](Self::Unknown),
/// never assumed to be ISO speed.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum SensitivityType {
    /// 0: not known (a file of Exif 2.21 or earlier, Annex G).
    Unknown,
    /// 1: standard output sensitivity (SOS).
    Sos,
    /// 2: recommended exposure index (REI).
    Rei,
    /// 3: ISO speed.
    IsoSpeed,
    /// 4: SOS and REI, the same number.
    SosRei,
    /// 5: SOS and ISO speed, the same number.
    SosIsoSpeed,
    /// 6: REI and ISO speed, the same number.
    ReiIsoSpeed,
    /// 7: SOS, REI and ISO speed, the same number.
    SosReiIsoSpeed,
}

impl SensitivityType {
    /// The type of a tag value, `None` for a reserved one (8 and above).
    ///
    /// ```
    /// use exact_units::exposure::SensitivityType;
    /// assert_eq!(SensitivityType::from_tag(2), Some(SensitivityType::Rei));
    /// assert_eq!(SensitivityType::from_tag(8), None);
    /// ```
    #[must_use]
    pub const fn from_tag(v: u16) -> Option<Self> {
        Some(match v {
            0 => Self::Unknown,
            1 => Self::Sos,
            2 => Self::Rei,
            3 => Self::IsoSpeed,
            4 => Self::SosRei,
            5 => Self::SosIsoSpeed,
            6 => Self::ReiIsoSpeed,
            7 => Self::SosReiIsoSpeed,
            _ => return None,
        })
    }

    /// The tag value.
    #[must_use]
    pub const fn tag(self) -> u16 {
        match self {
            Self::Unknown => 0,
            Self::Sos => 1,
            Self::Rei => 2,
            Self::IsoSpeed => 3,
            Self::SosRei => 4,
            Self::SosIsoSpeed => 5,
            Self::ReiIsoSpeed => 6,
            Self::SosReiIsoSpeed => 7,
        }
    }

    /// Whether the value is standard output sensitivity, alone or with others.
    #[must_use]
    pub const fn includes_sos(self) -> bool {
        matches!(
            self,
            Self::Sos | Self::SosRei | Self::SosIsoSpeed | Self::SosReiIsoSpeed
        )
    }

    /// Whether the value is the recommended exposure index, alone or with others.
    #[must_use]
    pub const fn includes_rei(self) -> bool {
        matches!(
            self,
            Self::Rei | Self::SosRei | Self::ReiIsoSpeed | Self::SosReiIsoSpeed
        )
    }

    /// Whether the value is ISO speed, alone or with others.
    #[must_use]
    pub const fn includes_iso_speed(self) -> bool {
        matches!(
            self,
            Self::IsoSpeed | Self::SosIsoSpeed | Self::ReiIsoSpeed | Self::SosReiIsoSpeed
        )
    }
}
