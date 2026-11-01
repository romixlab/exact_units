//! Properties of the crate, each against an exact reference, checked two ways from one function: a proptest draws
//! its inputs (`just test`), a Kani proof takes them symbolic, all of them at once (`just verify`). A property is a
//! plain `pub fn` that takes its inputs and asserts; the proptests and proofs beside it only call it. Where an input
//! space is small and cheap (every i8, every i16), a test walks it whole.
#![cfg(feature = "quantities")]
#![expect(
    clippy::as_conversions,
    reason = "a test casts the values it chose and the references it computes; the cast lints say why each fits"
)]

mod arith;
mod format;
mod literal;
mod no_panic;
mod reference;
mod rounding;
#[cfg(feature = "serde")]
mod serde_values;

/// Inputs for the proptests: integers weighted to the ends and to 0, where conversions overflow and round.
#[cfg(not(kani))]
mod strategy {
    use super::reference::{MODES, POLICIES};
    use exact_units::{Overflow, Rounding};
    use proptest::prelude::*;
    use proptest::sample::select;

    macro_rules! ints {
        ($($t:ident)*) => {$(
            /// Anything, the ends and a step in from them, or near 0.
            pub(crate) fn $t() -> impl Strategy<Value = $t> {
                prop_oneof![
                    4 => any::<$t>(),
                    1 => select(&[$t::MIN, $t::MIN + 1, $t::MAX - 1, $t::MAX][..]),
                    2 => -100..=100 as $t,
                ]
            }
        )*};
    }

    ints!(i8 i16 i32 i64);

    pub(crate) fn mode() -> impl Strategy<Value = Rounding> {
        select(&MODES[..])
    }

    pub(crate) fn policy() -> impl Strategy<Value = Overflow> {
        select(&POLICIES[..])
    }
}
