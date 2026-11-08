//! Ranges and tolerances of `q!`: two literals as the ends of a range, a nominal with a tolerance worked out into
//! the two ends, or a tolerance alone.
//!
//! ```text
//! input     := body (',' FIT)?            (FIT: Inside or Outside)
//! body      := part | part ('..=' | '..') part | part '+-' part | part '+' part '-' part | '+-' part | STRING
//! part      := a literal of `literal`, or a number alone that takes the unit written last
//! ```
//!
//! A string holds the whole body, `±` written as itself: `q!("5 V ± 1 %")`. Outside a string Rust's lexer refuses
//! `±` before any macro sees it, so the tokens spell it `+-`.
//!
//! The ends of a tolerance are worked out here, exactly, as decimals: `5 V +- 1 %` is 4.95 V and 5.05 V. An amount
//! in `%`, `‰`, `ppm` or `ppb` is of the nominal (of its magnitude); any other amount is in the nominal's unit, up to
//! its prefixes (`5 V +- 50 mV`).

use super::grammar::{At, Literal, Unit, flatten, number_and_unit};
use super::model::{Error, Tok};
use super::strlex;

/// What a range does with an end that is no stored value, as the literal names it.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(crate) enum Fit {
    /// Nothing named: such an end is a compile error.
    Exact,
    /// Each such end moves inward, to the nearest stored value within the range.
    Inside,
    /// Each such end moves outward, to the nearest stored value past it.
    Outside,
}

impl Fit {
    /// The number the library's `LiteralRange` takes for it.
    pub(crate) fn code(self) -> u8 {
        match self {
            Fit::Exact => 0,
            Fit::Inside => 1,
            Fit::Outside => 2,
        }
    }
}

/// What `q!` reads.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Input {
    /// One literal, `q!(5 V)`.
    One(Literal),
    /// A range, `q!(4.9 V ..= 5.1 V)`, or a nominal with a tolerance, worked out into its ends.
    Range {
        /// The lower end, always in.
        lo: Literal,
        /// The upper end.
        hi: Literal,
        /// Whether the upper end is out: `..` rather than `..=`.
        open: bool,
        /// What an end that is no stored value does.
        fit: Fit,
    },
    /// A tolerance alone, `q!(+- 50 mV)`.
    Tolerance {
        /// The amount, at least 0.
        amount: Literal,
        /// What an amount that is no whole number of steps does.
        fit: Fit,
    },
}

/// `q!`'s input, or the refusal and the token it points at. `whole` is the id of the macro's own call.
pub(crate) fn input(toks: &[Tok], whole: usize) -> Result<Input, Error> {
    let toks = flatten(toks);
    let (body, fit, fit_id) = split_fit(&toks, whole)?;
    let (lexed, in_string) = match body {
        [Tok::Lit { text, id }] => match strlex::content(text) {
            Some(s) => (strlex::lex(s.map_err(|m| Error::at(*id, m))?, *id)?, true),
            None => (body.to_vec(), false),
        },
        _ => (body.to_vec(), false),
    };
    let body = lexed.as_slice();
    let whole_id = body.first().map_or(whole, Tok::id);
    let texts = Texts::of(body);
    let read = Read {
        body,
        whole: whole_id,
        in_string,
        fit,
        texts: &texts,
    };
    if let Some((k, open, len)) = range_op(body) {
        return read.range(k, open, len);
    }
    if let Some(k) = top_plus(body) {
        return read.tolerance(k);
    }
    if fit != Fit::Exact {
        return Err(Error::at(
            fit_id,
            "`Inside` and `Outside` are for a range's ends: q!(4.91 V ..= 4.99 V, Inside); a literal alone has no ends to move",
        ));
    }
    number_and_unit(body, whole_id, in_string).map(Input::One)
}

/// What a body is read with: its tokens, the id an error without a token of its own points at, whether it came from
/// a string, the fit written after it, and the tokens' texts.
struct Read<'a> {
    body: &'a [Tok],
    whole: usize,
    in_string: bool,
    fit: Fit,
    texts: &'a Texts,
}

impl Read<'_> {
    /// `lo ..= hi` or `lo .. hi`, the operator at `k`, `len` tokens long.
    fn range(&self, k: usize, open: bool, len: usize) -> Result<Input, Error> {
        let (lo, hi) = (
            self.body.get(..k).unwrap_or_default(),
            self.body.get(k.saturating_add(len)..).unwrap_or_default(),
        );
        let op = self.body.get(k).map_or(self.whole, Tok::id);
        if lo.is_empty() {
            return Err(Error::at(
                op,
                "a range has a lower end: q!(4.9 V ..= 5.1 V); for an upper bound alone write v <= q!(5.1 V)",
            ));
        }
        if hi.is_empty() {
            return Err(Error::at(
                op,
                "a range has an upper end: q!(4.9 V ..= 5.1 V); for a lower bound alone write v >= q!(4.9 V)",
            ));
        }
        let (lo, hi) = borrow_unit(&[lo, hi])
            .into_iter()
            .fold((None, None), |(a, _), p| match a {
                None => (Some(p), None),
                a => (a, Some(p)),
            });
        let (lo, hi) = (lo.unwrap_or_default(), hi.unwrap_or_default());
        let lo = number_and_unit(&lo, self.whole, self.in_string)?;
        let hi = number_and_unit(&hi, self.whole, self.in_string)?;
        same_kind(&lo, &hi, op, self.texts)?;
        if same_unit(&lo, &hi, self.texts) {
            match compare(&lo, &hi) {
                core::cmp::Ordering::Greater => {
                    return Err(Error::at(
                        op,
                        "the upper end is below the lower one: write the lower end first, q!(4.9 V ..= 5.1 V)",
                    ));
                }
                core::cmp::Ordering::Equal if open => {
                    return Err(Error::at(
                        op,
                        "a range `a .. a` holds nothing: write `..=` for a itself, or v == q!(..)",
                    ));
                }
                _ => {}
            }
        }
        Ok(Input::Range {
            lo,
            hi,
            open,
            fit: self.fit,
        })
    }

    /// A tolerance, its `+` at `k`: alone (`+- 50 mV`), symmetric (`5 V +- 1 %`) or one amount each way (`5 V +1 %
    /// -2 %`).
    fn tolerance(&self, k: usize) -> Result<Input, Error> {
        let plus = self.body.get(k).map_or(self.whole, Tok::id);
        let minus_next = self.body.get(k.saturating_add(1)).and_then(Tok::punct) == Some('-');
        if k == 0 {
            if !minus_next {
                return Err(Error::at(
                    plus,
                    "a tolerance alone is `+-` and its amount: q!(+- 50 mV)",
                ));
            }
            let amount = self.body.get(2..).unwrap_or_default();
            if amount.is_empty() {
                return Err(Error::at(
                    plus,
                    "a tolerance has an amount after `+-`: q!(+- 50 mV), q!(+- 1 %)",
                ));
            }
            let amount = number_and_unit(amount, plus, self.in_string)?;
            if amount.at.is_some() {
                return Err(Error::at(
                    plus,
                    "a tolerance is an amount, not a point: q!(+- 0.5 K), never at an origin",
                ));
            }
            nonnegative(&amount, plus)?;
            return Ok(Input::Tolerance {
                amount,
                fit: self.fit,
            });
        }
        let nominal = self.body.get(..k).unwrap_or_default();
        let (parts, sym) = if minus_next {
            (
                vec![
                    nominal,
                    self.body.get(k.saturating_add(2)..).unwrap_or_default(),
                ],
                true,
            )
        } else {
            let rest = self.body.get(k.saturating_add(1)..).unwrap_or_default();
            let Some(j) = split_minus(rest) else {
                return Err(Error::at(
                    plus,
                    "a tolerance is `+-` and its amount (q!(5 V +- 1 %)), or `+` and `-` with one each (q!(5 V +1 % -2 %))",
                ));
            };
            (
                vec![
                    nominal,
                    rest.get(..j).unwrap_or_default(),
                    rest.get(j.saturating_add(1)..).unwrap_or_default(),
                ],
                false,
            )
        };
        if parts.iter().skip(1).any(|p| p.is_empty()) {
            return Err(Error::at(
                plus,
                "a tolerance has an amount after `+-`: q!(5 V +- 1 %)",
            ));
        }
        let parts = borrow_tolerance(&parts);
        let nominal = number_and_unit(
            parts.first().map_or(&[][..], Vec::as_slice),
            self.whole,
            self.in_string,
        )?;
        let up = number_and_unit(
            parts.get(1).map_or(&[][..], Vec::as_slice),
            plus,
            self.in_string,
        )?;
        let down = if sym {
            up.clone()
        } else {
            number_and_unit(
                parts.get(2).map_or(&[][..], Vec::as_slice),
                plus,
                self.in_string,
            )?
        };
        let lo = end(&nominal, &down, false, plus, self.texts)?;
        let hi = end(&nominal, &up, true, plus, self.texts)?;
        Ok(Input::Range {
            lo,
            hi,
            open: false,
            fit: self.fit,
        })
    }
}

/// The body before a trailing `, Inside` or `, Outside`, the fit and the id of its word.
fn split_fit(toks: &[Tok], whole: usize) -> Result<(&[Tok], Fit, usize), Error> {
    // a `,` inside a type's `<..>` (`Per<Volt, Second>`) is the type's
    let mut depth = 0_usize;
    let at = toks.iter().position(|t| {
        match t.punct() {
            Some('<') => depth = depth.saturating_add(1),
            Some('>') => depth = depth.saturating_sub(1),
            Some(',') => return depth == 0,
            _ => {}
        }
        false
    });
    let Some(k) = at else {
        return Ok((toks, Fit::Exact, whole));
    };
    let comma = toks.get(k).map_or(whole, Tok::id);
    let body = toks.get(..k).unwrap_or_default();
    match toks.get(k.saturating_add(1)..).unwrap_or_default() {
        [Tok::Ident { text, id }] if text == "Inside" => Ok((body, Fit::Inside, *id)),
        [Tok::Ident { text, id }] if text == "Outside" => Ok((body, Fit::Outside, *id)),
        [t, ..] => Err(Error::at(
            t.id(),
            "after the `,` of a range comes `Inside` (each inexact end moves inward) or `Outside` (outward)",
        )),
        [] => Err(Error::at(
            comma,
            "after the `,` of a range comes `Inside` (each inexact end moves inward) or `Outside` (outward)",
        )),
    }
}

/// The position of a top-level `..=` or `..`, whether it is `..` (open), and its length in tokens.
fn range_op(toks: &[Tok]) -> Option<(usize, bool, usize)> {
    toks.windows(2).enumerate().find_map(|(k, w)| match w {
        [
            Tok::Punct {
                ch: '.',
                joint: true,
                ..
            },
            Tok::Punct { ch: '.', joint, .. },
        ] => {
            if *joint && toks.get(k.saturating_add(2)).and_then(Tok::punct) == Some('=') {
                Some((k, false, 3))
            } else {
                Some((k, true, 2))
            }
        }
        _ => None,
    })
}

/// The position of the first top-level `+`.
fn top_plus(toks: &[Tok]) -> Option<usize> {
    toks.iter().position(|t| t.punct() == Some('+'))
}

/// The `-` that splits `a - b` of an asymmetric tolerance: not the first token (a sign) and not after `^` (a
/// power, `m^-1`).
fn split_minus(toks: &[Tok]) -> Option<usize> {
    (1..toks.len()).find(|&j| {
        toks.get(j).and_then(Tok::punct) == Some('-')
            && toks.get(j.saturating_sub(1)).and_then(Tok::punct) != Some('^')
    })
}

/// Whether `part` is a number alone, with its sign: `5`, `-40`.
fn bare(part: &[Tok]) -> bool {
    match part {
        [Tok::Lit { text, .. }] => strlex::content(text).is_none(),
        [s, Tok::Lit { text, .. }] => s.punct() == Some('-') && strlex::content(text).is_none(),
        _ => false,
    }
}

/// The tokens of `part` after its number: its unit (and origin).
fn unit_of(part: &[Tok]) -> &[Tok] {
    let skip = usize::from(part.first().and_then(Tok::punct) == Some('-')).saturating_add(1);
    part.get(skip..).unwrap_or_default()
}

/// The parts of a range with the unit written once: a number alone takes the unit of the last part, `q!(4.9 ..= 5.1
/// V)`, `q!(-40 ..= 85 degC)`.
fn borrow_unit(parts: &[&[Tok]]) -> Vec<Vec<Tok>> {
    let last = parts.last().copied().unwrap_or_default();
    parts
        .iter()
        .map(|p| {
            let mut v = p.to_vec();
            if bare(p) && !bare(last) {
                v.extend_from_slice(unit_of(last));
            }
            v
        })
        .collect()
}

/// The parts of a tolerance with the unit written once: a nominal alone takes the unit of the last amount (`q!(5
/// +- 0.05 V)`), an amount alone the nominal's (`q!(5 V +- 0.05)`, `q!(25 degC +- 0.5)`).
fn borrow_tolerance(parts: &[&[Tok]]) -> Vec<Vec<Tok>> {
    let nominal = parts.first().copied().unwrap_or_default();
    let last = parts.last().copied().unwrap_or_default();
    parts
        .iter()
        .enumerate()
        .map(|(i, p)| {
            let mut v = p.to_vec();
            if bare(p) {
                if i == 0 && !bare(last) {
                    v.extend_from_slice(unit_of(last));
                } else if i > 0 && !bare(nominal) {
                    v.extend_from_slice(unit_of(nominal));
                }
            }
            v
        })
        .collect()
}

/// The text of each token, by id, for comparing two units written as types.
struct Texts(Vec<(usize, String)>);

impl Texts {
    fn of(toks: &[Tok]) -> Texts {
        let mut v = Vec::new();
        fn walk(toks: &[Tok], v: &mut Vec<(usize, String)>) {
            for t in toks {
                match t {
                    Tok::Ident { text, id } | Tok::Lit { text, id } => v.push((*id, text.clone())),
                    Tok::Punct { ch, id, .. } => v.push((*id, ch.to_string())),
                    Tok::Group { inner, .. } => walk(inner, v),
                }
            }
        }
        walk(toks, &mut v);
        Texts(v)
    }

    fn get(&self, id: usize) -> &str {
        self.0
            .iter()
            .find(|(i, _)| *i == id)
            .map_or("", |(_, t)| t.as_str())
    }

    fn same(&self, a: &[usize], b: &[usize]) -> bool {
        a.len() == b.len() && a.iter().zip(b).all(|(x, y)| self.get(*x) == self.get(*y))
    }
}

/// Whether two units are the same up to their prefixes: `V` and `mV`, `m/s` and `km/s`.
fn unit_eq(a: &Unit, b: &Unit, texts: &Texts) -> bool {
    match (a, b) {
        (Unit::Marker { path: p, .. }, Unit::Marker { path: q, .. })
        | (Unit::Level { marker: p, .. }, Unit::Level { marker: q, .. }) => p == q,
        (Unit::Type(x), Unit::Type(y)) => texts.same(x, y),
        (Unit::Mul(a1, b1), Unit::Mul(a2, b2)) | (Unit::Div(a1, b1), Unit::Div(a2, b2)) => {
            unit_eq(a1, a2, texts) && unit_eq(b1, b2, texts)
        }
        (Unit::Pow(x, n), Unit::Pow(y, m)) => n == m && unit_eq(x, y, texts),
        (Unit::One, Unit::One) | (Unit::Steps, Unit::Steps) => true,
        _ => false,
    }
}

fn at_eq(a: Option<&At>, b: Option<&At>, texts: &Texts) -> bool {
    match (a, b) {
        (None, None) => true,
        (Some(At::Symbol { path: p, .. }), Some(At::Symbol { path: q, .. })) => p == q,
        (Some(At::Type(x)), Some(At::Type(y))) => texts.same(x, y),
        _ => false,
    }
}

/// Whether two literals are in one unit at one origin, so their numbers compare as decimals.
fn same_unit(a: &Literal, b: &Literal, texts: &Texts) -> bool {
    unit_eq(&a.unit, &b.unit, texts) && at_eq(a.at.as_ref(), b.at.as_ref(), texts)
}

/// Both ends amounts, both points, or both levels.
fn same_kind(lo: &Literal, hi: &Literal, op: usize, texts: &Texts) -> Result<(), Error> {
    let level = |l: &Literal| matches!(l.unit, Unit::Level { .. });
    if level(lo) != level(hi) {
        return Err(Error::at(
            op,
            "a range of levels has a level at both ends: q!(-3 dB ..= 3 dB)",
        ));
    }
    match (&lo.at, &hi.at) {
        (None, None) => Ok(()),
        (Some(_), Some(_)) if at_eq(lo.at.as_ref(), hi.at.as_ref(), texts) => Ok(()),
        (Some(_), Some(_)) => Err(Error::at(
            op,
            "both ends of a range of points are at one origin: q!(20 degC ..= 25 degC)",
        )),
        _ => Err(Error::at(
            op,
            "a range has two amounts or two points at one origin, not one of each: q!(20 degC ..= 25 degC)",
        )),
    }
}

/// `a · 10^e` as an exact `i128` pair.
#[derive(Copy, Clone)]
struct Dec {
    num: i128,
    exp: i32,
}

impl Dec {
    fn of(l: &Literal) -> Dec {
        Dec {
            num: i128::from(l.num),
            exp: i32::from(l.exp),
        }
    }

    /// Both at the smaller exponent, `None` where that doesn't fit.
    fn align(a: Dec, b: Dec) -> Option<(i128, i128, i32)> {
        let e = a.exp.min(b.exp);
        let up = |d: Dec| -> Option<i128> {
            let k = u32::try_from(d.exp.checked_sub(e)?).ok()?;
            d.num.checked_mul(10_i128.checked_pow(k)?)
        };
        Some((up(a)?, up(b)?, e))
    }
}

/// `a` against `b`, both in one unit. Exponents too far apart to align order by the larger one's sign alone.
fn compare(a: &Literal, b: &Literal) -> core::cmp::Ordering {
    if let Some((x, y, _)) = Dec::align(Dec::of(a), Dec::of(b)) {
        return x.cmp(&y);
    }
    if a.num == 0 || b.num == 0 {
        return a.num.signum().cmp(&b.num.signum());
    }
    // one is past 10^38 of the other's step: the sign of the larger one decides
    if a.exp > b.exp {
        a.num.signum().cmp(&0)
    } else {
        0.cmp(&b.num.signum())
    }
}

/// Whether `l` is in `%`, `‰`, `ppm` or `ppb`: a part of the nominal.
fn relative(l: &Literal) -> bool {
    l.at.is_none()
        && matches!(
            l.unit,
            Unit::Marker {
                path: "units::Ratio",
                ..
            }
        )
}

fn nonnegative(amount: &Literal, id: usize) -> Result<(), Error> {
    if amount.num < 0 {
        return Err(Error::at(
            id,
            "a tolerance's amount is at least 0, its sign in `+` and `-`: q!(5 V +1 % -2 %)",
        ));
    }
    Ok(())
}

/// The end of `nominal` that `amount` makes: above it for `up`, below otherwise, exactly.
fn end(
    nominal: &Literal,
    amount: &Literal,
    up: bool,
    id: usize,
    texts: &Texts,
) -> Result<Literal, Error> {
    nonnegative(amount, id)?;
    let delta = if relative(amount) {
        if matches!(nominal.unit, Unit::Level { .. }) {
            return Err(Error::at(
                id,
                "a part of a level reads as nothing: a tolerance of a level is in dB, q!(20 dB +- 1 dB)",
            ));
        }
        if nominal.at.is_some() {
            return Err(Error::at(
                id,
                "a part of a point means nothing (1 % of 25 °C depends on the origin): write the tolerance in the point's unit, q!(25 degC +- 0.5)",
            ));
        }
        if relative(nominal) {
            return Err(Error::at(
                id,
                "a part of a ratio reads two ways, of the nominal or as points of it (5 % +- 1 % is 4.95 % ..= 5.05 % or 4 % ..= 6 %): write the ends, q!(4 % ..= 6 %)",
            ));
        }
        if nominal.num == 0 {
            return Err(Error::at(
                id,
                "a part of a nominal of 0 is 0: write the tolerance in the nominal's unit",
            ));
        }
        Dec {
            num: i128::from(nominal.num)
                .abs()
                .checked_mul(i128::from(amount.num))
                .ok_or_else(|| {
                    Error::at(id, "the tolerance's end has more digits than an i64 holds")
                })?,
            exp: i32::from(nominal.exp).saturating_add(i32::from(amount.exp)),
        }
    } else {
        if !same_unit(nominal, amount, texts) {
            return Err(Error::at(
                id,
                "a tolerance is in the nominal's unit (5 V +- 50 mV) or a part of it (5 V +- 1 %): for other units write the ends, q!(4.95 V ..= 5.05 V)",
            ));
        }
        Dec::of(amount)
    };
    let digits = || {
        Error::at(
            id,
            "the tolerance's end has more digits than an i64 holds: write the ends",
        )
    };
    let (n, d, e) = Dec::align(Dec::of(nominal), delta).ok_or_else(digits)?;
    let num = if up {
        n.checked_add(d)
    } else {
        n.checked_sub(d)
    }
    .ok_or_else(digits)?;
    let (mut num, mut e) = (num, e);
    if num == 0 {
        e = 0;
    }
    while num != 0 && num % 10 == 0 {
        num /= 10;
        e = e.saturating_add(1);
    }
    Ok(Literal {
        num: i64::try_from(num).map_err(|_| digits())?,
        exp: i16::try_from(e)
            .map_err(|_| Error::at(id, "the tolerance's end has a power of ten out of range"))?,
        unit: nominal.unit.clone(),
        at: nominal.at.clone(),
    })
}
