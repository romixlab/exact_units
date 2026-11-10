//! Property 10: `q!`'s reader takes any text without a panic. A unit or a whole literal in a string (`q!("9.81
//! m/s²")`) is text the compiler hands the proc-macro: whatever it is, the reader gives a literal or an error with a
//! message, never a panic (a panic in a proc-macro is a crash of the build). The reader is the macro crate's own code,
//! compiled in by path.
#![expect(
    clippy::allow_attributes,
    reason = "the reader's items are dead code in some configurations of this crate, not in others"
)]

#[path = "../../macros/src/pure/mod.rs"]
#[allow(
    dead_code,
    unreachable_pub,
    reason = "only the reader's entries are used here"
)]
pub(crate) mod pure;

use pure::grammar::literal;
use pure::model::Tok;

/// Text `s` read as a whole literal in a string, as the number of a literal and as the unit after `1`: a literal or
/// an error with a message, each time.
pub(crate) fn reads_or_refuses(s: &str) {
    let quoted = Tok::Lit {
        text: format!("\"{s}\""),
        id: 1,
    };
    let number = Tok::Lit {
        text: s.to_owned(),
        id: 1,
    };
    let one = Tok::Lit {
        text: "1".to_owned(),
        id: 1,
    };
    for toks in [vec![quoted.clone()], vec![number], vec![one, quoted]] {
        if let Err(e) = literal(&toks, 0) {
            assert!(!e.msg.is_empty(), "{s:?}: an error without a message");
        }
    }
}

#[cfg(not(any(kani, fuzzing)))]
mod tests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn any_text(s in ".*") {
            reads_or_refuses(&s);
        }

        #[test]
        fn unit_like_text(s in "[-0-9.eE_ ]{0,6}[a-zA-Zµμ°′″²³⁻¹·*/^()%‰ ]{0,12}") {
            reads_or_refuses(&s);
        }
    }

    #[test]
    fn edges() {
        for s in [
            "",
            "(",
            ")",
            "((m)",
            "m)",
            "1 m^",
            "1 m^-",
            "1 ⁻",
            "²",
            "1 ²m",
            "1e",
            "1e-",
            ".5 m",
            "1 \\ m",
            "9.81 m/s²",
            "1 °C",
        ] {
            reads_or_refuses(s);
        }
    }
}
