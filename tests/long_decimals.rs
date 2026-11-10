//! No long decimal in the book (`docs/`) or the examples (`examples/`): a result shown as 9.542425094393248 is checked
//! with `is_in(q!(9.542425... V))`, and a value is built by a computation or `fixed!` with a named rounding, not
//! from a float's digits. A decimal of [LONG] digits or more after its point fails this test, unless it is exact
//! digits listed in [ALLOWED] with the reason it stays: the decimal of a binary step, an exact definition, the text
//! `Display` writes. `just lint` runs it, and `just test` does too.
#![expect(
    clippy::arithmetic_side_effects,
    reason = "a test steps through the bytes of a line, far below usize::MAX"
)]
#![expect(
    clippy::unwrap_used,
    reason = "the test fails by panicking, on a file it can't read"
)]

use std::fs;
use std::path::{Path, PathBuf};

/// A decimal with this many digits after its point or more is long.
const LONG: usize = 10;

/// The long decimals that are exact digits and stay, each with the reason.
const ALLOWED: &[(&str, &str)] = &[
    (
        "1.25663706212",
        "μ₀ as CODATA 2018 prints it: the set's exact value (constants::Codata2018)",
    ),
    (
        "1.25663706127",
        "μ₀ as CODATA 2022 prints it: the set's exact value (constants::Codata2022)",
    ),
    (
        "1.66053906660",
        "m_u as CODATA 2018 prints it, in kg: the set's exact value (constants::Codata2018)",
    ),
    (
        "1.66053906892",
        "m_u as CODATA 2022 prints it, in kg: the set's exact value (constants::Codata2022)",
    ),
    (
        "9.1093837015",
        "m_e as CODATA 2018 prints it, in kg: the set's exact value (constants::Codata2018)",
    ),
    (
        "9.1093837139",
        "m_e as CODATA 2022 prints it, in kg: the set's exact value (constants::Codata2022)",
    ),
    (
        "1.67262192369",
        "m_p as CODATA 2018 prints it, in kg: the set's exact value (constants::Codata2018)",
    ),
    (
        "1.67262192595",
        "m_p as CODATA 2022 prints it, in kg: the set's exact value (constants::Codata2022)",
    ),
    (
        "5.48579909065",
        "A_r(e) as CODATA 2018 prints it: the set's exact value (constants::Codata2018)",
    ),
    (
        "5.485799090441",
        "A_r(e) as CODATA 2022 prints it: the set's exact value (constants::Codata2022)",
    ),
    (
        "1.007276466621",
        "A_r(p) as CODATA 2018 prints it: the set's exact value (constants::Codata2018)",
    ),
    (
        "1.0072764665789",
        "A_r(p) as CODATA 2022 prints it: the set's exact value (constants::Codata2022)",
    ),
    (
        "0.99999999965",
        "M_u as CODATA 2018 prints it, in g/mol: N_A·m_u of the set, checked against it",
    ),
    (
        "7.2973525693",
        "α as CODATA 2018 prints it, what α from μ₀ is checked against",
    ),
    (
        "7.2973525643",
        "α as CODATA 2022 prints it, what α from μ₀ is checked against",
    ),
    (
        "0.09999999962747097015380859375",
        "the 2⁻³⁰ step below 0.1, exactly: fixed!(0.1)'s error names it",
    ),
    (
        "0.100000000558793544769287109375",
        "the 2⁻³⁰ step above 0.1, exactly: fixed!(0.1)'s error names it",
    ),
    (
        "0.0999999940395355224609375",
        "the f32 below 0.1, exactly: fixed!(0.1)'s error names it",
    ),
    (
        "0.100000001490116119384765625",
        "the f32 above 0.1, exactly: fixed!(0.1)'s error names it",
    ),
    (
        "0.100000001490116",
        "the leading digits of that f32, cut and marked `..`, never rounded",
    ),
    (
        "0.79998779296875",
        "a Q15 of 0.8 as `{:#}` writes it: the stored value, exactly",
    ),
    (
        "519.999755859375",
        "an STM32H7 PLL output, a finite decimal of a fraction: Exact's Display, exactly",
    ),
    ("173.333251953125", "that PLL output divided by 3, exactly"),
    (
        "79.99998046875",
        "an FDCAN clock, a finite decimal of a fraction: Exact's Display and parse, exactly",
    ),
];

/// Pages written by a generator (`just customary-docs`, `just q-symbols`), whose numbers are its own.
const GENERATED: &[&str] = &["docs/customary.md", "docs/start/symbols.md"];

/// Every file under `dir` whose extension is `ext`, recursively.
fn files(dir: &Path, ext: &str, out: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            files(&path, ext, out);
        } else if path.extension().is_some_and(|e| e == ext) {
            out.push(path);
        }
    }
}

/// The decimals in `line` with [LONG] digits or more after the point, as written.
fn long_decimals(line: &str) -> Vec<&str> {
    let b = line.as_bytes();
    let mut found = Vec::new();
    let mut i = 0;
    while let Some(c) = b.get(i) {
        if !c.is_ascii_digit() {
            i += 1;
            continue;
        }
        let start = i;
        while b.get(i).is_some_and(u8::is_ascii_digit) {
            i += 1;
        }
        if b.get(i) != Some(&b'.') {
            continue;
        }
        let point = i;
        i += 1;
        while b.get(i).is_some_and(u8::is_ascii_digit) {
            i += 1;
        }
        if i - point > LONG {
            found.push(line.get(start..i).unwrap());
        }
    }
    found
}

#[test]
fn finds_long_decimals() {
    assert_eq!(
        long_decimals("bounded(9.542425094393248); q!(9.542425... dB)"),
        ["9.542425094393248"]
    );
    assert_eq!(
        long_decimals("-6.0205999132 and 1.602176634"),
        ["6.0205999132"]
    );
    let none: [&str; 0] = [];
    assert_eq!(
        long_decimals("v0.1.0, 1..2, 9.54242509.., 12345678901"),
        none
    );
}

#[test]
fn book_and_examples_have_no_long_decimals() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut paths = Vec::new();
    files(&root.join("docs"), "md", &mut paths);
    files(&root.join("examples"), "rs", &mut paths);
    paths.sort();
    let mut bad = Vec::new();
    let mut seen = Vec::new();
    for path in &paths {
        let rel = path
            .strip_prefix(root)
            .unwrap()
            .to_string_lossy()
            .replace('\\', "/");
        if GENERATED.contains(&rel.as_str()) {
            continue;
        }
        let text = fs::read_to_string(path).unwrap();
        for (n, line) in text.lines().enumerate() {
            for d in long_decimals(line) {
                if ALLOWED.iter().any(|(a, _)| *a == d) {
                    seen.push(d.to_owned());
                } else {
                    bad.push(format!("{rel}:{}: {d}", n + 1));
                }
            }
        }
    }
    assert!(
        bad.is_empty(),
        "long decimals in the book or the examples; check a shown result with `is_in(q!(9.542425... V))` and build \
         a value by a computation, or list exact digits in ALLOWED with the reason they stay:\n{}",
        bad.join("\n")
    );
    let stale: Vec<_> = ALLOWED
        .iter()
        .filter(|(a, _)| !seen.iter().any(|s| s == a))
        .map(|(a, _)| *a)
        .collect();
    assert!(
        stale.is_empty(),
        "ALLOWED lists decimals no page has any more: {stale:?}"
    );
}
