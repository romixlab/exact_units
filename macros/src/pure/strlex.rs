//! A unit, or a whole literal, written in a string: `q!(90 "°")`, `q!("9.81 m/s²")`. A string takes what Rust's
//! lexer refuses outside one: `°`, `′`, `″`, `²`, `³`, `⁻¹`, `·`, `µ`, `‰`, `℃`, and `±` of a tolerance
//! (`q!("5 V ± 1 %")`), lexed as the `+-` written outside a string.

use super::model::{Delim, Error, Tok};

/// The value of a string literal's text (`"m/s²"`, with its quotes), or `None` for another literal. Escapes are
/// refused: a unit has no use for them.
pub(crate) fn content(text: &str) -> Option<Result<&str, String>> {
    let inner = text.strip_prefix('"')?.strip_suffix('"')?;
    Some(if inner.contains('\\') {
        Err("a unit in a string has no escapes: write the characters themselves".to_owned())
    } else {
        Ok(inner)
    })
}

/// A superscript digit's value, `²` is 2.
fn superscript(c: char) -> Option<u32> {
    Some(match c {
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
    })
}

/// Whether `c` is part of a symbol: a letter (`m`, `Ω`, `µ`) or a sign written as a unit (`°`, `′`, `″`, `%`).
fn symbol_char(c: char) -> bool {
    c.is_alphabetic() || matches!(c, '°' | '′' | '″' | '_' | '%' | '‰' | '℃' | '℉')
}

/// The length of the number `b` starts with: digits, `_` and `.`, then an exponent `e`, `e-`, `E+` with digits.
/// A letter ends it, so `5m` is 5 and `m`.
fn number_len(b: &[u8]) -> usize {
    let digits = |from: usize| {
        b.get(from..).map_or(0, |t| {
            t.iter()
                .take_while(|c| c.is_ascii_digit() || **c == b'_')
                .count()
        })
    };
    // a `.` before another is a range's `..`, not the number's point: `4.9..5.1`
    let mut n = 0;
    while let Some(&c) = b.get(n) {
        let range = c == b'.' && b.get(n.saturating_add(1)) == Some(&b'.');
        if range || !(c.is_ascii_digit() || c == b'_' || c == b'.') {
            break;
        }
        n = n.saturating_add(1);
    }
    if matches!(b.get(n), Some(b'e' | b'E')) {
        let sign = usize::from(matches!(b.get(n.saturating_add(1)), Some(b'+' | b'-')));
        let first = n.saturating_add(1).saturating_add(sign);
        if b.get(first).is_some_and(u8::is_ascii_digit) {
            n = first.saturating_add(digits(first));
        }
    }
    n
}

/// A power in superscripts, `²` or `⁻¹`, from its first character `c` on: `^`, `-` for a `⁻`, the digits.
fn power(
    c: char,
    chars: &mut core::iter::Peekable<core::str::CharIndices<'_>>,
    out: &mut Vec<Tok>,
    id: usize,
) -> Result<(), Error> {
    let mut text: String = superscript(c)
        .and_then(|d| char::from_digit(d, 10))
        .into_iter()
        .collect();
    while let Some(d) = chars.peek().and_then(|(_, n)| superscript(*n)) {
        text.push(char::from_digit(d, 10).unwrap_or('0'));
        chars.next();
    }
    if text.is_empty() {
        return Err(Error::at(id, "`⁻` without a superscript power after it"));
    }
    out.push(Tok::Punct {
        ch: '^',
        joint: false,
        id,
    });
    if c == '⁻' {
        out.push(Tok::Punct {
            ch: '-',
            joint: false,
            id,
        });
    }
    out.push(Tok::Lit { text, id });
    Ok(())
}

/// The punctuation of a range or a tolerance in a string, from its first character `c`: `±` as the `+-` written
/// outside one, `+`, and `..` or `..=`. `false` for a `.` that starts no `..`.
fn range_punct(
    c: char,
    chars: &mut core::iter::Peekable<core::str::CharIndices<'_>>,
    out: &mut Vec<Tok>,
    id: usize,
) -> bool {
    let mut punct = |ch: char, joint: bool| out.push(Tok::Punct { ch, joint, id });
    match c {
        '±' => {
            punct('+', true);
            punct('-', false);
        }
        '+' => punct('+', chars.peek().is_some_and(|(_, n)| *n == '-')),
        '.' if chars.peek().is_some_and(|(_, n)| *n == '.') => {
            chars.next();
            let incl = chars.peek().is_some_and(|(_, n)| *n == '=');
            punct('.', true);
            punct('.', incl);
            if incl {
                chars.next();
                punct('=', false);
            }
        }
        _ => return false,
    }
    true
}

/// The tokens of string `s`, each with id `id` (the string literal's own): numbers as literals, symbols as
/// identifiers, `·` `⋅` `×` as `*`, superscripts as `^` and the power, parentheses as groups.
pub(crate) fn lex(s: &str, id: usize) -> Result<Vec<Tok>, Error> {
    let mut stack: Vec<Vec<Tok>> = vec![Vec::new()];
    let mut chars = s.char_indices().peekable();
    let err = |msg: String| Error::at(id, msg);
    while let Some((at, c)) = chars.next() {
        let out = stack
            .last_mut()
            .ok_or_else(|| err("unbalanced parentheses".to_owned()))?;
        if c.is_whitespace() {
            continue;
        }
        if c.is_ascii_digit() || (c == '.' && chars.peek().is_some_and(|(_, d)| d.is_ascii_digit()))
        {
            let rest = s.get(at..).unwrap_or_default();
            let stop = at.saturating_add(number_len(rest.as_bytes()));
            while chars.peek().is_some_and(|&(i, _)| i < stop) {
                chars.next();
            }
            let text = s
                .get(at..stop)
                .ok_or_else(|| err("a number cut in the middle".to_owned()))?;
            out.push(Tok::Lit {
                text: text.to_owned(),
                id,
            });
            continue;
        }
        if c == '⁻' || superscript(c).is_some() {
            power(c, &mut chars, out, id)?;
            continue;
        }
        match c {
            '(' => {
                stack.push(Vec::new());
            }
            ')' => {
                let inner = stack
                    .pop()
                    .ok_or_else(|| err("unbalanced parentheses".to_owned()))?;
                let out = stack
                    .last_mut()
                    .ok_or_else(|| err("a `)` without its `(`".to_owned()))?;
                out.push(Tok::Group {
                    delim: Delim::Paren,
                    inner,
                    id,
                });
            }
            '*' | '·' | '⋅' | '×' => out.push(Tok::Punct {
                ch: '*',
                joint: false,
                id,
            }),
            '±' | '+' | '.' if range_punct(c, &mut chars, out, id) => {}
            '/' | '^' | '-' | '@' => out.push(Tok::Punct {
                ch: c,
                joint: false,
                id,
            }),
            '%' | '‰' => out.push(Tok::Ident {
                text: c.to_string(),
                id,
            }),
            c if symbol_char(c) => {
                let mut text = String::from(c);
                while let Some(&(_, n)) = chars.peek() {
                    if !symbol_char(n) || matches!(n, '%' | '‰') {
                        break;
                    }
                    text.push(n);
                    chars.next();
                }
                if chars.peek().is_some_and(|(_, n)| n.is_ascii_digit()) {
                    return Err(err(format!(
                        "`{text}` followed by a digit: a power is `{text}^2` or `{text}²`"
                    )));
                }
                out.push(Tok::Ident { text, id });
            }
            _ => return Err(err(format!("`{c}` is not part of a unit"))),
        }
    }
    match (stack.pop(), stack.is_empty()) {
        (Some(top), true) => Ok(top),
        _ => Err(err("a `(` without its `)`".to_owned())),
    }
}
