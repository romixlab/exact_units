//! Any text into `Decimal::parse` (what `fixed!` runs): the reference's value, or a documented refusal.
#![no_main]

#[path = "props.rs"]
mod props;

libfuzzer_sys::fuzz_target!(|s: &str| props::text::decimal(s));
