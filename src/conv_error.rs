//! A readable message for a refused conversion, built at compile time (P2401#5nov-0954): names both units (their
//! type name and symbol from [UnitExpr]), both dimensions, and (for a dimension mismatch) the dimension left when
//! one divides the other, so a mismatch says what differs instead of only that something does. Used by
//! [Fixed::convert](crate::Fixed::convert) and its kin in `fixed.rs`, by `Exact`'s conversions in `exact.rs`, and
//! by a decimal literal's unit check in `literal.rs`.
//!
//! Every function here ends in `panic!`, never returns: the message is built and panicked with in one call, so its
//! buffer never has to outlive the function that builds it (a `const` buffer can't be borrowed into a `'static`
//! `&str` any other way on stable Rust).

use crate::dim::{TRUE_ORDER, base_symbol};
use crate::unit::superscript_digit;
use crate::{Dim, Exp, MarkerId, QuantityExpr, QuantityRef, Unit, UnitExpr};

/// Text written into a buffer on the stack, for a `panic!("{}", msg.as_str())` right after. A push that wouldn't
/// fit whole is dropped whole (never split a char in half: [as_str](Self::as_str) always decodes), so a cut-off
/// message is still readable. [Msg::CAP] holds the longest message built here, a missing bridge between two
/// products of units with its reason last (`tests/compile_fail/no_bridge.stderr` pins it whole).
struct Msg {
    bytes: [u8; Msg::CAP],
    len: usize,
}

impl Msg {
    const CAP: usize = 768;

    const fn new() -> Self {
        Msg {
            bytes: [0; Self::CAP],
            len: 0,
        }
    }

    #[expect(
        clippy::indexing_slicing,
        reason = "checked by the `if` just above: len < bytes.len()"
    )]
    #[expect(
        clippy::arithmetic_side_effects,
        reason = "len < bytes.len(), checked by the `if` just above, so len + 1 fits usize"
    )]
    const fn push_byte(mut self, b: u8) -> Self {
        if self.len < self.bytes.len() {
            self.bytes[self.len] = b;
            self.len += 1;
        }
        self
    }

    /// `s` whole, or not at all: dropped past [Self::CAP] rather than split, so a multi-byte char (`·`, `⁻`, `²`)
    /// is never cut in half and left at the end of the buffer for [as_str](Self::as_str) to fail to decode.
    #[expect(clippy::indexing_slicing, reason = "i < bytes.len(), the loop bound")]
    #[expect(
        clippy::arithmetic_side_effects,
        reason = "i < bytes.len(), the loop bound"
    )]
    const fn push(mut self, s: &str) -> Self {
        let bytes = s.as_bytes();
        let Some(end) = self.len.checked_add(bytes.len()) else {
            return self;
        };
        if end > self.bytes.len() {
            return self;
        }
        let mut i = 0;
        while i < bytes.len() {
            self = self.push_byte(bytes[i]);
            i += 1;
        }
        self
    }

    /// `e` in decimal digits, `-` before a negative one: a plain number, for a type name's `Pow<Meter, 2>`.
    const fn push_int(self, e: i32) -> Self {
        let msg = if e < 0 { self.push_byte(b'-') } else { self };
        push_exp_digits(msg, e.unsigned_abs(), false)
    }

    /// `e` in superscript digits, `⁻` before a negative one, as [Dim]'s `Display` writes an exponent.
    const fn push_superscript(self, e: i32) -> Self {
        let msg = if e < 0 { self.push("⁻") } else { self };
        push_exp_digits(msg, e.unsigned_abs(), true)
    }

    const fn as_str(&self) -> &str {
        match self.bytes.split_at_checked(self.len) {
            Some((head, _)) => match core::str::from_utf8(head) {
                Ok(s) => s,
                // push never writes part of a str, so the bytes written are whole strs end to end: always valid
                // utf8, this arm is unreachable in practice but a `str` here would be unsound without it
                Err(_) => "(a conversion message that did not decode)",
            },
            None => "(a conversion message past its own buffer)",
        }
    }
}

/// `i8` to `i32`, widening and exact: `const fn`, where `From` is not const on stable.
#[expect(
    clippy::as_conversions,
    reason = "const fn: From is not const on stable; i8 widens into i32 exactly"
)]
const fn widen(e: i8) -> i32 {
    e as i32
}

/// `e`'s digits, as plain ASCII (`superscript: false`) or superscript (`true`).
#[expect(
    clippy::arithmetic_side_effects,
    reason = "p * 10 only while p <= e / 10, so it stays at most e; p is at least 1, never a division by 0"
)]
#[expect(
    clippy::as_conversions,
    reason = "const fn: From is not const on stable; d is e / p % 10, a single digit 0..=9"
)]
const fn push_exp_digits(mut msg: Msg, e: u32, superscript: bool) -> Msg {
    let mut p: u32 = 1;
    while p <= e / 10 {
        p *= 10;
    }
    loop {
        let d = e / p % 10;
        msg = if superscript {
            msg.push(superscript_digit(d))
        } else {
            msg.push_byte(b'0' + d as u8)
        };
        if p == 1 {
            return msg;
        }
        p /= 10;
    }
}

/// The dimension as [Dim]'s `Display` writes it: `kg·m²·s⁻²·rad⁻¹`, `1` for a ratio; `…` for distinguished bases
/// read at run time, which a unit type's dimension never has.
#[expect(
    clippy::indexing_slicing,
    reason = "i < TRUE_ORDER.len() or exps.len(), the loop bounds, TRUE_ORDER's indices below MAX_TRUE_BASES, and \
              b[i] where i < b.len()"
)]
#[expect(clippy::arithmetic_side_effects, reason = "i below a loop bound")]
const fn push_dim(mut msg: Msg, d: Dim<'_>) -> Msg {
    let exps = d.exps();
    let mut first = true;
    let mut i = 0;
    let system = d.system();
    match system.def() {
        Some(def) if !def.is_si() => {
            let bases = crate::system::slice_of(&def.bases);
            while i < exps.len() {
                if !exps[i].is_zero() {
                    let symbol = match bases {
                        Some(b) if i < b.len() => b[i],
                        _ => "?",
                    };
                    (msg, first) = push_dim_part(msg, first, symbol, exps[i]);
                }
                i += 1;
            }
            if !first {
                msg = msg.push(" (").push(def.id.name).push(")");
            }
        }
        _ => {
            while i < TRUE_ORDER.len() {
                let (symbol, at) = TRUE_ORDER[i];
                if !exps[at].is_zero() {
                    (msg, first) = push_dim_part(msg, first, symbol, exps[at]);
                }
                i += 1;
            }
        }
    }
    let mut after = None;
    loop {
        match d.const_next_base(after) {
            Some(Some(b)) => {
                (msg, first) = push_dim_part(msg, first, base_symbol(b.base), b.exp);
                after = Some(b.base);
            }
            Some(None) => break,
            None => {
                (msg, first) = (msg.push(if first { "…" } else { "·…" }), false);
                break;
            }
        }
    }
    if first { msg.push("1") } else { msg }
}

/// One base of a [Dim]: `·` before it unless it is the first, its symbol, and its exponent unless that is 1, a half
/// as `⁻⁵ᐟ²`.
const fn push_dim_part(mut msg: Msg, first: bool, symbol: &str, exp: Exp) -> (Msg, bool) {
    if !first {
        msg = msg.push("·");
    }
    msg = msg.push(symbol);
    match exp.whole() {
        Some(1) => {}
        Some(n) => msg = msg.push_superscript(widen(n)),
        None => msg = msg.push_superscript(widen(exp.halves())).push("ᐟ²"),
    }
    (msg, false)
}

/// The quantity beyond the dimension, never empty: a spec's name split at its capitals (`path length`), "a ratio
/// of" a dimension, "its dimension's quantity" for [QuantityExpr::Derived].
const fn push_quantity(msg: Msg, q: QuantityExpr<'_>) -> Msg {
    match q {
        QuantityExpr::Spec(r) => push_quantity_name(msg, r),
        QuantityExpr::RatioOf(d) => push_dim(msg.push("a ratio of "), d),
        QuantityExpr::Derived(_) => msg.push("its dimension's quantity"),
    }
}

/// A quantity's name: its marker's name split at the capitals, lower case.
#[expect(clippy::indexing_slicing, reason = "i < b.len(), the loop bound")]
#[expect(
    clippy::arithmetic_side_effects,
    reason = "i < b.len(), the loop bound; b'a' - b'A' added to an upper-case ASCII letter stays ASCII"
)]
const fn push_quantity_name(mut msg: Msg, r: QuantityRef<'_>) -> Msg {
    let b = r.id().name.as_bytes();
    let mut i = 0;
    while i < b.len() {
        let c = b[i];
        if c.is_ascii_uppercase() {
            if i > 0 {
                msg = msg.push_byte(b' ');
            }
            msg = msg.push_byte(c + (b'a' - b'A'));
        } else {
            msg = msg.push_byte(c);
        }
        i += 1;
    }
    msg
}

/// The symbol of a unit expression, as written after a number: `N·m`, empty for [units::Ratio](crate::units::Ratio).
const fn push_symbol(mut msg: Msg, e: &UnitExpr) -> Msg {
    match e {
        UnitExpr::Named(def) => msg.push(def.names.symbol),
        UnitExpr::Mul(a, b) => {
            msg = push_symbol(msg, a);
            msg = msg.push("·");
            push_symbol(msg, b)
        }
        UnitExpr::Div(a, b) => {
            msg = push_symbol(msg, a);
            msg = msg.push("/");
            push_symbol(msg, b)
        }
        UnitExpr::Pow(a, n) => push_symbol(msg, a).push_superscript(widen(*n)),
        UnitExpr::Sqrt(a) => push_symbol(msg.push("√"), a),
    }
}

/// The type name of a unit expression: `Volt`, `Per<Torque, AngularVelocity>`.
const fn push_name(mut msg: Msg, e: &UnitExpr) -> Msg {
    match e {
        UnitExpr::Named(def) => msg.push(def.id.name),
        UnitExpr::Mul(a, b) => push_pair(msg, "Times", a, b),
        UnitExpr::Div(a, b) => push_pair(msg, "Per", a, b),
        UnitExpr::Pow(a, n) => {
            msg = msg.push("Pow<");
            msg = push_name(msg, a);
            msg = msg.push(", ");
            msg.push_int(widen(*n)).push(">")
        }
        UnitExpr::Sqrt(a) => push_name(msg.push("Sqrt<"), a).push(">"),
    }
}

const fn push_pair(mut msg: Msg, ty: &str, a: &UnitExpr, b: &UnitExpr) -> Msg {
    msg = msg.push(ty);
    msg = msg.push("<");
    msg = push_name(msg, a);
    msg = msg.push(", ");
    msg = push_name(msg, b);
    msg.push(">")
}

/// A unit by its type name and symbol: `Torque (N·m)`.
const fn push_unit(mut msg: Msg, e: &'static UnitExpr) -> Msg {
    msg = push_name(msg, e);
    msg = msg.push(" (");
    push_symbol(msg, e).push(")")
}

/// The radians of `d` when it is only an angle (no true base, no other distinguished base) a whole number of times.
const fn only_rad(d: Dim<'_>) -> Option<i8> {
    if !d.true_part().is_one_typed() {
        return None;
    }
    let Some(rad) = d.const_exp_of(MarkerId::RADIAN) else {
        return None;
    };
    if rad.is_zero() {
        return None;
    }
    // nothing but the radian: the same as rad to that power
    let slot = [crate::BaseExp {
        base: MarkerId::RADIAN,
        exp: rad,
    }];
    match Dim::checked_with_list([Exp::ZERO; crate::MAX_TRUE_BASES], &slot) {
        Some(r) => match r.const_same(d) {
            Some(true) => rad.whole(),
            _ => None,
        },
        None => None,
    }
}

/// `n` widened into an `i32`, for [push_angle_hint]'s `push_superscript(widen_u8(n))`: `const fn`, where `From` is
/// not const on stable.
#[expect(
    clippy::as_conversions,
    reason = "const fn: From is not const on stable; n is a u8, widening into i32 exactly"
)]
const fn widen_u8(n: u8) -> i32 {
    n as i32
}

/// The name of the one angle marker that spans `n` radians on its own: [Radian](crate::units::Radian) for one,
/// [Steradian](crate::units::Steradian) for two (a solid angle is a plane one squared) — the only angle bases this
/// crate names more than a scale apart from a plain radian. `None` for any other `n`, where no single named unit
/// spans it, so [push_angle_hint] never claims a step through a type that doesn't exist.
const fn angle_unit_name(n: u8) -> Option<&'static str> {
    match n {
        1 => Some("Radian"),
        2 => Some("Steradian"),
        _ => None,
    }
}

/// Whether `low` (fewer radians) reaches `high` (more) in one step of this crate's own product table
/// ([products::TABLE](crate::products::TABLE)): scans every entry for a side whose [Dim] and quantity match `low`
/// exactly, the entry's output matching `high` — the table doesn't care which of its two factors that side was,
/// so both are tried. Never a separate list of pairs to keep in step with `products.rs` by hand: an entry added,
/// changed or removed there changes what this sees in the same build, nothing here to update. The quantity as well as
/// the [Dim]: [Watt](crate::units::Watt), [VoltAmpere](crate::units::VoltAmpere) and [Var](crate::units::Var) share a
/// dimension but only [Watt] is really [RadiantIntensity](crate::units::RadiantIntensity) times a
/// [Steradian](crate::units::Steradian), so a bare [Dim] match would wrongly say yes for the other two.
#[expect(clippy::indexing_slicing, reason = "i < table.len(), the loop bound")]
#[expect(
    clippy::arithmetic_side_effects,
    reason = "i < table.len(), the loop bound, so i + 1 fits usize"
)]
const fn known_angle_step(
    low: Dim<'_>,
    low_kind: QuantityExpr<'_>,
    high: Dim<'_>,
    high_kind: QuantityExpr<'_>,
) -> bool {
    let table = crate::products::TABLE;
    let mut i = 0;
    while i < table.len() {
        let entry = &table[i];
        let o_matches = entry.o.0.same_typed(high) && entry.o.1.same_typed(high_kind);
        let a_matches = entry.a.0.same_typed(low) && entry.a.1.same_typed(low_kind);
        let b_matches = entry.b.0.same_typed(low) && entry.b.1.same_typed(low_kind);
        if o_matches && (a_matches || b_matches) {
            return true;
        }
        i += 1;
    }
    false
}

/// When `diff` (`A::DIM` divided by `B::DIM`) is only an angle ([only_rad]), the hint that crosses it: when
/// [known_angle_step] says the product table really has that step, `A`'s and `B`'s real names around the angle
/// marker that carries it (`Torque × Radian is Joule` when `A`'s dimension has fewer radians than `B`'s, `Candela ×
/// Steradian is Lumen` when the gap is two), the same wording whichever direction `A`/`B` come in, since the
/// product table doesn't care which side the caller started from. Where there's no such entry (by choice, or
/// because the gap isn't one radian or one steradian), a fallback that never calls it named: the `Times`/`Per`
/// compound a caller would actually have to build, then convert from. Does nothing when `diff` isn't only an
/// angle.
const fn push_angle_hint<A: Unit, B: Unit>(mut msg: Msg, diff: Dim<'_>) -> Msg {
    let Some(rad) = only_rad(diff) else {
        return msg;
    };
    let n = rad.unsigned_abs();
    let (low_dim, low_kind, high_dim, high_kind) = if rad < 0 {
        (A::DIM, A::QUANTITY, B::DIM, B::QUANTITY)
    } else {
        (B::DIM, B::QUANTITY, A::DIM, A::QUANTITY)
    };
    if let Some(angle_unit) = angle_unit_name(n)
        && known_angle_step(low_dim, low_kind, high_dim, high_kind)
    {
        msg = msg.push(" — cross it with a named step: ");
        msg = push_name(msg, A::EXPR);
        msg = msg.push(if rad < 0 { " × " } else { " / " });
        msg = msg.push(angle_unit);
        msg = msg.push(" is ");
        return push_name(msg, B::EXPR);
    }
    msg = msg.push(" — not a named step: converts through ");
    msg = msg.push(if rad < 0 { "Times<" } else { "Per<" });
    msg = push_name(msg, A::EXPR);
    msg = msg.push(", Radian");
    if n != 1 {
        msg = msg.push_superscript(widen_u8(n));
    }
    msg = msg.push(">, then convert");
    msg
}

/// The panic for two units whose dimension differs (`Fixed::convert`, `Exact::convert`, a literal's unit): both
/// units by name, symbol and dimension, the dimension left when `A`'s divides `B`'s (when it fits a `Dim`) — `rad⁻¹`
/// for a torque and a joule — and, where that leftover is only an angle, [push_angle_hint]'s named-step hint.
#[expect(
    clippy::panic,
    reason = "a compile error in const code (a panic at run time), as documented at every call site"
)]
pub(crate) const fn dim_mismatch<A: Unit, B: Unit>(what: &str) -> ! {
    let mut msg = Msg::new().push(what);
    msg = msg.push(" converts only between units of the same dimension: ");
    msg = push_unit(msg, A::EXPR);
    msg = msg.push(" is ");
    msg = push_dim(msg, A::DIM);
    msg = msg.push(", not ");
    msg = push_unit(msg, B::EXPR);
    msg = msg.push(" which is ");
    msg = push_dim(msg, B::DIM);
    if let Some(diff) = A::DIM.const_div(&B::DIM) {
        msg = msg.push("; they differ by ");
        msg = push_dim(msg, diff);
        msg = push_angle_hint::<A, B>(msg, diff);
    }
    panic!("{}", msg.as_str())
}

/// The panic for a crossing between systems with no bridge ([Fixed::to_system](crate::Fixed::to_system)): the two
/// units, their quantities and dimensions, and the reason.
#[expect(
    clippy::panic,
    reason = "a compile error in const code (a panic at run time), as documented at every call site"
)]
pub(crate) const fn no_bridge<A: Unit, B: Unit>(e: crate::BridgeError) -> ! {
    let mut msg = Msg::new().push("no bridge from ");
    msg = push_unit(msg, A::EXPR);
    msg = msg.push(" (");
    msg = push_quantity(msg, A::QUANTITY);
    msg = msg.push(", ");
    msg = push_dim(msg, A::DIM);
    msg = msg.push(") to ");
    msg = push_unit(msg, B::EXPR);
    msg = msg.push(" (");
    msg = push_quantity(msg, B::QUANTITY);
    msg = msg.push(", ");
    msg = push_dim(msg, B::DIM);
    msg = msg.push("): ").push(e.reason());
    panic!("{}", msg.as_str())
}

/// The panic for two units of the same dimension but a different [Kind] (`Fixed::convert`, `Exact::convert`): a
/// transimpedance (V/A, plain) is not an ohm (a resistance), a duty cycle (a plain ratio) is not a voltage gain (a
/// ratio of volts).
#[expect(
    clippy::panic,
    reason = "a compile error in const code (a panic at run time), as documented at every call site"
)]
pub(crate) const fn kind_mismatch<A: Unit, B: Unit>(what: &str) -> ! {
    let mut msg = Msg::new().push(what);
    msg = msg.push(" converts only to the same quantity or one above it: ");
    msg = push_unit(msg, A::EXPR);
    msg = msg.push(" is ");
    msg = push_quantity(msg, A::QUANTITY);
    msg = msg.push(", ");
    msg = push_unit(msg, B::EXPR);
    msg = msg.push(" is ");
    msg = push_quantity(msg, B::QUANTITY);
    msg = msg.push(
        " (down the tree or across a kind with a named step: as_quantity, as_ratio, as_phase, ..)",
    );
    panic!("{}", msg.as_str())
}

/// The panic for [Exact::recip](crate::Exact::recip) into a unit that isn't `A`'s reciprocal dimension: `A` by name
/// and symbol, the dimension `B` would need to be, and `B`'s actual dimension.
#[expect(
    clippy::panic,
    reason = "a compile error in const code (a panic at run time), as documented at every call site"
)]
pub(crate) const fn recip_mismatch<A: Unit, B: Unit>() -> ! {
    let mut msg = Msg::new().push("recip from ");
    msg = push_unit(msg, A::EXPR);
    msg = msg.push(" needs a unit of the reciprocal dimension");
    if let Some(want) = Dim::ONE.const_div(&A::DIM) {
        msg = msg.push(", ");
        msg = push_dim(msg, want);
    }
    msg = msg.push(": ");
    msg = push_unit(msg, B::EXPR);
    msg = msg.push(" is ");
    msg = push_dim(msg, B::DIM);
    panic!("{}", msg.as_str())
}

/// The panic for a unit whose factor to SI isn't exact: a product or quotient whose [Magnitude](crate::Magnitude)
/// leaves what it can hold.
#[expect(
    clippy::panic,
    reason = "a compile error in const code (a panic at run time), as documented at every call site"
)]
pub(crate) const fn no_exact_factor<A: Unit, B: Unit>(what: &str) -> ! {
    let mut msg = Msg::new().push(what);
    msg = msg.push(" needs an exact factor to SI for both units: ");
    msg = push_unit(msg, A::EXPR);
    msg = msg.push(" and ");
    msg = push_unit(msg, B::EXPR);
    msg = msg.push(" (one leaves what a Magnitude holds: more than 16 primes, or past ±63½)");
    panic!("{}", msg.as_str())
}

/// A unit's symbol as text, `N·m`: for the messages of a `q!` range, which name stored values in the unit of the
/// literal.
#[cfg(feature = "q")]
pub(crate) struct Symbol(Msg);

#[cfg(feature = "q")]
impl Symbol {
    pub(crate) const fn of(e: &UnitExpr) -> Symbol {
        Symbol(push_symbol(Msg::new(), e))
    }

    /// The symbol and an origin, `K @ Celsius`: a point's frame, as `q!` writes a point literal in it.
    pub(crate) const fn at(e: &UnitExpr, origin: &str) -> Symbol {
        Symbol(push_symbol(Msg::new(), e).push(" @ ").push(origin))
    }

    pub(crate) const fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // two markers of the shape the angle hint cares about, without depending on the `mechanical` / `angle`
    // features that `units::Torque` and `units::Joule` need: a torque's dimension (kg·m²·s⁻²·rad⁻¹) and its
    // energy (kg·m²·s⁻²), one radian apart. `Energy`'s own quantity (not `units::Joule`'s derived one) keeps this pair
    // from ever matching the real `Torque * Radian = Joule` entry of `known_angle_step`, whether or not those
    // features are on: these two test the fallback wording, the feature-gated tests below the named one.
    crate::quantity! {
        struct TestOnly = kind [kg 1, m 2, s -2];
    }

    crate::unit! {
        struct Torque = "N·m", [kg 1, m 2, s -2, rad -1];
        struct Energy = "J", [kg 1, m 2, s -2] quantity TestOnly;
    }

    #[test]
    fn push_dim_writes_negative_exponents() {
        let msg = push_dim(Msg::new(), Torque::DIM);
        assert_eq!(msg.as_str(), "kg·m²·s⁻²·rad⁻¹");
    }

    #[test]
    fn push_dim_writes_half_exponents_of_a_root() {
        // V/√Hz: s⁻⁵ᐟ² is -5 halves, shown with `ᐟ²`, the whole ones plainly.
        let d = Dim::of_exps([
            Exp::from_halves(-5),
            Exp::int(2),
            Exp::int(1),
            Exp::int(-1),
            Exp::ZERO,
            Exp::ZERO,
            Exp::ZERO,
        ]);
        let msg = push_dim(Msg::new(), d);
        assert_eq!(msg.as_str(), "kg·m²·s⁻⁵ᐟ²·A⁻¹");
    }

    #[test]
    fn push_dim_of_a_ratio_is_one() {
        assert_eq!(push_dim(Msg::new(), Dim::ONE).as_str(), "1");
    }

    #[test]
    fn push_superscript_writes_signed_digits() {
        assert_eq!(Msg::new().push_superscript(0).as_str(), "⁰");
        assert_eq!(Msg::new().push_superscript(2).as_str(), "²");
        assert_eq!(Msg::new().push_superscript(12).as_str(), "¹²");
        assert_eq!(Msg::new().push_superscript(-1).as_str(), "⁻¹");
        assert_eq!(Msg::new().push_superscript(-23).as_str(), "⁻²³");
    }

    #[test]
    fn angle_hint_multiplies_when_a_has_fewer_radians() {
        // Torque's rad is -1, Energy's is 0: crossing Torque to Energy multiplies by Radian. No product table
        // entry backs this invented pair (that's the point), so the fallback names the Times<..> instead.
        let diff = Torque::DIM.checked_div(&Energy::DIM).unwrap();
        let msg = push_angle_hint::<Torque, Energy>(Msg::new(), diff);
        assert_eq!(
            msg.as_str(),
            " — not a named step: converts through Times<Torque, Radian>, then convert"
        );
    }

    #[test]
    fn angle_hint_divides_when_a_has_more_radians() {
        // the other way round: Energy's rad is 0, Torque's is -1, so going from Energy to Torque divides.
        let diff = Energy::DIM.checked_div(&Torque::DIM).unwrap();
        let msg = push_angle_hint::<Energy, Torque>(Msg::new(), diff);
        assert_eq!(
            msg.as_str(),
            " — not a named step: converts through Per<Energy, Radian>, then convert"
        );
    }

    #[test]
    fn angle_hint_does_nothing_when_more_than_angle_differs() {
        // kg differs too (a mass, not only an angle): no hint, same as a dimension that doesn't differ at all.
        let not_only_rad = Dim::with_list(
            Dim::new(0, 0, 1, 0, 0, 0, 0).exps(),
            &[crate::BaseExp {
                base: MarkerId::RADIAN,
                exp: Exp::ONE,
            }],
        );
        let msg = push_angle_hint::<Torque, Energy>(Msg::new(), not_only_rad);
        assert_eq!(msg.as_str(), "");
    }

    /// Compiles only when `A * B = O` is really a `UnitMul` entry of the product table: used by the tests below to
    /// confirm a "named step" hint isn't just a [known_angle_step] `Dim`/`Kind` coincidence with nothing backing
    /// it (P2401#5nov-1340's own worry: the lookup table and `products.rs` drifting apart). `cfg`-gated the same
    /// as its callers so it's never dead code when none of them are built.
    #[cfg(any(
        all(feature = "time", feature = "angle"),
        all(feature = "mechanical", feature = "angle"),
        all(feature = "photometric", feature = "angle"),
        all(feature = "radiometric", feature = "angle", feature = "electrical"),
    ))]
    fn assert_is_a_product<A: crate::UnitMul<B, Output = O>, B: Unit, O: Unit>() {}

    // The angle-differing pairs of named units this crate ships (the P2401#5nov-1340 walk): one test per pair,
    // each gated on the features its units need, so `cargo test` only sees it when the units exist. A "named step"
    // test also asserts the real product with `assert_is_a_product`; a "not a named step" one is a pair that
    // differs only by a radian or a steradian but was left out of `products.rs` on purpose (the ready note says
    // why for each).

    #[cfg(all(feature = "time", feature = "angle"))]
    #[test]
    fn hertz_times_radian_is_angular_velocity_a_named_step() {
        use crate::units::{AngularVelocity, Hertz, Radian};
        assert_is_a_product::<Hertz, Radian, AngularVelocity>();
        let diff = Hertz::DIM.checked_div(&AngularVelocity::DIM).unwrap();
        let msg = push_angle_hint::<Hertz, AngularVelocity>(Msg::new(), diff);
        assert_eq!(
            msg.as_str(),
            " — cross it with a named step: Hertz × Radian is AngularVelocity"
        );
    }

    #[cfg(all(feature = "time", feature = "angle"))]
    #[test]
    fn hertz_per_second_times_radian_is_angular_acceleration_a_named_step() {
        use crate::units::{AngularAcceleration, HertzPerSecond, Radian};
        assert_is_a_product::<HertzPerSecond, Radian, AngularAcceleration>();
        let diff = HertzPerSecond::DIM
            .checked_div(&AngularAcceleration::DIM)
            .unwrap();
        let msg = push_angle_hint::<HertzPerSecond, AngularAcceleration>(Msg::new(), diff);
        assert_eq!(
            msg.as_str(),
            " — cross it with a named step: HertzPerSecond × Radian is AngularAcceleration"
        );
    }

    #[cfg(all(feature = "mechanical", feature = "angle"))]
    #[test]
    fn real_torque_times_radian_is_joule_a_named_step() {
        use crate::units::{Joule, Radian, Torque as RealTorque};
        assert_is_a_product::<RealTorque, Radian, Joule>();
        let diff = RealTorque::DIM.checked_div(&Joule::DIM).unwrap();
        let msg = push_angle_hint::<RealTorque, Joule>(Msg::new(), diff);
        assert_eq!(
            msg.as_str(),
            " — cross it with a named step: Torque × Radian is Joule"
        );
    }

    #[cfg(all(feature = "photometric", feature = "angle"))]
    #[test]
    fn candela_times_steradian_is_lumen_a_named_step() {
        use crate::units::{Candela, Lumen, Steradian};
        assert_is_a_product::<Candela, Steradian, Lumen>();
        let diff = Candela::DIM.checked_div(&Lumen::DIM).unwrap();
        let msg = push_angle_hint::<Candela, Lumen>(Msg::new(), diff);
        assert_eq!(
            msg.as_str(),
            " — cross it with a named step: Candela × Steradian is Lumen"
        );
    }

    #[cfg(all(feature = "radiometric", feature = "angle", feature = "electrical"))]
    #[test]
    fn radiant_intensity_times_steradian_is_watt_a_named_step() {
        use crate::units::{RadiantIntensity, Steradian, Watt};
        assert_is_a_product::<RadiantIntensity, Steradian, Watt>();
        let diff = RadiantIntensity::DIM.checked_div(&Watt::DIM).unwrap();
        let msg = push_angle_hint::<RadiantIntensity, Watt>(Msg::new(), diff);
        assert_eq!(
            msg.as_str(),
            " — cross it with a named step: RadiantIntensity × Steradian is Watt"
        );
    }

    #[cfg(all(feature = "angle", feature = "dimensionless"))]
    #[test]
    fn radian_and_ratio_is_not_a_named_step() {
        // the crate's own design (Radian's doc comment): angle is a base dimension of its own so it never mixes
        // with a plain Ratio, so there's no product crossing them even though they differ only by one radian.
        use crate::units::{Radian, Ratio};
        let diff = Ratio::DIM.checked_div(&Radian::DIM).unwrap();
        let msg = push_angle_hint::<Ratio, Radian>(Msg::new(), diff);
        assert_eq!(
            msg.as_str(),
            " — not a named step: converts through Times<Ratio, Radian>, then convert"
        );
    }

    #[cfg(all(feature = "photometric", feature = "angle"))]
    #[test]
    fn nit_and_lux_is_not_a_named_step() {
        // illuminance (lux) is not luminance (nit) times a solid angle: the real relation integrates luminance
        // over the cosine-weighted solid angle (Lambert's law), not a bare product, so this is left out even
        // though the two differ only by a steradian.
        use crate::units::{Lux, Nit};
        let diff = Nit::DIM.checked_div(&Lux::DIM).unwrap();
        let msg = push_angle_hint::<Nit, Lux>(Msg::new(), diff);
        assert_eq!(
            msg.as_str(),
            " — not a named step: converts through Times<Nit, Radian²>, then convert"
        );
    }

    #[cfg(feature = "radiometric")]
    #[test]
    fn radiance_and_irradiance_is_not_a_named_step() {
        // the radiometric analogue of nit/lux, same reason: no bare product, Lambert's law in the way.
        use crate::units::{Irradiance, Radiance};
        let diff = Radiance::DIM.checked_div(&Irradiance::DIM).unwrap();
        let msg = push_angle_hint::<Radiance, Irradiance>(Msg::new(), diff);
        assert_eq!(
            msg.as_str(),
            " — not a named step: converts through Times<Radiance, Radian²>, then convert"
        );
    }

    #[cfg(all(feature = "radiometric", feature = "angle", feature = "electrical"))]
    #[test]
    fn radiant_intensity_and_volt_ampere_is_not_a_named_step() {
        // VoltAmpere shares Watt's dimension but not its kind (an apparent power, never a plain power by
        // `convert`): known_angle_step must tell them apart by kind, not dimension alone, so RadiantIntensity ×
        // Steradian stays Watt's step only, never VoltAmpere's.
        use crate::units::{RadiantIntensity, VoltAmpere};
        let diff = RadiantIntensity::DIM.checked_div(&VoltAmpere::DIM).unwrap();
        let msg = push_angle_hint::<RadiantIntensity, VoltAmpere>(Msg::new(), diff);
        assert_eq!(
            msg.as_str(),
            " — not a named step: converts through Times<RadiantIntensity, Radian²>, then convert"
        );
    }

    #[test]
    fn every_angle_differing_table_entry_is_a_named_step() {
        // Walks products::TABLE itself (not a hand-picked list of pairs): for every entry whose `a` or `b` side
        // differs from its `o` only by rad (one radian or one steradian, whichever the build has), that's exactly
        // the shape known_angle_step must say yes to — it reads the very table this test walks, so this also
        // catches an angle product added to products.rs without `known_angle_step` noticing, were that still
        // possible (it scans the table now, so it can't drift from it, but the property is worth keeping true).
        for entry in crate::products::TABLE {
            for side in [entry.a, entry.b] {
                let Some(diff) = side.0.checked_div(&entry.o.0) else {
                    continue;
                };
                let Some(rad) = only_rad(diff) else {
                    continue;
                };
                let (low, low_kind, high, high_kind) = if rad < 0 {
                    (side.0, side.1, entry.o.0, entry.o.1)
                } else {
                    (entry.o.0, entry.o.1, side.0, side.1)
                };
                assert!(
                    known_angle_step(low, low_kind, high, high_kind),
                    "a product table entry differs only by rad but known_angle_step misses it"
                );
            }
        }
    }

    /// `Msg::new()` filled with `CAP - 1` single-byte pushes: one byte short of the cap.
    fn almost_full() -> Msg {
        let mut msg = Msg::new();
        for _ in 0..Msg::CAP - 1 {
            msg = msg.push("a");
        }
        msg
    }

    #[test]
    fn push_drops_a_multi_byte_push_whole_rather_than_split_it() {
        // one byte short of the cap, a 2-byte char must not be split (which would leave a dangling byte that
        // `as_str` can't decode): it's dropped whole, so the content is unchanged. No `alloc` here (the crate is
        // `no_std`, no alloc): two independent buffers compared instead of cloning one's `&str` before mutating it.
        let before = almost_full();
        assert_eq!(before.as_str().len(), Msg::CAP - 1);
        let after = almost_full().push("·");
        assert_eq!(
            after.as_str(),
            before.as_str(),
            "a push past the cap must change nothing"
        );
    }

    #[test]
    fn push_takes_a_multi_byte_char_that_fits_exactly() {
        let mut msg = Msg::new();
        for _ in 0..Msg::CAP - 2 {
            msg = msg.push("a");
        }
        msg = msg.push("·"); // 2 bytes, exactly fills the last 2 of CAP
        let s = msg.as_str();
        assert_eq!(s.len(), Msg::CAP);
        assert!(s.ends_with('·'));
    }

    #[test]
    fn push_past_the_cap_stays_a_valid_whole_prefix() {
        // "·" is 2 bytes; CAP is a multiple of 2, so this lands exactly on the cap with no partial char.
        const { assert!(Msg::CAP.is_multiple_of(2)) };
        let mut msg = Msg::new();
        for _ in 0..Msg::CAP {
            msg = msg.push("·");
        }
        let s = msg.as_str();
        assert_eq!(s.len(), Msg::CAP);
        assert!(s.chars().all(|c| c == '·'));
        assert_eq!(s.chars().count(), Msg::CAP / "·".len());
    }
}
