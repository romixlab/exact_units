//! Tests of [PrefixDecimal](super::PrefixDecimal): every prefix at the ends of the integers against the exact value in
//! `num-rational`, written without the kernel.

#![expect(
    clippy::as_conversions,
    clippy::cast_lossless,
    reason = "the tests cast the values they chose"
)]
#![expect(
    clippy::arithmetic_side_effects,
    reason = "the reference computes in big integers, which don't overflow"
)]

use crate::{FullScaleBits, Pow2Exp, Pow10Exp, Prefix};
use num_bigint::BigInt;
use num_rational::BigRational;
use std::format;
use std::string::{String, ToString};
use std::vec::Vec;

/// Every prefix there is.
fn every_prefix() -> Vec<Prefix> {
    let mut v = Vec::from(Prefix::ALL);
    v.extend((-63..=63).filter_map(Pow10Exp::new).map(Prefix::Pow10));
    v.extend((-63..=63).filter_map(Pow2Exp::new).map(Prefix::Pow2));
    v.extend(
        (2..=32)
            .filter_map(FullScaleBits::new)
            .map(Prefix::FullScale),
    );
    v
}

/// The scale of a prefix, exactly.
fn scale(p: Prefix) -> BigRational {
    let pow = |b: i64, e: i32| {
        let p = BigRational::from_integer(BigInt::from(b).pow(e.unsigned_abs()));
        if e < 0 { p.recip() } else { p }
    };
    match p {
        Prefix::Kibi => pow(2, 10),
        Prefix::Mebi => pow(2, 20),
        Prefix::Gibi => pow(2, 30),
        Prefix::Tebi => pow(2, 40),
        Prefix::Pow10(e) => pow(10, e.get().into()),
        Prefix::Pow2(e) => pow(2, e.get().into()),
        Prefix::FullScale(n) => pow((1i64 << n.get()) - 1, -1),
        Prefix::Unknown(_) => panic!("no scale"),
        p => pow(10, p.si_pow10().unwrap().into()),
    }
}

/// `|v|`.
fn abs(v: &BigRational) -> BigRational {
    if *v < BigRational::from_integer(0.into()) {
        -v.clone()
    } else {
        v.clone()
    }
}

/// `n` (>= 0) with `places` digits after the point.
fn with_point(n: &BigInt, places: usize) -> String {
    let s = n.to_string();
    if places == 0 {
        return s;
    }
    let s = format!("{s:0>width$}", width = places + 1);
    let (int, frac) = s.split_at(s.len() - places);
    format!("{int}.{frac}")
}

/// `|v|` rounded to `places`, to the nearest and a tie to even, as text.
fn rounded(v: &BigRational, places: usize) -> String {
    let k = BigRational::from_integer(BigInt::from(10).pow(u32::try_from(places).unwrap()));
    let x = abs(v) * k;
    let (n, d) = x.into_raw();
    let (q, r) = (&n / &d, &n % &d);
    let twice = r * 2;
    let up = twice > d || (twice == d && &q % 2 == BigInt::from(1));
    with_point(&if up { q + 1 } else { q }, places)
}

/// The decimal of `|v|` if it ends: no prime but 2 and 5 in the denominator.
fn exact(v: &BigRational) -> Option<String> {
    let mut d = v.denom().clone();
    let mut places = 0;
    let (two, five) = (BigInt::from(2), BigInt::from(5));
    let (mut twos, mut fives) = (0usize, 0usize);
    while &d % &two == BigInt::from(0) {
        d /= &two;
        twos += 1;
    }
    while &d % &five == BigInt::from(0) {
        d /= &five;
        fives += 1;
    }
    if d != BigInt::from(1) {
        return None;
    }
    places += twos.max(fives);
    let s = rounded(v, places);
    // no trailing zeros, no point without places
    let s = if s.contains('.') {
        s.trim_end_matches('0').trim_end_matches('.').to_string()
    } else {
        s
    };
    Some(s)
}

/// Checks the text of `x` at `p` against the exact value, plain and at some precisions.
fn check(p: Prefix, x: &BigInt, text: &str, prec_texts: &[(usize, String)]) {
    let neg = *x < BigInt::from(0);
    let v = BigRational::from_integer(x.clone()) * scale(p);
    let sign = if neg { "-" } else { "" };
    if let Some(s) = exact(&v) {
        assert_eq!(text, format!("{sign}{s}"), "{p:?} of {x}");
    } else {
        // the first 9 significant digits, cut, then …
        let body = text.strip_prefix(sign).unwrap().strip_suffix('…').unwrap();
        let (int, frac) = body.split_once('.').unwrap();
        let digits = format!("{int}{frac}");
        let significant = digits.trim_start_matches('0').len();
        assert!(significant >= 9 || int.len() >= 9, "{p:?} of {x}: {text}");
        assert!(significant == 9 || frac.len() == 1, "{p:?} of {x}: {text}");
        let cut = BigRational::new(
            body.replace('.', "").parse::<BigInt>().unwrap(),
            BigInt::from(10).pow(u32::try_from(frac.len()).unwrap()),
        );
        let ulp = BigRational::new(
            1.into(),
            BigInt::from(10).pow(u32::try_from(frac.len()).unwrap()),
        );
        assert!(
            cut <= abs(&v) && abs(&v) < cut + ulp,
            "{p:?} of {x}: {text}"
        );
    }
    for (places, got) in prec_texts {
        let want = rounded(&v, *places);
        // a value that rounds to 0 keeps its sign, as Rust's float formatting does
        assert_eq!(*got, format!("{sign}{want}"), "{p:?} of {x} to {places}");
    }
}

#[test]
fn every_prefix_against_the_reference() {
    let values: [i128; 14] = [
        0,
        1,
        -1,
        3,
        5,
        15,
        999,
        65535,
        -1000,
        i32::MIN as i128,
        u32::MAX as i128,
        i64::MAX as i128,
        i128::MAX,
        i128::MIN + 1,
    ];
    let precs = [0, 1, 2, 3, 7, 20, 70];
    for p in every_prefix() {
        for x in values {
            let d = p.decimal(x).unwrap();
            let texts: Vec<(usize, String)> =
                precs.iter().map(|&n| (n, format!("{d:.n$}"))).collect();
            check(p, &BigInt::from(x), &d.to_string(), &texts);
        }
        // the ends of u128 and i128 fit the room
        for (x, d) in [
            (BigInt::from(u128::MAX), p.decimal(u128::MAX).unwrap()),
            (BigInt::from(i128::MIN), p.decimal(i128::MIN).unwrap()),
        ] {
            let texts: Vec<(usize, String)> =
                precs.iter().map(|&n| (n, format!("{d:.n$}"))).collect();
            check(p, &x, &d.to_string(), &texts);
        }
    }
}

#[test]
fn carries_and_ties() {
    let milli = Prefix::Milli;
    assert_eq!(format!("{:.1}", milli.decimal(9960).unwrap()), "10.0");
    assert_eq!(format!("{:.0}", milli.decimal(9500).unwrap()), "10");
    assert_eq!(format!("{:.0}", milli.decimal(8500).unwrap()), "8");
    assert_eq!(format!("{:.0}", milli.decimal(-500).unwrap()), "-0");
    assert_eq!(
        format!("{:.2}", milli.decimal(-999_995).unwrap()),
        "-1000.00"
    );
    assert_eq!(format!("{:.3}", milli.decimal(5).unwrap()), "0.005");
    assert_eq!(format!("{:.1}", milli.decimal(5).unwrap()), "0.0");
    assert_eq!(format!("{:.1}", milli.decimal(51).unwrap()), "0.1");
    // 2/3 of a step of FullScale(2) is 0.666..: cut without a precision, rounded with
    let third = Prefix::FullScale(FullScaleBits::new(2).unwrap());
    assert_eq!(third.decimal(2u8).unwrap().to_string(), "0.666666666…");
    assert_eq!(format!("{:.3}", third.decimal(2u8).unwrap()), "0.667");
    assert_eq!(
        third.decimal(3_000_000_000u32).unwrap().to_string(),
        "1000000000"
    );
    assert_eq!(
        third.decimal(3_000_000_001u32).unwrap().to_string(),
        "1000000000.3…"
    );
    // 3/15 is 0.2: a remainder whose fraction ends
    let fifteen = Prefix::FullScale(FullScaleBits::new(4).unwrap());
    assert_eq!(fifteen.decimal(3u8).unwrap().to_string(), "0.2");
    assert_eq!(fifteen.decimal(18u8).unwrap().to_string(), "1.2");
    assert_eq!(Prefix::Unknown(3).decimal(1u8).map(|d| d.to_string()), None);
}

#[test]
fn flags() {
    let d = Prefix::Milli.decimal(-3300i32).unwrap();
    assert_eq!(format!("{d:>8}"), "    -3.3");
    assert_eq!(format!("{d:<8}|"), "-3.3    |");
    assert_eq!(format!("{d:^8}"), "  -3.3  ");
    assert_eq!(format!("{d:08.2}"), "-0003.30");
    assert_eq!(format!("{:+}", Prefix::Kilo.decimal(2u8).unwrap()), "+2000");
}
