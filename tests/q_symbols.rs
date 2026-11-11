//! The symbol table of `q!` (`macros/src/pure/table.rs`), the book's page of its symbols (`docs/start/symbols.md`)
//! and the long names and quantities its `[[symbol]]` tooltips show (`docs_ext/eu_docs_ext/unit_info.json`),
//! generated from the `unit!` and `origin!` lines, the catalogue they declare and the prefixes of `src/si_prefix.rs`.
//! `Q_SYMBOLS=update` (`just q-symbols`) writes all three; without it the test fails when any is stale, so the
//! table, the book and the markers agree.
//!
//! The rules, so that a symbol never reads two ways (AGENTS.md, rule 5):
//!
//! - A symbol is a table symbol when it is one word: no `/`, `·`, space, parenthesis, digit or exponent. A
//!   qualified one (`gal (US)`, `liq qt (US)`: `liq` and `dry` qualify like a parenthesis) gives its stem (`gal`,
//!   `qt`) a reading, and a stem is always refused, even with one reading: no bare default (`ft` is never the
//!   international foot while `ft (US survey)` exists).
//! - Prefixes go only on the units the SI lets take one: a coherent SI unit, and g, L, eV; t from kilo up; bit and B
//!   take the binary prefixes too. Never h, min, d, °, a customary unit or a counted one.
//! - A symbol's readings are its own lines, the stems, and every split into a prefix and a unit that takes that
//!   prefix (`cd` is the candela and the cord, but no centi-day, as the day takes no prefix; `mcd` is a millicandela,
//!   never a millicord).
//!   Readings of the same dimension, kind and exact scale fold (`km` is `Kilometre` and kilo `Meter`); a symbol left
//!   with two is refused, with its readings as what to write instead.
//! - Every unit group is on, so which symbols are refused never depends on the features a build turns on.
#![cfg(all(
    feature = "q",
    feature = "std",
    feature = "quantities",
    feature = "customary",
    feature = "nuclear"
))]
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
use exact_units::Prefix::{Exa, Giga, Kilo, Mega, Peta, Quetta, Ronna, Tera, Yotta, Zetta};
use exact_units::{Dim, Magnitude, PrefixPolicy, PrefixSet, UnitExpr, units};
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

/// Symbols `q!` reads as a point at an origin, never as a unit (a unit has no offset). `(symbol, origin)`.
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

/// Symbols `q!` reads as a level of a gain when they stand alone after the number (`q!(-3 dB)`), compared with a
/// gain whose kind picks 20·lg or 10·lg; inside a unit they stay refused. `(symbol, marker under literal::)`.
const LEVEL_SYMBOLS: &[(&str, &str)] = &[("Np", "Np"), ("dB", "Db")];

/// Words before a symbol that qualify it like a parenthesis does: `liq qt (US)` and `dry qt (US)` share the stem
/// `qt` with `qt (imp)`.
const QUALIFIER_WORDS: &[&str] = &["liq", "dry"];

/// Words after a symbol that name its system, as a parenthesis does: `oz avdp`, `lb t`, `dr ap`.
const TRAILING_QUALIFIER_WORDS: &[&str] = &["avdp", "t", "ap"];

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
    ) || matches!((prefixes, p.scale), (Prefixes::SiFromKilo, prefix::Scale::Pow10(e)) if e >= 3)
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
            UnitExpr::Named(def) => def.id.name,
            e => panic!("{e:?} in the catalogue"),
        })
        .collect();
    assert_eq!(names, in_catalogue, "the unit! lines read from source");

    let value = |line: &Line, pow10: i8| {
        let Some(UnitExpr::Named(def)) = units::by_name(&line.name) else {
            panic!("{} is no named unit", line.name);
        };
        assert_eq!(def.names.symbol, line.symbol, "the symbol of {}", line.name);
        Value {
            dim: def.dim,
            kind: line.kind.clone(),
            scale: Some(def.scale.mul(Magnitude::pow10(pow10))),
            far: None,
        }
    };
    // the prefixes a unit takes are its line's (UnitNames::prefixes): the SI's rules, written there
    let prefixes = |line: &Line| {
        let Some(UnitExpr::Named(def)) = units::by_name(&line.name) else {
            panic!("{} is no named unit", line.name);
        };
        match def.names.prefixes {
            PrefixPolicy::None => Prefixes::None,
            PrefixPolicy::Decimal => Prefixes::Si,
            PrefixPolicy::DecimalAndBinary => Prefixes::SiBinary,
            PrefixPolicy::Only(set)
                if set
                    == PrefixSet::of(&[
                        Kilo, Mega, Giga, Tera, Peta, Exa, Zetta, Yotta, Ronna, Quetta,
                    ]) =>
            {
                Prefixes::SiFromKilo
            }
            PrefixPolicy::Only(set) => {
                panic!("{}: a prefix set q! has no rule for: {set:?}", line.name)
            }
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
            .map(|(stem, q)| {
                let words: Vec<&str> = stem.split(' ').collect();
                match words.as_slice() {
                    [w, rest] if QUALIFIER_WORDS.contains(w) => (*rest, q),
                    _ => (stem, q),
                }
            })
            .filter(|(stem, q)| atomic(stem) && q.ends_with(')'))
        {
            stems.entry(stem.to_owned()).or_default().push(Reading {
                text: line.name.clone(),
                value: value(line, 0),
            });
        }
        // `oz avdp`, `lb t`, `dr ap`: a trailing system word qualifies too, where the stem is no unit's own symbol
        // (`s ap`, the scruple, leaves `s` the second)
        if let Some((stem, word)) = line.symbol.split_once(' ')
            && TRAILING_QUALIFIER_WORDS.contains(&word)
            && atomic(stem)
            && !lines.iter().any(|l| l.symbol == stem)
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
        let Some(UnitExpr::Named(def)) = units::by_name(&line.name) else {
            panic!("{} is no named unit", line.name);
        };
        aliases.extend(def.names.aliases().map(|s| (s, line.name.as_str(), 0)));
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
        // a symbol splits only into a prefix and a unit that takes it: `yd` is no yocto-day, `mcd` no millicord
        for p in PREFIXES {
            let Some(rest) = c.strip_prefix(p.text) else {
                continue;
            };
            for s in syms
                .iter()
                .filter(|s| s.symbol == rest && takes(s.prefixes, p))
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
        // a stem of a qualified symbol is never read as one of its units, so no bare default is picked (`qt` is
        // refused although only `qt (imp)` is atomic before its parenthesis)
        if distinct.len() > 1 || stems.contains_key(c) {
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
        Prefixes::SiFromKilo => "SiFromKilo",
    }
}

/// `macros/src/pure/table.rs`, one line per entry, `#[rustfmt::skip]` so rustfmt leaves it as written.
fn table_rs(g: &Generated) -> String {
    let mut out = String::from(
        "//! The symbol table of `q!`, generated from exact_units' `unit!` and `origin!` lines by its test\n\
         //! `tests/q_symbols.rs`: `just q-symbols` writes it, `just test` fails when it is stale. Don't edit it by hand.\n\
         \n\
         use super::entry::{Entry, Level, Origin, Prefixes, Refused};\n\
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
    out.push_str(
        "];\n\n/// The symbols read as a level of a gain when they stand alone after the number, sorted.\n#[rustfmt::skip]\npub(crate) static LEVELS: &[Level] = &[\n",
    );
    for (symbol, marker) in LEVEL_SYMBOLS {
        writeln!(
            out,
            "    Level {{ symbol: {symbol:?}, marker: \"literal::{marker}\" }},"
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

/// Whether Rust's lexer takes `s` as one identifier, so `q!` reads it without quotes: `degC`, not `°C` or `℃`.
fn bare_ident(s: &str) -> bool {
    let mut c = s.chars();
    c.next().is_some_and(|f| f.is_alphabetic() || f == '_')
        && c.all(|x| x.is_alphanumeric() || x == '_')
}

/// An origin's spellings: those written bare, those written in quotes.
type Spellings<'a> = (Vec<&'a str>, Vec<&'a str>);

/// The origins' table of the book's page: one row per origin, its spellings split by whether Rust's lexer takes
/// them bare.
fn origins_table(g: &Generated) -> String {
    let mut by_origin: BTreeMap<(&str, &str), Spellings<'_>> = BTreeMap::new();
    for (symbol, path, group) in &g.origins {
        let row = by_origin.entry((path, group)).or_default();
        if bare_ident(symbol) {
            row.0.push(symbol);
        } else {
            row.1.push(symbol);
        }
    }
    let mut origins = String::from(
        "| Origin | Feature | Written bare | Written in quotes |\n|---|---|---|---|\n",
    );
    for ((path, group), (bare, quoted)) in &by_origin {
        let list = |v: &[&str], q: &str| {
            v.iter()
                .map(|s| format!("`{q}{s}{q}`"))
                .collect::<Vec<_>>()
                .join(", ")
        };
        writeln!(
            origins,
            "| `{path}` | `{group}` | {} | {} |",
            list(bare, ""),
            list(quoted, "\"")
        )
        .unwrap();
    }
    origins
}

/// The levels' table of the book's page: each level symbol, its marker and its gain of each form.
fn levels_table() -> String {
    let mut levels = String::from(
        "| Symbol | Level | A gain of V/V, A/A, Pa/Pa | A gain of W/W |\n|---|---|---|---|\n",
    );
    for (symbol, marker) in LEVEL_SYMBOLS {
        let (root, power) = match *marker {
            "Db" => ("10^(x/20)", "10^(x/10)"),
            "Np" => ("e^x", "e^(2x)"),
            m => panic!("no level form for {m}"),
        };
        writeln!(
            levels,
            "| `{symbol}` | `literal::{marker}` | {root} | {power} |"
        )
        .unwrap();
    }
    levels
}

/// The tables of the book's page `docs/start/symbols.md`, by the name of their generated block.
fn docs_blocks(g: &Generated) -> Vec<(&'static str, String)> {
    let mut units = String::from("| Symbol | Marker | Feature | Prefixes |\n|---|---|---|---|\n");
    for s in &g.units {
        let prefixes = match s.prefixes {
            Prefixes::None => "none",
            Prefixes::Si => "SI",
            Prefixes::SiBinary => "SI and binary",
            Prefixes::SiFromKilo => "SI from kilo up",
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
    let origins = origins_table(g);
    let levels = levels_table();
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
        ("levels", levels),
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

/// The quantity a unit measures, as the book's tooltips name it: its line's `quantity`, else its dimension's open
/// root (`Derived(L)` is a length), else "dimensionless" for dimension one, else the name of the dimension's one
/// coherent unit with a compound symbol (`units::Area`, m², "area"); `None` where none of these is.
fn quantity_name(def: &exact_units::UnitDef<'_>) -> Option<String> {
    use exact_units::QuantityRef;
    if let Some(q) = def.quantity {
        return Some(match q {
            QuantityRef::Def(d) => d.name().to_string(),
            QuantityRef::Id(id) => exact_units::quantity::QuantityName(id.name).to_string(),
        });
    }
    if let Some(root) = exact_units::quantities::ALL
        .iter()
        .find(|q| q.is_open_root() && q.dim.same(def.dim))
    {
        return Some(root.name().to_string());
    }
    if def.dim.is_one() {
        return Some("dimensionless".to_owned());
    }
    // a coherent unit with a compound symbol is named after its quantity (`Area`, m²; `Jerk`, m/s³)
    let mut coherent = units::ALL.iter().filter_map(|e| match e {
        UnitExpr::Named(u)
            if u.dim.same(def.dim)
                && u.quantity.is_none()
                && u.scale == Magnitude::ONE
                && !atomic(u.names.symbol) =>
        {
            Some(exact_units::quantity::QuantityName(u.id.name).to_string())
        }
        _ => None,
    });
    match (coherent.next(), coherent.next()) {
        (Some(name), None) => Some(name),
        _ => None,
    }
}

/// A marker's entry of `unit_info.json`: its long name, plural and quantity, `null` where it has none.
fn info_entry(long: &str, plural: &str, quantity: Option<&str>) -> serde_json::Value {
    let text = |s: &str| (!s.is_empty()).then(|| s.to_owned());
    serde_json::json!({ "long": text(long), "plural": text(plural), "quantity": quantity })
}

/// The origins of `src/point/origins.rs`, each with the unit its `origin!` line names: `(origin, unit)`.
fn origin_units() -> Vec<(String, String)> {
    let src = std::fs::read_to_string(root().join("src/point/origins.rs")).unwrap();
    let mut out = Vec::new();
    for block in src.split("crate::origin! {").skip(1) {
        let (_, rest) = block.split_once("pub struct ").unwrap();
        let name: String = rest
            .chars()
            .take_while(char::is_ascii_alphanumeric)
            .collect();
        let (_, unit) = rest.split_once("unit: ").unwrap();
        let unit: String = unit
            .chars()
            .take_while(char::is_ascii_alphanumeric)
            .collect();
        out.push((name, unit));
    }
    out
}

/// `docs_ext/eu_docs_ext/unit_info.json`, what the book's `[[symbol]]` and `\unit{}` tooltips show: every unit marker
/// of the catalogue (the `unit!` lines), origin and level with its long name, plural and quantity, and every prefix
/// of [exact_units::Prefix::ALL] with its name, kind and power of ten.
fn unit_info_json() -> String {
    let unit_of = |name: &str| match units::by_name(name) {
        Some(UnitExpr::Named(def)) => *def,
        _ => panic!("{name} is no named unit"),
    };
    let mut unit_map = serde_json::Map::new();
    for e in units::ALL.iter().chain(units::CUSTOMARY) {
        let UnitExpr::Named(def) = e else {
            panic!("{e:?} in the catalogue")
        };
        unit_map.insert(
            def.id.name.to_owned(),
            info_entry(
                def.names.long,
                def.names.plural,
                quantity_name(def).as_deref(),
            ),
        );
    }
    // an origin is named after its marker, as a quantity is (`Celsius` "celsius"), and measures its unit's quantity
    let mut origin_map = serde_json::Map::new();
    for (origin, unit) in origin_units() {
        let name = exact_units::quantity::QuantityName(&origin).to_string();
        let quantity = quantity_name(unit_of(&unit));
        origin_map.insert(origin, info_entry(&name, "", quantity.as_deref()));
    }
    // a level of a gain is named as the unit of its symbol (`dB` the decibel)
    let mut level_map = serde_json::Map::new();
    for (symbol, marker) in LEVEL_SYMBOLS {
        let def = units::ALL
            .iter()
            .find_map(|e| match e {
                UnitExpr::Named(def) if def.names.symbol == *symbol => Some(*def),
                _ => None,
            })
            .unwrap_or_else(|| panic!("no unit of the level symbol {symbol}"));
        level_map.insert(
            (*marker).to_owned(),
            info_entry(def.names.long, def.names.plural, Some("gain level")),
        );
    }
    let mut prefix_map = serde_json::Map::new();
    for p in exact_units::Prefix::ALL
        .iter()
        .filter(|p| **p != exact_units::Prefix::Unit)
    {
        let kind = if p.si_pow10().is_some() {
            "si"
        } else {
            "binary"
        };
        prefix_map.insert(
            p.symbol().to_owned(),
            serde_json::json!({ "name": p.name().to_lowercase(), "kind": kind, "exp": p.si_pow10() }),
        );
    }
    let info = serde_json::json!({
        "_generated": "by tests/q_symbols.rs from the unit! and origin! lines and src/si_prefix.rs: run `just q-symbols`, don't edit it by hand",
        "units": unit_map,
        "origins": origin_map,
        "levels": level_map,
        "prefixes": prefix_map,
    });
    let mut out = serde_json::to_string_pretty(&info).unwrap();
    out.push('\n');
    out
}

/// Every marker `q!`'s table reads, every origin and level has a long name and a quantity in `unit_info.json`, so
/// no tooltip of the book is left without one; a unit that is a base of its own (a counted thing, a level) has no
/// quantity.
#[test]
fn every_table_marker_has_a_name_and_quantity() {
    let info: serde_json::Value = serde_json::from_str(&unit_info_json()).unwrap();
    let mut missing = Vec::new();
    let markers = table::UNITS
        .iter()
        .map(|e| ("units", e.marker.trim_start_matches("units::")))
        .chain(
            table::ORIGINS
                .iter()
                .map(|o| ("origins", o.path.trim_start_matches("origins::"))),
        )
        .chain(
            table::LEVELS
                .iter()
                .map(|l| ("levels", l.marker.trim_start_matches("literal::"))),
        );
    // a counted thing (`Item`) or a level (`Stop`, Np) is its own base: no quantity names it
    let own_base = |kind: &str, marker: &str| {
        kind == "units"
            && units::by_name(marker)
                .and_then(UnitExpr::def)
                .is_some_and(|d| {
                    !d.dim.has_true_exps()
                        && [
                            d.id,
                            exact_units::MarkerId::NEPER,
                            exact_units::MarkerId::NEPER_LOSS,
                        ]
                        .iter()
                        .any(|b| !d.dim.exp_of(*b).is_zero())
                })
    };
    for (kind, marker) in markers {
        let entry = &info[kind][marker];
        if !entry["long"].is_string() || !(entry["quantity"].is_string() || own_base(kind, marker))
        {
            missing.push(format!("{kind}::{marker}: {entry}"));
        }
    }
    missing.sort();
    missing.dedup();
    assert!(
        missing.is_empty(),
        "markers of q!'s table without a long name or quantity:\n{}",
        missing.join("\n")
    );
}

/// The committed table, page and tooltip data are the generator's: `Q_SYMBOLS=update` (`just q-symbols`) writes
/// them.
#[test]
fn table_and_page_are_generated() {
    let g = generate();
    let update = std::env::var("Q_SYMBOLS").as_deref() == Ok("update");
    let files = [
        ("macros/src/pure/table.rs", None),
        ("docs/start/symbols.md", Some(true)),
        ("docs_ext/eu_docs_ext/unit_info.json", Some(false)),
    ];
    for (file, page) in files {
        let path = root().join(file);
        let old = std::fs::read_to_string(&path).unwrap_or_default();
        let new = match page {
            None => table_rs(&g),
            Some(true) => docs_page(&old, &g),
            Some(false) => unit_info_json(),
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
/// No bare symbol reads as a unit whose own symbol is qualified (`qt` is never `qt (imp)`, `ft` never `ft (US survey)`
/// nor the international foot beside it, `t` of `lb t` never the troy pound): every symbol `q!` reads is a unit whose
/// own symbol is exactly one word, and every word of a qualified symbol either is refused or reads as a unit of its
/// own one-word symbol.
#[test]
fn no_bare_symbol_reads_a_qualified_unit() {
    use resolve::{Symbol, resolve};
    let own = |marker: &str| match units::by_name(marker.trim_start_matches("units::")) {
        Some(UnitExpr::Named(def)) => def.names.symbol,
        _ => panic!("{marker} is no named unit"),
    };
    // qualified: a parenthesis or a qualifier word (`gal (US)`, `liq qt (US)`, `oz t`); `cd/m²` is a compound
    let qualified = |s: &str| {
        let words: Vec<&str> = s.split(' ').collect();
        s.contains(" (")
            || (words.len() > 1
                && (words.first().is_some_and(|w| QUALIFIER_WORDS.contains(w))
                    || words
                        .iter()
                        .skip(1)
                        .any(|w| TRAILING_QUALIFIER_WORDS.contains(w))))
    };
    for e in table::UNITS {
        assert!(
            !qualified(own(e.marker)),
            "{} reads as {}, whose own symbol {:?} is qualified",
            e.symbol,
            e.marker,
            own(e.marker)
        );
    }
    let g = generate();
    for (symbol, marker) in &g.not_symbols {
        for word in symbol.split([' ', '(', ')']).filter(|w| !w.is_empty()) {
            if let Ok(Symbol::Unit { entry, .. }) = resolve(word) {
                assert_ne!(
                    entry.marker.trim_start_matches("units::"),
                    marker,
                    "`{word}` of `{symbol}` reads as {marker}"
                );
            }
        }
    }
    for stem in [
        "qt", "pt", "gal", "ft", "mi", "oz", "lb", "nmi", "bbl", "cup", "tbsp", "tsp",
    ] {
        assert!(
            resolve(stem).is_err(),
            "{stem} is the stem of qualified symbols and refused"
        );
    }
}

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
    for (symbol, marker) in LEVEL_SYMBOLS {
        assert!(
            g.refused.iter().any(|(r, _)| r == symbol),
            "{symbol} is refused inside a unit, a level only alone"
        );
        assert!(
            matches!(resolve::level(symbol), Some(l) if l.marker == format!("literal::{marker}")),
            "{symbol} is the level literal::{marker}"
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
                Absorbance, Ampere, Arcminute, Arcsecond, AtmosphereStandard, AtmosphereTechnical, Barn, Baud, Becquerel, Bit, Block, BtuIt,
                BtuMean, BtuTh, Byte, CalorieFoodIt, CalorieFoodTh, CalorieIt, CalorieMean, CalorieTh, Candela,
                Cental, Char, Chunk, CircularMil, Code, Coulomb, Curie, Cycle, Dalton, Day, Degree, Doubleword, ElectronVolt, Entry,
                Farad, Frame, Gon, Grain, Gram, Gray, Halfword, Hand, Henry, Hertz, Hour, Inch, Item, JulianYear, Joule, Katal, Kelvin,
                KilocalorieIt, KilocalorieTh, KilogramForce, Kip, Knot, Ksi, Line, Litre, Lumen, Lux, Meter,
                Microinch, MilePerHour, Mole, Newton, Nibble, Nit, Ohm, OpticalDepth, OunceForce, Packet, Pascal,
                Pennyweight, Poundal, PoundForce, Psf, Psi, Quad, Rad, Radian, Ratio, Record, Rem, Roentgen, Rood, Row, Sample, Second,
                Sector, Siemens, Sievert, Slug, Steradian, Stone, Stop, StreckSwedish, Tesla, Thou, TonForceShort, Tonne,
                TonOfRefrigeration, TonOfTnt, TonShort, Torr, Turn, Var, Volt, VoltAmpere, Watt, Weber, Week, Word, Yard,
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
