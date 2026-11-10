"""The long name and quantity of each unit, origin and level marker, for the tooltip of `[[symbol]]` and `\\unit{}`.

q!'s generated table (docs/start/symbols.md, from macros/src/pure/table.rs) has the symbol and the marker, but not
the marker's long name or quantity (`src/quantities.rs` has those only as rustdoc prose), so they are curated here
by hand, keyed by the bare marker name (`Becquerel`, not `units::Becquerel`). A marker used in a converted page
that has no entry here fails the build with its own name, so adding a page means adding its markers here too.

Prefix names and exponents (`unit_markup.py`'s prefix-splitting, `keV` = kilo `ElectronVolt`): the SI prefixes and
the four binary ones `src/si_prefix.rs` defines (no prefix past Tebi exists in the crate at all). `kind` is "si" or
"binary"; `exp` is the power of ten an SI prefix carries, used to tell "SI from kilo up" (the tonne) apart from the
rest.
"""

from __future__ import annotations

from dataclasses import dataclass


@dataclass(frozen=True)
class PrefixEntry:
    name: str
    kind: str  # "si" or "binary"
    exp: int | None = None  # power of ten, SI prefixes only


PREFIX_INFO: dict[str, PrefixEntry] = {
    "Q": PrefixEntry("quetta", "si", 30),
    "R": PrefixEntry("ronna", "si", 27),
    "Y": PrefixEntry("yotta", "si", 24),
    "Z": PrefixEntry("zetta", "si", 21),
    "E": PrefixEntry("exa", "si", 18),
    "P": PrefixEntry("peta", "si", 15),
    "T": PrefixEntry("tera", "si", 12),
    "G": PrefixEntry("giga", "si", 9),
    "M": PrefixEntry("mega", "si", 6),
    "k": PrefixEntry("kilo", "si", 3),
    "h": PrefixEntry("hecto", "si", 2),
    "da": PrefixEntry("deca", "si", 1),
    "d": PrefixEntry("deci", "si", -1),
    "c": PrefixEntry("centi", "si", -2),
    "m": PrefixEntry("milli", "si", -3),
    "µ": PrefixEntry("micro", "si", -6),
    "n": PrefixEntry("nano", "si", -9),
    "p": PrefixEntry("pico", "si", -12),
    "f": PrefixEntry("femto", "si", -15),
    "a": PrefixEntry("atto", "si", -18),
    "z": PrefixEntry("zepto", "si", -21),
    "y": PrefixEntry("yocto", "si", -24),
    "r": PrefixEntry("ronto", "si", -27),
    "q": PrefixEntry("quecto", "si", -30),
    "Ki": PrefixEntry("kibi", "binary"),
    "Mi": PrefixEntry("mebi", "binary"),
    "Gi": PrefixEntry("gibi", "binary"),
    "Ti": PrefixEntry("tebi", "binary"),
}


@dataclass(frozen=True)
class Info:
    name: str
    quantity: str


# Keyed by the bare marker (docs/start/symbols.md's Marker column).
UNIT_INFO: dict[str, Info] = {
    "Ampere": Info("ampere", "electric current"),
    "Barn": Info("barn", "area"),
    "Becquerel": Info("becquerel", "activity"),
    "Byte": Info("byte", "information"),
    "Bit": Info("bit", "information"),
    "Candela": Info("candela", "luminous intensity"),
    "Coulomb": Info("coulomb", "electric charge"),
    "Curie": Info("curie", "activity"),
    "Dalton": Info("dalton", "mass"),
    "Day": Info("day", "time"),
    "Degree": Info("degree", "angle"),
    "ElectronVolt": Info("electronvolt", "energy"),
    "Farad": Info("farad", "capacitance"),
    "Gram": Info("gram", "mass"),
    "Gray": Info("gray", "absorbed dose"),
    "Henry": Info("henry", "inductance"),
    "Hertz": Info("hertz", "frequency"),
    "Hour": Info("hour", "time"),
    "Joule": Info("joule", "energy"),
    "Kelvin": Info("kelvin", "temperature"),
    "Litre": Info("litre", "volume"),
    "Lumen": Info("lumen", "luminous flux"),
    "Lux": Info("lux", "illuminance"),
    "Meter": Info("metre", "length"),
    "Mole": Info("mole", "amount of substance"),
    "Newton": Info("newton", "force"),
    "Ohm": Info("ohm", "electrical resistance"),
    "Pascal": Info("pascal", "pressure"),
    "Radian": Info("radian", "angle"),
    "Ratio": Info("ratio", "dimensionless"),
    "Second": Info("second", "time"),
    "Siemens": Info("siemens", "electrical conductance"),
    "Sievert": Info("sievert", "dose equivalent"),
    "Steradian": Info("steradian", "solid angle"),
    "Tesla": Info("tesla", "magnetic flux density"),
    "Volt": Info("volt", "electric potential"),
    "VoltAmpere": Info("voltampere", "apparent power"),
    "Watt": Info("watt", "power"),
    "Weber": Info("weber", "magnetic flux"),
    "Week": Info("week", "time"),
}

# Keyed by the Origin column's path (docs/start/symbols.md strips `origins::`).
ORIGIN_INFO: dict[str, Info] = {
    "Celsius": Info("celsius", "temperature"),
    "Fahrenheit": Info("fahrenheit", "temperature"),
    "Rankine": Info("rankine", "temperature"),
}

# Keyed by the Level column's marker (docs/start/symbols.md strips `literal::`).
LEVEL_INFO: dict[str, Info] = {
    "Db": Info("decibel", "gain level"),
    "Np": Info("neper", "gain level"),
}
