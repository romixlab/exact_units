//! A symbol to its unit: the table's own symbols, then a prefix split off one.

use super::entry::{Entry, Origin, Prefixes};
use super::prefix::{PREFIXES, Prefix, Scale};
use super::table::{ORIGINS, REFUSED, UNITS};

/// What a symbol reads as.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(crate) enum Symbol {
    /// A unit marker, with the prefix split off it.
    Unit {
        /// The table's line.
        entry: &'static Entry,
        /// The prefix before the line's symbol, if any.
        prefix: Option<&'static Prefix>,
    },
    /// A point at an origin, `°C`.
    Origin(&'static Origin),
    /// Not in the table, nor a prefix and a symbol of it.
    Unknown,
}

/// The table's line of `symbol`, if any.
pub(crate) fn unit(symbol: &str) -> Option<&'static Entry> {
    UNITS
        .binary_search_by(|e| e.symbol.cmp(symbol))
        .ok()
        .and_then(|i| UNITS.get(i))
}

/// Whether `entry` takes `prefix`.
fn takes(entry: &Entry, prefix: &Prefix) -> bool {
    match (entry.prefixes, prefix.scale) {
        (Prefixes::None, _) | (Prefixes::Si, Scale::Pow1024(_)) => false,
        (Prefixes::Si | Prefixes::SiBinary, Scale::Pow10(_))
        | (Prefixes::SiBinary, Scale::Pow1024(_)) => true,
    }
}

/// What `symbol` reads as, or why it is refused: a symbol that reads two ways, a prefix on a unit that takes none.
pub(crate) fn resolve(symbol: &str) -> Result<Symbol, String> {
    if let Some(r) = REFUSED.iter().find(|r| r.symbol == symbol) {
        return Err(format!(
            "`{symbol}` reads more than one way ({}): write the one you mean as a type, as in q!(5 {})",
            r.readings.join(", "),
            r.readings.first().copied().unwrap_or("Unit")
        ));
    }
    if let Some(o) = ORIGINS.iter().find(|o| o.symbol == symbol) {
        return Ok(Symbol::Origin(o));
    }
    if let Some(entry) = unit(symbol) {
        return Ok(Symbol::Unit {
            entry,
            prefix: None,
        });
    }
    let mut found = None;
    let mut refused = None;
    for p in PREFIXES {
        let Some(rest) = symbol.strip_prefix(p.text) else {
            continue;
        };
        let Some(entry) = unit(rest) else {
            continue;
        };
        if !takes(entry, p) {
            refused = Some((p, entry));
            continue;
        }
        if let Some((q, other)) = found.replace((p, entry)) {
            let q: &Prefix = q;
            let other: &Entry = other;
            return Err(format!(
                "`{symbol}` splits two ways, `{}` `{}` or `{}` `{}`: write the unit as a type",
                q.text, other.symbol, p.text, entry.symbol
            ));
        }
    }
    match (found, refused) {
        (Some((p, entry)), _) => Ok(Symbol::Unit {
            entry,
            prefix: Some(p),
        }),
        (None, Some((p, entry))) => Err(format!(
            "`{symbol}`: `{}` takes no prefix `{}`{}",
            entry.symbol,
            p.text,
            match p.scale {
                Scale::Pow1024(_) if entry.prefixes == Prefixes::Si =>
                    " (binary prefixes are for bit and B)",
                _ => "",
            }
        )),
        (None, None) => Ok(Symbol::Unknown),
    }
}
