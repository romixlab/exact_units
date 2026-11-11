//! Properties: RF's formulas written with the crate's exact types (`Fraction`, `Exact`, `Magnitude`, `RunScale`, the
//! compile-time levels) against the same formulas on `i128` rationals (`reference::rf`), as `examples/19_rf.rs`
//! uses them.
//!
//! - Mismatch: VSWR, |Γ| back from it and the delivered power 1 − |Γ|², walked over every |Γ| = p/q with q ≤ 255;
//!   a Kani proof of the round trip on `Fraction8`. A return loss of n·ln 2 Np in mdB and µdB by every rounding,
//!   every `u8` n, against lg 2's digits.
//! - Levels: the certified brackets `from_db_const_rounded` gives every tenth of a dB from −30 to +30 dB hold the
//!   power ratio 10^(dB/10) (checked in big integers), one step wide, and the `f64` edge lands in them; Friis' cascade
//!   on drawn rationals; kT₀ exact, −174 dBm/Hz and −204 dBW/Hz roundings of it.
//! - Fractional-N: a Σ-Δ synthesizer's N·2^m + K by every rounding against the reference's division, its output a
//!   whole number of steps; ADF4351's eq. (3) for every FRAC of three moduli.
//! - Two-ports: S of a drawn ladder of series and shunt elements against Pozar's Table 4.2 on `i128`, S → ABCD → S
//!   exact, Z and Y refused exactly where C or B is 0; a lossless line's Z_in against the reference and |Γ| kept.
//! - Free space and skin depth: η₀ under each set, 120π under none, η₀ε₀c = 1; δ²·πfμ₀/ρ = 1 for drawn ρ and f.
//! - Phase noise: a flat L(f) of a whole number of 10 dB integrates exactly; σ_t² (2πf₀)² = 2A.
#![expect(
    clippy::arithmetic_side_effects,
    reason = "a test computes with the values it chose, which fit; an overflow fails the test"
)]
#![expect(
    clippy::unwrap_used,
    reason = "a property fails by panicking on a value it knows is there"
)]

use super::reference::rf::{self as reference, Q};
use exact_units::constants::{Ampere1948, C, Codata2018, Codata2022, EPSILON0, ETA0, K_B};
use exact_units::decibel::Decibels;
use exact_units::prefix::{Micro, Milli, Nano};
use exact_units::units::{Decibel, Hertz, Loss, Neper, Per, Watt};
use exact_units::{Exact, Fixed, Fraction, Magnitude, Rounding, RunScale};

/// A power ratio in steps of 10⁻⁹.
type Nano64 = Fixed<Per<Watt, Watt>, Nano, u64>;

/// The crate's fraction of a reference one.
fn frac(q: Q) -> Fraction {
    Fraction::new(q.0, q.1.unsigned_abs())
}

/// VSWR = (1 + |Γ|)/(1 − |Γ|), as the example writes it.
fn vswr(g: Fraction) -> Option<Fraction> {
    let one = Fraction::int(1);
    one.checked_add(g)?.checked_div(one.checked_sub(g)?)
}

/// |Γ| = (S − 1)/(S + 1).
fn gamma_of_vswr(s: Fraction) -> Option<Fraction> {
    let one = Fraction::int(1);
    s.checked_sub(one)?.checked_div(s.checked_add(one)?)
}

/// |Γ| = p/q: the VSWR and the delivered power against the reference, and |Γ| back from the VSWR.
pub(crate) fn mismatch_is_exact(p: u8, q: u8) {
    if p >= q {
        return;
    }
    let (pi, qi) = (i128::from(p), i128::from(q));
    let g = Fraction::new(pi, u128::from(q));
    let s = vswr(g).unwrap();
    assert_eq!(s, frac(reference::vswr(pi, qi)), "VSWR of {p}/{q}");
    assert_eq!(
        gamma_of_vswr(s),
        Some(g),
        "|Γ| back from the VSWR of {p}/{q}"
    );
    let delivered = Fraction::int(1)
        .checked_sub(g.checked_mul(g).unwrap())
        .unwrap();
    assert_eq!(
        delivered,
        frac(reference::delivered(pi, qi)),
        "1 − |Γ|² of {p}/{q}"
    );
}

#[test]
fn walk_mismatch() {
    for q in 1..=u8::MAX {
        for p in 0..q {
            mismatch_is_exact(p, q);
        }
    }
}

/// The round trip on `Fraction8`: where the VSWR of p/q fits, |Γ| comes back from it, and it is (q + p)/(q − p).
pub(crate) fn mismatch_round_trip_8(p: u8, q: u8) {
    use exact_units::Fraction8;
    if p >= q {
        return;
    }
    let one = Fraction8::from_parts(false, 1, 1).unwrap();
    let g = Fraction8::from_parts(false, p, q).unwrap();
    let Some(s) = one
        .checked_add(g)
        .and_then(|n| one.checked_sub(g).and_then(|d| n.checked_div(d)))
    else {
        return;
    };
    let want = super::reference::rf::vswr(i128::from(p), i128::from(q));
    if let (Ok(n), Ok(d)) = (u8::try_from(want.0), u8::try_from(want.1)) {
        assert_eq!(
            Some(s),
            Fraction8::from_parts(false, n, d),
            "VSWR of {p}/{q}"
        );
    }
    let back = s
        .checked_sub(one)
        .and_then(|n| s.checked_add(one).and_then(|d| n.checked_div(d)));
    if let Some(back) = back {
        assert_eq!(back, g, "|Γ| back from the VSWR of {p}/{q}");
    }
}

#[test]
fn walk_mismatch_8() {
    for q in 1..=u8::MAX {
        for p in 0..q {
            mismatch_round_trip_8(p, q);
        }
    }
}

exact_units::scale! {
    /// ln 2: a return loss of |Γ| = 2⁻ⁿ is n of it in nepers.
    pub struct Ln2 = "", Exact(1 / 1, ln2 1);
}

/// A return loss of n·ln 2 Np (|Γ| = 2⁻ⁿ) in mdB and µdB, by Floor and Ceil against lg 2's digits, and by the
/// nearest modes within half a step.
#[test]
fn walk_return_loss_of_pow2() {
    for n in 0..=u8::MAX {
        let rl = Fixed::<Loss<Neper>, Ln2, u8>::new(n);
        let milli = |r| {
            rl.convert_rounded::<Loss<Decibel>, Milli, i64>(r)
                .unwrap()
                .get()
        };
        let micro = |r| {
            rl.convert_rounded::<Loss<Decibel>, Micro, i64>(r)
                .unwrap()
                .get()
        };
        let n = i128::from(n);
        for (k, get) in [
            (3, &milli as &dyn Fn(Rounding) -> i64),
            (6, &micro as &dyn Fn(Rounding) -> i64),
        ] {
            let (floor, ceil) = reference::db_of_pow2(n, k);
            assert_eq!(
                i128::from(get(Rounding::Floor)),
                floor,
                "⌊{n}·20·lg 2⌋ at 10^-{k}"
            );
            assert_eq!(
                i128::from(get(Rounding::Ceil)),
                ceil,
                "⌈{n}·20·lg 2⌉ at 10^-{k}"
            );
            let nearest = i128::from(get(Rounding::NearestEven));
            assert!(
                nearest == floor || nearest == ceil,
                "nearest of {n}·20·lg 2"
            );
        }
    }
}

/// The gain at `k/10` dB, a power ratio, rounded down and up: certified to hold 10^(k/100), one step wide, and the
/// `f64` edge 10^(k/100) within a step of the bracket.
pub(crate) fn db_bracket_holds(k: i16) {
    let db = Decibels::new(i64::from(k), 1);
    let lo = Nano64::from_db_const_rounded(db, Rounding::Floor).get();
    let hi = Nano64::from_db_const_rounded(db, Rounding::Ceil).get();
    let k = i32::from(k);
    assert!(
        reference::below_pow10_centi(lo, k, false),
        "the floor of {k}/10 dB is above it"
    );
    assert!(
        !reference::below_pow10_centi(hi, k, true),
        "the ceiling of {k}/10 dB is below it"
    );
    assert!(
        hi - lo <= 1,
        "the bracket of {k}/10 dB is wider than a step"
    );
    #[expect(
        clippy::cast_precision_loss,
        clippy::as_conversions,
        reason = "a nanostep count below 2⁵³"
    )]
    let (lo_f, hi_f) = (lo as f64 * 1e-9, hi as f64 * 1e-9);
    let edge = 10f64.powf(f64::from(k) / 100.0);
    assert!(
        edge >= lo_f - 1e-9 && edge <= hi_f + 1e-9,
        "10^({k}/100) in f64 is outside [{lo_f}, {hi_f}]"
    );
}

#[test]
fn walk_db_brackets() {
    for k in -300..=300 {
        db_bracket_holds(k);
    }
}

/// Friis' cascade on the crate's fractions, as the example writes it.
fn friis(stages: &[(Fraction, Fraction)]) -> Option<Fraction> {
    let one = Fraction::int(1);
    let (mut f, mut g) = (Fraction::int(0), one);
    for (i, &(fi, gi)) in stages.iter().enumerate() {
        let term = if i == 0 {
            fi
        } else {
            fi.checked_sub(one)?.checked_div(g)?
        };
        f = f.checked_add(term)?;
        g = g.checked_mul(gi)?;
    }
    Some(f)
}

/// Friis' cascade of stages (F = 1 + a/b, G = c/d) against the reference.
pub(crate) fn friis_is_exact(stages: &[(u8, u8, u8, u8)]) {
    let ours: Vec<_> = stages
        .iter()
        .map(|&(a, b, c, d)| {
            let f = Fraction::int(1)
                .checked_add(Fraction::new(i128::from(a), u128::from(b.max(1))))
                .unwrap();
            (f, Fraction::new(i128::from(c.max(1)), u128::from(d.max(1))))
        })
        .collect();
    let theirs: Vec<_> = stages
        .iter()
        .map(|&(a, b, c, d)| {
            let f = Q::int(1).add(Q::new(i128::from(a), i128::from(b.max(1))));
            (f, Q::new(i128::from(c.max(1)), i128::from(d.max(1))))
        })
        .collect();
    assert_eq!(
        friis(&ours),
        reference::friis(&theirs).map(frac),
        "Friis of {stages:?}"
    );
}

#[test]
fn kt0_is_exact() {
    // k_B · 290 K, the reference temperature of ITU-R P.372-18 §1.2.1
    let kt0 = Exact::<Per<Watt, Hertz>>::from_scaled(Fraction::int(290), &K_B).unwrap();
    assert_eq!(
        kt0.get(),
        Fraction::int(40_038_821)
            .checked_mul(Fraction::pow10(-28))
            .unwrap()
    );
    let per_milliwatt =
        Fixed::<Per<Watt, Watt>, exact_units::prefix::Pow10<-25>, u64>::new(40_038_821);
    assert_eq!(
        Exact::from_fixed(per_milliwatt).unwrap().get(),
        kt0.get().checked_mul(Fraction::int(1000)).unwrap()
    );
    #[cfg(feature = "q")]
    {
        use exact_units::q;
        // −174 dBm/Hz is a rounding: kT₀ is between −173.98 and −173.97 dBm/Hz
        assert!(
            per_milliwatt > q!(-174 dB)
                && per_milliwatt > q!(-173.98 dB)
                && per_milliwatt < q!(-173.97 dB)
        );
        // P.372-18's −204 dBW/Hz the same
        let per_watt =
            Fixed::<Per<Watt, Watt>, exact_units::prefix::Pow10<-28>, u64>::new(40_038_821);
        assert!(per_watt > q!(-204 dB) && per_watt < q!(-203.97 dB));
    }
}

/// A Σ-Δ synthesizer of modulus 2^m on `pfd` Hz, asked for `target` Hz: N·2^m + K by `r` is the reference's
/// rounded division, and the output is that many steps of pfd/2^m, exactly.
pub(crate) fn sigma_delta_is_exact(pfd: u32, target: u64, m: u8, r: Rounding) {
    let (pfd, m) = (pfd.max(1), m.clamp(1, 30));
    let step = RunScale::from_u64(u64::from(pfd), 1 << m).unwrap();
    let step_f = Fraction::from_magnitude(&step.to_magnitude()).unwrap();
    let nk = Fraction::int(i128::from(target))
        .checked_div(step_f)
        .unwrap()
        .round(r)
        .unwrap();
    let want = super::reference::round_div(i128::from(target) << m, i128::from(pfd), r);
    assert_eq!(nk, want, "N·2^{m} + K of {target} Hz on {pfd} Hz by {r:?}");
    let out = Exact::<Hertz>::from_scaled(Fraction::int(nk), &step.to_magnitude()).unwrap();
    assert_eq!(
        out.get(),
        Fraction::new(nk * i128::from(pfd), 1u128 << m),
        "the output of N·2^{m} + K = {nk}"
    );
}

/// ADF4351's eq. (3), RF_OUT = (INT + FRAC/MOD)·f_PFD / RF divider, against the reference for every FRAC.
#[test]
fn walk_adf4351() {
    let pfd = Exact::<Hertz>::new_in::<exact_units::prefix::Mega>(10, 1);
    for modulus in [25u16, 65, 4095] {
        for frac_ in 0..modulus {
            let n = Fraction::int(422)
                .checked_add(Fraction::new(i128::from(frac_), u128::from(modulus)))
                .unwrap();
            let out = ((pfd * n).unwrap() / 2).unwrap();
            let m = i128::from(modulus);
            let want = Q::new((422 * m + i128::from(frac_)) * 10_000_000, m * 2);
            assert_eq!(
                out.get(),
                frac(want),
                "INT 422, FRAC {frac_}, MOD {modulus}"
            );
        }
    }
    // the datasheet's worked example: 2112.6 MHz
    let n = Fraction::int(422)
        .checked_add(Fraction::new(13, 25))
        .unwrap();
    assert_eq!(
        ((pfd * n).unwrap() / 2).unwrap(),
        Exact::new_in::<exact_units::prefix::Mega>(21_126, 10)
    );
}

/// A complex fraction, as the example's `Cq`.
#[derive(Copy, Clone, Debug, PartialEq)]
struct Cq(Fraction, Fraction);

impl Cq {
    fn int(re: i128, im: i128) -> Cq {
        Cq(Fraction::int(re), Fraction::int(im))
    }

    fn of(c: reference::Cq) -> Cq {
        Cq(frac(c.0), frac(c.1))
    }

    fn add(self, o: Cq) -> Option<Cq> {
        Some(Cq(self.0.checked_add(o.0)?, self.1.checked_add(o.1)?))
    }

    fn sub(self, o: Cq) -> Option<Cq> {
        Some(Cq(self.0.checked_sub(o.0)?, self.1.checked_sub(o.1)?))
    }

    fn mul(self, o: Cq) -> Option<Cq> {
        Some(Cq(
            self.0
                .checked_mul(o.0)?
                .checked_sub(self.1.checked_mul(o.1)?)?,
            self.0
                .checked_mul(o.1)?
                .checked_add(self.1.checked_mul(o.0)?)?,
        ))
    }

    fn div(self, o: Cq) -> Option<Cq> {
        let d = o.0.checked_mul(o.0)?.checked_add(o.1.checked_mul(o.1)?)?;
        let n = self.mul(Cq(o.0, o.1.negate()))?;
        Some(Cq(n.0.checked_div(d)?, n.1.checked_div(d)?))
    }

    fn norm_sqr(self) -> Option<Fraction> {
        self.0
            .checked_mul(self.0)?
            .checked_add(self.1.checked_mul(self.1)?)
    }
}

type M2 = [[Cq; 2]; 2];

fn cascade(a: &M2, b: &M2) -> Option<M2> {
    let [[a11, a12], [a21, a22]] = *a;
    let [[b11, b12], [b21, b22]] = *b;
    let e = |x: Cq, y: Cq, z: Cq, w: Cq| x.mul(y)?.add(z.mul(w)?);
    Some([
        [e(a11, b11, a12, b21)?, e(a11, b12, a12, b22)?],
        [e(a21, b11, a22, b21)?, e(a21, b12, a22, b22)?],
    ])
}

fn det(m: &M2) -> Option<Cq> {
    let [[a, b], [c, d]] = *m;
    a.mul(d)?.sub(b.mul(c)?)
}

/// S from ABCD, as the example writes it.
fn s_of_abcd(m: &M2, z0: Fraction) -> Option<M2> {
    let [[a, b], [c, d]] = *m;
    let z = Cq(z0, Fraction::int(0));
    let bz = b.div(z)?;
    let cz = c.mul(z)?;
    let den = a.add(bz)?.add(cz)?.add(d)?;
    let two = Cq::int(2, 0);
    Some([
        [
            a.add(bz)?.sub(cz)?.sub(d)?.div(den)?,
            two.mul(det(m)?)?.div(den)?,
        ],
        [two.div(den)?, d.sub(a)?.add(bz)?.sub(cz)?.div(den)?],
    ])
}

/// ABCD from S, as the example writes it.
fn abcd_of_s(s: &M2, z0: Fraction) -> Option<M2> {
    let [[s11, s12], [s21, s22]] = *s;
    let (one, z) = (Cq::int(1, 0), Cq(z0, Fraction::int(0)));
    let den = s21.mul(Cq::int(2, 0))?;
    let x = s12.mul(s21)?;
    Some([
        [
            one.add(s11)?.mul(one.sub(s22)?)?.add(x)?.div(den)?,
            one.add(s11)?.mul(one.add(s22)?)?.sub(x)?.div(den)?.mul(z)?,
        ],
        [
            one.sub(s11)?.mul(one.sub(s22)?)?.sub(x)?.div(den)?.div(z)?,
            one.sub(s11)?.mul(one.add(s22)?)?.add(x)?.div(den)?,
        ],
    ])
}

/// Z from ABCD: `None` where C is 0.
fn z_of_abcd(m: &M2) -> Option<M2> {
    let [[a, _], [c, d]] = *m;
    Some([
        [a.div(c)?, det(m)?.div(c)?],
        [Cq::int(1, 0).div(c)?, d.div(c)?],
    ])
}

/// Y from ABCD: `None` where B is 0.
fn y_of_abcd(m: &M2) -> Option<M2> {
    let [[a, b], [_, d]] = *m;
    let zero = Cq::int(0, 0);
    Some([
        [d.div(b)?, zero.sub(det(m)?.div(b)?)?],
        [zero.sub(Cq::int(1, 0).div(b)?)?, a.div(b)?],
    ])
}

/// A ladder of elements `(series, re, im)`, each a series impedance or a shunt admittance of re + j·im over 8:
/// S over `z0` against Pozar's Table 4.2 on `i128`, ABCD back from S, and Z, Y refused exactly where C, B is 0.
pub(crate) fn ladder_is_exact(elements: &[(bool, i8, i8)], z0: u8) {
    let z0 = i128::from(z0.max(1));
    let one = reference::Cq::int(1, 0);
    let zero = reference::Cq::int(0, 0);
    let mut theirs = [[one, zero], [zero, one]];
    let mut ours = [
        [Cq::int(1, 0), Cq::int(0, 0)],
        [Cq::int(0, 0), Cq::int(1, 0)],
    ];
    for &(is_series, re, im) in elements {
        let x = reference::Cq(Q::new(i128::from(re), 8), Q::new(i128::from(im), 8));
        let el = if is_series {
            [[one, x], [zero, one]]
        } else {
            [[one, zero], [x, one]]
        };
        let [[a11, a12], [a21, a22]] = theirs;
        let [[b11, b12], [b21, b22]] = el;
        theirs = [
            [
                a11.mul(b11).add(a12.mul(b21)),
                a11.mul(b12).add(a12.mul(b22)),
            ],
            [
                a21.mul(b11).add(a22.mul(b21)),
                a21.mul(b12).add(a22.mul(b22)),
            ],
        ];
        ours = cascade(&ours, &el.map(|row| row.map(Cq::of))).unwrap();
    }
    assert_eq!(
        ours,
        theirs.map(|row| row.map(Cq::of)),
        "ABCD of {elements:?}"
    );
    let z = Fraction::int(z0);
    let s = s_of_abcd(&ours, z);
    assert_eq!(
        s,
        reference::s_of_abcd(theirs, Q::int(z0)).map(|m| m.map(|row| row.map(Cq::of))),
        "S of {elements:?} on {z0} Ω"
    );
    // exact or refused: a product past `Fraction`'s u128 refuses, never rounds
    if let Some(back) = s.and_then(|s| abcd_of_s(&s, z)) {
        assert_eq!(back, ours, "ABCD back from S of {elements:?}");
    }
    let [[_, b], [c, _]] = ours;
    assert_eq!(
        z_of_abcd(&ours).is_none(),
        c == Cq::int(0, 0),
        "Z of {elements:?}"
    );
    assert_eq!(
        y_of_abcd(&ours).is_none(),
        b == Cq::int(0, 0),
        "Y of {elements:?}"
    );
}

/// A lossless line of `z0` Ω with tan βl = t loaded by Z_L: Z_in against the reference, and |Γ| at its input that
/// of the load.
pub(crate) fn line_is_exact(z0: u8, re: i8, im: i8, t: (i8, u8)) {
    let z0 = i128::from(z0.max(1));
    let t = Q::new(i128::from(t.0), i128::from(t.1.max(1)));
    let zl = reference::Cq::int(i128::from(re), i128::from(im));
    let Some(want) = reference::z_in(Q::int(z0), zl, t) else {
        return;
    };
    let (z, jt) = (Cq::int(z0, 0), Cq(Fraction::int(0), frac(t)));
    let zl_ = Cq::of(zl);
    let got = z
        .mul(zl_.add(jt.mul(z).unwrap()).unwrap())
        .unwrap()
        .div(z.add(jt.mul(zl_).unwrap()).unwrap())
        .unwrap();
    assert_eq!(
        got,
        Cq::of(want),
        "Z_in of {zl:?} through tan βl = {t:?} on {z0} Ω"
    );
    let gamma = |x: Cq| x.sub(z)?.div(x.add(z)?)?.norm_sqr();
    if let (Some(gin), Some(gl)) = (gamma(got), gamma(zl_)) {
        assert_eq!(gin, gl, "|Γ|² at the input of a lossless line");
    }
}

#[test]
fn eta0_under_each_set() {
    // μ₀ as each set prints it, times the exact c
    for (mu0, eta0) in [
        (
            Magnitude::new(125_663_706_127, 1, -17, 0, 0),
            ETA0.resolve::<Codata2022>().value,
        ),
        (
            Magnitude::new(125_663_706_212, 1, -17, 0, 0),
            ETA0.resolve::<Codata2018>().value,
        ),
    ] {
        assert_eq!(eta0, mu0.mul(C));
    }
    // the 1948 ampere: μ₀ = 4π·10⁻⁷, so η₀ = 119.9169832·π Ω
    let old = ETA0.resolve::<Ampere1948>().value;
    assert_eq!(old, Magnitude::new(1_199_169_832, 1, -7, 0, 1));
    let pi120 = Magnitude::new(120, 1, 0, 0, 1);
    for set in [
        old,
        ETA0.resolve::<Codata2018>().value,
        ETA0.resolve::<Codata2022>().value,
    ] {
        assert_ne!(set, pi120);
    }
    // η₀ = 1/(ε₀c), with no set
    assert_eq!(ETA0.mul(EPSILON0).mul(C), Magnitude::ONE);
}

/// δ = √(ρ/(πfμ₀)) of ρ = a/b Ω·m and f = m·10^e Hz: δ²·πfμ₀/ρ is 1, and a hundred times f is a tenth of δ.
pub(crate) fn skin_depth_is_exact(a: u16, b: u16, m: u16, e: u8) {
    let (a, b, m, e) = (a.max(1), b.max(1), m.max(1), i8::try_from(e % 10).unwrap());
    let rho = Magnitude::ratio(u64::from(a), u64::from(b));
    let pi_f_mu0 = |e: i8| Magnitude::new(u64::from(m), 1, e, 0, 1).mul(Magnitude::MU0);
    let delta = rho.div(pi_f_mu0(e)).sqrt();
    assert_eq!(delta.mul(delta).mul(pi_f_mu0(e)).div(rho), Magnitude::ONE);
    let higher = rho.div(pi_f_mu0(e + 2)).sqrt();
    assert_eq!(delta.div(higher), Magnitude::ratio(10, 1));
}

/// A flat L(f) of −10·j dBc/Hz over `band` Hz: its gain is 10⁻ʲ exactly, the area A = 10⁻ʲ·band, and σ_t =
/// √(2A)/(2πf₀) squares back to 2A/(2πf₀)².
pub(crate) fn flat_phase_noise_is_exact(j: u8, band: u32, f0_mhz: u16) {
    let (j, band, f0) = (i8::try_from(j % 19).unwrap(), band.max(1), f0_mhz.max(1));
    let l = Magnitude::pow10(-j);
    let a = l.mul(Magnitude::ratio(u64::from(band), 1));
    assert_eq!(
        Fraction::from_magnitude(&a).unwrap(),
        Fraction::int(i128::from(band))
            .checked_mul(Fraction::pow10(-i32::from(j)))
            .unwrap()
    );
    let two_pi_f0 = Magnitude::new(2 * u64::from(f0), 1, 6, 0, 1);
    let two_a = a.mul(Magnitude::ratio(2, 1));
    let sigma_t = two_a.sqrt().div(two_pi_f0);
    assert_eq!(sigma_t.mul(sigma_t).mul(two_pi_f0).mul(two_pi_f0), two_a);
}

#[test]
fn flat_phase_noise_of_mt008() {
    // MT-008 Figure 5: −150 dBc/Hz from 10 kHz to 200 MHz on 100 MHz: 1.0066 ps
    flat_phase_noise_is_exact(15, 199_990_000, 100);
    let t = Magnitude::new(2 * 19_999, 1, -11, 0, 0)
        .sqrt()
        .div(Magnitude::new(2, 1, 8, 0, 1))
        .to_f64();
    assert!((t * 1e12 - 1.0066).abs() < 1e-4);
}

#[cfg(not(any(kani, fuzzing)))]
mod props {
    use super::super::reference::MODES;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn friis(stages in prop::collection::vec(any::<(u8, u8, u8, u8)>(), 1..=5)) {
            super::friis_is_exact(&stages);
        }

        #[test]
        fn sigma_delta(pfd in 1_000_000u32..=100_000_000, target in 100_000_000u64..=6_000_000_000, m in 1u8..=30, r in prop::sample::select(&MODES[..])) {
            super::sigma_delta_is_exact(pfd, target, m, r);
        }

        #[test]
        fn ladder(elements in prop::collection::vec((any::<bool>(), -16i8..=16, -16i8..=16), 0..=4), z0 in 1u8..=100) {
            super::ladder_is_exact(&elements, z0);
        }

        #[test]
        fn line(z0: u8, re: i8, im: i8, t: (i8, u8)) {
            super::line_is_exact(z0, re, im, t);
        }

        #[test]
        fn skin_depth(a: u16, b: u16, m: u16, e: u8) {
            super::skin_depth_is_exact(a, b, m, e);
        }

        #[test]
        fn flat_phase_noise(j: u8, band: u32, f0: u16) {
            super::flat_phase_noise_is_exact(j, band, f0);
        }
    }
}

#[cfg(kani)]
mod proofs {
    #[kani::proof]
    #[kani::unwind(10)]
    fn mismatch_round_trip_8() {
        let (p, q): (u8, u8) = (kani::any(), kani::any());
        kani::assume(q <= 15);
        super::mismatch_round_trip_8(p, q);
    }
}
