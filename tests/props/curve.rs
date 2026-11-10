//! Transfer curves (LVL-9): a form's exact decode against big-rational powers, the exact encode against a scan of
//! every code, the certified `f64` edge against the exact one, the float-free certified encode against both, PQ and
//! HLG through the data and the typed path.
#![cfg(not(any(kani, fuzzing)))]
#![expect(
    clippy::arithmetic_side_effects,
    reason = "a test's codes and steps are small integers it chose"
)]
#![expect(clippy::unwrap_used, reason = "a property fails by panicking")]

use crate::reference::curve::{encode_by_scan, pow};
use exact_units::codec::video::{Hlg, Pq};
use exact_units::codec::{
    CurveDef, CurveForm, Domain, EncodeError, ExpBase, Quantizer, Referred, Segment,
};
use exact_units::{Fraction, MarkerId, Rounding};
use num_bigint::BigInt;
use num_rational::BigRational;
use proptest::prelude::*;

/// Every rounding.
const ROUNDINGS: [Rounding; 5] = [
    Rounding::Truncate,
    Rounding::Floor,
    Rounding::Ceil,
    Rounding::NearestEven,
    Rounding::NearestAway,
];

/// A `Fraction` as a big rational.
fn big(f: Fraction) -> BigRational {
    let (n, d) = f.mantissa();
    crate::reference::rational_exp10(f.is_negative(), n, d, f.exp10().into())
}

/// A big rational as a `Fraction`, where it fits.
fn small(r: &BigRational) -> Option<Fraction> {
    let num = i128::try_from(r.numer()).ok()?;
    let den = u128::try_from(r.denom()).ok()?;
    Fraction::checked_new(num, den)
}

/// The 10-bit narrow-range quantizer as data: code `k` is the signal `(k − 64)/876`, codes 4..=1019.
const NARROW10: Quantizer = Quantizer {
    zero: 64,
    step: Fraction::new(1, 876),
    low: 4,
    high: 1019,
};

/// A curve of `segments` over 0..=1, relative light.
fn curve(segments: &[Segment]) -> CurveDef<'_> {
    CurveDef {
        id: MarkerId {
            crate_name: "props",
            name: "Curve",
        },
        referred: Referred::Scene,
        unit: MarkerId {
            crate_name: "exact_units",
            name: "Ratio",
        },
        scale: Fraction::ONE,
        low: Fraction::ZERO,
        high: Fraction::ONE,
        segments: exact_units::system::def_list(segments),
    }
}

/// `E = E′²/3`: HLG's lower half over the whole domain, exact at every rational signal.
const SQUARE: [Segment; 1] = [Segment {
    from: Fraction::ZERO,
    form: CurveForm::Power {
        gain: Fraction::new(1, 3),
        scale: Fraction::ONE,
        shift: Fraction::ZERO,
        exponent: Fraction::int(2),
        offset: Fraction::ZERO,
    },
}];

/// The power `x^(p/q)` of every `x = a/b` with `a` in 0..=40 and `b` in 1..=40, for exponents with roots and
/// without: `decode_exact` answers exactly where the big-rational reference has an exact value that fits.
#[test]
fn a_power_is_exact_where_the_reference_is() {
    for (p, q) in [
        (2_i64, 1_u32),
        (1, 2),
        (3, 2),
        (12, 5),
        (5, 12),
        (-1, 1),
        (2, 3),
        (-3, 2),
        (32, 2523),
    ] {
        let form = CurveForm::Power {
            gain: Fraction::ONE,
            scale: Fraction::ONE,
            shift: Fraction::ZERO,
            exponent: Fraction::new(i128::from(p), u128::from(q)),
            offset: Fraction::ZERO,
        };
        for a in 0..=40_i128 {
            for b in 1..=40_u128 {
                let x = Fraction::new(a, b);
                let want = pow(&big(x), p, q).as_ref().and_then(small);
                assert_eq!(form.decode_exact(x), want, "({a}/{b})^({p}/{q})");
            }
        }
    }
}

/// A shift moves the base and `max(.., 0)` stops it at 0: `(x − ¼)²` is 0 below ¼.
#[test]
fn a_power_clamps_its_base_at_zero() {
    let form = CurveForm::Power {
        gain: Fraction::ONE,
        scale: Fraction::ONE,
        shift: Fraction::new(-1, 4),
        exponent: Fraction::int(2),
        offset: Fraction::new(1, 10),
    };
    assert_eq!(
        form.decode_exact(Fraction::new(1, 8)),
        Some(Fraction::new(1, 10))
    );
    assert_eq!(
        form.decode_exact(Fraction::new(3, 4)),
        Some(Fraction::new(7, 20))
    );
}

/// PQ's decode is exact at E′ = 0 and 1 alone, among every 10-bit code from black to white; e⁰ is the one exact
/// value of an `ExpBase::E` form, and a rational base is exact at a whole exponent.
#[test]
fn pq_and_exp_are_exact_where_they_can_be() {
    let pq = Pq::DEF;
    for k in 64..=940 {
        let e = NARROW10.signal(k).unwrap();
        let light = pq.decode_exact(e);
        match k {
            64 => assert_eq!(light, Some(Fraction::ZERO)),
            940 => assert_eq!(light, Some(Fraction::int(10_000))),
            _ => assert_eq!(light, None, "code {k}"),
        }
    }
    let hlg = Hlg::DEF;
    assert_eq!(
        hlg.decode_exact(Fraction::new(1, 2)),
        Some(Fraction::new(1, 12))
    );
    assert_eq!(hlg.decode_exact(Fraction::new(3, 4)), None);
    let tenth = CurveForm::Exp {
        gain: Fraction::ONE,
        base: ExpBase::Exactly(Fraction::new(11, 10)),
        scale: Fraction::ONE,
        shift: Fraction::ZERO,
        offset: Fraction::ZERO,
    };
    assert_eq!(
        tenth.decode_exact(Fraction::int(3)),
        Some(Fraction::new(1331, 1000))
    );
    assert_eq!(tenth.decode_exact(Fraction::new(1, 2)), None);
}

/// The lights of every code of `q` within `c`'s domain, and of each half-code between them, in big rationals.
fn lights(c: &CurveDef<'_>, q: &Quantizer) -> (Quantizer, Vec<BigRational>, Vec<BigRational>) {
    let q = q.within(c).unwrap();
    let mut at = Vec::new();
    let mut ties = Vec::new();
    for k in q.low..=q.high {
        at.push(big(c.decode_exact(q.signal(k).unwrap()).unwrap()));
        let half = Fraction::new(2 * (k - q.zero) + 1, 2)
            .checked_mul(q.step)
            .unwrap();
        if k < q.high {
            ties.push(big(c.decode_exact(half).unwrap()));
        }
    }
    (q, at, ties)
}

/// `encode_exact` of `y` by every rounding in both domains, against the scan.
fn encode_matches_scan(
    c: &CurveDef<'_>,
    q: &Quantizer,
    at: &[BigRational],
    ties: &[BigRational],
    y: Fraction,
) {
    for r in ROUNDINGS {
        for d in [Domain::Signal, Domain::Light] {
            let want = encode_by_scan(at, ties, q.low, q.zero, &big(y), r, d == Domain::Light);
            let got = c.encode_exact(q, y, r, d);
            match want {
                Some(k) => assert_eq!(got, Ok(k), "{y:?} by {r:?} in {d:?}"),
                None => assert_eq!(got, Err(EncodeError::OutOfRange), "{y:?} by {r:?} in {d:?}"),
            }
        }
    }
}

/// On `E′²/3` over 10-bit codes, every code's light and every half-code's light (the ties, where the roundings
/// differ) encode as the scan says, by every rounding in both domains; every code comes back from its own light.
#[test]
fn the_exact_encode_decides_every_code_and_every_tie() {
    let c = curve(&SQUARE);
    let (q, at, ties) = lights(&c, &NARROW10);
    assert_eq!((q.low, q.high), (64, 940));
    for (i, k) in (q.low..=q.high).enumerate() {
        let y = small(&at[i]).unwrap();
        for r in ROUNDINGS {
            for d in [Domain::Signal, Domain::Light] {
                assert_eq!(
                    c.encode_exact(&q, y, r, d),
                    Ok(k),
                    "code {k} by {r:?} in {d:?}"
                );
            }
        }
        if let Some(t) = ties.get(i) {
            encode_matches_scan(&c, &q, &at, &ties, small(t).unwrap());
        }
    }
}

/// Some light rounds to one code in the signal and to the other in the light: the domains are two answers, and a
/// caller names one.
#[test]
fn the_domains_differ_somewhere() {
    let c = curve(&SQUARE);
    let (q, at, _) = lights(&c, &NARROW10);
    let differs = at.windows(2).any(|w| {
        // a quarter of the way past the light midpoint: nearer the upper code in light, the lower in signal
        let y = small(&((&w[0] + &w[1]) / BigRational::from_integer(BigInt::from(2)))).unwrap();
        let s = c.encode_exact(&q, y, Rounding::NearestEven, Domain::Signal);
        let l = c.encode_exact(&q, y, Rounding::NearestEven, Domain::Light);
        s != l
    });
    assert!(differs);
}

/// HLG's lower half, every 10-bit code 64..=502 (E′ = 0..=½): its light is `(D − 64)²/(3·876²)` exactly, and
/// comes back to the code by every rounding in both domains. Every half-code's light (a tie in the signal) and
/// every midpoint of two codes' lights (a tie in the light) encodes as a scan of the codes says, though E′ = √(3E)
/// is irrational for almost all of them. Above 1/12 the exact encode says it can't.
#[test]
fn hlg_is_exact_on_its_lower_half() {
    let c = Hlg::DEF;
    let half = 502;
    let at: Vec<BigRational> = (64..=half)
        .map(|k| {
            let d = k - 64;
            BigRational::new(BigInt::from(d * d), BigInt::from(3 * 876 * 876))
        })
        .collect();
    let ties: Vec<BigRational> = (64..half)
        .map(|k| {
            let twice = 2 * (k - 64) + 1;
            BigRational::new(BigInt::from(twice * twice), BigInt::from(4 * 3 * 876 * 876))
        })
        .collect();
    let two = BigRational::from_integer(BigInt::from(2));
    for (i, k) in (64..=half).enumerate() {
        let y = small(&at[i]).unwrap();
        assert_eq!(
            c.decode_exact(NARROW10.signal(k).unwrap()),
            Some(y),
            "code {k}"
        );
        for r in ROUNDINGS {
            for d in [Domain::Signal, Domain::Light] {
                assert_eq!(
                    c.encode_exact(&NARROW10, y, r, d),
                    Ok(k),
                    "code {k} by {r:?} in {d:?}"
                );
            }
        }
        let Some(next) = at.get(i + 1) else { continue };
        for t in [ties[i].clone(), (&at[i] + next) / &two] {
            let y = small(&t).unwrap();
            for r in ROUNDINGS {
                for d in [Domain::Signal, Domain::Light] {
                    let want = encode_by_scan(&at, &ties, 64, 64, &t, r, d == Domain::Light);
                    assert_eq!(
                        c.encode_exact(&NARROW10, y, r, d).ok(),
                        want,
                        "{y:?} by {r:?} in {d:?}"
                    );
                }
            }
        }
    }
    // the footroom is outside HLG's domain: no light, and no light below black
    assert_eq!(c.decode_exact(NARROW10.signal(63).unwrap()), None);
    // just above 1/12, between codes 502 and 503: 503's light is irrational, so no exact answer
    let above = Fraction::new(1, 12)
        .checked_add(Fraction::new(1, 1_000_000))
        .unwrap();
    for d in [Domain::Signal, Domain::Light] {
        assert_eq!(
            c.encode_exact(&NARROW10, above, Rounding::Floor, d),
            Err(EncodeError::NotExact)
        );
        assert_eq!(
            c.encode_exact(&NARROW10, Fraction::new(1, 2), Rounding::NearestAway, d),
            Err(EncodeError::NotExact)
        );
    }
}

/// HLG's halves meet at E′ = ½ exactly: both forms give 1/12 there, so the curve is continuous, and in `f64` both
/// sides of the joint are within an ulp or two of 1/12.
#[test]
fn hlg_is_continuous_at_one_half() {
    let c = Hlg::DEF;
    let half = Fraction::new(1, 2);
    let lower = c.segment_at(Fraction::new(1, 4)).unwrap().form;
    let upper = c.segment_at(half).unwrap().form;
    assert_ne!(lower, upper);
    assert_eq!(lower.decode_exact(half), Some(Fraction::new(1, 12)));
    assert_eq!(upper.decode_exact(half), Some(Fraction::new(1, 12)));
    assert_eq!(c.decode_exact(half), Some(Fraction::new(1, 12)));
    for form in [lower, upper] {
        let y = form.decode_f64(0.5).unwrap();
        assert!((y - 1.0 / 12.0).abs() < 1e-16, "{y}");
    }
    let below = c.decode_f64(0.5_f64.next_down()).unwrap();
    let above = c.decode_f64(0.5_f64.next_up()).unwrap();
    assert!(below < above && above - below < 1e-15, "{below} {above}");
}

/// HLG's OETF as BT.2100-3 Table 5 writes it, in `f64` with std's `ln` and `sqrt` (the crate's runs libm's `log` on
/// the `Exp` form's inverse), c as the formula: E′ for a scene light E in 0..=1.
fn hlg_oetf_reference(e: f64) -> f64 {
    let a = 0.178_832_77_f64;
    let b = 1.0 - 4.0 * a;
    let c = 0.5 - a * (4.0 * a).ln();
    if e <= 1.0 / 12.0 {
        (3.0 * e).sqrt()
    } else {
        a * (12.0 * e - b).ln() + c
    }
}

/// HLG's inverse OETF as BT.2100-3 Table 5 writes it, in `f64` with std's `exp`: E for E′.
fn hlg_inverse_oetf_reference(e: f64) -> f64 {
    let a = 0.178_832_77_f64;
    let b = 1.0 - 4.0 * a;
    let c = 0.5 - a * (4.0 * a).ln();
    if e <= 0.5 {
        e * e / 3.0
    } else {
        (((e - c) / a).exp() + b) / 12.0
    }
}

/// HLG's upper half, every 10-bit code 503..=940, through the certified `f64` edge: the light matches the
/// standard's formula in std's `f64`, rises strictly, and comes back to its code by the nearest roundings in both
/// domains (a code's own light is far from every tie); a directed rounding gives the code, the one beside it, or
/// both where `f64` can't tell. The exact path says it can't.
#[test]
fn hlg_upper_half_by_the_f64_edge() {
    let c = Hlg::DEF;
    let mut last = c.decode_f64(0.5).unwrap();
    for k in 503..=940 {
        let e = NARROW10.signal(k).unwrap();
        assert_eq!(c.decode_exact(e), None, "code {k}");
        let y = c.decode_f64(e.to_f64()).unwrap();
        let want = hlg_inverse_oetf_reference(e.to_f64());
        assert!(
            (y - want).abs() <= 4.0 * f64::EPSILON * want,
            "code {k}: {y} against {want}"
        );
        assert!(y > last, "code {k}");
        last = y;
        for r in ROUNDINGS {
            for d in [Domain::Signal, Domain::Light] {
                let got = c.encode_f64(&NARROW10, y, r, d);
                // the f64 light is a rounding of the code's own: a directed rounding may go a code down or up from
                // it, and where f64 can't tell, the edge names both
                let (below, above) = match r {
                    Rounding::Floor | Rounding::Truncate => (k - 1, k),
                    Rounding::Ceil => (k, (k + 1).min(940)),
                    _ => (k, k),
                };
                match got {
                    Ok(got) => assert!(
                        (below..=above).contains(&got),
                        "code {k} by {r:?} in {d:?}: {got}"
                    ),
                    Err(EncodeError::NearTie { below: b, above: a }) => {
                        assert_eq!((b, a), (below, above), "code {k} by {r:?} in {d:?}");
                    }
                    Err(e) => panic!("code {k} by {r:?} in {d:?}: {e:?}"),
                }
            }
        }
        assert_eq!(
            c.encode_exact(
                &NARROW10,
                Fraction::from_f64(y).unwrap(),
                Rounding::NearestAway,
                Domain::Signal
            ),
            Err(EncodeError::NotExact)
        );
    }
}

/// Scene lights from 1/12 to 1 in steps of 1/100 000 encode by the certified edge, in the signal domain, to the code
/// the standard's OETF in std's `f64` rounds to: where the reference's code position is at least 10⁻⁶ from a
/// tie, far beyond either side's few ulps, the edge gives that code; nearer, it gives that code or both candidates.
#[test]
fn hlg_encodes_every_light_as_the_standard() {
    let c = Hlg::DEF;
    for i in 0..=100_000_u32 {
        let e = 1.0 / 12.0 + f64::from(i) * (11.0 / 12.0) / 100_000.0;
        let e = e.min(1.0);
        let pos = 876.0 * hlg_oetf_reference(e) + 64.0;
        let want = pos.round();
        let got = c.encode_f64(&NARROW10, e, Rounding::NearestAway, Domain::Signal);
        if (pos - pos.floor() - 0.5).abs() >= 1e-6 {
            assert_eq!(got.map(|k| k as f64), Ok(want), "E = {e}, position {pos}");
        } else {
            match got {
                Ok(k) => assert_eq!(k as f64, want, "E = {e}"),
                Err(EncodeError::NearTie { below, above }) => {
                    assert_eq!((below as f64, above as f64), (pos.floor(), pos.ceil()));
                }
                Err(err) => panic!("E = {e}: {err:?}"),
            }
        }
        let x = Hlg::oetf_f64(e).unwrap();
        assert!((x - hlg_oetf_reference(e)).abs() < 1e-15, "E = {e}");
    }
}

/// The ends of HLG's float edge: 0 both ways; E′ = 1 is E ≈ 1.000000027, a rounded 8-decimal a; E = 1 is E′ ≈
/// 0.999999995, both inside code 940; outside 0..=1 is refused.
#[test]
fn hlg_ends() {
    assert_eq!(Hlg::inverse_oetf_f64(0.0), Some(0.0));
    assert_eq!(Hlg::oetf_f64(0.0), Some(0.0));
    let top = Hlg::inverse_oetf_f64(1.0).unwrap();
    assert!(top > 1.000_000_026 && top < 1.000_000_028, "{top}");
    let white = Hlg::oetf_f64(1.0).unwrap();
    assert!(white > 0.999_999_994 && white < 0.999_999_996, "{white}");
    for bad in [-1e-300, 1.0 + f64::EPSILON, f64::NAN, f64::INFINITY] {
        assert_eq!(Hlg::inverse_oetf_f64(bad), None);
        assert_eq!(Hlg::oetf_f64(bad), None);
    }
    assert_eq!(
        Hlg::DEF.encode_f64(&NARROW10, 1.0, Rounding::NearestAway, Domain::Signal),
        Ok(940)
    );
    assert_eq!(
        Hlg::DEF.encode_f64(&NARROW10, 1.000_001, Rounding::NearestAway, Domain::Signal),
        Err(EncodeError::OutOfRange)
    );
}

/// BT.2100-3 writes b and c as formulas of a; ARIB STD-B67 1.0 Table 2 prints them as digits, b = 0.28466892 and c
/// = 0.55991073, with E normalised to reference white (its E is BT.2100's 12E, running to 12) and r = 0.5. Its b is
/// 1 − 4a exactly, its c is 0.5 − a·ln(4a) to 8 decimals, and its OETF is BT.2100's of E/12.
#[test]
fn hlg_matches_arib_std_b67() {
    assert_eq!(Hlg::A, Fraction::new(17_883_277, 100_000_000));
    assert_eq!(Hlg::B, Fraction::new(28_466_892, 100_000_000));
    let a = Hlg::A.to_f64();
    let c = 0.5 - a * (4.0 * a).ln();
    assert_eq!(format!("{c:.8}"), "0.55991073");
    let (b_arib, c_arib, r) = (0.284_668_92, 0.559_910_73, 0.5_f64);
    for i in 0..=1200 {
        let e_arib = f64::from(i) / 100.0;
        let arib = if e_arib <= 1.0 {
            r * e_arib.sqrt()
        } else {
            a * (e_arib - b_arib).ln() + c_arib
        };
        let ours = Hlg::oetf_f64(e_arib / 12.0).unwrap();
        // ARIB's c is c rounded to 8 decimals: under 5·10⁻⁹ apart
        assert!(
            (arib - ours).abs() < 5e-9,
            "E = {e_arib}: {arib} against {ours}"
        );
    }
}

/// The certified `f64` edge on the square curve never contradicts the exact encode of the same light (the `f64`'s
/// own value): where it answers, it is the exact answer; where it can't, both candidates bracket it.
fn f64_agrees_with_exact(y: f64) {
    f64_agrees_on(&curve(&SQUARE), y);
}

/// [f64_agrees_with_exact] on the curve `c`, for a light where its exact encode answers.
fn f64_agrees_on(c: &CurveDef<'_>, y: f64) {
    let exact = Fraction::from_f64(y).unwrap();
    for r in ROUNDINGS {
        for d in [Domain::Signal, Domain::Light] {
            let want = c.encode_exact(&NARROW10, exact, r, d);
            match c.encode_f64(&NARROW10, y, r, d) {
                Ok(k) => assert_eq!(Ok(k), want, "{y} by {r:?} in {d:?}"),
                Err(EncodeError::NearTie { below, above }) => {
                    let k = want.unwrap();
                    assert!(
                        k == below || k == above,
                        "{y} by {r:?} in {d:?}: {k} not in {below}..={above}"
                    );
                    let any = c
                        .encode_f64_off_by_one_near_ties(&NARROW10, y, r, d)
                        .unwrap();
                    assert!(any == below || any == above);
                }
                Err(e) => assert_eq!(Err(e), want, "{y} by {r:?} in {d:?}"),
            }
        }
    }
}

proptest! {
    #[test]
    fn the_f64_edge_never_contradicts_the_exact_one(y in 0.0..=1.0_f64 / 3.0) {
        f64_agrees_with_exact(y);
    }

    #[test]
    fn the_f64_edge_at_ties(k in 64..940_i128, nudge in -4_i32..=4) {
        // the f64 nearest a half-code's light, nudged by a few ulps: the hardest inputs
        let half = Fraction::new(2 * (k - 64) + 1, 2 * 876);
        let mut y = curve(&SQUARE).decode_exact(half).unwrap().to_f64();
        for _ in 0..nudge.unsigned_abs() {
            y = if nudge > 0 { y.next_up() } else { y.next_down() };
        }
        f64_agrees_with_exact(y);
    }

    #[test]
    fn hlg_f64_edge_never_contradicts_the_exact_one(y in 0.0..=1.0_f64 / 12.0) {
        f64_agrees_on(&Hlg::DEF, y);
    }

    #[test]
    fn hlg_f64_edge_at_ties(k in 64..502_i128, nudge in -4_i32..=4) {
        // the f64 nearest a half-code's light on HLG's lower half, nudged by a few ulps
        let half = Fraction::new(2 * (k - 64) + 1, 2 * 876);
        let mut y = Hlg::DEF.decode_exact(half).unwrap().to_f64();
        for _ in 0..nudge.unsigned_abs() {
            y = if nudge > 0 { y.next_up() } else { y.next_down() };
        }
        f64_agrees_on(&Hlg::DEF, y);
    }
}

/// PQ's black by every rounding is code 64, whose light is 0 exactly, by the `f64` edge as by the exact one: the
/// closed-form inverse puts 0 cd/m² at E′ = c₁^m₂ ≈ 7·10⁻⁷, the top of the stretch that decodes to black, which
/// rounded up to code 65 before the edge took a light at an end code's exact light as that code.
#[test]
fn pq_black_is_code_64_by_every_rounding() {
    for r in ROUNDINGS {
        for d in [Domain::Signal, Domain::Light] {
            assert_eq!(
                Pq::DEF.encode_f64(&NARROW10, 0.0, r, d),
                Ok(64),
                "{r:?} in {d:?}"
            );
            assert_eq!(
                Pq::DEF.encode_exact(&NARROW10, Fraction::ZERO, r, d),
                Ok(64)
            );
        }
    }
}

/// The ends of the square curve's lights in `f64`, and past them.
#[test]
fn the_f64_edge_at_the_ends() {
    f64_agrees_with_exact(0.0);
    f64_agrees_with_exact(1.0 / 3.0);
    let c = curve(&SQUARE);
    assert_eq!(
        c.encode_f64(&NARROW10, 0.34, Rounding::NearestEven, Domain::Signal),
        Err(EncodeError::OutOfRange)
    );
    assert_eq!(
        c.encode_f64(&NARROW10, -0.01, Rounding::NearestEven, Domain::Signal),
        Err(EncodeError::OutOfRange)
    );
}

/// PQ at its ties: the `f64` of the light at a half-code puts the code position within the margin of the tie, so
/// the certified edge names both codes (the exact path can't help, PQ's tie lights are irrational), and the
/// always-a-code variant gives one of them.
#[test]
fn pq_names_both_codes_at_a_tie() {
    let pq = Pq::DEF;
    let mut near = 0;
    for k in [100_i128, 300, 500, 700, 900] {
        let e = (f64::from(i32::try_from(k - 64).unwrap()) + 0.5) / 876.0;
        let y = pq.decode_f64(e).unwrap();
        match pq.encode_f64(&NARROW10, y, Rounding::NearestEven, Domain::Signal) {
            Err(EncodeError::NearTie { below, above }) => {
                assert_eq!((below, above), (k, k + 1));
                near += 1;
            }
            Ok(c) => assert!(c == k || c == k + 1),
            Err(e) => panic!("code {k}: {e:?}"),
        }
        let any = pq
            .encode_f64_off_by_one_near_ties(&NARROW10, y, Rounding::NearestEven, Domain::Signal)
            .unwrap();
        assert!(any == k || any == k + 1);
    }
    assert!(near >= 4, "only {near} of 5 ties seen as ties");
}

/// BT.709's inverse OETF above its toe over the whole domain, `((V + 0.099)/1.099)^(20/9)`: a `Power` whose base has
/// a scale, irrational almost everywhere, rising throughout (the toe left out: the standard's two segments don't
/// meet, the power's light at V = 0.081 is below the toe's).
const BT709_POWER: [Segment; 1] = [Segment {
    from: Fraction::ZERO,
    form: CurveForm::Power {
        gain: Fraction::ONE,
        scale: Fraction::new(1000, 1099),
        shift: Fraction::new(99, 1099),
        exponent: Fraction::new(20, 9),
        offset: Fraction::ZERO,
    },
}];

/// The root precisions the interval properties try, from coarse to what `u128` holds.
const BITS: [u32; 4] = [8, 32, 64, 128];

/// Every code from 64 to 940 and every half-code between them, as signals of the 10-bit narrow quantizer.
fn codes_and_halves() -> impl Iterator<Item = Fraction> {
    (2 * 64..=2 * 940).map(|twice| Fraction::new(twice - 128, 2 * 876))
}

/// The interval holds the exact value wherever `decode_exact` answers, at every precision: every code and half-code
/// of `E′²/3` and of the HLG-shaped curve's lower half, PQ's black and white.
#[test]
fn the_interval_holds_the_exact_value() {
    let mut seen = 0;
    for c in [curve(&SQUARE), Hlg::DEF, Pq::DEF] {
        for x in codes_and_halves() {
            let Some(want) = c.decode_exact(x) else {
                continue;
            };
            for bits in BITS {
                let got = c.decode_interval(x, bits).unwrap();
                assert!(
                    got.contains(want),
                    "{x:?} at {bits}: {got:?} misses {want:?}"
                );
            }
            seen += 1;
        }
    }
    // the square curve's 1753, the HLG-shaped lower half's 877, PQ's 2
    assert_eq!(seen, 1753 + 877 + 2);
}

/// Where the light is irrational (PQ between black and white, the HLG-shaped curve's upper half, BT.709's power), the
/// interval holds the `f64` edge's value within that edge's own error, 10⁻¹² relative; at 128 bits its `f64` bounds
/// are adjacent floats at most, the true value pinned to one `f64` gap.
#[test]
fn the_interval_holds_the_f64_value() {
    for c in [Pq::DEF, Hlg::DEF, curve(&BT709_POWER)] {
        for x in codes_and_halves() {
            if c.decode_exact(x).is_some() {
                continue;
            }
            let f = c.decode_f64(x.to_f64()).unwrap();
            for bits in BITS {
                let (lo, hi) = c.decode_interval(x, bits).unwrap().to_f64();
                let slack = f.abs() * 1e-12;
                assert!(
                    lo - slack <= f && f <= hi + slack,
                    "{x:?} at {bits}: {f} not in {lo}..={hi}"
                );
            }
            let (lo, hi) = c.decode_interval(x, 128).unwrap().to_f64();
            // both positive: adjacent floats have adjacent bits
            assert!(hi.to_bits() <= lo.to_bits() + 1, "{x:?}: {lo}..={hi}");
        }
    }
}

/// `encode_certified` of `y` equals `encode_exact` wherever that answers, by every rounding in both domains.
fn certified_agrees_with_exact(c: &CurveDef<'_>, y: Fraction) {
    for r in ROUNDINGS {
        for d in [Domain::Signal, Domain::Light] {
            if let Ok(k) = c.encode_exact(&NARROW10, y, r, d) {
                assert_eq!(
                    c.encode_certified(&NARROW10, y, r, d),
                    Ok(k),
                    "{y:?} by {r:?} in {d:?}"
                );
            }
        }
    }
}

/// On `E′²/3`, where every decode is exact, the certified encode is the exact one at every code's light and every
/// tie's: the inputs where the roundings differ.
#[test]
fn the_certified_encode_is_the_exact_one_at_every_code_and_tie() {
    let c = curve(&SQUARE);
    for x in codes_and_halves() {
        let y = c.decode_exact(x).unwrap();
        certified_agrees_with_exact(&c, y);
        assert!(
            c.encode_exact(&NARROW10, y, Rounding::NearestEven, Domain::Signal)
                .is_ok()
        );
    }
}

/// `encode_certified` of the light `y` (an `f64`, exact as a rational) never contradicts the certified `f64` edge:
/// where that answers a code, the same code; where it names two, one of them. The certified encode itself never
/// answers `NearTie` on these inputs, which lie nowhere near 2⁻¹¹⁶ of a tie.
fn certified_agrees_with_f64(c: &CurveDef<'_>, y: f64) {
    let exact = Fraction::from_f64(y).unwrap();
    for r in ROUNDINGS {
        for d in [Domain::Signal, Domain::Light] {
            let got = c.encode_certified(&NARROW10, exact, r, d);
            assert!(
                !matches!(got, Err(EncodeError::NearTie { .. })),
                "{y} by {r:?} in {d:?}: {got:?}"
            );
            match c.encode_f64(&NARROW10, y, r, d) {
                Ok(k) => assert_eq!(got, Ok(k), "{y} by {r:?} in {d:?}"),
                Err(EncodeError::NearTie { below, above }) => {
                    let k = got.unwrap();
                    assert!(
                        k == below || k == above,
                        "{y} by {r:?} in {d:?}: {k} not in {below}..={above}"
                    );
                }
                Err(e) => assert_eq!(got, Err(e), "{y} by {r:?} in {d:?}"),
            }
        }
    }
}

proptest! {
    #[test]
    fn the_certified_encode_is_the_exact_one(a in 0..=1000_i128, b in 1..=3000_u128) {
        certified_agrees_with_exact(&curve(&SQUARE), Fraction::new(a, b));
        certified_agrees_with_exact(&Hlg::DEF, Fraction::new(a, b));
    }

    #[test]
    fn the_certified_encode_never_contradicts_the_f64_one(y in 0.0..=10_000.0_f64) {
        certified_agrees_with_f64(&Pq::DEF, y);
        certified_agrees_with_f64(&Hlg::DEF, y / 10_000.0);
        certified_agrees_with_f64(&curve(&BT709_POWER), y / 10_000.0);
    }

    #[test]
    fn the_certified_encode_decides_the_f64_ties(k in 64..940_i128, nudge in -4_i32..=4) {
        // the f64 nearest a half-code's light, nudged by a few ulps: where the f64 edge names two codes
        let e = (f64::from(i32::try_from(k - 64).unwrap()) + 0.5) / 876.0;
        for c in [Pq::DEF, Hlg::DEF, curve(&BT709_POWER)] {
            let mut y = c.decode_f64(e).unwrap();
            for _ in 0..nudge.unsigned_abs() {
                y = if nudge > 0 { y.next_up() } else { y.next_down() };
            }
            certified_agrees_with_f64(&c, y);
        }
    }
}

/// PQ at whole cd/m², every 37th from 0 to 10 000 and its ends: the certified encode always decides, and agrees
/// with the `f64` edge.
#[test]
fn pq_decides_whole_nits() {
    for n in (0..=10_000).step_by(37).chain([1, 9_999, 10_000]) {
        certified_agrees_with_f64(&Pq::DEF, f64::from(n));
    }
}

/// Beyond the codes' lights, out of range; a light within 2⁻¹²⁰ of a PQ tie (a rational between the interval's
/// bounds) is a `NearTie` naming both codes, never a guess.
#[test]
fn the_certified_encode_refuses_what_it_cant_decide() {
    let pq = Pq::DEF;
    let enc = |y| pq.encode_certified(&NARROW10, y, Rounding::NearestEven, Domain::Signal);
    assert_eq!(enc(Fraction::int(10_001)), Err(EncodeError::OutOfRange));
    assert_eq!(enc(Fraction::int(-1)), Err(EncodeError::OutOfRange));
    // a light with no plain terms in u128, 10⁻⁵⁰ cd/m² past black: the interval's order can't take it
    let past = Fraction::pow10(-50);
    assert!(past.denom().is_none());
    assert_eq!(enc(past), Err(EncodeError::NotExact));
    // the tie between codes 500 and 501, bounded at 128 bits; a rational from its f64 bounds' midpoint sits within
    // one f64 gap of it, so the 8-bit interval holds it and can't decide
    let half = Fraction::new(2 * (500 - 64) + 1, 2 * 876);
    let (lo, hi) = pq.decode_interval(half, 128).unwrap().to_f64();
    let near = Fraction::from_f64(f64::midpoint(lo, hi)).unwrap();
    assert!(pq.decode_interval(half, 8).unwrap().contains(near));
    match enc(near) {
        Ok(k) => assert!(k == 500 || k == 501),
        Err(e) => assert_eq!(
            e,
            EncodeError::NearTie {
                below: 500,
                above: 501
            }
        ),
    }
}

#[cfg(feature = "photometric")]
mod typed {
    use exact_units::codec::video::{Narrow10, Pq};
    use exact_units::codec::{Codec, Coded, Domain, EncodeError, Transfer};
    use exact_units::units::Nit;
    use exact_units::{Exact, Fraction, Rounding};

    type PqCode = Coded<Transfer<Narrow10, Pq>>;

    /// `Transfer<Narrow10, Pq>` is Narrow10's code with PQ named as its curve, and the code layer as data.
    #[test]
    fn its_def_names_the_curve() {
        let def = Transfer::<Narrow10, Pq>::DEF;
        assert_eq!(def.curve, Some(Pq::DEF));
        assert_eq!(def.shape, Narrow10::DEF.shape);
        let q = Transfer::<Narrow10, Pq>::QUANTIZER;
        assert_eq!(
            (q.zero, q.step, q.low, q.high),
            (64, Fraction::new(1, 876), 4, 1019)
        );
    }

    /// Black and white are exact both ways; between them the light is no rational, and the exact encode says so.
    #[test]
    fn exact_at_its_ends_only() {
        let code = |k| PqCode::try_from_raw(k).unwrap();
        assert_eq!(code(64).light_exact(), Some(Exact::<Nit>::int(0)));
        assert_eq!(code(940).light_exact(), Some(Exact::<Nit>::int(10_000)));
        assert_eq!(code(500).light_exact(), None);
        assert_eq!(code(63).light_exact(), None); // footroom: outside PQ's domain
        let enc = |n| {
            PqCode::from_light_exact(Exact::<Nit>::int(n), Rounding::NearestAway, Domain::Signal)
        };
        assert_eq!(enc(10_000).map(Coded::raw), Ok(940));
        assert_eq!(enc(0).map(Coded::raw), Ok(64));
        assert_eq!(enc(100).map(Coded::raw), Err(EncodeError::NotExact));
        assert_eq!(enc(10_001).map(Coded::raw), Err(EncodeError::OutOfRange));
        assert_eq!(PqCode::of(code(500).code()).raw(), 500);
    }

    /// The certified encode decides where the exact one can't: 100 cd/m² is code 509 by both edges.
    #[test]
    fn certified_between_black_and_white() {
        let enc = |n| {
            PqCode::from_light_certified(
                Exact::<Nit>::int(n),
                Rounding::NearestAway,
                Domain::Signal,
            )
        };
        assert_eq!(enc(100).map(Coded::raw), Ok(509));
        assert_eq!(enc(10_000).map(Coded::raw), Ok(940));
        assert_eq!(enc(0).map(Coded::raw), Ok(64));
        assert_eq!(enc(10_001).map(Coded::raw), Err(EncodeError::OutOfRange));
    }
}

/// BT.709's inverse OETF above its toe, `((V + 0.099)/1.099)^(20/9)`, as a `Power` whose base carries the divisor:
/// exact at V = 1 (base 1) and at V = 0.099·k-free points only where the base is a rational's 9th power; the `f64`
/// inverse brings a value back.
#[test]
fn a_power_with_a_divisor_in_its_base() {
    let form = CurveForm::Power {
        gain: Fraction::ONE,
        scale: Fraction::new(1000, 1099),
        shift: Fraction::new(99, 1099),
        exponent: Fraction::new(20, 9),
        offset: Fraction::ZERO,
    };
    assert_eq!(form.decode_exact(Fraction::ONE), Some(Fraction::ONE));
    // base (1/2 + 0.099)/1.099 = 599/1099, no 9th power
    assert_eq!(form.decode_exact(Fraction::new(1, 2)), None);
    // base 512/1099·… : V with (1000·V + 99)/1099 = 1/512 is V = (1099/512 − 99)/1000, below the toe but exact
    let v = Fraction::new(1099 - 99 * 512, 512_000);
    assert_eq!(form.decode_exact(v), Some(Fraction::new(1, 1 << 20)));
    let y = form.decode_f64(0.5).unwrap();
    assert!((form.inverse_f64(y).unwrap() - 0.5).abs() < 1e-15);
}

/// HLG through the typed path, `Coded<Transfer<Narrow10, Hlg>>`: the same answers as the data, by the code's type.
mod hlg_typed {
    use exact_units::codec::video::{Hlg, Narrow10};
    use exact_units::codec::{Coded, Domain, EncodeError, Transfer};
    use exact_units::units::Ratio;
    use exact_units::{Exact, Fraction, Rounding};

    type HlgCode = Coded<Transfer<Narrow10, Hlg>>;

    /// Every code of the lower half decodes exactly and encodes back by both domains; the upper half has no exact
    /// light and its float light comes back to the code; footroom and headroom are outside HLG's domain.
    #[test]
    fn every_code() {
        for k in 4..=1019_u16 {
            let code = HlgCode::try_from_raw(k).unwrap();
            let light = code.light_exact();
            match k {
                64..=502 => {
                    let d = i128::from(k) - 64;
                    let want = Exact::<Ratio>::of(Fraction::new(d * d, 3 * 876 * 876));
                    assert_eq!(light, Some(want), "code {k}");
                    for domain in [Domain::Signal, Domain::Light] {
                        let back = HlgCode::from_light_exact(want, Rounding::NearestAway, domain);
                        assert_eq!(back.map(Coded::raw), Ok(k));
                    }
                }
                _ => assert_eq!(light, None, "code {k}"),
            }
            let float = code.light_f64();
            if (64..=940).contains(&k) {
                let y = float.unwrap();
                for domain in [Domain::Signal, Domain::Light] {
                    let back = HlgCode::from_light_f64(y, Rounding::NearestEven, domain);
                    assert_eq!(back.map(Coded::raw), Ok(k), "code {k}");
                }
            } else {
                assert_eq!(float, None, "code {k}");
            }
        }
        let bright = Exact::<Ratio>::of(Fraction::new(1, 2));
        assert_eq!(
            HlgCode::from_light_exact(bright, Rounding::NearestAway, Domain::Signal)
                .map(Coded::raw),
            Err(EncodeError::NotExact)
        );
    }
}
