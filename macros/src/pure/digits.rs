//! Known leading digits, `q!(9.542425... dB)`: the values whose decimal starts with the digits written, as a range.
//!
//! ```text
//! prefix := '-'? NUMBER '...' unit      (NUMBER with digits after its point, no exponent)
//! ```
//!
//! `9.542425...` is `9.542425 ..< 9.542426`, `-9.54...` is `-9.55 <.. -9.54`: the digits are cut, never rounded,
//! so the interval's ends are the digits and the digits with one added to the last. Against a storage the range
//! holds exactly the stored values within it, as `Inside` does; `Outside` would take in values with other digits.
//! A whole number before `...` reads two ways (`9...` as 9 to 10 or as 9, 90, 900, ..), so a prefix has a point and
//! a digit after it.

use super::grammar::number_and_unit;
use super::model::{Error, Tok};
use super::range::{Fit, Input};

/// The position of the first `.` of a top-level `...`.
pub(crate) fn dots(toks: &[Tok]) -> Option<usize> {
    toks.windows(3).position(|w| {
        matches!(
            w,
            [
                Tok::Punct {
                    ch: '.',
                    joint: true,
                    ..
                },
                Tok::Punct {
                    ch: '.',
                    joint: true,
                    ..
                },
                Tok::Punct { ch: '.', .. },
            ]
        )
    })
}

/// The range of the values whose decimal starts with the digits before the `...` at `k`. `fit` is what was
/// written after a `,` (at token `fit_id`), `whole` the id an error without a token of its own points at.
pub(crate) fn prefix(
    body: &[Tok],
    k: usize,
    fit: Fit,
    fit_id: usize,
    whole: usize,
    in_string: bool,
) -> Result<Input, Error> {
    let at = body.get(k).map_or(whole, Tok::id);
    let (before, unit) = (
        body.get(..k).unwrap_or_default(),
        body.get(k.saturating_add(3)..).unwrap_or_default(),
    );
    let (negative, text, num_id) = match before {
        [Tok::Lit { text, id }] => (false, text, *id),
        [s, Tok::Lit { text, id }] if s.punct() == Some('-') => (true, text, *id),
        [] => {
            return Err(Error::at(
                at,
                "`...` comes after the digits it continues: q!(9.542425... dB)",
            ));
        }
        _ => {
            return Err(Error::at(
                at,
                "`...` follows the digits of a decimal and the unit comes after it: q!(9.542425... dB); a range is `..=` or `..`",
            ));
        }
    };
    if matches!(unit.first(), Some(Tok::Lit { .. })) {
        return Err(Error::at(
            at,
            "`...` ends a prefix of digits, q!(9.54... V); a range between two numbers is `..=` or `..`, q!(4.9 V ..= 5.1 V)",
        ));
    }
    if unit.iter().any(|t| matches!(t.punct(), Some('.' | '+'))) {
        return Err(Error::at(
            at,
            "a prefix is a range on its own: no `..`, `..=` or `+-` beside it, q!(9.54... V)",
        ));
    }
    match fit {
        Fit::Exact => {}
        Fit::Inside | Fit::Outside | Fit::Prefix => {
            return Err(Error::at(
                fit_id,
                "a prefix holds exactly the stored values whose digits start so, nothing to move: drop the `, Inside` or `, Outside`",
            ));
        }
    }
    let (digits, places) = digits(text).map_err(|m| Error::at(num_id, m))?;
    if negative && digits == 0 {
        return Err(Error::at(
            num_id,
            format!(
                "`-{text}...` reads two ways (is 0 in it?): write the range's ends, q!(-0.1 V ..= 0 V) or q!(-0.1 V .. 0 V)"
            ),
        ));
    }
    let next = digits.checked_add(1).ok_or_else(|| {
        Error::at(
            num_id,
            format!("`{text}` has more digits than an i64 holds: write fewer"),
        )
    })?;
    let end = |n: u64| -> Result<_, Error> {
        let mut toks = Vec::new();
        if negative {
            toks.push(Tok::Punct {
                ch: '-',
                joint: false,
                id: num_id,
            });
        }
        toks.push(Tok::Lit {
            text: decimal(n, places),
            id: num_id,
        });
        toks.extend_from_slice(unit);
        number_and_unit(&toks, whole, in_string)
    };
    let (near, far) = (end(digits)?, end(next)?);
    Ok(if negative {
        Input::Range {
            lo: far,
            hi: near,
            lo_open: true,
            open: false,
            fit: Fit::Prefix,
        }
    } else {
        Input::Range {
            lo: near,
            hi: far,
            lo_open: false,
            open: true,
            fit: Fit::Prefix,
        }
    })
}

/// The digits of a prefix's number without its point, and how many come after the point: `9.54` is (954, 2).
fn digits(text: &str) -> Result<(u64, u32), String> {
    if text.contains(['e', 'E']) {
        return Err(format!(
            "`{text}...`: a prefix is plain digits, its exponent reads two ways (of the digits or of the value): write them out, q!(9542.4... V)"
        ));
    }
    let Some((whole, frac)) = text.split_once('.') else {
        return Err(format!(
            "`{text}...`: a prefix has digits after its point, q!({text}.5... V); a whole number before `...` reads two ways ({text} up to the next whole number, or {text}, {text}0, {text}00, ..)"
        ));
    };
    let mut n: u64 = 0;
    let mut places = 0_u32;
    for (c, after) in whole
        .chars()
        .map(|c| (c, false))
        .chain(frac.chars().map(|c| (c, true)))
    {
        match c {
            '0'..='9' => {
                n = n
                    .checked_mul(10)
                    .and_then(|n| n.checked_add(u64::from(c.to_digit(10).unwrap_or(0))))
                    .filter(|n| i64::try_from(*n).is_ok())
                    .ok_or_else(|| {
                        format!("`{text}` has more digits than an i64 holds: write fewer")
                    })?;
                if after {
                    places = places.saturating_add(1);
                }
            }
            '_' => {}
            _ => {
                return Err(format!(
                    "`{text}`: a literal of q! is a plain decimal, without a suffix; the storage comes from the compare"
                ));
            }
        }
    }
    if places == 0 {
        return Err(format!(
            "`{text}...`: a prefix has digits after its point, q!({text}5... V)"
        ));
    }
    Ok((n, places))
}

/// `n · 10^-places` as decimal text: `decimal(955, 2)` is `9.55`.
fn decimal(n: u64, places: u32) -> String {
    let s = n.to_string();
    let places = usize::try_from(places).unwrap_or(usize::MAX);
    let s = format!("{s:0>width$}", width = places.saturating_add(1));
    let (a, b) = s.split_at(s.len().saturating_sub(places));
    format!("{a}.{b}")
}
