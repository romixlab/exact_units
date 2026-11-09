//! The markers of [customary](super::super) are the catalogue: one per unit, with its symbol, dimension and exact
//! magnitude, and none named by an ambiguous word.

use super::{AMBIGUOUS, Dim as CatDim, Exact, Mag, ORIGINS, Src, UNITS, Unit as CatUnit};
use crate::origins::{Fahrenheit, Rankine};
use crate::prefix::Milli;
use crate::units::{self, CUSTOMARY, Degree, Kelvin, Radian, Turn};
use crate::{Dim, FixedScale, Magnitude, Offset, Origin, Provenance, Unit, UnitExpr};
use std::collections::BTreeSet;
use std::string::String;

/// Catalogue rows that are an exact_units marker of their own already, or a scale of one.
const ELSEWHERE: [&str; 5] = [
    "degree_angle",
    "turn",
    "milliradian",
    "degree_fahrenheit",
    "degree_rankine",
];

fn camel(id: &str) -> String {
    id.split('_')
        .map(|w| w[..1].to_uppercase() + &w[1..])
        .collect()
}

fn magnitude(m: Mag) -> Magnitude {
    Magnitude::new(
        u64::try_from(m.num).unwrap(),
        u64::try_from(m.den).unwrap(),
        i8::try_from(m.pow10).unwrap(),
        i8::try_from(m.pow2).unwrap(),
        m.pi,
    )
}

/// Whether `d` is the catalogue's dimension `c`: its exponents, the radian's, and no other base.
fn is_dim(d: Dim<'_>, c: CatDim) -> bool {
    let exps = c.map(|e| crate::__dim_exp(e, 1));
    let [s, m, kg, a, k, mol, cd, rad] = exps;
    let mut others = false;
    d.for_each_base(|b| others |= b.base != crate::MarkerId::RADIAN);
    d.exps() == [s, m, kg, a, k, mol, cd] && d.exp_of(crate::MarkerId::RADIAN) == rad && !others
}

fn row(id: &str) -> &'static CatUnit {
    UNITS.iter().find(|u| u.id == id).unwrap()
}

#[test]
fn one_marker_per_unit_with_its_definition() {
    let rows: Vec<_> = UNITS
        .iter()
        .filter(|u| !ELSEWHERE.contains(&u.id))
        .collect();
    assert_eq!(rows.len(), CUSTOMARY.len());
    for (u, expr) in rows.iter().zip(CUSTOMARY) {
        let UnitExpr::Named(def) = expr else {
            panic!("{} is not a named unit", u.id)
        };
        let (name, provenance) = (def.id.name, def.provenance);
        assert_eq!(def.id.crate_name, "exact_units", "{name}");
        assert_eq!(name, camel(u.id), "markers in catalogue order");
        assert_eq!(def.names.symbol, u.symbol, "{name}");
        assert!(is_dim(def.dim, u.dim), "{name}: {}", def.dim);
        assert_eq!(def.scale, magnitude(u.mag), "{name}");
        assert_eq!(
            matches!(provenance, Provenance::Conventional { .. }),
            u.exact == Exact::Measured,
            "{name}: conventional(..) iff measured"
        );
        assert_eq!(units::by_name(name), Some(*expr));
    }
}

#[test]
fn the_rest_is_other_markers_and_origins() {
    let scale = |e: &UnitExpr| match e {
        UnitExpr::Named(def) => def.scale,
        other => panic!("{other:?}"),
    };
    assert_eq!(scale(Degree::EXPR), magnitude(row("degree_angle").mag));
    assert_eq!(scale(Turn::EXPR), magnitude(row("turn").mag));
    assert_eq!(
        scale(Radian::EXPR).mul(Milli::SCALE),
        magnitude(row("milliradian").mag)
    );
    // °F and °R: steps of 5/9 K, 0 °F at 45967/180 K, 0 °R at 0 K
    for (o, step, zero) in [
        (
            row("degree_fahrenheit"),
            Fahrenheit::STEP,
            Fahrenheit::OFFSET,
        ),
        (row("degree_rankine"), Rankine::STEP, Rankine::OFFSET),
    ] {
        assert_eq!(step, magnitude(o.mag), "{}", o.id);
        let origin = ORIGINS.iter().find(|x| x.unit == o.id).unwrap();
        let want = origin
            .zero
            .map_or(Offset::ZERO, |z| Offset::new(false, magnitude(z)));
        assert_eq!(zero, want, "{}", o.id);
        assert!(is_dim(Kelvin::DIM, o.dim));
    }
}

#[test]
fn no_marker_is_named_by_an_ambiguous_word() {
    let names: BTreeSet<&str> = units::ALL
        .iter()
        .chain(CUSTOMARY)
        .filter_map(|e| match e {
            UnitExpr::Named(def) => Some(def.id.name),
            _ => None,
        })
        .collect();
    for a in AMBIGUOUS {
        let bare: String = a.bare.split([' ', '-']).map(camel).collect();
        // SI Brochure Table 6 names the angle unit "degree", and a temperature unit is always "degree Celsius" or
        // "degree Fahrenheit": the marker `Degree` is the angle. The group stays for text ("deg", IO-4).
        if bare != "Degree" {
            assert!(!names.contains(bare.as_str()), "a marker named {bare}");
        }
        for m in a.members {
            let marker = camel(m);
            assert!(
                ELSEWHERE.contains(m) || names.contains(marker.as_str()),
                "{} of {:?} has no marker",
                m,
                a.bare
            );
        }
    }
    for bare in [
        "Gallon",
        "Ounce",
        "Pound",
        "Ton",
        "Btu",
        "Calorie",
        "Horsepower",
        "Mil",
        "Foot",
        "Mile",
    ] {
        assert!(!names.contains(bare), "{bare}");
    }
}

/// The marker a catalogue row is, in the words of the docs page.
fn marker_of(id: &str) -> String {
    match id {
        "milliradian" => "Fixed<Radian, Milli, _>".into(),
        "degree_fahrenheit" => "origins::Fahrenheit".into(),
        "degree_rankine" => "origins::Rankine".into(),
        _ => camel(id),
    }
}

/// The tables of `docs/customary.md` made from the catalogue, by the name of their block.
fn docs_blocks() -> Vec<(&'static str, String)> {
    use core::fmt::Write as _;
    let mut ambiguous = String::from("| Name | Symbols | Markers | Why |\n|---|---|---|---|\n");
    for a in AMBIGUOUS {
        let code = |s: &[String]| {
            s.iter()
                .map(|s| format!("`{s}`"))
                .collect::<Vec<_>>()
                .join(", ")
        };
        let symbols: Vec<String> = a.symbols.iter().map(ToString::to_string).collect();
        let members: Vec<String> = a.members.iter().map(|m| marker_of(m)).collect();
        writeln!(
            ambiguous,
            "| {} | {} | {} | {} |",
            a.bare,
            code(&symbols),
            code(&members),
            a.note
        )
        .unwrap();
    }
    let mut conventional = String::from("| Marker | Symbol | Value in SI |\n|---|---|---|\n");
    for e in CUSTOMARY {
        if let UnitExpr::Named(def) = e
            && let Provenance::Conventional { source, year } = def.provenance
        {
            let (name, symbol) = (def.id.name, def.names.symbol);
            let u = UNITS.iter().find(|u| camel(u.id) == *name).unwrap();
            let p = u.sp811.unwrap();
            writeln!(
                conventional,
                "| `{name}` | {symbol} | {} E{:+03} ({source}, {year}) |",
                p.mantissa, p.exp10
            )
            .unwrap();
        }
    }
    let mut unsourced =
        String::from("| Marker | Definition | Why no primary source |\n|---|---|---|\n");
    for u in UNITS.iter().filter(|u| u.unsourced()) {
        let why = u.sources.iter().find(|c| c.0 == Src::Unsourced).unwrap().1;
        writeln!(unsourced, "| `{}` | {} | {why} |", marker_of(u.id), u.def).unwrap();
    }
    let mut sources = String::new();
    for s in Src::CITED {
        writeln!(sources, "- [{}]({})", s.title(), s.url()).unwrap();
    }
    vec![
        ("ambiguous", ambiguous),
        ("conventional", conventional),
        ("unsourced", unsourced),
        ("sources", sources),
    ]
}

/// The docs page's tables are the catalogue's: `CUSTOMARY_DOCS=update` (`just customary-docs`) writes them.
#[test]
fn docs_page_tables() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("docs/customary.md");
    let page = std::fs::read_to_string(&path).unwrap();
    let mut out = page.clone();
    for (name, table) in docs_blocks() {
        let start = format!("<!-- generated: {name} -->\n");
        let end = format!("<!-- end: {name} -->");
        let a = out
            .find(&start)
            .unwrap_or_else(|| panic!("no {start:?} in the page"))
            + start.len();
        let b = a + out[a..]
            .find(&end)
            .unwrap_or_else(|| panic!("no {end:?} in the page"));
        out.replace_range(a..b, &format!("\n{table}\n"));
    }
    if std::env::var("CUSTOMARY_DOCS").as_deref() == Ok("update") {
        std::fs::write(&path, out).unwrap();
    } else {
        assert!(
            out == page,
            "{} is stale: run `just customary-docs`",
            path.display()
        );
    }
}
