//! The SI prefixes (BIPM, SI Brochure 9th ed. 2019, with ronna, quetta, ronto and quecto of 2022) and the binary
//! prefixes of IEC 80000-13, as `q!` splits them off a symbol.

/// A prefix's scale.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(crate) enum Scale {
    /// `10^e`.
    Pow10(i32),
    /// `1024^e`.
    Pow1024(u32),
}

/// A prefix: how it is written and its scale.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(crate) struct Prefix {
    /// The text before the unit's symbol.
    pub(crate) text: &'static str,
    /// Its scale.
    pub(crate) scale: Scale,
}

const fn si(text: &'static str, e: i32) -> Prefix {
    Prefix {
        text,
        scale: Scale::Pow10(e),
    }
}

const fn bin(text: &'static str, e: u32) -> Prefix {
    Prefix {
        text,
        scale: Scale::Pow1024(e),
    }
}

/// Every prefix `q!` reads. Micro is `μ` (Greek mu, U+03BC), `µ` (the micro sign, U+00B5, which rustc warns about
/// in an identifier: write it in a string) or `u`.
pub(crate) static PREFIXES: &[Prefix] = &[
    si("Q", 30),
    si("R", 27),
    si("Y", 24),
    si("Z", 21),
    si("E", 18),
    si("P", 15),
    si("T", 12),
    si("G", 9),
    si("M", 6),
    si("k", 3),
    si("h", 2),
    si("da", 1),
    si("d", -1),
    si("c", -2),
    si("m", -3),
    si("μ", -6),
    si("µ", -6),
    si("u", -6),
    si("n", -9),
    si("p", -12),
    si("f", -15),
    si("a", -18),
    si("z", -21),
    si("y", -24),
    si("r", -27),
    si("q", -30),
    bin("Ki", 1),
    bin("Mi", 2),
    bin("Gi", 3),
    bin("Ti", 4),
    bin("Pi", 5),
    bin("Ei", 6),
    bin("Zi", 7),
    bin("Yi", 8),
];
