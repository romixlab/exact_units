//! A unit, or a whole literal, written in a string: `q!(90 "°")`, `q!("9.81 m/s²")`. A string takes what Rust's
//! lexer refuses outside one: `°`, `′`, `″`, `²`, `³`, `⁻¹`, `·`, `µ`, `‰`, `℃`.

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
    let mut n = b
        .iter()
        .take_while(|c| c.is_ascii_digit() || **c == b'_' || **c == b'.')
        .count();
    if matches!(b.get(n), Some(b'e' | b'E')) {
        let sign = usize::from(matches!(b.get(n.saturating_add(1)), Some(b'+' | b'-')));
        let first = n.saturating_add(1).saturating_add(sign);
        if b.get(first).is_some_and(u8::is_ascii_digit) {
            n = first.saturating_add(digits(first));
        }
    }
    n
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
        if let Some(d) = superscript(c) {
            let mut text = String::from(char::from_digit(d, 10).unwrap_or('0'));
            while let Some(d) = chars.peek().and_then(|(_, n)| superscript(*n)) {
                text.push(char::from_digit(d, 10).unwrap_or('0'));
                chars.next();
            }
            out.push(Tok::Punct {
                ch: '^',
                joint: false,
                id,
            });
            out.push(Tok::Lit { text, id });
            continue;
        }
        if c == '⁻' {
            out.push(Tok::Punct {
                ch: '^',
                joint: false,
                id,
            });
            out.push(Tok::Punct {
                ch: '-',
                joint: false,
                id,
            });
            let mut text = String::new();
            while let Some(d) = chars.peek().and_then(|(_, n)| superscript(*n)) {
                text.push(char::from_digit(d, 10).unwrap_or('0'));
                chars.next();
            }
            if text.is_empty() {
                return Err(err("`⁻` without a superscript power after it".to_owned()));
            }
            out.push(Tok::Lit { text, id });
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
