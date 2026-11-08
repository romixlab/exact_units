// The property functions of tests/props, compiled into each target by path: the same functions the proptests call.
// `cargo fuzz` sets cfg(fuzzing), which leaves out their proptest blocks and derives `Arbitrary` for their inputs.
#![allow(
    dead_code,
    reason = "each target calls a few of the property functions"
)]

#[path = "../../tests/props/exact_text.rs"]
pub mod exact_text;
#[path = "../../tests/props/nan.rs"]
pub mod nan;
#[path = "../../tests/props/panics.rs"]
pub mod panics;
#[path = "../../tests/props/q_text.rs"]
pub mod q_text;
#[path = "../../tests/props/reference.rs"]
pub mod reference;
#[path = "../../tests/props/serde_input.rs"]
pub mod serde_input;
#[path = "../../tests/props/text.rs"]
pub mod text;
#[path = "../../tests/props/wire.rs"]
pub mod wire;
