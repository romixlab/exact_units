# exact_units_macros

The proc-macro behind [exact_units](https://crates.io/crates/exact_units)' `q!`, the literal that names its unit: `speed < q!(100 m/s)` is one integer compare against a constant, and `speed < q!(100 m)` doesn't compile.

Use it through exact_units (`exact_units::q`), never on its own: exact_units passes its own path in. The crate has no dependencies.
