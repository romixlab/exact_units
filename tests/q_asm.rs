//! `speed < q!(100 m/s)` is the same code as `speed < LIMIT` for a typed `const`, and `gain < q!(-3 dB)` the same
//! as `gain.get() <= 707`: builds `tools/q_literal_asm` for a Cortex-M0 (`thumbv6m-none-eabi`) and a Cortex-M4F
//! (`thumbv7em-none-eabihf`, for the float compare in its FPU) with `--emit asm` and compares the normalised
//! instructions of each pair named in its doc comment. Skipped, with a note, where a target isn't installed
//! (`rustup target add thumbv6m-none-eabi thumbv7em-none-eabihf`); run in CI (`.github/workflows/ci.yml`, job
//! `test`) and from `just test`.
//!
//! Labels are normalised, not compared literally: LLVM numbers a function's local labels (`.LBB0_2`,
//! `.LCPI0_0`) by its position in the file, so `below_right_angle`'s own compare reads `.LBB2_2`, the same
//! shape under a different number. A function this test says is another's body may come out of
//! `tools/q_literal_asm/src/lib.rs` as an alias (`below_const = at_most_raw`, what the compiler did when
//! this test was written) or as a body of its own with identical instructions (what a future LLVM might do
//! instead): this test resolves an alias to its body before comparing, so either shape passes.
#![cfg(all(feature = "q", feature = "std"))]
#![expect(
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "the parser below fails the test by panicking, on asm that isn't in the shape it reads"
)]
#![expect(
    clippy::arithmetic_side_effects,
    reason = "a test adds line positions within one asm file, which fit"
)]
#![expect(
    clippy::print_stdout,
    reason = "the note when the target isn't installed is this test's whole point"
)]

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

/// This crate's root, to run `cargo rustc` against `tools/q_literal_asm` from any working directory.
fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf()
}

/// Builds `tools/q_literal_asm` for `target` with `--emit asm` into `target_dir`, `cargo`'s own
/// build directory as well as its target directory. The `.s` is a build artifact `cargo` never uplifts to a
/// public target directory (only `--emit`'s declared kinds, `link` and `metadata`, are): pointing
/// `--target-dir` at a directory cargo also uses as its *build* directory (`--config build.build-dir=..`)
/// keeps it there to be read; a host whose cargo config gives builds a build directory of their own (ops'
/// build nodes do, for smaller per-workspace target directories) throws it away unseen otherwise. A host
/// without that config builds the same either way.
fn build(target_dir: &Path, target: &str) -> Output {
    Command::new(env!("CARGO"))
        .current_dir(root())
        .arg("rustc")
        .arg("--release")
        .arg("--manifest-path")
        .arg(root().join("tools/q_literal_asm/Cargo.toml"))
        .arg("--target")
        .arg(target)
        .arg("--target-dir")
        .arg(target_dir)
        .arg("--config")
        .arg(format!("build.build-dir=\"{}\"", target_dir.display()))
        .arg("--")
        .arg("--emit")
        .arg("asm")
        .output()
        .expect("cargo rustc runs")
}

/// The one `.s` file `build` emits, `q_literal_asm`'s own (cargo passes `--emit asm` only to the package
/// named by `--manifest-path`, never to `exact_units` or `exact_units_macros`).
fn asm_file(target_dir: &Path, target: &str) -> String {
    let deps = target_dir.join(target).join("release/deps");
    let file = std::fs::read_dir(&deps)
        .unwrap_or_else(|e| panic!("{}: {e}", deps.display()))
        .filter_map(Result::ok)
        .find(|e| {
            let name = e.file_name();
            let name = name.to_string_lossy();
            name.starts_with("q_literal_asm-") && name.ends_with(".s")
        })
        .unwrap_or_else(|| panic!("no q_literal_asm-*.s in {}", deps.display()))
        .path();
    std::fs::read_to_string(&file).unwrap_or_else(|e| panic!("{}: {e}", file.display()))
}

/// `name`'s body in `asm`: the lines from its label to the `.Lfunc_end*` marker that follows. `None` where
/// `asm` has no such label (an alias, `name = target`, has none).
fn body<'a>(asm: &'a str, name: &str) -> Option<Vec<&'a str>> {
    let lines: Vec<&str> = asm.lines().collect();
    let label = format!("{name}:");
    let start = lines.iter().position(|l| l.trim() == label)?;
    let end = lines[start + 1..]
        .iter()
        .position(|l| l.trim_start().starts_with(".Lfunc_end"))
        .unwrap_or_else(|| panic!("{name}: no .Lfunc_end after its label"));
    Some(lines[start + 1..start + 1 + end].to_vec())
}

/// `name`'s alias target, `asm`'s shape for a function LLVM merged into another's body: `name = target`, one
/// line, no label of its own.
fn alias_of(asm: &str, name: &str) -> Option<String> {
    let prefix = format!("{name} = ");
    asm.lines()
        .find_map(|l| l.trim().strip_prefix(&prefix))
        .map(ToOwned::to_owned)
}

/// `name`'s own body: its label's, or (`asm` merged it into another function) the alias target's, one level
/// deep (every name this test reads is either a label with a body or an alias to one).
fn resolve<'a>(asm: &'a str, name: &str) -> Vec<&'a str> {
    match alias_of(asm, name) {
        Some(target) => body(asm, &target)
            .unwrap_or_else(|| panic!("{name} = {target}, but {target} has no body")),
        None => {
            body(asm, name).unwrap_or_else(|| panic!("{name}: no label and no alias in the asm"))
        }
    }
}

/// Whether `token` is a function-local label LLVM numbers by file position (`.LBB0_2`, `.LCPI0_0`): compared
/// normalised, not literally, as the number differs by position.
fn is_local_label(token: &str) -> bool {
    token.starts_with(".LBB") || token.starts_with(".LCPI")
}

/// `s`'s words: split on whitespace, `,`, `:`, `{` and `}`, so a label's name reads the same whether it is
/// `.LBB0_2:` (a definition) or an operand reading it.
fn words(s: &str) -> impl Iterator<Item = &str> {
    s.split(|c: char| c.is_whitespace() || matches!(c, ',' | ':' | '{' | '}'))
        .filter(|w| !w.is_empty())
}

/// Runs of whitespace in `s` collapsed to one space, trimmed; every other character (`,`, `:`, `{`, `}`) as
/// written, so a label definition keeps its trailing `:`.
fn collapse(s: &str) -> String {
    let mut out = String::new();
    let mut space = false;
    for c in s.trim().chars() {
        if c.is_whitespace() {
            if !space {
                out.push(' ');
            }
            space = true;
        } else {
            out.push(c);
            space = false;
        }
    }
    out
}

/// `body`'s instructions and data, directives of layout and unwind info dropped (`.fnstart`, `.save`,
/// `.setfp`, `.p2align`: none change what runs), local labels renamed `L0`, `L1`, .. by the order they're
/// first read (a definition or an operand, whichever comes first): the same function however LLVM numbered
/// its labels.
fn normalize(body: &[&str]) -> Vec<String> {
    let kept: Vec<String> = body
        .iter()
        .map(|l| l.trim())
        .filter(|l| !l.is_empty())
        .filter(|l| {
            !(l.starts_with(".fnstart")
                || l.starts_with(".save")
                || l.starts_with(".setfp")
                || l.starts_with(".p2align"))
        })
        .map(collapse)
        .collect();

    let mut names: Vec<&str> = Vec::new();
    for line in &kept {
        for w in words(line).filter(|w| is_local_label(w)) {
            if !names.contains(&w) {
                names.push(w);
            }
        }
    }
    // the longer token first: a label here is never a prefix of another's, but a general-case safeguard
    let mut mapping: Vec<(&str, String)> = names
        .iter()
        .enumerate()
        .map(|(i, n)| (*n, format!("L{i}")))
        .collect();
    mapping.sort_by_key(|(n, _)| core::cmp::Reverse(n.len()));

    kept.iter()
        .map(|line| {
            let mut out = line.clone();
            for (from, to) in &mapping {
                out = out.replace(from, to);
            }
            out
        })
        .collect()
}

/// The normalised body of a single bounded compare, `t < c` folded to `t <= c'`: sign-extend where the
/// storage is narrower than a register, load `c'` from the constant pool, compare, branch strictly less
/// (`blt`), return `true` or `false`. `at_most_raw` and `below_right_angle` are both this shape, each with
/// its own `c'` and sign-extend.
fn expected_compare(sign_extend: bool, c: i64) -> Vec<String> {
    let mut v = vec!["push {r7, lr}".to_owned(), "add r7, sp, #0".to_owned()];
    if sign_extend {
        v.push("sxth r0, r0".to_owned());
    }
    v.extend([
        "ldr r1, L0".to_owned(),
        "cmp r0, r1".to_owned(),
        "blt L1".to_owned(),
        "movs r0, #0".to_owned(),
        "pop {r7, pc}".to_owned(),
        "L1:".to_owned(),
        "movs r0, #1".to_owned(),
        "pop {r7, pc}".to_owned(),
        "L0:".to_owned(),
        format!(".long {c}"),
    ]);
    v
}

/// `below_literal`, `below_literal_kmh`, `below_const` and `below_inexact` (`tools/q_literal_asm/src/lib.rs`)
/// are `at_most_raw`'s body, written by hand on the stored value: `speed < q!(100 m/s)` and its typed-const,
/// km/h and inexact-decimal forms compile to the same instructions as `speed.get() <= 9_999`.
fn same_as_at_most_raw(asm: &str, name: &str) {
    assert_eq!(
        normalize(&resolve(asm, name)),
        normalize(&resolve(asm, "at_most_raw")),
        "{name} should be at_most_raw's body (the typed-const compare), but its asm differs"
    );
}

/// `tools/q_literal_asm`'s asm for `target`, `None` (with a note) where the target isn't installed.
fn asm_for(target: &str) -> Option<String> {
    let target_dir = root().join("tools/q_literal_asm/target/asm_test");
    let output = build(&target_dir, target);
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        if stderr.contains("E0463") {
            println!(
                "q_asm: skipped, the `{target}` target isn't installed here (`rustup target add {target}`); it \
                 runs in CI and on a machine that has it"
            );
            return None;
        }
        panic!("cargo rustc tools/q_literal_asm --target {target} failed:\n{stderr}");
    }
    Some(asm_file(&target_dir, target))
}

#[test]
fn speed_literal_is_the_typed_const_compare() {
    let Some(asm) = asm_for("thumbv6m-none-eabi") else {
        return;
    };

    // speed < q!(100 m/s) on CmPerS (steps of 1 cm/s, i16): 100 m/s is 10_000 steps, so `speed < x` is
    // `speed <= 9_999`, at_most_raw's own body (its `speed.get() <= 9_999`, written by hand).
    assert_eq!(
        normalize(&resolve(&asm, "at_most_raw")),
        expected_compare(true, 10000),
        "at_most_raw (speed.get() <= 9_999, hand-written) should be one sign-extend, load, compare, branch"
    );
    for name in [
        "below_literal",
        "below_literal_kmh",
        "below_const",
        "below_inexact",
    ] {
        same_as_at_most_raw(&asm, name);
    }

    // q!(1000 m/s) on CmPerS: 100_000 steps is past i16's range, so every stored value compares below it, a
    // constant with no compare left.
    let past_range = normalize(&resolve(&asm, "below_past_range"));
    assert_eq!(
        past_range,
        [
            "push {r7, lr}",
            "add r7, sp, #0",
            "movs r0, #1",
            "pop {r7, pc}"
        ],
        "below_past_range (q!(1000 m/s) past i16's range) should be a bare `true`"
    );

    // q!(90 deg) against steps of 1 mrad (Radian, not Speed, so never at_most_raw's alias): 90° is
    // 1570.796.. mrad, floor 1570, not exact (π in the scale), so `angle < x` is `angle <= 1570`.
    assert_eq!(
        normalize(&resolve(&asm, "below_right_angle")),
        expected_compare(true, 1571),
        "below_right_angle (q!(90 deg) against mrad) should compare against 1571 (<= 1570 is < 1571)"
    );
}

/// The instructions of `body` that compare: `cmp`, `cmp.w`, `vcmp.f32`, or a call to the soft-float compare a
/// Cortex-M0 has instead of an FPU (`__aeabi_fcmple`).
fn compares(body: &[String]) -> usize {
    body.iter()
        .filter(|l| {
            let op = l.split(' ').next().unwrap_or("");
            op == "cmp" || op == "cmp.w" || op.starts_with("vcmp") || l.contains("__aeabi_fcmp")
        })
        .count()
}

/// `gain < q!(-3 dB)` is one compare against the floor of 10^(−3/20) in the gain's steps, the same instructions as
/// written by hand on the stored value: `gain_below_level` is `gain_at_most_raw`'s body (`gain.get() <= 707`) and
/// `gain_f32_below_level` is `gain_f32_at_most_raw`'s (`<=` the largest `f32` below the gain), on a Cortex-M0 and
/// on a Cortex-M4F, where the float compare is one `vcmp.f32` in the FPU.
#[test]
fn gain_level_is_one_compare() {
    for target in ["thumbv6m-none-eabi", "thumbv7em-none-eabihf"] {
        let Some(asm) = asm_for(target) else {
            continue;
        };
        for (level, raw) in [
            ("gain_below_level", "gain_at_most_raw"),
            ("gain_f32_below_level", "gain_f32_at_most_raw"),
        ] {
            let body = normalize(&resolve(&asm, raw));
            assert_eq!(
                normalize(&resolve(&asm, level)),
                body,
                "{target}: {level} should be {raw}'s body, but its asm differs"
            );
            assert_eq!(
                compares(&body),
                1,
                "{target}: {raw} should be one compare: {body:#?}"
            );
        }
        let f32_body = normalize(&resolve(&asm, "gain_f32_at_most_raw"));
        assert!(
            f32_body
                .iter()
                .any(|l| l.contains("1060453359") || l.contains("0x3f353bef")),
            "{target}: gain_f32_at_most_raw should compare against the f32 0x3f353bef: {f32_body:#?}"
        );
        if target == "thumbv7em-none-eabihf" {
            assert!(
                f32_body.iter().any(|l| l.starts_with("vcmp.f32")),
                "{target}: the f32 compare should be the FPU's vcmp.f32: {f32_body:#?}"
            );
        }
    }
}

/// A `q!` range is the two compares written by hand, `4900 <= t && t <= 5100`, whichever form wrote it; an end past
/// the storage's range leaves one compare.
#[test]
fn range_literal_is_the_hand_written_compares() {
    let Some(asm) = asm_for("thumbv6m-none-eabi") else {
        return;
    };
    let raw = normalize(&resolve(&asm, "in_range_raw"));
    assert!(
        compares(&raw) <= 2,
        "in_range_raw (4900 <= t && t <= 5100 by hand) should be at most two compares: {raw:#?}"
    );
    for name in [
        "in_range_literal",
        "in_tolerance_literal",
        "in_range_inside",
    ] {
        assert_eq!(
            normalize(&resolve(&asm, name)),
            raw,
            "{name} should be in_range_raw's body (the hand-written limits), but its asm differs"
        );
    }
    let past = normalize(&resolve(&asm, "in_range_past"));
    assert_eq!(
        past,
        normalize(&resolve(&asm, "at_least_raw")),
        "in_range_past (q!(4.9 V ..= 40 V), the upper end past i16) should be at_least_raw's body"
    );
    assert_eq!(
        compares(&past),
        1,
        "in_range_past should be one compare: {past:#?}"
    );
}

/// A `q!` range of points and one of levels are the two compares written by hand too: `q!(68 degF ..= 77 degF)`
/// against a point in 0.01 K from 0 °C is `2000 <= t && t <= 2500`, `q!(-3 dB ..= 3 dB, Inside)` against a gain in
/// mV/V `708 <= t && t <= 1412`.
#[test]
fn point_and_level_ranges_are_the_hand_written_compares() {
    let Some(asm) = asm_for("thumbv6m-none-eabi") else {
        return;
    };
    for (name, raw) in [
        ("point_in_range_literal", "point_in_range_raw"),
        ("gain_in_band_literal", "gain_in_band_raw"),
    ] {
        let raw_body = normalize(&resolve(&asm, raw));
        assert!(
            compares(&raw_body) <= 2,
            "{raw} (the limits by hand) should be at most two compares: {raw_body:#?}"
        );
        assert_eq!(
            normalize(&resolve(&asm, name)),
            raw_body,
            "{name} should be {raw}'s body (the hand-written limits), but its asm differs"
        );
    }
}

/// A NaN-free float's sum compared with `gt_or` costs what plain `f32` code does, on a Cortex-M4F:
/// `(a + b).gt_or(l, true)` is `!(a + b <= l)` on plain `f32`s, and as many instructions as `a + b > l`, one
/// `vadd.f32` and one `vcmp.f32` each; only the condition differs (`hi`, true where the compare is unordered, for
/// `gt`). The book's page "Floats: NaN and infinity" shows the two bodies.
#[test]
fn nan_free_sum_compare_is_the_plain_float_code() {
    let target = "thumbv7em-none-eabihf";
    let Some(asm) = asm_for(target) else {
        return;
    };
    let instructions = |name: &str| -> Vec<String> {
        normalize(&resolve(&asm, name))
            .into_iter()
            .filter(|l| !l.ends_with(':') && !l.starts_with('.'))
            .collect()
    };
    let fixed = instructions("fixed_sum_above");
    assert_eq!(
        fixed,
        instructions("plain_sum_not_at_most"),
        "{target}: fixed_sum_above should be !(a + b <= l) on plain f32s"
    );
    let plain = instructions("plain_sum_above");
    assert_eq!(
        fixed.len(),
        plain.len(),
        "{target}: gt_or should cost what a + b > l does: {fixed:#?} against {plain:#?}"
    );
    for body in [&fixed, &plain] {
        for op in ["vadd.f32", "vcmp.f32"] {
            assert_eq!(
                body.iter().filter(|l| l.starts_with(op)).count(),
                1,
                "{target}: one {op}: {body:#?}"
            );
        }
    }
}
