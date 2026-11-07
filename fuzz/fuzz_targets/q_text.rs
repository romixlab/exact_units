//! Any text into `q!`'s reader, as a string unit or literal: a literal or an error with a message, never a panic.
#![no_main]

#[path = "props.rs"]
mod props;

libfuzzer_sys::fuzz_target!(|s: &str| props::q_text::reads_or_refuses(s));
