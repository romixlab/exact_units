//! Marker units of the angle, information and counting groups, and prefix names.
#![cfg(all(feature = "std", feature = "quantities"))]

use exact_units::units::{self, Per};
use exact_units::{MarkerId, Prefix, Unit};
#[cfg(feature = "shrink_wrap")]
use shrink_wrap::prelude::*;

/// rad, bit and distinguished bases are dimensions of their own (STD-60): bytes never turn into a ratio, an angular
/// velocity is not a frequency, and a count per thing times the thing is the count.
#[test]
fn angle_information_counting() {
    let byte = <units::Byte as Unit>::DIM;
    assert_eq!(byte, <units::Bit as Unit>::DIM);
    assert_eq!(byte.to_string(), "bit");
    assert!(!byte.same(<units::Ratio as Unit>::DIM));
    assert!(!byte.same(<units::Radian as Unit>::DIM));
    assert_eq!(<units::Byte as Unit>::EXPR.factor(), 8.0);
    assert_eq!(<units::Nibble as Unit>::EXPR.factor(), 4.0);
    assert_eq!(<units::Halfword as Unit>::EXPR.factor(), 16.0);
    assert_eq!(<units::Word as Unit>::EXPR.factor(), 32.0);
    assert_eq!(<units::Doubleword as Unit>::EXPR.factor(), 64.0);
    assert_eq!(<units::Doubleword as Unit>::DIM, byte);
    assert!(!<units::AngularVelocity as Unit>::DIM.same(<units::Hertz as Unit>::DIM));
    assert!(
        <units::AngularVelocity as Unit>::DIM
            .strip(MarkerId::RADIAN)
            .same(<units::Hertz as Unit>::DIM)
    );
    assert!(
        <units::Torque as Unit>::DIM
            .strip(MarkerId::RADIAN)
            .same(<units::Joule as Unit>::DIM)
    );
    assert!(!<units::Torque as Unit>::DIM.same(<units::Joule as Unit>::DIM));
    assert_eq!(<units::AngularVelocity as Unit>::DIM.to_string(), "s⁻¹·rad");
    assert!(!<units::Sample as Unit>::DIM.same(<units::Packet as Unit>::DIM));
    for (a, b) in [
        (<units::Line as Unit>::DIM, <units::Row as Unit>::DIM),
        (<units::Record as Unit>::DIM, <units::Entry as Unit>::DIM),
        (<units::Line as Unit>::DIM, <units::Item as Unit>::DIM),
    ] {
        assert!(!a.same(b));
    }
    assert!(!<units::Sample as Unit>::DIM.is_one());
    let per_sample = <Per<units::Byte, units::Sample> as Unit>::DIM;
    assert_eq!(per_sample.to_string(), "bit·Sample⁻¹");
    assert!(
        per_sample
            .checked_mul(&<units::Sample as Unit>::DIM)
            .unwrap()
            .same(byte)
    );
    let rate = <Per<units::Sample, units::Second> as Unit>::DIM;
    assert!(!rate.same(<units::Hertz as Unit>::DIM));
    assert!(
        rate.checked_mul(&<units::Second as Unit>::DIM)
            .unwrap()
            .same(<units::Sample as Unit>::DIM)
    );
}

#[test]
fn prefix_names() {
    for prefix in Prefix::ALL {
        #[cfg(feature = "shrink_wrap")]
        {
            let bytes = prefix.to_ww_bytes_owned().unwrap();
            let back = Prefix::from_ww_bytes_owned(&bytes).unwrap();
            assert_eq!(back, prefix);
        }
        assert_eq!(Prefix::from_name(prefix.name()), Some(prefix));
        assert_eq!(format!("{prefix:?}"), prefix.name());
    }
    assert_eq!(Prefix::from_name("Milli"), Some(Prefix::Milli));
    assert_eq!(Prefix::from_name("milli"), None);
}

/// Quantities that share a dimension (UNIT-2, UNIT-8): a quotient keeps what cancelled, a product is derived, and Ω
/// and S are kinds of their own.
#[test]
fn quantities_of_markers() {
    use exact_units::quantities::{Conductance, Resistance};
    use exact_units::{Exact, Fixed, LevelForm, QuantityExpr, prefix::One};
    use units::{Ampere, Decibel, Ohm, Ratio, Siemens, Times, Volt, Watt};

    let volt = <Volt as Unit>::DIM;
    assert_eq!(
        <Per<Volt, Volt> as Unit>::QUANTITY,
        QuantityExpr::RatioOf(volt)
    );
    assert_eq!(<Per<Volt, Volt> as Unit>::DIM, <Ratio as Unit>::DIM);
    let no = |a: QuantityExpr<'_>, b: QuantityExpr<'_>| a.same(b, &()) == Some(false);
    assert!(no(
        <Per<Volt, Volt> as Unit>::QUANTITY,
        <Per<Ampere, Ampere> as Unit>::QUANTITY
    ));
    assert!(no(
        <Per<Volt, Volt> as Unit>::QUANTITY,
        <Ratio as Unit>::QUANTITY
    ));
    assert_eq!(<Ohm as Unit>::QUANTITY, QuantityExpr::of::<Resistance>());
    assert_eq!(
        <Siemens as Unit>::QUANTITY,
        QuantityExpr::of::<Conductance>()
    );
    let ohm = <Ohm as Unit>::DIM;
    assert_eq!(
        <Per<Volt, Ampere> as Unit>::QUANTITY,
        QuantityExpr::Derived(ohm)
    );
    assert_eq!(
        <Times<Per<Volt, Volt>, Volt> as Unit>::QUANTITY,
        QuantityExpr::Derived(volt)
    );
    assert_eq!(
        <Times<Per<Volt, Volt>, Per<Volt, Volt>> as Unit>::QUANTITY,
        QuantityExpr::RatioOf(volt)
    );
    assert_eq!(
        <Per<Ohm, Ratio> as Unit>::QUANTITY,
        QuantityExpr::of::<Resistance>()
    );
    assert_eq!(
        <Per<Watt, Watt> as Unit>::QUANTITY.level_form(),
        Some(LevelForm::Power)
    );
    assert_eq!(
        <Per<Ampere, Ampere> as Unit>::QUANTITY.level_form(),
        Some(LevelForm::RootPower)
    );

    // the named steps keep the number
    let g = Fixed::<Per<Volt, Volt>, One, i16>::new(-3);
    assert_eq!(g.as_ratio().get(), -3);
    assert_eq!(g.as_ratio().as_gain::<Volt>(), g);
    let gm = Fixed::<Per<Ampere, Volt>, One, f32>::bounded(0.02);
    assert_eq!(gm.as_conductance().as_transconductance(), gm);
    let r = Fixed::<Ohm, One, u8>::new(50);
    assert_eq!(r.as_transimpedance().as_resistance(), r);
    // and an Exact's, as a Fixed's
    let q = (Exact::<Volt>::new(3, 2) / Exact::<Volt>::int(-2)).unwrap();
    assert_eq!(q.as_ratio(), Exact::<Ratio>::new(-3, 4));
    assert_eq!(q.as_ratio().as_gain::<Volt>(), q);
    // scaled by a ratio keeps the unit, refusing what doesn't fit
    let v = Exact::<Volt>::new(3, 2);
    assert_eq!(v.scaled(Exact::new(2, 3)), Some(Exact::int(1)));
    assert_eq!(
        (v * Exact::<Ratio>::new(2, 3)).unwrap().convert::<Volt>(),
        Some(Exact::int(1))
    );
    assert_eq!(Exact::<Volt>::int(i128::MAX).scaled(Exact::int(4)), None);

    // decibels by kind: 20·lg for V/V and A/A, 10·lg for W/W, as a level in dB
    let db = |x: Option<Fixed<Decibel, One, f64>>, y: f64| (x.unwrap().get() - y).abs() < 1e-12;
    assert!(db(
        Fixed::<Per<Volt, Volt>, One, f64>::bounded(2.0).to_db(),
        20.0 * 2f64.log10()
    ));
    assert!(db(
        Fixed::<Per<Watt, Watt>, One, f64>::bounded(2.0).to_db(),
        10.0 * 2f64.log10()
    ));
    assert!(db(
        Fixed::<Per<Ampere, Ampere>, One, f64>::bounded(0.5).to_db(),
        -20.0 * 2f64.log10()
    ));
    assert_eq!(
        Fixed::<Per<Volt, Volt>, One, f64>::bounded(-1.0).to_db(),
        None
    );
    let back = Fixed::<Per<Watt, Watt>, One, f64>::from_db(-3.0)
        .unwrap()
        .get();
    assert!((back - 10f64.powf(-0.3)).abs() < 1e-12);
}

/// Minutes, hours, days and weeks are exact scales of the second: they convert both ways where the count is whole.
#[test]
fn time_units() {
    use exact_units::prefix::{Milli, One};
    use exact_units::{Days, Fixed, Hours, Minutes, Secs};

    let s = <units::Second as Unit>::DIM;
    assert_eq!(<units::Minute as Unit>::DIM, s);
    assert_eq!(<units::Week as Unit>::DIM, s);
    assert_eq!(<units::Minute as Unit>::EXPR.factor(), 60.0);
    assert_eq!(<units::Hour as Unit>::EXPR.factor(), 3600.0);
    assert_eq!(<units::Day as Unit>::EXPR.factor(), 86_400.0);
    assert_eq!(<units::Week as Unit>::EXPR.factor(), 604_800.0);
    assert!(units::by_name("Hour").is_some());

    assert_eq!(
        Hours::new(2u32).convert::<units::Second, One, u32>(),
        Some(Secs::new(7200))
    );
    assert_eq!(
        Days::new(1u32).convert::<units::Second, Milli, u32>(),
        Some(Fixed::new(86_400_000))
    );
    assert_eq!(
        Fixed::<units::Week, One, u8>::new(2).convert::<units::Day, One, u8>(),
        Some(Days::new(14))
    );
    assert_eq!(
        Secs::new(5400u32).convert::<units::Minute, One, u16>(),
        Some(Minutes::new(90))
    );
    // 90 min is not a whole number of hours
    assert_eq!(Minutes::new(90u16).convert::<units::Hour, One, u16>(), None);
    assert_eq!(Hours::new(36u16).to_string(), "36 h");
    assert_eq!(Minutes::new(5u8).to_string(), "5 min");
}

/// The derived units of SI Brochure tables 5 and 6 and their everyday scales: each is the dimension of the product or
/// quotient it is named for, converts from it where both are plain, and keeps its kind where a named marker shares
/// its dimension.
#[test]
fn derived_units() {
    use exact_units::Dim;
    use units::{
        AbsorbedDoseRate, Ampere, AngularAcceleration, AngularMomentum, AngularVelocity, Area,
        Candela, ChargeDensity, Conductivity, Coulomb, CurrentDensity, Density, DynamicViscosity,
        ElectricDisplacement, ElectricFieldStrength, EnergyDensity, Entropy, Exposure, Farad, Gram,
        GramPerCubicCentimeter, GramPerMole, Gray, HeatCapacity, HeatFluxDensity,
        HeatTransferCoefficient, Henry, Irradiance, Joule, Kelvin, KinematicViscosity, Lumen,
        LuminousEfficacy, MagneticFieldStrength, MassFlow, Meter, Molality, MolarConcentration,
        MolarEnergy, MolarHeatCapacity, MolarMass, Mole, MolePerLitre, MomentOfInertia, Momentum,
        Newton, Nit, Ohm, Pascal, Permeability, Permittivity, PowerDensity, Radiance,
        RadiantIntensity, Resistivity, Second, Siemens, Sievert, SievertPerHour,
        SpecificHeatCapacity, SpecificVolume, Steradian, SurfaceDensity, SurfaceTension,
        ThermalConductance, ThermalConductivity, ThermalResistance, Times, Torque, Volt, Volume,
        Watt, Wavenumber,
    };

    fn dim<U: Unit>() -> Dim<'static> {
        U::DIM
    }
    // each named marker has the dimension of what it is named for
    macro_rules! same {
        ($($named:ty = $built:ty;)*) => {$(
            assert_eq!(dim::<$named>(), dim::<$built>(), stringify!($named = $built));
        )*};
    }
    same! {
        Area = Times<Meter, Meter>;
        Volume = Times<Area, Meter>;
        Wavenumber = Per<units::Ratio, Meter>;
        Density = Per<Gram, Volume>;
        GramPerCubicCentimeter = Density;
        SurfaceDensity = Per<Gram, Area>;
        SpecificVolume = Per<Volume, Gram>;
        MassFlow = Per<Gram, Second>;
        Momentum = Times<Newton, Second>;
        AngularMomentum = Times<Torque, Second>;
        AngularMomentum = Times<MomentOfInertia, AngularVelocity>;
        Torque = Times<MomentOfInertia, AngularAcceleration>;
        Joule = Times<MomentOfInertia, Times<AngularVelocity, AngularVelocity>>;
        SurfaceTension = Per<Newton, Meter>;
        DynamicViscosity = Times<Pascal, Second>;
        KinematicViscosity = Per<DynamicViscosity, Density>;
        EnergyDensity = Per<Joule, Volume>;
        PowerDensity = Per<Watt, Volume>;
        ElectricFieldStrength = Per<Volt, Meter>;
        MagneticFieldStrength = Per<Ampere, Meter>;
        CurrentDensity = Per<Ampere, Area>;
        ChargeDensity = Per<Coulomb, Volume>;
        ElectricDisplacement = Per<Coulomb, Area>;
        Permittivity = Per<Farad, Meter>;
        Permeability = Per<Henry, Meter>;
        Resistivity = Times<Ohm, Meter>;
        Conductivity = Per<Siemens, Meter>;
        HeatFluxDensity = Per<Watt, Area>;
        HeatCapacity = Per<Joule, Kelvin>;
        Entropy = Per<Joule, Kelvin>;
        SpecificHeatCapacity = Per<HeatCapacity, Gram>;
        ThermalConductivity = Per<Watt, Times<Meter, Kelvin>>;
        ThermalConductance = Per<Watt, Kelvin>;
        ThermalResistance = Per<Kelvin, Watt>;
        HeatTransferCoefficient = Per<Watt, Times<Area, Kelvin>>;
        MolarConcentration = Per<Mole, Volume>;
        MolePerLitre = MolarConcentration;
        Molality = Per<Mole, Gram>;
        MolarMass = Per<Gram, Mole>;
        GramPerMole = MolarMass;
        MolarEnergy = Per<Joule, Mole>;
        MolarHeatCapacity = Per<Joule, Times<Mole, Kelvin>>;
        Nit = Per<Candela, Area>;
        LuminousEfficacy = Per<Lumen, Watt>;
        Irradiance = Per<Watt, Area>;
        RadiantIntensity = Per<Watt, Steradian>;
    }
    assert_eq!(dim::<Radiance>(), dim::<Per<RadiantIntensity, Area>>());
    assert_eq!(dim::<Exposure>(), dim::<Per<Coulomb, Gram>>());
    assert_eq!(dim::<AbsorbedDoseRate>(), dim::<Per<Gray, Second>>());
    assert_eq!(dim::<SievertPerHour>(), dim::<Per<Sievert, Second>>());
    // a moment of inertia is per rad², so it is no plain kg·m²; an angular momentum is no J·s
    assert!(!dim::<MomentOfInertia>().same(dim::<Times<Gram, Area>>()));
    assert!(!dim::<AngularMomentum>().same(dim::<Times<Joule, Second>>()));
    // a K/W times a W is a temperature rise
    assert_eq!(dim::<Times<ThermalResistance, Watt>>(), dim::<Kelvin>());
}

/// The derived units measure their dimension's quantity but for two, `Of` aliases over a compound, and the related
/// ones convert exactly.
#[test]
fn derived_quantities_and_conversions() {
    use exact_units::prefix::{Kilo, Micro, Milli, One};
    use exact_units::{Fixed, QuantityExpr, quantities};
    use units::{
        AbsorbedDoseRate, Acceleration, Area, Candela, Density, EnergyDensity, Entropy,
        GramPerCubicCentimeter, GramPerMole, HeatCapacity, HeatFluxDensity, Irradiance, Joule,
        Kelvin, MolarConcentration, MolarMass, MolePerLitre, Nit, Resistivity, Second,
        SievertPerHour, Speed, ThermalResistance, Volume, Watt,
    };

    // quantities: derived but for the two that share a named marker's dimension
    assert_eq!(
        <EnergyDensity as Unit>::QUANTITY,
        QuantityExpr::of::<quantities::EnergyDensity>()
    );
    assert_eq!(
        <Entropy as Unit>::QUANTITY,
        QuantityExpr::of::<quantities::Entropy>()
    );
    for (q, d) in [
        (<Irradiance as Unit>::QUANTITY, <Irradiance as Unit>::DIM),
        (
            <HeatFluxDensity as Unit>::QUANTITY,
            <HeatFluxDensity as Unit>::DIM,
        ),
        (
            <HeatCapacity as Unit>::QUANTITY,
            <HeatCapacity as Unit>::DIM,
        ),
        (
            <ThermalResistance as Unit>::QUANTITY,
            <ThermalResistance as Unit>::DIM,
        ),
        (<Resistivity as Unit>::QUANTITY, <Resistivity as Unit>::DIM),
        (<Nit as Unit>::QUANTITY, <Nit as Unit>::DIM),
    ] {
        assert_eq!(q, QuantityExpr::Derived(d));
    }

    // conversions between the related ones, exact
    assert_eq!(
        Fixed::<MolePerLitre, Milli, u32>::new(150).convert::<MolarConcentration, One, u32>(),
        Some(Fixed::new(150))
    );
    assert_eq!(
        Fixed::<MolarConcentration, One, u32>::new(2500).convert::<MolePerLitre, One, u32>(),
        None
    );
    assert_eq!(
        Fixed::<MolarConcentration, One, u32>::new(2000).convert::<MolePerLitre, One, u32>(),
        Some(Fixed::new(2))
    );
    assert_eq!(
        Fixed::<GramPerCubicCentimeter, Milli, u32>::new(998).convert::<Density, One, u32>(),
        Some(Fixed::new(998))
    );
    assert_eq!(
        Fixed::<GramPerMole, Milli, u32>::new(18015).convert::<MolarMass, Micro, u32>(),
        Some(Fixed::new(18015))
    );
    assert_eq!(
        Fixed::<SievertPerHour, Micro, u64>::new(3_600_000)
            .convert::<AbsorbedDoseRate, Micro, u64>(),
        Some(Fixed::new(1000))
    );
    assert_eq!(
        Fixed::<Irradiance, One, u16>::new(1000).convert::<HeatFluxDensity, One, u16>(),
        Some(Fixed::new(1000))
    );
    assert_eq!(
        Fixed::<Per<Watt, Area>, Kilo, u16>::new(1).convert::<Irradiance, One, u16>(),
        Some(Fixed::new(1000))
    );
    assert_eq!(
        Fixed::<Per<Candela, Area>, One, u16>::new(500).convert::<Nit, One, u16>(),
        Some(Fixed::new(500))
    );
    assert_eq!(
        Fixed::<Per<Kelvin, Watt>, Milli, u16>::new(2500).convert::<ThermalResistance, One, f32>(),
        Some(Fixed::<_, _, f32>::bounded(2.5))
    );
    assert_eq!(
        Fixed::<Per<Speed, Acceleration>, One, u8>::new(3).convert::<Second, One, u8>(),
        Some(Fixed::new(3))
    );
    assert_eq!(
        Fixed::<Per<Joule, Volume>, One, u8>::new(7)
            .as_energy_density()
            .as_joule_per_volume(),
        Fixed::new(7)
    );
    assert_eq!(
        Fixed::<Per<Joule, Kelvin>, One, u8>::new(7)
            .as_entropy()
            .as_heat_capacity(),
        Fixed::new(7)
    );
}

/// Every derived unit is found by its name.
#[test]
fn derived_names() {
    for name in [
        "Area",
        "Volume",
        "Wavenumber",
        "Density",
        "GramPerCubicCentimeter",
        "SurfaceDensity",
        "SpecificVolume",
        "MassFlow",
        "Momentum",
        "AngularMomentum",
        "MomentOfInertia",
        "SurfaceTension",
        "DynamicViscosity",
        "KinematicViscosity",
        "PowerDensity",
        "ElectricFieldStrength",
        "MagneticFieldStrength",
        "CurrentDensity",
        "ChargeDensity",
        "ElectricDisplacement",
        "Permittivity",
        "Permeability",
        "Resistivity",
        "Conductivity",
        "HeatFluxDensity",
        "HeatCapacity",
        "SpecificHeatCapacity",
        "ThermalConductivity",
        "ThermalConductance",
        "ThermalResistance",
        "HeatTransferCoefficient",
        "MolarConcentration",
        "MolePerLitre",
        "Molality",
        "MolarMass",
        "GramPerMole",
        "MolarEnergy",
        "MolarHeatCapacity",
        "Nit",
        "LuminousEfficacy",
        "Irradiance",
        "RadiantIntensity",
        "Radiance",
        "Exposure",
        "AbsorbedDoseRate",
        "SievertPerHour",
    ] {
        assert!(units::by_name(name).is_some(), "{name}");
    }
    let aliases = |name| match units::by_name(name) {
        Some(exact_units::UnitExpr::Named(def)) => def.names.aliases().collect::<Vec<_>>(),
        _ => panic!("{name} is no named unit"),
    };
    assert_eq!(aliases("Nit"), ["nit", "nt"]);
    assert_eq!(aliases("Volt"), [""; 0]);
}

/// No two named units share a symbol but the pairs that are one unit of two quantities (SI Brochure table 6 lists each
/// pair on one line): W/m² of heat flux and irradiance; m²·s⁻² of absorbed and equivalent dose differ in their
/// symbols. A loss shares its gain's symbol (dB, Np): their dimensions tell them apart, and so do the bel's and the
/// byte's B. Entropy, energy density, a damping ratio and a quality factor are no named units but quantities of
/// one (`Of<Entropy, HeatCapacity>`), so they share no line here.
#[test]
fn symbols_are_unique_but_for_shared_units() {
    use exact_units::UnitExpr;
    let named: Vec<(&str, &str)> = units::ALL
        .iter()
        .filter_map(|e| match e {
            UnitExpr::Named(def) => Some((def.id.name, def.names.symbol)),
            _ => None,
        })
        .collect();
    let mut shared = Vec::new();
    for (i, (a, sa)) in named.iter().enumerate() {
        for (b, sb) in named.iter().skip(i + 1) {
            if sa == sb {
                shared.push((*a, *b));
            }
        }
    }
    assert_eq!(
        shared,
        [
            #[cfg(feature = "levels")]
            ("Neper", "NeperLoss"),
            #[cfg(feature = "levels")]
            ("Decibel", "DecibelLoss"),
            #[cfg(all(feature = "levels", feature = "information"))]
            ("Bel", "Byte"),
            ("HeatFluxDensity", "Irradiance"),
        ]
    );
}

/// A prefix goes straight before a symbol whose first factor is plain (`mm/s`, `mN·m`), and on the whole of one whose
/// first factor has an exponent or a prefix (`m³/kg`) and none moves inside: `mm³/kg` would be 10⁻⁹ m³/kg, not 10⁻³;
/// `mm²` is 10⁻⁶ m².
#[test]
fn prefixes_on_powered_symbols() {
    use exact_units::Fixed;
    use exact_units::prefix::{Kilo, Micro, Milli};
    use units::{Area, Density, Nit, SpecificVolume, Speed, Torque};

    assert_eq!(Fixed::<Speed, Milli, u8>::new(5).to_string(), "5 mm/s");
    assert_eq!(Fixed::<Torque, Milli, u8>::new(5).to_string(), "5 mN·m");
    assert_eq!(Fixed::<Nit, Milli, u8>::new(5).to_string(), "5 mcd/m²");
    assert_eq!(Fixed::<Area, Milli, u8>::new(5).to_string(), "5 m(m²)");
    assert_eq!(Fixed::<Area, Kilo, u8>::new(5).to_string(), "5 k(m²)");
    assert_eq!(Fixed::<Area, Micro, u8>::new(5).to_string(), "5 mm²");
    assert_eq!(
        Fixed::<SpecificVolume, Micro, u8>::new(5).to_string(),
        "5 cm³/kg"
    );
    assert_eq!(
        Fixed::<SpecificVolume, Kilo, u8>::new(5).to_string(),
        "5 k(m³/kg)"
    );
    assert_eq!(Fixed::<Density, Milli, u8>::new(5).to_string(), "5 mg/L");
}

/// A unit is who declares it (UNIT-7): this test crate's `Rpm` carries the test crate's name, and another crate's
/// `Rpm` with the very same line is another unit, which prints the same.
#[test]
fn identity_is_crate_and_name() {
    use exact_units::{MarkerId, UnitDef, UnitExpr};
    exact_units::unit! {
        /// Revolutions per minute.
        pub struct Rpm = "rpm", [rad 1, s -1], Exact(1 / 30, pi 1);
    }
    let ours = <Rpm as Unit>::EXPR.def().unwrap();
    assert_eq!(
        ours.id,
        MarkerId {
            crate_name: env!("CARGO_CRATE_NAME"),
            name: "Rpm"
        }
    );
    assert_eq!(ours.id.to_string(), "units::Rpm");
    let theirs = UnitDef {
        id: MarkerId {
            crate_name: "my_api",
            name: "Rpm",
        },
        ..*ours
    };
    let theirs: &'static UnitDef<'static> = Box::leak(Box::new(theirs));
    let theirs = UnitExpr::Named(theirs);
    assert_ne!(theirs, *<Rpm as Unit>::EXPR);
    assert_eq!(theirs.to_string(), <Rpm as Unit>::EXPR.to_string());
    assert_eq!(theirs.magnitude(), <Rpm as Unit>::EXPR.magnitude());
    assert_eq!(
        <units::Volt as Unit>::EXPR.def().unwrap().id.to_string(),
        "exact_units::Volt"
    );
    assert_eq!(units::by_id(ours.id), None);
    assert_eq!(
        units::by_id(<units::Volt as Unit>::EXPR.def().unwrap().id),
        Some(<units::Volt as Unit>::EXPR)
    );
}

/// A marker is the same unit wherever it moves in its crate, so this crate has one marker per name: `by_name` and
/// `by_id` find each one.
#[test]
fn one_marker_per_name() {
    use exact_units::UnitExpr;
    let all = units::ALL.iter();
    #[cfg(feature = "customary")]
    let all = all.chain(units::CUSTOMARY);
    let mut names = std::collections::BTreeSet::new();
    for e in all {
        let UnitExpr::Named(def) = e else {
            panic!("{e:?} in the catalogue")
        };
        assert_eq!(def.id.crate_name, "exact_units");
        assert!(names.insert(def.id.name), "{} twice", def.id.name);
        assert_eq!(units::by_id(def.id), Some(*e));
    }
}
