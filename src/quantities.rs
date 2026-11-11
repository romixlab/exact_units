//! This crate's quantities: parts of ISO 80000's tree, the quantity markers of [Of<Q, U>](crate::units::Of)
//! ([quantity](mod@crate::quantity) has the rules). A unit measures its dimension's quantity unless its line names one,
//! so most units need none of these: `Meter` is a [Length], `Joule` an [Energy].
//!
//! - Length (ISO 80000-3): [Length] → [Height], [Width], [Radius], [Distance] → [PathLength].
//! - Energy (ISO 80000-4, -5): [Energy] → [MechanicalEnergy] → [KineticEnergy], [PotentialEnergy]; [Energy] →
//!   [Work], [Heat].
//! - s⁻¹: [Frequency] (`Hertz`), and two kinds of their own, [Activity] (`Becquerel`) and [SymbolRate] (`Baud`):
//!   Hz, Bq and Bd don't convert.
//! - m²·s⁻²: [AbsorbedDose] (`Gray`) with [Kerma], which shares the gray, as a kind under it; [DoseEquivalent]
//!   (`Sievert`, the CIPM's H = Q·D) with the operational [AmbientDoseEquivalent10] H*(10),
//!   [PersonalDoseEquivalent10] Hp(10) and [PersonalDoseEquivalent0_07] Hp(0.07) and ICRP's [EquivalentDose] and
//!   [EffectiveDose], which share the sievert, as kinds under it. None converts or adds to another; a weighting
//!   ([radiation](crate::radiation)) crosses from a gray to a sievert, never `convert`. Their rates, m²·s⁻³, are
//!   kinds in the same shape ([AbsorbedDoseRate], [KermaRate], [DoseEquivalentRate], [EquivalentDoseRate],
//!   [EffectiveDoseRate]).
//! - [LinearEnergyTransfer], keV/µm: not a force.
//! - [Exposure], C/kg: ionizing radiation's charge per mass of air, not a specific charge.
//! - Electrical: [Resistance] (`Ohm`) and [Conductance] (`Siemens`), apart from V/A and A/V; [Power] with
//!   [ApparentPower] (`VoltAmpere`) and [ReactivePower] (`Var`).
//! - Electromagnetic, the SI side of Gaussian's rows ([systems](crate::systems)): [ElectricCurrent],
//!   [ElectricCharge], [Voltage], [ElectricField], [MagneticFlux], [MagneticFluxDensity], [Capacitance],
//!   [Inductance] as open roots; at C/m² and A/m, whose quantities Gaussian counts 4π apart, no open root:
//!   [ElectricFluxDensity] (D) with [ElectricPolarization] under it, [SurfaceChargeDensity]; [MagnetizingField] (H)
//!   with [Magnetization] under it, [LinearCurrentDensity]. [MagnetomotiveForce] apart from a current.
//! - [Pressure] and [EnergyDensity] (one dimension, two quantities); [Entropy], apart from J/K's own quantity.
//! - Dimension one: [DampingRatio] and [QualityFactor].
//! - Angles: [PlaneAngle] and [PhaseAngle], the angle of a phasor or of a motor's electrical rotation.
//! - The open roots of the base units and of common derived ones, so that a unit's quantity has a name where its line
//!   names none: [Time], [Mass], [ThermodynamicTemperature], [AmountOfSubstance], [LuminousIntensity], [Force],
//!   [CatalyticActivity], [SolidAngle], [LuminousFlux], [Illuminance], [Luminance], [Information]. No `Area` or
//!   `Volume`: the units [Area](crate::units::Area) and [Volume](crate::units::Volume) have those names.
//!
//! [ALL] lists them, the table [units::TABLE](crate::units::TABLE) resolves them by id.

pub use crate::quantity::Common;

crate::quantity! {
    @table
    /// Every quantity of this crate.
    pub static ALL;

    /// Length, l (ISO 80000-3, 3-1.1): the open root of m.
    pub struct Length = [m 1];
    /// Height, h: a length measured upwards.
    pub struct Height: Length;
    /// Width, b: a length across.
    pub struct Width: Length;
    /// Radius, r: a length from a centre.
    pub struct Radius: Length;
    /// Distance, d: the length between two points.
    pub struct Distance: Length;
    /// Path length, s: a distance along a path.
    pub struct PathLength: Distance;

    /// Energy, E (ISO 80000-5): the open root of J.
    pub struct Energy = [kg 1, m 2, s -2];
    /// Mechanical energy: kinetic and potential.
    pub struct MechanicalEnergy: Energy;
    /// Kinetic energy, ½·m·v².
    pub struct KineticEnergy: MechanicalEnergy;
    /// Potential energy, m·g·h.
    pub struct PotentialEnergy: MechanicalEnergy;
    /// Work, W: energy moved by a force along a path.
    pub struct Work: Energy;
    /// Heat, Q: energy moved by a difference of temperature.
    pub struct Heat: Energy;

    /// Frequency, f (ISO 80000-3): the open root of s⁻¹, [Hertz](crate::units::Hertz)'s.
    pub struct Frequency = [s -1];
    /// Activity, A (ISO 80000-10): decays per second, [Becquerel](crate::units::Becquerel)'s; not a frequency.
    pub struct Activity = kind [s -1];
    /// Symbol rate: symbols per second, [Baud](crate::units::Baud)'s; not a clock's frequency.
    pub struct SymbolRate = kind [s -1];

    /// Absorbed dose, D (ISO 80000-10): energy imparted per mass, [Gray](crate::units::Gray)'s.
    pub struct AbsorbedDose = kind [m 2, s -2];
    /// Kerma, K (ISO 80000-10): kinetic energy released per mass, energy transferred to charged particles. The SI
    /// gives it the gray as absorbed dose has (SI Brochure, 9th ed., table 4), but it is not an absorbed dose:
    /// `Of<Kerma, Gray>`, a kind under [AbsorbedDose] so that the gray measures it. Under charged-particle
    /// equilibrium a kerma is taken as an absorbed dose by the named step `as_quantity::<AbsorbedDose>()`.
    pub struct Kerma: kind AbsorbedDose;
    /// Dose equivalent, H = Q·D at a point, Q the ICRU's quality factor of linear energy transfer (CIPM 2002,
    /// Recommendation 2; SI Brochure, 9th ed., table 4): [Sievert](crate::units::Sievert)'s. Not ICRP's
    /// [EquivalentDose] or [EffectiveDose], which share the sievert as kinds under it: none of them converts or adds
    /// to another, `as_quantity` through this one is the only crossing.
    pub struct DoseEquivalent = kind [m 2, s -2];
    /// Ambient dose equivalent, H*(10) (ICRU Report 51; ICRP Publication 103, Table B.5): the dose equivalent at 10 mm
    /// depth in the ICRU sphere in an expanded and aligned field, what an area monitor reads. A kind under
    /// [DoseEquivalent]: `as_quantity` crosses, and it neither converts nor adds to Hp(10).
    pub struct AmbientDoseEquivalent10: kind DoseEquivalent;
    /// Personal dose equivalent, Hp(10) (ICRU Report 51; ICRP Publication 103, Table B.5): the dose equivalent in soft
    /// tissue 10 mm below a point on the body, what a personal dosimeter reads for the effective dose.
    pub struct PersonalDoseEquivalent10: kind DoseEquivalent;
    /// Personal dose equivalent, Hp(0.07) (ICRU Report 51; ICRP Publication 103, Table B.5): at 0.07 mm depth, for the
    /// dose to the skin, hands and feet.
    pub struct PersonalDoseEquivalent0_07: kind DoseEquivalent;
    /// Equivalent dose, H_T = Σ w_R·D_T,R (ICRP Publication 103, Eq. 4.2): the mean absorbed dose in a tissue or
    /// organ, weighted by radiation weighting factors. Not a [DoseEquivalent], which is a point value weighted by
    /// Q, but a kind under it, so the sievert measures it: `Of<EquivalentDose, Sievert>`.
    pub struct EquivalentDose: kind DoseEquivalent;
    /// Effective dose, E = Σ w_T·H_T (ICRP Publication 103, glossary): the tissue-weighted sum of equivalent doses
    /// over the whole body of a Reference Person, a protection quantity: `Of<EffectiveDose, Sievert>`.
    pub struct EffectiveDose: kind DoseEquivalent;

    /// Absorbed dose rate, Ḋ: [GrayPerSecond](crate::units::GrayPerSecond)'s. Each dose has a rate of its own
    /// kind, so Gy/s doesn't convert to Sv/h.
    pub struct AbsorbedDoseRate = kind [m 2, s -3];
    /// Kerma rate, K̇: an air kerma rate in `Of<KermaRate, GrayPerSecond>`.
    pub struct KermaRate: kind AbsorbedDoseRate;
    /// Dose equivalent rate, Ḣ: [SievertPerHour](crate::units::SievertPerHour)'s.
    pub struct DoseEquivalentRate = kind [m 2, s -3];
    /// Ambient dose equivalent rate, Ḣ*(10): a survey meter's or a Geiger counter's µSv/h,
    /// `Of<AmbientDoseEquivalentRate10, SievertPerHour>`.
    pub struct AmbientDoseEquivalentRate10: kind DoseEquivalentRate;
    /// Personal dose equivalent rate, Ḣp(10): an electronic personal dosimeter's rate.
    pub struct PersonalDoseEquivalentRate10: kind DoseEquivalentRate;
    /// Personal dose equivalent rate, Ḣp(0.07).
    pub struct PersonalDoseEquivalentRate0_07: kind DoseEquivalentRate;
    /// Equivalent dose rate, Ḣ_T.
    pub struct EquivalentDoseRate: kind DoseEquivalentRate;
    /// Effective dose rate, Ė.
    pub struct EffectiveDoseRate: kind DoseEquivalentRate;

    /// Linear energy transfer, L (ISO 80000-10): energy a charged particle loses per length of its track, in keV/µm.
    /// A kind of its own, not a force, though both are J/m.
    pub struct LinearEnergyTransfer = kind [kg 1, m 1, s -2];

    /// Exposure, X (ISO 80000-10): ionizing radiation's charge liberated per mass of dry air,
    /// [CoulombPerKilogram](crate::units::CoulombPerKilogram)'s and the röntgen's. A kind of its own, not a
    /// specific charge, though both are C/kg.
    pub struct Exposure = kind [kg -1, s 1, a 1];

    /// Resistance, R (ISO 80000-6): a component's V/A, not an amplifier's transimpedance.
    pub struct Resistance = kind [kg 1, m 2, s -3, a -2];
    /// Conductance, G: a component's A/V, not an amplifier's transconductance.
    pub struct Conductance = kind [kg -1, m -2, s 3, a 2];
    /// Power, P: the open root of W.
    pub struct Power = [kg 1, m 2, s -3];
    /// Apparent power, |S|: the magnitude of the complex power, [VoltAmpere](crate::units::VoltAmpere)'s.
    pub struct ApparentPower = kind [kg 1, m 2, s -3];
    /// Reactive power, Q: the imaginary part of the complex power, [Var](crate::units::Var)'s.
    pub struct ReactivePower = kind [kg 1, m 2, s -3];

    /// Pressure, p: the open root of Pa.
    pub struct Pressure = [kg 1, m -1, s -2];
    /// Energy density, w: energy per volume, not a pressure.
    pub struct EnergyDensity = kind [kg 1, m -1, s -2];
    /// Entropy, S: not a heat capacity, though both are J/K ([HeatCapacity](crate::units::HeatCapacity) measures
    /// the dimension's own quantity, a heat capacity).
    pub struct Entropy = kind [kg 1, m 2, s -2, k -1];

    /// Damping ratio, ζ, of a second-order system: a number of its own, not a plain ratio.
    pub struct DampingRatio = kind [];
    /// Quality factor, Q = 1/(2ζ): a number of its own.
    pub struct QualityFactor = kind [];

    /// Electric current, I (ISO 80000-6): the open root of A.
    pub struct ElectricCurrent = [a 1];
    /// Magnetomotive force, F_m: ampere-turns around a magnetic circuit, not a current (Gaussian's gilbert is
    /// another unit than its statampere).
    pub struct MagnetomotiveForce = kind [a 1];
    /// Electric charge, Q: the open root of C.
    pub struct ElectricCharge = [s 1, a 1];
    /// Voltage, U: a potential difference or an electromotive force, the open root of V.
    pub struct Voltage = [kg 1, m 2, s -3, a -1];
    /// Electric field strength, E: the open root of V/m.
    pub struct ElectricField = [kg 1, m 1, s -3, a -1];
    /// Electric flux density, D (the displacement field): a kind of its own, as C/m² is also a polarization and a
    /// surface charge density, which Gaussian units count 4π apart.
    pub struct ElectricFluxDensity = kind [s 1, a 1, m -2];
    /// Electric polarization, P: a kind of its own under D (D = ε₀E + P, after `as_quantity`).
    pub struct ElectricPolarization: kind ElectricFluxDensity;
    /// Surface density of charge, σ: a kind of its own, not a flux density.
    pub struct SurfaceChargeDensity = kind [s 1, a 1, m -2];
    /// Magnetic flux, Φ: the open root of Wb.
    pub struct MagneticFlux = [kg 1, m 2, s -2, a -1];
    /// Magnetic flux density, B: the open root of T.
    pub struct MagneticFluxDensity = [kg 1, s -2, a -1];
    /// Magnetic field strength, H (the magnetizing field): a kind of its own, as A/m is also a magnetization and a
    /// linear current density, which Gaussian units count 4π apart.
    pub struct MagnetizingField = kind [a 1, m -1];
    /// Magnetization, M: a kind of its own under H (B = μ₀(H + M), after `as_quantity`).
    pub struct Magnetization: kind MagnetizingField;
    /// Linear current density, A_s: current per width of a sheet, a kind of its own.
    pub struct LinearCurrentDensity = kind [a 1, m -1];
    /// Capacitance, C: the open root of F.
    pub struct Capacitance = [kg -1, m -2, s 4, a 2];
    /// Inductance, L: the open root of H.
    pub struct Inductance = [kg 1, m 2, s -2, a -2];

    /// Plane angle, the open root of rad.
    pub struct PlaneAngle = [rad 1];
    /// Phase angle, φ: the argument of a phasor, an NCO's phase, a motor's electrical angle. Not a mechanical
    /// angle: the named steps `as_phase` and `as_plane_angle` cross.
    pub struct PhaseAngle: kind PlaneAngle;

    /// Time, t (ISO 80000-3): the open root of s, [Second](crate::units::Second)'s and the hour's.
    pub struct Time = [s 1];
    /// Mass, m (ISO 80000-4): the open root of kg.
    pub struct Mass = [kg 1];
    /// Thermodynamic temperature, T (ISO 80000-5): the open root of K.
    pub struct ThermodynamicTemperature = [k 1];
    /// Amount of substance, n (ISO 80000-9): the open root of mol.
    pub struct AmountOfSubstance = [mol 1];
    /// Luminous intensity, I_v (ISO 80000-7): the open root of cd.
    pub struct LuminousIntensity = [cd 1];
    /// Force, F (ISO 80000-4): the open root of N.
    pub struct Force = [kg 1, m 1, s -2];
    /// Catalytic activity, z (ISO 80000-9): the open root of kat.
    pub struct CatalyticActivity = [mol 1, s -1];
    /// Solid angle, Ω (ISO 80000-3): the open root of sr.
    pub struct SolidAngle = [rad 2];
    /// Luminous flux, Φ_v (ISO 80000-7): the open root of lm, cd·sr.
    pub struct LuminousFlux = [cd 1, rad 2];
    /// Illuminance, E_v (ISO 80000-7): the open root of lx, lm/m².
    pub struct Illuminance = [cd 1, rad 2, m -2];
    /// Luminance, L_v (ISO 80000-7): the open root of cd/m², the nit's.
    pub struct Luminance = [cd 1, m -2];
    /// Information, in bits (ISO 80000-13): the open root of bit, the byte's and a word's.
    pub struct Information = [bit 1];
}
