//! Customary units (feature `customary`, not part of `quantities`/`std`: an API that doesn't ask for them pays
//! nothing): imperial and US units, each an exact fraction or power-of-two-times-power-of-ten of its SI base.
//!
//! `cargo run --example customary --features customary`

use exact_units::units::{GallonUs, Inch, InchOfMercury32f, MileInternational, PoundForce, Psi};
use exact_units::{Fixed, Provenance, Unit, UnitExpr, prefix::One};

fn main() {
    // an inch is exactly 127/10000 · 2¹ m (2.54 cm); a mile 12573/1000 · 2⁷ m — both exact fractions, not
    // approximations, so a mile in inches is a whole number
    let mile = Fixed::<MileInternational, One, i64>::new(1);
    let inches: Option<Fixed<Inch, One, i64>> = mile.convert();
    println!("{mile} is {inches:?}"); // Some(63360)
    assert_eq!(inches, Some(Fixed::new(63360)));

    // psi and inHg (32 °F) are both pressure (kg·m⁻¹·s⁻²): a customary unit converts to another of the same
    // dimension exactly, same as any other pair — no need for Pascal in between
    let tire = Fixed::<Psi, One, i32>::new(32);
    let in_hg: Option<Fixed<InchOfMercury32f, One, f64>> = tire.convert();
    println!("{tire} is {in_hg:?} inHg (32 °F)");
    assert!(in_hg.is_some());

    // a US gallon, a pound of force: named so no bare `Gallon` or `Pound` is ever ambiguous between US,
    // imperial and avoirdupois/force readings — see docs/customary.md
    let _ = Fixed::<GallonUs, One, f32>::new(5.0);
    let _ = Fixed::<PoundForce, One, f32>::new(150.0);

    // provenance: most units here are `Defined` (exact by the SI or by law), a few are `Conventional` — fixed
    // by a physical property at a stated condition, from a named, dated source
    match <InchOfMercury32f as Unit>::EXPR {
        UnitExpr::Named { provenance, .. } => match *provenance {
            Provenance::Conventional { source, year } => {
                println!("inHg (32 °F) is conventional, from {source} {year}");
                assert_eq!((source, year), ("NIST SP 811", 2008));
            }
            Provenance::Defined => unreachable!("inHg (32 °F) is conventional, not defined"),
        },
        _ => unreachable!("a named unit"),
    }
    match <Inch as Unit>::EXPR {
        UnitExpr::Named { provenance, .. } => assert_eq!(*provenance, Provenance::Defined),
        _ => unreachable!("a named unit"),
    }
}
