//! Unit names (IO-9): every unit's names are consistent, and a unit is written in each [Notation].
#![cfg(all(feature = "std", feature = "quantities"))]

use exact_units::notation::{symbol_in, unit_in};
use exact_units::units::{self, Meter, Ohm, Per, Second, Sqrt, Times};
use exact_units::{Notation, Prefix, PrefixPolicy, Unit, UnitExpr};

const ALL: [Notation; 4] = [
    Notation::Unicode,
    Notation::Ascii,
    Notation::Ucum,
    Notation::Typst,
];

fn every(prefix: Prefix, e: &UnitExpr) -> [Option<String>; 4] {
    ALL.map(|n| unit_in(n, prefix, &e).map(|s| s.to_string()))
}

fn some(s: [&str; 4]) -> [Option<String>; 4] {
    s.map(|s| Some(s.to_owned()))
}

#[test]
fn notations() {
    assert_eq!(
        every(Prefix::Kilo, Ohm::EXPR),
        some([" kΩ", " kOhm", " kOhm", " upright(\"kΩ\")"])
    );
    assert_eq!(
        every(Prefix::Micro, Second::EXPR),
        some([" µs", " us", " us", " upright(\"µs\")"])
    );
    type Accel = Per<Meter, Times<Second, Second>>;
    assert_eq!(
        every(Prefix::Unit, Accel::EXPR),
        some([
            " m/(s·s)",
            " m/(s*s)",
            " m/(s.s)",
            " upright(m)/(upright(s) dot.op upright(s))"
        ])
    );
    assert_eq!(
        every(Prefix::Unit, units::Acceleration::EXPR),
        some([" m/s²", " m/s^2", " m/s2", " upright(m)/upright(s)^2"])
    );
    // a root has no form in UCUM
    type Noise = Per<units::Volt, Sqrt<units::Hertz>>;
    assert_eq!(
        every(Prefix::Nano, Noise::EXPR),
        [
            Some(" nV/√Hz".into()),
            Some(" n(V/sqrt(Hz))".into()),
            None,
            Some(" upright(\"n\")(upright(V)/sqrt(upright(\"Hz\")))".into())
        ]
    );
    // a unit without a UCUM code isn't written in UCUM
    assert!(unit_in(Notation::Ucum, Prefix::Unit, &units::Turn::EXPR).is_none());
    assert_eq!(
        symbol_in(&units::Turn::EXPR, Notation::Ascii)
            .unwrap()
            .to_string(),
        "rev"
    );
    assert_eq!(
        symbol_in(&units::OpticalDepth::EXPR, Notation::Ascii)
            .unwrap()
            .to_string(),
        "tau"
    );
    assert_eq!(
        every(Prefix::Centi, units::Ratio::EXPR),
        some([" %", " %", " %", " \"%\""])
    );
    assert_eq!(
        every(Prefix::Kibi, units::Byte::EXPR),
        some([" KiB", " KiB", " KiBy", " upright(\"KiB\")"])
    );
}

/// Every named unit of the crate: a symbol in ASCII, a plural with its long name, a UCUM code with no space, and
/// prefixes only on a one-word coherent symbol or where the line says so.
#[test]
fn every_unit_has_consistent_names() {
    let all = units::ALL.iter();
    #[cfg(feature = "customary")]
    let all = all.chain(units::CUSTOMARY.iter());
    for e in all {
        let UnitExpr::Named(def) = e else {
            panic!("{e:?} in the catalogue")
        };
        let names = def.names;
        let mut ascii = String::new();
        names.write_ascii(&mut ascii).unwrap().unwrap();
        assert!(ascii.is_ascii(), "{}: {ascii}", def.id.name);
        assert_eq!(
            names.long.is_empty(),
            names.plural.is_empty(),
            "{}",
            def.id.name
        );
        if let Some(code) = names.ucum {
            assert!(
                code.is_ascii() && !code.contains(' '),
                "{}: {code}",
                def.id.name
            );
        }
        let one_word = !names.symbol.contains(['/', '·', ' ', '(', '²', '³', '⁻']);
        if !one_word {
            assert!(
                matches!(names.prefixes, PrefixPolicy::None) || def.id.name == "Nit",
                "{} takes prefixes on a compound symbol",
                def.id.name
            );
        }
    }
}

mod parse {
    use exact_units::registry::ParseError;
    use exact_units::units::{
        self, Acceleration, Bel, Byte, DynamicViscosity, Entropy, HeatCapacity, Minute, Ohm,
        Second, Speed,
    };
    use exact_units::{Exact, Fixed, Prefix, Registry, Unit, UnitExpr, prefix::One};

    exact_units::unit! {
        @table
        static MINE;
        /// A user crate's unit: shaft speed in revolutions per minute.
        pub struct Rpm = "rpm" (long "revolution per minute" plural "revolutions per minute", prefixes only [Kilo]),
            [rad 1, s -1], Exact(1 / 30, pi 1);
    }

    #[cfg(feature = "customary")]
    static TABLES: &[&[&UnitExpr]] = &[units::ALL, units::CUSTOMARY, MINE];
    #[cfg(not(feature = "customary"))]
    static TABLES: &[&[&UnitExpr]] = &[units::ALL, MINE];
    const R: Registry<'static> = Registry::new(TABLES);

    #[test]
    fn done_criteria() {
        for text in [
            "4.7 kOhm",
            "4.7 kΩ",
            "4.7 kilohm",
            "4.7 Kilohms",
            "4700 Ω",
            "0.0047 MΩ",
        ] {
            let x = R.parse::<Ohm>(text).unwrap();
            assert_eq!(x.to::<One, u16>(), Some(Fixed::new(4700)), "{text}");
        }
        assert_eq!(
            R.parse::<Ohm>("47 kΩ").unwrap().to(),
            Some(Fixed::<Ohm, exact_units::prefix::Kilo, u16>::new(47))
        );
        assert_eq!(R.parse::<Second>("5 min").unwrap(), Exact::int(300));
        assert_eq!(R.parse::<Minute>("5 min").unwrap(), Exact::int(5));
        let min = R.parse_unit("min", None).unwrap();
        assert_eq!(min.factors().count(), 1);
        assert_eq!(
            min.factors().next().unwrap().reading.def,
            Minute::EXPR.def().unwrap()
        );
        // a viscosity, its own named unit and a pascal times a second
        assert_eq!(
            R.parse::<DynamicViscosity>("1 Pa·s").unwrap(),
            Exact::int(1)
        );
        assert_eq!(
            R.parse::<DynamicViscosity>("1 Pa s").unwrap(),
            Exact::int(1)
        );
        assert_eq!(
            R.parse::<DynamicViscosity>("1 mPa·s").unwrap(),
            Exact::new(1, 1000)
        );
        let pas = R.parse_unit("Pa·s", None).unwrap();
        assert_eq!(pas.dim(), Some(DynamicViscosity::DIM));
        // a byte where a byte is expected, ambiguous without
        assert_eq!(R.parse::<Byte>("5 B").unwrap(), Exact::int(5));
        assert_eq!(R.parse::<Bel>("5 B").unwrap(), Exact::int(5));
        assert_eq!(R.parse::<Byte>("2 KiB").unwrap(), Exact::int(2048));
        let err = R.parse_unit("B", None).unwrap_err();
        assert!(
            matches!(err, ParseError::Ambiguous { at: 0, len: 1, .. }),
            "{err:?}"
        );
        assert_eq!(
            err.to_string(),
            "`B` reads as a bel and as a byte: name the quantity it goes into"
        );
        assert!(R.parse_unit("B", Some(Byte::DIM)).is_ok());
        // a user crate's unit, through the registry that lists its table
        assert_eq!(R.parse::<Rpm>("1.5 krpm").unwrap(), Exact::int(1500));
        assert_eq!(
            R.parse::<Rpm>("3 revolutions per minute").unwrap(),
            Exact::int(3)
        );
        assert!(matches!(
            R.parse::<Rpm>("1 Mrpm"),
            Err(ParseError::Unknown { .. })
        ));
        // rpm to rad/s is a factor with π: not exact
        assert_eq!(
            R.parse::<units::AngularVelocity>("60 rpm"),
            Err(ParseError::Inexact)
        );
    }

    #[test]
    fn compounds() {
        for text in [
            "36 km/h",
            "36 km·h⁻¹",
            "36 km*h^-1",
            "36 km.h-1",
            "36 km/hour",
            "36 kilometres per hour",
            "10 m/s",
        ] {
            let parsed = R.parse::<Speed>(text);
            assert_eq!(parsed, Ok(Exact::int(10)), "{text}");
        }
        for text in [
            "9.81 m/s²",
            "9.81 m/s^2",
            "9.81 m/s2",
            "9.81 m·s⁻²",
            "9.81 m/(s·s)",
            "9.81 (m/s)/s",
        ] {
            assert_eq!(
                R.parse::<Acceleration>(text),
                Ok(Exact::new(981, 100)),
                "{text}"
            );
        }
        assert_eq!(R.parse::<units::Area>("1 km²"), Ok(Exact::int(1_000_000)));
        assert_eq!(R.parse::<units::Area>("1 cm2"), Ok(Exact::new(1, 10_000)));
        assert_eq!(
            R.parse::<units::Volume>("1 mL"),
            Ok(Exact::new(1, 1_000_000))
        );
        assert_eq!(R.parse::<units::Density>("1 g/cm³"), Ok(Exact::int(1000)));
        let u = R.parse_unit("kg·m²/(s³·A)", None).unwrap();
        assert_eq!(u.dim(), Some(units::Volt::DIM));
        assert_eq!(u.to_string(), "kg·m²/(s³·A)");
        // quantities: a named unit of a quantity of its own only into that one, a unit that names only its
        // dimension (J/K, V/A) into any quantity of it, the type's
        assert_eq!(R.parse::<Entropy>("2 J/K"), Ok(Exact::int(2)));
        assert_eq!(R.parse::<HeatCapacity>("2 J/K"), Ok(Exact::int(2)));
        assert_eq!(R.parse::<Ohm>("2 V/A"), Ok(Exact::int(2)));
        assert_eq!(
            R.parse::<units::Becquerel>("2 Hz"),
            Err(ParseError::Mismatch)
        );
        assert_eq!(R.parse::<units::Hertz>("2 Bq"), Err(ParseError::Mismatch));
        assert_eq!(R.parse::<units::Sievert>("2 Gy"), Err(ParseError::Mismatch));
        assert_eq!(R.parse::<Speed>("2 s"), Err(ParseError::Mismatch));
    }

    #[test]
    fn prefixes_follow_the_policy() {
        // no milli-inch, no centi-day, no kilo-minute
        assert!(R.parse_unit("kmin", None).is_err());
        assert!(R.parse_unit("cmin", None).is_err());
        let cd = R
            .parse_unit("mcd", None)
            .unwrap()
            .factors()
            .next()
            .unwrap()
            .reading;
        assert_eq!(
            (cd.def, cd.prefix),
            (units::Candela::EXPR.def().unwrap(), Prefix::Milli)
        );
        assert_eq!(R.parse::<Second>("5 us"), Ok(Exact::new(5, 1_000_000)));
        assert_eq!(R.parse::<Second>("5 µs"), Ok(Exact::new(5, 1_000_000)));
        assert_eq!(R.parse::<Second>("5 μs"), Ok(Exact::new(5, 1_000_000)));
        assert_eq!(R.parse::<units::Nit>("2 knit"), Ok(Exact::int(2000)));
        // km is kilo metre and the kilometre, the same value: no ambiguity
        assert!(R.parse_unit("km", None).is_ok());
    }

    #[test]
    fn errors() {
        assert_eq!(R.parse_unit("", None), Err(ParseError::Empty));
        assert_eq!(R.parse::<Second>("s"), Err(ParseError::Number));
        assert_eq!(
            R.parse::<Second>("5 xyz"),
            Err(ParseError::Unknown { at: 2, len: 3 })
        );
        assert_eq!(
            R.parse_unit("m/xyz", None),
            Err(ParseError::Unknown { at: 2, len: 3 })
        );
        assert!(matches!(
            R.parse_unit("m//s", None),
            Err(ParseError::Syntax { .. })
        ));
        assert!(matches!(
            R.parse_unit("(m/s", None),
            Err(ParseError::Syntax { .. })
        ));
        assert!(matches!(
            R.parse_unit("m^", None),
            Err(ParseError::Syntax { .. })
        ));
        assert_eq!(
            R.parse_unit("((((((m))))))", None),
            Err(ParseError::TooComplex)
        );
        assert_eq!(
            R.parse_unit("m·m·m·m·m·m·m·m·m", None),
            Err(ParseError::TooComplex)
        );
        assert_eq!(
            R.parse_unit("m^200", None),
            Err(ParseError::Syntax { at: 2 })
        );
        // a dimension past what a Dim holds
        assert_eq!(
            R.parse_unit("m^100", None).map(|u| u.dim().is_none()),
            Ok(true)
        );
        // an exponent 0 names no unit (found by fuzzing: `h0` was a unit written `1`)
        assert_eq!(R.parse_unit("h0", None), Err(ParseError::Syntax { at: 0 }));
        assert_eq!(R.parse_unit("m^0", None), Err(ParseError::Syntax { at: 0 }));
        assert!(matches!(
            R.parse_unit("(m/s)^0", None),
            Err(ParseError::Syntax { .. })
        ));
        // i8's own -128 reads in each form (found by fuzzing: `d⁻¹²⁸` overflowed)
        for text in ["d-128", "d^-128", "d⁻¹²⁸"] {
            let u = R.parse_unit(text, None).unwrap();
            assert_eq!(u.factors().next().unwrap().exp, -128, "{text}");
        }
        // a reciprocal writes `m⁻¹` and reads `1/m` (found by proptest: `1/m` didn't read)
        assert_eq!(R.parse_unit("/m", None).unwrap().to_string(), "m⁻¹");
        assert!(R.parse_unit("1/m", None).is_ok());
        // a compound symbol with an exponent in parentheses (found by proptest: `nit^2` wrote `cd/m²²`)
        assert_eq!(R.parse_unit("nit^2", None).unwrap().to_string(), "(cd/m²)²");
    }
}
