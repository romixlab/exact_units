//! [Uncertain]: a value with its standard uncertainty, both exact, as data tables print them.

use super::text::Kind;
use super::{Exact, ExactNum, Fraction, auto_prefix_of, prefix_for, value_in};
use crate::text::unit::{UnitText, prefix_of};
use crate::tree::write_unit;
use crate::{Prefix, Rounding, Unit};
use core::fmt;
use core::hash::{Hash, Hasher};
use core::marker::PhantomData;
use core::str::FromStr;

/// A value of `U` with its standard uncertainty u (GUM, JCGM 100 §2.3.1: an uncertainty as a standard deviation,
/// k = 1), both exact as a data table prints them: CODATA's `6.674 30(15)e-11`, NUBASE's half-life `1.600 ky` with
/// `0.007` in the next column, AME's mass excess `2345.67 ± 2.78 keV`.
///
/// Data, not a calculation: there is no arithmetic on an `Uncertain`. Operators that combine uncertainties as if
/// every operand were independent give `a − a` an uncertainty of √2·u instead of 0, so this type has none; the value
/// and its uncertainty are [value](Self::value) and [u](Self::u), and an exact change of unit is
/// [convert](Self::convert). [rel_u](Self::rel_u) is u/|value|, [expanded](Self::expanded) is k·u for a coverage
/// factor k the caller names: there is no default k.
///
/// Text, every form read exactly or refused:
///
/// - The concise form (GUM §7.2.2, as CODATA prints it), which `{}` writes and `parse` reads: `1600(7) y` is 1600
///   with u = 7 in the units of the value's last digit, `6.674 30(15)e-11 m³/(kg·s²)` is 6.674 30·10⁻¹¹ with u =
///   0.000 15·10⁻¹¹. Digits in parentheses that hold a point (or a repetend or a fraction) are in the value's unit
///   itself, the GUM's other form: `100.021 47(0.000 35) g`. Digits may be grouped by spaces (CODATA's thin spaces
///   too) in the value and inside the parentheses, never in an exponent; one exponent, after the parentheses or in
///   the value. The parentheses follow the value's last digit: `1600 (7)` is refused by name, and `1600 y
///   (NUBASE2020)` has no uncertainty.
/// - Columns, as AME and NUBASE print them: [from_columns](Self::from_columns), the value and its uncertainty as two
///   texts in one prefix of `U`.
/// - `±` with a coverage factor the caller names: [parse_pm](Self::parse_pm) and [pm](Self::pm). AME prints ± for
///   one standard deviation, the GUM keeps ± for an expanded uncertainty (§7.2.2), so `2.78` after a ± reads two ways
///   and the k that says which is never assumed.
///
/// Refused, each by its own error: a `#` (AME's and NUBASE's mark for a value estimated from systematic trends, not
/// measured, in place of the decimal point), a limit (`>300`, `<1`, `~5`), an asymmetric or signed uncertainty
/// (`(+4-3)`). `{}` never rounds: where the uncertainty's last digit is below the value's, the value is padded
/// (`1600.0(15)`, not `1600.0(1.5)`).
///
/// ```
/// # #[cfg(feature = "nuclear")] {
/// use exact_units::{Exact, Prefix, Uncertain, units::{ElectronVolt, TropicalYearNubase2020 as Y}};
///
/// // NUBASE2020: 226Ra's half-life, 1.600 ky in one column, 0.007 in the next
/// let t = Uncertain::<Y>::from_columns("1.600", "0.007", Prefix::Kilo).unwrap();
/// assert_eq!(*t.value(), Exact::int(1600));
/// assert_eq!(*t.u(), Exact::int(7));
/// assert_eq!(format!("{t:#}"), "1600(7) y (NUBASE2020)");
/// assert_eq!("1600(7) y (NUBASE2020)".parse(), Ok(t));
///
/// // an estimated value, # in place of the point: refused by name
/// let e = Uncertain::<ElectronVolt>::from_columns("-2160#", "300#", Prefix::Kilo).unwrap_err();
/// assert!(e.to_string().contains("estimated"));
/// # }
/// ```
///
/// `R` is the number both are held in, an [ExactNum], as for [Exact].
pub struct Uncertain<U, R: ExactNum = Fraction> {
    value: Exact<U, R>,
    u: Exact<U, R>,
}

impl<U, R: ExactNum> Uncertain<U, R> {
    /// `value` with the standard uncertainty `u`, `None` for a `u` below zero.
    #[must_use]
    pub fn new(value: Exact<U, R>, u: Exact<U, R>) -> Option<Self> {
        (!u.value.is_negative()).then_some(Self { value, u })
    }

    /// `value` known exactly: u = 0 (a constant the SI fixes, a count).
    #[must_use]
    pub fn exact(value: Exact<U, R>) -> Self {
        Self {
            value,
            u: Exact::of(R::zero()),
        }
    }

    /// The value, the best estimate.
    #[must_use]
    pub const fn value(&self) -> &Exact<U, R> {
        &self.value
    }

    /// The standard uncertainty u, at least 0.
    #[must_use]
    pub const fn u(&self) -> &Exact<U, R> {
        &self.u
    }

    /// The value and its standard uncertainty.
    #[must_use]
    pub fn into_parts(self) -> (Exact<U, R>, Exact<U, R>) {
        (self.value, self.u)
    }

    /// Whether u is 0.
    #[must_use]
    pub fn is_exact(&self) -> bool {
        self.u.value.is_zero()
    }

    /// The expanded uncertainty U = k·u (GUM §6.2.1) for the coverage factor `k` the caller names (2 for about 95 %
    /// of a normal distribution, by the GUM's own example): `None` for a `k` below 0 and where it doesn't fit.
    ///
    /// ```
    /// # #[cfg(feature = "time")] {
    /// use exact_units::{Fraction, Uncertain, units::Second};
    ///
    /// let t: Uncertain<Second> = "1600(7) s".parse().unwrap();
    /// assert_eq!(t.expanded(&Fraction::int(2)).unwrap().to_string(), "14 s");
    /// # }
    /// ```
    #[must_use]
    pub fn expanded(&self, k: &R) -> Option<Exact<U, R>> {
        if k.is_negative() {
            return None;
        }
        Some(Exact::of(self.u.value.checked_mul(k)?))
    }

    /// This value written with `±` and the expanded uncertainty k·u, k named by the caller: `(2345.67 ± 2.78) keV`
    /// for AME's k = 1. `None` for a `k` at or below 0 and where k·u doesn't fit. [parse_pm](Self::parse_pm) with the
    /// same k reads it back.
    #[must_use]
    pub fn pm(&self, k: &R) -> Option<PlusMinus<U, R>> {
        if k.is_negative() || k.is_zero() {
            return None;
        }
        Some(PlusMinus {
            value: self.value.clone(),
            expanded: self.expanded(k)?,
        })
    }
}

#[cfg(feature = "dimensionless")]
impl<U, R: ExactNum> Uncertain<U, R> {
    /// The relative standard uncertainty u/|value|, `None` for a value of 0 and where it doesn't fit.
    ///
    /// ```
    /// # #[cfg(feature = "time")] {
    /// use exact_units::{Exact, Uncertain, units::Second};
    ///
    /// let t: Uncertain<Second> = "1600(7) s".parse().unwrap();
    /// assert_eq!(t.rel_u(), Some(Exact::new(7, 1600)));
    /// assert_eq!(t.rel_u().unwrap().to_string(), "0.4375 %");
    /// # }
    /// ```
    #[must_use]
    pub fn rel_u(&self) -> Option<Exact<crate::units::Ratio, R>> {
        Some(Exact::of(
            self.u.value.checked_div(&self.value.value.abs())?,
        ))
    }
}

impl<U: Unit, R: ExactNum> Uncertain<U, R> {
    /// The same value and uncertainty in `U2`, both exactly: `None` where either doesn't fit. A factor that isn't a
    /// fraction (π, ln 2) or an offset between the units doesn't compile, as for [Exact::convert].
    #[must_use]
    pub fn convert<U2: Unit>(self) -> Option<Uncertain<U2, R>> {
        Some(Uncertain {
            value: self.value.convert()?,
            u: self.u.convert()?,
        })
    }

    /// The value and its standard uncertainty from two columns, as AME and NUBASE print them: each a number
    /// (`1.600`, `-2160.5`, `1_000`, `1e-3`), both in the prefix `prefix` of `U` (NUBASE's unit column, `ky` is
    /// [Prefix::Kilo] on a year). Refused, each by name: a `#` in either (an estimated value), a limit (`>`, `<`,
    /// `~`, `≥`, `≤`), a signed or asymmetric uncertainty, and a number no `R` holds exactly.
    ///
    /// ```
    /// # #[cfg(feature = "nuclear")] {
    /// use exact_units::{Exact, Prefix, Uncertain, units::ElectronVolt};
    ///
    /// // AME2020: a mass excess and its one-standard-deviation column, in keV
    /// let m = Uncertain::<ElectronVolt>::from_columns("-2345.67", "2.78", Prefix::Kilo).unwrap();
    /// assert_eq!(*m.value(), Exact::int(-2_345_670));
    /// assert_eq!(m.to_string(), "-2.34567(278) MeV");
    /// for (v, u) in [("3398#", "200#"), (">300", "5"), ("5", "+4-3")] {
    ///     assert!(Uncertain::<ElectronVolt>::from_columns(v, u, Prefix::Kilo).is_err());
    /// }
    /// # }
    /// ```
    ///
    /// # Errors
    ///
    /// A [ParseUncertainError] that says which of the refusals above it is.
    pub fn from_columns(
        value: &str,
        u: &str,
        prefix: Prefix,
    ) -> Result<Self, ParseUncertainError<U>> {
        columns(value.trim(), u.trim(), prefix).map_err(ParseUncertainError::new)
    }

    /// `v ± U` with its unit, as AME prints it, read with the coverage factor `k` the caller names: u = U/k, so AME's
    /// one-standard-deviation columns are `k = 1` and a GUM expanded uncertainty is its own k (often 2). Reads
    /// `2345.67 ± 2.78 keV` and `(2345.67 ± 2.78) keV`, `+/-` for `±`; what [pm](Self::pm) writes.
    ///
    /// ```
    /// # #[cfg(feature = "electrical")] {
    /// use exact_units::{Exact, Fraction, Uncertain, units::Ohm};
    ///
    /// // a certificate's expanded uncertainty, with its coverage factor k = 2
    /// let k = Fraction::int(2);
    /// let r = Uncertain::<Ohm>::parse_pm("(10.000742 ± 0.000129) Ω", &k).unwrap();
    /// assert_eq!(*r.u(), Exact::new(129, 2_000_000));
    /// assert_eq!(r.pm(&k).unwrap().to_string(), "(10.000742 ± 0.000129) Ω");
    /// # }
    /// ```
    ///
    /// # Errors
    ///
    /// A [ParseUncertainError]: no `±`, a `k` at or below 0, the refusals of [from_columns](Self::from_columns),
    /// another unit.
    pub fn parse_pm(s: &str, k: &R) -> Result<Self, ParseUncertainError<U>> {
        plus_minus(s.trim(), k).map_err(ParseUncertainError::new)
    }
}

/// Why text isn't an [Uncertain]: one of the forms a data table prints refused by name, or a number that isn't one.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum Why {
    /// The value or the uncertainty isn't a number of `U`, as [Exact]'s text says.
    Number(Kind),
    /// A `#`: AME's and NUBASE's estimated value.
    Estimated,
    /// `>`, `<`, `~`, `≥`, `≤`: a limit, not a value.
    Limit,
    /// A signed uncertainty, or two (`+4-3`).
    Asymmetric,
    /// No uncertainty in parentheses (or no `±`).
    Missing,
    /// A space between the value and its parentheses (`1600 (7)`).
    Spaced,
    /// Whole digits in parentheses after a value with a repetend or a fraction, which has no last digit to count in.
    NoLastDigit,
    /// A coverage factor at or below 0.
    Coverage,
}

impl From<Kind> for Why {
    fn from(k: Kind) -> Self {
        Self::Number(k)
    }
}

/// Why text isn't an [Uncertain] of `U`: an estimated value (`#`), a limit, an asymmetric uncertainty, no
/// uncertainty, or a number that isn't one of `U`.
pub struct ParseUncertainError<U> {
    why: Why,
    _unit: PhantomData<U>,
}

impl<U> ParseUncertainError<U> {
    const fn new(why: Why) -> Self {
        Self {
            why,
            _unit: PhantomData,
        }
    }
}

impl<U> Clone for ParseUncertainError<U> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<U> Copy for ParseUncertainError<U> {}

impl<U> PartialEq for ParseUncertainError<U> {
    fn eq(&self, other: &Self) -> bool {
        self.why == other.why
    }
}

impl<U: Unit> fmt::Debug for ParseUncertainError<U> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ParseUncertainError({self})")
    }
}

impl<U: Unit> fmt::Display for ParseUncertainError<U> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let unit = UnitText::<U>(PhantomData);
        match self.why {
            Why::Number(kind) => write!(f, "{}", super::ParseExactError::<U>::new(kind)),
            Why::Estimated => f.write_str(
                "a # marks an estimated value (from systematic trends, not measured): not read as a measured one",
            ),
            Why::Limit => f.write_str("a limit (>, <, ~, ≥, ≤) is not a value with an uncertainty"),
            Why::Asymmetric => f.write_str(
                "a signed or asymmetric uncertainty (+4-3): a standard uncertainty is one number, at least 0",
            ),
            Why::Missing => write!(
                f,
                "no uncertainty: a value with its standard uncertainty in parentheses, like \"1600(7){unit}\""
            ),
            Why::Spaced => f.write_str(
                "a space between a value and its uncertainty in parentheses: the concise form writes them together, \
                 1600(7)",
            ),
            Why::NoLastDigit => f.write_str(
                "whole digits in parentheses count the value's last digit, and a value with a repetend or a \
                 fraction has none: write the uncertainty with its point, 0.[3](0.1)",
            ),
            Why::Coverage => f.write_str("a coverage factor k at or below 0"),
        }
    }
}

impl<U: Unit> core::error::Error for ParseUncertainError<U> {}

/// Spaces, thin spaces and narrow no-break spaces between two digits, as CODATA groups them (`6.674 30`).
const GROUP: [char; 3] = [' ', '\u{2009}', '\u{202F}'];

/// The longest number text read with its digits grouped: room for every number a `u128` fraction writes, grouped.
/// Text without group spaces is read in place, at any length.
const GROUPED: usize = 512;

/// A number's text without the spaces that group its digits: `s` itself where there are none, else a copy on the
/// stack.
#[expect(
    clippy::large_enum_variant,
    reason = "no alloc: a buffer on the stack for the one number being read, gone when it is read"
)]
enum Ungrouped<'a> {
    Plain(&'a str),
    Copied { bytes: [u8; GROUPED], len: usize },
}

impl<'a> Ungrouped<'a> {
    /// `s` with each group space between two digits of its mantissa (before an exponent's `e`) left out, `None` past
    /// [GROUPED] bytes where there are such spaces.
    fn of(s: &'a str) -> Option<Self> {
        let mantissa = s.find(['e', 'E']).unwrap_or(s.len());
        let grouped = |i: usize, c: char| {
            i < mantissa
                && GROUP.contains(&c)
                && s.get(..i)
                    .is_some_and(|b| b.ends_with(|p: char| p.is_ascii_digit()))
                && s.get(i.saturating_add(c.len_utf8())..)
                    .is_some_and(|a| a.starts_with(|n: char| n.is_ascii_digit()))
        };
        if !s.char_indices().any(|(i, c)| grouped(i, c)) {
            return Some(Self::Plain(s));
        }
        let (mut bytes, mut len) = ([0u8; GROUPED], 0usize);
        for (i, c) in s.char_indices() {
            if grouped(i, c) {
                continue;
            }
            let piece = s.get(i..i.checked_add(c.len_utf8())?)?;
            let end = len.checked_add(piece.len())?;
            bytes.get_mut(len..end)?.copy_from_slice(piece.as_bytes());
            len = end;
        }
        Some(Self::Copied { bytes, len })
    }

    fn as_str(&self) -> &str {
        match self {
            Self::Plain(s) => s,
            Self::Copied { bytes, len } => bytes
                .get(..*len)
                .and_then(|b| core::str::from_utf8(b).ok())
                .unwrap_or_default(),
        }
    }
}

/// A `#` anywhere in `s`: an estimated value, refused by name before anything is read as a number.
fn estimated(s: &str) -> Result<(), Why> {
    if s.contains('#') {
        return Err(Why::Estimated);
    }
    Ok(())
}

/// A limit sign first in `s`, the text of one number (the value or the uncertainty, each at its place).
fn limit(s: &str) -> Result<(), Why> {
    if s.trim_start().starts_with(['>', '<', '~', '≥', '≤']) {
        return Err(Why::Limit);
    }
    Ok(())
}

/// [estimated] and [limit] of one number's text.
fn marks(s: &str) -> Result<(), Why> {
    estimated(s)?;
    limit(s)
}

/// A sign in an uncertainty's text (other than an exponent's): a signed or asymmetric uncertainty.
fn signed(s: &str) -> bool {
    let mut prev = None;
    for c in s.chars() {
        if matches!(c, '+' | '-' | '−' | '∓' | '±') && !matches!(prev, Some('e' | 'E')) {
            return true;
        }
        prev = Some(c);
    }
    false
}

/// The whole text `s` as a number, `x · 10^e`, its digits grouped by spaces or not.
fn whole<R: ExactNum>(s: &str) -> Result<(R, i32), Why> {
    let s = Ungrouped::of(s).ok_or(Why::Number(Kind::Fit))?;
    plain(s.as_str())
}

/// The whole text `s` as a number, `x · 10^e`, without group spaces: an uncertainty outside parentheses, where a
/// space ends the number (`2 2` is two columns, not 22).
fn plain<R: ExactNum>(s: &str) -> Result<(R, i32), Why> {
    match R::read_number(s)? {
        (x, e, "") => Ok((x, e)),
        _ => Err(Why::Number(Kind::Form)),
    }
}

/// `x · 10^e` in steps of `prefix` of `U`.
fn in_prefix<R: ExactNum>(x: R, e: i32, prefix: Prefix) -> Result<R, Why> {
    let x = x.scaled_by(0, e).ok_or(Why::Number(Kind::Fit))?;
    let s = prefix
        .magnitude()
        .and_then(|m| R::from_magnitude(&m))
        .ok_or(Why::Number(Kind::Fit))?;
    x.checked_mul(&s).ok_or(Why::Number(Kind::Fit))
}

/// [Uncertain::from_columns].
fn columns<U, R: ExactNum>(value: &str, u: &str, prefix: Prefix) -> Result<Uncertain<U, R>, Why> {
    marks(value)?;
    marks(u)?;
    if signed(u) {
        return Err(Why::Asymmetric);
    }
    let (v, ve) = whole::<R>(value)?;
    let (x, xe) = plain::<R>(u)?;
    Ok(Uncertain {
        value: Exact::of(in_prefix(v, ve, prefix)?),
        u: Exact::of(in_prefix(x, xe, prefix)?),
    })
}

/// The power of ten of the last digit of a plain decimal's text (`6.67430` −5, `1600` 0, `1.5e-3` −4), `None` for a
/// repetend or a fraction.
fn last_digit(s: &str) -> Option<i32> {
    if s.contains(['/', '[']) {
        return None;
    }
    let (digits, exp) = match s.find(['e', 'E']) {
        Some(i) => (
            s.get(..i)?,
            s.get(i.checked_add(1)?..)?.parse::<i32>().ok()?,
        ),
        None => (s, 0),
    };
    let places = digits.split_once('.').map_or(0, |(_, frac)| {
        frac.bytes().filter(u8::is_ascii_digit).count()
    });
    exp.checked_sub(i32::try_from(places).ok()?)
}

/// An exponent at the start of `s` (`e-11`, `E+3`), and the text after it; 0 and `s` without one.
fn exponent(s: &str) -> Result<(i32, &str), Why> {
    let Some(r) = s.strip_prefix(['e', 'E']) else {
        return Ok((0, s));
    };
    let digits_from = usize::from(r.starts_with(['+', '-']));
    let end = r
        .bytes()
        .skip(digits_from)
        .take_while(u8::is_ascii_digit)
        .count()
        .checked_add(digits_from)
        .ok_or(Why::Number(Kind::Fit))?;
    if end == digits_from {
        // an "e" with no digit after it: the unit's text (`5(1) eV`)
        return Ok((0, s));
    }
    let e = r
        .get(..end)
        .and_then(|t| t.parse::<i32>().ok())
        .ok_or(Why::Number(Kind::Fit))?;
    Ok((e, r.get(end..).unwrap_or_default()))
}

/// The concise form, `1600(7) y`: [Uncertain]'s `FromStr`.
fn concise<U: Unit, R: ExactNum>(s: &str) -> Result<Uncertain<U, R>, Why> {
    estimated(s)?;
    limit(s)?;
    let (v_text, rest) = s.split_once('(').ok_or(Why::Missing)?;
    let (u_text, after) = rest.split_once(')').ok_or(Why::Number(Kind::Form))?;
    let u_text = u_text.trim();
    limit(u_text)?;
    // the parentheses follow the value's last digit; any other is the unit's (`y (NUBASE2020)`, `m³/(kg·s²)`)
    let ends_number = |t: &str| t.ends_with(|c: char| c.is_ascii_digit() || matches!(c, '.' | ']'));
    if !ends_number(v_text) {
        let trimmed = v_text.trim_end();
        return Err(if trimmed.is_empty() {
            Why::Number(Kind::Form)
        } else if trimmed.len() < v_text.len() && ends_number(trimmed) {
            Why::Spaced
        } else {
            Why::Missing
        });
    }
    if signed(u_text) {
        return Err(Why::Asymmetric);
    }
    let v_clean = Ungrouped::of(v_text).ok_or(Why::Number(Kind::Fit))?;
    let (v, ve) = whole::<R>(v_clean.as_str())?;
    let (x, xe) = whole::<R>(u_text)?;
    let xe = if u_text.contains(['.', '/', '[']) {
        xe
    } else {
        // whole digits, nothing else: in units of the value's last digit
        if !u_text
            .chars()
            .all(|c| c.is_ascii_digit() || c == '_' || GROUP.contains(&c))
        {
            return Err(Why::Number(Kind::Form));
        }
        let last = last_digit(v_clean.as_str()).ok_or(Why::NoLastDigit)?;
        xe.checked_add(last).ok_or(Why::Number(Kind::Fit))?
    };
    let (e, unit) = exponent(after)?;
    // one exponent, before the parentheses or after them, never both
    if e != 0 && v_clean.as_str().contains(['e', 'E']) {
        return Err(Why::Number(Kind::Form));
    }
    let p = prefix_of::<U>(unit.trim()).ok_or(Why::Number(Kind::Unit))?;
    let scale = |x: R, xe: i32| {
        xe.checked_add(e)
            .and_then(|k| k.checked_add(p.pow10))
            .and_then(|k| x.scaled_by(p.pow2, k))
            .ok_or(Why::Number(Kind::Fit))
    };
    Ok(Uncertain {
        value: Exact::of(scale(v, ve)?),
        u: Exact::of(scale(x, xe)?),
    })
}

/// [Uncertain::parse_pm].
fn plus_minus<U: Unit, R: ExactNum>(s: &str, k: &R) -> Result<Uncertain<U, R>, Why> {
    if k.is_negative() || k.is_zero() {
        return Err(Why::Coverage);
    }
    estimated(s)?;
    let (inner, unit) = match s.strip_prefix('(') {
        Some(r) => r.split_once(')').ok_or(Why::Number(Kind::Form))?,
        None => (s, ""),
    };
    let (v_text, rest) = inner
        .split_once('±')
        .or_else(|| inner.split_once("+/-"))
        .ok_or(Why::Missing)?;
    let rest = rest.trim();
    limit(v_text)?;
    limit(rest)?;
    if rest.starts_with(['+', '-', '−']) {
        return Err(Why::Asymmetric);
    }
    let (v, ve) = whole::<R>(v_text.trim())?;
    // the uncertainty after ± is not grouped: a space ends it (`1 ± 2 2 s` is no 22)
    let (x, xe, unit) = if unit.is_empty() {
        let (x, xe, after) = R::read_number(rest)?;
        if after.trim_start().starts_with(|c: char| c.is_ascii_digit()) {
            return Err(Why::Number(Kind::Form));
        }
        let p = prefix_of::<U>(after.trim()).ok_or(Why::Number(Kind::Unit))?;
        (x, xe, p)
    } else {
        let (x, xe) = plain::<R>(rest)?;
        let p = prefix_of::<U>(unit.trim()).ok_or(Why::Number(Kind::Unit))?;
        (x, xe, p)
    };
    let scale = |x: R, xe: i32| {
        xe.checked_add(unit.pow10)
            .and_then(|e| x.scaled_by(unit.pow2, e))
            .ok_or(Why::Number(Kind::Fit))
    };
    let expanded = scale(x, xe)?;
    if expanded.is_negative() {
        return Err(Why::Asymmetric);
    }
    Ok(Uncertain {
        value: Exact::of(scale(v, ve)?),
        u: Exact::of(expanded.checked_div(k).ok_or(Why::Number(Kind::Fit))?),
    })
}

impl<U: Unit, R: ExactNum> FromStr for Uncertain<U, R> {
    type Err = ParseUncertainError<U>;

    /// The concise form: `1600(7) y`, `6.674 30(15)e-11 m³/(kg·s²)`, `100.021 47(0.000 35) g`, the unit with or
    /// without a prefix as [Exact] reads it. Refused by name: `#`, a limit, a signed or asymmetric uncertainty, no
    /// parentheses.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        concise::<U, R>(s.trim()).map_err(ParseUncertainError::new)
    }
}

/// The most places after the point the concise form writes: past them, the value and its uncertainty are each
/// written as [Exact] writes them, the uncertainty with its point.
const MAX_PLACES: u8 = 38;

/// `v` and `u` as whole numbers of the same last digit, `(v·10^d, u·10^d, d)`, `None` where no `d` up to
/// [MAX_PLACES] makes both whole or they don't fit `i128`.
fn last_digits<R: ExactNum>(v: &R, u: &R) -> Option<(i128, i128, u8)> {
    for d in 0..=MAX_PLACES {
        let k = R::from_fraction(Fraction::pow10(i32::from(d)));
        let (vk, uk) = (v.checked_mul(&k)?, u.checked_mul(&k)?);
        if vk.is_integer() && uk.is_integer() {
            return Some((
                vk.round(Rounding::Truncate)?,
                uk.round(Rounding::Truncate)?,
                d,
            ));
        }
    }
    None
}

/// `n / 10^d` in decimal, every one of its `d` places written (`16000, 1` is `1600.0`).
fn write_places(n: i128, d: u8, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    let mut digits = [0u8; 40];
    let (mut m, mut len) = (n.unsigned_abs(), 0usize);
    let d = usize::from(d);
    while m != 0 || len <= d {
        let Some(slot) = digits.get_mut(len) else {
            return Err(fmt::Error);
        };
        *slot = u8::try_from(m % 10).map_err(|_| fmt::Error)?;
        m /= 10;
        len = len.checked_add(1).ok_or(fmt::Error)?;
    }
    if n < 0 {
        f.write_str("-")?;
    }
    for i in (0..len).rev() {
        write!(f, "{}", digits.get(i).ok_or(fmt::Error)?)?;
        if i == d && d != 0 {
            f.write_str(".")?;
        }
    }
    Ok(())
}

/// The prefix both are written in: the one `{}` picks for the value (for the uncertainty where the value is 0), or
/// the unit itself.
fn shared_prefix<U: Unit, R: ExactNum>(v: &R, u: &R, alternate: bool) -> Prefix {
    if alternate {
        return Prefix::Unit;
    }
    prefix_for::<U>(auto_prefix_of::<U, R>(if v.is_zero() { u } else { v }))
}

/// `v` and `u` of `U` in `prefix`, or in the unit itself where either doesn't fit there.
fn both_in<R: ExactNum>(v: &R, u: &R, prefix: Prefix) -> (R, R, Prefix) {
    match (value_in(v, prefix), value_in(u, prefix)) {
        (Some(a), Some(b)) => (a, b, prefix),
        _ => (v.clone(), u.clone(), Prefix::Unit),
    }
}

impl<U: Unit, R: ExactNum> fmt::Display for Uncertain<U, R> {
    /// The concise form, exactly: `1600(7) y`, the uncertainty in units of the value's last digit and the value
    /// padded to the uncertainty's (`1600.0(15)`). `{}` in the prefix the value picks, `{:#}` in the unit itself.
    /// Where the two have no common last digit (a repetend, digits past 38 places), each as [Exact] writes it, the
    /// uncertainty with its point: `333.[3](0.[3]) Hz`, `1000/7919(2.0) Hz`.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let (v, u) = (&self.value.value, &self.u.value);
        let (v, u, p) = both_in(v, u, shared_prefix::<U, R>(v, u, f.alternate()));
        if let Some((vd, ud, d)) = last_digits(&v, &u) {
            write_places(vd, d, f)?;
            write!(f, "({ud})")?;
        } else {
            write!(f, "{v}({u}")?;
            if u.is_integer() {
                f.write_str(".0")?;
            }
            f.write_str(")")?;
        }
        write_unit(p, &U::EXPR, f)
    }
}

impl<U: Unit, R: ExactNum> fmt::Debug for Uncertain<U, R> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Uncertain({self:#})")
    }
}

impl<U, R: ExactNum> Clone for Uncertain<U, R> {
    fn clone(&self) -> Self {
        Self {
            value: self.value.clone(),
            u: self.u.clone(),
        }
    }
}

/// `Copy` where its number is: `Uncertain<U>` is.
impl<U, R: ExactNum + Copy> Copy for Uncertain<U, R> {}

/// The same value and the same uncertainty.
impl<U, R: ExactNum> PartialEq for Uncertain<U, R> {
    fn eq(&self, other: &Self) -> bool {
        self.value == other.value && self.u == other.u
    }
}

impl<U, R: ExactNum> Eq for Uncertain<U, R> {}

impl<U, R: ExactNum> Hash for Uncertain<U, R> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.value.hash(state);
        self.u.hash(state);
    }
}

/// An [Uncertain] written with `±` and an expanded uncertainty: [Uncertain::pm].
pub struct PlusMinus<U, R: ExactNum = Fraction> {
    value: Exact<U, R>,
    expanded: Exact<U, R>,
}

impl<U: Unit, R: ExactNum> fmt::Display for PlusMinus<U, R> {
    /// `(v ± U) unit`, both as [Exact] writes them, in the prefix the value picks (`{:#}`: the unit itself).
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let (v, x) = (&self.value.value, &self.expanded.value);
        let (v, x, p) = both_in(v, x, shared_prefix::<U, R>(v, x, f.alternate()));
        write!(f, "({v} ± {x})")?;
        write_unit(p, &U::EXPR, f)
    }
}

impl<U: Unit, R: ExactNum> fmt::Debug for PlusMinus<U, R> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "PlusMinus({self:#})")
    }
}
