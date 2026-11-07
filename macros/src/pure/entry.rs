//! The shape of a line of the symbol table ([table](super::table)).

/// Which prefixes a unit's symbol takes.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(crate) enum Prefixes {
    /// None: `h`, `min`, `°`, every customary unit.
    None,
    /// The SI prefixes: the coherent SI units and those the SI lets take one (g, L, eV).
    Si,
    /// The SI prefixes from kilo up: the tonne (`kt`, `Mt`), never a millitonne.
    SiFromKilo,
    /// The SI and the binary prefixes: bit and B (`kB`, `KiB`).
    SiBinary,
}

/// A symbol `q!` reads as a unit marker.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(crate) struct Entry {
    /// The symbol as written: `s`, `Hz`, `Ω`, `deg`, `%`.
    pub(crate) symbol: &'static str,
    /// The marker's path under exact_units' root: `units::Second`, `units::customary::Inch`.
    pub(crate) marker: &'static str,
    /// The cargo feature the marker needs (`time`), which names the helper macro `__q_time!` it is written through.
    pub(crate) group: &'static str,
    /// The prefixes the symbol takes.
    pub(crate) prefixes: Prefixes,
    /// A power of ten the symbol carries itself: `%` is `-2` of `Ratio`, `ppm` `-6`.
    pub(crate) pow10: i8,
}

/// A symbol `q!` refuses, as it reads two ways (`min`: minute, US or imperial minim, milli-inch).
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(crate) struct Refused {
    /// The symbol.
    pub(crate) symbol: &'static str,
    /// Its readings, each what to write instead: a marker (`Minute`) or a prefixed one (`milli Inch`).
    pub(crate) readings: &'static [&'static str],
}

/// A symbol `q!` reads as a point at an origin: `°C` is a temperature from the `Celsius` origin.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(crate) struct Origin {
    /// The symbol: `°C`, `degC`.
    pub(crate) symbol: &'static str,
    /// The origin's path under exact_units' root: `origins::Celsius`.
    pub(crate) path: &'static str,
    /// The cargo feature the origin needs.
    pub(crate) group: &'static str,
}

/// A symbol `q!` reads as a level of a gain when it stands alone after the number: `q!(-3 dB)`. The gain's kind picks
/// the form (20·lg of V/V, 10·lg of W/W), so it reads one way there; inside a unit (`dB/km`) it stays refused.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(crate) struct Level {
    /// The symbol: `dB`, `Np`.
    pub(crate) symbol: &'static str,
    /// The level marker's path under exact_units' root: `literal::Db`.
    pub(crate) marker: &'static str,
}
