//! A PQ encode by the float-free certified edge ([encode_certified](exact_units::codec::CurveDef::encode_certified),
//! integer intervals) beside the certified `f64` one ([encode_f64](exact_units::codec::CurveDef::encode_f64), `libm`'s
//! `pow` in software `f64`, the M4F's FPU being single precision): builds `tools/curve_size`'s two images,
//! `certified` and `float`, for `thumbv7em-none-eabihf` at a firmware's release profile (`opt-level = "s"`, LTO) and
//! measures the bytes each puts in flash, every allocated section that has contents. Both carry the exact path and
//! `Fraction`'s arithmetic, most of either image; the float-free edge costs at most 4 KiB beyond the `f64` one, and
//! needs neither `libm` nor an FPU. `docs/fixed_point/video.md` quotes the numbers this prints. Skipped, with a note,
//! where the target isn't installed (`rustup target add thumbv7em-none-eabihf`); CI's `test` job installs it.
#![cfg(all(feature = "libm", feature = "std"))]
#![expect(
    clippy::expect_used,
    clippy::panic,
    reason = "the test fails by panicking where the build or the ELF isn't what it reads"
)]
#![expect(
    clippy::arithmetic_side_effects,
    reason = "offsets within an ELF image of a few tens of kB, which fit"
)]
#![expect(
    clippy::print_stdout,
    reason = "the sizes, and the note when the target isn't installed, are what a reader of the run looks for"
)]

use std::path::{Path, PathBuf};
use std::process::Command;

/// The target, a Cortex-M4F: an FPU for `f32` alone, so `f64` runs in software.
const TARGET: &str = "thumbv7em-none-eabihf";

/// This crate's root, to build `tools/curve_size` from any working directory.
fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf()
}

/// Builds both images into `target_dir`, `None` where the target isn't installed.
fn build(target_dir: &Path) -> Option<()> {
    let out = Command::new(env!("CARGO"))
        .arg("build")
        .arg("--release")
        .arg("--manifest-path")
        .arg(root().join("tools/curve_size/Cargo.toml"))
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
        "tools/curve_size didn't build:\n{stderr}"
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
fn the_float_free_edge_costs_at_most_4_kib_beyond_the_f64_one() {
    let target_dir = root().join("target/curve_size");
    if build(&target_dir).is_none() {
        println!("skipped: {TARGET} isn't installed (rustup target add {TARGET})");
        return;
    }
    let size = |bin: &str| {
        let path = target_dir.join(TARGET).join("release").join(bin);
        flash(&std::fs::read(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display())))
    };
    let (certified, float) = (size("certified"), size("float"));
    println!("flash on {TARGET}: certified {certified} B, f64 {float} B");
    assert!(
        certified <= float + 4096,
        "the float-free edge takes {certified} B of flash, the f64 one {float} B"
    );
}
