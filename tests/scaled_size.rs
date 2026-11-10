//! A count at a time base read at run time ([Ticks](exact_units::Ticks), `u32` and `u64`) into 90 kHz ticks by a
//! named rounding, beside the same conversion between two compile-time scales (`Fixed`): builds `tools/scaled_size`'s
//! three images for `thumbv7em-none-eabihf` (Cortex-M4F) and `thumbv6m-none-eabi` (Cortex-M0+) at a firmware's
//! release profile (`opt-level = "s"`, LTO) and checks what they link. The run-time scale costs at most 6 KiB of
//! flash on either core (the compile-time one folds to a few instructions), and links no float routine and no 128-bit
//! arithmetic: the kernel's 32-bit limbs and the core's own `u64` division. `docs/numbers/run_time_scale.md` quotes
//! the numbers this prints. Skipped, with a note, where a target isn't installed (`rustup target add ..`); CI's
//! `test` job installs both.

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
    reason = "the sizes, and the note when a target isn't installed, are what a reader of the run looks for"
)]

use std::path::{Path, PathBuf};
use std::process::Command;

/// A Cortex-M4F and a Cortex-M0+, which has no divide instruction.
const TARGETS: [&str; 2] = ["thumbv7em-none-eabihf", "thumbv6m-none-eabi"];

/// The most flash a run-time conversion may take, either storage, either core.
const MAX_RUNTIME: usize = 6 * 1024;

/// Routines a run-time conversion must not link: soft floats and 128-bit arithmetic.
const FORBIDDEN: [&str; 9] = [
    "__aeabi_d",
    "__aeabi_f",
    "__aeabi_i2d",
    "__aeabi_ul2d",
    "__udivti3",
    "__umodti3",
    "__multi3",
    "u128_div_rem",
    "libm",
];

/// This crate's root, to build `tools/scaled_size` from any working directory.
fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf()
}

/// Builds the images for `target` into `target_dir`, `None` where the target isn't installed.
fn build(target: &str, target_dir: &Path) -> Option<()> {
    let out = Command::new(env!("CARGO"))
        .arg("build")
        .arg("--release")
        .arg("--manifest-path")
        .arg(root().join("tools/scaled_size/Cargo.toml"))
        .arg("--target")
        .arg(target)
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
        "tools/scaled_size didn't build for {target}:\n{stderr}"
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

/// The section headers of an ELF32 image: their offsets in the file.
fn sections(elf: &[u8]) -> impl Iterator<Item = usize> + '_ {
    assert_eq!(
        elf.get(..5),
        Some(&b"\x7fELF\x01"[..]),
        "not an ELF32 image"
    );
    let (shoff, shentsize, shnum) = (le(elf, 0x20, 4), le(elf, 0x2E, 2), le(elf, 0x30, 2));
    (0..shnum).map(move |i| shoff + i * shentsize)
}

/// The bytes an image puts in flash: the sizes of its sections that are allocated (`SHF_ALLOC`) and have contents in
/// the file (not `SHT_NOBITS`, `.bss`).
fn flash(elf: &[u8]) -> usize {
    sections(elf)
        .filter(|&h| le(elf, h + 8, 4) & 0x2 != 0 && le(elf, h + 4, 4) != 8)
        .map(|h| le(elf, h + 20, 4))
        .sum()
}

/// The names in the image's symbol table (`SHT_SYMTAB`, its strings in the section it links to).
#[expect(
    clippy::indexing_slicing,
    reason = "offsets read from the ELF's own headers; a wrong one fails the test"
)]
fn symbols(elf: &[u8]) -> Vec<String> {
    let headers: Vec<usize> = sections(elf).collect();
    let symtab = *headers
        .iter()
        .find(|&&h| le(elf, h + 4, 4) == 2)
        .expect("a symbol table");
    let strtab = headers[le(elf, symtab + 24, 4)];
    let (off, size) = (le(elf, symtab + 16, 4), le(elf, symtab + 20, 4));
    let strings = le(elf, strtab + 16, 4);
    (off..off + size)
        .step_by(16)
        .map(|entry| {
            let start = strings + le(elf, entry, 4);
            let end = elf[start..]
                .iter()
                .position(|&b| b == 0)
                .map_or(start, |n| start + n);
            String::from_utf8_lossy(&elf[start..end]).into_owned()
        })
        .collect()
}

#[test]
fn a_run_time_scale_costs_at_most_6_kib_and_links_no_float_or_128_bit_code() {
    let target_dir = root().join("target/scaled_size");
    for target in TARGETS {
        if build(target, &target_dir).is_none() {
            println!("skipped: {target} isn't installed (rustup target add {target})");
            continue;
        }
        let read = |bin: &str| {
            let path = target_dir.join(target).join("release").join(bin);
            std::fs::read(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
        };
        let fixed = flash(&read("fixed"));
        let mut sizes = Vec::new();
        for bin in ["ticks_u32", "ticks_u64"] {
            let elf = read(bin);
            let size = flash(&elf);
            sizes.push(format!("{bin} {size} B"));
            assert!(
                size <= MAX_RUNTIME,
                "{bin} takes {size} B of flash on {target}"
            );
            for name in symbols(&elf) {
                assert!(
                    !FORBIDDEN.iter().any(|f| name.contains(f)),
                    "{bin} links {name} on {target}"
                );
            }
        }
        println!("flash on {target}: Fixed {fixed} B, {}", sizes.join(", "));
    }
}
