//! Property 12: a prefix on a unit with a power, as `{}` writes it. For every marker whose symbol has a power and every
//! SI prefix, the text `{}` writes reads back with `with_unit` to the same value, and means the scale of the type by
//! a reference that takes the text apart on its own: the prefix before a power is raised to it (SI Brochure, 9th ed.,
//! §3), a prefix before parentheses isn't. The old spelling on the whole unit still reads. Strings of a known set, so
//! a walk over all of it, no Kani proof.
//!
//! The same for a scale on a quotient: for a list of units and every power of ten from 10⁻¹⁵ to 10¹⁵ the text reads back
//! and means that power, by a reference that splits it at the `/`. A proptest draws the value, and draws spellings
//! people write that `{}` doesn't, in a field of any scale.
#![expect(clippy::expect_used, reason = "a test fails by panicking")]
#![expect(
    clippy::arithmetic_side_effects,
    reason = "a test computes with small exponents it chose"
)]

use exact_units::prefix::{
    Atto, Centi, Deca, Deci, Exa, Femto, FixedScale, Giga, Hecto, Kilo, Mega, Micro, Milli, Nano,
    One, Peta, Pico, Pow10, Quecto, Quetta, Ronna, Ronto, Tera, Yocto, Yotta, Zepto, Zetta,
};
use exact_units::units::{self, Per, Second, Sqrt, Times};
use exact_units::{Fixed, Unbounded, Unit, UnitExpr};
use serde::{Deserialize, Serialize};

/// A [Fixed] in the `with_unit` text form.
#[derive(Serialize, Deserialize, Debug, PartialEq)]
#[serde(transparent)]
struct Text<U: Unit, P: FixedScale>(
    #[serde(with = "exact_units::serde::with_unit")] Fixed<U, P, i64, Unbounded>,
);

/// `s` read with `with_unit`.
fn read<U: Unit, P: FixedScale>(s: &str) -> Option<i64> {
    serde_json::from_value::<Text<U, P>>(serde_json::Value::String(s.into()))
        .ok()
        .map(|t| t.0.get())
}

/// The power of ten of an SI prefix symbol, without the crate's table.
fn prefix_exp(s: &str) -> Option<i32> {
    Some(match s {
        "Q" => 30,
        "R" => 27,
        "Y" => 24,
        "Z" => 21,
        "E" => 18,
        "P" => 15,
        "T" => 12,
        "G" => 9,
        "M" => 6,
        "k" => 3,
        "h" => 2,
        "da" => 1,
        "d" => -1,
        "c" => -2,
        "m" => -3,
        "µ" => -6,
        "n" => -9,
        "p" => -12,
        "f" => -15,
        "a" => -18,
        "z" => -21,
        "y" => -24,
        "r" => -27,
        "q" => -30,
        _ => return None,
    })
}

/// The exponent of a superscript power.
fn power(s: &str) -> Option<i32> {
    Some(match s {
        "²" => 2,
        "³" => 3,
        _ => return None,
    })
}

/// The power of ten that `text`, the unit after the number, puts on the unit with the symbol `symbol`. The reference:
/// a litre is 10⁻³ m³, a prefix before parentheses is on the whole unit, one before m² and m³ is on the metre.
fn scale_of_text(text: &str, symbol: &str) -> Option<i32> {
    if text == symbol {
        return Some(0);
    }
    if let Some(inner) = text.strip_suffix(')') {
        let (p, inner) = inner.split_once('(')?;
        return (inner == symbol).then(|| prefix_exp(p))?;
    }
    let lead = symbol.split(['/', '·']).next()?;
    let tail = symbol.strip_prefix(lead)?;
    let head = text.strip_suffix(tail)?;
    if let Some(p) = head.strip_suffix('L') {
        return Some(if p.is_empty() { 0 } else { prefix_exp(p)? } - 3);
    }
    let n = power(lead.strip_prefix('m')?)?;
    Some(prefix_exp(head.strip_suffix(lead)?)? * n)
}

/// One value of `U` in `P`: the text `{}` writes reads back to it, and says the scale of `P`.
fn written_reads_back<U: Unit, P: FixedScale>() {
    let x = Fixed::<U, P, i64, Unbounded>::new(5);
    let written = x.to_string();
    assert_eq!(read::<U, P>(&written), Some(5), "{written:?}");
    let symbol = U::EXPR.to_string();
    let text = written.strip_prefix("5 ").expect("the number first");
    let exp = P::SCALE.as_pow10().expect("a power of ten");
    assert_eq!(
        scale_of_text(text, &symbol),
        Some(i32::from(exp)),
        "{written:?} on {symbol}"
    );
    // the spelling before, on the whole unit, where it had parentheses
    if P::SYMBOL.is_empty() {
        return;
    }
    let lead = symbol.split(['/', '·']).next().expect("a first factor");
    if lead.contains(['²', '³', '⁴', '⁻']) {
        let old = format!("5 {}({symbol})", P::SYMBOL);
        assert_eq!(read::<U, P>(&old), Some(5), "{old:?}");
    }
}

/// `$f` for the one unit, in every SI prefix.
macro_rules! every_prefix {
    ($u:ty) => {
        every_prefix!($u: Quetta Ronna Yotta Zetta Exa Peta Tera Giga Mega Kilo Hecto Deca One Deci Centi Milli Micro
            Nano Pico Femto Atto Zepto Yocto Ronto Quecto);
    };
    ($u:ty: $($p:ident)*) => {
        $(written_reads_back::<$u, $p>();)*
    };
}

/// [every_prefix] for the listed markers and a list of their names, to check that none is missing.
macro_rules! every_marker {
    ($($u:ident)*) => {{
        $(every_prefix!(units::$u);)*
        [$(stringify!($u)),*]
    }};
}

/// Whether the first factor of a symbol has a power.
fn has_power(e: &UnitExpr) -> Option<&'static str> {
    let UnitExpr::Named(def) = e else {
        return None;
    };
    let (name, symbol) = (def.id.name, def.names.symbol);
    let lead = symbol.split(['/', '·']).next()?;
    lead.contains(['²', '³', '⁴', '⁵', '⁶']).then_some(name)
}

#[test]
fn every_marker_with_a_power() {
    let tested = every_marker!(Area KinematicViscosity SpecificVolume Volume VolumetricFlow);
    for name in units::ALL.iter().filter_map(|e| has_power(e)) {
        assert!(tested.contains(&name), "{name} is not in the walk");
    }
}

#[cfg(feature = "customary")]
#[test]
fn every_customary_marker_with_a_power() {
    let tested = every_marker!(
        SquareInch SquareFoot SquareYard SquareMileInternational SquareMileUsSurvey CubicInch CubicFoot CubicYard
    );
    for name in units::CUSTOMARY.iter().filter_map(|e| has_power(e)) {
        assert!(tested.contains(&name), "{name} is not in the walk");
    }
}

#[test]
fn spelled_compounds() {
    every_prefix!(Per<units::Volume, Second>);
    every_prefix!(Per<units::Area, Second>);
    every_prefix!(Times<units::Area, Second>);
}

/// `U` in `P` is written `$text` and reads back.
macro_rules! writes {
    ($($u:ty, $p:ty, $text:literal;)*) => {$(
        assert_eq!(Fixed::<$u, $p, i64, Unbounded>::new(5).to_string(), $text);
        assert_eq!(read::<$u, $p>($text), Some(5), $text);
    )*};
}

/// Exactly the prefixes people use with the power move inside, and litres stand for the two markers.
#[test]
fn the_allow_list() {
    use units::{Area, Litre, SpecificVolume, Volume, VolumetricFlow};
    type Spelled = Per<Volume, Second>;

    writes! {
        Area, Deci, "5 d(m²)";
        Area, Pow10<-2>, "5 dm²";
        Area, Pow10<-4>, "5 cm²";
        Area, Micro, "5 mm²";
        Area, Pico, "5 µm²";
        Area, Atto, "5 nm²";
        Area, Mega, "5 km²";
        Area, Milli, "5 m(m²)";
        Area, Kilo, "5 k(m²)";
        Area, Hecto, "5 h(m²)";
        Area, Pow10<4>, "5·10⁴ m²";
        Area, Femto, "5 f(m²)";
        Volume, Milli, "5 L";
        Volume, Micro, "5 mL";
        Volume, Nano, "5 µL";
        Volume, Pico, "5 p(m³)";
        Volume, Atto, "5 µm³";
        Volume, Ronto, "5 nm³";
        Volume, Kilo, "5 k(m³)";
        Volume, Giga, "5 G(m³)";
        VolumetricFlow, Milli, "5 L/s";
        VolumetricFlow, Micro, "5 mL/s";
        VolumetricFlow, Nano, "5 µL/s";
        VolumetricFlow, Atto, "5 µm³/s";
        Spelled, Milli, "5 dm³/s";
        Spelled, Micro, "5 cm³/s";
        Spelled, Nano, "5 mm³/s";
        SpecificVolume, Milli, "5 dm³/kg";
        Litre, Milli, "5 mL";
        Litre, Micro, "5 µL";
    }
}

/// The parenthesised spelling names the scale of the type it is read into, whichever spelling that type writes.
#[test]
fn old_form_reads() {
    assert_eq!(read::<units::VolumetricFlow, Milli>("5 m(m³/s)"), Some(5));
    assert_eq!(read::<units::Area, Micro>("5 µ(m²)"), Some(5));
    assert_eq!(read::<units::Volume, Micro>("5 µ(m³)"), Some(5));
    // written in litres, read in metres too
    assert_eq!(read::<units::VolumetricFlow, Nano>("5 mm³/s"), Some(5));
    assert_eq!(read::<units::VolumetricFlow, Micro>("5 cm³/s"), Some(5));
    assert_eq!(read::<units::VolumetricFlow, Milli>("5 dm³/s"), Some(5));
    assert_eq!(read::<units::Volume, Nano>("5 mm³"), Some(5));
    assert_eq!(read::<units::Volume, Nano>("5 um³"), None);
}

#[test]
fn the_reference_itself() {
    assert_eq!(scale_of_text("cm³/s", "m³/s"), Some(-6));
    assert_eq!(scale_of_text("mL/s", "m³/s"), Some(-6));
    assert_eq!(scale_of_text("L", "m³"), Some(-3));
    assert_eq!(scale_of_text("k(m³)", "m³"), Some(3));
    assert_eq!(scale_of_text("km²", "m²"), Some(6));
    assert_eq!(scale_of_text("mm", "m²"), None);
}

/// The power of ten of a prefix on the left of `unit` in `text`, none as 0.
fn prefix_before(text: &str, unit: &str) -> Option<i32> {
    let head = text.strip_suffix(unit)?;
    if head.is_empty() {
        Some(0)
    } else {
        prefix_exp(head)
    }
}

/// The reference for a quotient: the power of ten that `text` puts on the unit `num/den`, taken apart at the `/`. A
/// prefix before a denominator's power is raised to it, a litre is 10⁻³ m³, a gram 10⁻³ kg.
fn quotient_scale(text: &str, num: &str, den: &str) -> Option<i32> {
    let symbol = format!("{num}/{den}");
    if let Some(rest) = text.strip_prefix('·') {
        let (power, unit) = rest.split_once(' ')?;
        return (unit == symbol).then(|| superscript(power.strip_prefix("10")?))?;
    }
    if let Some(inner) = text.strip_suffix(')') {
        let (p, inner) = inner.split_once('(')?;
        return (inner == symbol).then(|| prefix_exp(p))?;
    }
    let (top, bottom) = text.split_once('/')?;
    let top = match num {
        "kg" => prefix_before(top, "g")? - 3,
        num => prefix_before(top, num)?,
    };
    let bottom = if den == "m³" && bottom.ends_with('L') {
        prefix_before(bottom, "L")? - 3
    } else {
        let n = den.strip_prefix('m').map_or(Some(1), |p| match p {
            "" => Some(1),
            p => power(p),
        })?;
        prefix_before(bottom, den)? * n
    };
    Some(top - bottom)
}

/// The value of a superscript integer.
fn superscript(s: &str) -> Option<i32> {
    let mut sign = 1;
    let mut x = 0;
    for c in s.chars() {
        match "⁰¹²³⁴⁵⁶⁷⁸⁹".chars().position(|d| d == c) {
            Some(d) => x = x * 10 + i32::try_from(d).expect("a digit"),
            None if c == '⁻' => sign = -1,
            None => return None,
        }
    }
    Some(sign * x)
}

/// The text that `U` wrote before a scale on a quotient moved, for the old spelling: on the whole unit in parentheses
/// for a tree or a marker whose numerator takes no bare prefix, straight before a marker's symbol otherwise.
fn old_spelling<U: Unit, P: FixedScale>(v: i64) -> Option<String> {
    if P::SYMBOL.is_empty() {
        return None;
    }
    let symbol = U::EXPR.to_string();
    let bare = matches!(U::EXPR, UnitExpr::Named(_))
        && !symbol.starts_with("kg")
        && !symbol.split('/').next()?.contains(['²', '³']);
    Some(if bare {
        format!("{v} {}{symbol}", P::SYMBOL)
    } else {
        format!("{v} {}({symbol})", P::SYMBOL)
    })
}

/// `v` of `U` in `P`: `{}` and `{:#}` read back, the text means the scale, the old spelling still reads. `num` and `den`
/// are what `U` is written of.
fn quotient_reads_back<U: Unit, P: FixedScale>(v: i64, num: &str, den: &str) {
    let x = Fixed::<U, P, i64, Unbounded>::new(v);
    let written = x.to_string();
    assert_eq!(read::<U, P>(&written), Some(v), "{written:?}");
    let exact = format!("{x:#}");
    assert_eq!(read::<U, P>(&exact), Some(v), "{exact:?}");
    let exp = P::SCALE.as_pow10().expect("a power of ten");
    // a scale after the number is joined to it: 5·10⁴ V/m
    let at = written.find(['·', ' ']).expect("a unit after the number");
    let text = written[at..].trim_start();
    assert_eq!(
        quotient_scale(text, num, den),
        Some(i32::from(exp)),
        "{written:?}"
    );
    if let Some(old) = old_spelling::<U, P>(v) {
        assert_eq!(read::<U, P>(&old), Some(v), "{old:?}");
    }
}

/// The units a scale on a quotient is checked on, with what they are written of, and each power of ten as a function
/// of the value.
macro_rules! quotient_cases {
    ($($u:ty, $num:literal, $den:literal;)*) => {
        vec![$(quotient_cases!(@powers $u, $num, $den:
            -15 -14 -13 -12 -11 -10 -9 -8 -7 -6 -5 -4 -3 -2 -1 0 1 2 3 4 5 6 7 8 9 10 11 12 13 14 15)),*].concat()
    };
    (@powers $u:ty, $num:literal, $den:literal: $($e:literal)*) => {{
        let cases: Vec<fn(i64)> = vec![$(|v| quotient_reads_back::<$u, Pow10<$e>>(v, $num, $den)),*];
        cases
    }};
}

/// Every unit and power of [quotient_cases].
fn quotients() -> Vec<fn(i64)> {
    use units::{
        Conductivity, CurrentDensity, Decibel, DecibelLoss, Density, ElectricFieldStrength, Gram,
        HeatFluxDensity, Hertz, Litre, MassFlow, Meter, MolarConcentration, Neper, SurfaceDensity,
        Volt, Volume, Watt,
    };
    quotient_cases! {
        Per<Decibel, Meter>, "dB", "m";
        Per<DecibelLoss, Meter>, "dB", "m";
        Per<Neper, Meter>, "Np", "m";
        Per<Decibel, Hertz>, "dB", "Hz";
        Per<Decibel, Second>, "dB", "s";
        Per<Volt, Meter>, "V", "m";
        Per<Volt, Second>, "V", "s";
        Per<Watt, units::Area>, "W", "m²";
        Per<Gram, Volume>, "g", "m³";
        Per<Gram, Litre>, "g", "L";
        Density, "kg", "m³";
        MolarConcentration, "mol", "m³";
        SurfaceDensity, "kg", "m²";
        MassFlow, "kg", "s";
        Conductivity, "S", "m";
        ElectricFieldStrength, "V", "m";
        CurrentDensity, "A", "m²";
        HeatFluxDensity, "W", "m²";
        Per<units::Byte, Second>, "B", "s";
        Per<units::Bit, Second>, "bit", "s";
    }
}

/// Data rates: the byte takes a prefix on the numerator like any SI unit although it is 8 bit, and the binary
/// prefixes go there too, where the bit and the byte take them.
#[test]
fn data_rates() {
    use exact_units::prefix::{Gibi, Kibi, Mebi, Tebi};
    use units::{Bit, Byte};

    writes! {
        Per<Byte, Second>, One, "5 B/s";
        Per<Byte, Second>, Kilo, "5 kB/s";
        Per<Byte, Second>, Mega, "5 MB/s";
        Per<Byte, Second>, Kibi, "5 KiB/s";
        Per<Byte, Second>, Mebi, "5 MiB/s";
        Per<Byte, Second>, Gibi, "5 GiB/s";
        Per<Bit, Second>, Kilo, "5 kbit/s";
        Per<Bit, Second>, Giga, "5 Gbit/s";
        Per<Bit, Second>, Mebi, "5 Mibit/s";
        Per<Bit, Second>, Tebi, "5 Tibit/s";
        Per<Byte, units::Sample>, Kibi, "5 KiB/sample";
        // a binary prefix stays off a numerator that doesn't take it
        Per<units::Volt, Second>, Kibi, "5 Ki(V/s)";
    }
}

/// Every power of ten on every unit, for one value.
#[test]
fn quotients_walk() {
    for case in quotients() {
        case(5);
    }
}

/// Which side a power of ten goes to, a case of each rule.
#[test]
fn the_quotient_rules() {
    use units::{
        Conductivity, CurrentDensity, Decibel, Density, ElectricFieldStrength, HeatFluxDensity,
        Hertz, Kelvin, Kilometre, MassFlow, Meter, MolarConcentration, Momentum, Neper, Ratio,
        SurfaceDensity, Volt, Volume,
    };

    writes! {
        // a level: the denominator, where a prefix is used on it
        Per<Decibel, Meter>, Milli, "5 dB/km";
        Per<Decibel, Meter>, Hecto, "5 dB/cm";
        Per<Decibel, Meter>, Kilo, "5 dB/mm";
        Per<Neper, Meter>, Milli, "5 Np/km";
        Per<Decibel, Hertz>, Micro, "5 dB/MHz";
        Per<Decibel, Second>, Kilo, "5 dB/ms";
        Per<Decibel, Meter>, Centi, "5 c(dB/m)";
        Per<Decibel, Meter>, Deci, "5 d(dB/m)";
        Per<Decibel, Second>, Milli, "5 m(dB/s)";
        Per<Decibel, Kilometre>, One, "5 dB/km";
        Per<Decibel, Kilometre>, Milli, "5 m(dB/km)";
        // per litre, on the two markers
        Density, One, "5 kg/m³";
        Density, Milli, "5 mg/L";
        Density, Micro, "5 µg/L";
        Density, Nano, "5 ng/L";
        Density, Kilo, "5 kg/L";
        Density, Mega, "5 M(kg/m³)";
        MolarConcentration, Kilo, "5 mol/L";
        MolarConcentration, Milli, "5 µmol/L";
        // a unit spelled another way keeps its metres
        Per<units::Gram, Volume>, Micro, "5 µg/m³";
        Per<units::Gram, units::Litre>, Milli, "5 mg/L";
        // per centimetre, where the numerator then has an engineering prefix
        Conductivity, Pow10<-4>, "5 µS/cm";
        Conductivity, Deci, "5 mS/cm";
        ElectricFieldStrength, Pow10<5>, "5 kV/cm";
        ElectricFieldStrength, Hecto, "5 V/cm";
        CurrentDensity, Pow10<4>, "5 A/cm²";
        HeatFluxDensity, Deca, "5 mW/cm²";
        Per<Volt, Meter>, Hecto, "5 V/cm";
        // a wire's current density
        CurrentDensity, Mega, "5 A/mm²";
        // the numerator
        Conductivity, Milli, "5 mS/m";
        ElectricFieldStrength, Milli, "5 mV/m";
        Per<Volt, Meter>, Milli, "5 mV/m";
        Per<Volt, Sqrt<Hertz>>, Nano, "5 nV/√Hz";
        Per<Volt, Second>, Mega, "5 MV/s";
        SurfaceDensity, Milli, "5 g/m²";
        MassFlow, Milli, "5 g/s";
        MassFlow, Micro, "5 mg/s";
        // what stays
        MassFlow, Kilo, "5 k(kg/s)";
        Momentum, Milli, "5 m(kg·m/s)";
        Per<Ratio, Kelvin>, Micro, "5 µ(/K)";
    }
}

/// Every spelling people use reads in a field of the unit, whichever one the field writes.
#[test]
fn other_spellings_read() {
    use units::{Conductivity, Decibel, Density, Meter, MolarConcentration, Volume};

    assert_eq!(read::<Conductivity, Deci>("5 dS/m"), Some(5));
    assert_eq!(read::<Conductivity, Deci>("0.5 S/m"), Some(5));
    assert_eq!(read::<Conductivity, Deci>("5 S/m"), Some(50));
    assert_eq!(read::<Conductivity, Pow10<-4>>("5 uS/cm"), Some(5));
    assert_eq!(read::<Conductivity, Pow10<-4>>("5 \u{3bc}S/cm"), Some(5));
    assert_eq!(read::<Conductivity, Pow10<-4>>("5·10⁻⁴ S/m"), Some(5));
    assert_eq!(read::<Density, One>("5 g/L"), Some(5));
    assert_eq!(read::<Density, Milli>("5 g/m³"), Some(5));
    assert_eq!(read::<Density, Milli>("5 mg/L"), Some(5));
    assert_eq!(read::<Density, Micro>("5 mg/m³"), Some(5));
    assert_eq!(read::<Density, Milli>("5 m(kg/m³)"), Some(5));
    assert_eq!(read::<Density, One>("5 mg/mL"), Some(5));
    assert_eq!(read::<Density, Kilo>("5 g/cm³"), Some(5));
    assert_eq!(read::<MolarConcentration, One>("5 mmol/L"), Some(5));
    assert_eq!(read::<Per<Decibel, Meter>, Milli>("5 m(dB/m)"), Some(5));
    assert_eq!(read::<Per<Decibel, Meter>, One>("5 dB/m"), Some(5));
    assert_eq!(read::<Per<Decibel, Meter>, Milli>("5 mdB/m"), None);
    assert_eq!(read::<Per<units::Gram, Volume>, Milli>("5 mg/m³"), Some(5));
    assert_eq!(read::<Per<Decibel, Meter>, One>("5 dB/hm"), None);
    assert_eq!(read::<Per<Decibel, Meter>, One>("5 dB/s"), None);
    assert_eq!(read::<Per<units::Volt, Meter>, One>("5 V/s"), None);
}

/// A number and a spelling drawn apart: the value `read` gets is the reference's, in a field of 10⁻¹⁸.
fn drawn_spelling<U: Unit>(v: i16, top: (&str, i32), bottom: (&str, i32), num: &str, den: &str) {
    type Fine = Pow10<-18>;
    let text = format!("{v} {}{num}/{}{den}", top.0, bottom.0);
    let steps =
        i128::from(v) * 10i128.pow(u32::try_from(18 + top.1 + bottom.1).expect("in the field"));
    assert_eq!(
        read::<U, Fine>(&text),
        i64::try_from(steps).ok(),
        "{text:?}"
    );
}

#[cfg(not(any(kani, fuzzing)))]
mod tests {
    use super::{drawn_spelling, quotients, units};
    use proptest::prelude::*;

    /// An SI prefix and its power of ten, for a numerator.
    fn top() -> impl Strategy<Value = (&'static str, i32)> {
        prop::sample::select(vec![
            ("", 0),
            ("k", 3),
            ("m", -3),
            ("µ", -6),
            ("u", -6),
            ("n", -9),
            ("d", -1),
        ])
    }

    /// A prefix used on a metre and what it does to a quotient.
    fn per_metre() -> impl Strategy<Value = (&'static str, i32)> {
        prop::sample::select(vec![
            ("", 0),
            ("k", -3),
            ("c", 2),
            ("m", 3),
            ("µ", 6),
            ("n", 9),
        ])
    }

    proptest! {
        #[test]
        fn quotient_text_round_trips(i in any::<prop::sample::Index>(), v in -(1i64 << 40)..(1i64 << 40)) {
            let cases = quotients();
            cases[i.index(cases.len())](v);
        }

        #[test]
        fn spellings_read_exactly(v in any::<i16>(), a in top(), q in per_metre()) {
            drawn_spelling::<units::Conductivity>(v, a, q, "S", "m");
            drawn_spelling::<units::ElectricFieldStrength>(v, a, q, "V", "m");
            drawn_spelling::<units::Per<units::Volt, units::Meter>>(v, a, q, "V", "m");
        }

        #[test]
        fn a_level_takes_only_the_denominator(v in any::<i16>(), q in per_metre()) {
            drawn_spelling::<units::Per<units::Decibel, units::Meter>>(v, ("", 0), q, "dB", "m");
            drawn_spelling::<units::Per<units::Neper, units::Meter>>(v, ("", 0), q, "Np", "m");
        }
    }
}
