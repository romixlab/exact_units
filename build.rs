//! `cfg(thumb1)` on a target whose cores run Thumb-1 alone (Cortex-M0, M0+, M1, M23; ARMv4T and v5TE in Thumb
//! state): no CLZ and no 32×32→64 multiply, so a count of leading zeros and a wide product are library calls.
//! Stable Rust has no `target_feature` for that, so the target's name decides.

fn main() {
    println!("cargo::rustc-check-cfg=cfg(thumb1)");
    println!("cargo::rerun-if-env-changed=TARGET");
    let target = std::env::var("TARGET").unwrap_or_default();
    if ["thumbv6m", "thumbv8m.base", "thumbv4t", "thumbv5te"]
        .iter()
        .any(|t| target.starts_with(t))
    {
        println!("cargo::rustc-cfg=thumb1");
    }
}
