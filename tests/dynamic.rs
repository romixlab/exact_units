//! Units known only at run time (STD-37): `Dim` and `UnitDef` on the wire, `SIExpr`; golden bytes and round trips,
//! and that each is shown like the typed unit it maps onto.
#![cfg(all(feature = "std", feature = "quantities"))]
#![expect(clippy::panic, reason = "a test fails by panicking")]
#![expect(
    clippy::arithmetic_side_effects,
    reason = "a test counts tree levels far below usize::MAX"
)]

use core::fmt;
use exact_units::units::{self, AngularVelocity, Per, Times, Torque};
use exact_units::{
    Dim, MAX_UNIT_DEPTH, Magnitude, MarkerId, Prefix, Provenance, SIExpr, SIExprOwned, Unit,
    UnitDef, UnitDefOwned, UnitExpr, UnitNames, tree,
};
use shrink_wrap::prelude::*;

type Viscous = Per<Torque, AngularVelocity>;

#[test]
fn dim_bytes() {
    let mut buf = [0u8; 8];
    let volt = <units::Volt as Unit>::DIM;
    // s -3, m 2, kg 1, A -1, K 0, mol 0, cd 0, rad 0, bit 0: nine nibbles, the root bit, then four bits of empty
    // counted slots
    assert_eq!(
        volt.to_ww_bytes(&mut buf).unwrap(),
        [0xd2, 0x1f, 0x00, 0x00, 0x00, 0x00]
    );
    assert_eq!(
        Dim::from_ww_bytes(&[0xd2, 0x1f, 0x00, 0x00, 0x00, 0x00]).unwrap(),
        volt
    );
    assert_eq!(
        <Dim<'_> as SerializeShrinkWrap>::ELEMENT_SIZE,
        ElementSize::UnsizedFinalStructure
    );
    let edge = Dim::new(-8, 7, 0, 0, 0, 0, -1).with_rad(7).with_bit(-8);
    let bytes = edge.to_ww_bytes(&mut buf).unwrap().to_vec();
    assert_eq!(bytes, [0x87, 0x00, 0x00, 0xf7, 0x80, 0x00]);
    assert_eq!(Dim::from_ww_bytes(&bytes).unwrap(), edge);
    // samples per second: one counted slot, its name and exponent
    let rate = <Per<units::Sample, units::Second> as Unit>::DIM;
    let mut buf = [0u8; 32];
    let bytes = rate.to_ww_bytes(&mut buf).unwrap().to_vec();
    // s -1 and eight 0 nibbles, no root, slot 1 used: crate "exact_units", "Sample" and exponent 1, three empty slots
    assert_eq!(
        bytes,
        [
            0xf0, 0x00, 0x00, 0x00, 0x04, 0x65, 0x78, 0x61, 0x63, 0x74, 0x5f, 0x75, 0x6e, 0x69,
            0x74, 0x73, 0x53, 0x61, 0x6d, 0x70, 0x6c, 0x65, 0x10, 0x06, 0x39
        ]
    );
    assert_eq!(Dim::from_ww_bytes(&bytes).unwrap(), rate);
    assert_eq!(rate.to_string(), "s⁻¹·Sample");
}

/// Counted bases: sorted by name, added up, gone at exponent 0, at most `MAX_COUNTED`; a dimension that isn't in
/// that form doesn't decode.
#[test]
fn counted_bases() {
    let sample = Dim::counted("exact_units", "Sample");
    let block = Dim::counted("exact_units", "Block");
    let d = sample.mul(block).div(Dim::counted("exact_units", "Frame"));
    assert_eq!(
        d.counted_bases().collect::<Vec<_>>(),
        [("Block", 1), ("Frame", -1), ("Sample", 1)]
    );
    assert_eq!(
        d,
        block.div(Dim::counted("exact_units", "Frame")).mul(sample)
    );
    assert!(
        d.div(sample)
            .div(block)
            .mul(Dim::counted("exact_units", "Frame"))
            .is_one()
    );
    let four = d.mul(Dim::counted("exact_units", "Item"));
    assert_eq!(
        four.checked_mul(Dim::counted("exact_units", "Packet")),
        None
    );
    assert_eq!(
        four.checked_mul(Dim::counted("exact_units", "Item"))
            .unwrap()
            .to_string(),
        "Block·Frame⁻¹·Item²·Sample"
    );
    assert_ne!(sample, block);
    assert_ne!(sample, Dim::ONE);

    // a base is its crate and marker name: another crate's Sample is another base, sorted by crate first
    let theirs = Dim::counted("my_api", "Sample");
    assert_ne!(theirs, sample);
    let both = sample.mul(theirs);
    assert_eq!(both.counted[0].unwrap().crate_name, "exact_units");
    assert_eq!(both.counted[1].unwrap().crate_name, "my_api");
    assert!(both.div(theirs).same(sample));
    assert_eq!(
        exact_units::counted!(Bucket),
        Dim::counted("dynamic", "Bucket")
    );

    let mut buf = [0u8; 256];
    let bytes = d.to_ww_bytes(&mut buf).unwrap().to_vec();
    assert_eq!(Dim::from_ww_bytes(&bytes).unwrap(), d);
    // out of order: refused on decode
    let mut unsorted = d;
    unsorted.counted.swap(0, 2);
    assert!(!unsorted.is_canonical());
    let bytes = unsorted.to_ww_bytes(&mut buf).unwrap().to_vec();
    assert!(Dim::from_ww_bytes(&bytes).is_err());
}

#[test]
fn dim_range() {
    assert_eq!(Dim::checked_new([8, 0, 0, 0, 0, 0, 0, 0, 0]), None);
    assert_eq!(Dim::checked_new([0, 0, 0, 0, 0, 0, 0, 0, -9]), None);
    let m4 = Dim::new(0, 4, 0, 0, 0, 0, 0);
    assert_eq!(m4.checked_mul(m4), None);
    assert_eq!(m4.checked_pow(2), None);
    assert_eq!(m4.checked_pow(-1), Some(Dim::new(0, -4, 0, 0, 0, 0, 0)));
    assert_eq!(m4.checked_div(m4), Some(Dim::ONE));
}

/// A plain coherent unit of this test: `name` as its marker, `symbol` after a number.
const fn def(name: &'static str, symbol: &'static str, dim: Dim<'static>) -> UnitDef<'static> {
    UnitDef {
        id: MarkerId {
            crate_name: "dynamic",
            name,
        },
        names: UnitNames::new(symbol, exact_units::PrefixPolicy::Decimal),
        dim,
        quantity: None,
        scale: Magnitude::ONE,
        provenance: Provenance::Defined,
    }
}

/// [def] as an owned run-time unit.
fn owned(name: &'static str, symbol: &'static str, dim: Dim<'static>) -> SIExprOwned {
    SIExprOwned::Def(def(name, symbol, dim).into())
}

/// The scale of a named unit is a `Magnitude` on the wire, whatever spelling the `unit!` line used.
#[test]
fn scale_is_a_magnitude() {
    let mut buf = [0u8; 32];
    for (unit, m) in [
        (<units::Volt as Unit>::EXPR, Magnitude::ONE),
        (<units::Gram as Unit>::EXPR, Magnitude::pow10(-3)),
        (
            <units::Degree as Unit>::EXPR,
            Magnitude::new(1, 180, 0, 0, 1),
        ),
    ] {
        let d = unit.def().unwrap();
        assert_eq!(d.scale, m, "{unit}");
        let bytes = m.to_ww_bytes(&mut buf).unwrap().to_vec();
        assert_eq!(Magnitude::from_ww_bytes(&bytes).unwrap(), m);
    }
}

/// `Volt` as a run-time unit: the `Def` tag, then its `UnitDef`: the crate, the name, its names (the symbol, no ASCII
/// form of its own, the long names, no aliases, the UCUM code, the SI prefixes), the dimension, no kind of its own,
/// the scale one and the provenance `Defined`.
const VOLT_BYTES: [u8; 45] = [
    0x00, b'e', b'x', b'a', b'c', b't', b'_', b'u', b'n', b'i', b't', b's', b'V', b'o', b'l', b't',
    b'V', 0x00, b'v', b'o', b'l', b't', b'v', b'o', b'l', b't', b's', 0x80, b'V', 0x11, 0x05, b'A',
    0xd2, 0x1f, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x10, 0xa4, 0x39, 0x3d,
];

#[test]
fn named_bytes() {
    let mut buf = [0u8; 64];
    let volt = <units::Volt as Unit>::SI_EXPR;
    assert_eq!(volt.to_ww_bytes(&mut buf).unwrap(), VOLT_BYTES);
    let back = SIExpr::from_ww_bytes(&VOLT_BYTES).unwrap();
    assert_eq!(&back, volt);
    assert_eq!(back, *<units::Volt as Unit>::EXPR);
    assert_eq!(back.known(), Some(<units::Volt as Unit>::EXPR));
    assert_eq!(back.to_string(), "V");
    assert_eq!(back.name().to_string(), "Volt");
    assert_eq!(back.dim(&()), Some(<units::Volt as Unit>::DIM));
}

#[test]
fn compound_round_trip() {
    let mut buf = [0u8; 512];
    let si = <Viscous as Unit>::SI_EXPR;
    let bytes = si.to_ww_bytes(&mut buf).unwrap().to_vec();
    assert_eq!(bytes.len(), 167);
    assert_eq!(&bytes[..2], [0x30, 0x00]);
    let back = SIExpr::from_ww_bytes(&bytes).unwrap();
    assert_eq!(&back, si);
    assert_eq!(back, *<Viscous as Unit>::EXPR);
    assert_eq!(back.known(), None);
    // shown exactly as the typed unit
    assert_eq!(back.to_string(), <Viscous as Unit>::EXPR.to_string());
    assert_eq!(back.to_string(), "N·m/(rad/s)");
    assert_eq!(back.name().to_string(), "Per<Torque, AngularVelocity>");
    assert_eq!(back.dim(&()), Some(<Viscous as Unit>::DIM));
    assert_eq!(back.factor(&()), Some(1.0));

    type Fan = Per<Torque, Times<AngularVelocity, AngularVelocity>>;
    let fan = SIExpr::from_ww_bytes(<Fan as Unit>::SI_EXPR.to_ww_bytes(&mut buf).unwrap()).unwrap();
    assert_eq!(fan.to_string(), "N·m/((rad/s)·(rad/s))");
    assert_eq!(fan.to_string(), <Fan as Unit>::EXPR.to_string());
    assert_eq!(fan.dim(&()).unwrap().to_string(), "kg·m²·rad⁻³");

    // a different tree of the same dimension is not the same unit
    assert_ne!(back, *<Fan as Unit>::EXPR);
    assert_ne!(back, *<Torque as Unit>::EXPR);
}

#[test]
fn owned_and_unit_expr() {
    let owned = SIExprOwned::from(<Viscous as Unit>::EXPR);
    assert_eq!(owned, *<Viscous as Unit>::EXPR);
    let bytes = owned.to_ww_bytes_owned().unwrap();
    let mut buf = [0u8; 256];
    assert_eq!(
        bytes,
        <Viscous as Unit>::SI_EXPR.to_ww_bytes(&mut buf).unwrap()
    );
    let borrowed = SIExpr::from_ww_bytes(&bytes).unwrap();
    assert_eq!(SIExprOwned::from(&borrowed), owned);
    assert_eq!(SIExprOwned::from_ww_bytes_owned(&bytes).unwrap(), owned);
    assert_eq!(owned.to_string(), "N·m/(rad/s)");
    assert_eq!(owned.name().to_string(), "Per<Torque, AngularVelocity>");
}

#[test]
fn pow() {
    static METER_DEF: UnitDef<'static> = def("Meter", "m", Dim::new(0, 1, 0, 0, 0, 0, 0));
    static METER: UnitExpr = UnitExpr::Named(&METER_DEF);
    static SQUARE: UnitExpr = UnitExpr::Pow(&METER, 2);
    static SPEED_DEF: UnitDef<'static> = def("Speed", "m/s", Dim::new(-1, 1, 0, 0, 0, 0, 0));
    static SPEED: UnitExpr = UnitExpr::Named(&SPEED_DEF);
    static SPEED_SQ: UnitExpr = UnitExpr::Pow(&SPEED, 2);
    static PER_SQ: UnitExpr = UnitExpr::Div(&METER, &SQUARE);
    assert_eq!(SQUARE.to_string(), "m²");
    assert_eq!(SQUARE.name().to_string(), "Pow<Meter, 2>");
    assert_eq!(SPEED_SQ.to_string(), "(m/s)²");
    assert_eq!(PER_SQ.to_string(), "m/m²");
    assert_eq!(SPEED_SQ.dim().to_string(), "m²·s⁻²");

    let owned = SIExprOwned::from(&SPEED_SQ);
    let bytes = owned.to_ww_bytes_owned().unwrap();
    let back = SIExpr::from_ww_bytes(&bytes).unwrap();
    assert_eq!(back, SPEED_SQ);
    assert_eq!(back.to_string(), "(m/s)²");
    assert_eq!(back.dim(&()), Some(SPEED_SQ.dim()));
    // out of -8..=7 at run time: no dimension, no panic
    let too_big = SIExprOwned::Pow(Box::new(owned), 4);
    let back = too_big.to_ww_bytes_owned().unwrap();
    assert_eq!(SIExpr::from_ww_bytes(&back).unwrap().dim(&()), None);
}

#[test]
fn depth_limit() {
    let mut e = SIExprOwned::from(<units::Volt as Unit>::EXPR);
    for _ in 0..MAX_UNIT_DEPTH + 8 {
        e = SIExprOwned::Pow(Box::new(e), 1);
    }
    let bytes = e.to_ww_bytes_owned().unwrap();
    let back = SIExpr::from_ww_bytes(&bytes).unwrap();
    assert_eq!(back.dim(&()), None);
    assert_eq!(back.factor(&()), None);
    assert!(back.to_string().contains('…'));
    assert!(back.name().to_string().contains('…'));
    // `==` stops at the depth limit instead of recursing
    assert_ne!(back, *<units::Volt as Unit>::EXPR);
}

/// A typed unit of [MAX_UNIT_DEPTH] levels compiles, and every walk follows it to the end; one more level is a
/// compile error (the `compile_fail` example of `MAX_UNIT_DEPTH`).
#[test]
fn typed_depth_up_to_the_limit() {
    use units::Meter;
    type L2 = Per<Meter, Meter>;
    type L8 = Times<Per<Times<Per<Times<Per<L2, Meter>, Meter>, Meter>, Meter>, Meter>, Meter>;
    type L14 = Times<Per<Times<Per<Times<Per<L8, Meter>, Meter>, Meter>, Meter>, Meter>, Meter>;
    type L20 = Times<Per<Times<Per<Times<Per<L14, Meter>, Meter>, Meter>, Meter>, Meter>, Meter>;
    type L26 = Times<Per<Times<Per<Times<Per<L20, Meter>, Meter>, Meter>, Meter>, Meter>, Meter>;
    type L32 = Times<Per<Times<Per<Times<Per<L26, Meter>, Meter>, Meter>, Meter>, Meter>, Meter>;
    assert_eq!(<Meter as Unit>::DEPTH, 1);
    assert_eq!(<L2 as Unit>::DEPTH, 2);
    assert_eq!(<L32 as Unit>::DEPTH, MAX_UNIT_DEPTH);
    assert_eq!(<L32 as Unit>::EXPR.depth(), MAX_UNIT_DEPTH);
    assert_eq!(<units::Sqrt<units::Hertz> as Unit>::DEPTH, 2);

    let si = <L32 as Unit>::SI_EXPR;
    assert!(!si.to_string().contains('…'), "{si}");
    assert_eq!(si.dim(&()), Some(<L32 as Unit>::DIM));
    assert_eq!(*si, *<L32 as Unit>::EXPR);
    let owned = SIExprOwned::from(si);
    let bytes = owned.to_ww_bytes_owned().unwrap();
    assert_eq!(SIExprOwned::from_ww_bytes_owned(&bytes).unwrap(), owned);
}

#[test]
fn bad_bytes() {
    // `Mul` whose children don't decode: shown as `?`, no dimension
    let mut buf = [0u8; 256];
    let si = <Viscous as Unit>::SI_EXPR;
    let mut bytes = si.to_ww_bytes(&mut buf).unwrap().to_vec();
    bytes[2] = 0xf0; // a Named child's tag turned into an unknown variant
    if let Ok(back) = SIExpr::from_ww_bytes(&bytes) {
        assert_eq!(back.dim(&()), None);
        let _ = back.to_string();
    }
    for len in 0..bytes.len() {
        if let Ok(back) = SIExpr::from_ww_bytes(&bytes[..len]) {
            let _ = (
                back.dim(&()),
                back.factor(&()),
                back.to_string(),
                back.name().to_string(),
            );
        }
    }
}

/// The root flag: a noise density in V/√Hz is the square root of V²/Hz, shown with half exponents; roots of even
/// exponents are plain dimensions, and a root that isn't canonical doesn't decode.
#[test]
fn root() {
    let volt = <units::Volt as Unit>::DIM;
    let hertz = <units::Hertz as Unit>::DIM;
    let density = volt.div(hertz.sqrt());
    assert!(density.root);
    assert_eq!(density.exps()[..4], [-5, 4, 2, -2]);
    assert_eq!(density.to_string(), "kg·m²·s⁻⁵ᐟ²·A⁻¹");
    // back to volts: times √Hz, or squared times Hz
    assert_eq!(density.mul(hertz.sqrt()), volt);
    assert_eq!(
        density.checked_pow(2).unwrap().mul(hertz),
        volt.checked_pow(2).unwrap()
    );
    // √(m²) is m, not a root
    let m2 = Dim::new(0, 2, 0, 0, 0, 0, 0);
    assert_eq!(m2.sqrt(), Dim::new(0, 1, 0, 0, 0, 0, 0));
    assert_eq!(density.checked_sqrt(), None);

    let mut buf = [0u8; 16];
    let bytes = density.to_ww_bytes(&mut buf).unwrap().to_vec();
    assert_eq!(bytes, [0xb4, 0x2e, 0x00, 0x00, 0x08, 0x00]);
    assert_eq!(Dim::from_ww_bytes(&bytes).unwrap(), density);
    // m² with the root bit is √(m²) written the long way: refused
    let mut long = m2;
    long.root = true;
    assert!(!long.is_canonical());
    let bytes = long.to_ww_bytes(&mut buf).unwrap().to_vec();
    assert!(Dim::from_ww_bytes(&bytes).is_err());
}

/// The typed root, `Sqrt<U>`: V/√Hz as `Per<Volt, Sqrt<Hertz>>` has the root dimension, the symbol `V/√Hz`, and goes
/// through the wire as an `SIExpr::Sqrt` node.
#[test]
fn sqrt_marker() {
    use exact_units::units::{Hertz, Sqrt, Volt};
    type Density = Per<Volt, Sqrt<Hertz>>;
    let dim = <Density as Unit>::DIM;
    assert_eq!(dim, <Volt as Unit>::DIM.div(<Hertz as Unit>::DIM.sqrt()));
    assert!(dim.root);
    let expr = <Density as Unit>::EXPR;
    assert_eq!(expr.to_string(), "V/√Hz");
    assert_eq!(expr.name().to_string(), "Per<Volt, Sqrt<Hertz>>");
    assert_eq!(expr.dim(), dim);
    assert_eq!(
        <Sqrt<Per<Volt, Hertz>> as Unit>::EXPR.to_string(),
        "√(V/Hz)"
    );
    // √(m²) is m, √Hz in kHz is √1000 times √Hz
    assert_eq!(
        <Sqrt<Times<units::Meter, units::Meter>> as Unit>::DIM,
        <units::Meter as Unit>::DIM
    );
    assert_eq!(<Sqrt<Hertz> as Unit>::EXPR.factor(), 1.0);
    assert_eq!(exact_units::num::sqrt_f64(4.0), 2.0);
    assert_eq!(exact_units::num::sqrt_f64(1e6), 1e3);
    // rounded like std's sqrt
    for x in [
        2.0,
        3.0,
        0.5,
        1e-3,
        5e-324,
        1e-310,
        f64::MAX,
        f64::MIN_POSITIVE,
    ] {
        assert_eq!(
            exact_units::num::sqrt_f64(x).to_bits(),
            x.sqrt().to_bits(),
            "{x:e}"
        );
    }
    assert!(exact_units::num::sqrt_f64(-1.0).is_nan());

    let mut buf = [0u8; 256];
    let bytes = <Density as Unit>::SI_EXPR
        .to_ww_bytes(&mut buf)
        .unwrap()
        .to_vec();
    let back = SIExpr::from_ww_bytes(&bytes).unwrap();
    assert!(matches!(back, SIExpr::Div(..)));
    assert_eq!(back, *expr);
    assert_eq!(back.to_string(), "V/√Hz");
    assert_eq!(back.dim(&()), Some(dim));
    let owned = SIExprOwned::from(expr);
    assert_eq!(owned.to_ww_bytes_owned().unwrap(), bytes);
    assert_eq!(SIExprOwned::from(&back), owned);
    // the Sqrt node itself: variant 5 in the first nibble
    let sqrt = <Sqrt<Hertz> as Unit>::SI_EXPR
        .to_ww_bytes(&mut buf)
        .unwrap()
        .to_vec();
    assert_eq!(sqrt[0] >> 4, 5, "{sqrt:02x?}");
}

/// `prefix` and `unit` as written after a number.
fn prefixed<'t, E: tree::Tree<'t>>(prefix: Prefix, unit: &E) -> String {
    struct Written<'a, E>(Prefix, &'a E);

    impl<'t, E: tree::Tree<'t>> fmt::Display for Written<'_, E> {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            tree::write_unit(self.0, self.1, f)
        }
    }

    Written(prefix, unit).to_string()
}

/// A prefix on a powered unit reads the same from the type's tree, the wire's and the owned one, for every marker and
/// every power of ten; the wire model carries no text of it.
#[test]
fn prefixed_units_agree() {
    for &expr in units::ALL {
        let owned = SIExprOwned::from(expr);
        let bytes = owned.to_ww_bytes_owned().unwrap();
        let wire = SIExpr::from_ww_bytes(&bytes).unwrap();
        for prefix in (-63..=63).filter_map(Prefix::from_pow10) {
            let typed = prefixed(prefix, &expr);
            assert_eq!(prefixed(prefix, &&owned), typed);
            assert_eq!(prefixed(prefix, &wire), typed);
        }
    }
}

/// A scale on a quotient reads the same from a type's tree, the wire's and the owned one, a `Per` as well as a marker.
#[test]
fn quotients_agree() {
    static PER_M: UnitExpr =
        UnitExpr::Div(<units::Volt as Unit>::EXPR, <units::Meter as Unit>::EXPR);
    static LOSS: UnitExpr =
        UnitExpr::Div(<units::Decibel as Unit>::EXPR, <units::Meter as Unit>::EXPR);
    static PER_M2: UnitExpr = UnitExpr::Div(
        <units::Watt as Unit>::EXPR,
        &UnitExpr::Pow(<units::Meter as Unit>::EXPR, 2),
    );

    for expr in [&PER_M, &LOSS, &PER_M2] {
        let owned = SIExprOwned::from(expr);
        let bytes = owned.to_ww_bytes_owned().unwrap();
        let wire = SIExpr::from_ww_bytes(&bytes).unwrap();
        for prefix in (-63..=63).filter_map(Prefix::from_pow10) {
            let typed = prefixed(prefix, &expr);
            assert_eq!(prefixed(prefix, &&owned), typed);
            assert_eq!(prefixed(prefix, &wire), typed);
        }
    }
    assert_eq!(prefixed(Prefix::Milli, &&PER_M), " mV/m");
    assert_eq!(prefixed(Prefix::Hecto, &&PER_M), " V/cm");
    assert_eq!(prefixed(Prefix::Milli, &&LOSS), " dB/km");
    assert_eq!(prefixed(Prefix::Deca, &&PER_M2), " mW/cm²");

    // nothing says this `m` is a metre
    let not_a_metre = SIExprOwned::Div(
        Box::new(SIExprOwned::from(<units::Decibel as Unit>::EXPR)),
        Box::new(owned("m", "m", Dim::ONE)),
    );
    assert_eq!(prefixed(Prefix::Milli, &&not_a_metre), " m(dB/m)");
}

/// Metres to a power in a tree: `UnitExpr::Pow` takes the prefix inside as a named symbol does, a parenthesis in front
/// or another unit keeps it outside, and a part known by name only is not known to be a metre.
#[test]
fn prefixes_on_trees() {
    const METER: &UnitExpr = <units::Meter as Unit>::EXPR;
    const SECOND: &UnitExpr = <units::Second as Unit>::EXPR;
    static M2: UnitExpr = UnitExpr::Pow(METER, 2);
    static M3: UnitExpr = UnitExpr::Pow(METER, 3);
    static M4: UnitExpr = UnitExpr::Pow(METER, 4);
    static FLOW: UnitExpr = UnitExpr::Div(&M3, SECOND);
    static PER_S: UnitExpr = UnitExpr::Div(&FLOW, SECOND);
    static TIMES: UnitExpr = UnitExpr::Mul(&M2, SECOND);
    static IN_FRONT: UnitExpr = UnitExpr::Mul(&FLOW, SECOND);
    static V2: UnitExpr = UnitExpr::Pow(<units::Volt as Unit>::EXPR, 2);

    assert_eq!(prefixed(Prefix::Micro, &&M2), " mm²");
    assert_eq!(prefixed(Prefix::Milli, &&M2), " m(m²)");
    // not litres: a volume spelled as a power of the metre
    assert_eq!(prefixed(Prefix::Milli, &&M3), " dm³");
    assert_eq!(prefixed(Prefix::Milli, &&FLOW), " dm³/s");
    assert_eq!(prefixed(Prefix::Nano, &&PER_S), " n((m³/s)/s)");
    assert_eq!(prefixed(Prefix::Micro, &&TIMES), " mm²·s");
    assert_eq!(prefixed(Prefix::Nano, &&IN_FRONT), " n((m³/s)·s)");
    assert_eq!(prefixed(Prefix::Pico, &&M4), " p(m⁴)");
    assert_eq!(prefixed(Prefix::Micro, &&V2), " µ(V²)");

    let name_only = owned("m³", "m³", Dim::ONE);
    assert_eq!(prefixed(Prefix::Micro, &&name_only), " µ(m³)");
}

/// Volt as an owned tree, its definition changed by `f`.
fn volt_with(f: impl FnOnce(&mut UnitDefOwned)) -> SIExprOwned {
    let Some(volt) = <units::Volt as Unit>::EXPR.def() else {
        panic!("Volt is a named unit")
    };
    let mut d = UnitDefOwned::from(*volt);
    f(&mut d);
    SIExprOwned::Def(d)
}

/// A named unit read from the wire is a known unit only when all of it matches: one that differs from Volt in one
/// part (a forged or mistaken description) is neither `==` Volt nor `known()`, owned or decoded.
#[test]
fn forged_named_units() {
    let volt = <units::Volt as Unit>::EXPR;
    let ampere = <units::Ampere as Unit>::EXPR;
    let real = volt_with(|_| {});
    assert_eq!(real, *volt);
    assert_eq!(real.known(), Some(volt));
    let forged = [
        volt_with(|d| d.id.name = "Volts".into()),
        // another crate's Volt is another unit
        volt_with(|d| d.id.crate_name = "my_api".into()),
        volt_with(|d| d.names.symbol = "A".into()),
        volt_with(|d| d.dim = <units::Ampere as Unit>::DIM.into()),
        volt_with(|d| d.scale = Magnitude::pow10(-3)),
        SIExprOwned::from(ampere),
    ];
    for f in &forged {
        assert_ne!(*f, *volt, "{f:?}");
        assert_eq!(f.known().filter(|k| *k == volt), None, "{f:?}");
        let bytes = f.to_ww_bytes_owned().unwrap();
        let decoded = SIExpr::from_ww_bytes(&bytes).unwrap();
        assert_ne!(decoded, *volt, "{f:?}");
        assert_eq!(decoded.known().filter(|k| *k == volt), None, "{f:?}");
        let owned = SIExprOwned::from_ww_bytes_owned(&bytes).unwrap();
        assert_ne!(owned, *volt, "{f:?}");
        assert_eq!(owned.known().filter(|k| *k == volt), None, "{f:?}");
    }
    // Volt's name on Ampere's description is not a known unit at all
    let SIExprOwned::Def(mut renamed) = SIExprOwned::from(ampere) else {
        panic!("Ampere is a named unit")
    };
    renamed.id.name = "Volt".into();
    let renamed = SIExprOwned::Def(renamed);
    assert_eq!(renamed.known(), None);
    let bytes = renamed.to_ww_bytes_owned().unwrap();
    assert_eq!(SIExpr::from_ww_bytes(&bytes).unwrap().known(), None);
    // a compound unit is never a known named unit
    let per = SIExprOwned::Div(Box::new(real.clone()), Box::new(real.clone()));
    assert_eq!(per.known(), None);
}

/// Compound trees are the same only in the same shape: the same exponent, the same operator, the same order.
#[test]
fn compound_shapes() {
    let m = || SIExprOwned::from(<units::Meter as Unit>::EXPR);
    let s = || SIExprOwned::from(<units::Second as Unit>::EXPR);
    let square: &'static UnitExpr =
        Box::leak(Box::new(UnitExpr::Pow(<units::Meter as Unit>::EXPR, 2)));
    let speed: &'static UnitExpr = Box::leak(Box::new(UnitExpr::Div(
        <units::Meter as Unit>::EXPR,
        <units::Second as Unit>::EXPR,
    )));
    let root: &'static UnitExpr = Box::leak(Box::new(UnitExpr::Sqrt(<units::Meter as Unit>::EXPR)));
    let cube = SIExprOwned::Pow(Box::new(m()), 3);
    let cases = [
        (SIExprOwned::Pow(Box::new(m()), 2), square, true),
        (cube, square, false),
        (SIExprOwned::Pow(Box::new(s()), 2), square, false),
        (SIExprOwned::Div(Box::new(m()), Box::new(s())), speed, true),
        (SIExprOwned::Mul(Box::new(m()), Box::new(s())), speed, false),
        (SIExprOwned::Div(Box::new(s()), Box::new(m())), speed, false),
        (SIExprOwned::Div(Box::new(m()), Box::new(m())), speed, false),
        (SIExprOwned::Sqrt(Box::new(m())), root, true),
        (SIExprOwned::Sqrt(Box::new(s())), root, false),
        (m(), root, false),
    ];
    for (e, typed, same) in cases {
        assert_eq!(e == *typed, same, "{e:?}");
        let bytes = e.to_ww_bytes_owned().unwrap();
        assert_eq!(
            SIExpr::from_ww_bytes(&bytes).unwrap() == *typed,
            same,
            "{e:?}"
        );
    }
}

/// A ratio as a typed unit: no symbol, dimensionless, coherent.
static RATIO_DEF: UnitDef<'static> = def("Ratio", "", Dim::ONE);
/// [RATIO_DEF] as a typed unit.
static RATIO: UnitExpr = UnitExpr::Named(&RATIO_DEF);

/// `RATIO` in `levels` levels of the tree `wrap` makes, as a typed tree.
fn deep_typed(levels: usize, wrap: fn(&'static UnitExpr) -> UnitExpr) -> &'static UnitExpr {
    let mut e = &RATIO;
    for _ in 1..levels {
        e = Box::leak(Box::new(wrap(e)));
    }
    e
}

/// Past [MAX_UNIT_DEPTH] every walk stops, whatever the nodes: two equal trees that deep are not the same unit, have
/// no magnitude, are not ratios and are written with `…`; a tree of [MAX_UNIT_DEPTH] levels is all of these.
#[test]
fn depth_limit_every_node() {
    let wraps: [fn(&'static UnitExpr) -> UnitExpr; 6] = [
        |e| UnitExpr::Pow(e, 1),
        |e| UnitExpr::Mul(e, &RATIO),
        |e| UnitExpr::Div(e, &RATIO),
        // the deep side on the right: each child counts its own level
        |e| UnitExpr::Mul(&RATIO, e),
        |e| UnitExpr::Div(&RATIO, e),
        UnitExpr::Sqrt,
    ];
    for wrap in wraps {
        for levels in [MAX_UNIT_DEPTH, MAX_UNIT_DEPTH + 1] {
            let typed = deep_typed(levels, wrap);
            let within = levels <= MAX_UNIT_DEPTH;
            let owned = SIExprOwned::from(typed);
            let bytes = owned.to_ww_bytes_owned().unwrap();
            let decoded = SIExpr::from_ww_bytes(&bytes).unwrap();
            assert_eq!(owned == *typed, within, "{levels}: {owned}");
            assert_eq!(decoded == *typed, within, "{levels}: {decoded}");
            assert_eq!(tree::same(&decoded, &&owned), within, "{levels}: {decoded}");
            assert_eq!(
                decoded.magnitude(&()).is_some(),
                within,
                "{levels}: {decoded}"
            );
            assert_eq!(owned.magnitude(&()).is_some(), within, "{levels}: {owned}");
            assert_eq!(tree::is_ratio(&decoded), within, "{levels}: {decoded}");
            assert_eq!(decoded.factor(&()).is_some(), within, "{levels}: {decoded}");
            assert_eq!(decoded.dim(&()).is_some(), within, "{levels}: {decoded}");
            let shown = (decoded.to_string(), decoded.name().to_string());
            assert_eq!(!shown.0.contains('…'), within, "{levels}: {shown:?}");
            assert_eq!(!shown.1.contains('…'), within, "{levels}: {shown:?}");
        }
    }
}

/// The `Pow` levels above the first named unit of an owned tree, and that unit's name.
fn pow_levels(mut e: &SIExprOwned) -> (usize, &str) {
    let mut levels = 0;
    while let SIExprOwned::Pow(a, _) = e {
        levels += 1;
        e = a;
    }
    let SIExprOwned::Def(d) = e else {
        panic!("a chain of Pow over a named unit")
    };
    (levels, &d.id.name)
}

/// A tree read from the wire becomes owned only up to [MAX_UNIT_DEPTH] levels, the rest is the unknown unit `?`:
/// however deep the bytes, the owned tree a client keeps is bounded.
#[test]
fn owned_from_wire_stops_at_the_limit() {
    let volt = SIExprOwned::from(<units::Volt as Unit>::EXPR);
    for levels in [MAX_UNIT_DEPTH - 1, MAX_UNIT_DEPTH, MAX_UNIT_DEPTH + 1, 100] {
        let mut e = volt.clone();
        for _ in 1..levels {
            e = SIExprOwned::Pow(Box::new(e), 1);
        }
        let bytes = e.to_ww_bytes_owned().unwrap();
        let owned = SIExprOwned::from(&SIExpr::from_ww_bytes(&bytes).unwrap());
        let (pows, name) = pow_levels(&owned);
        if levels <= MAX_UNIT_DEPTH {
            assert_eq!((pows, name), (levels - 1, "Volt"));
            assert_eq!(owned, e);
        } else {
            assert_eq!((pows, name), (MAX_UNIT_DEPTH, "?"));
        }
    }
}

/// Bytes outside what a type holds are refused on reading, borrowed and owned: an [exact_units::Index] past its
/// bounds, a negative zero [exact_units::Offset], a dimension with its counted bases out of order or a root it
/// doesn't need, and a unit tree with such a dimension.
#[test]
fn validate_refuses() {
    use exact_units::{DimOwned, Index, Offset, Range};
    type Slot = Index<units::Sample, u8, Range<0, 7>>;
    let err = shrink_wrap::Error::SubtypeOutOfRange;
    assert_eq!(Slot::from_ww_bytes(&[7]).unwrap().get(), 7);
    assert_eq!(Slot::from_ww_bytes(&[8]).err(), Some(err));
    assert_eq!(Slot::from_ww_bytes_owned(&[8]).err(), Some(err));

    let mut buf = [0u8; 16];
    let zero = Offset::ZERO.to_ww_bytes(&mut buf).unwrap().to_vec();
    assert_eq!(zero, [0x00]);
    assert_eq!(Offset::from_ww_bytes(&zero), Ok(Offset::ZERO));
    // the sign bit set on zero
    assert_eq!(Offset::from_ww_bytes(&[0x80]).err(), Some(err));
    assert_eq!(Offset::from_ww_bytes_owned(&[0x80]).err(), Some(err));
    let minus = Offset::new(true, Magnitude::ratio(1, 2));
    let bytes = minus.to_ww_bytes(&mut buf).unwrap().to_vec();
    assert_eq!(Offset::from_ww_bytes(&bytes), Ok(minus));

    let counted = Dim::counted("exact_units", "Sample")
        .mul(Dim::counted("exact_units", "Block"))
        .div(Dim::counted("exact_units", "Frame"));
    let mut unsorted = counted;
    unsorted.counted.swap(0, 2);
    let mut long = Dim::new(0, 2, 0, 0, 0, 0, 0);
    long.root = true;
    for bad in [unsorted, long] {
        assert!(!bad.is_canonical());
        let mut buf = [0u8; 256];
        let bytes = bad.to_ww_bytes(&mut buf).unwrap().to_vec();
        assert_eq!(Dim::from_ww_bytes(&bytes).err(), Some(err), "{bad:?}");
        assert_eq!(
            DimOwned::from_ww_bytes_owned(&bytes).err(),
            Some(err),
            "{bad:?}"
        );
        let mut unit = UnitDefOwned::from(def("Forged", "f", Dim::ONE));
        unit.dim = bad.into();
        let unit = SIExprOwned::Def(unit);
        let bytes = unit.to_ww_bytes_owned().unwrap();
        assert!(SIExprOwned::from_ww_bytes_owned(&bytes).is_err(), "{bad:?}");
        assert!(SIExpr::from_ww_bytes(&bytes).is_err(), "{bad:?}");
    }
    let mut buf = [0u8; 256];
    let bytes = counted.to_ww_bytes(&mut buf).unwrap().to_vec();
    assert_eq!(DimOwned::from_ww_bytes_owned(&bytes), Ok(counted.into()));
}

/// A `Ref` on the wire: the variant, then the identity alone, two strings.
#[test]
fn ref_bytes() {
    let mut buf = [0u8; 64];
    let id = <units::Volt as Unit>::EXPR.def().unwrap().id;
    let bytes = SIExpr::Ref(id).to_ww_bytes(&mut buf).unwrap().to_vec();
    assert_eq!(bytes.first().map(|b| b >> 4), Some(1), "{bytes:02x?}");
    assert_eq!((bytes.len(), VOLT_BYTES.len()), (18, 45));
    let back = SIExpr::from_ww_bytes(&bytes).unwrap();
    assert_eq!(back, SIExpr::Ref(id));
    assert_eq!(back.known(), Some(<units::Volt as Unit>::EXPR));
    assert_eq!(back.dim(&units::TABLE), Some(<units::Volt as Unit>::DIM));
    assert_eq!(back.dim(&()), None);
    assert_eq!(back.to_string(), "V");
    let owned = SIExprOwned::from(&back);
    assert_eq!(owned.to_ww_bytes_owned().unwrap(), bytes);
    assert_eq!(SIExprOwned::from_ww_bytes_owned(&bytes).unwrap(), owned);
}

/// A unit read with serde says where its scale comes from: one without a provenance is refused, never taken for a
/// definition (`Provenance` has no `Default`).
#[cfg(feature = "serde")]
#[test]
fn serde_def_without_provenance_is_refused() {
    let Some(volt) = <units::Volt as Unit>::EXPR.def() else {
        panic!("Volt is a named unit")
    };
    let Ok(serde_json::Value::Object(mut json)) = serde_json::to_value(UnitDefOwned::from(*volt))
    else {
        panic!("a UnitDefOwned is a JSON object")
    };
    let full = serde_json::Value::Object(json.clone());
    assert!(serde_json::from_value::<UnitDefOwned>(full).is_ok());
    assert!(json.remove("provenance").is_some());
    let without = serde_json::Value::Object(json);
    assert!(serde_json::from_value::<UnitDefOwned>(without).is_err());
}
