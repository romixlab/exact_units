//! The number of a `q!` literal: a decimal, exact, as `num · 10^exp`.

/// A decimal `num · 10^exp`, trailing zeros of `num` moved into `exp` (0 is `0 · 10^0`), so `100` and `100.0` are
/// one literal.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(crate) struct Decimal {
    /// The digits, with the sign.
    pub(crate) num: i64,
    /// The power of ten.
    pub(crate) exp: i32,
}

/// The decimal a literal's text writes (`100`, `3.3`, `1_000`, `1e-3`, `2.5E+3`), negated for a `-` before it.
/// `Err` says why the text is no such number: a suffix (`100u8`, `1.0f32`), a radix (`0x10`), digits past `i64`, a
/// negative zero (`-0`, which a float storage would take as +0.0).
pub(crate) fn parse(text: &str, negative: bool) -> Result<Decimal, String> {
    let (mantissa, e) = match text.find(['e', 'E']) {
        Some(at) => match (text.get(..at), text.get(at.saturating_add(1)..)) {
            (Some(m), Some(e)) => (m, Some(e)),
            _ => return Err(format!("`{text}` is not a decimal number")),
        },
        None => (text, None),
    };
    if text.starts_with("0x") || text.starts_with("0o") || text.starts_with("0b") {
        return Err(format!(
            "`{text}`: a literal of q! is decimal, write it in base 10"
        ));
    }
    let mut num: i128 = 0;
    let mut exp: i64 = 0;
    let mut digits = 0_u32;
    let mut point = false;
    for c in mantissa.chars() {
        match c {
            '0'..='9' => {
                let d = i128::from(c.to_digit(10).unwrap_or(0));
                num = num
                    .checked_mul(10)
                    .and_then(|n| n.checked_add(d))
                    .ok_or_else(|| too_many(text))?;
                digits = digits.saturating_add(1);
                if point {
                    exp = exp.saturating_sub(1);
                }
            }
            '_' => {}
            '.' if !point => point = true,
            _ => return Err(suffix(text)),
        }
    }
    if digits == 0 {
        return Err(format!("`{text}` is not a decimal number"));
    }
    if let Some(e) = e {
        let (neg, e) = match e.strip_prefix('-') {
            Some(rest) => (true, rest),
            None => (false, e.strip_prefix('+').unwrap_or(e)),
        };
        let mut v: i64 = 0;
        let mut any = false;
        for c in e.chars() {
            match c {
                '0'..='9' => {
                    v = v
                        .checked_mul(10)
                        .and_then(|v| v.checked_add(i64::from(c.to_digit(10).unwrap_or(0))))
                        .ok_or_else(|| exponent(text))?;
                    any = true;
                }
                '_' => {}
                _ => return Err(suffix(text)),
            }
        }
        if !any {
            return Err(format!("`{text}` is not a decimal number"));
        }
        exp = if neg {
            exp.checked_sub(v)
        } else {
            exp.checked_add(v)
        }
        .ok_or_else(|| exponent(text))?;
    }
    if num == 0 {
        if negative {
            return Err(format!(
                "`-{text}` reads two ways for a float storage, -0.0 or +0.0: write 0"
            ));
        }
        return Ok(Decimal { num: 0, exp: 0 });
    }
    while num % 10 == 0 {
        num /= 10;
        exp = exp.checked_add(1).ok_or_else(|| exponent(text))?;
    }
    if negative {
        num = num.checked_neg().ok_or_else(|| too_many(text))?;
    }
    let num = i64::try_from(num).map_err(|_| too_many(text))?;
    let exp = i32::try_from(exp).map_err(|_| exponent(text))?;
    Ok(Decimal { num, exp })
}

fn too_many(text: &str) -> String {
    format!("`{text}` has more digits than an i64 holds: write fewer")
}

fn exponent(text: &str) -> String {
    format!("`{text}`: the exponent is out of range")
}

fn suffix(text: &str) -> String {
    format!(
        "`{text}`: a literal of q! is a plain decimal, without a suffix; the storage comes from the compare"
    )
}
