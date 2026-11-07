//! The symbol table of `q!` (`macros/src/pure/table.rs`) and the book's page of its symbols (`docs/start/symbols.md`),
//! generated from the `unit!` and `origin!` lines and the catalogue they declare. `Q_SYMBOLS=update` (`just
//! q-symbols`) writes both; without it the test fails when either is stale, so the table and the markers agree.
//!
//! The rules, so that a symbol never reads two ways (AGENTS.md, rule 5):
//!
//! - A symbol is a table symbol when it is one word: no `/`, `·`, space, parenthesis, digit or exponent. A
//!   qualified one (`gal (US)`) gives its stem (`gal`) a reading, so the stem is refused next to the other gallon.
//! - Prefixes go only on the units the SI lets take one: a coherent SI unit, and g, L, eV, t; bit and B take the
//!   binary prefixes too. Never h, min, d, °, a customary unit or a counted one.
//! - A table symbol's readings are its own lines, the stems, and every split into a prefix and a table symbol,
//!   whether that unit takes the prefix or not (`cd` is the candela, the cord and a centi-day, `min` the minute and a
//!   milli-inch). A prefixed form's are its splits into a prefix and a unit that takes it (`mcd` is a millicandela,
//!   never a millicord).
//!   Readings of the same dimension, kind and exact scale fold (`km` is `Kilometre` and kilo `Meter`); a symbol left
//!   with two is refused, with its readings as what to write instead.
//! - Every unit group is on, so which symbols are refused never depends on the features a build turns on.
#![cfg(all(feature = "std", feature = "quantities", feature = "customary"))]
#![expect(
    clippy::arithmetic_side_effects,
    reason = "a test adds string offsets and small exponents, which fit"
)]
#![expect(
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "the generator's helpers fail the test by panicking, on source that isn't in the shape they read"
)]

#[path = "../macros/src/pure/entry.rs"]
mod entry;
#[path = "../macros/src/pure/prefix.rs"]
mod prefix;
#[path = "../macros/src/pure/resolve.rs"]
mod resolve;
#[path = "../macros/src/pure/table.rs"]
mod table;

use core::fmt::Write as _;
use entry::Prefixes;
use exact_units::{Dim, Magnitude, Scale, UnitExpr, units};
use prefix::{PREFIXES, Prefix};
use std::collections::BTreeMap;
use std::path::Path;

/// Spellings `q!` reads next to the `unit!` symbols: ASCII for what Rust's lexer can't take, and the ratios with a
/// power of ten of their own. `(symbol, marker, pow10)`.
const ALIASES: &[(&str, &str, i8)] = &[
    ("%", "Ratio", -2),
    ("percent", "Ratio", -2),
    ("‰", "Ratio", -3),
    ("ppm", "Ratio", -6),
    ("ppb", "Ratio", -9),
    ("ohm", "Ohm", 0),
    ("deg", "Degree", 0),
];

/// Symbols `q!` reads as a point at an origin, never as a unit (the `unit!` line `DegreeCelsius = "°C"` is a
/// difference in °C, written as a type). `(symbol, origin)`.
const ORIGIN_SYMBOLS: &[(&str, &str)] = &[
    ("°C", "Celsius"),
    ("degC", "Celsius"),
    ("℃", "Celsius"),
    ("°F", "Fahrenheit"),
    ("degF", "Fahrenheit"),
    ("℉", "Fahrenheit"),
    ("°R", "Rankine"),
    ("degR", "Rankine"),
];

/// Scaled units the SI lets take a prefix (SI Brochure, 9th ed., tables 8 and 4.1): gram, litre, electronvolt, tonne.
const SCALED_WITH_PREFIX: &[&str] = &["Gram", "Litre", "ElectronVolt", "Tonne"];

/// The units that take the binary prefixes of IEC 80000-13 as well as the SI ones.
const BINARY: &[&str] = &["Bit", "Byte"];

/// A `unit!` line as the source writes it.
#[derive(Clone, Debug)]
struct Line {
    name: String,
    symbol: String,
    group: String,
    kind: Option<String>,
}

/// The `unit!` lines of `file`, each with the feature group of the `#[cfg(feature = ..)]` before it (or `group` for
/// a module that is one group).
fn unit_lines(file: &str, group: Option<&str>) -> Vec<Line> {
    let src = std::fs::read_to_string(root().join(file)).unwrap();
    let mut feature: Option<String> = None;
    let mut out = Vec::new();
    for l in src.lines().map(str::trim) {
        if let Some(rest) = l.strip_prefix("#[cfg(") {
            feature = rest
                .strip_prefix("feature = \"")
                .and_then(|r| r.strip_suffix("\")]"))
                .map(str::to_owned);
            continue;
        }
        let Some(rest) = l.strip_prefix("pub struct ") else {
            continue;
        };
        let Some((name, rest)) = rest.split_once(" = \"") else {
            continue;
        };
        let (symbol, rest) = rest.split_once('"').unwrap();
        let kind = rest
            .split_once(" kind \"")
            .map(|(_, k)| k.split_once('"').unwrap().0.to_owned());
        out.push(Line {
            name: name.to_owned(),
            symbol: symbol.to_owned(),
            group: group
                .map(str::to_owned)
                .or_else(|| feature.clone())
                .unwrap_or_default(),
            kind,
        });
        feature = None;
    }
    out
}

/// The feature group of each origin of `src/point/origins.rs`: the `#[cfg(feature = ..)]` of the module it is in.
fn origin_groups() -> BTreeMap<String, String> {
    let src = std::fs::read_to_string(root().join("src/point/origins.rs")).unwrap();
    let mut feature = String::new();
    let mut out = BTreeMap::new();
    for l in src.lines() {
        if let Some(f) = l
            .strip_prefix("#[cfg(feature = \"")
            .and_then(|r| r.strip_suffix("\")]"))
        {
            f.clone_into(&mut feature);
        } else if let Some(rest) = l.trim().strip_prefix("pub struct ") {
            let name: String = rest
                .chars()
                .take_while(char::is_ascii_alphanumeric)
                .collect();
            out.insert(name, feature.clone());
        }
    }
    out
}

fn root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}

/// Whether `s` is one word `q!` can look up: no operator, space, parenthesis, digit or exponent.
fn atomic(s: &str) -> bool {
    !s.is_empty()
        && !s
            .chars()
            .any(|c| c.is_ascii_digit() || "/·() ⁻⁰¹²³⁴⁵⁶⁷⁸⁹√".contains(c) || c.is_whitespace())
}

/// What a reading is, for folding: the dimension, the kind, the exact scale in coherent SI (`None` for an affine
/// one, which folds with nothing).
#[derive(Clone, Debug, PartialEq)]
struct Value {
    dim: Dim<'static>,
    kind: Option<String>,
    scale: Option<Magnitude>,
    /// The reading itself, where its scale is past what a [Magnitude] holds.
    far: Option<String>,
}

/// A symbol of the table: a `unit!` line's or an alias.
#[derive(Clone, Debug)]
struct Sym {
    symbol: String,
    marker: String,
    group: String,
    prefixes: Prefixes,
    pow10: i8,
    value: Value,
}

/// One way to read a symbol, as the refusal names it.
#[derive(Clone, Debug)]
struct Reading {
    text: String,
    value: Value,
}

/// A prefix's name, as a refusal writes it: `milli`, `kibi`.
fn prefix_name(p: &Prefix) -> &'static str {
    const SI: [&str; 24] = [
        "quecto", "ronto", "yocto", "zepto", "atto", "femto", "pico", "nano", "micro", "milli",
        "centi", "deci", "deca", "hecto", "kilo", "mega", "giga", "tera", "peta", "exa", "zetta",
        "yotta", "ronna", "quetta",
    ];
    const SI_EXP: [i32; 24] = [
        -30, -27, -24, -21, -18, -15, -12, -9, -6, -3, -2, -1, 1, 2, 3, 6, 9, 12, 15, 18, 21, 24,
        27, 30,
    ];
    const BIN: [&str; 8] = [
        "kibi", "mebi", "gibi", "tebi", "pebi", "exbi", "zebi", "yobi",
    ];
    match p.scale {
        prefix::Scale::Pow10(e) => SI[SI_EXP.iter().position(|x| *x == e).unwrap()],
        prefix::Scale::Pow1024(e) => BIN[usize::try_from(e).unwrap() - 1],
    }
}

/// A prefix's scale, `None` past a magnitude's exponents (±63: 1024⁷ and up).
fn prefix_magnitude(p: &Prefix) -> Option<Magnitude> {
    match p.scale {
        prefix::Scale::Pow10(e) => Some(Magnitude::pow10(i8::try_from(e).unwrap())),
        prefix::Scale::Pow1024(e) => {
            (e * 10 <= 63).then(|| Magnitude::pow2(i8::try_from(e * 10).unwrap()))
        }
    }
}

/// `scale` times the prefix `p`, `None` past a magnitude's exponents.
fn scaled(scale: Option<Magnitude>, p: &Prefix) -> Option<Magnitude> {
    scale?.checked_mul(prefix_magnitude(p)?).ok()
}

/// Whether a symbol with `prefixes` takes `p`.
fn takes(prefixes: Prefixes, p: &Prefix) -> bool {
    matches!(
        (prefixes, p.scale),
        (Prefixes::Si, prefix::Scale::Pow10(_)) | (Prefixes::SiBinary, _)
    )
}

/// The generated table, its refusals and the facts the book page lists.
struct Generated {
    units: Vec<Sym>,
    refused: Vec<(String, Vec<String>)>,
    origins: Vec<(String, String, String)>,
    /// Qualified `unit!` symbols and those with an operator: written as a type in `q!`.
    not_symbols: Vec<(String, String)>,
    /// `unit!` symbols read as a prefix and another unit, the same value: `(symbol, marker)`.
    folded: Vec<(String, String)>,
}

#[expect(
    clippy::too_many_lines,
    reason = "the rules of the module's docs, in their order"
)]
fn generate() -> Generated {
    let mut lines = unit_lines("src/units.rs", None);
    lines.extend(unit_lines("src/units/customary.rs", Some("customary")));

    // the source and the catalogue name the same markers
    let catalogue: Vec<&UnitExpr> = units::ALL.iter().chain(units::CUSTOMARY).copied().collect();
    let names: Vec<&str> = lines.iter().map(|l| l.name.as_str()).collect();
    let in_catalogue: Vec<&str> = catalogue
        .iter()
        .map(|e| match e {
            UnitExpr::Named { name, .. } => *name,
            e => panic!("{e:?} in the catalogue"),
        })
        .collect();
    assert_eq!(names, in_catalogue, "the unit! lines read from source");

    let value = |line: &Line, pow10: i8| {
        let Some(UnitExpr::Named {
            symbol, dim, scale, ..
        }) = units::by_name(&line.name)
        else {
            panic!("{} is no named unit", line.name);
        };
        assert_eq!(*symbol, line.symbol, "the symbol of {}", line.name);
        Value {
            dim: *dim,
            kind: line.kind.clone(),
            scale: match scale {
                Scale::Affine { .. } => None,
                s => s.magnitude().map(|m| m.mul(Magnitude::pow10(pow10))),
            },
            far: None,
        }
    };
    let prefixes = |line: &Line| {
        let Some(UnitExpr::Named { dim, scale, .. }) = units::by_name(&line.name) else {
            panic!("{} is no named unit", line.name);
        };
        if BINARY.contains(&line.name.as_str()) {
            Prefixes::SiBinary
        } else if SCALED_WITH_PREFIX.contains(&line.name.as_str())
            || (matches!(scale, Scale::Coherent)
                && dim.counted_bases().next().is_none()
                && !matches!(
                    line.group.as_str(),
                    "customary" | "levels" | "dimensionless"
                ))
        {
            Prefixes::Si
        } else {
            Prefixes::None
        }
    };

    let origin_symbols: Vec<&str> = ORIGIN_SYMBOLS.iter().map(|o| o.0).collect();
    let mut syms: Vec<Sym> = Vec::new();
    let mut stems: BTreeMap<String, Vec<Reading>> = BTreeMap::new();
    let mut not_symbols = Vec::new();
    for line in &lines {
        if line.symbol.is_empty() {
            continue;
        }
        if atomic(&line.symbol) && !origin_symbols.contains(&line.symbol.as_str()) {
            syms.push(Sym {
                symbol: line.symbol.clone(),
                marker: line.name.clone(),
                group: line.group.clone(),
                prefixes: prefixes(line),
                pow10: 0,
                value: value(line, 0),
            });
            continue;
        }
        not_symbols.push((line.symbol.clone(), line.name.clone()));
        if let Some((stem, _)) = line
            .symbol
            .split_once(" (")
            .filter(|(stem, q)| atomic(stem) && q.ends_with(')'))
        {
            stems.entry(stem.to_owned()).or_default().push(Reading {
                text: line.name.clone(),
                value: value(line, 0),
            });
        }
    }
    let line_of = |name: &str| lines.iter().find(|l| l.name == name).unwrap();
    let mut aliases: Vec<(&str, &str, i8)> = ALIASES.to_vec();
    for line in &lines {
        aliases.extend(
            units::spellings(&line.name)
                .iter()
                .map(|s| (*s, line.name.as_str(), 0)),
        );
    }
    for (symbol, marker, pow10) in aliases {
        let line = line_of(marker);
        syms.push(Sym {
            symbol: symbol.to_owned(),
            marker: marker.to_owned(),
            group: line.group.clone(),
            prefixes: if pow10 == 0 {
                prefixes(line)
            } else {
                Prefixes::None
            },
            pow10,
            value: value(line, pow10),
        });
    }

    // every symbol that may read two ways: the table's own, the stems, and every prefix on a symbol that takes it
    let mut candidates: Vec<String> = syms.iter().map(|s| s.symbol.clone()).collect();
    candidates.extend(stems.keys().cloned());
    for s in &syms {
        for p in PREFIXES.iter().filter(|p| takes(s.prefixes, p)) {
            candidates.push(format!("{}{}", p.text, s.symbol));
        }
    }
    candidates.sort();
    candidates.dedup();

    let mut refused: Vec<(String, Vec<String>)> = Vec::new();
    let mut folded = Vec::new();
    for c in &candidates {
        let mut readings: Vec<Reading> = syms
            .iter()
            .filter(|s| s.symbol == *c)
            .map(|s| Reading {
                text: s.marker.clone(),
                value: s.value.clone(),
            })
            .collect();
        readings.extend(stems.get(c).into_iter().flatten().cloned());
        // a table symbol splits against every unit, a prefixed form only against those that take its prefix
        let own = !readings.is_empty();
        for p in PREFIXES {
            let Some(rest) = c.strip_prefix(p.text) else {
                continue;
            };
            for s in syms
                .iter()
                .filter(|s| s.symbol == rest && (own || takes(s.prefixes, p)))
            {
                readings.push(Reading {
                    text: format!("{} {}", prefix_name(p), s.marker),
                    value: Value {
                        // past a magnitude's exponents (1024⁸ on a bit, quecto on eV) the reading folds only with
                        // itself
                        scale: scaled(s.value.scale, p),
                        far: (s.value.scale.is_some() && scaled(s.value.scale, p).is_none())
                            .then(|| format!("{} {}", prefix_name(p), s.marker)),
                        ..s.value.clone()
                    },
                });
            }
        }
        // readings of the same value fold; an affine one folds with nothing
        let mut distinct: Vec<Reading> = Vec::new();
        for r in readings {
            let same = |d: &Reading| {
                d.value == r.value && (r.value.scale.is_some() || r.value.far.is_some())
            };
            if !distinct.iter().any(same) && !distinct.iter().any(|d| d.text == r.text) {
                distinct.push(r);
            }
        }
        if distinct.len() > 1 {
            refused.push((c.clone(), distinct.into_iter().map(|r| r.text).collect()));
        }
    }

    // The table's lines: a refused symbol keeps its line only where it takes prefixes (and is the only one that
    // does), so `mcd` is a millicandela although `cd` is refused; a line a prefix on another reads the same way
    // (`km`, `Kilometre`, is kilo `Meter`) gives way to the split, which keeps `km/h` in metres.
    let mut units: Vec<Sym> = Vec::new();
    for s in &syms {
        let is_refused = refused.iter().any(|(r, _)| *r == s.symbol);
        let mut shared = syms.iter().filter(|o| o.symbol == s.symbol);
        let only = shared.all(|o| o.prefixes == Prefixes::None || core::ptr::eq(o, s));
        let split = PREFIXES.iter().any(|p| {
            s.symbol.strip_prefix(p.text).is_some_and(|rest| {
                syms.iter().any(|o| {
                    o.symbol == rest
                        && takes(o.prefixes, p)
                        && o.value.scale.is_some()
                        && Value {
                            scale: scaled(o.value.scale, p),
                            ..o.value.clone()
                        } == s.value
                })
            })
        });
        let keep = if is_refused {
            s.prefixes != Prefixes::None && only
        } else {
            !split
        };
        if keep {
            units.push(s.clone());
        } else if split {
            folded.push((s.symbol.clone(), s.marker.clone()));
        }
    }
    units.sort_by(|a, b| a.symbol.cmp(&b.symbol));

    let groups = origin_groups();
    let mut origins: Vec<(String, String, String)> = ORIGIN_SYMBOLS
        .iter()
        .map(|(s, o)| ((*s).to_owned(), format!("origins::{o}"), groups[*o].clone()))
        .collect();
    origins.sort();
    Generated {
        units,
        refused,
        origins,
        not_symbols,
        folded,
    }
}

fn prefixes_text(p: Prefixes) -> &'static str {
    match p {
        Prefixes::None => "None",
        Prefixes::Si => "Si",
        Prefixes::SiBinary => "SiBinary",
    }
}

/// `macros/src/pure/table.rs`, one line per entry, `#[rustfmt::skip]` so rustfmt leaves it as written.
fn table_rs(g: &Generated) -> String {
    let mut out = String::from(
        "//! The symbol table of `q!`, generated from exact_units' `unit!` and `origin!` lines by its test\n\
         //! `tests/q_symbols.rs`: `just q-symbols` writes it, `just test` fails when it is stale. Don't edit it by hand.\n\
         \n\
         use super::entry::{Entry, Origin, Prefixes, Refused};\n\
         \n\
         /// The symbols read as unit markers, sorted.\n\
         #[rustfmt::skip]\n\
         pub(crate) static UNITS: &[Entry] = &[\n",
    );
    for s in &g.units {
        writeln!(
            out,
            "    Entry {{ symbol: {:?}, marker: \"units::{}\", group: {:?}, prefixes: Prefixes::{}, pow10: {} }},",
            s.symbol,
            s.marker,
            s.group,
            prefixes_text(s.prefixes),
            s.pow10
        )
        .unwrap();
    }
    out.push_str(
        "];\n\n/// The symbols refused, as they read two ways, sorted.\n#[rustfmt::skip]\npub(crate) static REFUSED: &[Refused] = &[\n",
    );
    for (symbol, readings) in &g.refused {
        writeln!(
            out,
            "    Refused {{ symbol: {symbol:?}, readings: &{readings:?} }},"
        )
        .unwrap();
    }
    out.push_str(
        "];\n\n/// The symbols read as points at an origin, sorted.\n#[rustfmt::skip]\npub(crate) static ORIGINS: &[Origin] = &[\n",
    );
    for (symbol, path, group) in &g.origins {
        writeln!(
            out,
            "    Origin {{ symbol: {symbol:?}, path: {path:?}, group: {group:?} }},"
        )
        .unwrap();
    }
    out.push_str("];\n");
    out
}

/// `e` in superscript digits: `⁻²`.
fn superscript(e: i8) -> String {
    e.to_string()
        .chars()
        .map(|c| match c {
            '-' => '⁻',
            d => "⁰¹²³⁴⁵⁶⁷⁸⁹"
                .chars()
                .nth(usize::try_from(d.to_digit(10).unwrap()).unwrap())
                .unwrap(),
        })
        .collect()
}

/// The tables of the book's page `docs/start/symbols.md`, by the name of their generated block.
fn docs_blocks(g: &Generated) -> Vec<(&'static str, String)> {
    let mut units = String::from("| Symbol | Marker | Feature | Prefixes |\n|---|---|---|---|\n");
    for s in &g.units {
        let prefixes = match s.prefixes {
            Prefixes::None => "none",
            Prefixes::Si => "SI",
            Prefixes::SiBinary => "SI and binary",
        };
        let marker = match s.pow10 {
            0 => format!("`{}`", s.marker),
            e => format!("`{}` · 10{}", s.marker, superscript(e)),
        };
        writeln!(
            units,
            "| `{}` | {marker} | `{}` | {prefixes} |",
            s.symbol, s.group
        )
        .unwrap();
    }
    let mut refused = String::from("| Symbol | Reads as | Write instead |\n|---|---|---|\n");
    for (symbol, readings) in &g.refused {
        let types: Vec<String> = readings
            .iter()
            .map(|r| match r.split_once(' ') {
                Some((p, m)) => format!("{p} `{m}`"),
                None => format!("`{r}`"),
            })
            .collect();
        writeln!(
            refused,
            "| `{symbol}` | {} | {} |",
            types.join(", "),
            readings
                .iter()
                .filter(|r| !r.contains(' '))
                .map(|r| format!("`q!(1 {r})`"))
                .collect::<Vec<_>>()
                .join(", ")
        )
        .unwrap();
    }
    let mut origins = String::from("| Symbol | Origin | Feature |\n|---|---|---|\n");
    for (symbol, path, group) in &g.origins {
        writeln!(origins, "| `{symbol}` | `{path}` | `{group}` |").unwrap();
    }
    let mut types = String::from("| `unit!` symbol | Marker |\n|---|---|\n");
    for (symbol, marker) in &g.not_symbols {
        writeln!(types, "| {symbol} | `{marker}` |").unwrap();
    }
    let mut folded = String::from("| Symbol | `unit!` line | Read as |\n|---|---|---|\n");
    for (symbol, marker) in &g.folded {
        let (p, unit) = PREFIXES
            .iter()
            .find_map(|p| {
                let rest = symbol.strip_prefix(p.text)?;
                g.units.iter().find(|u| u.symbol == rest).map(|u| (p, u))
            })
            .unwrap();
        writeln!(
            folded,
            "| `{symbol}` | `{marker}` | {} `{}` |",
            prefix_name(p),
            unit.marker
        )
        .unwrap();
    }
    vec![
        ("units", units),
        ("folded", folded),
        ("refused", refused),
        ("origins", origins),
        ("types", types),
    ]
}

/// The page with each generated block replaced.
fn docs_page(page: &str, g: &Generated) -> String {
    let mut out = page.to_owned();
    for (name, table) in docs_blocks(g) {
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
    out
}

/// The committed table and page are the generator's: `Q_SYMBOLS=update` (`just q-symbols`) writes them.
#[test]
fn table_and_page_are_generated() {
    let g = generate();
    let update = std::env::var("Q_SYMBOLS").as_deref() == Ok("update");
    let files = [
        ("macros/src/pure/table.rs", None),
        ("docs/start/symbols.md", Some(())),
    ];
    for (file, page) in files {
        let path = root().join(file);
        let old = std::fs::read_to_string(&path).unwrap();
        let new = match page {
            None => table_rs(&g),
            Some(()) => docs_page(&old, &g),
        };
        if update {
            std::fs::write(&path, new).unwrap();
        } else {
            assert!(new == old, "{file} is stale: run `just q-symbols`");
        }
    }
}

/// `q!` reads every symbol of the generator as the generator says: a table symbol as its line, a prefixed form as the
/// prefix and its line, an origin as the origin, a refused symbol never.
#[test]
fn resolve_agrees() {
    use resolve::{Symbol, resolve};
    let g = generate();
    for (symbol, _) in &g.refused {
        assert!(resolve(symbol).is_err(), "{symbol} is refused");
    }
    for (symbol, path, _) in &g.origins {
        assert!(
            matches!(resolve(symbol), Ok(Symbol::Origin(o)) if o.path == path),
            "{symbol} is {path}"
        );
    }
    let refused = |s: &str| g.refused.iter().any(|(r, _)| r == s);
    for s in &g.units {
        if !refused(&s.symbol) {
            assert!(
                matches!(resolve(&s.symbol), Ok(Symbol::Unit { entry, prefix: None }) if entry.symbol == s.symbol),
                "{} is {}",
                s.symbol,
                s.marker
            );
        }
        for p in PREFIXES.iter().filter(|p| takes(s.prefixes, p)) {
            let form = format!("{}{}", p.text, s.symbol);
            if refused(&form) {
                continue;
            }
            assert!(
                matches!(resolve(&form), Ok(Symbol::Unit { entry, prefix: Some(q) }) if entry.symbol == s.symbol && q == p),
                "{form} is {} {}",
                prefix_name(p),
                s.marker
            );
        }
    }
    for (symbol, marker) in &g.folded {
        assert!(
            matches!(
                resolve(symbol),
                Ok(Symbol::Unit {
                    prefix: Some(_),
                    ..
                })
            ),
            "{symbol} ({marker}) is a prefix and a unit"
        );
    }
}

/// What the run-time text reader makes of `1 {form}` for the marker `M`, against `10^pow10 · 2^pow2` of it.
fn reads<M: exact_units::Unit>(form: &str, pow10: i32, pow2: u32) -> Result<(), String> {
    use exact_units::Exact;
    let ten = 10i128.pow(pow10.unsigned_abs());
    let two = 2i128.pow(pow2);
    let want = if pow10 >= 0 {
        Exact::<M>::new(ten * two, 1)
    } else {
        Exact::<M>::new(two, ten.unsigned_abs())
    };
    match format!("1 {form}").parse::<Exact<M>>() {
        Ok(v) if v == want => Ok(()),
        Ok(v) => Err(format!("{form}: text reads {v}, q! {want}")),
        Err(_) => Err(format!("{form}: text refuses it")),
    }
}

/// Every spelling `q!` reads of a unit (its symbol, and with each prefix it takes) reads the same through the run-time
/// text reader (`Exact::from_str`, the reader of `serde::with_unit`), or the text reader refuses it: never another
/// value. The spellings only `q!` reads are listed, so a new one is a decision.
#[test]
fn text_reader_agrees() {
    let g = generate();
    let refused = |s: &str| g.refused.iter().any(|(r, _)| r == s);
    let mut only_q = Vec::new();
    let mut differ = Vec::new();
    for s in &g.units {
        let mut forms = vec![];
        if !refused(&s.symbol) {
            forms.push((s.symbol.clone(), i32::from(s.pow10), 0));
        }
        for p in PREFIXES.iter().filter(|p| takes(s.prefixes, p)) {
            let form = format!("{}{}", p.text, s.symbol);
            if !refused(&form) {
                forms.push(match p.scale {
                    prefix::Scale::Pow10(e) => (form, e + i32::from(s.pow10), 0),
                    prefix::Scale::Pow1024(e) => (form, i32::from(s.pow10), e * 10),
                });
            }
        }
        for (form, pow10, pow2) in forms {
            macro_rules! markers {
                ($($m:ident),* $(,)?) => {
                    match s.marker.as_str() {
                        $(stringify!($m) => reads::<units::$m>(&form, pow10, pow2),)*
                        m => panic!("add {m} to the markers of this test"),
                    }
                };
            }
            #[rustfmt::skip]
            let r = markers!(
                Absorbance, Ampere, Arcminute, Arcsecond, AtmosphereStandard, Baud, Becquerel, Bit, Block, BtuIt,
                BtuMean, BtuTh, Byte, CalorieFoodIt, CalorieFoodTh, CalorieIt, CalorieMean, CalorieTh, Candela,
                Cental, Char, Chunk, CircularMil, Code, Coulomb, Cycle, Day, Degree, Doubleword, ElectronVolt, Entry,
                Farad, Frame, Gon, Grain, Gram, Gray, Halfword, Henry, Hertz, Hour, Inch, Item, Joule, Katal, Kelvin,
                KilocalorieIt, KilocalorieTh, KilogramForce, Kip, Knot, Ksi, Line, Litre, Lumen, Lux, Meter,
                Microinch, MilePerHour, Mole, Newton, Nibble, Nit, Ohm, OpticalDepth, OunceForce, Packet, Pascal,
                Pennyweight, Poundal, PoundForce, Psf, Psi, Quad, Radian, Ratio, Record, Rood, Row, Sample, Second,
                Sector, Siemens, Sievert, Slug, Steradian, Stone, StreckSwedish, Tesla, Thou, TonForceShort, Tonne,
                TonOfRefrigeration, TonOfTnt, TonShort, Torr, Turn, Var, Volt, VoltAmpere, Watt, Weber, Week, Word,
            );
            match r {
                Ok(()) => {}
                Err(e) if e.ends_with("refuses it") => only_q.push(e),
                Err(e) => differ.push(e),
            }
        }
    }
    assert!(
        differ.is_empty(),
        "q! and the text reader differ:\n{}",
        differ.join("\n")
    );
    // what only q! reads: the ASCII spellings of ALIASES (a `q!` token can't be `Ω`), and the binary prefixes past
    // tebi, which the text reader's `Prefix` doesn't have
    let gap = |e: &String| {
        let form = e.split_once(':').unwrap().0;
        let base = PREFIXES
            .iter()
            .find_map(|p| {
                form.strip_prefix(p.text)
                    .filter(|b| ALIASES.iter().any(|a| a.0 == *b))
            })
            .unwrap_or(form);
        ALIASES.iter().any(|a| a.0 == base && a.0.is_ascii())
            || ["Pi", "Ei", "Zi", "Yi"].iter().any(|p| form.starts_with(p))
    };
    let other: Vec<&String> = only_q.iter().filter(|e| !gap(e)).collect();
    assert!(other.is_empty(), "only q! reads these: {other:?}");
}
