//! The integer scaling path across run-time prefixes (`Prefix::ratio_to`, `PrefixRatio::requantize` on `u32`, a power
//! of 2's ratio from `Prefix::from_pow2`) in the shape ww_si's `<u32 as ToIn>::of_value` and `scale_pow2` call it,
//! beside a `Fixed` conversion between two compile-time prefixes on `u32`: builds `tools/prefix_size`'s image for
//! `thumbv7em-none-eabihf` at a firmware's release profile (`opt-level = "s"`, LTO) and checks the bytes it puts in
//! flash, every allocated section that has contents. They stay at most what exact_units e362e57 took (7252 B), so a
//! change that copies `ratio_to`'s fast path back into each caller fails here, not in a firmware later. Skipped, with
//! a note, where the target isn't installed (`rustup target add thumbv7em-none-eabihf`); CI's `test` job installs it.
#![expect(
    clippy::expect_used,
    clippy::panic,
    reason = "the test fails by panicking where the build or the ELF isn't what it reads"
)]
#![expect(
    clippy::arithmetic_side_effects,
    reason = "offsets within an ELF image of a few kB, which fit"
)]
#![expect(
    clippy::print_stdout,
    reason = "the size, and the note when the target isn't installed, are what a reader of the run looks for"
)]

use std::path::{Path, PathBuf};
use std::process::Command;

/// The target, a Cortex-M4F as the firmware that converts across prefixes with ww_si runs on.
const TARGET: &str = "thumbv7em-none-eabihf";

/// The flash the image took with exact_units at e362e57, before `ratio_to`'s fast path was a table: the most it may
/// take. The fix took it to 6726 B (rustc 1.99).
const MAX_FLASH: usize = 7252;

/// This crate's root, to build `tools/prefix_size` from any working directory.
fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf()
}

/// Builds the image into `target_dir`, `None` where the target isn't installed.
fn build(target_dir: &Path) -> Option<()> {
    let out = Command::new(env!("CARGO"))
        .arg("build")
        .arg("--release")
        .arg("--manifest-path")
        .arg(root().join("tools/prefix_size/Cargo.toml"))
        .arg("--target")
        .arg(TARGET)
        .arg("--target-dir")
        .arg(target_dir)
        .output()
        .expect("cargo build runs");
    let stderr = String::from_utf8_lossy(&out.stderr);
    if !out.status.success() && stderr.contains("may not be installed") {
        return None;
    }
    assert!(
        out.status.success(),
        "tools/prefix_size didn't build:\n{stderr}"
    );
    Some(())
}

/// A little-endian `u16` or `u32` of `elf` at `at`.
fn le(elf: &[u8], at: usize, len: usize) -> usize {
    let bytes = elf
        .get(at..at.saturating_add(len))
        .unwrap_or_else(|| panic!("the ELF ends before byte {at}"));
    bytes
        .iter()
        .rev()
        .fold(0, |n, &b| (n << 8) | usize::from(b))
}

/// The bytes an ELF32 image puts in flash: the sizes of its sections that are allocated (`SHF_ALLOC`) and have
/// contents in the file (not `SHT_NOBITS`, `.bss`).
fn flash(elf: &[u8]) -> usize {
    assert_eq!(
        elf.get(..5),
        Some(&b"\x7fELF\x01"[..]),
        "not an ELF32 image"
    );
    let (shoff, shentsize, shnum) = (le(elf, 0x20, 4), le(elf, 0x2E, 2), le(elf, 0x30, 2));
    (0..shnum)
        .map(|i| shoff + i * shentsize)
        .filter(|&h| le(elf, h + 8, 4) & 0x2 != 0 && le(elf, h + 4, 4) != 8)
        .map(|h| le(elf, h + 20, 4))
        .sum()
}

#[test]
fn scaling_across_run_time_prefixes_takes_no_more_flash_than_at_e362e57() {
    let target_dir = root().join("target/prefix_size");
    if build(&target_dir).is_none() {
        println!("skipped: {TARGET} isn't installed (rustup target add {TARGET})");
        return;
    }
    let path = target_dir.join(TARGET).join("release").join("prefix");
    let size = flash(&std::fs::read(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display())));
    println!("flash on {TARGET}: {size} B, at most {MAX_FLASH} B");
    assert!(
        size <= MAX_FLASH,
        "the integer scaling path takes {size} B of flash, {MAX_FLASH} B at exact_units e362e57"
    );
}
