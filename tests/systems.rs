//! Natural and Gaussian against SI: the crossings UNIT-9 promises, each with its exact factor, and the bridge read
//! at run time against the one `const` code computes, for every row of Gaussian and every base of both rules.
#![cfg(feature = "systems")]
#![expect(clippy::expect_used, reason = "a test fails by panicking")]

use exact_units::constants::{Ampere1948, C, Codata2018, Codata2022, E};
use exact_units::prefix::{Femto, Kilo, Mega, Milli, One, Peta};
use exact_units::quantities::{self as si, Frequency};
use exact_units::system::{Side, bridge, const_bridge, def_list};
use exact_units::systems::gaussian::{
    self, Dyne, Erg, Gauss, GaussianCentimeter, GaussianSecond, Maxwell, Oersted, S, Statcoulomb,
    Statvolt, quantities as g,
};
use exact_units::systems::natural::{HBAR, NaturalElectronVolt};
use exact_units::systems::{Gaussian, Natural, Si};
use exact_units::units::{
    Ampere, Coulomb, ElectricDisplacement, Gram, Hertz, Joule, MagneticFieldStrength, Meter,
    Newton, Per, Radian, Second, Tesla, Times, Torque, Volt, Wavenumber, Weber,
};
use exact_units::{
    BridgeError, Dim, Fixed, Magnitude, MarkerId, QuantityExpr, QuantityRef, QuantitySpec,
    Rounding, System, SystemRef, Unit, UnitTable,
};

/// The side of a unit type, as `to_system` builds it (no product of rows here).
fn side<U: Unit>() -> Side<'static> {
    Side {
        dim: U::DIM,
        quantity: U::QUANTITY,
        rows_only_leaf: false,
    }
}

/// The factor of 1 coherent unit of `A`'s quantity in coherent units of `B`'s, `const` and at run time, which must
/// agree.
fn factor<A: Unit, B: Unit>() -> Result<Magnitude, BridgeError> {
    let c = const_bridge(side::<A>(), side::<B>());
    let r = bridge(side::<A>(), side::<B>(), &());
    assert_eq!(
        c,
        r,
        "const and run-time bridges of {} to {}",
        A::EXPR,
        B::EXPR
    );
    c
}

#[test]
fn natural_mass_from_an_energy() {
    // 1 eV as a mass is e/c² kg, exactly
    assert_eq!(factor::<NaturalElectronVolt, Gram>(), Ok(E.div(C.mul(C))));
    // the electron: 0.51099895 MeV is 9.1093837…·10⁻³¹ kg
    let m_e = Fixed::<NaturalElectronVolt, Mega, f64>::try_new(0.510_998_95).expect("finite");
    let kg = m_e
        .to_system::<Si>()
        .convert_rounded::<Gram, Kilo, f64>(Rounding::NearestEven)
        .expect("fits");
    assert!((kg.get() / 9.109_383_7e-31 - 1.0).abs() < 1e-8, "{kg:?}");
    // and as an energy it is SI's electronvolt, exactly
    assert_eq!(factor::<NaturalElectronVolt, Joule>(), Ok(E));
}

#[test]
fn natural_energy_of_an_inverse_length() {
    // 1 m⁻¹ is ħc/e eV
    assert_eq!(
        factor::<Wavenumber, NaturalElectronVolt>(),
        Ok(HBAR.mul(C).div(E))
    );
    // 1/fm is 197.3269804… MeV
    let k = Fixed::<Wavenumber, Peta, f64>::try_new(1.0).expect("finite");
    let mev = k
        .to_system::<Natural>()
        .convert_rounded::<NaturalElectronVolt, Mega, f64>(Rounding::NearestEven)
        .expect("fits");
    assert!((mev.get() - 197.326_980_4).abs() < 1e-7, "{mev:?}");
    // and back, by the target's quantity: 197.3269804 MeV as an inverse length is 1/fm
    let back = mev
        .to_system::<Si>()
        .convert_rounded::<Wavenumber, Peta, f64>(Rounding::NearestEven)
        .expect("fits");
    assert!((back.get() - 1.0).abs() < 1e-12, "{back:?}");
    let _ = Fixed::<Meter, Femto, i32>::new(1);
}

#[test]
fn natural_keeps_the_radian_and_refuses_a_frequency() {
    // ω in rad/s becomes rad·eV by ħ/e, its radian intact: not an energy until it is stripped
    type Omega = Per<Radian, Second>;
    type RadEv = Times<Radian, NaturalElectronVolt>;
    assert_eq!(factor::<Omega, RadEv>(), Ok(HBAR.div(E)));
    assert_ne!(RadEv::DIM, NaturalElectronVolt::DIM);
    assert_eq!(RadEv::DIM.strip(MarkerId::RADIAN), NaturalElectronVolt::DIM);
    assert_eq!(
        factor::<Omega, NaturalElectronVolt>(),
        Err(BridgeError::Dimension)
    );
    // Hertz has no rule: E = hf and E = ħω differ by 2π, and an anonymous s⁻¹ is a frequency too
    assert_eq!(
        factor::<Hertz, NaturalElectronVolt>(),
        Err(BridgeError::NoRule)
    );
    assert_eq!(
        factor::<Per<exact_units::units::Ratio, Second>, NaturalElectronVolt>(),
        Err(BridgeError::NoRule)
    );
    assert!(QuantityExpr::of::<Frequency>().is_derived() || Hertz::QUANTITY != Omega::QUANTITY);
}

#[test]
fn natural_dimension_one_names_nothing() {
    // E·L is dimension one in Natural: a product with it keeps the other's quantity, which is Derived there
    type EL = Times<NaturalElectronVolt, Per<exact_units::units::Ratio, NaturalElectronVolt>>;
    assert!(EL::DIM.is_one());
    type Product = Times<NaturalElectronVolt, EL>;
    let q = NaturalElectronVolt::QUANTITY.mul(
        NaturalElectronVolt::DIM,
        EL::QUANTITY,
        EL::DIM,
        Product::DIM,
    );
    assert_eq!(q, QuantityExpr::Derived(NaturalElectronVolt::DIM));
}

#[test]
fn torque_crosses_with_its_radian() {
    type EvPerRad = Per<NaturalElectronVolt, Radian>;
    assert_eq!(factor::<Torque, EvPerRad>(), Ok(Magnitude::ONE.div(E)));
    type ErgPerRad = Per<Erg, Radian>;
    assert_eq!(factor::<Torque, ErgPerRad>(), Ok(Magnitude::pow10(7)));
    let t = Fixed::<Torque, Milli, i64>::new(3);
    assert_eq!(
        t.to_system::<Gaussian>().convert::<ErgPerRad, One, i64>(),
        Some(Fixed::new(30_000))
    );
}

#[test]
fn gaussian_under_the_1948_ampere() {
    let one =
        |m: Result<Magnitude, BridgeError>| m.expect("a bridge").resolve::<Ampere1948>().value;
    // 1 G = 10⁻⁴ T, 1 Oe = 1000/(4π) A/m, 1 statC = 1/2997924580 C, exactly
    assert_eq!(one(factor::<Gauss, Tesla>()), Magnitude::pow10(-4));
    assert_eq!(
        one(factor::<Oersted, MagneticFieldStrength>()),
        Magnitude::pow10(3).div(Magnitude::new(4, 1, 0, 0, 1))
    );
    assert_eq!(
        one(factor::<Statcoulomb, Coulomb>()),
        Magnitude::ratio(1, 2_997_924_580)
    );
    assert_eq!(one(factor::<Maxwell, Weber>()), Magnitude::pow10(-8));
    assert_eq!(
        one(factor::<Statvolt, Volt>()),
        Magnitude::ratio(299_792_458, 1).mul(Magnitude::pow10(-6))
    );
    // S is exactly 1
    assert_eq!(S.resolve::<Ampere1948>().value, Magnitude::ONE);
    let b = Fixed::<Gauss, One, i32>::new(10_000);
    assert_eq!(
        b.to_system::<Si>()
            .convert_with::<Ampere1948, Tesla, One, i32>(),
        Some(Fixed::new(1))
    );
}

#[test]
fn gaussian_symbolically() {
    let f = |m: Result<Magnitude, BridgeError>| m.expect("a bridge");
    // 1 Oe is (1 G)/μ₀, with no set
    assert_eq!(
        f(factor::<Oersted, MagneticFieldStrength>()),
        f(factor::<Gauss, Tesla>()).div(Magnitude::MU0)
    );
    // P is 4π·D: the factor of P's row is 4π times D's
    type D = Fixed<ElectricDisplacement, One, i32>;
    let _ = D::new(0);
    let p = g::GaussianPolarization::DEF;
    let d = g::GaussianDisplacement::DEF;
    let row = |q: &exact_units::QuantityDef<'_>| {
        let mut out = None;
        for r in Gaussian::DEF.rows() {
            if r.here.id() == q.id {
                out = Some(r.factor);
            }
        }
        out.expect("a row")
    };
    assert_eq!(row(p), row(d).mul(Magnitude::new(4, 1, 0, 0, 1)));
    // under every set the factors differ from the 1948 table by S alone
    for s in [
        S.resolve::<Codata2018>().value.to_f64(),
        S.resolve::<Codata2022>().value.to_f64(),
    ] {
        assert!((s - 1.0).abs() < 1e-9, "{s}");
    }
}

#[test]
fn gaussian_needs_a_set_for_mu0() {
    // without a set the factor keeps √μ₀: no number, so `convert` would not compile; the factor says so
    assert!(f_has_mu0(factor::<Gauss, Tesla>()));
    assert!(!f_has_mu0(factor::<Erg, Joule>()));
}

fn f_has_mu0(m: Result<Magnitude, BridgeError>) -> bool {
    m.expect("a bridge").has_measured()
}

#[test]
fn gaussian_mechanics_by_the_rule() {
    assert_eq!(factor::<Joule, Erg>(), Ok(Magnitude::pow10(7)));
    assert_eq!(factor::<Newton, Dyne>(), Ok(Magnitude::pow10(5)));
    assert_eq!(
        factor::<Meter, GaussianCentimeter>(),
        Ok(Magnitude::pow10(2))
    );
    assert_eq!(factor::<Second, GaussianSecond>(), Ok(Magnitude::ONE));
    // the ampere has no image: a current crosses by its row only
    assert_eq!(
        factor::<Ampere, Times<GaussianSecond, GaussianCentimeter>>(),
        Err(BridgeError::RowMismatch)
    );
}

#[test]
fn gaussian_b_and_h_are_kinds() {
    assert_eq!(Gauss::DIM, Oersted::DIM);
    assert_eq!(
        Gauss::QUANTITY.converts(Oersted::QUANTITY, &()),
        Some(false)
    );
    // H's row takes an oersted, not a gauss
    assert_eq!(
        factor::<Gauss, MagneticFieldStrength>(),
        Err(BridgeError::RowMismatch)
    );
    // a plain A/m is no H: it crosses once named
    type Am = Per<Ampere, Meter>;
    assert_eq!(factor::<Am, Oersted>(), Err(BridgeError::RowMismatch));
}

#[test]
fn products_of_rows_dont_take_the_rule() {
    // SI's E×H is W/m²: as a product of units with the ampere it doesn't cross by the rule
    type EH = Times<Per<Volt, Meter>, Per<Ampere, Meter>>;
    let s = Side {
        rows_only_leaf: exact_units::system::rows_only_leaves(EH::EXPR, Gaussian::DEF)
            .expect("const data"),
        ..side::<EH>()
    };
    assert!(s.rows_only_leaf);
    type Flux = Per<Erg, Times<GaussianSecond, Times<GaussianCentimeter, GaussianCentimeter>>>;
    assert_eq!(
        const_bridge(s, side::<Flux>()),
        Err(BridgeError::ProductOfRows)
    );
    // nor Gaussian's B·H, which is not SI's B·H by 4π
    type BH = Times<Gauss, Oersted>;
    assert_eq!(
        exact_units::system::rows_only_leaves(BH::EXPR, Gaussian::DEF),
        Some(true)
    );
    // a named mechanical unit does
    assert_eq!(
        exact_units::system::rows_only_leaves(<Per<Joule, Second>>::EXPR, Gaussian::DEF),
        Some(false)
    );
}

#[test]
fn every_row_const_and_at_run_time() {
    let mut n = 0;
    for r in Gaussian::DEF.rows() {
        let (QuantityRef::Def(si), QuantityRef::Def(here)) = (r.si, r.here) else {
            panic!("rows of const data hold their definitions")
        };
        let si_side = Side {
            dim: si.dim,
            quantity: QuantityExpr::Spec(r.si),
            rows_only_leaf: false,
        };
        let x_side = Side {
            dim: here.dim,
            quantity: QuantityExpr::Spec(r.here),
            rows_only_leaf: false,
        };
        // by id through a table, as data read from an API model
        let by_id = |q: QuantityRef<'static>, d: Dim<'static>| Side {
            dim: d,
            quantity: QuantityExpr::Spec(QuantityRef::Id(q.id())),
            rows_only_leaf: false,
        };
        let table = TABLE;
        assert_eq!(const_bridge(x_side, si_side), Ok(r.factor));
        assert_eq!(bridge(x_side, si_side, &()), Ok(r.factor));
        assert_eq!(
            bridge(by_id(r.here, here.dim), by_id(r.si, si.dim), &table),
            Ok(r.factor),
            "{}",
            here.id
        );
        assert_eq!(
            const_bridge(si_side, x_side),
            Ok(Magnitude::ONE.div(r.factor))
        );
        n += 1;
    }
    assert_eq!(n, 13);
}

/// This crate's quantities, SI's and Gaussian's, by id.
struct Table;

const TABLE: Table = Table;

impl UnitTable for Table {
    fn unit(&self, id: MarkerId<'_>) -> Option<exact_units::UnitDef<'_>> {
        exact_units::units::TABLE.unit(id)
    }

    fn quantity(&self, id: MarkerId<'_>) -> Option<exact_units::QuantityDef<'_>> {
        si::ALL
            .iter()
            .chain(g::ALL)
            .find(|q| q.id == id)
            .map(|q| **q)
    }
}

#[test]
fn systems_by_id() {
    assert!(SystemRef::of_id(Gaussian::ID).def().is_some());
    assert_eq!(SystemRef::of_id(Natural::ID).base_count(), 5);
    assert_eq!(Gauss::DIM.system(), SystemRef::Def(Gaussian::DEF));
    assert_eq!(format!("{}", Gauss::DIM), "g¹ᐟ²·cm⁻¹ᐟ²·s⁻¹");
    assert_eq!(format!("{}", NaturalElectronVolt::DIM), "eV");
    assert_eq!(
        format!("{}", <Per<Radian, NaturalElectronVolt>>::DIM),
        "eV⁻¹·rad"
    );
    // a ratio and an angle are in every system
    assert_eq!(<Per<Gauss, Gauss>>::DIM, Dim::ONE);
    assert_eq!(Radian::DIM, <Per<Times<Radian, Erg>, Erg>>::DIM);
    // two systems with exponents differ
    assert_ne!(GaussianSecond::DIM, Second::DIM);
    let _ = (gaussian::ALL, def_list::<u8>(&[]));
}

/// A dimension and a system on the wire: a system known by its id only keeps the count of exponents it was read
/// with, a dimension without true exponents is written as SI's, and a system's definition becomes owned whole.
#[cfg(all(feature = "std", feature = "shrink_wrap"))]
mod wire {
    use exact_units::systems::Gaussian;
    use exact_units::systems::gaussian::GaussianCentimeter;
    use exact_units::{
        Dim, DimOwned, DimWire, Exp, MarkerId, System, SystemDefOwned, SystemRef, Unit,
    };
    use shrink_wrap::prelude::*;

    const ELSEWHERE: MarkerId<'static> = MarkerId {
        crate_name: "other_crate",
        name: "Planck",
    };

    #[test]
    fn unknown_system_keeps_its_count() {
        let exps = [
            Exp::ONE,
            Exp::ZERO,
            Exp::from_halves(-1),
            Exp::ZERO,
            Exp::ZERO,
        ];
        let wire = DimWire {
            system: SystemRef::Id(ELSEWHERE),
            exps: RefVec::Slice { slice: &exps },
            bases: RefVec::Slice { slice: &[] },
        };
        let bytes = wire.to_ww_bytes(&mut [0u8; 64]).expect("fits").to_vec();
        let d = Dim::from_ww_bytes(&bytes).expect("an unknown system of five bases");
        assert_eq!(d.true_count(), 5);
        assert_eq!(d.to_string(), "b0·b2⁻¹ᐟ²");
        assert_eq!(d.to_ww_bytes(&mut [0u8; 64]).expect("fits"), bytes);
        let owned = DimOwned::from_ww_bytes_owned(&bytes).expect("as borrowed");
        assert_eq!(owned.to_ww_bytes_owned().expect("fits"), bytes);
        assert_eq!(DimOwned::from(d).to_ww_bytes_owned().expect("fits"), bytes);
        // a product keeps it too
        let sq = owned.checked_mul(&owned).expect("in range");
        assert_eq!(sq.as_dim().true_count(), 5);
        // more than a Dim holds is refused
        let eight = [Exp::ONE; 8];
        let wire = DimWire {
            exps: RefVec::Slice { slice: &eight },
            ..wire
        };
        let bytes = wire.to_ww_bytes(&mut [0u8; 64]).expect("fits").to_vec();
        assert!(Dim::from_ww_bytes(&bytes).is_err());
        assert!(DimOwned::from_ww_bytes_owned(&bytes).is_err());
    }

    #[test]
    fn a_ratio_is_written_as_si() {
        let cm = DimOwned::from(GaussianCentimeter::DIM);
        let ratio = cm.checked_div(&cm).expect("in range");
        assert_eq!(ratio, DimOwned::from(Dim::ONE));
        let si = Dim::ONE.to_ww_bytes(&mut [0u8; 64]).expect("fits").to_vec();
        assert_eq!(ratio.to_ww_bytes_owned().expect("fits"), si);
        let zero = cm.checked_pow(0).expect("in range");
        assert_eq!(zero, ratio);
        // the borrowed quotient names Gaussian, and is written as SI's all the same
        let (a, b) = (GaussianCentimeter::DIM, GaussianCentimeter::DIM);
        let q = a.checked_div(&b).expect("in range");
        assert!(q.system().same(&SystemRef::Id(Gaussian::ID)));
        assert_eq!(q.to_ww_bytes(&mut [0u8; 64]).expect("fits"), si);
        assert_eq!(DimOwned::from(q), ratio);
        // Gaussian's id with no true exponent is a second encoding of one dimension: refused
        let zeros = [Exp::ZERO; 6];
        let wire = DimWire {
            system: SystemRef::Id(Gaussian::ID),
            exps: RefVec::Slice { slice: &zeros },
            bases: RefVec::Slice { slice: &[] },
        };
        let bytes = wire.to_ww_bytes(&mut [0u8; 64]).expect("fits").to_vec();
        assert!(Dim::from_ww_bytes(&bytes).is_err());
        assert!(DimOwned::from_ww_bytes_owned(&bytes).is_err());
    }

    #[test]
    fn system_def_owned() {
        let owned = SystemDefOwned::try_from(*Gaussian::DEF).expect("const data decodes");
        let bytes = Gaussian::DEF
            .to_ww_bytes(&mut [0u8; 4096])
            .expect("fits")
            .to_vec();
        assert_eq!(owned.to_ww_bytes_owned().expect("fits"), bytes);
        assert_eq!(owned.bases.len(), Gaussian::BASES.len());
        // and from bytes, borrowed
        let read = exact_units::SystemDef::from_ww_bytes(&bytes).expect("decodes");
        assert_eq!(SystemDefOwned::try_from(read).expect("decodes"), owned);
    }
}

#[test]
fn within_one_system_a_crossing_checks_as_convert() {
    // the same system on both sides: no factor, but the dimension and the quantity are checked
    assert_eq!(factor::<Meter, Second>(), Err(BridgeError::Dimension));
    assert_eq!(factor::<Gauss, Oersted>(), Err(BridgeError::Quantity));
    assert_eq!(factor::<Erg, Joule>().map(|_| ()), Ok(()));
    // a ratio counts in the other side's system: no length, no energy
    type R = exact_units::units::Ratio;
    assert_eq!(factor::<R, Meter>(), Err(BridgeError::Dimension));
    assert_eq!(
        factor::<NaturalElectronVolt, R>(),
        Err(BridgeError::Dimension)
    );
    // a ratio named in Natural is one there: 1 J·s is 1/ħ, ħ set to one
    let in_natural = |s: Side<'static>| Side {
        dim: Dim::in_system(Natural::ID, Natural::BASES, &[], &[]),
        ..s
    };
    let to = in_natural(side::<R>());
    type Action = Times<Joule, Second>;
    assert_eq!(
        const_bridge(side::<Action>(), to),
        Ok(Magnitude::ONE.div(HBAR))
    );
    assert_eq!(
        bridge(side::<Action>(), to, &()),
        Ok(Magnitude::ONE.div(HBAR))
    );
}
