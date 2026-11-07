//! The proc-macro behind exact_units' `q!`, the literal that names its unit: `speed < q!(100 m/s)`. Use it through
//! exact_units (`exact_units::q`), which passes its own path in; this crate has no dependencies and no API of its
//! own.
//!
//! The logic is in `pure`, plain Rust over a copy of the tokens; this file converts the compiler's tokens into that
//! copy and the result back, keeping each token's span, so an error points at the symbol it is about.

use proc_macro::{Delimiter, Group, Ident, Literal, Punct, Spacing, Span, TokenStream, TokenTree};

mod pure;

use pure::grammar::{self, At, Unit};
use pure::model::{Delim, Error, Tok};

/// `q!($crate; 100 m/s)`: the literal `exact_units::literal::LiteralIn<N, E, Unit>::NEW`. Called by exact_units'
/// `q!`, which writes the `$crate;` in front.
#[proc_macro]
pub fn q(input: TokenStream) -> TokenStream {
    let mut trees = input.into_iter();
    let mut krate = Vec::new();
    for t in trees.by_ref() {
        if matches!(&t, TokenTree::Punct(p) if p.as_char() == ';') {
            break;
        }
        krate.push(t);
    }
    let mut side = Side { trees: vec![None] };
    let toks = side.convert(trees.collect());
    match grammar::literal(&toks, 0) {
        Ok(lit) => {
            let mut out = Out::new(&krate);
            match &lit.at {
                None => out.literal(&lit, &side),
                Some(at) => out.point(&lit, at, &side),
            }
            out.ts
        }
        Err(e) => error(&e, &side, "q!"),
    }
}

/// `fixed!($crate; 100 m/s)`: the type `exact_units::literal::LiteralIn<N, E, Unit>`, so `fixed!(100 m/s)` reads
/// against `AtLiteral` as `fixed!(0.1 in Volt)` already does, through the unit written as `q!` reads it rather than
/// a bare type path. Called by exact_units' `fixed!`, which writes the `$crate;` in front. A mode of its own, not a
/// flag on [q], so the two emission paths don't have to stay in step.
#[proc_macro]
pub fn fixed(input: TokenStream) -> TokenStream {
    let mut trees = input.into_iter();
    let mut krate = Vec::new();
    for t in trees.by_ref() {
        if matches!(&t, TokenTree::Punct(p) if p.as_char() == ';') {
            break;
        }
        krate.push(t);
    }
    let mut side = Side { trees: vec![None] };
    let toks = side.convert(trees.collect());
    match grammar::literal(&toks, 0) {
        Ok(lit) => match &lit.at {
            None => {
                let mut out = Out::new(&krate);
                out.literal_type(&lit, &side);
                out.ts
            }
            Some(at) => {
                let id = match at {
                    At::Symbol { id, .. } => *id,
                    At::Type(ids) => ids.first().copied().unwrap_or(0),
                };
                let e = Error::at(
                    id,
                    format!(
                        "`{}` names an origin, and fixed! is an amount, a Fixed: write the point's number with Point::from_raw, or compare a Point with q!(..)",
                        side.text(id)
                    ),
                );
                error(&e, &side, "fixed!")
            }
        },
        Err(e) => error(&e, &side, "fixed!"),
    }
}

/// The compiler's token under each id of the copy; id 0 is the macro's call.
struct Side {
    trees: Vec<Option<TokenTree>>,
}

impl Side {
    fn push(&mut self, t: &TokenTree) -> usize {
        self.trees.push(Some(t.clone()));
        self.trees.len().saturating_sub(1)
    }

    /// The source text of token `id`: the symbol a marker was read from (`in`, `"°"`).
    fn text(&self, id: usize) -> String {
        match self.trees.get(id) {
            Some(Some(t)) => t.to_string(),
            _ => String::new(),
        }
    }

    fn span(&self, id: usize) -> Span {
        match self.trees.get(id) {
            Some(Some(t)) => t.span(),
            _ => Span::call_site(),
        }
    }

    fn convert(&mut self, trees: Vec<TokenTree>) -> Vec<Tok> {
        trees
            .into_iter()
            .map(|t| {
                let id = self.push(&t);
                match t {
                    TokenTree::Ident(i) => Tok::Ident {
                        text: i.to_string(),
                        id,
                    },
                    TokenTree::Punct(p) => Tok::Punct {
                        ch: p.as_char(),
                        joint: p.spacing() == Spacing::Joint,
                        id,
                    },
                    TokenTree::Literal(l) => Tok::Lit {
                        text: l.to_string(),
                        id,
                    },
                    TokenTree::Group(g) => {
                        let delim = match g.delimiter() {
                            Delimiter::Parenthesis => Delim::Paren,
                            Delimiter::Bracket => Delim::Bracket,
                            Delimiter::Brace => Delim::Brace,
                            Delimiter::None => Delim::None,
                        };
                        let inner = self.convert(g.stream().into_iter().collect());
                        Tok::Group { delim, inner, id }
                    }
                }
            })
            .collect()
    }
}

/// `::core::compile_error!("q!: ..")` (or `fixed!: ..`) at the token the error is about.
fn error(e: &Error, side: &Side, name: &str) -> TokenStream {
    let span = side.span(e.id);
    let mut msg = Literal::string(&format!("{name}: {}", e.msg));
    msg.set_span(span);
    let mut args = Group::new(
        Delimiter::Parenthesis,
        TokenStream::from(TokenTree::Literal(msg)),
    );
    args.set_span(span);
    let trees = [
        TokenTree::Punct(joint(':', span)),
        TokenTree::Punct(alone(':', span)),
        TokenTree::Ident(Ident::new("core", span)),
        TokenTree::Punct(joint(':', span)),
        TokenTree::Punct(alone(':', span)),
        TokenTree::Ident(Ident::new("compile_error", span)),
        TokenTree::Punct(alone('!', span)),
        TokenTree::Group(args),
    ];
    trees.into_iter().collect()
}

fn joint(c: char, span: Span) -> Punct {
    let mut p = Punct::new(c, Spacing::Joint);
    p.set_span(span);
    p
}

fn alone(c: char, span: Span) -> Punct {
    let mut p = Punct::new(c, Spacing::Alone);
    p.set_span(span);
    p
}

/// The output, built token by token.
struct Out<'a> {
    krate: &'a [TokenTree],
    ts: TokenStream,
}

impl<'a> Out<'a> {
    fn new(krate: &'a [TokenTree]) -> Self {
        Out {
            krate,
            ts: TokenStream::new(),
        }
    }

    /// Source text without spans of its own: `::literal::LiteralIn::<`. Every caller passes text that lexes.
    fn text(&mut self, s: &str) {
        if let Ok(ts) = s.parse::<TokenStream>() {
            self.ts.extend(ts);
        }
    }

    /// `$crate` and then `rest`.
    fn krate(&mut self, rest: &str) {
        self.ts.extend(self.krate.iter().cloned());
        self.text(rest);
    }

    fn literal(&mut self, lit: &grammar::Literal, side: &Side) {
        self.krate("::literal::LiteralIn::<");
        self.number(lit);
        self.unit(&lit.unit, side);
        self.text(">::NEW");
    }

    /// A point literal: `$crate::literal::LiteralAt::<N, E, Unit, Origin>::NEW`, the unit `literal::Steps` for a
    /// number in the origin's own step.
    fn point(&mut self, lit: &grammar::Literal, at: &At, side: &Side) {
        self.krate("::literal::LiteralAt::<");
        self.number(lit);
        self.unit(&lit.unit, side);
        self.text(",");
        match at {
            At::Symbol { path, group, id } => self.marker(path, group, *id, side),
            At::Type(ids) => self.tokens(ids, side),
        }
        self.text(">::NEW");
    }

    /// `{N}, {E},`
    fn number(&mut self, lit: &grammar::Literal) {
        let mut n = TokenStream::new();
        n.extend([TokenTree::Literal(Literal::i64_suffixed(lit.num))]);
        self.ts
            .extend([TokenTree::Group(Group::new(Delimiter::Brace, n))]);
        self.text(",");
        let mut e = TokenStream::new();
        e.extend([TokenTree::Literal(Literal::i16_suffixed(lit.exp))]);
        self.ts
            .extend([TokenTree::Group(Group::new(Delimiter::Brace, e))]);
        self.text(",");
    }

    /// `$crate::__q_{group}!("`symbol`"; path)`, spanned at the symbol's token.
    fn marker(&mut self, path: &str, group: &str, id: usize, side: &Side) {
        let span = side.span(id);
        self.krate(&format!("::__q_{group}!"));
        let mut inner = TokenStream::new();
        let mut symbol = Literal::string(&format!("`{}`", side.text(id)));
        symbol.set_span(span);
        inner.extend([
            TokenTree::Literal(symbol),
            TokenTree::Punct(alone(';', span)),
        ]);
        if let Ok(ts) = path.parse::<TokenStream>() {
            inner.extend(ts.into_iter().map(|mut t| {
                t.set_span(span);
                t
            }));
        }
        let mut g = Group::new(Delimiter::Parenthesis, inner);
        g.set_span(span);
        self.ts.extend([TokenTree::Group(g)]);
    }

    /// The user's own tokens, a type.
    fn tokens(&mut self, ids: &[usize], side: &Side) {
        for id in ids {
            if let Some(Some(t)) = side.trees.get(*id) {
                self.ts.extend([t.clone()]);
            }
        }
    }

    /// As [literal](Self::literal), but the type `LiteralIn<N, E, Unit>` alone, no turbofish and no `::NEW`: what
    /// `fixed!(100 m/s)` names against `AtLiteral`, the same shape `fixed!(x in S)` already writes by hand.
    fn literal_type(&mut self, lit: &grammar::Literal, side: &Side) {
        self.krate("::literal::LiteralIn<");
        let mut n = TokenStream::new();
        n.extend([TokenTree::Literal(Literal::i64_suffixed(lit.num))]);
        self.ts
            .extend([TokenTree::Group(Group::new(Delimiter::Brace, n))]);
        self.text(",");
        let mut e = TokenStream::new();
        e.extend([TokenTree::Literal(Literal::i16_suffixed(lit.exp))]);
        self.ts
            .extend([TokenTree::Group(Group::new(Delimiter::Brace, e))]);
        self.text(",");
        self.unit(&lit.unit, side);
        self.text(">");
    }

    fn unit(&mut self, u: &Unit, side: &Side) {
        match u {
            Unit::Marker {
                path, group, id, ..
            } => self.marker(path, group, *id, side),
            Unit::One => self.krate("::literal::Unitless"),
            Unit::Steps => self.krate("::literal::Steps"),
            Unit::Type(ids) => self.tokens(ids, side),
            Unit::Mul(a, b) => match b.as_ref() {
                Unit::Pow(x, n) if *n < 0 => {
                    self.pair("Per", a, &Unit::Pow(x.clone(), n.saturating_neg()), side);
                }
                _ => self.pair("Times", a, b, side),
            },
            Unit::Div(a, b) => match b.as_ref() {
                Unit::Pow(x, n) if *n < 0 => {
                    self.pair("Times", a, &Unit::Pow(x.clone(), n.saturating_neg()), side);
                }
                _ => self.pair("Per", a, b, side),
            },
            Unit::Pow(x, n) if *n < 0 => {
                self.krate("::units::Per<");
                self.krate("::literal::Unitless");
                self.text(",");
                self.unit(&Unit::Pow(x.clone(), n.saturating_neg()), side);
                self.text(">");
            }
            Unit::Pow(x, n) => {
                // x^3 is Times<Times<x, x>, x>
                for _ in 1..*n {
                    self.krate("::units::Times<");
                }
                self.unit(x, side);
                for _ in 1..*n {
                    self.text(",");
                    self.unit(x, side);
                    self.text(">");
                }
            }
        }
    }

    /// `$crate::units::{name}<a, b>`
    fn pair(&mut self, name: &str, a: &Unit, b: &Unit, side: &Side) {
        self.krate(&format!("::units::{name}<"));
        self.unit(a, side);
        self.text(",");
        self.unit(b, side);
        self.text(">");
    }
}
