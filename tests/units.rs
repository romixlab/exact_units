//! Marker units of the angle, information and counting groups, and prefix names.
#![cfg(all(feature = "std", feature = "quantities"))]

use exact_units::units::{self, Per};
use exact_units::{Prefix, Unit};
#[cfg(feature = "shrink_wrap")]
use shrink_wrap::prelude::*;

/// rad, bit and counted bases are dimensions of their own (STD-60): bytes never turn into a ratio, an angular
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
            .strip_radian()
            .same(<units::Hertz as Unit>::DIM)
    );
    assert!(
        <units::Torque as Unit>::DIM
            .strip_radian()
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
    assert!(per_sample.mul(<units::Sample as Unit>::DIM).same(byte));
    let rate = <Per<units::Sample, units::Second> as Unit>::DIM;
    assert!(!rate.same(<units::Hertz as Unit>::DIM));
    assert!(
        rate.mul(<units::Second as Unit>::DIM)
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

/// Kinds that share a dimension (UNIT-2): a quotient keeps what cancelled, a product cancels it, and Ω and S are kinds
/// of their own.
#[test]
fn kinds_of_markers() {
    use exact_units::{Fixed, Kind, LevelForm, prefix::One};
    use units::{Ampere, Ohm, Ratio, Siemens, Times, Volt, Watt};

    let volt = <Volt as Unit>::DIM;
    assert_eq!(<Per<Volt, Volt> as Unit>::KIND, Kind::RatioOf(volt));
    assert_eq!(<Per<Volt, Volt> as Unit>::DIM, <Ratio as Unit>::DIM);
    assert!(!<Per<Volt, Volt> as Unit>::KIND.same(<Per<Ampere, Ampere> as Unit>::KIND));
    assert!(!<Per<Volt, Volt> as Unit>::KIND.same(<Ratio as Unit>::KIND));
    assert_eq!(<Ohm as Unit>::KIND, Kind::Named("resistance"));
    assert_eq!(<Siemens as Unit>::KIND, Kind::Named("conductance"));
    assert_eq!(<Per<Volt, Ampere> as Unit>::KIND, Kind::Plain);
    assert_eq!(<Times<Per<Volt, Volt>, Volt> as Unit>::KIND, Kind::Plain);
    assert_eq!(
        <Times<Per<Volt, Volt>, Per<Volt, Volt>> as Unit>::KIND,
        Kind::RatioOf(volt)
    );
    assert_eq!(<Per<Ohm, Ratio> as Unit>::KIND, Kind::Named("resistance"));
    assert_eq!(
        <Per<Watt, Watt> as Unit>::KIND.level_form(),
        Some(LevelForm::Power)
    );
    assert_eq!(
        <Per<Ampere, Ampere> as Unit>::KIND.level_form(),
        Some(LevelForm::RootPower)
    );

    // the named steps keep the number
    let g = Fixed::<Per<Volt, Volt>, One, i16>::new(-3);
    assert_eq!(g.as_ratio().get(), -3);
    assert_eq!(g.as_ratio().as_gain::<Volt>(), g);
    let gm = Fixed::<Per<Ampere, Volt>, One, f32>::new(0.02);
    assert_eq!(gm.as_conductance().as_transconductance(), gm);
    let r = Fixed::<Ohm, One, u8>::new(50);
    assert_eq!(r.as_transimpedance().as_resistance(), r);

    // decibels by kind: 20·lg for V/V and A/A, 10·lg for W/W
    let db = |x: f64, y: f64| (x - y).abs() < 1e-12;
    assert!(db(
        Fixed::<Per<Volt, Volt>, One, f64>::new(2.0).to_db(),
        6.020599913279624
    ));
    assert!(db(
        Fixed::<Per<Watt, Watt>, One, f64>::new(2.0).to_db(),
        3.010299956639812
    ));
    assert!(db(
        Fixed::<Per<Ampere, Ampere>, One, f64>::new(0.5).to_db(),
        -6.020599913279624
    ));
    let back = Fixed::<Per<Watt, Watt>, One, f64>::from_db(-3.0)
        .unwrap()
        .get();
    assert!(db(back, 10f64.powf(-0.3)));
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

/// The derived units are plain but for two that share a named marker's dimension, and the related ones convert
/// exactly.
#[test]
fn derived_kinds_and_conversions() {
    use exact_units::prefix::{Kilo, Micro, Milli, One};
    use exact_units::{Fixed, Kind};
    use units::{
        AbsorbedDoseRate, Acceleration, Area, Candela, Density, EnergyDensity, Entropy,
        GramPerCubicCentimeter, GramPerMole, HeatCapacity, HeatFluxDensity, Irradiance, Joule,
        Kelvin, MolarConcentration, MolarMass, MolePerLitre, Nit, Resistivity, Second,
        SievertPerHour, Speed, ThermalResistance, Volume, Watt,
    };

    // kinds: plain but for the two that share a named marker's dimension
    assert_eq!(<EnergyDensity as Unit>::KIND, Kind::Named("energy density"));
    assert_eq!(<Entropy as Unit>::KIND, Kind::Named("entropy"));
    for kind in [
        <Irradiance as Unit>::KIND,
        <HeatFluxDensity as Unit>::KIND,
        <HeatCapacity as Unit>::KIND,
        <ThermalResistance as Unit>::KIND,
        <Resistivity as Unit>::KIND,
        <Nit as Unit>::KIND,
    ] {
        assert_eq!(kind, Kind::Plain);
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
        Some(Fixed::new(2.5))
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
        "EnergyDensity",
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
        "Entropy",
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
    assert_eq!(units::spellings("Nit"), ["nit", "nt"]);
    assert_eq!(units::spellings("Volt"), [""; 0]);
}

/// No two named units share a symbol but the pairs that are one unit of two quantities (SI Brochure table 6 lists each
/// pair on one line): W/m² of heat flux and irradiance, J/K of heat capacity and entropy, m²·s⁻² of absorbed and
/// equivalent dose differ in their symbols. A loss shares its gain's symbol (dB, Np): their dimensions tell them
/// apart. A damping ratio and a quality factor are numbers like a ratio, told apart by their kinds.
#[test]
fn symbols_are_unique_but_for_shared_units() {
    use exact_units::UnitExpr;
    let named: Vec<(&str, &str)> = units::ALL
        .iter()
        .filter_map(|e| match e {
            UnitExpr::Named { name, symbol, .. } => Some((*name, *symbol)),
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
            #[cfg(feature = "levels")]
            ("DampingRatio", "QualityFactor"),
            #[cfg(all(feature = "levels", feature = "dimensionless"))]
            ("DampingRatio", "Ratio"),
            #[cfg(all(feature = "levels", feature = "dimensionless"))]
            ("QualityFactor", "Ratio"),
            ("HeatFluxDensity", "Irradiance"),
            ("HeatCapacity", "Entropy"),
        ]
    );
}

/// A prefix goes straight before a symbol whose first factor is plain (`mm/s`, `mN·m`), and on the whole of one whose
/// first factor has an exponent or a prefix (`m³/kg`, `kg/m³`) and none moves inside: `mm³/kg` would be 10⁻⁹ m³/kg, not
/// 10⁻³; `mm²` is 10⁻⁶ m².
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
    assert_eq!(
        Fixed::<Density, Milli, u8>::new(5).to_string(),
        "5 m(kg/m³)"
    );
}
