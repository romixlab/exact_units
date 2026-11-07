//! The grammar of `q!`: a number and a unit.
//!
//! ```text
//! literal := '-'? NUMBER (unit | ORIGIN | LEVEL | unit? '@' TYPE) | STRING   (the whole literal in a string)
//! unit    := (term | '/' term) (('*' | '/') term)*   (left to right: m/s/s is (m/s)/s; /ms is 1/ms)
//! term    := atom ('^' '-'? INTEGER)?
//! atom    := SYMBOL | TYPE | '(' unit ')' | '%' | STRING   (a unit in a string)
//! ```
//!
//! A symbol is looked up in the table, with a prefix split off where the unit takes one; the prefix goes into the
//! number's exponent, so `mV` is `Volt` and `10^-3` more. A type is a path with `::` or `<..>` (`units::Speed`,
//! `Per<Volt, Second>`) or a capitalised identifier the table doesn't know (`Speed`).
//!
//! A point literal is a number at an origin: an origin symbol alone after the number (`25 degC`, `25 "°C"`, the
//! number in the origin's own step), or `@` and the origin's type after a unit (`300 K @ AbsoluteZero`) or after the
//! number alone (`77 @ Fahrenheit`, again in the origin's step).
//!
//! A level literal is `dB` or `Np` alone after the number (`-3 dB`, `-3 "dB"`): a level of a gain, whose kind picks
//! the form. Inside a unit the two are refused, as they read two ways there (a level of gain or of loss).

use super::model::{Delim, Error, Tok};
use super::number::{self, Decimal};
use super::prefix::Scale;
use super::resolve::{Symbol, level, resolve};
use super::strlex;

/// A unit as written, prefixes taken out.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Unit {
    /// A marker from the table.
    Marker {
        /// Its path under exact_units' root, `units::Volt`.
        path: &'static str,
        /// Its feature group, `electrical`.
        group: &'static str,
        /// The symbol's token.
        id: usize,
        /// The power of ten of its prefix and its own (`%` is -2 of `Ratio`).
        pow10: i64,
        /// The power of 1024 of its binary prefix.
        pow1024: i64,
    },
    /// A type the user wrote, as the ids of its tokens.
    Type(Vec<usize>),
    /// `a * b`
    Mul(Box<Unit>, Box<Unit>),
    /// `a / b`
    Div(Box<Unit>, Box<Unit>),
    /// `a ^ n`, `n` not 0.
    Pow(Box<Unit>, i32),
    /// The unit 1, before a leading `/`: `q!(5 /ms)`.
    One,
    /// The step of a point literal's origin, for a number without a unit: `q!(25 degC)`, `q!(77 @ Fahrenheit)`.
    Steps,
    /// A level of a gain, `dB` or `Np` alone after the number: `q!(-3 dB)`.
    Level {
        /// The level marker's path under exact_units' root, `literal::Db`.
        marker: &'static str,
        /// The symbol's token.
        id: usize,
    },
}

/// The origin of a point literal.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum At {
    /// An origin symbol of the table, `degC`.
    Symbol {
        /// The origin's path under exact_units' root, `origins::Celsius`.
        path: &'static str,
        /// Its feature group, `thermal`.
        group: &'static str,
        /// The symbol's token.
        id: usize,
    },
    /// The type after `@`, as the ids of its tokens.
    Type(Vec<usize>),
}

/// A literal: `num · 10^exp` of `unit`, its prefixes folded into `num` and `exp`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Literal {
    /// The digits, with the sign.
    pub(crate) num: i64,
    /// The power of ten.
    pub(crate) exp: i16,
    /// The unit.
    pub(crate) unit: Unit,
    /// The origin of a point literal, `None` for an amount.
    pub(crate) at: Option<At>,
}

/// The largest power `^n` takes.
const MAX_POW: i32 = 9;

/// `q!`'s input to a literal, or the refusal and the token it points at. `whole` is the id of the macro's own
/// call, for an input that has no token to point at (`q!()`).
pub(crate) fn literal(toks: &[Tok], whole: usize) -> Result<Literal, Error> {
    let toks = flatten(toks);
    if let [Tok::Lit { text, id }] = toks.as_slice()
        && let Some(s) = strlex::content(text)
    {
        let s = s.map_err(|m| Error::at(*id, m))?;
        let lexed = strlex::lex(s, *id)?;
        return number_and_unit(&lexed, *id, true);
    }
    number_and_unit(&toks, whole, false)
}

/// `[tok]` with the invisible groups of `macro_rules!` fragments opened.
fn flatten(toks: &[Tok]) -> Vec<Tok> {
    let mut out = Vec::new();
    for t in toks {
        match t {
            Tok::Group {
                delim: Delim::None,
                inner,
                ..
            } => out.extend(flatten(inner)),
            t => out.push(t.clone()),
        }
    }
    out
}

fn number_and_unit(toks: &[Tok], whole: usize, in_string: bool) -> Result<Literal, Error> {
    let (negative, rest) = match toks.split_first() {
        Some((t, rest)) if t.punct() == Some('-') => (true, rest),
        _ => (false, toks),
    };
    let (num, num_id, rest) = match rest.split_first() {
        Some((Tok::Lit { text, id }, rest)) if strlex::content(text).is_none() => (
            number::parse(text, negative).map_err(|m| Error::at(*id, m))?,
            *id,
            rest,
        ),
        Some((t, _)) => {
            return Err(Error::at(
                t.id(),
                "a literal starts with a decimal number: q!(100 m/s)",
            ));
        }
        None => {
            return Err(Error::at(
                whole,
                "a literal is a number and its unit: q!(100 m/s)",
            ));
        }
    };
    if rest.is_empty() {
        return Err(Error::at(
            toks.last().map_or(whole, Tok::id),
            "a unit is missing after the number: q!(100 m/s), q!(5 %), q!(0.5 Ratio); fixed!(..) is a number in the unit of its context",
        ));
    }
    if let Some(k) = rest.iter().position(|t| t.punct() == Some('@')) {
        return point(rest, k, num, num_id, in_string);
    }
    if let Some(at) = origin_symbol(rest, in_string)? {
        return fold(num, Unit::Steps, num_id, Some(at));
    }
    if let Some(unit) = level_symbol(rest, in_string)? {
        return fold(num, unit, num_id, None);
    }
    let mut p = Parser {
        toks: rest,
        at: 0,
        in_string,
    };
    let unit = p.unit()?;
    if let Some(t) = p.peek() {
        return Err(Error::at(
            t.id(),
            "a unit is symbols joined by `*` and `/`, with powers `^2`, or a type",
        ));
    }
    fold(num, unit, num_id, None)
}

/// `rest` as one origin symbol, `degC` or `"°C"`, if it is one.
fn origin_symbol(rest: &[Tok], in_string: bool) -> Result<Option<At>, Error> {
    let (text, id) = match rest {
        [Tok::Ident { text, id }] => (text.clone(), *id),
        [Tok::Lit { text, id }] if !in_string => match strlex::content(text) {
            Some(s) => match strlex::lex(s.map_err(|m| Error::at(*id, m))?, *id)?.as_slice() {
                [Tok::Ident { text, .. }] => (text.clone(), *id),
                _ => return Ok(None),
            },
            None => return Ok(None),
        },
        _ => return Ok(None),
    };
    match resolve(&text) {
        Ok(Symbol::Origin(o)) => Ok(Some(At::Symbol {
            path: o.path,
            group: o.group,
            id,
        })),
        _ => Ok(None),
    }
}

/// `rest` as one level symbol, `dB` or `"Np"`, if it is one.
fn level_symbol(rest: &[Tok], in_string: bool) -> Result<Option<Unit>, Error> {
    let (text, id) = match rest {
        [Tok::Ident { text, id }] => (text.clone(), *id),
        [Tok::Lit { text, id }] if !in_string => match strlex::content(text) {
            Some(s) => match strlex::lex(s.map_err(|m| Error::at(*id, m))?, *id)?.as_slice() {
                [Tok::Ident { text, .. }] => (text.clone(), *id),
                _ => return Ok(None),
            },
            None => return Ok(None),
        },
        _ => return Ok(None),
    };
    Ok(level(&text).map(|l| Unit::Level {
        marker: l.marker,
        id,
    }))
}

/// A point literal with `@` at `rest[k]`: the unit before it (none: the origin's step), the origin's type after.
fn point(
    rest: &[Tok],
    k: usize,
    num: Decimal,
    num_id: usize,
    in_string: bool,
) -> Result<Literal, Error> {
    let (unit, after) = rest.split_at(k);
    let (amp, after) = match after.split_first() {
        Some((amp, after)) => (amp.id(), after),
        None => (num_id, after),
    };
    if in_string {
        return Err(Error::at(
            amp,
            "an origin is written after the string: q!(300 \"K\" @ AbsoluteZero)",
        ));
    }
    let unit = if unit.is_empty() {
        Unit::Steps
    } else {
        if let Some(At::Symbol { id, .. }) = origin_symbol(unit, in_string)? {
            return Err(Error::at(
                id,
                "a point literal has one origin: an origin symbol (q!(25 degC)) or `@` and a type (q!(298.15 K @ AbsoluteZero))",
            ));
        }
        let mut p = Parser {
            toks: unit,
            at: 0,
            in_string,
        };
        let u = p.unit()?;
        if let Some(t) = p.peek() {
            return Err(Error::at(
                t.id(),
                "a unit is symbols joined by `*` and `/`, with powers `^2`, or a type",
            ));
        }
        u
    };
    let origin_needed = || {
        Error::at(
            amp,
            "an origin's type follows `@`: q!(300 K @ AbsoluteZero), q!(220 kPa @ Gauge)",
        )
    };
    let mut p = Parser {
        toks: after,
        at: 0,
        in_string,
    };
    let starts_type = match after.first() {
        Some(Tok::Ident { .. }) => true,
        Some(Tok::Punct {
            ch: ':',
            joint: true,
            ..
        }) => after.get(1).and_then(Tok::punct) == Some(':'),
        _ => false,
    };
    if !starts_type {
        return Err(after.first().map_or_else(origin_needed, |t| {
            Error::at(
                t.id(),
                "an origin's type follows `@`: q!(300 K @ AbsoluteZero), q!(220 kPa @ Gauge)",
            )
        }));
    }
    let Unit::Type(ids) = p.type_path()? else {
        return Err(origin_needed());
    };
    if let Some(t) = p.peek() {
        return Err(Error::at(
            t.id(),
            "a point literal ends with its origin's type: q!(300 K @ AbsoluteZero)",
        ));
    }
    fold(num, unit, num_id, Some(At::Type(ids)))
}

/// The prefixes of `unit` folded into the number: `num · 10^exp` with each decimal prefix and the symbol's own power
/// of ten times its power in the unit, and `num` times 1024 per binary prefix.
fn fold(num: Decimal, unit: Unit, num_id: usize, at: Option<At>) -> Result<Literal, Error> {
    let mut pow10 = i64::from(num.exp);
    let mut pow1024 = 0_i64;
    let mut first = num_id;
    walk(&unit, 1, &mut |id, p10, p1024, times| {
        pow10 = pow10.saturating_add(p10.saturating_mul(times));
        pow1024 = pow1024.saturating_add(p1024.saturating_mul(times));
        if p1024 != 0 {
            first = id;
        }
    });
    let mut n = num.num;
    let exponent = || Error::at(num_id, "the literal's power of ten is out of range");
    let exp = i16::try_from(pow10).map_err(|_| exponent())?;
    if pow1024 < 0 {
        return Err(Error::at(
            first,
            "a binary prefix in a denominator isn't exact as a decimal: write the unit as a type",
        ));
    }
    for _ in 0..pow1024 {
        n = n
            .checked_mul(1024)
            .ok_or_else(|| Error::at(first, "the literal times its binary prefix is past i64"))?;
    }
    Ok(Literal {
        num: n,
        exp,
        unit,
        at,
    })
}

/// Calls `f(id, pow10, pow1024, times)` for every symbol of `unit` with its prefix, at power `times`.
fn walk(unit: &Unit, times: i64, f: &mut impl FnMut(usize, i64, i64, i64)) {
    match unit {
        Unit::Marker {
            id, pow10, pow1024, ..
        } => f(*id, *pow10, *pow1024, times),
        Unit::Type(_) | Unit::One | Unit::Steps | Unit::Level { .. } => {}
        Unit::Mul(a, b) => {
            walk(a, times, f);
            walk(b, times, f);
        }
        Unit::Div(a, b) => {
            walk(a, times, f);
            walk(b, times.saturating_neg(), f);
        }
        Unit::Pow(a, n) => walk(a, times.saturating_mul(i64::from(*n)), f),
    }
}

/// A recursive-descent parser over one level of tokens.
struct Parser<'a> {
    toks: &'a [Tok],
    at: usize,
    in_string: bool,
}

impl<'a> Parser<'a> {
    fn peek(&self) -> Option<&'a Tok> {
        self.toks.get(self.at)
    }

    fn peek_at(&self, ahead: usize) -> Option<&'a Tok> {
        self.toks.get(self.at.saturating_add(ahead))
    }

    fn next(&mut self) -> Option<&'a Tok> {
        let t = self.toks.get(self.at)?;
        self.at = self.at.saturating_add(1);
        Some(t)
    }

    fn unit(&mut self) -> Result<Unit, Error> {
        let mut u = if self.peek().and_then(Tok::punct) == Some('/') {
            Unit::One
        } else {
            self.term()?
        };
        while let Some(op) = self.peek().and_then(Tok::punct) {
            match op {
                '*' => {
                    self.next();
                    u = Unit::Mul(Box::new(u), Box::new(self.term()?));
                }
                '/' => {
                    self.next();
                    u = Unit::Div(Box::new(u), Box::new(self.term()?));
                }
                _ => break,
            }
        }
        Ok(u)
    }

    fn term(&mut self) -> Result<Unit, Error> {
        let a = self.atom()?;
        if self.peek().and_then(Tok::punct) != Some('^') {
            return Ok(a);
        }
        let caret = self.next().map_or(0, Tok::id);
        let negative = self.peek().and_then(Tok::punct) == Some('-');
        if negative {
            self.next();
        }
        let power = match self.next() {
            Some(Tok::Lit { text, id }) => text
                .parse::<i32>()
                .ok()
                .filter(|n| (1..=MAX_POW).contains(n))
                .ok_or_else(|| {
                    Error::at(
                        *id,
                        format!("a power is an integer from 1 to {MAX_POW}, or its negative"),
                    )
                })?,
            Some(t) => {
                return Err(Error::at(
                    t.id(),
                    "a power after `^` is an integer: m^2, s^-1",
                ));
            }
            None => {
                return Err(Error::at(
                    caret,
                    "a power after `^` is an integer: m^2, s^-1",
                ));
            }
        };
        Ok(Unit::Pow(
            Box::new(a),
            if negative {
                power.saturating_neg()
            } else {
                power
            },
        ))
    }

    fn atom(&mut self) -> Result<Unit, Error> {
        let Some(t) = self.next() else {
            let id = self.toks.last().map_or(0, Tok::id);
            return Err(Error::at(id, "a unit symbol or type is missing at the end"));
        };
        match t {
            Tok::Group {
                delim: Delim::Paren,
                inner,
                id,
            } => {
                let inner = flatten(inner);
                let mut p = Parser {
                    toks: &inner,
                    at: 0,
                    in_string: self.in_string,
                };
                if inner.is_empty() {
                    return Err(Error::at(*id, "an empty `( )` in a unit"));
                }
                let u = p.unit()?;
                match p.peek() {
                    Some(t) => Err(Error::at(
                        t.id(),
                        "a unit is symbols joined by `*` and `/`, with powers `^2`, or a type",
                    )),
                    None => Ok(u),
                }
            }
            Tok::Group { id, .. } => Err(Error::at(*id, "a unit groups with `( )` only")),
            Tok::Lit { text, id } => match strlex::content(text) {
                Some(s) if !self.in_string => {
                    let s = s.map_err(|m| Error::at(*id, m))?;
                    let lexed = strlex::lex(s, *id)?;
                    let mut p = Parser {
                        toks: &lexed,
                        at: 0,
                        in_string: true,
                    };
                    let u = p.unit()?;
                    match p.peek() {
                        Some(_) => Err(Error::at(
                            *id,
                            "a unit is symbols joined by `*` and `/`, with powers `^2`",
                        )),
                        None if lexed.is_empty() => Err(Error::at(*id, "an empty unit")),
                        None => Ok(u),
                    }
                }
                _ => Err(Error::at(
                    *id,
                    "a unit symbol or type was expected here, not a number",
                )),
            },
            Tok::Punct { ch: '%', id, .. } => Self::symbol("%", *id),
            Tok::Punct {
                ch: ':',
                joint: true,
                ..
            } if self.peek().and_then(Tok::punct) == Some(':') && !self.in_string => {
                self.at = self.at.saturating_sub(1);
                self.type_path()
            }
            Tok::Ident { text, id } => {
                let path_follows = matches!(self.peek(), Some(Tok::Punct { ch: '<', .. }))
                    || (matches!(
                        self.peek(),
                        Some(Tok::Punct {
                            ch: ':',
                            joint: true,
                            ..
                        })
                    ) && self.peek_at(1).and_then(Tok::punct) == Some(':'));
                if path_follows && !self.in_string {
                    self.at = self.at.saturating_sub(1);
                    return self.type_path();
                }
                match Self::symbol(text, *id) {
                    Err(e) if e.msg.is_empty() => {
                        if !self.in_string && text.chars().next().is_some_and(char::is_uppercase) {
                            Ok(Unit::Type(vec![*id]))
                        } else {
                            Err(Error::at(*id, unknown(text)))
                        }
                    }
                    r => r,
                }
            }
            Tok::Punct { id, ch, .. } => Err(Error::at(
                *id,
                format!("`{ch}`: a unit symbol or type was expected here"),
            )),
        }
    }

    /// A table symbol, or an error with an empty message for one the table doesn't know.
    fn symbol(text: &str, id: usize) -> Result<Unit, Error> {
        match resolve(text).map_err(|m| Error::at(id, m))? {
            Symbol::Unit { entry, prefix } => {
                let (p10, p1024) = match prefix.map(|p| p.scale) {
                    None => (0, 0),
                    Some(Scale::Pow10(e)) => (i64::from(e), 0),
                    Some(Scale::Pow1024(e)) => (0, i64::from(e)),
                };
                Ok(Unit::Marker {
                    path: entry.marker,
                    group: entry.group,
                    id,
                    pow10: p10.saturating_add(i64::from(entry.pow10)),
                    pow1024: p1024,
                })
            }
            Symbol::Origin(o) => Err(Error::at(
                id,
                format!(
                    "`{text}` is a point at the origin {}: it stands alone after the number, as in q!(25 {text}); a difference of temperatures is in K",
                    o.path
                ),
            )),
            Symbol::Unknown => Err(Error::at(id, "")),
        }
    }

    /// A type path: `::a::B`, `units::Speed`, `Per<Volt, Second>`, as its token ids.
    fn type_path(&mut self) -> Result<Unit, Error> {
        let mut ids = Vec::new();
        loop {
            if let (
                Some(Tok::Punct {
                    ch: ':',
                    joint: true,
                    id: a,
                }),
                Some(Tok::Punct { ch: ':', id: b, .. }),
            ) = (self.peek(), self.peek_at(1))
            {
                ids.push(*a);
                ids.push(*b);
                self.at = self.at.saturating_add(2);
            }
            match self.next() {
                Some(Tok::Ident { id, .. }) => ids.push(*id),
                Some(t) => return Err(Error::at(t.id(), "a type path names a type after `::`")),
                None => {
                    return Err(Error::at(
                        ids.last().copied().unwrap_or(0),
                        "a type path ends in `::`",
                    ));
                }
            }
            if matches!(self.peek(), Some(Tok::Punct { ch: '<', .. })) {
                let mut depth = 0_usize;
                while let Some(t) = self.next() {
                    ids.push(t.id());
                    match t.punct() {
                        Some('<') => depth = depth.saturating_add(1),
                        Some('>') => {
                            depth = depth.saturating_sub(1);
                            if depth == 0 {
                                break;
                            }
                        }
                        _ => {}
                    }
                }
                if depth != 0 {
                    return Err(Error::at(
                        ids.last().copied().unwrap_or(0),
                        "a `<` without its `>`",
                    ));
                }
                return Ok(Unit::Type(ids));
            }
            let more = matches!(
                self.peek(),
                Some(Tok::Punct {
                    ch: ':',
                    joint: true,
                    ..
                })
            ) && self.peek_at(1).and_then(Tok::punct) == Some(':');
            if !more {
                return Ok(Unit::Type(ids));
            }
        }
    }
}

/// The message for a symbol nobody knows.
fn unknown(text: &str) -> String {
    format!(
        "`{text}` is no unit symbol q! knows: write a symbol (m/s, km/h, mV), its type (Speed, units::Volt), or a string for one Rust's lexer can't take (\"°\")"
    )
}
