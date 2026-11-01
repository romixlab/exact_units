//! Any text into `Decibels::parse` and a parsed level into `from_db_const` (what `db!` runs): the reference's level
//! and the nearest gain, or a documented refusal.
#![no_main]

#[path = "props.rs"]
mod props;

libfuzzer_sys::fuzz_target!(|s: &str| props::text::decibels(s));
