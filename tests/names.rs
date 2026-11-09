//! Unit names (IO-9): every unit's names are consistent, and a unit is written in each [Notation].
#![cfg(all(feature = "std", feature = "quantities"))]
#![expect(clippy::unwrap_used, reason = "a test fails by panicking")]

use exact_units::notation::{symbol_in, unit_in};
use exact_units::units::{self, Meter, Ohm, Per, Second, Sqrt, Times};
use exact_units::{Notation, Prefix, PrefixPolicy, Unit, UnitExpr};

const ALL: [Notation; 4] = [
    Notation::Unicode,
    Notation::Ascii,
    Notation::Ucum,
    Notation::Typst,
];

fn every(prefix: Prefix, e: &&UnitExpr) -> [Option<String>; 4] {
    ALL.map(|n| unit_in(n, prefix, e).map(|s| s.to_string()))
}

fn some(s: [&str; 4]) -> [Option<String>; 4] {
    s.map(|s| Some(s.to_owned()))
}

#[test]
fn notations() {
    assert_eq!(
        every(Prefix::Kilo, &Ohm::EXPR),
        some([" kΩ", " kOhm", " kOhm", " upright(\"kΩ\")"])
    );
    assert_eq!(
        every(Prefix::Micro, &units::Second::EXPR),
        some([" µs", " us", " us", " upright(\"µs\")"])
    );
    type Accel = Per<Meter, Times<Second, Second>>;
    assert_eq!(
        every(Prefix::Unit, &Accel::EXPR),
        some([
            " m/(s·s)",
            " m/(s*s)",
            " m/(s.s)",
            " upright(m)/(upright(s) dot.op upright(s))"
        ])
    );
    assert_eq!(
        every(Prefix::Unit, &units::Acceleration::EXPR),
        some([" m/s²", " m/s^2", " m/s2", " upright(m)/upright(s)^2"])
    );
    // a root has no form in UCUM
    type Noise = Per<units::Volt, Sqrt<units::Hertz>>;
    assert_eq!(
        every(Prefix::Nano, &Noise::EXPR),
        [
            Some(" nV/√Hz".into()),
            Some(" n(V/sqrt(Hz))".into()),
            None,
            Some(" upright(\"n\")(upright(V)/sqrt(upright(\"Hz\")))".into())
        ]
    );
    // a unit without a UCUM code isn't written in UCUM
    assert!(unit_in(Notation::Ucum, Prefix::Unit, &units::Turn::EXPR).is_none());
    assert_eq!(
        symbol_in(&units::Turn::EXPR, Notation::Ascii)
            .unwrap()
            .to_string(),
        "rev"
    );
    assert_eq!(
        symbol_in(&units::OpticalDepth::EXPR, Notation::Ascii)
            .unwrap()
            .to_string(),
        "tau"
    );
    assert_eq!(
        every(Prefix::Centi, &units::Ratio::EXPR),
        some([" %", " %", " %", " \"%\""])
    );
    assert_eq!(
        every(Prefix::Kibi, &units::Byte::EXPR),
        some([" KiB", " KiB", " KiBy", " upright(\"KiB\")"])
    );
}

/// Every named unit of the crate: a symbol in ASCII, a plural with its long name, a UCUM code with no space, and
/// prefixes only on a one-word coherent symbol or where the line says so.
#[test]
fn every_unit_has_consistent_names() {
    let all = units::ALL.iter();
    #[cfg(feature = "customary")]
    let all = all.chain(units::CUSTOMARY.iter());
    for e in all {
        let UnitExpr::Named(def) = e else {
            panic!("{e:?} in the catalogue")
        };
        let names = def.names;
        let mut ascii = String::new();
        names.write_ascii(&mut ascii).unwrap().unwrap();
        assert!(ascii.is_ascii(), "{}: {ascii}", def.id.name);
        assert_eq!(
            names.long.is_empty(),
            names.plural.is_empty(),
            "{}",
            def.id.name
        );
        if let Some(code) = names.ucum {
            assert!(
                code.is_ascii() && !code.contains(' '),
                "{}: {code}",
                def.id.name
            );
        }
        let one_word = !names.symbol.contains(['/', '·', ' ', '(', '²', '³', '⁻']);
        if !one_word {
            assert!(
                matches!(names.prefixes, PrefixPolicy::None) || def.id.name == "Nit",
                "{} takes prefixes on a compound symbol",
                def.id.name
            );
        }
    }
}
