//! [Registry]: units read from text through their [UnitNames], over tables of named units, without
//! a global and without alloc.

use crate::exact::{number, scale};
use crate::{
    Dim, Exact, Fraction, Magnitude, Prefix, QuantityExpr, Unit, UnitDef, UnitExpr, UnitNames,
};
use core::fmt::{self, Write as _};

/// Most factors a unit read from text has: `kg·m²/(s³·A)` is four.
pub const MAX_FACTORS: usize = 8;

/// Most readings one word of a unit keeps: `B` is a bel and a byte.
const MAX_READINGS: usize = 4;

/// Most combinations of readings tried against an expected dimension before the text is refused as ambiguous.
const MAX_COMBINATIONS: usize = 256;

/// Deepest nesting of parentheses read.
const MAX_NESTING: usize = 4;

/// The named units text can name: tables of [UnitExpr]s, none of them a global. `Registry::new(&[units::ALL,
/// my_api::UNITS])` reads this crate's units and an API crate's, each table a `unit! { @table static UNITS; .. }`.
/// It builds in `const` and on the stack, and reads untrusted text without a panic.
///
/// A unit is read by any of its names: its symbol (`Ω`), its ASCII form (`Ohm`), its aliases, its UCUM code, and its
/// English name, singular or plural (`ohm`, `ohms`), each with a prefix where the unit takes it
/// ([PrefixPolicy](crate::PrefixPolicy)): `kΩ`, `kOhm`, `kilohm`. The longest match comes first: `Pa·s` is a unit of
/// its own before it is a pascal times a second. Then the text splits into factors, joined by `·`, `*`, `.` or a space
/// and divided by `/` or `per`, left to right (`a/b·c` is `a·c/b`), with exponents as `²`, `^2` or UCUM's `m2`, and
/// parentheses.
///
/// A word that reads two ways (`B`, a bel and a byte) is refused naming both, unless the expected dimension
/// settles it: text that goes into a [Fixed](crate::Fixed) of bytes reads a byte.
///
/// ```
/// # #[cfg(all(feature = "electrical", feature = "time", feature = "mechanical", feature = "information", feature = "levels"))] {
/// use exact_units::units::{self, Byte, Minute, Ohm, Second};
/// use exact_units::{Exact, Fixed, Registry, prefix::{Kilo, One}};
///
/// let tables = [units::ALL];
/// let r = Registry::new(&tables);
/// for text in ["4.7 kOhm", "4.7 kΩ", "4.7 kilohm", "4700 ohms"] {
///     let x: Exact<Ohm> = r.parse(text).unwrap();
///     assert_eq!(x.to::<One, u16>(), Some(Fixed::new(4700)), "{text}");
///     // 4.7 isn't a whole number of kΩ
///     assert_eq!(x.to::<Kilo, u16>(), None);
/// }
/// assert_eq!(r.parse::<Second>("5 min").unwrap(), Exact::int(300));
/// assert_eq!(r.parse::<Byte>("5 B").unwrap(), Exact::int(5));
/// let err = r.parse_unit("B", None).unwrap_err();
/// assert_eq!(err.to_string(), "`B` reads as a bel and as a byte: name the quantity it goes into");
/// # }
/// ```
#[derive(Copy, Clone, Debug)]
pub struct Registry<'a> {
    tables: &'a [&'a [&'a UnitExpr]],
}

/// A named unit text names, with the prefix it was written with.
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Reading<'a> {
    /// The unit.
    pub def: &'a UnitDef<'a>,
    /// Its prefix, [Prefix::Unit] for none.
    pub prefix: Prefix,
}

impl Reading<'_> {
    /// The factor to coherent SI, prefix and all.
    fn scale(&self) -> Option<Magnitude> {
        self.def.scale.checked_mul(self.prefix.magnitude()?).ok()
    }

    /// The same value as `other`: dimension, scale and quantity (`km` as a kilometre and as kilo metre).
    fn same_value(&self, other: &Reading<'_>) -> bool {
        self.def.dim.same(other.def.dim)
            && self.scale().is_some()
            && self.scale() == other.scale()
            && self.def.quantity == other.def.quantity
    }
}

/// `kilo ohm`: the prefix's name and the unit's English name (its symbol where it has none).
impl fmt::Display for Reading<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.prefix != Prefix::Unit {
            for c in self.prefix.name().chars() {
                f.write_char(c.to_ascii_lowercase())?;
            }
        }
        let names = self.def.names;
        let name = if names.long.is_empty() {
            names.symbol
        } else {
            names.long
        };
        f.write_str(name)
    }
}

/// One factor of a unit read from text: a named unit, its prefix and its exponent.
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Factor<'a> {
    /// The unit and its prefix.
    pub reading: Reading<'a>,
    /// The exponent: `-1` for `s` in `m/s`.
    pub exp: i8,
}

/// A unit read from text: up to [MAX_FACTORS] named units with prefixes and exponents. `{}` writes it with `·` and
/// `/`, `km/s²`.
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct ParsedUnit<'a> {
    factors: [Option<Factor<'a>>; MAX_FACTORS],
}

impl<'a> ParsedUnit<'a> {
    /// The factors, in the order of the text.
    pub fn factors(&self) -> impl Iterator<Item = Factor<'a>> + '_ {
        self.factors.iter().flatten().copied()
    }

    /// The dimension, its distinguished bases read from the factors on demand (so it borrows the parse); `None`
    /// where an exponent leaves what a [Dim] holds.
    #[must_use]
    pub fn dim(&self) -> Option<Dim<'_>> {
        let mut exps = [0i32; crate::MAX_TRUE_BASES];
        let mut system: Option<(crate::SystemRef<'_>, u8)> = None;
        for f in self.factors() {
            let d = f.reading.def.dim;
            if d.has_true_exps() {
                match system {
                    Some((s, _)) if !s.same(&d.system()) => return None,
                    _ => system = Some(d.counted_system()),
                }
            }
            for (e, x) in exps.iter_mut().zip(d.exps()) {
                *e = e.checked_add(x.wide().checked_mul(i32::from(f.exp))?)?;
            }
        }
        Dim::of_source(
            system.unwrap_or_else(|| Dim::ONE.counted_system()),
            exps,
            self,
        )
    }

    /// The exact factor to coherent SI, prefixes and all; `None` past what a [Magnitude] holds.
    #[must_use]
    pub fn scale(&self) -> Option<Magnitude> {
        self.factors().try_fold(Magnitude::ONE, |m, f| {
            m.checked_mul(f.reading.scale()?.checked_powi(f.exp).ok()?)
                .ok()
        })
    }

    /// The quantity: a named unit's own (`Ω` is a resistance), its dimension's for a product or a power
    /// ([QuantityExpr::Derived]); `None` where the dimension isn't whole ([dim](Self::dim)).
    #[must_use]
    pub fn quantity(&self) -> Option<QuantityExpr<'_>> {
        let mut it = self.factors();
        match (it.next(), it.next()) {
            (Some(f), None) if f.exp == 1 => Some(f.reading.def.quantity_expr()),
            _ => self.dim().map(QuantityExpr::Derived),
        }
    }
}

impl fmt::Display for ParsedUnit<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let alone = self.factors().count() == 1;
        let write = |f: &mut fmt::Formatter<'_>, factor: &Factor<'_>, exp: i8| {
            let symbol = factor.reading.def.names.symbol;
            // a compound symbol in parentheses where it isn't the whole unit: `(cd/m²)²`, `m/(J/(kg·K))`
            let parens = !(crate::names::is_atom(symbol) || alone && exp == 1);
            let (open, close) = if parens { ("(", ")") } else { ("", "") };
            write!(f, "{open}{}{symbol}{close}", factor.reading.prefix.symbol())?;
            if exp != 1 {
                crate::unit::write_superscript(f, i32::from(exp))?;
            }
            Ok(())
        };
        if self.factors().all(|x| x.exp < 0) && self.factors().next().is_some() {
            // `s⁻¹`, `m⁻¹·s⁻¹`: nothing above the line
            let mut first = true;
            for factor in self.factors() {
                if !first {
                    f.write_str("·")?;
                }
                first = false;
                write(f, &factor, factor.exp)?;
            }
            return Ok(());
        }
        let mut first = true;
        for factor in self.factors().filter(|x| x.exp > 0) {
            if !first {
                f.write_str("·")?;
            }
            first = false;
            write(f, &factor, factor.exp)?;
        }
        if first {
            f.write_str("1")?;
        }
        let count = self.factors().filter(|x| x.exp < 0).count();
        let mut den = self.factors().filter(|x| x.exp < 0);
        if count > 0 {
            f.write_str("/")?;
            if count > 1 {
                f.write_str("(")?;
            }
            let mut first = true;
            for factor in &mut den {
                if !first {
                    f.write_str("·")?;
                }
                first = false;
                write(f, &factor, factor.exp.saturating_neg())?;
            }
            if count > 1 {
                f.write_str(")")?;
            }
        }
        Ok(())
    }
}

/// Why text isn't a unit or a quantity of the expected one.
#[derive(Copy, Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum ParseError<'a> {
    /// No unit where one was expected, or no number.
    Empty,
    /// Not a number at the start.
    Number,
    /// A word that names no unit of the registry, at these bytes of the text.
    Unknown {
        /// Its first byte.
        at: usize,
        /// Its length in bytes.
        len: usize,
    },
    /// Text that isn't a unit: an operator or a parenthesis out of place, at this byte.
    Syntax {
        /// The byte.
        at: usize,
    },
    /// A word that reads two ways, and nothing says which: the bytes of the word and two of its readings.
    Ambiguous {
        /// Its first byte.
        at: usize,
        /// Its length in bytes.
        len: usize,
        /// One reading.
        a: Reading<'a>,
        /// Another.
        b: Reading<'a>,
    },
    /// More than [MAX_FACTORS] factors, deeper parentheses than four, or an exponent past `i8`.
    TooComplex,
    /// A unit, but of another dimension or quantity than the one expected: [Registry::parse_unit] without one says
    /// which.
    Mismatch,
    /// The value isn't exact in the expected unit: a scale with π or a root, or past a fraction of `u128`s.
    Inexact,
}

impl fmt::Display for ParseError<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ParseError::Empty => f.write_str("no unit"),
            ParseError::Number => f.write_str("not a number"),
            ParseError::Unknown { at, len } => {
                write!(f, "no unit named by the {len} bytes at {at}")
            }
            ParseError::Syntax { at } => write!(f, "not a unit at byte {at}"),
            ParseError::Ambiguous { a, b, .. } => {
                let (p, s) = (a.prefix.symbol(), a.def.names.symbol);
                write!(
                    f,
                    "`{p}{s}` reads as a {a} and as a {b}: name the quantity it goes into"
                )
            }
            ParseError::TooComplex => write!(
                f,
                "a unit of more than {MAX_FACTORS} factors, {MAX_NESTING} levels of parentheses or a large exponent"
            ),
            ParseError::Mismatch => f.write_str("a unit of another quantity"),
            ParseError::Inexact => f.write_str("not exact in the unit"),
        }
    }
}

impl core::error::Error for ParseError<'_> {}

/// A word's readings, folded where they are the same value.
#[derive(Copy, Clone)]
struct Readings<'a> {
    list: [Option<Reading<'a>>; MAX_READINGS],
    at: usize,
    len: usize,
}

impl<'a> Readings<'a> {
    fn new(at: usize, len: usize) -> Self {
        Readings {
            list: [None; MAX_READINGS],
            at,
            len,
        }
    }

    fn count(&self) -> usize {
        self.list.iter().flatten().count()
    }

    fn get(&self, i: usize) -> Option<Reading<'a>> {
        self.list.get(i).copied().flatten()
    }

    /// Adds `r` unless a reading of the same value is in; one without a prefix replaces a prefixed one.
    fn add(&mut self, r: Reading<'a>) {
        for slot in &mut self.list {
            match slot {
                Some(old) if old.same_value(&r) || (old.def == r.def && old.prefix == r.prefix) => {
                    if old.prefix != Prefix::Unit && r.prefix == Prefix::Unit {
                        *old = r;
                    }
                    return;
                }
                Some(_) => {}
                None => {
                    *slot = Some(r);
                    return;
                }
            }
        }
    }
}

/// One factor of the text before its readings are chosen.
#[derive(Copy, Clone)]
struct Term<'a> {
    readings: Readings<'a>,
    exp: i8,
}

/// The terms of a unit's text.
struct Terms<'a> {
    list: [Option<Term<'a>>; MAX_FACTORS],
    len: usize,
}

impl<'a> Terms<'a> {
    /// Adds `t`; an exponent of 0 (`m^0`, `h0`) names no unit and is refused.
    fn push(&mut self, t: Term<'a>) -> Result<(), ParseError<'a>> {
        if t.exp == 0 {
            return Err(ParseError::Syntax { at: t.readings.at });
        }
        let slot = self.list.get_mut(self.len).ok_or(ParseError::TooComplex)?;
        *slot = Some(t);
        self.len = self.len.checked_add(1).ok_or(ParseError::TooComplex)?;
        Ok(())
    }
}

impl<'a> Registry<'a> {
    /// The units of `tables`.
    #[must_use]
    pub const fn new(tables: &'a [&'a [&'a UnitExpr]]) -> Self {
        Registry { tables }
    }

    /// Every named unit of the registry.
    fn units(&self) -> impl Iterator<Item = &'a UnitDef<'a>> + 'a {
        self.tables
            .iter()
            .flat_map(|t| t.iter())
            .filter_map(|e| e.def())
            .filter(|d| !d.names.symbol.is_empty())
    }

    /// The unit `text` names, settled by `expected` where a word reads two ways. Without an expected dimension an
    /// ambiguous word is an error that names two of its readings; with one, a unit of another dimension is a
    /// [ParseError::Mismatch].
    pub fn parse_unit(
        &self,
        text: &str,
        expected: Option<Dim<'_>>,
    ) -> Result<ParsedUnit<'a>, ParseError<'a>> {
        self.read(
            text,
            &|u: &ParsedUnit<'_>| expected.is_none_or(|e| u.dim().is_some_and(|d| d.same(e))),
            expected.is_some(),
        )
    }

    /// The unit `text` names, of the dimension `dim` going into the quantity `quantity`:
    /// [parse_unit](Self::parse_unit) where two units of one dimension are different quantities (`Hz` and `Bq`).
    /// A named unit is of its own quantity, which must [convert](QuantityExpr::converts) to `quantity`; a product or
    /// a power names only its dimension, and goes into any quantity of it (`1/s` into an activity).
    pub fn parse_unit_of(
        &self,
        text: &str,
        dim: Dim<'_>,
        quantity: QuantityExpr<'_>,
    ) -> Result<ParsedUnit<'a>, ParseError<'a>> {
        self.read(
            text,
            &|u: &ParsedUnit<'_>| u.dim().is_some_and(|d| d.same(dim)) && goes_into(u, quantity),
            true,
        )
    }

    /// The quantity `text` writes, a number then its unit (`4.7 kΩ`, `5 min`, `1.5e-3 Pa·s`), as an exact value of
    /// `U`: read with `U`'s dimension and quantity expected, then converted exactly. A unit of another dimension or
    /// quantity
    /// is a [ParseError::Mismatch], a value not exact in `U` (a scale with π, a fraction past `u128`)
    /// [ParseError::Inexact]. Then [Exact::to] gives a [Fixed](crate::Fixed), exact or `None`.
    pub fn parse<U: Unit>(&self, text: &str) -> Result<Exact<U>, ParseError<'a>> {
        let text = text.trim();
        let (x, e, rest) = number(text).map_err(|_| ParseError::Number)?;
        let offset = text.len().saturating_sub(rest.len());
        let unit_text = rest.trim_start();
        let offset = offset.saturating_add(rest.len().saturating_sub(unit_text.len()));
        let accept = |u: &ParsedUnit<'_>| {
            u.dim().is_some_and(|d| d.same(U::DIM)) && goes_into(u, U::QUANTITY)
        };
        let unit = if unit_text.is_empty() {
            ParsedUnit {
                factors: [None; MAX_FACTORS],
            }
        } else {
            self.read(unit_text, &accept, true)
                .map_err(|err| shift(err, offset))?
        };
        if !accept(&unit) {
            return Err(ParseError::Mismatch);
        }
        let own = crate::tree::magnitude(&U::EXPR).ok_or(ParseError::Inexact)?;
        let k = unit
            .scale()
            .and_then(|s| s.checked_div(own).ok())
            .ok_or(ParseError::Inexact)?;
        let (p10x2, p2x2) = k.pow10_pow2_x2();
        if p10x2 % 2 != 0 || p2x2 % 2 != 0 {
            return Err(ParseError::Inexact);
        }
        let (p10, p2) = (p10x2 / 2, p2x2 / 2);
        let rest_k = Magnitude::checked_pow10(p10)
            .and_then(|t| {
                Magnitude::checked_pow2(p2)
                    .ok()
                    .and_then(|w| t.checked_mul(w).ok())
            })
            .and_then(|tw| k.checked_div(tw).ok())
            .and_then(|r| Fraction::from_magnitude(&r))
            .ok_or(ParseError::Inexact)?;
        let v = x
            .checked_mul(rest_k)
            .and_then(|x| scale(x, i32::from(p2), e.checked_add(i32::from(p10))?))
            .ok_or(ParseError::Inexact)?;
        Ok(Exact::of(v))
    }

    /// The unit of `text`, the readings `accept` lets through; `expected` says an expectation filters them.
    fn read(
        &self,
        text: &str,
        accept: &dyn Fn(&ParsedUnit<'_>) -> bool,
        expected: bool,
    ) -> Result<ParsedUnit<'a>, ParseError<'a>> {
        let text = text.trim();
        if text.is_empty() {
            return Err(ParseError::Empty);
        }
        // the longest match first: the whole text as one unit (`Pa·s`, `mi² (US survey)`)
        let whole = self.word(text, 0);
        if whole.count() > 0 {
            let terms = Terms {
                list: {
                    let mut l = [None; MAX_FACTORS];
                    if let Some(slot) = l.first_mut() {
                        *slot = Some(Term {
                            readings: whole,
                            exp: 1,
                        });
                    }
                    l
                },
                len: 1,
            };
            match choose(&terms, accept, expected) {
                Err(ParseError::Mismatch) if expected => {}
                r => return r,
            }
        }
        let mut terms = Terms {
            list: [None; MAX_FACTORS],
            len: 0,
        };
        let mut p = Parser {
            registry: self,
            text,
            at: 0,
        };
        p.expr(&mut terms, 1, 0)?;
        p.skip_spaces();
        if p.at < text.len() {
            return Err(ParseError::Syntax { at: p.at });
        }
        if terms.len == 0 {
            return Err(ParseError::Empty);
        }
        choose(&terms, accept, expected)
    }

    /// The readings of one word, at byte `at` of the text.
    fn word(&self, w: &str, at: usize) -> Readings<'a> {
        let mut out = Readings::new(at, w.len());
        for def in self.units() {
            read_word(def, w, &mut out);
        }
        out
    }
}

/// The distinguished bases of the factors' units, each to the factor's power.
impl crate::BaseSource for ParsedUnit<'_> {
    fn exp_of(&self, base: crate::MarkerId<'_>) -> Option<i32> {
        self.factors().try_fold(0i32, |sum, f| {
            sum.checked_add(
                f.reading
                    .def
                    .dim
                    .exp_of(base)
                    .wide()
                    .checked_mul(i32::from(f.exp))?,
            )
        })
    }

    fn each_base<'s>(&'s self, f: &mut dyn FnMut(crate::MarkerId<'s>)) -> bool {
        for factor in self.factors.iter().flatten() {
            factor.reading.def.dim.for_each_base(|b| f(b.base));
        }
        true
    }
}

/// Whether a unit read goes into the quantity `want`: its own quantity converts to it, or it names only a dimension
/// (a product, a power), which the caller's quantity names.
fn goes_into(found: &ParsedUnit<'_>, want: QuantityExpr<'_>) -> bool {
    match found.quantity() {
        Some(QuantityExpr::Derived(_)) => true,
        Some(q) => q.converts(want, &()).unwrap_or(false),
        None => false,
    }
}

/// `err` with its byte positions moved by `by`: they count from the unit, the caller's from the whole text.
fn shift(err: ParseError<'_>, by: usize) -> ParseError<'_> {
    match err {
        ParseError::Unknown { at, len } => ParseError::Unknown {
            at: at.saturating_add(by),
            len,
        },
        ParseError::Syntax { at } => ParseError::Syntax {
            at: at.saturating_add(by),
        },
        ParseError::Ambiguous { at, len, a, b } => ParseError::Ambiguous {
            at: at.saturating_add(by),
            len,
            a,
            b,
        },
        e => e,
    }
}

/// The readings of `def` the word `w` has: by any of its names, with a prefix the unit takes.
fn read_word<'a>(def: &'a UnitDef<'a>, w: &str, out: &mut Readings<'a>) {
    let names: &UnitNames<'a> = &def.names;
    let symbols = [Some(names.symbol), names.ascii, names.ucum];
    let take = |out: &mut Readings<'a>, prefix: Prefix| out.add(Reading { def, prefix });
    // as a symbol, an alias or a code: with a prefix's symbol
    let by_symbol = |rest: &str| {
        symbols.iter().flatten().any(|s| *s == rest)
            || names.aliases().any(|a| a == rest)
            || (names.ascii.is_none() && crate::text::writes_as(Ascii(names.symbol), rest))
    };
    if by_symbol(w) {
        take(out, Prefix::Unit);
    }
    for p in prefixes(names.prefixes) {
        let sym = p.symbol();
        let rest = w.strip_prefix(sym).or_else(|| {
            (p == Prefix::Micro)
                .then(|| w.strip_prefix('u').or_else(|| w.strip_prefix('\u{3bc}')))
                .flatten()
        });
        if rest.is_some_and(|r| !r.is_empty() && by_symbol(r)) {
            take(out, p);
        }
    }
    // as an English name, any case: with a prefix's name, `kilohm` for kilo-ohm as the SI writes it
    if names.long.is_empty() {
        return;
    }
    let by_name = |rest: &str| {
        rest.eq_ignore_ascii_case(names.long) || rest.eq_ignore_ascii_case(names.plural)
    };
    if by_name(w) {
        take(out, Prefix::Unit);
    }
    for p in prefixes(names.prefixes) {
        let name = p.name();
        let after = |head: &str| {
            w.get(..head.len())
                .is_some_and(|h| h.eq_ignore_ascii_case(head))
                && w.get(head.len()..).is_some_and(by_name)
        };
        // the prefix's last vowel dropped before the same vowel, as the SI writes `kilohm` and `megohm`
        let elided = name
            .chars()
            .last()
            .filter(|v| matches!(v, 'a' | 'o') && names.long.starts_with(*v))
            .and_then(|_| name.get(..name.len().saturating_sub(1)));
        if after(name) || elided.is_some_and(after) {
            take(out, p);
        }
    }
}

/// The prefixes a policy lets a unit take, without [Prefix::Unit].
fn prefixes(policy: crate::PrefixPolicy) -> impl Iterator<Item = Prefix> {
    (-30..=30i8)
        .filter_map(Prefix::si_from_pow10)
        .chain([Prefix::Kibi, Prefix::Mebi, Prefix::Gibi, Prefix::Tebi])
        .filter(|p| *p != Prefix::Unit)
        .filter(move |p| policy.allows(*p))
}

/// A symbol's transliteration, matched against text by [writes_as](crate::text::writes_as).
struct Ascii<'s>(&'s str);

impl fmt::Display for Ascii<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        crate::names::write_ascii(self.0, f).unwrap_or(Err(fmt::Error))
    }
}

/// The readings of the terms that `accept` lets through: the one combination, or an error naming two readings of the
/// first word that has more.
fn choose<'a>(
    terms: &Terms<'a>,
    accept: &dyn Fn(&ParsedUnit<'_>) -> bool,
    expected: bool,
) -> Result<ParsedUnit<'a>, ParseError<'a>> {
    let terms = terms.list.iter().flatten();
    let mut combinations = 1usize;
    let mut first_ambiguous = None;
    for t in terms.clone() {
        let n = t.readings.count();
        if n == 0 {
            return Err(ParseError::Unknown {
                at: t.readings.at,
                len: t.readings.len,
            });
        }
        if n > 1 && first_ambiguous.is_none() {
            first_ambiguous = Some(t.readings);
        }
        combinations = combinations.saturating_mul(n);
    }
    let ambiguous = |r: Readings<'a>| match (r.get(0), r.get(1)) {
        (Some(a), Some(b)) => ParseError::Ambiguous {
            at: r.at,
            len: r.len,
            a,
            b,
        },
        _ => ParseError::TooComplex,
    };
    if let Some(r) = first_ambiguous
        && (!expected || combinations > MAX_COMBINATIONS)
    {
        return Err(ambiguous(r));
    }
    let mut found: Option<ParsedUnit<'a>> = None;
    let mut mismatch = false;
    for mut k in 0..combinations {
        let mut unit = ParsedUnit {
            factors: [None; MAX_FACTORS],
        };
        for (slot, t) in unit.factors.iter_mut().zip(terms.clone()) {
            let n = t.readings.count();
            let i = k.checked_rem(n).ok_or(ParseError::TooComplex)?;
            let reading = t.readings.get(i).ok_or(ParseError::TooComplex)?;
            k = k.checked_div(n).ok_or(ParseError::TooComplex)?;
            *slot = Some(Factor {
                reading,
                exp: t.exp,
            });
        }
        if !accept(&unit) {
            mismatch = true;
            continue;
        }
        match found {
            None => found = Some(unit),
            Some(f)
                if f.scale().is_some()
                    && f.scale() == unit.scale()
                    && f.quantity() == unit.quantity() => {}
            Some(_) => return Err(first_ambiguous.map_or(ParseError::TooComplex, ambiguous)),
        }
    }
    match found {
        Some(u) => Ok(u),
        None if mismatch => Err(ParseError::Mismatch),
        None => Err(ParseError::Empty),
    }
}

/// A recursive-descent reader of a unit's text: factors joined by `·`, `*`, `.` or a space, divided by `/`, with
/// exponents and parentheses.
struct Parser<'r, 'a, 't> {
    registry: &'r Registry<'a>,
    text: &'t str,
    at: usize,
}

impl<'a, 't> Parser<'_, 'a, 't> {
    fn rest(&self) -> &str {
        self.text.get(self.at..).unwrap_or_default()
    }

    fn peek(&self) -> Option<char> {
        self.rest().chars().next()
    }

    fn bump(&mut self, c: char) {
        self.at = self.at.saturating_add(c.len_utf8());
    }

    fn skip_spaces(&mut self) -> bool {
        let before = self.at;
        while let Some(c @ ' ') = self.peek() {
            self.bump(c);
        }
        self.at != before
    }

    /// Terms joined by `·`, `*`, `.`, a space and `/`, each with `sign` (-1 inside a denominator) times its own.
    fn expr(
        &mut self,
        terms: &mut Terms<'a>,
        sign: i8,
        depth: usize,
    ) -> Result<(), ParseError<'a>> {
        if depth > MAX_NESTING {
            return Err(ParseError::TooComplex);
        }
        self.skip_spaces();
        // `/ms` and `1/ms`: a unit that starts with a quotient
        if self
            .rest()
            .strip_prefix('1')
            .is_some_and(|r| r.trim_start().starts_with('/'))
        {
            self.bump('1');
            self.skip_spaces();
        }
        let mut op = if self.peek() == Some('/') {
            self.bump('/');
            -1
        } else {
            1
        };
        loop {
            self.skip_spaces();
            self.factor(
                terms,
                sign.checked_mul(op).ok_or(ParseError::TooComplex)?,
                depth,
            )?;
            let spaced = self.skip_spaces();
            // `per` between English names: `kilometres per hour`
            if spaced
                && self
                    .rest()
                    .strip_prefix("per")
                    .is_some_and(|r| r.starts_with(' '))
            {
                self.at = self.at.saturating_add(3);
                op = -1;
                continue;
            }
            op = match self.peek() {
                Some(c @ ('·' | '⋅' | '*' | '.')) => {
                    self.bump(c);
                    1
                }
                Some('/') => {
                    self.bump('/');
                    -1
                }
                None | Some(')') => return Ok(()),
                Some(_) if spaced => 1,
                Some(_) => return Err(ParseError::Syntax { at: self.at }),
            };
        }
    }

    /// A word or a parenthesised group, then its exponent.
    fn factor(
        &mut self,
        terms: &mut Terms<'a>,
        sign: i8,
        depth: usize,
    ) -> Result<(), ParseError<'a>> {
        if self.peek() == Some('(') {
            // the longest match first, as for the whole text: `(cd/m²)²` is a nit squared
            if let Some(inner) = self.group() {
                let start = self.at.saturating_add(1);
                let readings = self.registry.word(inner, start);
                if readings.count() > 0 {
                    self.at = start.saturating_add(inner.len()).saturating_add(1);
                    let exp = self.exponent()?.unwrap_or(1);
                    return terms.push(Term {
                        readings,
                        exp: exp.checked_mul(sign).ok_or(ParseError::TooComplex)?,
                    });
                }
            }
            self.bump('(');
            let from = terms.len;
            self.expr(terms, 1, depth.saturating_add(1))?;
            self.skip_spaces();
            if self.peek() != Some(')') {
                return Err(ParseError::Syntax { at: self.at });
            }
            self.bump(')');
            let exp = self.exponent()?.unwrap_or(1);
            let k = exp.checked_mul(sign).ok_or(ParseError::TooComplex)?;
            if k == 0 {
                return Err(ParseError::Syntax { at: self.at });
            }
            for t in terms.list.iter_mut().skip(from).flatten() {
                t.exp = t.exp.checked_mul(k).ok_or(ParseError::TooComplex)?;
            }
            return Ok(());
        }
        let start = self.at;
        while let Some(c) = self.peek() {
            if matches!(c, ' ' | '·' | '⋅' | '*' | '.' | '/' | '(' | ')' | '^')
                || superscript(c).is_some()
            {
                break;
            }
            self.bump(c);
        }
        let word = self.text.get(start..self.at).unwrap_or_default();
        if word.is_empty() {
            return Err(ParseError::Syntax { at: self.at });
        }
        let exp = self.exponent()?;
        let mut readings = self.registry.word(word, start);
        let mut exp = exp.unwrap_or(1);
        // UCUM's exponent: digits at the end of the word (`m2`, `s-1`)
        if readings.count() == 0 {
            let digits = word.trim_end_matches(|c: char| c.is_ascii_digit());
            let base = digits.strip_suffix('-').unwrap_or(digits);
            if digits.len() < word.len() && !base.is_empty() {
                let n: i8 = word
                    .get(base.len()..)
                    .and_then(|e| e.parse().ok())
                    .ok_or(ParseError::TooComplex)?;
                let found = self.registry.word(base, start);
                if found.count() > 0 {
                    readings = Readings {
                        len: word.len(),
                        ..found
                    };
                    exp = exp.checked_mul(n).ok_or(ParseError::TooComplex)?;
                }
            }
        }
        terms.push(Term {
            readings,
            exp: exp.checked_mul(sign).ok_or(ParseError::TooComplex)?,
        })
    }

    /// The text inside the parentheses that open here, up to the one that closes them; `None` where none does.
    fn group(&self) -> Option<&'t str> {
        let rest = self.text.get(self.at..)?;
        let mut depth = 0usize;
        for (i, c) in rest.char_indices() {
            match c {
                '(' => depth = depth.checked_add(1)?,
                ')' => {
                    depth = depth.checked_sub(1)?;
                    if depth == 0 {
                        return rest.get(1..i);
                    }
                }
                _ => {}
            }
        }
        None
    }

    /// `^2`, `^-1`, `^(-1)` or `²`, `⁻¹`: `None` for none.
    fn exponent(&mut self) -> Result<Option<i8>, ParseError<'a>> {
        if self.peek() == Some('^') {
            self.bump('^');
            let paren = self.peek() == Some('(');
            if paren {
                self.bump('(');
            }
            let start = self.at;
            if let Some(c @ ('-' | '+')) = self.peek() {
                self.bump(c);
            }
            while let Some(c) = self.peek().filter(char::is_ascii_digit) {
                self.bump(c);
            }
            let n = self
                .text
                .get(start..self.at)
                .and_then(|e| e.parse::<i8>().ok());
            let n = n.ok_or(ParseError::Syntax { at: start })?;
            if paren {
                if self.peek() != Some(')') {
                    return Err(ParseError::Syntax { at: self.at });
                }
                self.bump(')');
            }
            return Ok(Some(n));
        }
        // the digits are added with the sign, so -128 reads as i8's own
        let mut n: Option<i8> = None;
        let mut neg = false;
        while let Some(c) = self.peek() {
            match superscript(c) {
                Some(Sup::Minus) if n.is_none() && !neg => neg = true,
                Some(Sup::Digit(d)) => {
                    let d = if neg { d.saturating_neg() } else { d };
                    let v = n.unwrap_or(0);
                    n = Some(
                        v.checked_mul(10)
                            .and_then(|v| v.checked_add(d))
                            .ok_or(ParseError::TooComplex)?,
                    );
                }
                _ => break,
            }
            self.bump(c);
        }
        match (n, neg) {
            (Some(n), _) => Ok(Some(n)),
            (None, true) => Err(ParseError::Syntax { at: self.at }),
            (None, false) => Ok(None),
        }
    }
}

/// A superscript character in an exponent.
#[derive(Copy, Clone)]
enum Sup {
    Minus,
    Digit(i8),
}

/// A superscript digit or minus, `None` for any other character.
fn superscript(c: char) -> Option<Sup> {
    Some(Sup::Digit(match c {
        '⁻' => return Some(Sup::Minus),
        '⁰' => 0,
        '¹' => 1,
        '²' => 2,
        '³' => 3,
        '⁴' => 4,
        '⁵' => 5,
        '⁶' => 6,
        '⁷' => 7,
        '⁸' => 8,
        '⁹' => 9,
        _ => return None,
    }))
}
