//! Snapshots of the full rustc output for a few typical refused conversions (P2401#5nov-0954): the error message
//! can't grow long and unreadable again unnoticed. `pass_marker.rs` is a `pass()` fixture in the same batch: trybuild
//! runs `cargo check` for a batch of `compile_fail()` alone, which never reaches these panics (they fire only
//! post-monomorphization, during codegen); a `pass()` fixture switches the whole batch to `cargo build`.
//!
//! The snapshots need the toolchain's `rust-src` component (P2401#7nov-1747): rustc quotes the line of
//! `core/src/panic.rs` a const panic fails in only where that source is installed, and prints a bare `= note:` line
//! instead where it isn't (a `--profile minimal` toolchain, as CI's). Each test checks for it first and names the
//! fix, so a machine without it fails here, not with a diff that reads like a changed message.

use std::path::Path;
use std::process::Command;

/// Panics, naming `rustup component add rust-src`, where the toolchain has no `rust-src`.
#[expect(
    clippy::expect_used,
    reason = "a helper of the tests below: it fails them by panicking"
)]
fn require_rust_src() {
    let rustc = std::env::var("RUSTC").unwrap_or_else(|_| "rustc".into());
    let out = Command::new(rustc)
        .args(["--print", "sysroot"])
        .output()
        .expect("rustc runs");
    let sysroot = String::from_utf8(out.stdout).expect("the sysroot is UTF-8");
    let panic_rs = Path::new(sysroot.trim()).join("lib/rustlib/src/rust/library/core/src/panic.rs");
    assert!(
        panic_rs.exists(),
        "the .stderr snapshots of tests/compile_fail quote core's source, which this toolchain lacks \
         ({} is missing): run `rustup component add rust-src`",
        panic_rs.display()
    );
}

#[test]
fn compile_fail() {
    require_rust_src();
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
    require_rust_src();
    let t = trybuild::TestCases::new();
    t.pass("tests/compile_fail/pass_marker.rs");
    t.compile_fail("tests/compile_fail/q_dim_mismatch.rs");
    t.compile_fail("tests/compile_fail/q_float.rs");
    t.compile_fail("tests/compile_fail/q_symbols.rs");
    t.compile_fail("tests/compile_fail/q_grammar.rs");
    t.compile_fail("tests/compile_fail/q_point.rs");
    t.compile_fail("tests/compile_fail/fixed_dim_mismatch.rs");
    t.compile_fail("tests/compile_fail/fixed_grammar.rs");
    t.compile_fail("tests/compile_fail/fixed_point.rs");
}

/// A symbol of a unit group that is off: only where `customary` is (`just test`'s `std` run).
#[cfg(all(feature = "q", not(feature = "customary")))]
#[test]
fn compile_fail_feature_off() {
    require_rust_src();
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/compile_fail/q_feature_off.rs");
}
