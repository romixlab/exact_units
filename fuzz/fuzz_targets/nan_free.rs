//! Bytes read as a float `Fixed` of every NaN-free bound (`Unbounded`, `Finite`, `RangeDec`) and of `MaybeNan`:
//! through the wire, JSON and `with_unit` text. A NaN-free type never holds NaN, a `MaybeNan` keeps every bit pattern
//! of the wire.
#![no_main]

#[path = "props.rs"]
mod props;

libfuzzer_sys::fuzz_target!(|data: &[u8]| props::nan::input(data));
