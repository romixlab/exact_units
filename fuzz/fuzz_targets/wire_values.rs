//! Values built from the fuzzer's data (`arbitrary`): numbers, a series of rows, magnitudes from their parts,
//! prefixes, dimensions canonical or not and unit trees, each encoded within its bound and decoded to itself.
#![no_main]

#[path = "props.rs"]
mod props;

use props::wire::{self, Expr};

#[derive(Debug, arbitrary::Arbitrary)]
enum Input {
    Numbers(i16, u32),
    Series(Vec<(u32, i16, u8)>),
    Magnitude(i8, i8, [i8; 3], Vec<(u32, i8)>),
    Prefix(u8, i8, u16),
    Dim([i8; 9], bool, [Option<(String, i8)>; 4]),
    Unit(Expr),
}

libfuzzer_sys::fuzz_target!(|input: Input| match input {
    Input::Numbers(x, y) => wire::numbers(x, y),
    Input::Series(rows) => wire::series(&rows),
    Input::Magnitude(pow10, pow2, irr, primes) => wire::magnitude(pow10, pow2, irr, &primes),
    Input::Prefix(which, param, tag) => wire::prefix(which, param, tag),
    Input::Dim(exps, root, counted) => wire::dim(exps, root, &counted),
    Input::Unit(e) => wire::si_expr(&e),
});
