//! Any text into `Uncertain`'s readers (the concise form, the columns, `±` with k = 1): a refusal, or a value whose
//! text reads back the same.
#![no_main]

#[path = "props.rs"]
mod props;

libfuzzer_sys::fuzz_target!(|s: &str| {
    props::uncertain_text::any_text(s);
});
