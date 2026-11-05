//! A readable message for a refused conversion, built at compile time (P2401#5nov-0954): names both units (their
//! type name and symbol from [UnitExpr]), both dimensions, and (for a dimension mismatch) the dimension left when
//! one divides the other, so a mismatch says what differs instead of only that something does. Used by
//! [Fixed::convert](crate::Fixed::convert) and its kin in `fixed.rs`, by `Exact`'s conversions in `exact.rs`, and
//! by a decimal literal's unit check in `literal.rs`.
//!
//! Every function here ends in `panic!`, never returns: the message is built and panicked with in one call, so its
//! buffer never has to outlive the function that builds it (a `const` buffer can't be borrowed into a `'static`
//! `&str` any other way on stable Rust).

use crate::unit::superscript_digit;
use crate::{Dim, Kind, Unit, UnitExpr};

/// Text written into a buffer on the stack, for a `panic!("{}", msg.as_str())` right after. A push that wouldn't
/// fit whole is dropped whole (never split a char in half: [as_str](Self::as_str) always decodes); a cut-off
/// message is still readable, and no message built here comes close to [Msg::CAP] (a unit's name and symbol are a
/// handful of characters, a `Dim` at most a few dozen).
struct Msg {
    bytes: [u8; Msg::CAP],
    len: usize,
}

impl Msg {
    const CAP: usize = 320;

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

/// The dimension as [Dim]'s `Display` writes it: `kg·m²·s⁻²·rad⁻¹`, `1` for a ratio.
#[expect(clippy::indexing_slicing, reason = "i < parts.len(), the loop bound")]
#[expect(
    clippy::arithmetic_side_effects,
    reason = "i < parts.len() and i < MAX_COUNTED, the loop bounds"
)]
const fn push_dim(mut msg: Msg, d: Dim<'_>) -> Msg {
    let parts: [(&str, i8); 9] = [
        ("kg", d.kg),
        ("m", d.m),
        ("s", d.s),
        ("A", d.a),
        ("K", d.k),
        ("mol", d.mol),
        ("cd", d.cd),
        ("rad", d.rad),
        ("bit", d.bit),
    ];
    let mut first = true;
    let mut i = 0;
    while i < parts.len() {
        let (symbol, exp) = parts[i];
        if exp != 0 {
            (msg, first) = push_dim_part(msg, first, symbol, exp, d.root);
        }
        i += 1;
    }
    let mut i = 0;
    while i < crate::MAX_COUNTED {
        if let Some(c) = d.counted[i] {
            (msg, first) = push_dim_part(msg, first, c.name, c.exp, d.root);
        }
        i += 1;
    }
    if first { msg.push("1") } else { msg }
}

/// One base of a [Dim]: `·` before it unless it is the first, its symbol, and its exponent unless that is 1 (even
/// in `root`'s halved form).
const fn push_dim_part(
    mut msg: Msg,
    first: bool,
    symbol: &str,
    exp: i8,
    root: bool,
) -> (Msg, bool) {
    if !first {
        msg = msg.push("·");
    }
    msg = msg.push(symbol);
    if root && exp % 2 != 0 {
        msg = msg.push_superscript(widen(exp)).push("ᐟ²");
    } else {
        let exp = if root { exp / 2 } else { exp };
        if exp != 1 {
            msg = msg.push_superscript(widen(exp));
        }
    }
    (msg, false)
}

/// The kind beyond the dimension, as [Kind]'s `Display` writes it, but never empty: "a plain unit" for
/// [Kind::Plain].
const fn push_kind(msg: Msg, k: Kind<'_>) -> Msg {
    match k {
        Kind::Plain => msg.push("a plain unit"),
        Kind::Named(name) => msg.push(name),
        Kind::RatioOf(d) => push_dim(msg.push("a ratio of "), d),
    }
}

/// The symbol of a unit expression, as written after a number: `N·m`, empty for [units::Ratio](crate::units::Ratio).
const fn push_symbol(mut msg: Msg, e: &UnitExpr) -> Msg {
    match e {
        UnitExpr::Named { symbol, .. } => msg.push(symbol),
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
        UnitExpr::Named { name, .. } => msg.push(name),
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

/// Whether `d` is only an angle: every exponent 0 but `rad`'s, no counted base, not a root.
const fn is_only_rad(d: Dim<'_>) -> bool {
    d.rad != 0
        && d.s == 0
        && d.m == 0
        && d.kg == 0
        && d.a == 0
        && d.k == 0
        && d.mol == 0
        && d.cd == 0
        && d.bit == 0
        && !d.root
        && d.counted[0].is_none()
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
/// ([products::TABLE](crate::products::TABLE)): scans every entry for a side whose [Dim] and [Kind] match `low`
/// exactly, the entry's output matching `high` — the table doesn't care which of its two factors that side was,
/// so both are tried. Never a separate list of pairs to keep in step with `products.rs` by hand: an entry added,
/// changed or removed there changes what this sees in the same build, nothing here to update. [Kind] as well as
/// [Dim]: [Watt](crate::units::Watt), [VoltAmpere](crate::units::VoltAmpere) and [Var](crate::units::Var) share a
/// dimension but only [Watt] is really [RadiantIntensity](crate::units::RadiantIntensity) times a
/// [Steradian](crate::units::Steradian), so a bare [Dim] match would wrongly say yes for the other two.
#[expect(
    clippy::indexing_slicing,
    reason = "i < table.len(), the loop bound"
)]
#[expect(
    clippy::arithmetic_side_effects,
    reason = "i < table.len(), the loop bound, so i + 1 fits usize"
)]
const fn known_angle_step(low: Dim<'_>, low_kind: Kind<'_>, high: Dim<'_>, high_kind: Kind<'_>) -> bool {
    let table = crate::products::TABLE;
    let mut i = 0;
    while i < table.len() {
        let entry = &table[i];
        let o_matches = entry.o.0.same(high) && entry.o.1.same(high_kind);
        let a_matches = entry.a.0.same(low) && entry.a.1.same(low_kind);
        let b_matches = entry.b.0.same(low) && entry.b.1.same(low_kind);
        if o_matches && (a_matches || b_matches) {
            return true;
        }
        i += 1;
    }
    false
}

/// When `diff` (`A::DIM` divided by `B::DIM`) is only an angle ([is_only_rad]), the hint that crosses it: when
/// [known_angle_step] says the product table really has that step, `A`'s and `B`'s real names around the angle
/// marker that carries it (`Torque × Radian is Joule` when `A`'s dimension has fewer radians than `B`'s, `Candela ×
/// Steradian is Lumen` when the gap is two), the same wording whichever direction `A`/`B` come in, since the
/// product table doesn't care which side the caller started from. Where there's no such entry (by choice, or
/// because the gap isn't one radian or one steradian), a fallback that never calls it named: the `Times`/`Per`
/// compound a caller would actually have to build, then convert from. Does nothing when `diff` isn't only an
/// angle.
const fn push_angle_hint<A: Unit, B: Unit>(mut msg: Msg, diff: Dim<'_>) -> Msg {
    if !is_only_rad(diff) {
        return msg;
    }
    let n = diff.rad.unsigned_abs();
    let (low_dim, low_kind, high_dim, high_kind) = if diff.rad < 0 {
        (A::DIM, A::KIND, B::DIM, B::KIND)
    } else {
        (B::DIM, B::KIND, A::DIM, A::KIND)
    };
    if let Some(angle_unit) = angle_unit_name(n)
        && known_angle_step(low_dim, low_kind, high_dim, high_kind)
    {
        msg = msg.push(" — cross it with a named step: ");
        msg = push_name(msg, A::EXPR);
        msg = msg.push(if diff.rad < 0 { " × " } else { " / " });
        msg = msg.push(angle_unit);
        msg = msg.push(" is ");
        return push_name(msg, B::EXPR);
    }
    msg = msg.push(" — not a named step: converts through ");
    msg = msg.push(if diff.rad < 0 { "Times<" } else { "Per<" });
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
    if let Some(diff) = A::DIM.checked_div(B::DIM) {
        msg = msg.push("; they differ by ");
        msg = push_dim(msg, diff);
        msg = push_angle_hint::<A, B>(msg, diff);
    }
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
    msg = msg.push(" converts only between units of the same kind: ");
    msg = push_unit(msg, A::EXPR);
    msg = msg.push(" is ");
    msg = push_kind(msg, A::KIND);
    msg = msg.push(", ");
    msg = push_unit(msg, B::EXPR);
    msg = msg.push(" is ");
    msg = push_kind(msg, B::KIND);
    msg = msg.push(" (cross kinds with a named step: as_ratio, as_resistance, as_phase, ..)");
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
    if let Some(want) = A::DIM.checked_pow(-1) {
        msg = msg.push(", ");
        msg = push_dim(msg, want);
    }
    msg = msg.push(": ");
    msg = push_unit(msg, B::EXPR);
    msg = msg.push(" is ");
    msg = push_dim(msg, B::DIM);
    panic!("{}", msg.as_str())
}

/// The panic for a unit with an offset scale (°C) where a `Fixed` or `Exact` conversion needs an exact factor to
/// SI: it converts as a [Point](crate::Point), not these.
#[expect(
    clippy::panic,
    reason = "a compile error in const code (a panic at run time), as documented at every call site"
)]
pub(crate) const fn has_offset<A: Unit, B: Unit>(what: &str) -> ! {
    let mut msg = Msg::new().push(what);
    msg = msg.push(" converts as a Point, not these: ");
    msg = push_unit(msg, A::EXPR);
    if A::EXPR.has_offset() && B::EXPR.has_offset() {
        msg = msg.push(" and ");
        msg = push_unit(msg, B::EXPR);
        msg = msg.push(" both have an offset (like °C)");
    } else if A::EXPR.has_offset() {
        msg = msg.push(" has an offset (like °C)");
    } else {
        msg = msg.push(" converts with ");
        msg = push_unit(msg, B::EXPR);
        msg = msg.push(", which has an offset (like °C)");
    }
    panic!("{}", msg.as_str())
}

/// The panic for a unit whose factor to SI isn't exact (an `Affine` scale is [has_offset], this is the rarer case:
/// a product or quotient whose [Magnitude](crate::Magnitude) leaves what it can hold).
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
    msg = msg.push(" (one leaves what a Magnitude holds: more than 12 primes, or past ±63½)");
    panic!("{}", msg.as_str())
}

#[cfg(test)]
mod tests {
    use super::*;

    // two markers of the shape the angle hint cares about, without depending on the `mechanical` / `angle`
    // features that `units::Torque` and `units::Joule` need: a torque's dimension (kg·m²·s⁻²·rad⁻¹) and its
    // energy (kg·m²·s⁻²), one radian apart. `Energy`'s own kind (not `units::Joule`'s plain one) keeps this pair
    // from ever matching the real `Torque * Radian = Joule` entry of `known_angle_step`, whether or not those
    // features are on: these two test the fallback wording, the feature-gated tests below the named one.
    crate::unit! {
        struct Torque = "N·m", [kg 1, m 2, s -2, rad -1];
        struct Energy = "J", [kg 1, m 2, s -2] kind "test-only";
    }

    #[test]
    fn push_dim_writes_negative_exponents() {
        let msg = push_dim(Msg::new(), Torque::DIM);
        assert_eq!(msg.as_str(), "kg·m²·s⁻²·rad⁻¹");
    }

    #[test]
    fn push_dim_writes_half_exponents_of_a_root() {
        // V²/Hz, kg²·m⁴·s⁻⁵·A⁻² with `root` set (the module doc's own example): shown halved, the odd one
        // (s⁻⁵, odd) in superscript with `ᐟ²`, the even ones (kg, m, A) plainly halved.
        let d = Dim {
            s: -5,
            m: 4,
            kg: 2,
            a: -2,
            k: 0,
            mol: 0,
            cd: 0,
            rad: 0,
            bit: 0,
            root: true,
            counted: [None; crate::MAX_COUNTED],
        };
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
        let diff = Torque::DIM.checked_div(Energy::DIM).unwrap();
        let msg = push_angle_hint::<Torque, Energy>(Msg::new(), diff);
        assert_eq!(
            msg.as_str(),
            " — not a named step: converts through Times<Torque, Radian>, then convert"
        );
    }

    #[test]
    fn angle_hint_divides_when_a_has_more_radians() {
        // the other way round: Energy's rad is 0, Torque's is -1, so going from Energy to Torque divides.
        let diff = Energy::DIM.checked_div(Torque::DIM).unwrap();
        let msg = push_angle_hint::<Energy, Torque>(Msg::new(), diff);
        assert_eq!(
            msg.as_str(),
            " — not a named step: converts through Per<Energy, Radian>, then convert"
        );
    }

    #[test]
    fn angle_hint_does_nothing_when_more_than_angle_differs() {
        // kg differs too (a mass, not only an angle): no hint, same as a dimension that doesn't differ at all.
        let not_only_rad = Dim::new(0, 0, 1, 0, 0, 0, 0).with_rad(1);
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
        let diff = Hertz::DIM.checked_div(AngularVelocity::DIM).unwrap();
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
            .checked_div(AngularAcceleration::DIM)
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
        let diff = RealTorque::DIM.checked_div(Joule::DIM).unwrap();
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
        let diff = Candela::DIM.checked_div(Lumen::DIM).unwrap();
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
        let diff = RadiantIntensity::DIM.checked_div(Watt::DIM).unwrap();
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
        let diff = Ratio::DIM.checked_div(Radian::DIM).unwrap();
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
        let diff = Nit::DIM.checked_div(Lux::DIM).unwrap();
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
        let diff = Radiance::DIM.checked_div(Irradiance::DIM).unwrap();
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
        let diff = RadiantIntensity::DIM.checked_div(VoltAmpere::DIM).unwrap();
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
                let Some(diff) = side.0.checked_div(entry.o.0) else {
                    continue;
                };
                if !is_only_rad(diff) {
                    continue;
                }
                let (low, low_kind, high, high_kind) = if diff.rad < 0 {
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
        // "·" is 2 bytes; CAP (320) is a multiple of 2, so this lands exactly on the cap with no partial char.
        let mut msg = Msg::new();
        for _ in 0..200 {
            msg = msg.push("·");
        }
        let s = msg.as_str();
        assert_eq!(s.len(), Msg::CAP);
        assert!(s.chars().all(|c| c == '·'));
        assert_eq!(s.chars().count(), Msg::CAP / "·".len());
    }
}
