//! Snapshots of the full rustc output for a few typical refused conversions (P2401#5nov-0954): the error message
//! can't grow long and unreadable again unnoticed. `pass_marker.rs` is a `pass()` fixture in the same batch: trybuild
//! runs `cargo check` for a batch of `compile_fail()` alone, which never reaches these panics (they fire only
//! post-monomorphization, during codegen); a `pass()` fixture switches the whole batch to `cargo build`.

#[test]
fn compile_fail() {
    let t = trybuild::TestCases::new();
    t.pass("tests/compile_fail/pass_marker.rs");
    t.compile_fail("tests/compile_fail/dim_mismatch.rs");
    t.compile_fail("tests/compile_fail/dim_mismatch_angle_hint.rs");
    t.compile_fail("tests/compile_fail/kind_mismatch.rs");
    t.compile_fail("tests/compile_fail/literal_mismatch.rs");
}

/// The refusals of `q!`, with the pass fixture so the batch is built, not checked (a wrong dimension panics only
/// post-monomorphization).
#[cfg(feature = "q")]
#[test]
fn compile_fail_q() {
    let t = trybuild::TestCases::new();
    t.pass("tests/compile_fail/pass_marker.rs");
    t.compile_fail("tests/compile_fail/q_dim_mismatch.rs");
    t.compile_fail("tests/compile_fail/q_float.rs");
    t.compile_fail("tests/compile_fail/q_symbols.rs");
    t.compile_fail("tests/compile_fail/q_grammar.rs");
    t.compile_fail("tests/compile_fail/q_point.rs");
    t.compile_fail("tests/compile_fail/fixed_dim_mismatch.rs");
    t.compile_fail("tests/compile_fail/fixed_grammar.rs");
}

/// A symbol of a unit group that is off: only where `customary` is (`just test`'s `std` run).
#[cfg(all(feature = "q", not(feature = "customary")))]
#[test]
fn compile_fail_feature_off() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/compile_fail/q_feature_off.rs");
}
