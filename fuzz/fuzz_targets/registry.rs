//! Any text into a `Registry`, as a unit and as a quantity of a few units: a refusal, or a unit or value whose text
//! reads back the same.
#![no_main]

#[path = "props.rs"]
mod props;

libfuzzer_sys::fuzz_target!(|s: &str| {
    props::registry::any_unit_text(s);
    props::registry::any_quantity_text(s);
});
