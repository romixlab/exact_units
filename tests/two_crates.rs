//! Two crates, `rpm_a` and `rpm_b` (tests/fixtures), declare the same `unit!` line `Rpm`: two units that print the
//! same, each with its own identity. A `Ref` to each resolves through a table that lists both, a `Def` of a unit no
//! table lists reads alone, and compacting turns only what a table lists into a `Ref`.
#![expect(clippy::unwrap_used, reason = "a test fails by panicking")]

use exact_units::units::{self, Per, Second};
use exact_units::{
    CompactError, Magnitude, MarkerId, SIExpr, SIExprOwned, Unit, UnitDef, UnitExpr, UnitTable,
};
use shrink_wrap::prelude::*;

fn def_a() -> UnitDef<'static> {
    *rpm_a::Rpm::EXPR.def().unwrap()
}

fn def_b() -> UnitDef<'static> {
    *rpm_b::Rpm::EXPR.def().unwrap()
}

/// What each crate's table and this crate's lists, chained.
fn both() -> [&'static dyn UnitTable; 3] {
    [&units::TABLE, &rpm_a::UNITS, &rpm_b::UNITS]
}

#[test]
fn two_units_one_print() {
    let (a, b) = (def_a(), def_b());
    assert_eq!(
        a.id,
        MarkerId {
            crate_name: "rpm_a",
            name: "Rpm"
        }
    );
    assert_eq!(
        b.id,
        MarkerId {
            crate_name: "rpm_b",
            name: "Rpm"
        }
    );
    assert_ne!(a, b);
    assert_eq!(
        UnitDef { id: a.id, ..b },
        a,
        "the same definition otherwise"
    );
    assert_ne!(*rpm_a::Rpm::EXPR, *rpm_b::Rpm::EXPR);
    assert_eq!(rpm_a::Rpm::EXPR.to_string(), "rpm");
    assert_eq!(rpm_b::Rpm::EXPR.to_string(), "rpm");
}

#[test]
fn tables_list_by_identity() {
    let (a, b) = (def_a(), def_b());
    let tables = both();
    assert_eq!(tables.unit(a.id), Some(a));
    assert_eq!(tables.unit(b.id), Some(b));
    assert_eq!(rpm_a::UNITS.unit(b.id), None);
    assert_eq!(units::TABLE.unit(a.id), None);
    let volt = units::Volt::EXPR.def().unwrap();
    assert_eq!(tables.unit(volt.id), Some(*volt));
    // a unit of a tree is a table of its units
    assert_eq!(<Per<rpm_b::Rpm, Second>>::EXPR.unit(b.id), Some(b));
    assert_eq!(<Per<rpm_b::Rpm, Second>>::EXPR.unit(a.id), None);
}

#[test]
fn refs_resolve_through_a_table_that_lists_both() {
    let (a, b) = (def_a(), def_b());
    let tables = both();
    let (ra, rb) = (SIExpr::Ref(a.id), SIExpr::Ref(b.id));
    for (r, def) in [(&ra, a), (&rb, b)] {
        assert_eq!(r.dim(&tables), Some(def.dim.into()));
        assert_eq!(r.magnitude(&tables), Some(def.scale));
        assert_eq!(r.symbol(&tables).to_string(), "rpm");
        assert!(r.same(&SIExpr::Def(def), &tables));
    }
    assert!(!ra.same(&rb, &tables), "one print, two units");
    assert!(!ra.same(&SIExpr::Def(b), &tables));
    assert_eq!(ra.name().to_string(), "Rpm");
    // a Ref is the unit of the UnitExpr with its identity
    assert_eq!(ra, *rpm_a::Rpm::EXPR);
    assert_ne!(ra, *rpm_b::Rpm::EXPR);
    assert_eq!(SIExpr::Def(a), *rpm_a::Rpm::EXPR);
    assert_ne!(SIExpr::Def(b), *rpm_a::Rpm::EXPR);
}

#[test]
fn an_unlisted_ref_is_known_by_name_only() {
    let only_a: [&dyn UnitTable; 2] = [&units::TABLE, &rpm_a::UNITS];
    let rb = SIExpr::Ref(def_b().id);
    assert_eq!(rb.dim(&only_a), None);
    assert_eq!(rb.magnitude(&only_a), None);
    assert_eq!(rb.factor(&only_a), None);
    assert_eq!(rb.symbol(&only_a).to_string(), "Rpm");
    assert_eq!(rb.to_string(), "Rpm", "Display reads units::TABLE");
    assert!(rb.same(&rb, &only_a), "the same reference");
    assert!(!rb.same(&SIExpr::Def(def_b()), &only_a));
    // a compound unit over it has no dimension either
    let per = SIExprOwned::Div(
        Box::new(SIExprOwned::Ref(def_b().id.into())),
        Box::new(SIExprOwned::from(Second::EXPR)),
    );
    assert_eq!(per.dim(&only_a), None);
    assert_eq!(per.symbol(&only_a).to_string(), "Rpm/s");
    assert_eq!(per.symbol(&both()).to_string(), "rpm/s");
}

#[test]
fn an_unlisted_def_reads_alone() {
    let db = SIExpr::Def(def_b());
    assert_eq!(db.dim(&()), Some(def_b().dim.into()));
    assert_eq!(db.magnitude(&()), Some(def_b().scale));
    assert_eq!(db.to_string(), "rpm");
    assert_eq!(db.known(), None, "not a unit of exact_units");
}

#[test]
fn compact_turns_listed_defs_into_refs() {
    let (a, b) = (def_a(), def_b());
    let only_a: [&dyn UnitTable; 2] = [&units::TABLE, &rpm_a::UNITS];
    // rpm_a's per second and rpm_b's per second
    let speed_a = SIExpr::of::<Per<rpm_a::Rpm, Second>>();
    let speed_b = SIExpr::of::<Per<rpm_b::Rpm, Second>>();

    let mut arena = [const { None }; 2];
    let ca = speed_a.compact_in(&only_a, &mut arena).unwrap();
    let SIExpr::Div(num, den) = &ca else {
        panic!("{ca:?}")
    };
    assert_eq!(num.read().unwrap(), SIExpr::Ref(a.id));
    assert_eq!(
        den.read().unwrap(),
        SIExpr::Ref(Second::EXPR.def().unwrap().id)
    );
    assert!(ca.same(&speed_a, &only_a));
    assert_eq!(ca, *<Per<rpm_a::Rpm, Second>>::EXPR);

    let mut arena = [const { None }; 2];
    let cb = speed_b.compact_in(&only_a, &mut arena).unwrap();
    let SIExpr::Div(num, _) = &cb else {
        panic!("{cb:?}")
    };
    assert_eq!(num.read().unwrap(), SIExpr::Def(b), "rpm_b isn't listed");
    assert!(cb.same(&speed_b, &only_a));
    assert_eq!(cb.dim(&only_a), speed_b.dim(&()));
    assert_eq!(cb.magnitude(&only_a), speed_b.magnitude(&()));

    // owned: the same
    let owned = SIExprOwned::from(<Per<rpm_a::Rpm, Second>>::EXPR);
    let compact = owned.compact(&only_a).unwrap();
    assert_eq!(SIExprOwned::from(&ca), compact);
    assert_eq!(compact.resolve(&only_a).unwrap(), owned);

    // nothing to compact: no arena needed
    let none = speed_b.compact_in(&units::TABLE, &mut []);
    assert!(
        matches!(none, Err(CompactError::ArenaFull)),
        "Second is listed: {none:?}"
    );
    let alone = SIExpr::Def(b).compact_in(&units::TABLE, &mut []).unwrap();
    assert_eq!(alone, SIExpr::Def(b));
}

#[test]
fn compact_refuses_another_definition_under_a_listed_identity() {
    let a = def_a();
    let impostor = UnitDef {
        scale: Magnitude::ONE,
        ..a
    };
    let e = SIExpr::Def(impostor);
    assert_eq!(
        e.compact_in(&rpm_a::UNITS, &mut []),
        Err(CompactError::Conflict(a.id))
    );
    let owned = SIExprOwned::Div(
        Box::new(SIExprOwned::from(Second::EXPR)),
        Box::new(SIExprOwned::Def(impostor.into())),
    );
    assert_eq!(owned.compact(&both()), Err(CompactError::Conflict(a.id)));
    assert_eq!(
        CompactError::Conflict(a.id).to_string(),
        "the table lists rpm_a::Rpm with another definition"
    );
}

#[test]
fn refs_on_the_wire() {
    let tables = both();
    let owned = SIExprOwned::Div(
        Box::new(SIExprOwned::from(rpm_a::Rpm::EXPR)),
        Box::new(SIExprOwned::from(rpm_b::Rpm::EXPR)),
    );
    let compact = owned.compact(&tables).unwrap();
    let full = owned.to_ww_bytes_owned().unwrap();
    let bytes = compact.to_ww_bytes_owned().unwrap();
    assert!(bytes.len() < full.len(), "{} < {}", bytes.len(), full.len());

    let back = SIExpr::from_ww_bytes(&bytes).unwrap();
    assert_eq!(back.symbol(&tables).to_string(), "rpm/rpm");
    assert_eq!(back.dim(&tables), Some(exact_units::Dim::ONE.into()));
    assert_eq!(back.magnitude(&tables), Some(Magnitude::ONE));
    assert!(back.same(&SIExpr::from_ww_bytes(&full).unwrap(), &tables));
    // not a ratio of one unit by itself: two units
    assert!(!back.same(&SIExpr::of::<Per<rpm_a::Rpm, rpm_a::Rpm>>(), &tables));

    let mut arena = [const { None }; 2];
    let resolved = back.resolve_in(&tables, &mut arena).unwrap();
    assert_eq!(SIExprOwned::from(&resolved), owned);
    let listed: &[&UnitExpr] = rpm_a::UNITS;
    assert_eq!(listed, [rpm_a::Rpm::EXPR, rpm_a::Pixel::EXPR]);
}

/// A crate's own base takes the path of this crate's `Radian`: a `base` line declares it ([Unit::BASE]), its
/// dimension names it by crate and marker, a compound keeps it apart from the true bases, `strip` drops it, and it
/// goes through the wire and a table. Two crates' `Pixel` are two bases.
#[test]
fn a_crate_declares_a_base_as_this_crate_declares_the_radian() {
    use exact_units::{BaseDef, Dim, DimOwned, Exp, units::Radian};

    let (pa, pb) = (rpm_a::Pixel::BASE.unwrap(), rpm_b::Pixel::BASE.unwrap());
    let id_a = MarkerId {
        crate_name: "rpm_a",
        name: "Pixel",
    };
    assert_eq!(pa, BaseDef { id: id_a });
    assert_eq!(Radian::BASE.unwrap().id, MarkerId::RADIAN);
    assert_eq!(rpm_a::Pixel::DIM, pa.dim());
    assert_eq!(Radian::DIM, Radian::BASE.unwrap().dim());
    assert_eq!(rpm_a::Rpm::BASE, None, "a unit of a base declares none");
    assert_ne!(pa, pb);
    assert_ne!(rpm_a::Pixel::DIM, rpm_b::Pixel::DIM);
    assert_ne!(rpm_a::Pixel::DIM, Dim::ONE);
    assert_eq!(rpm_a::Pixel::DIM.to_string(), "Pixel");

    // pixels per second, as radians per second: not a hertz, the base stripped it is
    type PxRate = Per<rpm_a::Pixel, Second>;
    type RadRate = Per<Radian, Second>;
    let hz = <units::Hertz as Unit>::DIM;
    for (d, base) in [(PxRate::DIM, id_a), (RadRate::DIM, MarkerId::RADIAN)] {
        assert!(!d.same(hz), "{d}");
        assert_eq!(d.exp_of(base), Exp::ONE);
        assert_eq!(d.strip(base), hz);
        assert_eq!(d.true_part(), hz);
    }
    assert_eq!(PxRate::DIM.to_string(), "s⁻¹·Pixel");
    assert_eq!(RadRate::DIM.to_string(), "s⁻¹·rad");
    assert_ne!(PxRate::DIM, RadRate::DIM);
    assert_ne!(PxRate::DIM, <Per<rpm_b::Pixel, Second>>::DIM);
    let two = <units::Times<PxRate, RadRate>>::DIM;
    assert_eq!(two.to_string(), "s⁻²·rad·Pixel");

    // the wire, and a run-time unit through the tables that list it
    let mut buf = [0u8; 128];
    let bytes = two.to_ww_bytes(&mut buf).unwrap().to_vec();
    assert_eq!(Dim::from_ww_bytes(&bytes).unwrap(), two);
    assert_eq!(DimOwned::from_ww_bytes_owned(&bytes).unwrap(), two);
    let px = SIExpr::Ref(id_a);
    assert_eq!(px.dim(&both()), Some(rpm_a::Pixel::DIM.into()));
    let only_b: [&dyn UnitTable; 2] = [&units::TABLE, &rpm_b::UNITS];
    assert_eq!(px.dim(&only_b), None);
}
