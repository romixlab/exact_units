//! The proc-macro behind exact_units' `q!`, the literal that names its unit: `speed < q!(100 m/s)`. Use it through
//! exact_units (`exact_units::q`), which passes its own path in; this crate has no dependencies and no API of its
//! own.
//!
//! The logic is in `pure`, plain Rust over a copy of the tokens; this file converts the compiler's tokens into that
//! copy and the result back, keeping each token's span, so an error points at the symbol it is about.

use proc_macro::{Delimiter, Group, Ident, Literal, Punct, Spacing, Span, TokenStream, TokenTree};

mod pure;

use pure::grammar::{self, Unit};
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
            out.literal(&lit, &side);
            out.ts
        }
        Err(e) => error(&e, &side),
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

/// `::core::compile_error!("q!: ..")` at the token the error is about.
fn error(e: &Error, side: &Side) -> TokenStream {
    let span = side.span(e.id);
    let mut msg = Literal::string(&format!("q!: {}", e.msg));
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
        self.text(">::NEW");
    }

    fn unit(&mut self, u: &Unit, side: &Side) {
        match u {
            Unit::Marker {
                path, group, id, ..
            } => {
                let span = side.span(*id);
                self.krate(&format!("::__q_{group}!"));
                let mut inner = TokenStream::new();
                let mut symbol = Literal::string(&format!("`{}`", side.text(*id)));
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
            Unit::One => self.krate("::literal::Unitless"),
            Unit::Type(ids) => {
                for id in ids {
                    if let Some(Some(t)) = side.trees.get(*id) {
                        self.ts.extend([t.clone()]);
                    }
                }
            }
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
