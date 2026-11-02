//! Unit markers: zero-size types that name a unit in a type (`Fixed<units::Volt, Milli, u16>`), never sent themselves.
//! `ww_si` has an alias of the run-time quantity of each under the same name (`ww_si::Volt`).
//!
//! Compound units outside the list are [Per] and [Times] of markers: `Fixed<Per<Torque, AngularVelocity>, One, f32>` is in
//! N·m/(rad/s). [by_name] and [ALL] give the [UnitExpr] of every named marker of this crate, for dynamic clients that
//! see a unit only by its name in introspection data.
//!
//! The markers are declared with [unit!](crate::unit), one line each with its symbol, dimension and scale: the
//! compiler gets their [Unit] impls from it, and the snapshot of this crate (WireWeaver's walker reads the same
//! lines) their docs and definitions, so clients know them without a table (INT-13).

#[cfg(feature = "shrink_wrap")]
use crate::SIExpr;
use crate::unit::{Dim, Unit, UnitExpr};
use crate::{Kind, MAX_UNIT_DEPTH};
use core::marker::PhantomData;
#[cfg(feature = "shrink_wrap")]
use shrink_wrap::RefBox;

#[cfg(feature = "customary")]
mod customary;
#[cfg(feature = "customary")]
pub use customary::*;

crate::unit! {
    @table
    /// Every named unit of this crate that is enabled, in the order of the docs.
    pub static ALL;

    // Time

    /// Time in seconds, s.
    ///
    /// As a device timestamp it is the device's time since its clock started, see `ww_si::Second`.
    #[cfg(feature = "time")]
    pub struct Second = "s", [s 1];

    /// Time in minutes, min (60 [Second]), a unit accepted for use with the SI (SI Brochure table 8).
    #[cfg(feature = "time")]
    pub struct Minute = "min", [s 1], Exact(60 / 1);

    /// Time in hours, h (3600 [Second]), a unit accepted for use with the SI (SI Brochure table 8).
    #[cfg(feature = "time")]
    pub struct Hour = "h", [s 1], Exact(3600 / 1);

    /// Time in days, d (86400 [Second]), a unit accepted for use with the SI (SI Brochure table 8): always 86400 s,
    /// also across a leap second, so a UTC calendar day with one is 1 d + 1 s.
    #[cfg(feature = "time")]
    pub struct Day = "d", [s 1], Exact(86400 / 1);

    /// Time in weeks, wk (7 [Day], 604800 [Second]): a retention or a report period. Not in the SI Brochure and without a
    /// standard symbol: `wk` is the common one.
    #[cfg(feature = "time")]
    pub struct Week = "wk", [s 1], Exact(604800 / 1);

    /// Frequency in hertz, Hz (s⁻¹).
    #[cfg(feature = "time")]
    pub struct Hertz = "Hz", [s -1];

    /// Frequency drift in hertz per second, Hz/s (s⁻²).
    #[cfg(feature = "time")]
    pub struct HertzPerSecond = "Hz/s", [s -2];

    /// Symbol rate in baud, Bd (s⁻¹): one symbol per second, for a UART one bit per second on the line. A frequency of
    /// its own kind, so it is not mixed up with a clock in [Hertz].
    #[cfg(feature = "time")]
    pub struct Baud = "Bd", [s -1];

    // Electrical

    /// Electric current in amperes, A.
    #[cfg(feature = "electrical")]
    pub struct Ampere = "A", [a 1];

    /// Voltage, electric potential or electromotive force in volts, V (kg·m²·s⁻³·A⁻¹, W/A).
    #[cfg(feature = "electrical")]
    pub struct Volt = "V", [kg 1, m 2, s -3, a -1];

    /// Resistance, reactance or impedance in ohms, Ω (kg·m²·s⁻³·A⁻², V/A).
    /// Of the kind `resistance`: a transimpedance `Per<Volt, Ampere>` becomes one only by `as_resistance`.
    #[cfg(feature = "electrical")]
    pub struct Ohm = "Ω", [kg 1, m 2, s -3, a -2] kind "resistance";

    /// Capacitance in farads, F (kg⁻¹·m⁻²·s⁴·A², C/V).
    #[cfg(feature = "electrical")]
    pub struct Farad = "F", [kg -1, m -2, s 4, a 2];

    /// Admittance, conductance or susceptance in siemens, S (kg⁻¹·m⁻²·s³·A², A/V).
    /// Of the kind `conductance`: a transconductance `Per<Ampere, Volt>` becomes one only by `as_conductance`.
    #[cfg(feature = "electrical")]
    pub struct Siemens = "S", [kg -1, m -2, s 3, a 2] kind "conductance";

    /// Inductance in henries, H (kg·m²·s⁻²·A⁻², Wb/A).
    #[cfg(feature = "electrical")]
    pub struct Henry = "H", [kg 1, m 2, s -2, a -2];

    /// Electric charge in coulombs, C (s·A).
    #[cfg(feature = "electrical")]
    pub struct Coulomb = "C", [s 1, a 1];

    /// Power in watts, W (kg·m²·s⁻³, J/s, V·A): active power, P, of ISO 80000-6 (stays plain, next to the kinds
    /// below).
    #[cfg(feature = "electrical")]
    pub struct Watt = "W", [kg 1, m 2, s -3];

    /// Apparent power in volt-amperes, VA (kg·m²·s⁻³, like [Watt]): |S|, ISO 80000-6's S, of the complex power
    /// S = V·I*. Of the kind `apparent power`: a product `Times<Volt, Ampere>` becomes one only through
    /// [Complex::apparent_power](crate::Complex::apparent_power), never a plain `convert`.
    #[cfg(feature = "electrical")]
    pub struct VoltAmpere = "VA", [kg 1, m 2, s -3] kind "apparent power";

    /// Reactive power in volt-amperes reactive, var (kg·m²·s⁻³, like [Watt]): Im(S), ISO 80000-6's Q, of the
    /// complex power S = V·I*. Of the kind `reactive power`: never a plain [Watt] by `convert`.
    #[cfg(feature = "electrical")]
    pub struct Var = "var", [kg 1, m 2, s -3] kind "reactive power";

    /// Magnetic flux in webers, Wb (kg·m²·s⁻²·A⁻¹, V·s).
    #[cfg(feature = "electrical")]
    pub struct Weber = "Wb", [kg 1, m 2, s -2, a -1];

    /// Magnetic flux density in teslas, T (kg·s⁻²·A⁻¹, Wb/m²).
    #[cfg(feature = "electrical")]
    pub struct Tesla = "T", [kg 1, s -2, a -1];

    // Fields: electric and magnetic fields and the material constants between them

    /// Electric field strength in volts per meter, V/m (kg·m·s⁻³·A⁻¹, SI Brochure, 9th ed., table 6).
    #[cfg(feature = "fields")]
    pub struct ElectricFieldStrength = "V/m", [kg 1, m 1, s -3, a -1];

    /// Magnetic field strength in amperes per meter, A/m (SI Brochure, 9th ed., table 5).
    #[cfg(feature = "fields")]
    pub struct MagneticFieldStrength = "A/m", [m -1, a 1];

    /// Current density in amperes per square meter, A/m² (SI Brochure, 9th ed., table 5): a trace's or a wire's A/mm²
    /// is `Fixed<CurrentDensity, Mega, T>`.
    #[cfg(feature = "fields")]
    pub struct CurrentDensity = "A/m²", [m -2, a 1];

    /// Electric charge density in coulombs per cubic meter, C/m³ (m⁻³·s·A, SI Brochure, 9th ed., table 6).
    #[cfg(feature = "fields")]
    pub struct ChargeDensity = "C/m³", [m -3, s 1, a 1];

    /// Electric displacement (electric flux density) or surface charge density in coulombs per square meter, C/m²
    /// (m⁻²·s·A, SI Brochure, 9th ed., table 6).
    #[cfg(feature = "fields")]
    pub struct ElectricDisplacement = "C/m²", [m -2, s 1, a 1];

    /// Permittivity in farads per meter, F/m (kg⁻¹·m⁻³·s⁴·A², SI Brochure, 9th ed., table 6); also a cable's
    /// capacitance per length.
    #[cfg(feature = "fields")]
    pub struct Permittivity = "F/m", [kg -1, m -3, s 4, a 2];

    /// Permeability in henries per meter, H/m (kg·m·s⁻²·A⁻², SI Brochure, 9th ed., table 6); also a cable's
    /// inductance per length.
    #[cfg(feature = "fields")]
    pub struct Permeability = "H/m", [kg 1, m 1, s -2, a -2];

    /// Electrical resistivity in ohm meters, Ω·m (kg·m³·s⁻³·A⁻²): copper is about 17 nΩ·m.
    #[cfg(feature = "fields")]
    pub struct Resistivity = "Ω·m", [kg 1, m 3, s -3, a -2];

    /// Electrical conductivity in siemens per meter, S/m (kg⁻¹·m⁻³·s³·A²): seawater is about 5 S/m.
    #[cfg(feature = "fields")]
    pub struct Conductivity = "S/m", [kg -1, m -3, s 3, a 2];

    // Mechanical

    /// Length in meters, m.
    #[cfg(feature = "mechanical")]
    pub struct Meter = "m", [m 1];

    /// Length in kilometers, km: a fibre's own unit, so `Per<Loss<Decibel>, Kilometre>` reads as it prints,
    /// "dB/km" (0.2 dB/km at 1550 nm is typical). A unit of its own, not `Meter` in a prefixed scale: `{}` of a
    /// `Per<Loss<Decibel>, Meter>` in `Milli` writes `dB/km` too, but `{:#}` and the `with_unit` text write a value in
    /// a unit's bare symbol ([write_unit](crate::tree::write_unit) with [Prefix::Unit](crate::Prefix::Unit)), so
    /// "dB/km" there needs its own marker, as [Litre] already does for volume.
    #[cfg(feature = "mechanical")]
    pub struct Kilometre = "km", [m 1], Exact(1000 / 1);

    /// Length in hundreds of meters, 100 m: a cable datasheet's own unit, so `Per<Loss<Decibel>, Metres100>` reads
    /// as it prints, "dB/100 m".
    #[cfg(feature = "mechanical")]
    pub struct Metres100 = "100 m", [m 1], Exact(100 / 1);

    /// Length in hundreds of feet (international foot), 100 ft = 30.48 m: a cable datasheet's own unit, so
    /// `Per<Loss<Decibel>, Feet100>` reads as it prints, "dB/100 ft". A crate convenience scaled on
    /// [FootInternational], not a unit of its own in NIST SP 811, so it stays out of the sourced customary
    /// catalogue (`customary::sourced`).
    #[cfg(all(feature = "mechanical", feature = "customary"))]
    pub struct Feet100 = "100 ft", [m 1], Exact(381 / 1, pow10 -2, pow2 3);

    /// Length times frequency in centimetre-megahertz, cm·MHz (10⁴ m·Hz, the dimension of a speed): medical
    /// ultrasound's own unit for frequency-scaled attenuation, α(f) = α₀·f, so `Per<Loss<Decibel>,
    /// CentimetreMegahertz>` prints as a datasheet writes it, "dB/(cm·MHz)" (soft tissue ≈ 0.5), the middle dot
    /// parenthesizing the denominator as a `rad/s` one already does. A marker of its own, not `Times<Meter, Hertz>`
    /// in a prefixed scale, for the same reason as [Metres100] and [Kilometre].
    #[cfg(feature = "mechanical")]
    pub struct CentimetreMegahertz = "cm·MHz", [m 1, s -1], Exact(10000 / 1);

    /// Mass in grams, g (10⁻³ kg): 5 kg is `Gram::kilo(5u8)`.
    #[cfg(feature = "mechanical")]
    pub struct Gram = "g", [kg 1], Pow10(-3);

    /// Force or weight in newtons, N (kg·m·s⁻²).
    #[cfg(feature = "mechanical")]
    pub struct Newton = "N", [kg 1, m 1, s -2];

    /// Pressure or stress in pascals, Pa (kg·m⁻¹·s⁻², N/m²).
    #[cfg(feature = "mechanical")]
    pub struct Pascal = "Pa", [kg 1, m -1, s -2];

    /// Energy, work or heat in joules, J (kg·m²·s⁻², N·m, W·s).
    #[cfg(feature = "mechanical")]
    pub struct Joule = "J", [kg 1, m 2, s -2];

    /// Torque or moment of force in newton-meters, N·m, counted as joules per radian (kg·m²·s⁻²·rad⁻¹): not the
    /// dimension of a [Joule], and a torque times an [AngularVelocity] is a [Watt].
    #[cfg(feature = "mechanical")]
    pub struct Torque = "N·m", [kg 1, m 2, s -2, rad -1];

    /// Volume in litres, L (10⁻³ m³).
    #[cfg(feature = "mechanical")]
    pub struct Litre = "L", [m 3], Pow10(-3);

    /// Volumetric flow in cubic meters per second, m³/s. Written in litres from 10⁻³ m³/s down, like a [Volume]:
    /// `5 L/s`, `5 mL/s`, `5 µL/s`.
    #[cfg(feature = "mechanical")]
    pub struct VolumetricFlow = "m³/s", [m 3, s -1];

    /// Speed in meters per second, m/s.
    #[cfg(feature = "mechanical")]
    pub struct Speed = "m/s", [m 1, s -1];

    /// Acceleration in meters per second squared, m/s².
    #[cfg(feature = "mechanical")]
    pub struct Acceleration = "m/s²", [m 1, s -2];

    /// Jerk in meters per second cubed, m/s³.
    #[cfg(feature = "mechanical")]
    pub struct Jerk = "m/s³", [m 1, s -3];

    /// Snap or jounce in meters per second to the fourth, m/s⁴.
    #[cfg(feature = "mechanical")]
    pub struct Snap = "m/s⁴", [m 1, s -4];

    /// Yank, the rate of change of force, in newtons per second, N/s (kg·m·s⁻³).
    #[cfg(feature = "mechanical")]
    pub struct Yank = "N/s", [kg 1, m 1, s -3];

    /// Area in square meters, m² (SI Brochure, 9th ed., table 5). A prefix binds to the metre and the power applies
    /// to both: a `Fixed<Area, Micro, T>` is written `5 mm²`, and a `Fixed<Area, Milli, T>`, which no prefix on the
    /// metre makes, `5 m(m²)`.
    #[cfg(feature = "mechanical")]
    pub struct Area = "m²", [m 2];

    /// Volume in cubic meters, m³ (SI Brochure, 9th ed., table 5); a litre is [Litre], 10⁻³ m³. Written in litres from
    /// 10⁻³ m³ down to 10⁻⁹ m³: a `Fixed<Volume, Milli, T>` is `5 L`, in `Micro` `5 mL`, in `Nano` `5 µL`.
    #[cfg(feature = "mechanical")]
    pub struct Volume = "m³", [m 3];

    /// Wavenumber in reciprocal meters, m⁻¹ (SI Brochure, 9th ed., table 5): the cm⁻¹ of spectroscopy is
    /// `Fixed<Wavenumber, Hecto, T>`, 100 m⁻¹.
    #[cfg(feature = "mechanical")]
    pub struct Wavenumber = "m⁻¹", [m -1];

    /// Density, mass density or mass concentration in kilograms per cubic meter, kg/m³ (SI Brochure, 9th ed.,
    /// table 5); g/L is the same unit. A scale is written per litre from 10⁻¹² to 10³, `5 mg/L` for `Milli`; for
    /// `µg/m³` spell the unit `Per<Gram, Volume>`.
    #[cfg(feature = "mechanical")]
    pub struct Density = "kg/m³", [kg 1, m -3];

    /// Density in grams per cubic centimetre, g/cm³ (10³ kg/m³, also kg/L): water is about 1 g/cm³.
    #[cfg(feature = "mechanical")]
    pub struct GramPerCubicCentimeter = "g/cm³", [kg 1, m -3], Pow10(3);

    /// Surface density, mass per area, in kilograms per square meter, kg/m² (SI Brochure, 9th ed., table 5): a paper's
    /// grammage in g/m² is `Fixed<SurfaceDensity, Milli, T>`.
    #[cfg(feature = "mechanical")]
    pub struct SurfaceDensity = "kg/m²", [kg 1, m -2];

    /// Specific volume in cubic meters per kilogram, m³/kg (SI Brochure, 9th ed., table 5).
    #[cfg(feature = "mechanical")]
    pub struct SpecificVolume = "m³/kg", [m 3, kg -1];

    /// Mass flow rate in kilograms per second, kg/s.
    #[cfg(feature = "mechanical")]
    pub struct MassFlow = "kg/s", [kg 1, s -1];

    /// Momentum or impulse in kilogram meters per second, kg·m/s (N·s).
    #[cfg(feature = "mechanical")]
    pub struct Momentum = "kg·m/s", [kg 1, m 1, s -1];

    /// Angular momentum in kilogram square meters per second, kg·m²/s, counted as joule seconds per radian
    /// (kg·m²·s⁻¹·rad⁻¹), like [Torque]: a [MomentOfInertia] times an [AngularVelocity], and a [Torque] times a
    /// [Second].
    #[cfg(feature = "mechanical")]
    pub struct AngularMomentum = "kg·m²/s", [kg 1, m 2, s -1, rad -1];

    /// Moment of inertia in kilogram square meters, kg·m², counted per square radian (kg·m²·rad⁻²): times an
    /// [AngularAcceleration] it is a [Torque], and ½·I·ω² is a [Joule].
    #[cfg(feature = "mechanical")]
    pub struct MomentOfInertia = "kg·m²", [kg 1, m 2, rad -2];

    /// Surface tension in newtons per meter, N/m (kg·s⁻², SI Brochure, 9th ed., table 6); also a spring's stiffness.
    #[cfg(feature = "mechanical")]
    pub struct SurfaceTension = "N/m", [kg 1, s -2];

    /// Dynamic viscosity in pascal seconds, Pa·s (kg·m⁻¹·s⁻¹, SI Brochure, 9th ed., table 6): water at 20 °C is about
    /// 1 mPa·s.
    #[cfg(feature = "mechanical")]
    pub struct DynamicViscosity = "Pa·s", [kg 1, m -1, s -1];

    /// Kinematic viscosity, or a diffusivity, in square meters per second, m²/s: a dynamic viscosity over a density.
    /// The centistokes of oils is `Fixed<KinematicViscosity, Micro, T>`, 1 mm²/s.
    #[cfg(feature = "mechanical")]
    pub struct KinematicViscosity = "m²/s", [m 2, s -1];

    /// Energy density in joules per cubic meter, J/m³ (kg·m⁻¹·s⁻², SI Brochure, 9th ed., table 6). The dimension of a
    /// [Pascal], so of the kind `energy density`: a pressure is not one, and `Per<Joule, Volume>` becomes one only by
    /// `as_energy_density`.
    #[cfg(feature = "mechanical")]
    pub struct EnergyDensity = "J/m³", [kg 1, m -1, s -2] kind "energy density";

    /// Power density, or a volumetric heat source, in watts per cubic meter, W/m³ (kg·m⁻¹·s⁻³).
    #[cfg(feature = "mechanical")]
    pub struct PowerDensity = "W/m³", [kg 1, m -1, s -3];

    // Angle

    /// Angle in radians, rad: a base dimension of its own here, so an angle doesn't mix with a [Ratio].
    #[cfg(feature = "angle")]
    pub struct Radian = "rad", [rad 1];

    /// Angle in degrees of arc, ° (π/180 rad, exactly: [Scale::Exact](crate::Scale::Exact)).
    #[cfg(feature = "angle")]
    pub struct Degree = "°", [rad 1], Exact(1 / 180, pi 1);

    /// Angle in turns (revolutions), rev (2π rad, exactly). A binary angle where 65536 is a full turn is
    /// `Fixed<Turn, Rational<1, 65536>, u16>`.
    #[cfg(feature = "angle")]
    pub struct Turn = "rev", [rad 1], Exact(2 / 1, pi 1);

    /// Solid angle in steradians, sr (rad²).
    #[cfg(feature = "angle")]
    pub struct Steradian = "sr", [rad 2];

    /// Angular velocity in radians per second, rad/s (rad·s⁻¹): not a [Hertz], a turn is 2π rad.
    #[cfg(feature = "angle")]
    pub struct AngularVelocity = "rad/s", [rad 1, s -1];

    /// Angular acceleration in radians per second squared, rad/s² (rad·s⁻²).
    #[cfg(feature = "angle")]
    pub struct AngularAcceleration = "rad/s²", [rad 1, s -2];

    // Levels: the neper is a base of its own (a counted base of this crate), like the radian, so a level doesn't mix
    // with a Ratio and a rate of one (Np/m, dB/s) not with a wavenumber or a hertz. A loss is a second base: a loss and
    // a gain of the same number are each other's negation, crossed by the named steps `as_loss` and `as_gain`.

    /// A level in nepers, Np: ln(F/F₀) of a root-power quantity, ½·ln(P/P₀) of a power, so a field level and its
    /// power level are the same number (ISO 80000-3, IEC 60027-3). Signed as a gain: a decay is negative. A base of
    /// its own here: [Fixed::to_level](crate::Fixed::to_level) and [to_gain](crate::Fixed::to_gain) cross from and to
    /// a gain (V/V, W/W).
    #[cfg(feature = "levels")]
    pub struct Neper = "Np", counted;

    /// A level in decibels, dB ((ln 10)/20 [Neper], exactly): 20·lg of a root-power ratio, 10·lg of a power ratio,
    /// the same number for both. Signed as a gain.
    #[cfg(feature = "levels")]
    pub struct Decibel = "dB", counted(Neper), Exact(1 / 20, ln10 1);

    /// A loss in nepers, Np: a [Neper] level signed as a loss, positive for a decay, as attenuation coefficients and
    /// datasheets write it. A base of its own, so a loss doesn't convert to a gain: `as_gain` negates. Written
    /// `Loss<Neper>` ([Loss]).
    #[cfg(feature = "levels")]
    pub struct NeperLoss = "Np", counted;

    /// A loss in decibels, dB ((ln 10)/20 [NeperLoss], exactly): a cable's 17.7 dB/100 m is
    /// `Per<Loss<Decibel>, _>` of +17.7. Written `Loss<Decibel>` ([Loss]).
    #[cfg(feature = "levels")]
    pub struct DecibelLoss = "dB", counted(NeperLoss), Exact(1 / 20, ln10 1);

    /// Absorbance A = −lg T of a transmitted power ratio T, AU ((ln 10)/2 [NeperLoss], exactly): the loss in bels of
    /// power, so 1 AU is 10 dB. Decadic, and positive for a decay as a [DecibelLoss] is; a lab's "OD". `Fixed<Absorbance,
    /// Milli, _>` prints mAU. [to_gain](crate::Fixed::to_gain) into `Per<Watt, Watt>` is T.
    #[cfg(feature = "levels")]
    pub struct Absorbance = "AU", counted(NeperLoss), Exact(1 / 2, ln10 1);

    /// Optical depth τ = −ln T of a transmitted power ratio T, τ (½ [NeperLoss], exactly): power falls as e^(−τ), its
    /// field as e^(−τ/2), so 1 Np of loss is 2 of depth. τ = ln 10 · A of an [Absorbance], a conversion by name for
    /// integers.
    #[cfg(feature = "levels")]
    pub struct OpticalDepth = "τ", counted(NeperLoss), Exact(1 / 2);

    /// A damping ratio, ζ: δ/ω₀ of a second-order system (x″ + 2ζω₀·x′ + ω₀²·x = 0), 1 at critical damping. A number of
    /// the kind `damping ratio`, so it isn't a plain `Ratio` or a [QualityFactor] (Q = 1/(2ζ)): `damping_ratio` and
    /// `quality_factor` are the steps.
    #[cfg(feature = "levels")]
    pub struct DampingRatio = "", [] kind "damping ratio";

    /// A quality factor, Q: 1/(2ζ) of a second-order system, ω₀ over the half-power bandwidth. A number of the kind
    /// `quality factor`, so it isn't a plain `Ratio` or a [DampingRatio].
    #[cfg(feature = "levels")]
    pub struct QualityFactor = "", [] kind "quality factor";

    // Thermal

    /// Thermodynamic temperature in kelvins, K.
    #[cfg(feature = "thermal")]
    pub struct Kelvin = "K", [k 1];

    /// Temperature in degrees Celsius, °C (K − 273.15). No products: a product with an offset scale means nothing.
    #[cfg(feature = "thermal")]
    pub struct DegreeCelsius = "°C", [k 1], Affine { factor: 1.0, offset: 273.15 };

    /// Heat flux density in watts per square meter, W/m² (kg·s⁻³, SI Brochure, 9th ed., table 6): heat through a
    /// surface. The same unit as [Irradiance], and converts to it.
    #[cfg(feature = "thermal")]
    pub struct HeatFluxDensity = "W/m²", [kg 1, s -3];

    /// Heat capacity in joules per kelvin, J/K (kg·m²·s⁻²·K⁻¹, SI Brochure, 9th ed., table 6). `Per<Joule, Kelvin>`
    /// converts to it; an [Entropy] doesn't.
    #[cfg(feature = "thermal")]
    pub struct HeatCapacity = "J/K", [kg 1, m 2, s -2, k -1];

    /// Entropy in joules per kelvin, J/K (SI Brochure, 9th ed., table 6). The dimension of a [HeatCapacity], so of the
    /// kind `entropy`: one becomes the other only by `as_entropy` and `as_heat_capacity`.
    #[cfg(feature = "thermal")]
    pub struct Entropy = "J/K", [kg 1, m 2, s -2, k -1] kind "entropy";

    /// Specific heat capacity in joules per kilogram kelvin, J/(kg·K) (m²·s⁻²·K⁻¹, SI Brochure, 9th ed., table 6):
    /// water is about 4186 J/(kg·K).
    #[cfg(feature = "thermal")]
    pub struct SpecificHeatCapacity = "J/(kg·K)", [m 2, s -2, k -1];

    /// Thermal conductivity in watts per meter kelvin, W/(m·K) (kg·m·s⁻³·K⁻¹, SI Brochure, 9th ed., table 6): a
    /// thermal pad's datasheet value.
    #[cfg(feature = "thermal")]
    pub struct ThermalConductivity = "W/(m·K)", [kg 1, m 1, s -3, k -1];

    /// Thermal conductance in watts per kelvin, W/K (kg·m²·s⁻³·K⁻¹): the inverse of a [ThermalResistance].
    #[cfg(feature = "thermal")]
    pub struct ThermalConductance = "W/K", [kg 1, m 2, s -3, k -1];

    /// Thermal resistance in kelvins per watt, K/W (kg⁻¹·m⁻²·s³·K): a heatsink's, a package's θJA. Times a [Watt] it
    /// is a temperature rise in [Kelvin].
    #[cfg(feature = "thermal")]
    pub struct ThermalResistance = "K/W", [kg -1, m -2, s 3, k 1];

    /// Heat transfer coefficient in watts per square meter kelvin, W/(m²·K) (kg·s⁻³·K⁻¹): convection from a surface.
    #[cfg(feature = "thermal")]
    pub struct HeatTransferCoefficient = "W/(m²·K)", [kg 1, s -3, k -1];

    // Chemical

    /// Amount of substance in moles, mol.
    #[cfg(feature = "chemical")]
    pub struct Mole = "mol", [mol 1];

    /// Catalytic activity in katals, kat (mol·s⁻¹).
    #[cfg(feature = "chemical")]
    pub struct Katal = "kat", [mol 1, s -1];

    /// Amount concentration (molarity) in moles per cubic meter, mol/m³ (SI Brochure, 9th ed., table 5); a mol/L is
    /// [MolePerLitre], 10³ mol/m³.
    #[cfg(feature = "chemical")]
    pub struct MolarConcentration = "mol/m³", [mol 1, m -3];

    /// Amount concentration in moles per litre, mol/L (10³ mol/m³): the chemist's molar, M.
    #[cfg(feature = "chemical")]
    pub struct MolePerLitre = "mol/L", [mol 1, m -3], Pow10(3);

    /// A concentration times a path length in molar centimeters, M·cm (10 mol/m²): what a molar absorptivity is per,
    /// `Per<Absorbance, MolarCentimeter>` printing AU/(M·cm), the L/(mol·cm) of tables.
    #[cfg(feature = "chemical")]
    pub struct MolarCentimeter = "M·cm", [mol 1, m -2], Pow10(1);

    /// Molality in moles per kilogram of solvent, mol/kg.
    #[cfg(feature = "chemical")]
    pub struct Molality = "mol/kg", [mol 1, kg -1];

    /// Molar mass in kilograms per mole, kg/mol; the g/mol of tables is [GramPerMole].
    #[cfg(feature = "chemical")]
    pub struct MolarMass = "kg/mol", [kg 1, mol -1];

    /// Molar mass in grams per mole, g/mol (10⁻³ kg/mol): water is about 18.015 g/mol.
    #[cfg(feature = "chemical")]
    pub struct GramPerMole = "g/mol", [kg 1, mol -1], Pow10(-3);

    /// Molar energy in joules per mole, J/mol (kg·m²·s⁻²·mol⁻¹, SI Brochure, 9th ed., table 6).
    #[cfg(feature = "chemical")]
    pub struct MolarEnergy = "J/mol", [kg 1, m 2, s -2, mol -1];

    /// Molar heat capacity in joules per mole kelvin, J/(mol·K) (SI Brochure, 9th ed., table 6): the gas constant R is
    /// 8.314.. J/(mol·K).
    #[cfg(feature = "chemical")]
    pub struct MolarHeatCapacity = "J/(mol·K)", [kg 1, m 2, s -2, k -1, mol -1];

    // Photometric

    /// Luminous intensity in candelas, cd.
    #[cfg(feature = "photometric")]
    pub struct Candela = "cd", [cd 1];

    /// Luminous flux in lumens, lm (cd·sr, cd·rad²).
    #[cfg(feature = "photometric")]
    pub struct Lumen = "lm", [cd 1, rad 2];

    /// Illuminance in lux, lx (cd·rad²·m⁻², lm/m²).
    #[cfg(feature = "photometric")]
    pub struct Lux = "lx", [cd 1, rad 2, m -2];

    /// Luminance in candelas per square meter, cd/m² (SI Brochure, 9th ed., table 5), the nit: a display's
    /// brightness. Text input also takes `nit` and `nt`.
    #[cfg(feature = "photometric")]
    pub struct Nit = "cd/m²", [cd 1, m -2];

    /// Luminous efficacy in lumens per watt, lm/W (cd·rad²·kg⁻¹·m⁻²·s³): an LED's light per electrical watt.
    #[cfg(feature = "photometric")]
    pub struct LuminousEfficacy = "lm/W", [cd 1, rad 2, kg -1, m -2, s 3];

    // Radiometric

    /// Irradiance in watts per square meter, W/m² (kg·s⁻³, SI Brochure, 9th ed., table 6): sunlight on a solar panel,
    /// about 1000 W/m² at noon. The same unit as [HeatFluxDensity], and converts to it.
    #[cfg(feature = "radiometric")]
    pub struct Irradiance = "W/m²", [kg 1, s -3];

    /// Radiant intensity in watts per steradian, W/sr (kg·m²·s⁻³·rad⁻², SI Brochure, 9th ed., table 6): an IR LED's.
    #[cfg(feature = "radiometric")]
    pub struct RadiantIntensity = "W/sr", [kg 1, m 2, s -3, rad -2];

    /// Radiance in watts per square meter steradian, W/(m²·sr) (kg·s⁻³·rad⁻², SI Brochure, 9th ed., table 6).
    #[cfg(feature = "radiometric")]
    pub struct Radiance = "W/(m²·sr)", [kg 1, s -3, rad -2];

    // Radiation

    /// Radioactivity in becquerels, Bq (s⁻¹).
    #[cfg(feature = "radiation")]
    pub struct Becquerel = "Bq", [s -1];

    /// Absorbed dose in grays, Gy (m²·s⁻², J/kg).
    #[cfg(feature = "radiation")]
    pub struct Gray = "Gy", [m 2, s -2];

    /// Equivalent dose in sieverts, Sv (m²·s⁻², J/kg).
    #[cfg(feature = "radiation")]
    pub struct Sievert = "Sv", [m 2, s -2];

    /// Energy in electron-volts, eV (1.602176634·10⁻¹⁹ J).
    #[cfg(feature = "radiation")]
    pub struct ElectronVolt = "eV", [kg 1, m 2, s -2], Exact(1602176634 / 1, pow10 -28);

    /// Exposure to X and γ rays in coulombs per kilogram, C/kg (kg⁻¹·s·A, SI Brochure, 9th ed., table 6).
    #[cfg(feature = "radiation")]
    pub struct Exposure = "C/kg", [kg -1, s 1, a 1];

    /// Absorbed dose rate in grays per second, Gy/s (m²·s⁻³, SI Brochure, 9th ed., table 6).
    #[cfg(feature = "radiation")]
    pub struct AbsorbedDoseRate = "Gy/s", [m 2, s -3];

    /// Equivalent dose rate in sieverts per hour, Sv/h (1/3600 Sv/s): a dosimeter's µSv/h is
    /// `Fixed<SievertPerHour, Micro, T>`.
    #[cfg(feature = "radiation")]
    pub struct SievertPerHour = "Sv/h", [m 2, s -3], Exact(1 / 3600);

    // Dimensionless

    /// A fraction: a duty cycle, a probability, a scale factor, an efficiency (dimensionless, 1). 0.5 is half, not a
    /// level in decibels. Written with `%`, `‰`, `ppm`, `ppb` for `Centi`, `Milli`, `Micro`, `Nano`. Any quantity times a ratio keeps its
    /// unit (48 V · 0.5 = 24 V).
    #[cfg(feature = "dimensionless")]
    pub struct Ratio = "", [];

    // Information

    /// Information in bits, bit: the base of information, so a size never mixes with a [Ratio] or a count of things.
    #[cfg(feature = "information")]
    pub struct Bit = "bit", [bit 1];

    /// Information in nibbles, nibble (4 [Bit]): a hex digit, a BCD digit, a 4-bit field. Not to be confused with
    /// shrink_wrap's `Nibble`, the 4-bit storage a `Fixed` may hold its number in.
    #[cfg(feature = "information")]
    pub struct Nibble = "nibble", [bit 1], Exact(4 / 1);

    /// Information in bytes, B (8 [Bit]): a data size, a buffer's capacity, an offset. A rate is `Per<Byte, Second>`.
    #[cfg(feature = "information")]
    pub struct Byte = "B", [bit 1], Exact(8 / 1);

    /// Information in 16-bit halfwords, halfword (2 [Byte]): Arm's halfword, a 16-bit register, bus or DMA transfer.
    #[cfg(feature = "information")]
    pub struct Halfword = "halfword", [bit 1], Exact(16 / 1);

    /// Information in 32-bit words, word (4 [Byte]): flash and RAM sizes of a 32-bit MCU.
    #[cfg(feature = "information")]
    pub struct Word = "word", [bit 1], Exact(32 / 1);

    /// Information in 64-bit doublewords, doubleword (8 [Byte]): Arm's doubleword, the program width of an STM32G0's
    /// or L4's flash, a 64-bit DMA or bus transfer.
    #[cfg(feature = "information")]
    pub struct Doubleword = "doubleword", [bit 1], Exact(64 / 1);

    // Counting: each marker is a base dimension of its own (a counted base, named by its crate and marker name), so
    // samples don't add to bytes, and become bytes only through a `Per<Byte, Sample>`. An API that counts something else
    // declares its own marker with `exact_units::counted!(Name)`, see [Unit].

    /// A count of things, item: a queue's length, a number of filters, channels, presses, repeats, steps.
    #[cfg(feature = "counting")]
    pub struct Item = "item", counted;

    /// A count of samples, sample: an ADC's, a logic analyzer's; a sample rate is `Per<Sample, Second>`.
    #[cfg(feature = "counting")]
    pub struct Sample = "sample", counted;

    /// A count of frames, frame (a bus's, a video's).
    #[cfg(feature = "counting")]
    pub struct Frame = "frame", counted;

    /// A count of packets, packet.
    #[cfg(feature = "counting")]
    pub struct Packet = "packet", counted;

    /// A count of lines, line: of a text, a file, a log.
    #[cfg(feature = "counting")]
    pub struct Line = "line", counted;

    /// A count of rows, row: of a table, a grid, a report.
    #[cfg(feature = "counting")]
    pub struct Row = "row", counted;

    /// A count of records, record: of a store, a file format, a ring buffer.
    #[cfg(feature = "counting")]
    pub struct Record = "record", counted;

    /// A count of entries, entry: of a log, a list, a directory, a table of contents.
    #[cfg(feature = "counting")]
    pub struct Entry = "entry", counted;

    /// A count of cycles, cycle: clock cycles, time quanta; time only with a clock (`Per<Cycle, Second>`).
    #[cfg(feature = "counting")]
    pub struct Cycle = "cycle", counted;

    /// A count of blocks, block: of a block device, whose size differs per device (bytes through `Per<Byte, Block>`).
    #[cfg(feature = "counting")]
    pub struct Block = "block", counted;

    /// A count of sectors, sector: of a flash or a disk (bytes through `Per<Byte, Sector>`).
    #[cfg(feature = "counting")]
    pub struct Sector = "sector", counted;

    /// A count of chunks, chunk: of a transfer or a file format (bytes through `Per<Byte, Chunk>`).
    #[cfg(feature = "counting")]
    pub struct Chunk = "chunk", counted;

    /// A count of characters, char: of a UART line (an idle time in characters).
    #[cfg(feature = "counting")]
    pub struct Char = "char", counted;

    /// A count of raw codes of a converter, code: an ADC's or DAC's output before scaling (LSB).
    #[cfg(feature = "counting")]
    pub struct Code = "code", counted;
}

// Compound

/// `A / B`, a unit outside the named list: `Per<Torque, AngularVelocity>` is N·m/(rad/s). Not normalized,
/// `Per<Meter, Second>` is not [Speed] (convert with `to_si_f64` / `from_f32`).
///
/// A quotient of one dimension keeps it as its [Kind]: `Per<Volt, Volt>` is a voltage gain, which
/// doesn't convert to `Per<Ampere, Ampere>` or a [Ratio] (`as_ratio` is the step), and has a level in dB (`to_db`).
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct Per<A, B> {
    _a: PhantomData<A>,
    _b: PhantomData<B>,
}

/// `A · B`, a unit outside the named list: `Times<AngularVelocity, AngularVelocity>` is (rad/s)·(rad/s).
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct Times<A, B> {
    _a: PhantomData<A>,
    _b: PhantomData<B>,
}

/// `√U`, the square root of a unit: an amplitude spectral density in V/√Hz is `Per<Volt, Sqrt<Hertz>>`. Its
/// dimension has the root flag ([Dim::sqrt](crate::Dim::sqrt)): the exponents of V²/Hz, so the algebra stays integer.
/// `Sqrt<Sqrt<U>>` doesn't compile (a fourth root isn't a [Dim]).
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct Sqrt<U> {
    _u: PhantomData<U>,
}

/// A compile error for a compound unit of more than [MAX_UNIT_DEPTH] levels, where its tree is used: a unit deeper
/// than any reader follows.
const fn check_depth(levels: usize) {
    assert!(
        levels <= MAX_UNIT_DEPTH,
        "a unit tree of more than MAX_UNIT_DEPTH (32) levels: no reader follows it"
    );
}

/// Levels of a compound unit over parts of `a` and `b` levels.
const fn deeper(a: usize, b: usize) -> usize {
    let below = if a > b { a } else { b };
    below.saturating_add(1)
}

impl<U: Unit> Unit for Sqrt<U> {
    const DIM: Dim<'static> = U::DIM.sqrt();
    const KIND: Kind<'static> = U::KIND.sqrt();
    const DEPTH: usize = deeper(U::DEPTH, 0);
    const EXPR: &'static UnitExpr = {
        check_depth(Self::DEPTH);
        &UnitExpr::Sqrt(U::EXPR)
    };
    #[cfg(feature = "shrink_wrap")]
    const SI_EXPR: &'static SIExpr<'static> = {
        check_depth(Self::DEPTH);
        &SIExpr::Sqrt(RefBox::Ref { value: U::SI_EXPR })
    };
}

/// A unit of plane angle (rad¹ and no kind of its own): [Radian], [Degree], [Turn], or an API's own angle unit. What
/// [Phase] wraps, and what the named steps `as_phase` and `electrical` start from.
pub trait PlaneAngle: Unit {}

#[cfg(feature = "angle")]
impl PlaneAngle for Radian {}
#[cfg(feature = "angle")]
impl PlaneAngle for Degree {}
#[cfg(feature = "angle")]
impl PlaneAngle for Turn {}

/// A phase angle in the angle unit `A`: the argument of a phasor, an NCO's phase, a motor's electrical angle. Same
/// dimension, scale and symbol as `A`, but of the kind `phase angle` ([Kind::Named]), so it isn't a mechanical
/// angle: `Phase<Turn>` converts to `Phase<Degree>`, not to [Turn]. The steps between them are named: `as_phase` and
/// `as_plane_angle` (the same number), and `electrical(pole_pairs)` from a motor's mechanical angle to its electrical
/// angle, the phase a field-oriented control's Park transform takes.
///
/// ```
/// # #[cfg(feature = "angle")] {
/// use exact_units::{Bam16, Fixed, PhaseBam16, prefix::One, units::{Degree, Phase, Turn}};
///
/// let half = Fixed::<Phase<Turn>, One, f32>::new(0.5);
/// assert_eq!(half.convert::<Phase<Degree>, One, f32>(), Some(Fixed::new(180.0)));
/// // a rotor at 0.3 turn with 7 pole pairs is at 2.1 turns electrical, 0.1 turn of phase
/// let mechanical: Bam16 = exact_units::fixed!(0.3);
/// let phase: PhaseBam16 = mechanical.electrical(7);
/// assert_eq!(phase.get(), 19661u16.wrapping_mul(7));
/// # }
/// ```
///
/// On the wire and in a unit's [UnitExpr] it is `A`: the kind is in the type, as for [Ohm].
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct Phase<A> {
    _a: PhantomData<A>,
}

impl<A: PlaneAngle> Unit for Phase<A> {
    const DIM: Dim<'static> = {
        assert!(
            A::DIM.same(Dim::ONE.with_rad(1)) && A::KIND.same(Kind::Plain),
            "Phase wraps a plane angle (rad, °, rev) of no kind of its own"
        );
        A::DIM
    };
    const KIND: Kind<'static> = Kind::Named("phase angle");
    const DEPTH: usize = A::DEPTH;
    const EXPR: &'static UnitExpr = A::EXPR;
    #[cfg(feature = "shrink_wrap")]
    const SI_EXPR: &'static SIExpr<'static> = A::SI_EXPR;
}

impl<A: Unit, B: Unit> Unit for Per<A, B> {
    const DIM: Dim<'static> = A::DIM.div(B::DIM);
    const KIND: Kind<'static> = A::KIND.div(A::DIM, B::KIND, B::DIM);
    const DEPTH: usize = deeper(A::DEPTH, B::DEPTH);
    const EXPR: &'static UnitExpr = {
        check_depth(Self::DEPTH);
        &UnitExpr::Div(A::EXPR, B::EXPR)
    };
    #[cfg(feature = "shrink_wrap")]
    const SI_EXPR: &'static SIExpr<'static> = {
        check_depth(Self::DEPTH);
        &SIExpr::Div(
            RefBox::Ref { value: A::SI_EXPR },
            RefBox::Ref { value: B::SI_EXPR },
        )
    };
}

impl<A: Unit, B: Unit> Unit for Times<A, B> {
    const DIM: Dim<'static> = A::DIM.mul(B::DIM);
    const KIND: Kind<'static> = A::KIND.mul(A::DIM, B::KIND, B::DIM);
    const DEPTH: usize = deeper(A::DEPTH, B::DEPTH);
    const EXPR: &'static UnitExpr = {
        check_depth(Self::DEPTH);
        &UnitExpr::Mul(A::EXPR, B::EXPR)
    };
    #[cfg(feature = "shrink_wrap")]
    const SI_EXPR: &'static SIExpr<'static> = {
        check_depth(Self::DEPTH);
        &SIExpr::Mul(
            RefBox::Ref { value: A::SI_EXPR },
            RefBox::Ref { value: B::SI_EXPR },
        )
    };
}

/// The named unit of this crate called `name` (the marker's name, `"Volt"`), if its feature is enabled: what a
/// dynamic client needs to show a `Quantity<Volt>` it only knows from introspection data.
#[must_use]
pub fn by_name(name: &str) -> Option<&'static UnitExpr> {
    let all = ALL.iter();
    #[cfg(feature = "customary")]
    let all = all.chain(CUSTOMARY);
    all.copied()
        .find(|expr| matches!(expr, UnitExpr::Named { name: n, .. } if *n == name))
}

/// Other spellings text input takes for the named unit `name` (the marker's name, `"Nit"`), next to the symbol it is
/// written with: `nit` and `nt` for [Nit]'s cd/m². Empty for most units; a prefix goes before a spelling as before the
/// symbol (`knit`).
#[must_use]
pub fn spellings(name: &str) -> &'static [&'static str] {
    match name {
        "Nit" => &["nit", "nt"],
        _ => &[],
    }
}

/// Cycles per second are hertz: the counted [Cycle] and the hertz of a clock are different dimensions (Hz is s⁻¹),
/// so going from one to the other is this named step, never an implicit conversion.
#[cfg(all(feature = "counting", feature = "time"))]
impl<P, T: crate::Storage, B: crate::Bounds<T>> crate::Fixed<Per<Cycle, Second>, P, T, B> {
    /// The same number in Hz.
    pub fn to_hertz(self) -> crate::Fixed<Hertz, P, T, B> {
        crate::Fixed::new_unchecked(self.get())
    }
}

#[cfg(all(feature = "levels", feature = "time"))]
pub use crate::level::DampingCoefficient;
#[cfg(all(feature = "levels", feature = "counting"))]
pub use crate::level::LogarithmicDecrement;
#[cfg(all(feature = "levels", feature = "chemical"))]
pub use crate::level::MolarAbsorptivity;
#[cfg(all(feature = "levels", feature = "mechanical"))]
pub use crate::level::{DecadicAttenuation, LinearAttenuation};
#[cfg(feature = "levels")]
pub use crate::level::{Level, Loss, LossLevel};
