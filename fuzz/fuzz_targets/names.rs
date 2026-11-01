//! Any text into `Prefix::from_name` and `units::by_name`: exactly the names they list.
#![no_main]

#[path = "props.rs"]
mod props;

libfuzzer_sys::fuzz_target!(|s: &str| {
    props::text::prefix_name(s);
    props::text::unit_name(s);
});
