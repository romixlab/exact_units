//! A unit read from the text after a number: which prefix `ms`, `kHz`, `µS/cm` or `5 %` puts on a unit `U`, the way
//! `{}` writes it and the ways people write it.

use super::{Buf, writes_as};
use crate::tree::{den_prefixes, inside, is_ratio, moved, quotient, takes_bare_prefix, write_unit};
use crate::{Prefix, Unit, UnitExpr};
use core::fmt::{self, Write};
use core::marker::PhantomData;

/// `" s"`: the unit as `{:#}` writes it after the number, nothing for a ratio.
pub(crate) struct UnitText<U>(pub(crate) PhantomData<U>);

impl<U: Unit> fmt::Display for UnitText<U> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write_unit(Prefix::Unit, &U::EXPR, f)
    }
}

/// `" mm²"`: `U` with a prefix, as `{}` writes it after the number.
struct PrefixText<U>(Prefix, PhantomData<U>);

impl<U: Unit> fmt::Display for PrefixText<U> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write_unit(self.0, &U::EXPR, f)
    }
}

/// The scale a prefix in the text puts on the number: `10^pow10 · 2^pow2`.
#[derive(Copy, Clone)]
pub(crate) struct TextPrefix {
    #[cfg_attr(
        not(feature = "serde"),
        expect(
            dead_code,
            reason = "read by with_unit, for a Fixed in a float storage"
        )
    )]
    pub(crate) prefix: Prefix,
    pub(crate) pow10: i32,
    pub(crate) pow2: i32,
}

impl TextPrefix {
    const UNIT: TextPrefix = TextPrefix {
        prefix: Prefix::Unit,
        pow10: 0,
        pow2: 0,
    };

    pub(crate) fn of(prefix: Prefix) -> TextPrefix {
        let (pow10, pow2) = match (prefix.pow10(), prefix) {
            (Some(e), _) => (i32::from(e), 0),
            (None, Prefix::Kibi) => (0, 10),
            (None, Prefix::Mebi) => (0, 20),
            (None, Prefix::Gibi) => (0, 30),
            (None, Prefix::Tebi) => (0, 40),
            _ => (0, 0),
        };
        TextPrefix {
            prefix,
            pow10,
            pow2,
        }
    }
}

/// The prefix the unit text `t` puts on `U`: `ms` on `s` is milli, `s` none, `kg` on `m` none at all.
pub(crate) fn prefix_of<U: Unit>(t: &str) -> Option<TextPrefix> {
    let unit = UnitText::<U>(PhantomData);
    if is_ratio(&U::EXPR) {
        let prefix = match t {
            "" => Prefix::Unit,
            "%" => Prefix::Centi,
            "‰" => Prefix::Milli,
            "ppm" => Prefix::Micro,
            "ppb" => Prefix::Nano,
            _ => return None,
        };
        return Some(TextPrefix::of(prefix));
    }
    let spelled = |t: &str| {
        writes_as(&unit, t)
            || matches!(U::EXPR, UnitExpr::Named(def) if def.id.crate_name == env!("CARGO_CRATE_NAME") && crate::units::spellings(def.id.name).contains(&t))
    };
    if spelled(t) {
        return Some(TextPrefix::UNIT);
    }
    let found = moved_of::<U>(t)
        .or_else(|| quotient_of::<U>(t))
        .or_else(|| nameless_of::<U>(t));
    if let Some(p) = found {
        return Some(TextPrefix::of(p));
    }
    let named = takes_bare_prefix(&U::EXPR);
    let prefixes = (-30..=30i8).filter_map(Prefix::si_from_pow10).chain([
        Prefix::Kibi,
        Prefix::Mebi,
        Prefix::Gibi,
        Prefix::Tebi,
    ]);
    for p in prefixes {
        let Some(rest) = t
            .strip_prefix(p.symbol())
            .filter(|_| !p.symbol().is_empty())
        else {
            continue;
        };
        // a compound unit, and a named one whose first factor has an exponent or a prefix (`m²`, `kg/m³`), takes its
        // prefix on the whole of it in parentheses, as Display writes it: `m(V/A)`, `m(m²)`
        let rest = if named {
            Some(rest)
        } else {
            rest.strip_prefix('(').and_then(|r| r.strip_suffix(')'))
        };
        if rest.is_some_and(spelled) {
            return Some(TextPrefix::of(p));
        }
    }
    // micro as u, and as the Greek letter mu next to the micro sign
    let micro = t.strip_prefix('u').or_else(|| t.strip_prefix('\u{3bc}'));
    if named && micro.is_some_and(spelled) {
        return Some(TextPrefix::of(Prefix::Micro));
    }
    None
}

/// The scale of text that names it the way people write it on metres (`cm²`, `mL/s`): whatever `{}` writes for `U`
/// reads back, a volume in metres too (`5 cm³` where `5 mL` is written), and nothing else (a `hm²` isn't written, so it
/// is refused).
fn moved_of<U: Unit>(t: &str) -> Option<Prefix> {
    // micro as u, and as the Greek letter mu next to the micro sign
    let mut micro = Buf::new();
    let t = match t.strip_prefix('u').or_else(|| t.strip_prefix('\u{3bc}')) {
        Some(rest) if write!(micro, "µ{rest}").is_ok() => micro.as_str(),
        _ => t,
    };
    // every power a scale can have: 10⁻⁹ on V/√Hz is written nV/√Hz
    (-63..=63i8).filter_map(Prefix::from_pow10).find(|p| {
        moved(*p, &U::EXPR).is_some() && writes_as(PrefixText::<U>(*p, PhantomData), t)
            || inside(*p, &U::EXPR).is_some_and(|inner| {
                t.strip_prefix(inner.symbol())
                    .is_some_and(|rest| writes_as(UnitText::<U>(PhantomData), rest))
            })
    })
}

/// The scale of text that spells a quotient in any way people do, not only the one `{}` writes for `U`: `mS/cm` and
/// `dS/m`, `mg/L` and `g/m³`, `5 kV/cm` and `500 kV/m`. An SI prefix on the numerator (`g` on a `kg`) and one on the
/// denominator that is used with it, a litre for a cubic metre.
fn quotient_of<U: Unit>(t: &str) -> Option<Prefix> {
    let q = quotient(&U::EXPR)?;
    let (top, bottom) = t.split_once('/')?;
    let top = if top == q.num {
        0
    } else if !q.takes_prefix {
        return None;
    } else if q.num == "kg" {
        si_exponent(top.strip_suffix('g')?)?.checked_sub(3)?
    } else {
        si_exponent(top.strip_suffix(q.num)?)?
    };
    let bottom = den_exponent(bottom, q.den?)?;
    Prefix::from_pow10(top.checked_add(bottom)?)
}

/// The power of ten that `text` in the place of the denominator `den` puts on the quotient: `km` for `m` is -3.
fn den_exponent(text: &str, den: &str) -> Option<i8> {
    if text == den {
        return Some(0);
    }
    let (power, used) = den_prefixes(den)?;
    let on = |symbol: &str| {
        let e = si_exponent(text.strip_suffix(symbol)?)?;
        used.contains(&e).then_some(e)
    };
    match on(den) {
        Some(e) => power.checked_mul(e)?.checked_neg(),
        // a litre is 10⁻³ m³; a mL and a µL are in the table's powers
        None if den == "m³" => {
            let e = si_exponent(text.strip_suffix('L')?)?;
            matches!(e, 0 | -3 | -6).then(|| 3i8.checked_sub(e))?
        }
        None => None,
    }
}

/// The power of ten of an SI prefix's symbol, nothing as 0, micro as `u` and as the Greek letter mu too.
fn si_exponent(symbol: &str) -> Option<i8> {
    let symbol = if matches!(symbol, "u" | "\u{3bc}") {
        "µ"
    } else {
        symbol
    };
    (-30..=30i8)
        .filter_map(Prefix::si_from_pow10)
        .find(|p| p.symbol() == symbol)?
        .pow10()
}

/// The power of ten without a prefix's name that text starts with, then the unit: `·10⁴ m²`, as `{}` writes it, and as
/// it wrote `·10⁻⁴ S/m` before that became `µS/cm`.
fn nameless_of<U: Unit>(t: &str) -> Option<Prefix> {
    t.starts_with('·')
        .then(|| {
            (-63..=63i8)
                .filter_map(Prefix::from_pow10)
                .filter(|p| !p.is_named())
                .find(|p| writes_as(format_args!("{p}{}", UnitText::<U>(PhantomData)), t))
        })
        .flatten()
}
