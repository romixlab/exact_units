//! Any text into `Fraction` and `Exact` (`FromStr`), and as JSON into `with_unit` fields of an `Exact`: a refusal, or
//! a value whose text reads back the same.
#![no_main]

#[path = "props.rs"]
mod props;

libfuzzer_sys::fuzz_target!(|s: &str| {
    props::exact_text::any_text(s);
    props::exact_text::any_json(s);
});
