//! A unit definition and a unit tree from outside (the first byte says how many bytes the definition takes): the
//! tree walks, compacts and resolves against a table that lists the definition, and what compacts is the same unit.
//! Never a panic.
#![no_main]

#[path = "props.rs"]
mod props;

libfuzzer_sys::fuzz_target!(|data: &[u8]| {
    if let Some((&len, rest)) = data.split_first()
        && let Some((def, expr)) = rest.split_at_checked(usize::from(len))
    {
        props::wire::tree_against_table(def, expr);
    }
});
