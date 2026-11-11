---
level: advanced
---

# Unit names and text

Every named unit carries a record of how it is written and read, its `UnitNames`. A `Registry` reads units and quantities from text through these names. `Notation` writes a unit in ASCII, in UCUM or in Typst math as well as in its Unicode symbol.

## The names of a unit

A `unit!` line gives the names in parentheses after the symbol. Each item is optional and they come in any order:

```rust
exact_units::unit! {
    /// Shaft speed in revolutions per minute.
    pub struct Rpm = "rpm" (long "revolution per minute" plural "revolutions per minute", prefixes only [Kilo]),
        [rad 1, s -1], Exact(1 / 30, pi 1);
}

use exact_units::{Prefix, Unit};

let names = Rpm::EXPR.def().unwrap().names;
assert_eq!(names.long, "revolution per minute");
assert!(names.prefixes.allows(Prefix::Kilo));
assert!(!names.prefixes.allows(Prefix::Mega));
```

The items are these:

- `ascii "Ohm"`: the symbol in ASCII. Most units need none, because the symbol is transliterated: `µ` becomes `u`, `Ω` becomes `Ohm`, `°` becomes `deg`, `·` becomes `*`, and `m/s²` becomes `m/s^2`. A symbol with a character outside these rules needs one, or the line doesn't compile.
- `long "ohm"`: the English name. The plural adds an s, and `long "foot" plural "feet"` names it where it doesn't.
- `aliases ["nit", "nt"]`: other spellings text may use.
- `ucum "Ohm"`: the unit's code in UCUM, the Unified Code for Units of Measure.
- `prefixes none`, `decimal`, `decimal_and_binary` or `only [Milli, Kilo]`: the prefixes the unit takes in text.

The prefixes have a default, so most lines say nothing about them. A coherent unit whose symbol is one word takes the SI prefixes (`m`, `V`, `Pa`). Every other unit takes none: a scaled unit (`min`, `ft`), a base of its own (`sample`) and a compound symbol (`m/s`, whose parts take their own). This is what keeps `min` a minute and never a milli-inch, and `cd` a candela and never a centi-day.

```rust,compile_fail
exact_units::unit! {
    /// τ has no ASCII form by the rules: the line must name one, `"τ" (ascii "tau")`.
    pub struct Tau = "τ", [];
}
```

## Reading text

A `Registry` lists the tables of units text may name. It holds no global state and needs no allocation, so it works on a microcontroller as well as on a host. A crate's own units join through its `unit! { @table static UNITS; .. }` list:

```rust
use exact_units::units::{self, Ohm, Second, Speed};
use exact_units::{Exact, Fixed, Registry, UnitExpr, prefix::One};

exact_units::unit! {
    @table
    static MINE;
    /// Shaft speed in revolutions per minute.
    pub struct Rpm = "rpm" (prefixes only [Kilo]), [rad 1, s -1], Exact(1 / 30, pi 1);
}

static TABLES: &[&[&UnitExpr]] = &[units::ALL, MINE];
const UNITS: Registry<'static> = Registry::new(TABLES);

// one value, written four ways
for text in ["4.7 kOhm", "4.7 kΩ", "4.7 kilohm", "4700 ohms"] {
    let r: Exact<Ohm> = UNITS.parse(text).unwrap();
    assert_eq!(r.to::<One, u16>(), Some(Fixed::new(4700)));
}
assert_eq!(UNITS.parse::<Second>("5 min").unwrap(), Exact::int(300));
assert_eq!(UNITS.parse::<Speed>("36 km/h").unwrap(), Exact::int(10));
assert_eq!(UNITS.parse::<Rpm>("1.5 krpm").unwrap(), Exact::int(1500));
```

`parse::<U>` gives an `Exact<U>`, the exact value. `Exact::to` then gives a `Fixed`, or `None` where the value isn't a whole number of steps: 4.7 [[kΩ]] is 4700 [[Ω]] but not a whole number of [[kΩ]].

The reader goes in this order:

1. The whole text is tried as one unit first, so `Pa·s` is the unit of viscosity before it is a pascal times a second.
2. Otherwise the text splits into factors. They are joined by `·`, `*`, `.` or a space and divided by `/` or `per`, from left to right, so `a/b·c` is `a·c/b`. Parentheses group factors.
3. An exponent is written `²`, `^2` or in UCUM's way, `s-1`.
4. Each word is read by any of the unit's names: the symbol, the ASCII form, an alias, the UCUM code or the English name in any case. A prefix goes before it where the unit takes one: `km`, `kOhm`, `kilometre`. The SI's short forms `kilohm` and `megohm` read too.

## When a word reads two ways

A word that reads two ways is refused, and the error names both readings. `B` is the bel and the byte:

```rust
use exact_units::units::{self, Byte};
use exact_units::Unit;
use exact_units::{Exact, Registry};

let tables = [units::ALL];
let r = Registry::new(&tables);
let err = r.parse_unit("B", None).unwrap_err();
assert_eq!(err.to_string(), "`B` reads as a bel and as a byte: name the quantity it goes into");

// the quantity the text goes into settles it
assert_eq!(r.parse::<Byte>("5 B").unwrap(), Exact::int(5));
assert!(r.parse_unit("B", Some(Byte::DIM)).is_ok());
```

Readings of the same value are one reading: `km` is a kilometre and kilo metre, and they are equal. A named unit goes only into a quantity of its own kind, so `V/A` is no `Ohm`. A unit whose factor isn't exact in the target is refused too: rpm to [[rad/s]] needs π.

## Writing in a notation

`notation::unit_in` writes a unit after a number with its prefix, and `notation::symbol_in` writes the symbol alone. Each gives `None` where the notation has no form for the unit. A unit without a UCUM code has no UCUM form, and neither does a root.

```rust
use exact_units::notation::{symbol_in, unit_in};
use exact_units::units::{Meter, Ohm, Per, Second, Turn};
use exact_units::{Notation, Prefix, Unit};

let speed = Per::<Meter, Second>::EXPR;
assert_eq!(symbol_in(&speed, Notation::Unicode).unwrap().to_string(), "m/s");
assert_eq!(symbol_in(&speed, Notation::Ascii).unwrap().to_string(), "m/s");
assert_eq!(symbol_in(&speed, Notation::Ucum).unwrap().to_string(), "m/s");
assert_eq!(symbol_in(&speed, Notation::Typst).unwrap().to_string(), "upright(m)/upright(s)");

assert_eq!(unit_in(Notation::Ascii, Prefix::Kilo, &Ohm::EXPR).unwrap().to_string(), " kOhm");
// the revolution has no UCUM code
assert!(symbol_in(&Turn::EXPR, Notation::Ucum).is_none());
```

What each notation writes, and the registry reads back at the same exact scale, is checked for every unit of the crate with every prefix it takes. The parser takes text from outside, so a property test and a fuzz target check that any text is either refused or read as a unit whose own text reads back the same.
