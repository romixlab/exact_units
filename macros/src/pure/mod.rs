//! The logic of `q!` in plain Rust, without `proc_macro`: a token model, the number, the prefix and symbol table,
//! the unit's grammar. `lib.rs` converts the compiler's tokens into this model and the result back; the tests of
//! this crate, exact_units' `tests/props` and its fuzz targets include these files by `#[path]` and call them
//! directly.

pub(crate) mod entry;
pub(crate) mod grammar;
pub(crate) mod model;
pub(crate) mod number;
pub(crate) mod prefix;
pub(crate) mod resolve;
pub(crate) mod strlex;
pub(crate) mod table;

#[cfg(test)]
mod tests;
