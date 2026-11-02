//! Property 12: a prefix on a unit with a power, as `{}` writes it. For every marker whose symbol has a power and every
//! SI prefix, the text `{}` writes reads back with `with_unit` to the same value, and means the scale of the type by
//! a reference that takes the text apart on its own: the prefix before a power is raised to it (SI Brochure, 9th ed.,
//! §3), a prefix before parentheses isn't. The old spelling on the whole unit still reads. Strings of a known set, so
//! a walk over all of it, no Kani proof.
#![expect(clippy::expect_used, reason = "a test fails by panicking")]
#![expect(
    clippy::arithmetic_side_effects,
    reason = "a test computes with small exponents it chose"
)]

use exact_units::prefix::{
    Atto, Centi, Deca, Deci, Exa, Femto, FixedScale, Giga, Hecto, Kilo, Mega, Micro, Milli, Nano,
    One, Peta, Pico, Pow10, Quecto, Quetta, Ronna, Ronto, Tera, Yocto, Yotta, Zepto, Zetta,
};
use exact_units::units::{self, Per, Second, Times};
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
    let UnitExpr::Named { name, symbol, .. } = e else {
        return None;
    };
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
