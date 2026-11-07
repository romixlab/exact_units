//! The tokens `q!` reads, a copy of the compiler's token trees with an id per token for its span.

/// A token of the input. `id` names the compiler's token it came from (the glue keeps the span and the token
/// itself under it); every token lexed from a string literal has that literal's id.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Tok {
    /// An identifier, `m`, `Speed`, `kΩ`, or a run of unit letters lexed from a string (`°C`, `%`).
    Ident { text: String, id: usize },
    /// One punctuation character; `joint` when the next one follows without a space (`::`).
    Punct { ch: char, joint: bool, id: usize },
    /// A literal as the compiler wrote it: `100`, `3.3`, `1e-3`, `"m/s²"`.
    Lit { text: String, id: usize },
    /// A delimited group, `( .. )`; `[ .. ]` and `{ .. }` are refused where they appear.
    Group {
        delim: Delim,
        inner: Vec<Tok>,
        id: usize,
    },
}

impl Tok {
    /// The id of the compiler's token this one came from.
    pub(crate) fn id(&self) -> usize {
        match self {
            Tok::Ident { id, .. }
            | Tok::Punct { id, .. }
            | Tok::Lit { id, .. }
            | Tok::Group { id, .. } => *id,
        }
    }

    /// The character of a punctuation token, else `None`.
    pub(crate) fn punct(&self) -> Option<char> {
        match self {
            Tok::Punct { ch, .. } => Some(*ch),
            _ => None,
        }
    }
}

/// The delimiter of a [Tok::Group].
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(crate) enum Delim {
    /// `( .. )`
    Paren,
    /// `[ .. ]`
    Bracket,
    /// `{ .. }`
    Brace,
    /// An invisible group, from a `macro_rules!` fragment passed on (`$v:literal`).
    None,
}

/// A refusal of `q!`: the message and the token it points at.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Error {
    /// The token the compiler's error points at.
    pub(crate) id: usize,
    /// What is wrong and what to write instead.
    pub(crate) msg: String,
}

impl Error {
    /// A refusal at token `id`.
    pub(crate) fn at(id: usize, msg: impl Into<String>) -> Error {
        Error {
            id,
            msg: msg.into(),
        }
    }
}
