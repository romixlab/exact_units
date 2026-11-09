//! One walk over a unit tree: symbol, type name, dimension, factor, comparison and the unit after a number are
//! written once, for [UnitExpr], [SIExpr](crate::SIExpr) and its owned form, and for a client's own
//! unit model (`wire_weaver_client`'s `DynUnit`): a unit known at run time or only from introspection data is shown
//! exactly like the same unit known from a type.
//!
//! A unit model implements [Tree], the functions here do the rest:
//!
//! ```
//! use exact_units::tree::{self, Node, Tree};
//!
//! /// A unit as some client keeps it: names it only knows, and quotients of them.
//! enum MyUnit {
//!     Name(&'static str),
//!     Per(Box<MyUnit>, Box<MyUnit>),
//! }
//!
//! impl<'a> Tree<'a> for &'a MyUnit {
//!     fn node(&self) -> Option<Node<'a, Self>> {
//!         Some(match *self {
//!             MyUnit::Name(n) => Node::Named { crate_name: None, name: n, symbol: n, names: None, dim: None, scale: None },
//!             MyUnit::Per(a, b) => Node::Div(&**a, &**b),
//!         })
//!     }
//! }
//!
//! let u = MyUnit::Per(Box::new(MyUnit::Name("rpm")), Box::new(MyUnit::Name("s")));
//! assert_eq!(tree::symbol(&&u).to_string(), "rpm/s");
//! assert_eq!(tree::name(&&u).to_string(), "Per<rpm, s>");
//! assert_eq!(tree::dim(&&u), None);
//! ```

use crate::unit::{Dim, write_superscript};
use crate::{Magnitude, MarkerId, UnitNames, UnitTable};
use crate::{Prefix, UnitExpr};
use core::fmt;

/// Deepest nesting of a unit tree that is followed. A run-time unit comes from another device, so its depth is
/// bounded here and not by the stack: a deeper part is shown as `…` and has no dimension or factor.
///
/// A typed unit is never deeper: [Per](crate::units::Per), [Times](crate::units::Times) and
/// [Sqrt](crate::units::Sqrt) count their levels ([Unit::DEPTH](crate::Unit::DEPTH)), and one of more than 32 is a
/// compile error where its tree is used:
///
/// ```compile_fail
/// # #[cfg(feature = "quantities")] {
/// use exact_units::Unit;
/// use exact_units::units::{Meter, Per, Times};
///
/// // two levels per step over Meter's one, the dimension stays m
/// type L3 = Times<Per<Meter, Meter>, Meter>;
/// type L9 = Times<Per<Times<Per<Times<Per<L3, Meter>, Meter>, Meter>, Meter>, Meter>, Meter>;
/// type L15 = Times<Per<Times<Per<Times<Per<L9, Meter>, Meter>, Meter>, Meter>, Meter>, Meter>;
/// type L21 = Times<Per<Times<Per<Times<Per<L15, Meter>, Meter>, Meter>, Meter>, Meter>, Meter>;
/// type L27 = Times<Per<Times<Per<Times<Per<L21, Meter>, Meter>, Meter>, Meter>, Meter>, Meter>;
/// type L33 = Times<Per<Times<Per<Times<Per<L27, Meter>, Meter>, Meter>, Meter>, Meter>, Meter>;
/// let _ = <L33 as Unit>::EXPR;
/// # }
/// # #[cfg(not(feature = "quantities"))]
/// # compile_error!("the example needs Meter");
/// ```
pub const MAX_UNIT_DEPTH: usize = 32;

/// One node of a unit tree, its children as `E`.
// A node lives for one step of a walk on the stack; boxing the named variant would need alloc
#[expect(
    clippy::large_enum_variant,
    reason = "one node on the stack per step, boxing needs alloc"
)]
#[derive(Debug)]
pub enum Node<'a, E> {
    /// A named unit. `dim` and `scale` are `None` for a unit known by its name only (a user's marker that a client
    /// sees in introspection data, an [SIExpr](crate::SIExpr)'s `Ref` leaf): it is shown by name, without a dimension
    /// or a conversion to SI. With its crate it is a reference by [MarkerId], which [Resolved] looks up in a table.
    Named {
        /// The crate that declares the marker ([MarkerId]), `None` for a unit a client knows by
        /// name only.
        crate_name: Option<&'a str>,
        /// The marker's name: `Volt`.
        name: &'a str,
        /// As written after a number: `V`; a unit known by name only has its name here.
        symbol: &'a str,
        /// How it is written in other notations and read from text, `None` for a unit known by name only, or by its
        /// symbol only (a client's own unit model): [Notation::Ascii](crate::Notation::Ascii) transliterates the
        /// symbol, [Notation::Ucum](crate::Notation::Ucum) has no code for it.
        names: Option<UnitNames<'a>>,
        /// The unit's dimension, `None` for a unit known by name only.
        dim: Option<Dim<'a>>,
        /// The exact factor to coherent SI, `None` for a unit known by name only.
        scale: Option<Magnitude>,
    },
    /// `A · B`
    Mul(E, E),
    /// `A / B`
    Div(E, E),
    /// `A` to an integer power
    Pow(E, i8),
    /// `√A`, a dimension with the root flag (`V/√Hz`)
    Sqrt(E),
}

impl<'a, E> Node<'a, E> {
    /// The same node with its children turned into another tree's (a named node has none).
    pub fn map<F>(self, mut f: impl FnMut(E) -> F) -> Node<'a, F> {
        match self {
            Node::Named {
                crate_name,
                name,
                symbol,
                names,
                dim,
                scale,
            } => Node::Named {
                crate_name,
                name,
                symbol,
                names,
                dim,
                scale,
            },
            Node::Mul(a, b) => Node::Mul(f(a), f(b)),
            Node::Div(a, b) => Node::Div(f(a), f(b)),
            Node::Pow(a, n) => Node::Pow(f(a), n),
            Node::Sqrt(a) => Node::Sqrt(f(a)),
        }
    }
}

/// A unit tree: [UnitExpr] (`&'static` children), [SIExpr](crate::SIExpr) (children decoded on the
/// way), or a client's own model. Children are `Self`, usually a reference into the tree.
pub trait Tree<'a>: Sized {
    /// The node, `None` when it can't be read (children that don't decode). Its names live for `'a`, so a
    /// [dim] of the tree can name counted bases after the walk.
    fn node(&self) -> Option<Node<'a, Self>>;
}

/// The symbol as written after a number: `V`, `N·m/(rad/s)`, `(rad/s)·(rad/s)`, `m²`.
pub fn symbol<'t, E: Tree<'t>>(e: &E) -> impl fmt::Display + '_ {
    Show(e, |e, f, depth| write_symbol(*e, f, depth))
}

/// The type name: `Volt`, `Per<Torque, AngularVelocity>`, `Pow<Meter, 2>`.
pub fn name<'t, E: Tree<'t>>(e: &E) -> impl fmt::Display + '_ {
    Show(e, |e, f, depth| write_name(*e, f, depth))
}

/// [name] of a tree that is a reference itself (`&UnitExpr`), for an `impl Display` that outlives the call.
pub(crate) fn name_of<'t, E: Tree<'t>>(e: E) -> impl fmt::Display {
    Show(e, write_name)
}

/// [symbol] of a tree held by value, for an `impl Display` that outlives the call.
#[cfg(feature = "shrink_wrap")]
pub(crate) fn symbol_of<'t, E: Tree<'t>>(e: E) -> impl fmt::Display {
    Show(e, |e, f, depth| write_symbol(e, f, depth))
}

/// SI exponents of the whole tree; `None` when an exponent leaves -8..=7, a part is known by name only or doesn't
/// decode, or the tree is deeper than [MAX_UNIT_DEPTH].
pub fn dim<'t, E: Tree<'t>>(e: &E) -> Option<Dim<'t>> {
    dim_at(e, 0)
}

/// Factor to coherent SI of the whole tree, as a float; `None` like [dim], and where a scale keeps a measured constant
/// (μ₀, G), which has no value without a named set ([Magnitude::resolve](crate::Magnitude::resolve)).
pub fn factor<'t, E: Tree<'t>>(e: &E) -> Option<f64> {
    factor_at(e, 0)
}

/// The same tree: crates, names, symbols, dimensions and scales of every named unit, and the same shape.
pub fn same<'s, 't, A: Tree<'s>, B: Tree<'t>>(a: &A, b: &B) -> bool {
    same_at(a, b, 0)
}

impl Tree<'static> for &UnitExpr {
    fn node(&self) -> Option<Node<'static, Self>> {
        Some(match **self {
            UnitExpr::Named(def) => Node::Named {
                crate_name: Some(def.id.crate_name),
                name: def.id.name,
                symbol: def.names.symbol,
                names: Some(def.names),
                dim: Some(def.dim),
                scale: Some(def.scale),
            },
            UnitExpr::Mul(a, b) => Node::Mul(a, b),
            UnitExpr::Div(a, b) => Node::Div(a, b),
            UnitExpr::Pow(a, n) => Node::Pow(a, n),
            UnitExpr::Sqrt(a) => Node::Sqrt(a),
        })
    }
}

/// A tree whose leaves known by name only are looked up in a table: a leaf with its crate and no dimension (an
/// [SIExpr](crate::SIExpr)'s `Ref`) that the table lists walks as the table's definition, one it doesn't list stays
/// known by name only. Every walk here reads it like any other tree.
///
/// ```
/// # #[cfg(all(feature = "shrink_wrap", feature = "electrical"))] {
/// use exact_units::tree::{self, Resolved};
/// use exact_units::units::{self, Volt};
/// use exact_units::{SIExpr, Unit};
///
/// let id = Volt::EXPR.def().unwrap().id;
/// let r = SIExpr::Ref(id);
/// assert_eq!(tree::dim(&r), None);
/// assert_eq!(tree::symbol(&r).to_string(), "Volt");
/// assert_eq!(tree::dim(&Resolved::new(r.clone(), &units::TABLE)), Some(Volt::DIM));
/// assert_eq!(tree::symbol(&Resolved::new(r, &units::TABLE)).to_string(), "V");
/// # }
/// ```
#[derive(Debug)]
pub struct Resolved<'a, E, T: ?Sized> {
    tree: E,
    table: &'a T,
}

impl<'a, E, T: ?Sized> Resolved<'a, E, T> {
    /// `tree` with its references looked up in `table`.
    pub const fn new(tree: E, table: &'a T) -> Self {
        Resolved { tree, table }
    }
}

impl<E: Clone, T: ?Sized> Clone for Resolved<'_, E, T> {
    fn clone(&self) -> Self {
        Resolved {
            tree: self.tree.clone(),
            table: self.table,
        }
    }
}

impl<'a, E: Tree<'a>, T: UnitTable + ?Sized> Tree<'a> for Resolved<'a, E, T> {
    fn node(&self) -> Option<Node<'a, Self>> {
        let table = self.table;
        let node = self.tree.node()?;
        if let Node::Named {
            crate_name: Some(crate_name),
            name,
            dim: None,
            ..
        } = node
            && let Some(def) = table.unit(MarkerId { crate_name, name })
        {
            return Some(Node::Named {
                crate_name: Some(def.id.crate_name),
                name: def.id.name,
                symbol: def.names.symbol,
                names: Some(def.names),
                dim: Some(def.dim),
                scale: Some(def.scale),
            });
        }
        Some(node.map(|tree| Resolved { tree, table }))
    }
}

/// Needs parentheses when it stands next to `·` or `/`.
fn is_compound<E>(node: &Node<'_, E>) -> bool {
    match node {
        Node::Named { symbol, .. } => symbol.contains(['/', '·']),
        // `m²`, `(m/s)²`, `√Hz`: the power and the root bind tighter than `·` and `/`
        Node::Pow(..) | Node::Sqrt(..) => false,
        _ => true,
    }
}

/// The symbol: `V`, `N·m/(rad/s)`, `(rad/s)·(rad/s)`, `m²`.
#[expect(
    clippy::arithmetic_side_effects,
    reason = "depth < MAX_UNIT_DEPTH, checked first"
)]
fn write_symbol<'t, E: Tree<'t>>(e: &E, f: &mut fmt::Formatter<'_>, depth: usize) -> fmt::Result {
    if depth >= MAX_UNIT_DEPTH {
        return f.write_str("…");
    }
    let Some(node) = e.node() else {
        return f.write_str("?");
    };
    let side = |f: &mut fmt::Formatter<'_>, e: &E, parens: &dyn Fn(&Node<'_, E>) -> bool| {
        let parens = e.node().is_some_and(|n| parens(&n));
        if parens {
            f.write_str("(")?;
        }
        write_symbol(e, f, depth + 1)?;
        if parens {
            f.write_str(")")?;
        }
        Ok(())
    };
    match &node {
        Node::Named { symbol, .. } => f.write_str(symbol),
        Node::Mul(a, b) => {
            let parens = |n: &Node<'_, E>| is_compound(n) && !matches!(n, Node::Mul(..));
            side(f, a, &parens)?;
            f.write_str("·")?;
            side(f, b, &parens)
        }
        Node::Div(a, b) => {
            side(f, a, &|n| matches!(n, Node::Div(..)))?;
            f.write_str("/")?;
            side(f, b, &is_compound)
        }
        Node::Pow(a, n) => {
            side(f, a, &|n| is_compound(n) || matches!(n, Node::Pow(..)))?;
            write_superscript(f, i32::from(*n))
        }
        Node::Sqrt(a) => {
            f.write_str("√")?;
            side(f, a, &|n| {
                is_compound(n) || matches!(n, Node::Pow(..) | Node::Sqrt(..))
            })
        }
    }
}

/// The type name: `Volt`, `Per<Torque, AngularVelocity>`, `Pow<Meter, 2>`.
#[expect(
    clippy::arithmetic_side_effects,
    reason = "depth < MAX_UNIT_DEPTH, checked first"
)]
fn write_name<'t, E: Tree<'t>>(e: &E, f: &mut fmt::Formatter<'_>, depth: usize) -> fmt::Result {
    if depth >= MAX_UNIT_DEPTH {
        return f.write_str("…");
    }
    let Some(node) = e.node() else {
        return f.write_str("?");
    };
    let pair = |f: &mut fmt::Formatter<'_>, ty: &str, a: &E, b: &E| {
        write!(f, "{ty}<")?;
        write_name(a, f, depth + 1)?;
        f.write_str(", ")?;
        write_name(b, f, depth + 1)?;
        f.write_str(">")
    };
    match &node {
        Node::Named { name, .. } => f.write_str(name),
        Node::Mul(a, b) => pair(f, "Times", a, b),
        Node::Div(a, b) => pair(f, "Per", a, b),
        Node::Pow(a, n) => {
            f.write_str("Pow<")?;
            write_name(a, f, depth + 1)?;
            write!(f, ", {n}>")
        }
        Node::Sqrt(a) => {
            f.write_str("Sqrt<")?;
            write_name(a, f, depth + 1)?;
            f.write_str(">")
        }
    }
}

/// SI exponents of the whole tree, `None` when an exponent leaves -8..=7, a part doesn't decode or it is too deep.
#[expect(
    clippy::arithmetic_side_effects,
    reason = "depth < MAX_UNIT_DEPTH, checked first"
)]
fn dim_at<'t, E: Tree<'t>>(e: &E, depth: usize) -> Option<Dim<'t>> {
    if depth >= MAX_UNIT_DEPTH {
        return None;
    }
    let d = depth + 1;
    match e.node()? {
        Node::Named { dim, .. } => dim,
        Node::Mul(a, b) => dim_at(&a, d)?.checked_mul(dim_at(&b, d)?),
        Node::Div(a, b) => dim_at(&a, d)?.checked_div(dim_at(&b, d)?),
        Node::Pow(a, n) => dim_at(&a, d)?.checked_pow(n),
        Node::Sqrt(a) => dim_at(&a, d)?.checked_sqrt(),
    }
}

/// Factor to coherent SI of the whole tree, `None` when a part doesn't decode or it is too deep.
#[expect(
    clippy::arithmetic_side_effects,
    reason = "depth < MAX_UNIT_DEPTH, checked first"
)]
fn factor_at<'t, E: Tree<'t>>(e: &E, depth: usize) -> Option<f64> {
    if depth >= MAX_UNIT_DEPTH {
        return None;
    }
    let d = depth + 1;
    match e.node()? {
        Node::Named { scale, .. } => scale?.checked_to_f64(),
        Node::Mul(a, b) => Some(factor_at(&a, d)? * factor_at(&b, d)?),
        Node::Div(a, b) => Some(factor_at(&a, d)? / factor_at(&b, d)?),
        Node::Pow(a, n) => Some(crate::num::powi_f64(factor_at(&a, d)?, n)),
        Node::Sqrt(a) => Some(crate::num::sqrt_f64(factor_at(&a, d)?)),
    }
}

/// The exact factor to coherent SI of the whole tree, as [UnitExpr::magnitude]: `None` like [dim], or for a product a
/// [Magnitude] doesn't hold.
pub fn magnitude<'t, E: Tree<'t>>(e: &E) -> Option<Magnitude> {
    magnitude_at(e, 0)
}

#[expect(
    clippy::arithmetic_side_effects,
    reason = "depth < MAX_UNIT_DEPTH, checked first"
)]
fn magnitude_at<'t, E: Tree<'t>>(e: &E, depth: usize) -> Option<Magnitude> {
    if depth >= MAX_UNIT_DEPTH {
        return None;
    }
    let d = depth + 1;
    match e.node()? {
        Node::Named { scale, .. } => scale,
        Node::Mul(a, b) => magnitude_at(&a, d)?.checked_mul(magnitude_at(&b, d)?).ok(),
        Node::Div(a, b) => magnitude_at(&a, d)?.checked_div(magnitude_at(&b, d)?).ok(),
        Node::Pow(a, n) => {
            let a = magnitude_at(&a, d)?;
            let mut r = Magnitude::ONE;
            for _ in 0..n.unsigned_abs() {
                r = r.checked_mul(a).ok()?;
            }
            if n < 0 {
                Magnitude::ONE.checked_div(r).ok()
            } else {
                Some(r)
            }
        }
        Node::Sqrt(a) => magnitude_at(&a, d)?.checked_sqrt().ok(),
    }
}

/// A named unit without a symbol ([Ratio](crate::units::Ratio)), or products, quotients and powers of such
/// (`Ratio · Ratio`, a Q15 squared): a prefix is written as %, ‰, ppm, ppb, no prefix as nothing.
pub fn is_ratio<'t, E: Tree<'t>>(e: &E) -> bool {
    ratio_at(e, 0)
}

#[expect(
    clippy::arithmetic_side_effects,
    reason = "depth < MAX_UNIT_DEPTH, checked first"
)]
fn ratio_at<'t, E: Tree<'t>>(e: &E, depth: usize) -> bool {
    if depth >= MAX_UNIT_DEPTH {
        return false;
    }
    match e.node() {
        Some(Node::Named { symbol, .. }) => symbol.is_empty(),
        Some(Node::Mul(a, b) | Node::Div(a, b)) => {
            ratio_at(&a, depth + 1) && ratio_at(&b, depth + 1)
        }
        Some(Node::Pow(a, _) | Node::Sqrt(a)) => ratio_at(&a, depth + 1),
        None => false,
    }
}

/// A named unit (no parentheses after a prefix).
/// Whether a prefix is written straight before this unit's symbol (`mV`, `mm/s`): a named unit whose symbol takes one,
/// see [prefix_binds].
pub(crate) fn takes_bare_prefix<'t, E: Tree<'t>>(e: &E) -> bool {
    matches!(e.node(), Some(Node::Named { symbol, .. }) if prefix_binds(symbol))
}

/// Whether a prefix written straight before `symbol` means the prefix times the unit: `mV`, `mm/s`, `mN·m`. Not where
/// the first factor has an exponent (`mm²` is (mm)², 10⁻⁶ m², and `mm³/s` is 10⁻⁹ m³/s) or already has a prefix
/// (`kg/m³`, `kgf`): such a unit takes it on the whole of it in parentheses, `m(m²)`, as a compound unit does, unless
/// [moved] finds the form people write.
pub(crate) fn prefix_binds(symbol: &str) -> bool {
    let lead = symbol.split(['/', '·']).next().unwrap_or(symbol);
    !lead.starts_with("kg")
        && !lead.contains(['⁰', '¹', '²', '³', '⁴', '⁵', '⁶', '⁷', '⁸', '⁹', '⁻'])
}

/// The powers of ten that move inside a square metre: dm², cm², mm², µm², nm², km². Other prefixes aren't used on a
/// power (hm² is 10⁴ m², but nobody reads it), so such a scale stays on the whole unit.
const AREA_PREFIXES: [i8; 6] = [-1, -2, -3, -6, -9, 3];

/// The powers of ten that move inside a cubic metre: dm³, cm³, mm³, µm³, nm³. A km³ is rare, a dam³ rarer.
const VOLUME_PREFIXES: [i8; 5] = [-1, -2, -3, -6, -9];

/// A scale on a unit with a power of the metre, as people write it. The unit and its [Magnitude](crate::Magnitude)
/// don't change, only the text.
#[derive(Copy, Clone, Debug, PartialEq)]
pub(crate) enum Moved<'a> {
    /// The prefix binds to the metre and the power applies to the prefixed unit (SI Brochure, 9th ed., §3): 10⁻⁶ m³
    /// with `Centi` here is `cm³`.
    Inside(Prefix),
    /// A litre and what follows the metres in the symbol: `mL`, `/s`.
    Litre(&'static str, &'a str),
    /// A quotient written out: `µ`, `S`, `c`, `m` is `µS/cm`, and `""`, `dB`, `k`, `m` is `dB/km`.
    Quotient(&'static str, &'a str, &'static str, &'a str),
}

/// How `prefix` on `unit` is written when it doesn't go on the whole of it: a power of ten `k` on a unit whose first
/// factor is m² or m³ moves inside as `10^(k/n)` where `n` divides `k` and that prefix is one people use with the power
/// ([AREA_PREFIXES], [VOLUME_PREFIXES]). `None` otherwise: [write_unit] then keeps the prefix on the whole unit,
/// `k(m³)`, `da(m²)`, or the scale after the number where it has no symbol, `·10⁴ m²`. Works for a scale without a
/// prefix's name too: 10⁻⁴ m² is `cm²`.
///
/// Metres only: `mV²` and `mft³` keep their form, and so does a unit known by name only (no dimension says its `m` is
/// a metre).
///
/// A quotient goes on through [on_quotient] when no power of the metre comes first.
pub(crate) fn moved<'t, E: Tree<'t>>(prefix: Prefix, unit: &E) -> Option<Moved<'t>> {
    match inside(prefix, unit) {
        Some(inner) => prefix
            .pow10()
            .and_then(|k| litre(k, unit))
            .or(Some(Moved::Inside(inner))),
        None => on_quotient(prefix, unit),
    }
}

/// The prefix on the metre that stands for `prefix` on `unit`, see [moved]: 10⁻⁶ m³ is `Centi`.
pub(crate) fn inside<'t, E: Tree<'t>>(prefix: Prefix, unit: &E) -> Option<Prefix> {
    let k = prefix.pow10()?;
    let n = metre_power(unit, 0)?;
    if k.checked_rem(n)? != 0 {
        return None;
    }
    let inner = k.checked_div(n)?;
    let allowed: &[i8] = if n == 2 {
        &AREA_PREFIXES
    } else {
        &VOLUME_PREFIXES
    };
    if allowed.contains(&inner) {
        Prefix::si_from_pow10(inner)
    } else {
        None
    }
}

/// `L`, `mL` and `µL` for the markers `Volume` and `VolumetricFlow` themselves. Any other unit of the dimension was
/// spelled by its author (`Per<Volume, Second>`, a kind of its own, `Litre`, which has its own prefixes) and keeps its
/// spelling.
fn litre<'t, E: Tree<'t>>(k: i8, unit: &E) -> Option<Moved<'t>> {
    let Node::Named {
        crate_name: Some(env!("CARGO_CRATE_NAME")),
        name,
        symbol,
        scale: Some(Magnitude::ONE),
        ..
    } = unit.node()?
    else {
        return None;
    };
    if !matches!(
        (name, symbol),
        ("Volume", "m³") | ("VolumetricFlow", "m³/s")
    ) {
        return None;
    }
    let litres = match k {
        -3 => "L",
        -6 => "mL",
        -9 => "µL",
        _ => return None,
    };
    Some(Moved::Litre(litres, symbol.strip_prefix("m³")?))
}

/// A quotient with a plain numerator: a marker's `S/m` or a tree's `Per<Decibel, Meter>`, taken apart.
#[derive(Copy, Clone, Debug)]
pub(crate) struct Quotient<'a> {
    /// The numerator's symbol: `S`, `dB`, `kg`.
    pub(crate) num: &'a str,
    /// The denominator's symbol where it is one plain factor; `√Hz` or `(rad/s)` is `None`.
    pub(crate) den: Option<&'a str>,
    /// Whether the numerator is an SI unit, so a prefix before it, or a kilogram's in place of its `k`, means what it says.
    pub(crate) takes_prefix: bool,
    /// The marker of the whole quotient, for a tree `""`: `Density`.
    marker: &'a str,
}

/// [Quotient] of `unit`. `None` for what isn't one, or has a numerator with a power, a compound or nothing in it
/// (`m²/s`, `N·m/s`, `/K`), and for a unit known by name only: nothing says what its `m` is.
pub(crate) fn quotient<'t, E: Tree<'t>>(unit: &E) -> Option<Quotient<'t>> {
    let (num, den, takes_prefix, marker) = match unit.node()? {
        Node::Named {
            name,
            symbol,
            dim: Some(_),
            scale: Some(Magnitude::ONE),
            ..
        } => {
            let (num, den) = symbol.split_once('/')?;
            (num, plain(den), true, name)
        }
        Node::Div(a, b) => {
            let Node::Named {
                symbol,
                dim: Some(_),
                scale: Some(scale),
                ..
            } = a.node()?
            else {
                return None;
            };
            if matches!(b.node()?, Node::Named { dim: None, .. }) {
                return None;
            }
            let si = scale.as_pow10().is_some();
            (symbol, den_symbol(&b), si, "")
        }
        _ => return None,
    };
    let plain_num = !num.is_empty() && !num.contains(['/', '·', '(']);
    plain_num.then_some(Quotient {
        num,
        den,
        takes_prefix: takes_prefix && (num == "kg" || prefix_binds(num)) && !is_level(num),
        marker,
    })
}

/// The symbol of a denominator that is one plain factor: a tree's `m` has to be a metre, `s` a second and `Hz` a hertz.
fn den_symbol<'t, E: Tree<'t>>(e: &E) -> Option<&'t str> {
    match e.node()? {
        Node::Named { symbol, dim, .. } => {
            let dim = dim?;
            let known = match symbol {
                "m" => dim.m == 1,
                "s" => dim.s == 1,
                "Hz" => dim.s == -1,
                _ => true,
            };
            plain(symbol).filter(|_| known)
        }
        Node::Pow(a, n @ (2 | 3)) => match a.node()? {
            Node::Named {
                symbol: "m",
                dim: Some(dim),
                ..
            } if dim.m == 1 => Some(if n == 2 { "m²" } else { "m³" }),
            _ => None,
        },
        _ => None,
    }
}

/// `s` where it is one factor, no `/`, `·` or parenthesis in it.
fn plain(s: &str) -> Option<&str> {
    (!s.contains(['/', '·', '('])).then_some(s)
}

/// `dB` and `Np`: on a quotient the prefix goes on what is under them, [on_quotient].
fn is_level(symbol: &str) -> bool {
    matches!(symbol, "dB" | "Np")
}

/// The power of the denominator's own unit that a prefix on it is raised to, and the prefixes used on it: the powers of
/// ten of `km`, `cm`, `mm`, `µm`, `nm`; a `hm` isn't read or written. `None` for a denominator that takes none.
pub(crate) fn den_prefixes(den: &str) -> Option<(i8, &'static [i8])> {
    Some(match den {
        "m" => (1, &[3, -2, -3, -6, -9]),
        "m²" => (2, &AREA_PREFIXES),
        "m³" => (3, &VOLUME_PREFIXES),
        "Hz" => (1, &[3, 6, 9]),
        "s" | "L" => (1, &[-3, -6, -9]),
        _ => return None,
    })
}

/// A prefix written before the numerator with an engineering power of ten, `10^a`: the case of [on_quotient] where the
/// denominator takes one too.
fn engineering(a: i8) -> Option<&'static str> {
    let whole = a.checked_rem(3)? == 0 && (-12..=12).contains(&a);
    whole
        .then(|| Prefix::si_from_pow10(a))
        .flatten()
        .map(|p| p.symbol())
}

/// A power of ten on a quotient, as people write it. The first of these that fits; the book's "Prefixes on quotients"
/// has the table of cases.
///
/// 1. The markers `Density` and `MolarConcentration` are per litre: `mg/L`, `µmol/L`, `kg/L`. As with `L/s`, a unit
///    spelled another way keeps its metres.
/// 2. `dB` and `Np` take no prefix on a quotient, it goes into the denominator ([den_prefixes]): `dB/km`, `Np/km`,
///    `dB/MHz`, `dB/ms`. `dB/100 m` has no prefix, so 10⁻² keeps its form.
/// 3. Over `m` and `m²`, `cm` or `cm²` where that leaves an engineering prefix on the numerator: `µS/cm`, `kV/cm`,
///    `mW/cm²`.
/// 4. 10⁶ over `m²` is `mm²`: `A/mm²`.
/// 5. The numerator takes the prefix: `mV/m`, `nV/√Hz`, and `g` stands for `kg` from `g/m²` down to `ng/s`.
///
/// `None` otherwise, for a binary prefix, and for a marker where rule 5 is what [write_unit] writes anyway.
fn on_quotient<'t, E: Tree<'t>>(prefix: Prefix, unit: &E) -> Option<Moved<'t>> {
    let k = prefix.pow10().filter(|k| *k != 0)?;
    let q = quotient(unit)?;
    if let Some(m) = per_litre(k, &q) {
        return Some(m);
    }
    if is_level(q.num) {
        let den = q.den?;
        let (_, used) = den_prefixes(den).filter(|_| den != "L")?;
        let p = used
            .contains(&k.checked_neg()?)
            .then(|| Prefix::si_from_pow10(k.checked_neg()?))??;
        return Some(Moved::Quotient("", q.num, p.symbol(), den));
    }
    if !q.takes_prefix {
        return None;
    }
    // the gram is what a prefix goes on: 10⁻³ kg is g
    let (num, base) = if q.num == "kg" {
        ("g", k.checked_add(3)?)
    } else {
        (q.num, k)
    };
    if let Some(den) = q.den {
        // what cm or cm² takes off the scale
        let cm = match den {
            "m" => 2,
            "m²" => 4,
            _ => 0,
        };
        let per_cm = (cm != 0)
            .then(|| engineering(base.checked_sub(cm)?))
            .flatten();
        if let Some(head) = per_cm {
            return Some(Moved::Quotient(head, num, "c", den));
        }
        if den == "m²" && k == 6 {
            return Some(Moved::Quotient("", q.num, "m", den));
        }
        if q.num == "kg" {
            return (base <= 0)
                .then(|| engineering(base))?
                .map(|head| Moved::Quotient(head, num, "", den));
        }
    }
    (q.marker.is_empty() && q.num != "kg" && prefix.is_named()).then_some(Moved::Inside(prefix))
}

/// `Density` and `MolarConcentration` per litre, 10⁻¹² up to 10³ on the marker in steps of 10³.
fn per_litre<'t>(k: i8, q: &Quotient<'_>) -> Option<Moved<'t>> {
    let (num, shift) = match (q.marker, q.num, q.den) {
        ("Density", "kg", Some("m³")) => ("g", 0),
        ("MolarConcentration", "mol", Some("m³")) => ("mol", -3),
        _ => return None,
    };
    let head = (-12..=3)
        .contains(&k)
        .then(|| engineering(k.checked_add(shift)?))??;
    Some(Moved::Quotient(head, num, "", "L"))
}

/// `n` when the symbol of `e` starts with mⁿ, a metre, for `n` of 2 or 3, with nothing before it that a prefix would
/// have to go past (a parenthesis, as in `(m³/s)/s`).
#[expect(
    clippy::arithmetic_side_effects,
    reason = "depth < MAX_UNIT_DEPTH, checked first"
)]
fn metre_power<'t, E: Tree<'t>>(e: &E, depth: usize) -> Option<i8> {
    if depth >= MAX_UNIT_DEPTH {
        return None;
    }
    match e.node()? {
        Node::Named { symbol, dim, .. } => {
            let n = match symbol.split(['/', '·']).next()? {
                "m²" => 2,
                "m³" => 3,
                _ => return None,
            };
            (dim?.m == n).then_some(n)
        }
        Node::Pow(a, n @ (2 | 3)) => match a.node()? {
            Node::Named {
                symbol: "m",
                dim: Some(dim),
                ..
            } if dim.m == 1 => Some(n),
            _ => None,
        },
        // as write_symbol parenthesises them
        Node::Div(a, _) if !matches!(a.node()?, Node::Div(..)) => metre_power(&a, depth + 1),
        Node::Mul(a, _)
            if a.node()
                .is_some_and(|n| !is_compound(&n) || matches!(n, Node::Mul(..))) =>
        {
            metre_power(&a, depth + 1)
        }
        _ => None,
    }
}

#[expect(
    clippy::arithmetic_side_effects,
    reason = "depth < MAX_UNIT_DEPTH, checked first"
)]
fn same_at<'s, 't, A: Tree<'s>, B: Tree<'t>>(a: &A, b: &B, depth: usize) -> bool {
    if depth >= MAX_UNIT_DEPTH {
        return false;
    }
    let (Some(a), Some(b)) = (a.node(), b.node()) else {
        return false;
    };
    let d = depth + 1;
    match (a, b) {
        (
            Node::Named {
                crate_name: k1,
                name: n1,
                symbol: s1,
                names: _,
                dim: d1,
                scale: c1,
            },
            Node::Named {
                crate_name: k2,
                name: n2,
                symbol: s2,
                names: _,
                dim: d2,
                scale: c2,
            },
        ) => k1 == k2 && n1 == n2 && s1 == s2 && d1 == d2 && c1 == c2,
        (Node::Mul(a1, b1), Node::Mul(a2, b2)) | (Node::Div(a1, b1), Node::Div(a2, b2)) => {
            same_at(&a1, &a2, d) && same_at(&b1, &b2, d)
        }
        (Node::Pow(a1, n1), Node::Pow(a2, n2)) => n1 == n2 && same_at(&a1, &a2, d),
        (Node::Sqrt(a1), Node::Sqrt(a2)) => same_at(&a1, &a2, d),
        _ => false,
    }
}

/// The prefix and the unit's symbol, as written after a number: ` mV`, ` kg`, ` %` for a ratio in `Centi`,
/// ` k(N·m/(rad/s))` for a compound unit. A power of ten on square or cubic metres goes inside where people write it
/// so: ` mm²` for `Micro` on m², ` cm³/kg` for `Micro` on m³/kg, and the markers `Volume` and `VolumetricFlow` are in
/// litres from 10⁻³ down to 10⁻⁹ (` mL/s`). A scale on a quotient goes where people write it: ` dB/km`, ` mg/L`, ` µS/cm`.
pub fn write_unit<'t, E: Tree<'t>>(
    prefix: Prefix,
    unit: &E,
    f: &mut fmt::Formatter<'_>,
) -> fmt::Result {
    match moved(prefix, unit) {
        Some(Moved::Inside(p)) => {
            write!(f, " {}", p.symbol())?;
            return write_symbol(unit, f, 0);
        }
        Some(Moved::Litre(litres, rest)) => return write!(f, " {litres}{rest}"),
        Some(Moved::Quotient(head, num, under, den)) => {
            return write!(f, " {head}{num}/{under}{den}");
        }
        None => {}
    }
    if !prefix.is_named() {
        // a factor without a symbol: after the number, then the unit
        write!(f, "{prefix}")?;
        return write_unit(Prefix::Unit, unit, f);
    }
    if is_ratio(unit) {
        return match prefix {
            Prefix::Unit => Ok(()),
            Prefix::Centi => f.write_str(" %"),
            Prefix::Milli => f.write_str(" ‰"),
            Prefix::Micro => f.write_str(" ppm"),
            Prefix::Nano => f.write_str(" ppb"),
            p => match p.pow10() {
                Some(e) => write!(f, "e{e}"),
                None => write!(f, " {}", p.symbol()),
            },
        };
    }
    f.write_str(" ")?;
    match (prefix, takes_bare_prefix(unit)) {
        (Prefix::Unit, _) => write_symbol(unit, f, 0),
        (p, true) => {
            f.write_str(p.symbol())?;
            write_symbol(unit, f, 0)
        }
        (p, false) => {
            write!(f, "{}(", p.symbol())?;
            write_symbol(unit, f, 0)?;
            f.write_str(")")
        }
    }
}

/// Writes a tree with one of the functions above, for `impl Display` returns.
struct Show<T>(T, fn(&T, &mut fmt::Formatter<'_>, usize) -> fmt::Result);

impl<T> fmt::Display for Show<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        (self.1)(&self.0, f, 0)
    }
}
