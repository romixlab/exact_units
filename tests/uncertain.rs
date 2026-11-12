//! `Uncertain` (IO-11): a value and its standard uncertainty as the data tables print them, each form read exactly
//! or refused by name, and written back exactly.
#![cfg(all(feature = "time", feature = "dimensionless"))]
#![expect(
    clippy::unwrap_used,
    clippy::panic,
    reason = "a test fails by panicking"
)]

use exact_units::units::{Hertz, Ratio, Second};
use exact_units::{Exact, ExactNum, Fraction, Prefix, Uncertain};

fn read<U: exact_units::Unit>(s: &str) -> Uncertain<U> {
    s.parse().unwrap_or_else(|e| panic!("{s:?}: {e}"))
}

fn refused<U: exact_units::Unit>(s: &str, why: &str) {
    match s.parse::<Uncertain<U>>() {
        Ok(x) => panic!("{s:?} read as {x}"),
        Err(e) => assert!(e.to_string().contains(why), "{s:?}: {e}"),
    }
}

#[test]
fn concise_form() {
    // the uncertainty in units of the value's last digit
    let t = read::<Second>("1600(7) s");
    assert_eq!((*t.value(), *t.u()), (Exact::int(1600), Exact::int(7)));
    let t = read::<Second>("1.600(7) ks");
    assert_eq!((*t.value(), *t.u()), (Exact::int(1600), Exact::int(7)));
    let t = read::<Second>("1600.0(15) s");
    assert_eq!(*t.u(), Exact::new(3, 2));
    // CODATA's grouped digits and exponent after the parentheses (G, as 2018 and 2022 print it)
    let g = read::<Ratio>("6.674 30(15)e-11");
    assert_eq!(*g.value(), Exact::new(667_430, 10_u128.pow(16)));
    assert_eq!(*g.u(), Exact::new(15, 10_u128.pow(16)));
    assert_eq!(read::<Ratio>("6.67430(15)e-11"), g);
    assert_eq!(read::<Ratio>("6.674\u{2009}30(15)e-11"), g);
    // the GUM's other form, the uncertainty in the value's unit (§7.2.2)
    let m = read::<Second>("100.021 47(0.000 35) s");
    assert_eq!(*m.u(), Exact::new(35, 100_000));
    assert_eq!(read::<Second>("100.02147(35) s"), m);
    // a repetend or a fraction as the uncertainty is in the value's unit too
    let third = read::<Hertz>("333.[3](0.[3]) Hz");
    assert_eq!(
        (*third.value(), *third.u()),
        (Exact::new(1000, 3), Exact::new(1, 3))
    );
    assert_eq!(read::<Hertz>("1000/3(1/3) Hz"), third);
    // an exponent's e is not a unit's: 5(1) Hz, and the e of a unit after the parentheses
    assert_eq!(*read::<Second>("5(1)e3 s").value(), Exact::int(5000));
    assert_eq!(*read::<Second>("-0.25(5) s").value(), Exact::new(-1, 4));
}

#[test]
fn concise_refusals() {
    refused::<Second>("1600 s", "no uncertainty");
    refused::<Second>("1600(7) kg", "not in s");
    refused::<Second>("16#00(7) s", "estimated");
    refused::<Second>("1600(7#) s", "estimated");
    refused::<Second>(">1600(7) s", "limit");
    refused::<Second>("~1600(7) s", "limit");
    refused::<Second>("1600(+4-3) s", "asymmetric");
    refused::<Second>("1600(-7) s", "asymmetric");
    refused::<Second>("333.[3](3) s", "last digit");
    refused::<Second>("1000/3(3) s", "last digit");
    refused::<Second>("1600(7 s", "not a number");
    refused::<Second>("(7) s", "not a number");
    // review of 1bb9530: each refusal by the position it is at
    refused::<Second>("1e3(1)e3 s", "not a number");
    refused::<Second>("1600(15e-1) s", "not a number");
    refused::<Second>("(<5 ± 1) s", "limit");
    refused::<Second>("5(<1) s", "limit");
    refused::<Second>("1600 (7) s", "a space between");
    refused::<Second>("1600 s (7)", "no uncertainty");
    refused::<Ratio>("6.674 30e-11 m³/(kg·s²)", "no uncertainty");
    // digits are grouped before the exponent only
    refused::<Second>("1.5e1 2(3) s", "not a number");
}

#[test]
fn written_exactly() {
    let w = |s: &str| read::<Second>(s).to_string();
    assert_eq!(w("1600(7) s"), "1.600(7) ks");
    assert_eq!(format!("{:#}", read::<Second>("1.600(7) ks")), "1600(7) s");
    // the value padded to the uncertainty's last digit, never rounded
    assert_eq!(w("1600.0(1.5) s"), "1.6000(15) ks");
    assert_eq!(w("100.02147(0.00035) s"), "100.02147(35) s");
    assert_eq!(w("2(0.25) s"), "2.00(25) s");
    assert_eq!(w("-0.25(5) s"), "-250(50) ms");
    assert_eq!(w("7(0) s"), "7(0) s");
    assert_eq!(w("0(1) ms"), "0(1) ms");
    // no common last digit: each as Exact writes it, the uncertainty with its point
    assert_eq!(
        read::<Hertz>("1000/3(1/3) Hz").to_string(),
        "333.[3](0.[3]) Hz"
    );
    assert_eq!(
        read::<Hertz>("1/3(2.0) Hz").to_string(),
        "333.[3](2000.0) mHz"
    );
    for s in [
        "333.[3](0.[3]) Hz",
        "0.[3](2.0) Hz",
        "1/7919(1.5) Hz",
        "6.674 30(15)e-11 Hz",
    ] {
        let x = read::<Hertz>(s);
        assert_eq!(read::<Hertz>(&x.to_string()), x, "{s}");
        assert_eq!(read::<Hertz>(&format!("{x:#}")), x, "{s}");
    }
}

#[test]
fn columns() {
    let t = Uncertain::<Second>::from_columns("1.600", "0.007", Prefix::Kilo).unwrap();
    assert_eq!(t, read("1600(7) s"));
    assert_eq!(
        Uncertain::<Second>::from_columns(" 2345.67 ", "2.78", Prefix::Unit).unwrap(),
        read("2345.67(278) s")
    );
    let err = |v: &str, u: &str| {
        Uncertain::<Second>::from_columns(v, u, Prefix::Kilo)
            .unwrap_err()
            .to_string()
    };
    assert!(err("-2160#", "300#").contains("estimated"));
    assert!(err("3398.84", "0.2#").contains("estimated"));
    assert!(err(">300", "5").contains("limit"));
    assert!(err("<1", "1").contains("limit"));
    assert!(err("5", "+4-3").contains("asymmetric"));
    assert!(err("5", "-3").contains("asymmetric"));
    assert!(err("stbl", "").contains("not a number"));
    assert!(err("5", "1 s").contains("not a number"));
    assert!(err("5", "2 2").contains("not a number"));
    assert!(err("5", "<1").contains("limit"));
}

#[test]
fn plus_minus() {
    let one = Fraction::ONE;
    let a = Uncertain::<Second>::parse_pm("2345.67 ± 2.78 s", &one).unwrap();
    assert_eq!(a, read("2345.67(278) s"));
    assert_eq!(
        Uncertain::<Second>::parse_pm("(2345.67 ± 2.78) s", &one),
        Ok(a)
    );
    assert_eq!(
        Uncertain::<Second>::parse_pm("2345.67 +/- 2.78 s", &one),
        Ok(a)
    );
    assert_eq!(a.pm(&one).unwrap().to_string(), "(2.34567 ± 0.00278) ks");
    // k = 2: the standard uncertainty is half the half-width
    let two = Fraction::int(2);
    let b = Uncertain::<Second>::parse_pm("(10 ± 0.5) ms", &two).unwrap();
    assert_eq!(*b.u(), Exact::new(1, 4000));
    assert_eq!(format!("{:#}", b.pm(&two).unwrap()), "(0.01 ± 0.0005) s");
    let err = |s: &str, k: Fraction| {
        Uncertain::<Second>::parse_pm(s, &k)
            .unwrap_err()
            .to_string()
    };
    assert!(err("10 ± 1 s", Fraction::ZERO).contains("coverage"));
    assert!(err("10 ± 1 s", Fraction::int(-1)).contains("coverage"));
    assert!(err("10(1) s", one).contains("no uncertainty"));
    assert!(err("10# ± 1 s", one).contains("estimated"));
    assert!(err("10 ± -1 s", one).contains("asymmetric"));
    assert!(err("10 ± 1 kg", one).contains("not in s"));
    // review of 1bb9530: a space ends the uncertainty after ±, a limit at either place
    assert!(err("1 ± 2 2 s", one).contains("not a number"));
    assert!(err("(1 ± 2 2) s", one).contains("not a number"));
    assert!(err("(<5 ± 1) s", one).contains("limit"));
    assert!(err("5 ± <1 s", one).contains("limit"));
    assert_eq!(
        Uncertain::<Second>::parse_pm("1 000 ± 2 s", &one),
        Ok(read("1000(2) s"))
    );
    assert!(a.pm(&Fraction::ZERO).is_none());
}

#[test]
fn values() {
    let t = read::<Second>("1600(7) s");
    assert_eq!(t.rel_u(), Some(Exact::new(7, 1600)));
    assert_eq!(read::<Second>("0(7) s").rel_u(), None);
    assert_eq!(t.expanded(&Fraction::int(2)), Some(Exact::int(14)));
    assert_eq!(t.expanded(&Fraction::int(-2)), None);
    assert!(!t.is_exact());
    let c = Uncertain::exact(Exact::<Hertz>::int(9_192_631_770));
    assert!(c.is_exact());
    assert_eq!(c.to_string(), "9.19263177(0) GHz");
    assert_eq!(
        Uncertain::new(Exact::<Second>::int(1), Exact::int(-1)),
        None
    );
    assert_eq!(t.into_parts(), (Exact::int(1600), Exact::int(7)));
}

#[cfg(feature = "nuclear")]
#[test]
fn nubase_and_ame() {
    use exact_units::units::{ElectronVolt, JulianYear, TropicalYearNubase2020 as Y};
    // NUBASE2020 prints 226Ra's half-life as 1.600 ky in one column and 0.007 in the next
    let t = Uncertain::<Y>::from_columns("1.600", "0.007", Prefix::Kilo).unwrap();
    assert_eq!(format!("{t:#}"), "1600(7) y (NUBASE2020)");
    assert_eq!(read::<Y>("1600(7) y (NUBASE2020)"), t);
    // the unit's own parentheses are no uncertainty
    refused::<Y>("1600 y (NUBASE2020)", "no uncertainty");
    // a change of unit is exact, the uncertainty with the value (here into seconds: a measured year, exact digits)
    let s = t.convert::<Second>().unwrap();
    assert_eq!(*s.u(), Exact::new(7 * 3_155_692_608, 100));
    assert_eq!(s.convert::<Y>(), Some(t));
    assert!(t.convert::<JulianYear>().is_some());
    // AME2020: a mass excess with its ± column (one standard deviation), and an estimated one
    let m = Uncertain::<ElectronVolt>::parse_pm("-2345.67 ± 2.78 keV", &Fraction::ONE).unwrap();
    assert_eq!(
        Uncertain::from_columns("-2345.67", "2.78", Prefix::Kilo),
        Ok(m)
    );
    assert!(Uncertain::<ElectronVolt>::from_columns("-2160#", "300#", Prefix::Kilo).is_err());
}

/// The same calls over every number an `Uncertain` holds.
fn over_any<R: ExactNum>() {
    let t: Uncertain<Second, R> = "1600(7) s".parse().unwrap();
    assert_eq!(t.to_string(), "1.600(7) ks");
    assert_eq!(format!("{t:#}"), "1600(7) s");
    let back: Uncertain<Second, R> = t.to_string().parse().unwrap();
    assert_eq!(back, t);
    assert!(t.rel_u().is_some());
    let x: Uncertain<Second, R> = "1000/3(1/3) s".parse().unwrap();
    assert_eq!(x.to_string(), "333.[3](0.[3]) s");
}

#[test]
fn every_number() {
    over_any::<Fraction>();
    #[cfg(feature = "bignum")]
    over_any::<exact_units::BigFraction>();
}

#[cfg(feature = "serde")]
#[test]
fn serde() {
    #[derive(serde::Serialize, serde::Deserialize, Debug, PartialEq)]
    struct Row {
        #[serde(with = "exact_units::serde::with_unit")]
        half_life: Uncertain<Second>,
        plain: Uncertain<Second>,
    }
    let r: Row =
        serde_json::from_str(r#"{ "half_life": "1600(7) s", "plain": ["1600", "7"] }"#).unwrap();
    assert_eq!(r.half_life, read("1600(7) s"));
    assert_eq!(r.plain, r.half_life);
    let json = serde_json::to_string(&r).unwrap();
    assert_eq!(json, r#"{"half_life":"1.600(7) ks","plain":["1600","7"]}"#);
    assert_eq!(serde_json::from_str::<Row>(&json).unwrap(), r);
    let mut buf = [0u8; 256];
    let bytes = postcard::to_slice(&r, &mut buf).unwrap();
    assert_eq!(postcard::from_bytes::<Row>(bytes).unwrap(), r);
    for (json, why) in [
        (
            r#"{ "half_life": "1600# s", "plain": ["1", "0"] }"#,
            "estimated",
        ),
        (
            r#"{ "half_life": "1600(7) s", "plain": ["1", "-1"] }"#,
            "below 0",
        ),
    ] {
        let e = serde_json::from_str::<Row>(json).unwrap_err().to_string();
        assert!(e.contains(why), "{json}: {e}");
    }
}
