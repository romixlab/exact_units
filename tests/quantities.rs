//! The quantity rules walked whole: every pair of this crate's quantities, the derived quantity of each of their
//! dimensions and a few ratios, asked [converts](QuantityExpr::converts), [reaches](QuantityExpr::reaches) and
//! [add](QuantityExpr::add) four ways (in a `const` item, through the `const` twins, by id through
//! [units::TABLE], and by id through a table whose parents are ids too, as data read from an API model), each against
//! the rules written out by hand in [TREE].
#![cfg(feature = "std")]
#![expect(clippy::expect_used, reason = "a test fails by panicking")]
#![expect(
    clippy::indexing_slicing,
    reason = "the const items index their arrays below the lengths their loops run to"
)]
#![expect(
    clippy::option_option,
    reason = "const_add's own answer: unknown, two kinds, or the sum"
)]

use exact_units::quantities::{
    self, AbsorbedDose, Activity, ApparentPower, Conductance, DampingRatio, Distance,
    DoseEquivalent, Energy, EnergyDensity, Entropy, Frequency, Heat, Height, KineticEnergy, Length,
    MechanicalEnergy, PathLength, PhaseAngle, PlaneAngle, PotentialEnergy, Power, Pressure,
    QualityFactor, Radius, ReactivePower, Resistance, SymbolRate, Width, Work,
};
use exact_units::units::{self, Meter, Per, Second};
use exact_units::{
    Dim, MarkerId, QuantityDef, QuantityExpr, QuantityRef, QuantitySpec, Unit, UnitDef, UnitTable,
};
use std::collections::BTreeSet;

/// Each quantity of [quantities::ALL] by name, with the quantities it converts to with no step (up within its
/// kind; `D` its dimension's derived quantity, where the walk reaches the open root) and all the ones above it.
const TREE: &[(&str, &[&str], &[&str])] = &[
    ("Length", &["D"], &[]),
    ("Height", &["Length", "D"], &["Length"]),
    ("Width", &["Length", "D"], &["Length"]),
    ("Radius", &["Length", "D"], &["Length"]),
    ("Distance", &["Length", "D"], &["Length"]),
    (
        "PathLength",
        &["Distance", "Length", "D"],
        &["Distance", "Length"],
    ),
    ("Energy", &["D"], &[]),
    ("MechanicalEnergy", &["Energy", "D"], &["Energy"]),
    (
        "KineticEnergy",
        &["MechanicalEnergy", "Energy", "D"],
        &["MechanicalEnergy", "Energy"],
    ),
    (
        "PotentialEnergy",
        &["MechanicalEnergy", "Energy", "D"],
        &["MechanicalEnergy", "Energy"],
    ),
    ("Work", &["Energy", "D"], &["Energy"]),
    ("Heat", &["Energy", "D"], &["Energy"]),
    ("Frequency", &["D"], &[]),
    ("Activity", &[], &[]),
    ("SymbolRate", &[], &[]),
    ("AbsorbedDose", &[], &[]),
    ("Kerma", &[], &["AbsorbedDose"]),
    ("DoseEquivalent", &[], &[]),
    ("AmbientDoseEquivalent10", &[], &["DoseEquivalent"]),
    ("PersonalDoseEquivalent10", &[], &["DoseEquivalent"]),
    ("PersonalDoseEquivalent0_07", &[], &["DoseEquivalent"]),
    ("EquivalentDose", &[], &["DoseEquivalent"]),
    ("EffectiveDose", &[], &["DoseEquivalent"]),
    ("AbsorbedDoseRate", &[], &[]),
    ("KermaRate", &[], &["AbsorbedDoseRate"]),
    ("DoseEquivalentRate", &[], &[]),
    ("AmbientDoseEquivalentRate10", &[], &["DoseEquivalentRate"]),
    ("PersonalDoseEquivalentRate10", &[], &["DoseEquivalentRate"]),
    (
        "PersonalDoseEquivalentRate0_07",
        &[],
        &["DoseEquivalentRate"],
    ),
    ("EquivalentDoseRate", &[], &["DoseEquivalentRate"]),
    ("EffectiveDoseRate", &[], &["DoseEquivalentRate"]),
    ("LinearEnergyTransfer", &[], &[]),
    ("Exposure", &[], &[]),
    ("Resistance", &[], &[]),
    ("Conductance", &[], &[]),
    ("Power", &["D"], &[]),
    ("ApparentPower", &[], &[]),
    ("ReactivePower", &[], &[]),
    ("Pressure", &["D"], &[]),
    ("EnergyDensity", &[], &[]),
    ("Entropy", &[], &[]),
    ("DampingRatio", &[], &[]),
    ("QualityFactor", &[], &[]),
    ("ElectricCurrent", &["D"], &[]),
    ("MagnetomotiveForce", &[], &[]),
    ("ElectricCharge", &["D"], &[]),
    ("Voltage", &["D"], &[]),
    ("ElectricField", &["D"], &[]),
    ("ElectricFluxDensity", &[], &[]),
    ("ElectricPolarization", &[], &["ElectricFluxDensity"]),
    ("SurfaceChargeDensity", &[], &[]),
    ("MagneticFlux", &["D"], &[]),
    ("MagneticFluxDensity", &["D"], &[]),
    ("MagnetizingField", &[], &[]),
    ("Magnetization", &[], &["MagnetizingField"]),
    ("LinearCurrentDensity", &[], &[]),
    ("Capacitance", &["D"], &[]),
    ("Inductance", &["D"], &[]),
    ("PlaneAngle", &["D"], &[]),
    ("PhaseAngle", &[], &["PlaneAngle"]),
    ("Time", &["D"], &[]),
    ("Mass", &["D"], &[]),
    ("ThermodynamicTemperature", &["D"], &[]),
    ("AmountOfSubstance", &["D"], &[]),
    ("LuminousIntensity", &["D"], &[]),
    ("Area", &["D"], &[]),
    ("Volume", &["D"], &[]),
    ("Force", &["D"], &[]),
    ("CatalyticActivity", &["D"], &[]),
    ("SolidAngle", &["D"], &[]),
    ("LuminousFlux", &["D"], &[]),
    ("Illuminance", &["D"], &[]),
    ("Luminance", &["D"], &[]),
    ("Information", &["D"], &[]),
];

/// The quantities as `const` data, in [TREE]'s order.
const DEFS: [&QuantityDef<'static>; 74] = [
    Length::DEF,
    Height::DEF,
    Width::DEF,
    Radius::DEF,
    Distance::DEF,
    PathLength::DEF,
    Energy::DEF,
    MechanicalEnergy::DEF,
    KineticEnergy::DEF,
    PotentialEnergy::DEF,
    Work::DEF,
    Heat::DEF,
    Frequency::DEF,
    Activity::DEF,
    SymbolRate::DEF,
    AbsorbedDose::DEF,
    quantities::Kerma::DEF,
    DoseEquivalent::DEF,
    quantities::AmbientDoseEquivalent10::DEF,
    quantities::PersonalDoseEquivalent10::DEF,
    quantities::PersonalDoseEquivalent0_07::DEF,
    quantities::EquivalentDose::DEF,
    quantities::EffectiveDose::DEF,
    quantities::AbsorbedDoseRate::DEF,
    quantities::KermaRate::DEF,
    quantities::DoseEquivalentRate::DEF,
    quantities::AmbientDoseEquivalentRate10::DEF,
    quantities::PersonalDoseEquivalentRate10::DEF,
    quantities::PersonalDoseEquivalentRate0_07::DEF,
    quantities::EquivalentDoseRate::DEF,
    quantities::EffectiveDoseRate::DEF,
    quantities::LinearEnergyTransfer::DEF,
    quantities::Exposure::DEF,
    Resistance::DEF,
    Conductance::DEF,
    Power::DEF,
    ApparentPower::DEF,
    ReactivePower::DEF,
    Pressure::DEF,
    EnergyDensity::DEF,
    Entropy::DEF,
    DampingRatio::DEF,
    QualityFactor::DEF,
    quantities::ElectricCurrent::DEF,
    quantities::MagnetomotiveForce::DEF,
    quantities::ElectricCharge::DEF,
    quantities::Voltage::DEF,
    quantities::ElectricField::DEF,
    quantities::ElectricFluxDensity::DEF,
    quantities::ElectricPolarization::DEF,
    quantities::SurfaceChargeDensity::DEF,
    quantities::MagneticFlux::DEF,
    quantities::MagneticFluxDensity::DEF,
    quantities::MagnetizingField::DEF,
    quantities::Magnetization::DEF,
    quantities::LinearCurrentDensity::DEF,
    quantities::Capacitance::DEF,
    quantities::Inductance::DEF,
    PlaneAngle::DEF,
    PhaseAngle::DEF,
    quantities::Time::DEF,
    quantities::Mass::DEF,
    quantities::ThermodynamicTemperature::DEF,
    quantities::AmountOfSubstance::DEF,
    quantities::LuminousIntensity::DEF,
    quantities::Area::DEF,
    quantities::Volume::DEF,
    quantities::Force::DEF,
    quantities::CatalyticActivity::DEF,
    quantities::SolidAngle::DEF,
    quantities::LuminousFlux::DEF,
    quantities::Illuminance::DEF,
    quantities::Luminance::DEF,
    quantities::Information::DEF,
];

/// The expressions walked: each quantity, then the derived quantity of each of their dimensions and of a velocity,
/// then ratios of length and of energy.
const EXPRS: [QuantityExpr<'static>; 96] = {
    let dims: [Dim<'static>; 19] = [
        Length::DEF.dim,
        Energy::DEF.dim,
        Frequency::DEF.dim,
        AbsorbedDose::DEF.dim,
        quantities::AbsorbedDoseRate::DEF.dim,
        quantities::LinearEnergyTransfer::DEF.dim,
        Resistance::DEF.dim,
        Conductance::DEF.dim,
        Power::DEF.dim,
        Pressure::DEF.dim,
        Entropy::DEF.dim,
        Dim::ONE,
        PlaneAngle::DEF.dim,
        <Per<Meter, Second> as Unit>::DIM,
        // the same dimensions again, a second derived each: two Derived of one dimension are one quantity
        Length::DEF.dim,
        Energy::DEF.dim,
        Frequency::DEF.dim,
        AbsorbedDose::DEF.dim,
        Resistance::DEF.dim,
    ];
    let mut out = [QuantityExpr::RatioOf(Dim::ONE); 96];
    let mut i = 0;
    while i < DEFS.len() {
        out[i] = QuantityExpr::Spec(QuantityRef::Def(DEFS[i]));
        i += 1;
    }
    let mut j = 0;
    while j < dims.len() {
        out[i + j] = QuantityExpr::Derived(dims[j]);
        j += 1;
    }
    out[i + j] = QuantityExpr::RatioOf(Length::DEF.dim);
    out[i + j + 1] = QuantityExpr::RatioOf(Energy::DEF.dim);
    out[i + j + 2] = QuantityExpr::RatioOf(Length::DEF.dim);
    out
};

const N: usize = EXPRS.len();

/// [QuantityExpr::const_converts] of every pair, worked out by the compiler.
static CONVERTS: [[Option<bool>; N]; N] = {
    let mut m = [[None; N]; N];
    let mut a = 0;
    while a < N {
        let mut b = 0;
        while b < N {
            m[a][b] = EXPRS[a].const_converts(EXPRS[b]);
            b += 1;
        }
        a += 1;
    }
    m
};

/// [QuantityExpr::const_reaches] of every expression to every quantity, worked out by the compiler.
static REACHES: [[Option<bool>; DEFS.len()]; N] = {
    let mut m = [[None; DEFS.len()]; N];
    let mut a = 0;
    while a < N {
        let mut b = 0;
        while b < DEFS.len() {
            m[a][b] = EXPRS[a].const_reaches(DEFS[b]);
            b += 1;
        }
        a += 1;
    }
    m
};

/// [QuantityExpr::const_add] of every pair, worked out by the compiler.
static ADD: [[Option<Option<QuantityExpr<'static>>>; N]; N] = {
    let mut m = [[None; N]; N];
    let mut a = 0;
    while a < N {
        let mut b = 0;
        while b < N {
            m[a][b] = EXPRS[a].const_add(EXPRS[b]);
            b += 1;
        }
        a += 1;
    }
    m
};

/// A table of quantities read from data: each parent by its id, so every walk up looks it up here.
struct IdOnly(Vec<QuantityDef<'static>>);

impl IdOnly {
    fn new() -> Self {
        IdOnly(
            quantities::ALL
                .iter()
                .map(|q| QuantityDef {
                    parent: q.parent.map(|p| QuantityRef::Id(p.id())),
                    ..**q
                })
                .collect(),
        )
    }

    fn get(&self, name: &str) -> &QuantityDef<'static> {
        self.0
            .iter()
            .find(|q| q.id.name == name)
            .expect("a quantity of the table")
    }
}

impl UnitTable for IdOnly {
    fn unit(&self, _: MarkerId<'_>) -> Option<UnitDef<'_>> {
        None
    }

    fn quantity(&self, id: MarkerId<'_>) -> Option<QuantityDef<'_>> {
        self.0.iter().find(|q| q.id == id).copied()
    }
}

/// The expression as a name: a quantity's own, `D:` or `R:` and the dimension.
fn key(e: QuantityExpr<'_>) -> String {
    match e {
        QuantityExpr::Spec(r) => r.id().name.to_owned(),
        QuantityExpr::Derived(d) => format!("D:{d}"),
        QuantityExpr::RatioOf(d) => format!("R:{d}"),
    }
}

fn row(name: &str) -> (&'static [&'static str], &'static [&'static str]) {
    let (_, up, above) = TREE
        .iter()
        .find(|(n, ..)| *n == name)
        .expect("a quantity of TREE");
    (up, above)
}

fn def(name: &str) -> &'static QuantityDef<'static> {
    DEFS.iter()
        .find(|d| d.id.name == name)
        .expect("a quantity of DEFS")
}

/// Its dimension's open root, by [TREE]: nothing above it, its derived quantity reached with no step.
fn open_root(d: Dim<'_>) -> Option<&'static str> {
    TREE.iter()
        .find(|(n, up, above)| above.is_empty() && *up == ["D"] && def(n).dim.same(d))
        .map(|(n, ..)| *n)
}

/// What `e` converts to with no step, itself included, by [TREE].
fn closure(e: QuantityExpr<'_>) -> BTreeSet<String> {
    let mut s = BTreeSet::from([key(e)]);
    match e {
        QuantityExpr::Spec(r) => {
            let dim = def(r.id().name).dim;
            for up in row(r.id().name).0 {
                s.insert(if *up == "D" {
                    format!("D:{dim}")
                } else {
                    (*up).to_owned()
                });
            }
        }
        QuantityExpr::Derived(d) => {
            s.extend(open_root(d).map(str::to_owned));
        }
        QuantityExpr::RatioOf(_) => {}
    }
    s
}

fn expect_converts(a: QuantityExpr<'_>, b: QuantityExpr<'_>) -> bool {
    closure(a).contains(&key(b))
}

/// The sum's quantity by [TREE]: `None` for two kinds, the derived quantity where one side is, else the lowest
/// quantity both convert to.
fn expect_add(a: QuantityExpr<'_>, b: QuantityExpr<'_>) -> Option<String> {
    let (ca, cb) = (closure(a), closure(b));
    let common: Vec<&String> = ca.intersection(&cb).collect();
    if common.is_empty() {
        return None;
    }
    if let QuantityExpr::Derived(d) = a {
        return Some(format!("D:{d}"));
    }
    if let QuantityExpr::Derived(d) = b {
        return Some(format!("D:{d}"));
    }
    if let QuantityExpr::RatioOf(_) = a {
        return Some(key(a));
    }
    common
        .into_iter()
        .filter(|k| !k.starts_with("D:"))
        .max_by_key(|k| row(k).0.len())
        .cloned()
}

/// Whether `as_quantity::<to>()` takes `a` there, by [TREE]: one dimension, and `to` the open root (but not from a
/// ratio), or `a` derived, the open root or on one line with `to`.
fn expect_reaches(a: QuantityExpr<'_>, to: &str) -> bool {
    let to_dim = def(to).dim;
    let (dim, from) = match a {
        QuantityExpr::Spec(r) => (def(r.id().name).dim, Some(r.id().name)),
        QuantityExpr::Derived(d) => (d, None),
        QuantityExpr::RatioOf(_) => (Dim::ONE, None),
    };
    if !dim.same(to_dim) {
        return false;
    }
    if open_root(to_dim) == Some(to) {
        return !matches!(a, QuantityExpr::RatioOf(_));
    }
    match (a, from) {
        (QuantityExpr::Derived(_), _) => true,
        (QuantityExpr::Spec(_), Some(from)) => {
            open_root(dim) == Some(from)
                || from == to
                || row(from).1.contains(&to)
                || row(to).1.contains(&from)
        }
        _ => false,
    }
}

/// `e` with each quantity by its id, as read from data.
fn by_id(e: QuantityExpr<'_>) -> QuantityExpr<'_> {
    match e {
        QuantityExpr::Spec(r) => QuantityExpr::Spec(QuantityRef::Id(r.id())),
        other => other,
    }
}

#[test]
fn tree_is_the_catalogue() {
    assert_eq!(TREE.len(), quantities::ALL.len());
    assert_eq!(DEFS.len(), quantities::ALL.len());
    for ((name, _, above), d) in TREE.iter().zip(DEFS) {
        assert_eq!(d.id.name, *name);
        assert!(quantities::ALL.iter().any(|q| q == &d), "{name} in ALL");
        // the parent is the first quantity above, and a child has its parent's dimension
        assert_eq!(
            d.parent.map(|p| p.id().name),
            above.first().copied(),
            "{name}'s parent"
        );
        if let Some(p) = d.parent {
            assert!(
                p.def().is_some_and(|p| p.dim.same(d.dim)),
                "{name}'s dimension"
            );
        }
    }
}

#[test]
fn converts_every_pair() {
    let ids = IdOnly::new();
    for (a, &ea) in EXPRS.iter().enumerate() {
        for (b, &eb) in EXPRS.iter().enumerate() {
            let want = expect_converts(ea, eb);
            let what = format!("{} converts to {}", key(ea), key(eb));
            assert_eq!(CONVERTS[a][b], Some(want), "{what} in a const item");
            assert_eq!(ea.const_converts(eb), Some(want), "{what}, const_converts");
            assert_eq!(ea.converts(eb, &()), Some(want), "{what}, converts");
            assert_eq!(
                by_id(ea).converts(by_id(eb), &units::TABLE),
                Some(want),
                "{what} by id"
            );
            let (ia, ib) = (by_id(ea), by_id(eb));
            assert_eq!(ia.converts(ib, &ids), Some(want), "{what}, parents by id");
            assert_eq!(
                ea.const_same(eb),
                Some(want && expect_converts(eb, ea)),
                "{what}, same"
            );
            if matches!(ea, QuantityExpr::Spec(_)) {
                assert_eq!(
                    by_id(ea).const_converts(eb),
                    None,
                    "{what}: const code can't look up an id"
                );
            }
        }
    }
}

#[test]
fn reaches_every_pair() {
    let ids = IdOnly::new();
    for (a, &ea) in EXPRS.iter().enumerate() {
        for (b, to) in DEFS.iter().enumerate() {
            let want = expect_reaches(ea, to.id.name);
            let what = format!("{} reaches {}", key(ea), to.id.name);
            assert_eq!(REACHES[a][b], Some(want), "{what} in a const item");
            assert_eq!(ea.const_reaches(to), Some(want), "{what}, const_reaches");
            assert_eq!(ea.reaches(to, &()), Some(want), "{what}, reaches");
            assert_eq!(
                by_id(ea).reaches(to, &units::TABLE),
                Some(want),
                "{what} by id"
            );
            let ia = by_id(ea);
            assert_eq!(
                ia.reaches(ids.get(to.id.name), &ids),
                Some(want),
                "{what}, parents by id"
            );
        }
    }
}

#[test]
fn add_every_pair() {
    let ids = IdOnly::new();
    for (a, &ea) in EXPRS.iter().enumerate() {
        for (b, &eb) in EXPRS.iter().enumerate() {
            let want = expect_add(ea, eb);
            let what = format!("{} + {}", key(ea), key(eb));
            assert_eq!(
                ADD[a][b].map(|s| s.map(key)),
                Some(want.clone()),
                "{what} in a const item"
            );
            assert_eq!(
                ea.const_add(eb).map(|s| s.map(key)),
                Some(want.clone()),
                "{what}, const_add"
            );
            assert_eq!(
                ea.add(eb, &()).map(|s| s.map(key)),
                Some(want.clone()),
                "{what}, add"
            );
            let table = by_id(ea).add(by_id(eb), &units::TABLE);
            assert_eq!(
                table.map(|s| s.map(key)),
                Some(want.clone()),
                "{what} by id"
            );
            let (ia, ib) = (by_id(ea), by_id(eb));
            assert_eq!(
                ia.add(ib, &ids).map(|s| s.map(key)),
                Some(want.clone()),
                "{what}, parents by id"
            );
            // a sum is commutative, and each side converts to it
            assert_eq!(expect_add(eb, ea), want, "{what} both ways");
            if let Some(Some(sum)) = ADD[a][b] {
                assert_eq!(
                    ea.const_converts(sum),
                    Some(true),
                    "{what}: the left side converts to the sum"
                );
                assert_eq!(
                    eb.const_converts(sum),
                    Some(true),
                    "{what}: the right side converts to the sum"
                );
            }
        }
    }
}

/// A walk of data read at run time ends: a cycle of parents is refused, not looped on.
#[test]
fn a_cycle_is_refused() {
    let a = MarkerId {
        crate_name: "data",
        name: "A",
    };
    let b = MarkerId {
        crate_name: "data",
        name: "B",
    };
    let mk = |id, parent| QuantityDef {
        id,
        parent: Some(QuantityRef::Id(parent)),
        ..*Length::DEF
    };
    let t = IdOnly(vec![mk(a, b), mk(b, a)]);
    let ea = QuantityExpr::Spec(QuantityRef::Id(a));
    let eb = QuantityExpr::Spec(QuantityRef::Id(b));
    assert_eq!(
        ea.converts(QuantityExpr::Derived(Length::DEF.dim), &t),
        None
    );
    assert_eq!(ea.add(eb, &t), None);
    // an id the table doesn't list is not known
    let c = QuantityExpr::Spec(QuantityRef::Id(MarkerId {
        crate_name: "data",
        name: "C",
    }));
    assert_eq!(c.converts(ea, &t), None);
}

/// One open root per dimension, so [Derived](QuantityExpr::Derived) means one quantity; no quantity shares a unit's
/// name, as both are markers of this crate, found by name in one table.
#[test]
fn one_open_root_per_dimension() {
    for (i, a) in quantities::ALL.iter().enumerate() {
        for b in quantities::ALL.iter().skip(i + 1) {
            assert!(
                !(a.is_open_root() && b.is_open_root() && a.dim.same(b.dim)),
                "{} and {} are both open roots of {}",
                a.id,
                b.id,
                a.dim
            );
            assert_ne!(a.id, b.id);
        }
        assert!(
            units::ALL
                .iter()
                .all(|u| u.def().is_none_or(|u| u.id.name != a.id.name)),
            "{} is a unit",
            a.id
        );
        assert_eq!(units::TABLE.quantity(a.id), Some(**a));
    }
}

/// A definition goes on the wire with its parent as an id, and comes back as the definition [IdOnly] holds.
#[cfg(feature = "shrink_wrap")]
#[test]
fn wire_round_trip() {
    use exact_units::QuantityDefOwned;
    use shrink_wrap::prelude::*;

    let ids = IdOnly::new();
    for q in quantities::ALL {
        let mut buf = [0u8; 256];
        let bytes = q.to_ww_bytes(&mut buf).expect("a definition fits").to_vec();
        let back = QuantityDef::from_ww_bytes(&bytes).expect("its own bytes");
        assert_eq!(back, **q);
        assert!(
            back.parent.is_none_or(|p| p.def().is_none()),
            "{}: the parent read is an id",
            q.id
        );
        assert_eq!(&back, ids.get(q.id.name));
        let owned = QuantityDefOwned::from(**q);
        assert_eq!(&owned.as_def(), ids.get(q.id.name));
        let bytes = owned.to_ww_bytes_owned().expect("a definition fits");
        assert_eq!(
            QuantityDefOwned::from_ww_bytes_owned(&bytes).expect("its own bytes"),
            owned
        );
    }
}

/// A quantity's English name splits at the capitals and before a depth, `_` its decimal point.
#[test]
fn names() {
    assert_eq!(PathLength::DEF.name().to_string(), "path length");
    assert_eq!(
        quantities::AmbientDoseEquivalent10::DEF.name().to_string(),
        "ambient dose equivalent 10"
    );
    assert_eq!(
        quantities::PersonalDoseEquivalent0_07::DEF
            .name()
            .to_string(),
        "personal dose equivalent 0.07"
    );
    assert_eq!(
        quantities::PersonalDoseEquivalentRate10::DEF
            .name()
            .to_string(),
        "personal dose equivalent rate 10"
    );
}
