//! Any bytes decoded as one of the crate's wire types (the first byte picks which): an error or a value that
//! round-trips, a decoded unit tree walked to its end. Never a panic.
#![no_main]

#[path = "props.rs"]
mod props;

libfuzzer_sys::fuzz_target!(|data: &[u8]| {
    if let Some((&which, bytes)) = data.split_first() {
        props::wire::any_bytes(which, bytes);
    }
});
