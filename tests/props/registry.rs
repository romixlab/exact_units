//! Property 13: units and quantities read from text through a `Registry`, any string at all. A unit that reads writes
//! itself (`{}`) as text that reads back to the same dimension, exact scale and kind, and reads again with its own
//! dimension expected; a quantity that reads writes text that reads back to the same exact value. Every unit of the
//! crate, with every prefix it takes, reads back from what each `Notation` writes for it, at its exact scale, and
//! from its English name. Strings, so no Kani proof; `fuzz/` runs the same functions (target `registry`).
#![expect(clippy::panic, reason = "a property fails by panicking")]

use exact_units::units::{self, Byte, Ohm, Second, Speed};
use exact_units::{Exact, Registry, Unit, UnitExpr};

/// Every unit of the crate.
#[cfg(feature = "customary")]
static TABLES: &[&[&UnitExpr]] = &[units::ALL, units::CUSTOMARY];
#[cfg(not(feature = "customary"))]
static TABLES: &[&[&UnitExpr]] = &[units::ALL];

const R: Registry<'static> = Registry::new(TABLES);

/// Any text as a unit: a refusal, or a unit whose text reads back the same.
pub(crate) fn any_unit_text(s: &str) {
    let Ok(u) = R.parse_unit(s, None) else {
        return;
    };
    let text = u.to_string();
    let back = R
        .parse_unit(&text, None)
        .unwrap_or_else(|e| panic!("{s:?} read as {text:?}, which doesn't read: {e}"));
    assert_eq!(back.dim(), u.dim(), "{s:?} as {text:?}");
    assert_eq!(back.scale(), u.scale(), "{s:?} as {text:?}");
    assert_eq!(back.quantity(), u.quantity(), "{s:?} as {text:?}");
    if let Some(d) = u.dim() {
        let again = R
            .parse_unit(s, Some(d))
            .unwrap_or_else(|e| panic!("{s:?} doesn't read with its own dimension: {e}"));
        assert_eq!(again.scale(), u.scale(), "{s:?}");
    }
}

/// Any text as a quantity of `U`: a refusal, or a value whose text reads back the same.
fn quantity<U: Unit>(s: &str) {
    let Ok(x) = R.parse::<U>(s) else {
        return;
    };
    for text in [format!("{x}"), format!("{x:#}")] {
        match R.parse::<U>(&text) {
            Ok(y) => assert_eq!(y, x, "{s:?} as {text:?}"),
            Err(e) => panic!("{s:?} read as {x:?}, written {text:?}, which doesn't read: {e}"),
        }
    }
}

/// Any text as a time, a resistance, a size in bytes and a speed.
pub(crate) fn any_quantity_text(s: &str) {
    quantity::<Second>(s);
    quantity::<Ohm>(s);
    quantity::<Byte>(s);
    quantity::<Speed>(s);
    // the value of a quantity read is the value of its unit, read alone
    if let Ok(x) = R.parse::<Second>(s)
        && let Some((num, unit)) = s.trim().split_once(' ')
        && let Ok(one) = R.parse::<Second>(&format!("1 {unit}"))
        && let Ok(n) = num.parse::<exact_units::Fraction>()
    {
        assert_eq!(
            one.get().checked_mul(n).map(Exact::<Second>::of),
            Some(x),
            "{s:?}"
        );
    }
}

/// Every named unit, with each prefix it takes and none, in each notation that writes it and by its English name:
/// the registry reads it back at its exact scale, its own dimension and kind expected.
#[cfg(not(fuzzing))]
#[test]
fn every_unit_reads_back_in_every_notation() {
    use exact_units::notation::unit_in;
    use exact_units::{Notation, Prefix};

    let mut checked = 0usize;
    for e in TABLES.iter().flat_map(|t| t.iter()) {
        let Some(def) = e.def() else { continue };
        if def.names.symbol.is_empty() {
            continue;
        }
        let prefixes = core::iter::once(Prefix::Unit).chain(
            (-30..=30i8)
                .filter_map(Prefix::si_from_pow10)
                .chain([Prefix::Kibi, Prefix::Mebi, Prefix::Gibi, Prefix::Tebi])
                .filter(|p| *p != Prefix::Unit && def.names.prefixes.allows(*p)),
        );
        for p in prefixes {
            let scale = def.scale.checked_mul(p.magnitude().unwrap()).unwrap();
            let mut texts = Vec::new();
            for n in [Notation::Unicode, Notation::Ascii, Notation::Ucum] {
                if let Some(t) = unit_in(n, p, e) {
                    texts.push(t.to_string());
                }
            }
            if !def.names.long.is_empty() {
                let name = if p == Prefix::Unit {
                    String::new()
                } else {
                    p.name().to_lowercase()
                };
                texts.push(format!("{name}{}", def.names.long));
                texts.push(format!("{name}{}", def.names.plural));
            }
            for t in texts {
                let u = R
                    .parse_unit_of(&t, def.dim, def.quantity_expr())
                    .unwrap_or_else(|err| panic!("{}: {t:?} doesn't read: {err}", def.id));
                assert_eq!(u.scale(), Some(scale), "{}: {t:?}", def.id);
                checked += 1;
            }
        }
    }
    assert!(checked > 1000, "{checked}");
}

#[cfg(not(any(kani, fuzzing)))]
mod tests {
    use proptest::prelude::*;

    /// Text near the grammar: symbols, names, prefixes, operators, exponents and numbers, now and then a stray char.
    fn unit_text() -> impl Strategy<Value = String> {
        let token = prop_oneof![
            4 => prop::sample::select(vec![
                "m", "s", "kg", "g", "A", "V", "Ω", "Ohm", "ohm", "ohms", "Pa", "Pa·s", "min", "h", "d", "B", "bit",
                "Hz", "N", "J", "W", "K", "mol", "cd", "lx", "rad", "°", "deg", "L", "l", "eV", "dB", "Np", "rpm",
                "metre", "metres", "second", "kilohm", "per", "in", "ft", "mi", "gal (US)", "nit", "m/s", "J/K",
            ]),
            2 => prop::sample::select(vec!["k", "M", "m", "µ", "u", "n", "Ki", "kilo", "milli", "c", "da"]),
            3 => prop::sample::select(vec![
                "/", "·", "*", ".", " ", "(", ")", "^", "^2", "^-1", "²", "³", "⁻¹", "2", "-1", " per ",
            ]),
            1 => any::<char>().prop_map(|c| c.to_string().leak() as &'static str),
        ];
        prop::collection::vec(token, 0..10).prop_map(|t| t.concat())
    }

    /// A number, then a space, then unit text.
    fn quantity_text() -> impl Strategy<Value = String> {
        let number = prop_oneof![
            any::<i32>().prop_map(|n| n.to_string()),
            (any::<i16>(), 0u32..6).prop_map(|(n, k)| format!("{n}.{}", "5".repeat(k as usize))),
            Just("1e3".to_owned()),
            Just("333.(3)".to_owned()),
        ];
        (number, unit_text()).prop_map(|(n, u)| format!("{n} {u}"))
    }

    proptest! {
        #[test]
        fn unit_texts(s in unit_text()) {
            super::any_unit_text(&s);
        }

        #[test]
        fn quantity_texts(s in quantity_text()) {
            super::any_quantity_text(&s);
        }

        #[test]
        fn any_strings(s in ".*") {
            super::any_unit_text(&s);
            super::any_quantity_text(&s);
        }
    }
}
