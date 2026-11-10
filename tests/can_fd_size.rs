//! A CAN FD length and its DLC through `OneOf<CanFdLengths>` and `Coded<CanFdDlc>` cost no more flash on a
//! Cortex-M4F than the same written by hand on a `u8` and a table of 16 (AGENTS.md rule 6: the safe path adds no
//! work): builds `tools/can_fd_size`'s two images, `typed` and `hand`, for `thumbv7em-none-eabihf` at a firmware's
//! release profile (`opt-level = "s"`, LTO) and compares the bytes they put in flash, every allocated section that
//! has contents (`.text`, `.rodata`, `.ARM.exidx`, ..). Skipped, with a note, where the target isn't installed
//! (`rustup target add thumbv7em-none-eabihf`); CI's `test` job installs it.
#![cfg(all(feature = "information", feature = "std"))]
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
    reason = "the sizes, and the note when the target isn't installed, are what a reader of the run looks for"
)]

use std::path::{Path, PathBuf};
use std::process::Command;

/// The target, a Cortex-M4F as the CAN FD firmware that uses these types runs on.
const TARGET: &str = "thumbv7em-none-eabihf";

/// This crate's root, to build `tools/can_fd_size` from any working directory.
fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf()
}

/// Builds both images into `target_dir`, `None` where the target isn't installed.
fn build(target_dir: &Path) -> Option<()> {
    let out = Command::new(env!("CARGO"))
        .arg("build")
        .arg("--release")
        .arg("--manifest-path")
        .arg(root().join("tools/can_fd_size/Cargo.toml"))
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
        "tools/can_fd_size didn't build:\n{stderr}"
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
fn the_typed_can_fd_length_and_dlc_cost_what_hand_written_ones_do() {
    let target_dir = root().join("target/can_fd_size");
    if build(&target_dir).is_none() {
        println!("skipped: {TARGET} isn't installed (rustup target add {TARGET})");
        return;
    }
    let size = |bin: &str| {
        let path = target_dir.join(TARGET).join("release").join(bin);
        flash(&std::fs::read(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display())))
    };
    let (typed, hand) = (size("typed"), size("hand"));
    println!("flash on {TARGET}: typed {typed} B, hand-written {hand} B");
    assert!(
        typed <= hand,
        "OneOf<CanFdLengths> and Coded<CanFdDlc> take {typed} B of flash, the hand-written code {hand} B"
    );
}
