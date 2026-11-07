//! Property 8: text from outside, any string at all. `Decimal::parse` and `Decibels::parse` give the value the reference
//! reads, and panic (their documented refusal, a compile error in `const` code) exactly where the reference refuses,
//! with one of their documented messages: never an overflow, an index out of bounds or a wrong value. A level that
//! parses becomes a gain (`from_db_const`, what `db!` runs) or a documented refusal. `Prefix::from_name` and
//! `units::by_name` find exactly the names they list. Strings, so no Kani proof; `fuzz/` runs the same functions.
#![expect(
    clippy::arithmetic_side_effects,
    reason = "a test computes with the values it chose, which fit; an overflow fails the test"
)]
#![expect(clippy::panic, reason = "a property fails by panicking")]

use super::panics::outcome;
use super::reference;
use exact_units::decibel::Decibels;
use exact_units::literal::Decimal;
use exact_units::prefix::One;
use exact_units::units::{self, Per, Volt, Watt};
use exact_units::{Fixed, Prefix, UnitExpr};

/// Every message `Decimal::parse` refuses a literal with.
const DECIMAL_REFUSALS: [&str; 4] = [
    "a literal is a decimal number: 0.1, -3, 2.5e-3",
    "a literal of more than 18 digits",
    "a literal's exponent is out of range",
    "a literal of -0 reads two ways for a float storage, -0.0 or +0.0: write 0",
];

/// Every message `Decibels::parse` refuses a level with.
const DECIBELS_REFUSALS: [&str; 3] = [
    "a level in dB is a decimal number: -3, 6.02, +0.5",
    "a level in dB too long to parse",
    "a level in dB takes up to 9 decimals",
];

/// Every message `from_db_const` refuses a level with, for the storages below: a gain past the storage, or one too
/// close to halfway between two stored values to round at compile time.
const LEVEL_REFUSALS: [&str; 4] = [
    "this level is too large for its storage",
    "this level is too close to halfway between two stored values to round at compile time: write one more decimal",
    "this value is outside the normal range of its float storage",
    "this value is too close to halfway between two floats to round at compile time",
];

/// `Decimal::parse(s)` is the reference's value, or a documented refusal where the reference has none.
pub(crate) fn decimal(s: &str) {
    let want = reference::decimal(s).and_then(|(num, exp)| {
        // the digits are built as a positive i64, so i64::MIN's don't fit
        let n = i64::try_from(num.abs()).ok()?;
        Some(Decimal::new(
            if num < 0 { -n } else { n },
            i16::try_from(exp).ok()?,
        ))
    });
    match (outcome(|| Decimal::parse(s)), want) {
        (Ok(got), Some(want)) => assert_eq!(got, want, "{s:?}"),
        (Err(m), None) => assert!(DECIMAL_REFUSALS.contains(&m.as_str()), "{s:?}: {m}"),
        (got, want) => panic!("{s:?}: parsed {got:?}, the reference {want:?}"),
    }
}

/// `Decibels::parse(s)` is the reference's level, or a documented refusal where the reference has none; a level
/// becomes a gain, [level].
pub(crate) fn decibels(s: &str) {
    let want = reference::decibels(s).and_then(|(num, decimals)| {
        let n = i64::try_from(num.abs()).ok()?;
        let decimals = u8::try_from(decimals).ok().filter(|d| *d <= 9)?;
        Some(Decibels::new(if num < 0 { -n } else { n }, decimals))
    });
    match (outcome(|| Decibels::parse(s)), want) {
        (Ok(got), Some(want)) => {
            assert_eq!(got, want, "{s:?}");
            level(got.num(), got.decimals());
        }
        (Err(m), None) => assert!(DECIBELS_REFUSALS.contains(&m.as_str()), "{s:?}: {m}"),
        (got, want) => panic!("{s:?}: parsed {got:?}, the reference {want:?}"),
    }
}

/// The gain at `num · 10^-decimals` dB (`decimals` up to 9), as `db!` works it out: a power gain in a `u64` and an
/// amplitude gain in an `f32` are the nearest stored values to `10^(dB/10)` and `10^(dB/20)` (against `f64`, within
/// its rounding), or a documented refusal for a gain past the storage or one too close to a tie.
pub(crate) fn level(num: i64, decimals: u8) {
    let db = Decibels::new(num, decimals);
    let x = db.to_f64();
    match outcome(|| Fixed::<Per<Watt, Watt>, One, u64>::from_db_const(db).get()) {
        Ok(g) => {
            #[expect(
                clippy::as_conversions,
                clippy::cast_precision_loss,
                reason = "compared within f64's rounding"
            )]
            let g = g as f64;
            let want = 10f64.powf(x / 10.0);
            assert!(
                (g - want).abs() <= 0.5 + want * 1e-12,
                "{x} dB: {g}, 10^(dB/10) = {want}"
            );
        }
        Err(m) => {
            assert!(LEVEL_REFUSALS.contains(&m.as_str()), "{x} dB: {m}");
            // past u64::MAX (192.66 dB), or a tie: within a step of n + 1/2
            let want = 10f64.powf(x / 10.0);
            assert!(
                want > 1.8e19 || (want.fract() - 0.5).abs() < 1e-6 * want.max(1.0),
                "{x} dB refused: {m}"
            );
        }
    }
    match outcome(|| Fixed::<Per<Volt, Volt>, One, f32>::from_db_const(db).get()) {
        Ok(g) => {
            let want = 10f64.powf(x / 20.0);
            let g = f64::from(g);
            assert!(
                (g - want).abs() <= want * 1e-7,
                "{x} dB: {g}, 10^(dB/20) = {want}"
            );
        }
        Err(m) => {
            assert!(LEVEL_REFUSALS.contains(&m.as_str()), "{x} dB: {m}");
            // outside f32's normal range: below 2^-126 or past f32::MAX, or a tie
            let want = 10f64.powf(x / 20.0);
            let f = f64::from(f32::MAX);
            assert!(
                !(1.2e-38..=f).contains(&want) || is_f32_tie(want),
                "{x} dB refused: {m}"
            );
        }
    }
}

/// The gain at `num · 10^-decimals` Np, as `np!` works it out: a power gain in a `u64` and an amplitude gain in an
/// `f32` are the nearest stored values to `e^(2·Np)` and `e^Np` (against `f64`, within its rounding), or a
/// documented refusal for a gain past the storage or one too close to a tie.
pub(crate) fn neper_level(num: i64, decimals: u8) {
    use exact_units::decibel::Nepers;
    let np = Nepers::new(num, decimals);
    let x = np.to_f64();
    match outcome(|| Fixed::<Per<Watt, Watt>, One, u64>::from_np_const(np).get()) {
        Ok(g) => {
            #[expect(
                clippy::as_conversions,
                clippy::cast_precision_loss,
                reason = "compared within f64's rounding"
            )]
            let g = g as f64;
            let want = (2.0 * x).exp();
            assert!(
                (g - want).abs() <= 0.5 + want * 1e-12,
                "{x} Np: {g}, e^(2Np) = {want}"
            );
        }
        Err(m) => {
            assert!(LEVEL_REFUSALS.contains(&m.as_str()), "{x} Np: {m}");
            // past u64::MAX (22.18 Np), or a tie: within a step of n + 1/2
            let want = (2.0 * x).exp();
            assert!(
                want > 1.8e19 || (want.fract() - 0.5).abs() < 1e-6 * want.max(1.0),
                "{x} Np refused: {m}"
            );
        }
    }
    match outcome(|| Fixed::<Per<Volt, Volt>, One, f32>::from_np_const(np).get()) {
        Ok(g) => {
            let want = x.exp();
            let g = f64::from(g);
            assert!(
                (g - want).abs() <= want * 1e-7,
                "{x} Np: {g}, e^Np = {want}"
            );
        }
        Err(m) => {
            assert!(LEVEL_REFUSALS.contains(&m.as_str()), "{x} Np: {m}");
            // outside f32's normal range: below 2^-126 or past f32::MAX, or a tie
            let want = x.exp();
            let f = f64::from(f32::MAX);
            assert!(
                !(1.2e-38..=f).contains(&want) || is_f32_tie(want),
                "{x} Np refused: {m}"
            );
        }
    }
}

/// `x` within f64's rounding of halfway between two f32s.
#[expect(
    clippy::as_conversions,
    clippy::cast_possible_truncation,
    reason = "rounding to the nearest f32 is the question"
)]
fn is_f32_tie(x: f64) -> bool {
    let f = x as f32;
    let ulp = f64::from(f32::from_bits(f.to_bits() + 1)) - f64::from(f);
    ((x - f64::from(f)).abs() - ulp / 2.0).abs() <= x * 1e-15
}

/// A level written out parses back: `num · 10^-decimals` dB as its digits with the point `decimals` from the end.
/// `i64::MIN` is left out: its digits don't fit the positive `i64` the parser builds them in.
pub(crate) fn decibels_round_trip(num: i64, decimals: u8) {
    if num == i64::MIN {
        return;
    }
    let digits = format!("{:0>w$}", num.unsigned_abs(), w = usize::from(decimals) + 1);
    let (int, frac) = digits.split_at(digits.len() - usize::from(decimals));
    let sign = if num < 0 { "-" } else { "" };
    let s = if decimals == 0 {
        format!("{sign}{int}")
    } else {
        format!("{sign}{int}.{frac}")
    };
    assert_eq!(Decibels::parse(&s), Decibels::new(num, decimals), "{s}");
}

/// `Prefix::from_name(s)` is the named prefix whose name is `s`, nothing else: never a family or `Unit` for a name
/// that only looks like one.
pub(crate) fn prefix_name(s: &str) {
    let want = Prefix::ALL.into_iter().find(|p| p.name() == s);
    assert_eq!(Prefix::from_name(s), want, "{s:?}");
    if let Some(p) = want {
        assert_eq!(Prefix::from_name(p.name()), Some(p));
    }
}

/// The named units `units::by_name` looks through: `units::ALL`, and the customary ones where enabled.
fn all_units() -> impl Iterator<Item = &'static UnitExpr> {
    let all = units::ALL.iter();
    #[cfg(feature = "customary")]
    let all = all.chain(units::CUSTOMARY);
    all.copied()
}

/// `units::by_name(s)` is the named unit of this crate whose name is `s`, and that unit's name is `s`.
pub(crate) fn unit_name(s: &str) {
    match units::by_name(s) {
        Some(e @ UnitExpr::Named { name, .. }) => {
            assert_eq!(*name, s);
            assert!(all_units().any(|u| u == e), "{s:?}");
        }
        Some(e) => panic!("{s:?} names a compound unit {e:?}"),
        None => assert!(
            all_units().all(|u| !matches!(u, UnitExpr::Named { name, .. } if *name == s)),
            "{s:?} is a unit by_name doesn't find"
        ),
    }
}

/// Every name the crate lists finds its own prefix or unit: the name a dynamic client shows parses back.
pub(crate) fn names_round_trip() {
    for p in Prefix::ALL {
        prefix_name(p.name());
    }
    for u in all_units() {
        if let UnitExpr::Named { name, .. } = u {
            assert_eq!(units::by_name(name), Some(u), "{name}");
        }
    }
}

/// Every string a fuzz target gets: each parser above.
pub(crate) fn any_text(s: &str) {
    decimal(s);
    decibels(s);
    prefix_name(s);
    unit_name(s);
}

#[cfg(not(any(kani, fuzzing)))]
mod tests {
    use proptest::prelude::*;

    /// Strings near the grammar: signs, spaces, digits, `_`, points and exponents, and now and then a stray byte.
    fn literal() -> impl Strategy<Value = String> {
        prop_oneof![
            4 => "[-+]? ?[0-9_]{0,22}(\\.[0-9_]{0,22})?([eE][-+]?[0-9_]{0,7})?",
            1 => "[-+ 0-9_.eE]{0,30}",
            1 => any::<String>(),
            1 => "[0-9]{0,20}[^0-9]{1,3}[0-9]{0,5}",
        ]
    }

    #[test]
    fn edges() {
        for s in [
            "",
            "-",
            "+",
            " ",
            "- ",
            ".",
            "_",
            "e",
            "1e",
            "1e-",
            "1e_",
            ".e5",
            "1.",
            ".5",
            "-.5",
            "+ 1",
            "1..2",
            "1.2.3",
            "1e5e5",
            "1e5.",
            "9223372036854775807",
            "-9223372036854775807",
            "9223372036854775808",
            "-9223372036854775808",
            "0.000000000000000000000000001",
            "1e32767",
            "1e32768",
            "1e-32768",
            "1e-32769",
            "10e32767",
            "0.1e-32767",
            "1e99999",
            "1e100000",
            "1e0000000000000001",
            "1E5",
            "1_000",
            "0.0000000001",
            "0.000000001",
            "1f32",
            "١",
            "1\u{0}",
            "−3",
        ] {
            super::any_text(s);
        }
    }

    #[test]
    fn names() {
        super::names_round_trip();
        for s in [
            "",
            "milli",
            "MILLI",
            "Pow10",
            "Pow2",
            "FullScale",
            "Unknown",
            "Unit",
            "Volt ",
            "volt",
        ] {
            super::any_text(s);
        }
    }

    proptest! {
        #[test]
        fn decimals(s in literal()) {
            super::decimal(&s);
        }

        /// Zero written every way, with and without a sign: `-0` is refused, the others are 0.
        #[test]
        fn zeros(sign in prop_oneof![Just(""), Just("-"), Just("+"), Just("- ")], zeros in "0{1,3}(\\.0{0,3})?(e-?[0-9]{1,2})?") {
            super::decimal(&format!("{sign}{zeros}"));
        }

        #[test]
        fn levels(s in literal()) {
            super::decibels(&s);
        }

        /// Gains over the whole range of levels: far past every storage, and between.
        #[test]
        fn gains(num in prop_oneof![any::<i64>(), -400_000i64..=400_000], decimals in 0..=9u8) {
            super::level(num, decimals);
        }

        /// Gains in nepers over the whole range of levels, far past every storage and between.
        #[test]
        fn neper_gains(num in prop_oneof![any::<i64>(), -100_000i64..=100_000], decimals in 0..=9u8) {
            super::neper_level(num, decimals);
        }

        #[test]
        fn levels_round_trip(num in any::<i64>(), decimals in 0..=9u8) {
            super::decibels_round_trip(num, decimals);
        }

        #[test]
        fn names_and_strings(s in prop_oneof!["[A-Za-z]{0,12}", any::<String>()]) {
            super::prefix_name(&s);
            super::unit_name(&s);
        }
    }
}
