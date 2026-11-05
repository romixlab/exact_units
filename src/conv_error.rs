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

/// `n`, as plain (non-superscript) digits, `-` before a negative one: for `Radian` in [push_angle_hint], where a
/// `Dim`'s own exponents are superscript but this one stands for a repeated operator, not a power written on a
/// unit symbol.
#[expect(
    clippy::as_conversions,
    reason = "const fn: From is not const on stable; n is a u8, widening into i32 exactly"
)]
const fn widen_u8(n: u8) -> i32 {
    n as i32
}

/// When `diff` (`A::DIM` divided by `B::DIM`) is only an angle ([is_only_rad]), the hint that names the actual
/// operation from `A` and `B`'s real names: `Torque × Radian is Joule` when `A`'s dimension has fewer radians than
/// `B`'s (multiplying by an angle gets from `A` to `B`), `Hertz / Radian is AngularVelocity` the other way, with
/// the angle's own exponent written when crossing it takes more than one step. Does nothing otherwise.
const fn push_angle_hint<A: Unit, B: Unit>(mut msg: Msg, diff: Dim<'_>) -> Msg {
    if !is_only_rad(diff) {
        return msg;
    }
    let n = diff.rad.unsigned_abs();
    msg = msg.push(" — cross it with a named step: ");
    msg = push_name(msg, A::EXPR);
    msg = msg.push(if diff.rad < 0 {
        " × Radian"
    } else {
        " / Radian"
    });
    if n != 1 {
        msg = msg.push_superscript(widen_u8(n));
    }
    msg = msg.push(" is ");
    push_name(msg, B::EXPR)
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
    // energy (kg·m²·s⁻²), one radian apart.
    crate::unit! {
        struct Torque = "N·m", [kg 1, m 2, s -2, rad -1];
        struct Energy = "J", [kg 1, m 2, s -2];
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
        // Torque's rad is -1, Energy's is 0: Torque × Radian is Energy.
        let diff = Torque::DIM.checked_div(Energy::DIM).unwrap();
        let msg = push_angle_hint::<Torque, Energy>(Msg::new(), diff);
        assert_eq!(
            msg.as_str(),
            " — cross it with a named step: Torque × Radian is Energy"
        );
    }

    #[test]
    fn angle_hint_divides_when_a_has_more_radians() {
        // the other way round: Energy's rad is 0, Torque's is -1, so going from Energy to Torque divides.
        let diff = Energy::DIM.checked_div(Torque::DIM).unwrap();
        let msg = push_angle_hint::<Energy, Torque>(Msg::new(), diff);
        assert_eq!(
            msg.as_str(),
            " — cross it with a named step: Energy / Radian is Torque"
        );
    }

    #[test]
    fn angle_hint_does_nothing_when_more_than_angle_differs() {
        // kg differs too (a mass, not only an angle): no hint, same as a dimension that doesn't differ at all.
        let not_only_rad = Dim::new(0, 0, 1, 0, 0, 0, 0).with_rad(1);
        let msg = push_angle_hint::<Torque, Energy>(Msg::new(), not_only_rad);
        assert_eq!(msg.as_str(), "");
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
