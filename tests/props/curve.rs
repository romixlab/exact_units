//! Transfer curves (LVL-9): a form's exact decode against big-rational powers, the exact encode against a scan of
//! every code, the certified `f64` edge against the exact one, PQ and an HLG-shaped curve through the typed path.
#![cfg(not(any(kani, fuzzing)))]
#![expect(
    clippy::arithmetic_side_effects,
    reason = "a test's codes and steps are small integers it chose"
)]
#![expect(clippy::unwrap_used, reason = "a property fails by panicking")]

use crate::reference::curve::{encode_by_scan, pow};
use exact_units::codec::video::Pq;
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
    crate::reference::rational(f.is_negative(), f.numer(), f.denom())
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

/// An HLG-shaped curve, BT.2100-3 Table 5's inverse OETF with a = 0.17883277: `E′²/3` up to ½, then `(4a·e^((E′ − ½)/a) + b)/12` with
/// b = 1 − 4a, c absorbed into the exponent's shift. Exact up to ½ and at ½, not above.
const HLG: [Segment; 2] = [
    SQUARE[0],
    Segment {
        from: Fraction::new(1, 2),
        form: CurveForm::Exp {
            gain: Fraction::new(17_883_277, 300_000_000),
            base: ExpBase::E,
            scale: Fraction::new(100_000_000, 17_883_277),
            shift: Fraction::new(1, 2),
            // b/12 = (1 − 4a)/12
            offset: Fraction::new(100_000_000 - 4 * 17_883_277, 1_200_000_000),
        },
    },
];

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
    let hlg = curve(&HLG);
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

/// On the HLG-shaped curve, a light up to 1/12 (E′ = ½) encodes exactly, as the square curve's scan says, though
/// E′ = √(3E) is irrational for almost all of them; above 1/12 the exact encode says it can't.
#[test]
fn hlg_encodes_exactly_on_its_lower_half() {
    let c = curve(&HLG);
    let square = curve(&SQUARE);
    let (q, at, ties) = lights(&square, &NARROW10);
    // codes 64..=502 are the lower half (E′ = 438/876 = ½ at 502)
    let lower = 502 - q.low;
    let lower = usize::try_from(lower).unwrap();
    let (at, ties) = (&at[..=lower], &ties[..lower]);
    let q_lower = Quantizer { high: 502, ..q };
    for t in ties {
        let y = small(t).unwrap();
        for r in ROUNDINGS {
            for d in [Domain::Signal, Domain::Light] {
                let want = encode_by_scan(
                    at,
                    ties,
                    q_lower.low,
                    q_lower.zero,
                    &big(y),
                    r,
                    d == Domain::Light,
                );
                assert_eq!(
                    c.encode_exact(&NARROW10, y, r, d).ok(),
                    want,
                    "{y:?} by {r:?} in {d:?}"
                );
            }
        }
    }
    assert_eq!(
        c.encode_exact(
            &NARROW10,
            Fraction::new(1, 12),
            Rounding::NearestEven,
            Domain::Signal
        ),
        Ok(502)
    );
    assert_eq!(
        c.encode_exact(
            &NARROW10,
            Fraction::new(1, 2),
            Rounding::NearestEven,
            Domain::Signal
        ),
        Err(EncodeError::NotExact)
    );
}

/// The certified `f64` edge on the square curve never contradicts the exact encode of the same light (the `f64`'s
/// own value): where it answers, it is the exact answer; where it can't, both candidates bracket it.
fn f64_agrees_with_exact(y: f64) {
    let c = curve(&SQUARE);
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
