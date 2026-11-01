//! Any text read with serde as `Magnitude`, `Offset`, `DimOwned` and `ProvenanceOwned`: an error, or a value that
//! round-trips through JSON and, where canonical, through the wire.
#![no_main]

#[path = "props.rs"]
mod props;

libfuzzer_sys::fuzz_target!(|s: &str| props::serde_input::any_json(s));
