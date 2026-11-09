//! A unit in another notation than its Unicode symbol: ASCII, UCUM and Typst, by [Notation].

use crate::tree::{self, MAX_UNIT_DEPTH, Node, Tree, is_ratio, takes_bare_prefix};
use crate::{Prefix, UnitNames};
use core::fmt::{self, Write};

/// How a unit is written: [unit_in] and [symbol_in] take one.
///
/// ```
/// # #[cfg(all(feature = "mechanical", feature = "time"))] {
/// use exact_units::units::{Meter, Per, Second, Speed};
/// use exact_units::{Notation, Unit, notation};
///
/// type MPerS = Per<Meter, Second>;
/// let all = |e| {
///     [Notation::Unicode, Notation::Ascii, Notation::Ucum, Notation::Typst]
///         .map(|n| notation::symbol_in(e, n).map(|s| s.to_string()))
/// };
/// assert_eq!(all(&MPerS::EXPR), [
///     Some("m/s".into()),
///     Some("m/s".into()),
///     Some("m/s".into()),
///     Some("upright(m)/upright(s)".into()),
/// ]);
/// assert_eq!(all(&Speed::EXPR), all(&MPerS::EXPR));
/// # }
/// ```
#[non_exhaustive]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum Notation {
    /// The symbol as `{}` writes it: `Ω`, `m/s²`, `N·m`, `√Hz`, `cm²`.
    Unicode,
    /// The symbol in ASCII ([UnitNames::write_ascii]): `Ohm`, `m/s^2`, `N*m`, `sqrt(Hz)`, micro as `u`.
    Ascii,
    /// The case-sensitive code of UCUM (the Unified Code for Units of Measure): `Ohm`, `m/s2`, `N.m`, `By`. A named
    /// unit is written by its [UnitNames::ucum] code, so one without a code isn't written, nor is a root.
    Ucum,
    /// Typst math: `upright(Ω)`, `upright(m)/upright(s)^2`, `upright(N) dot.op upright(m)`.
    Typst,
}

/// Why a walk stopped: the writer failed, or a part has no form in the notation.
enum Stop {
    Fmt,
    NoForm,
}

impl From<fmt::Error> for Stop {
    fn from(_: fmt::Error) -> Self {
        Stop::Fmt
    }
}

/// The unit as written after a number in `notation`, with `prefix` (`" km/s"`), `None` where it has no form there:
/// a unit without a UCUM code or a root in [Notation::Ucum], a symbol without an ASCII form in [Notation::Ascii], a
/// scale without a prefix's name other than in [Notation::Unicode]. [Notation::Unicode] is
/// [tree::write_unit]'s, the way `{}` writes it.
pub fn unit_in<'t, E: Tree<'t>>(
    notation: Notation,
    prefix: Prefix,
    unit: &E,
) -> Option<impl fmt::Display + '_> {
    let shown = Shown {
        notation,
        prefix,
        unit,
        lead: true,
    };
    shown.check().then_some(shown)
}

/// The symbol in `notation`, without a leading space or a prefix: `m/s`, `upright(m)/upright(s)`; `None` like
/// [unit_in].
pub fn symbol_in<'t, E: Tree<'t>>(e: &E, notation: Notation) -> Option<impl fmt::Display + '_> {
    let shown = Shown {
        notation,
        prefix: Prefix::Unit,
        unit: e,
        lead: false,
    };
    shown.check().then_some(shown)
}

/// A unit to write in a notation, checked to have a form there.
struct Shown<'e, E> {
    notation: Notation,
    prefix: Prefix,
    unit: &'e E,
    /// Whether a space goes first, as after a number.
    lead: bool,
}

impl<'t, E: Tree<'t>> Shown<'_, E> {
    fn check(&self) -> bool {
        !matches!(self.write(&mut Sink), Err(Stop::NoForm))
    }

    fn write(&self, f: &mut dyn Write) -> Result<(), Stop> {
        let n = self.notation;
        if n == Notation::Unicode {
            let show = UnicodeUnit(self.prefix, self.unit);
            let mut out = Buffered::default();
            write!(out, "{show}")?;
            let text = out.as_str();
            return Ok(f.write_str(if self.lead {
                text
            } else {
                text.strip_prefix(' ').unwrap_or(text)
            })?);
        }
        if is_ratio(self.unit) {
            return ratio(n, self.prefix, f, self.lead);
        }
        if self.lead {
            f.write_char(' ')?;
        }
        if self.prefix == Prefix::Unit {
            return write_tree(n, self.unit, None, f, 0);
        }
        if !self.prefix.is_named() {
            return Err(Stop::NoForm);
        }
        if takes_bare_prefix(self.unit) {
            return write_tree(n, self.unit, Some(self.prefix), f, 0);
        }
        // a compound unit takes its prefix on the whole of it, as `{}` writes `m(V/A)`
        match n {
            Notation::Ucum => {
                let (base, e) = match (self.prefix.pow10(), self.prefix) {
                    (Some(e), _) => (10, i32::from(e)),
                    (None, Prefix::Kibi) => (2, 10),
                    (None, Prefix::Mebi) => (2, 20),
                    (None, Prefix::Gibi) => (2, 30),
                    (None, Prefix::Tebi) => (2, 40),
                    _ => return Err(Stop::NoForm),
                };
                write!(f, "{base}*{e}.(")?;
            }
            Notation::Typst => {
                f.write_str("upright(\"")?;
                f.write_str(prefix_text(n, self.prefix))?;
                f.write_str("\")(")?;
            }
            _ => {
                f.write_str(prefix_text(n, self.prefix))?;
                f.write_char('(')?;
            }
        }
        write_tree(n, self.unit, None, f, 0)?;
        Ok(f.write_char(')')?)
    }
}

impl<'t, E: Tree<'t>> fmt::Display for Shown<'_, E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.write(f).map_err(|_| fmt::Error)
    }
}

/// [tree::write_unit] as a `Display`.
struct UnicodeUnit<'e, E>(Prefix, &'e E);

impl<'t, E: Tree<'t>> fmt::Display for UnicodeUnit<'_, E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        tree::write_unit(self.0, self.1, f)
    }
}

/// A writer that keeps nothing: the dry run of [Shown::check].
struct Sink;

impl Write for Sink {
    fn write_str(&mut self, _: &str) -> fmt::Result {
        Ok(())
    }
}

/// The Unicode form, kept to strip its leading space: as long as a unit's text can be (a tree of [MAX_UNIT_DEPTH]
/// levels of short symbols), past it the text is cut, never a panic.
struct Buffered {
    bytes: [u8; 512],
    len: usize,
}

impl Default for Buffered {
    fn default() -> Self {
        Buffered {
            bytes: [0; 512],
            len: 0,
        }
    }
}

impl Buffered {
    fn as_str(&self) -> &str {
        let bytes = self.bytes.get(..self.len).unwrap_or_default();
        core::str::from_utf8(bytes).unwrap_or_default()
    }
}

impl Write for Buffered {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        let end = self.len.checked_add(s.len()).ok_or(fmt::Error)?;
        let slot = self.bytes.get_mut(self.len..end).ok_or(fmt::Error)?;
        slot.copy_from_slice(s.as_bytes());
        self.len = end;
        Ok(())
    }
}

/// A ratio's prefix: nothing, `%`, a per mille, ppm, ppb.
fn ratio(n: Notation, p: Prefix, f: &mut dyn Write, lead: bool) -> Result<(), Stop> {
    let text = match (n, p) {
        (_, Prefix::Unit) => return Ok(()),
        (Notation::Typst, Prefix::Centi) => "\"%\"",
        (_, Prefix::Centi) => "%",
        (Notation::Ascii, Prefix::Milli) => "permille",
        (Notation::Ucum, Prefix::Milli) => "[ppth]",
        (Notation::Typst, Prefix::Milli) => "\"‰\"",
        (Notation::Ucum, Prefix::Micro) => "[ppm]",
        (Notation::Typst, Prefix::Micro) => "\"ppm\"",
        (_, Prefix::Micro) => "ppm",
        (Notation::Ucum, Prefix::Nano) => "[ppb]",
        (Notation::Typst, Prefix::Nano) => "\"ppb\"",
        (_, Prefix::Nano) => "ppb",
        _ => return Err(Stop::NoForm),
    };
    if lead {
        f.write_char(' ')?;
    }
    Ok(f.write_str(text)?)
}

/// A prefix's symbol in a notation: micro as `u` in ASCII and UCUM.
fn prefix_text(n: Notation, p: Prefix) -> &'static str {
    match (n, p) {
        (Notation::Ascii | Notation::Ucum, Prefix::Micro) => "u",
        _ => p.symbol(),
    }
}

/// Needs parentheses next to `·` or `/`: [tree]'s rule.
fn is_compound<E>(node: &Node<'_, E>) -> bool {
    match node {
        Node::Named { symbol, .. } => symbol.contains(['/', '·']),
        Node::Pow(..) | Node::Sqrt(..) => false,
        _ => true,
    }
}

/// The tree in a notation other than Unicode, `prefix` on its leaf when it is one named unit.
#[expect(
    clippy::arithmetic_side_effects,
    reason = "depth < MAX_UNIT_DEPTH, checked first"
)]
fn write_tree<'t, E: Tree<'t>>(
    n: Notation,
    e: &E,
    prefix: Option<Prefix>,
    f: &mut dyn Write,
    depth: usize,
) -> Result<(), Stop> {
    if depth >= MAX_UNIT_DEPTH {
        return Err(Stop::NoForm);
    }
    let node = e.node().ok_or(Stop::NoForm)?;
    let side = |f: &mut dyn Write, e: &E, parens: &dyn Fn(&Node<'_, E>) -> bool| {
        let parens = e.node().is_some_and(|node| parens(&node));
        if parens {
            f.write_char('(')?;
        }
        write_tree(n, e, None, f, depth + 1)?;
        if parens {
            f.write_char(')')?;
        }
        Ok::<(), Stop>(())
    };
    match &node {
        Node::Named { symbol, names, .. } => leaf(n, symbol, names.as_ref(), prefix, f),
        Node::Mul(a, b) => {
            let parens = |node: &Node<'_, E>| is_compound(node) && !matches!(node, Node::Mul(..));
            side(f, a, &parens)?;
            f.write_str(match n {
                Notation::Ucum => ".",
                Notation::Typst => " dot.op ",
                _ => "*",
            })?;
            side(f, b, &parens)
        }
        Node::Div(a, b) => {
            side(f, a, &|node| matches!(node, Node::Div(..)))?;
            f.write_char('/')?;
            side(f, b, &is_compound)
        }
        Node::Pow(a, k) => {
            side(f, a, &|node| {
                is_compound(node) || matches!(node, Node::Pow(..))
            })?;
            match n {
                Notation::Ucum => write!(f, "{k}")?,
                Notation::Typst if *k < 0 => write!(f, "^({k})")?,
                _ => write!(f, "^{k}")?,
            }
            Ok(())
        }
        Node::Sqrt(a) => {
            if n == Notation::Ucum {
                return Err(Stop::NoForm);
            }
            f.write_str("sqrt(")?;
            write_tree(n, a, None, f, depth + 1)?;
            Ok(f.write_char(')')?)
        }
    }
}

/// A named unit in a notation other than Unicode.
fn leaf(
    n: Notation,
    symbol: &str,
    names: Option<&UnitNames<'_>>,
    prefix: Option<Prefix>,
    f: &mut dyn Write,
) -> Result<(), Stop> {
    let p = prefix.map_or("", |p| prefix_text(n, p));
    match n {
        Notation::Ucum => {
            let code = names.and_then(|names| names.ucum).ok_or(Stop::NoForm)?;
            f.write_str(p)?;
            Ok(f.write_str(code)?)
        }
        Notation::Typst => typst_leaf(p, symbol, f),
        _ => {
            f.write_str(p)?;
            let written = match names {
                Some(names) => names.write_ascii(&mut Adapter(f)),
                None => crate::names::write_ascii(symbol, &mut Adapter(f)),
            };
            Ok(written.ok_or(Stop::NoForm)??)
        }
    }
}

/// `dyn Write` as a sized `Write`, for the writers that take `impl Write`.
struct Adapter<'f, 'w>(&'f mut (dyn Write + 'w));

impl Write for Adapter<'_, '_> {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.0.write_str(s)
    }
}

/// A named unit's symbol in Typst math: each run of letters upright (one letter bare, more in quotes, so Typst reads
/// no variable), `·` as `dot.op`, superscripts as `^`, the rest as it is: `upright("kg") dot.op upright(m)^2`.
fn typst_leaf(prefix: &str, symbol: &str, f: &mut dyn Write) -> Result<(), Stop> {
    let mut word = Buffered::default();
    word.write_str(prefix)?;
    let mut sup = Buffered::default();
    let flush_word = |word: &mut Buffered, f: &mut dyn Write| -> Result<(), Stop> {
        let w = word.as_str();
        let mut chars = w.chars();
        match (chars.next(), chars.next()) {
            (None, _) => return Ok(()),
            (Some(c), None) if c.is_alphabetic() => write!(f, "upright({c})")?,
            _ => {
                f.write_str("upright(\"")?;
                for c in w.chars() {
                    if matches!(c, '"' | '\\') {
                        f.write_char('\\')?;
                    }
                    f.write_char(c)?;
                }
                f.write_str("\")")?;
            }
        }
        word.len = 0;
        Ok(())
    };
    let flush_sup = |sup: &mut Buffered, f: &mut dyn Write| -> Result<(), Stop> {
        match sup.as_str() {
            "" => {}
            s if s.len() == 1 => write!(f, "^{s}")?,
            s => write!(f, "^({s})")?,
        }
        sup.len = 0;
        Ok(())
    };
    for c in symbol.chars() {
        let digit = match c {
            '⁻' => Some('-'),
            '⁰' => Some('0'),
            '¹' => Some('1'),
            '²' => Some('2'),
            '³' => Some('3'),
            '⁴' => Some('4'),
            '⁵' => Some('5'),
            '⁶' => Some('6'),
            '⁷' => Some('7'),
            '⁸' => Some('8'),
            '⁹' => Some('9'),
            _ => None,
        };
        if let Some(d) = digit {
            flush_word(&mut word, f)?;
            sup.write_char(d)?;
            continue;
        }
        flush_sup(&mut sup, f)?;
        match c {
            '·' | '⋅' => {
                flush_word(&mut word, f)?;
                f.write_str(" dot.op ")?;
            }
            '/' | '(' | ')' => {
                flush_word(&mut word, f)?;
                f.write_char(c)?;
            }
            ' ' => {
                flush_word(&mut word, f)?;
                f.write_str(" ")?;
            }
            c => word.write_char(c)?,
        }
    }
    flush_word(&mut word, f)?;
    flush_sup(&mut sup, f)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::string::String;

    fn typst(symbol: &str) -> String {
        let mut out = String::new();
        assert!(typst_leaf("", symbol, &mut out).is_ok());
        out
    }

    #[test]
    fn typst_leaves() {
        assert_eq!(typst("m/s"), "upright(m)/upright(s)");
        assert_eq!(
            typst("kg·m²/s"),
            "upright(\"kg\") dot.op upright(m)^2/upright(s)"
        );
        assert_eq!(typst("m⁻¹"), "upright(m)^(-1)");
        assert_eq!(typst("Ω"), "upright(Ω)");
        assert_eq!(
            typst("J/(kg·K)"),
            "upright(J)/(upright(\"kg\") dot.op upright(K))"
        );
    }
}
