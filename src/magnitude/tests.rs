use super::*;
use core::f64::consts::{FRAC_PI_2, LN_10, LOG2_10, PI};
use std::string::ToString;
use std::vec::Vec;

/// The pound, 0.45359237 kg exactly (1959 yard and pound agreement).
const LB: Magnitude = Magnitude::ratio(45359237, 100_000_000);

/// Standard gravity, 9.80665 m/s² (CGPM 1901).
const GN: Magnitude = Magnitude::ratio(980665, 100_000);

/// Pound-force: lb · g_n.
const LBF: Magnitude = LB.mul(GN);

/// The inch, 0.0254 m.
const IN: Magnitude = Magnitude::ratio(254, 10_000);

/// psi = lbf / in².
const PSI: Magnitude = LBF.div(IN.powi(2));

#[test]
fn equal_however_spelled() {
    assert_eq!(Magnitude::pow10(-3), Magnitude::ratio(1, 1000));
    assert_eq!(Magnitude::ratio(2000, 2_000_000), Magnitude::pow10(-3));
    assert_eq!(Magnitude::ratio(1024, 1), Magnitude::pow2(10));
    assert_eq!(
        Magnitude::ratio(5, 1),
        Magnitude::pow10(1).div(Magnitude::pow2(1))
    );
    assert_eq!(
        Magnitude::new(1, 180, 0, 0, 1),
        Magnitude::new(2, 360, 0, 0, 1)
    );
    assert_eq!(
        Magnitude::PI.div(Magnitude::ratio(180, 1)),
        Magnitude::new(1, 180, 0, 0, 1)
    );
    assert_eq!(
        Magnitude::pow10(-3).div(Magnitude::pow10(-6)),
        Magnitude::pow10(3)
    );
    assert_eq!(Magnitude::pow10(0), Magnitude::ONE);
    assert_eq!(Magnitude::ratio(7, 7), Magnitude::ONE);
}

#[test]
fn factors() {
    let duty = Magnitude::ratio(1, 65535);
    // 65535 = 3 · 5 · 17 · 257
    let p: Vec<_> = duty.primes().map(|f| (f.prime, f.exp)).collect();
    assert_eq!(p, [(3, -2), (17, -2), (257, -2)]);
    assert_eq!(duty.exp_x2(5), -2);
    assert_eq!(duty.exp_x2(2), 0);
    // 45359237 = 7 · 11 · 97 · 6073
    let p: Vec<_> = LB.primes().map(|f| f.prime).collect();
    assert_eq!(p, [7, 11, 97, 6073]);
    assert_eq!((LB.exp_x2(2), LB.exp_x2(5)), (-16, -16));
    // US survey foot 1200/3937 m, 3937 = 31 · 127
    let survey_ft = Magnitude::ratio(1200, 3937);
    assert_eq!(survey_ft.exp_x2(31), -2);
    assert_eq!(survey_ft.exp_x2(127), -2);
    assert_eq!(survey_ft.exp_x2(3), 2);
    // eV 1.602176634e-19 J: 1602176634 = 2 · 3² · 19 · 389 · 12043
    let ev = Magnitude::ratio(1602176634, 1).mul(Magnitude::pow10(-28));
    let p: Vec<_> = ev.primes().map(|f| (f.prime, f.exp)).collect();
    assert_eq!(p, [(3, 4), (19, 2), (389, 2), (12043, 2)]);
    assert_eq!(ev.to_ratio(), Some((801088317, 5 * 10u128.pow(27))));
    // g_n = 5 · 7 · 28019 · 10⁻⁵ · ... : 980665 = 5 · 7 · 28019
    assert_eq!(GN.exp_x2(28019), 2);
}

#[test]
fn closed_under_products() {
    // psi² and (lbf·s)³ overflow u64 / u128 as num/den, not as prime powers
    let psi2 = PSI.powi(2);
    assert_eq!(psi2.div(PSI), PSI);
    let cube = LBF.powi(3);
    assert_eq!(cube.to_ratio(), None, "(lbf·s)³ doesn't fit u128");
    assert_eq!(cube.div(LBF).div(LBF), LBF);
    assert_eq!(cube.exp_x2(28019), 6);
    // x · y / y = x for every pair
    let all = [
        LB,
        GN,
        LBF,
        IN,
        PSI,
        Magnitude::PI,
        Magnitude::LN2,
        Magnitude::ratio(1, 65535),
    ];
    for a in all {
        for b in all {
            assert_eq!(a.mul(b).div(b), a);
            assert_eq!(a.div(b).mul(b), a);
            assert_eq!(a.mul(b), b.mul(a));
        }
        assert_eq!(a.mul(a.recip()), Magnitude::ONE);
    }
}

#[test]
fn irrational_and_roots() {
    let deg = Magnitude::new(1, 180, 0, 0, 1);
    assert_eq!(deg.symbols_x2().pi, 2);
    assert!(!deg.is_rational());
    assert_eq!(Magnitude::PI.powi(-2).symbols_x2().pi, -4);
    let sqrt2 = Magnitude::ratio(2, 1).sqrt();
    assert_eq!(sqrt2.mul(sqrt2), Magnitude::ratio(2, 1));
    assert_eq!(sqrt2.to_ratio(), None);
    assert_eq!(sqrt2.checked_sqrt(), Err(MagnitudeError::NotSquare));
    // √(mHz): 10^(-3/2)
    let sqrt_milli = Magnitude::pow10(-3).sqrt();
    assert_eq!(sqrt_milli.powi(2), Magnitude::pow10(-3));
    assert_eq!(sqrt_milli.exp_x2(5), -3);
    assert!((sqrt_milli.to_f64() - 0.001f64.sqrt()).abs() < 1e-17);
    // a decibel in neper: ln10 / 20; a decade in octaves: ln10 / ln2
    let db = Magnitude::LN10.div(Magnitude::ratio(20, 1));
    assert!((db.to_f64() - LN_10 / 20.0).abs() < 1e-16);
    let decade = Magnitude::LN10.div(Magnitude::LN2);
    assert!((decade.to_f64() - LOG2_10).abs() < 1e-15);
    assert_eq!(
        decade.symbols_x2(),
        Symbols {
            ln2: -2,
            ln10: 2,
            ..Symbols::NONE
        }
    );
}

#[test]
fn large_and_refused() {
    // two primes above 2¹⁶: found by Pollard's rho
    let m = Magnitude::ratio(65537 * 65539, 1);
    assert_eq!((m.exp_x2(65537), m.exp_x2(65539)), (2, 2));
    let m = Magnitude::ratio(65537 * 65539 * 65543, 1);
    assert_eq!(m.exp_x2(65543), 2);
    // the largest u32 prime is a factor, a larger one is refused
    assert_eq!(Magnitude::ratio(4294967291, 1).exp_x2(4294967291), 2);
    assert_eq!(
        Magnitude::checked_ratio((1 << 61) - 1, 1),
        Err(MagnitudeError::PrimeTooLarge)
    );
    assert_eq!(Magnitude::checked_ratio(0, 1), Err(MagnitudeError::Zero));
    assert_eq!(Magnitude::checked_ratio(1, 0), Err(MagnitudeError::Zero));
    // 17 primes other than 2 and 5: 13 in one u64, 4 more by a product
    let n: u64 = [3u64, 7, 11, 13, 17, 19, 23, 29, 31, 37, 41, 43, 47]
        .iter()
        .product();
    let m = Magnitude::ratio(n, 1);
    assert_eq!(
        m.checked_mul(Magnitude::ratio(53 * 59 * 61 * 67, 1)),
        Err(MagnitudeError::TooManyFactors)
    );
    assert!(m.checked_mul(Magnitude::ratio(53 * 59 * 61, 1)).is_ok());
    assert_eq!(Magnitude::checked_pow10(64), None);
    assert!(Magnitude::checked_pow10(-63).is_some());
    assert_eq!(
        Magnitude::pow10(40).checked_mul(Magnitude::pow10(30)),
        Err(MagnitudeError::ExponentRange)
    );
    // pow10(-64) has no pow10(64) to be its reciprocal: negating the doubled exponent leaves an i8
    assert_eq!(
        Magnitude::pow10(-64).checked_recip(),
        Err(MagnitudeError::ExponentRange)
    );
    assert_eq!(
        Magnitude::pow10(-63).checked_recip(),
        Ok(Magnitude::pow10(63))
    );
    assert_eq!(
        Magnitude::pow2(10).checked_recip(),
        Ok(Magnitude::pow2(-10))
    );
    assert_eq!(
        Magnitude::checked_pow2(64),
        Err(MagnitudeError::ExponentRange)
    );
    assert!(Magnitude::checked_pow2(63).is_ok());
    assert_eq!(
        Magnitude::pow10(-64).checked_powi(2),
        Err(MagnitudeError::ExponentRange)
    );
    assert_eq!(
        Magnitude::pow10(-64).checked_powi(1),
        Ok(Magnitude::pow10(-64))
    );
    assert_eq!(u64::MAX, 3 * 5 * 17 * 257 * 641 * 65537 * 6700417);
    let m = Magnitude::ratio(u64::MAX, 1);
    assert_eq!(m.to_ratio(), Some((u128::from(u64::MAX), 1)));
}

#[test]
fn primality() {
    let small: Vec<u64> = (0..100).filter(|&n| is_prime(n)).collect();
    assert_eq!(
        small,
        [
            2, 3, 5, 7, 11, 13, 17, 19, 23, 29, 31, 37, 41, 43, 47, 53, 59, 61, 67, 71, 73, 79, 83,
            89, 97
        ]
    );
    assert!(is_prime((1 << 61) - 1));
    assert!(!is_prime(3215031751)); // strong pseudoprime to 2, 3, 5, 7
    assert!(!is_prime(65537 * 65539));
}

/// The three bases of `is_prime_u32` against the twelve of `is_prime`: every number below 2²⁰, every 4099th number
/// up to 2³², the top of the range, and strong pseudoprimes to bases 2 (2047 ..), 2 and 3 (1373653), 2, 3 and 5
/// (25326001) and 2, 3, 5 and 7 (3215031751); none is one to 2, 7 and 61 below 4,759,123,141.
#[test]
fn primality_u32() {
    let check = |n: u32| assert_eq!(is_prime_u32(n), is_prime(u64::from(n)), "{n}");
    (0..1 << 20).for_each(check);
    (0..=u32::MAX).step_by(4099).for_each(check);
    (u32::MAX - (1 << 16)..=u32::MAX).for_each(check);
    for n in [
        2047, 3277, 4033, 4681, 8321, 15841, 29341, 42799, 49141, 52633, 65281, 74665, 80581,
        85489, 88357, 90751, 1373653, 25326001, 3215031751,
    ] {
        check(n);
        assert!(!is_prime_u32(n), "{n}");
    }
    assert!(is_prime_u32(u32::MAX - 4)); // 4294967291, the largest u32 prime
    assert!(!is_prime_u32(65521 * 65519));
}

#[test]
fn apply() {
    let duty = Magnitude::ratio(1, 65535);
    assert_eq!(duty.recip().apply_int(1), Some(65535));
    assert_eq!(duty.apply_int(65535), Some(1));
    assert_eq!(duty.apply_int(32767), None);
    assert_eq!(Magnitude::pow2(10).apply_int(3), Some(3072));
    assert_eq!(Magnitude::pow10(-3).apply_int(1500), None);
    assert_eq!(Magnitude::pow10(-3).apply_int(-3000), Some(-3));
    assert_eq!(Magnitude::PI.apply_int(1), None);
    let deg = Magnitude::new(1, 180, 0, 0, 1);
    assert!((deg.apply_f64(90.0).unwrap() - FRAC_PI_2).abs() < 1e-15);
    assert!((deg.to_f64() - PI / 180.0).abs() < 1e-18);
    assert_eq!(Magnitude::pow10(-1).apply_f64(30.0).unwrap(), 3.0);
    assert_eq!(LB.to_ratio(), Some((45359237, 100_000_000)));
    assert_eq!(Magnitude::pow10(-3).as_pow10(), Some(-3));
    assert_eq!(Magnitude::pow2(10).as_pow10(), None);
    assert_eq!(Magnitude::pow10(-3).sqrt().as_pow10(), None);
}

#[test]
fn display() {
    let s = |m: Magnitude| m.to_string();
    assert_eq!(s(Magnitude::ONE), "");
    assert_eq!(s(Magnitude::pow10(-4)), "·10⁻⁴");
    assert_eq!(s(Magnitude::pow2(10)), "·2¹⁰");
    assert_eq!(s(Magnitude::ratio(1, 3)), "·1/3");
    assert_eq!(s(Magnitude::new(1, 180, 0, 0, 1)), "·π/180");
    assert_eq!(s(Magnitude::new(2, 1, 0, 0, 1)), "·2π");
    assert_eq!(s(Magnitude::ratio(1, 65535)), "·1/65535");
    assert_eq!(s(Magnitude::ratio(3, 1024)), "·3/1024");
    assert_eq!(s(Magnitude::ratio(2, 1).sqrt()), "·√2");
    assert_eq!(s(Magnitude::ratio(1, 2).sqrt()), "·1/√2");
    assert_eq!(s(Magnitude::LN10.div(Magnitude::ratio(20, 1))), "·ln10/20");
    assert_eq!(s(Magnitude::PI.sqrt()), "·√π");
    assert_eq!(s(Magnitude::ONE.div(Magnitude::PI)), "·1/π");
    assert_eq!(s(LB), "·45359237/100000000");
    assert_eq!(
        s(Magnitude::pow10(-60).mul(Magnitude::ratio(3, 1))),
        "·2⁻⁶⁰·5⁻⁶⁰·3"
    );
}

#[test]
fn display_of_a_root_past_u128() {
    // √(p₁·p₂·p₃·p₄·p₅) of five primes near 2³²: the root under √ is past u128, so it prints as prime powers
    let m = [4294967291, 4294967279, 4294967231, 4294967197, 4294967189]
        .map(|p| Magnitude::ratio(p, 1).sqrt())
        .into_iter()
        .fold(Magnitude::ONE, Magnitude::mul);
    assert_eq!(
        m.to_string(),
        "·4294967189¹ᐟ²·4294967197¹ᐟ²·4294967231¹ᐟ²·4294967279¹ᐟ²·4294967291¹ᐟ²"
    );
}

#[test]
fn sizes() {
    // what a `&'static Magnitude` costs a firmware that holds one at run time
    assert_eq!(size_of::<PrimePower>(), 6);
    assert_eq!(size_of::<Magnitude>(), 138);
}

#[cfg(feature = "shrink_wrap")]
#[test]
#[expect(
    clippy::too_many_lines,
    reason = "one table of wire cases, read top to bottom"
)]
fn wire() {
    let mut buf = [0u8; 64];
    let bytes = |m: Magnitude, buf: &mut [u8]| m.to_ww_bytes(buf).unwrap().to_vec();
    assert_eq!(bytes(Magnitude::pow10(-3), &mut buf), [0xfa, 0x00]);
    assert_eq!(bytes(Magnitude::ONE, &mut buf), [0x00, 0x00]);
    for m in [
        Magnitude::ONE,
        Magnitude::pow2(10),
        Magnitude::ratio(1, 65535),
        Magnitude::new(1, 180, 0, 0, 1),
        Magnitude::ratio(2, 1).sqrt(),
        LBF.powi(3),
        Magnitude::LN10.div(Magnitude::ratio(20, 1)),
    ] {
        let b = bytes(m, &mut buf);
        assert_eq!(Magnitude::from_ww_bytes(&b).unwrap(), m, "{m}");
    }
    let refused = |m: Magnitude, buf: &mut [u8]| {
        let b = bytes(m, buf);
        assert_eq!(
            Magnitude::from_ww_bytes(&b),
            Err(shrink_wrap::Error::SubtypeOutOfRange),
            "{m:?}"
        );
    };
    let mut factors = [None; MAX_FACTORS];
    // a rest that is 1
    let one_rest = MagnitudeRest {
        pow2: 0,
        symbols: None,
        factors,
    };
    refused(
        Magnitude {
            pow10: 0,
            rest: Some(one_rest),
        },
        &mut buf,
    );
    // a 2 among the primes
    factors[0] = Some(PrimePower { prime: 2, exp: 2 });
    refused(
        Magnitude {
            pow10: 0,
            rest: Some(MagnitudeRest {
                factors,
                ..one_rest
            }),
        },
        &mut buf,
    );
    // not a prime
    factors[0] = Some(PrimePower { prime: 9, exp: 2 });
    refused(
        Magnitude {
            pow10: 0,
            rest: Some(MagnitudeRest {
                factors,
                ..one_rest
            }),
        },
        &mut buf,
    );
    // not sorted
    factors[0] = Some(PrimePower { prime: 7, exp: 2 });
    factors[1] = Some(PrimePower { prime: 3, exp: 2 });
    refused(
        Magnitude {
            pow10: 0,
            rest: Some(MagnitudeRest {
                factors,
                ..one_rest
            }),
        },
        &mut buf,
    );
    // a gap before a used slot
    factors[0] = None;
    refused(
        Magnitude {
            pow10: 0,
            rest: Some(MagnitudeRest {
                factors,
                ..one_rest
            }),
        },
        &mut buf,
    );
    // an exponent 0
    factors[0] = Some(PrimePower { prime: 3, exp: 0 });
    factors[1] = None;
    refused(
        Magnitude {
            pow10: 0,
            rest: Some(MagnitudeRest {
                factors,
                ..one_rest
            }),
        },
        &mut buf,
    );
    // symbols that are all 0
    let symbols = Some(Symbols::NONE);
    refused(
        Magnitude {
            pow10: 0,
            rest: Some(MagnitudeRest {
                symbols,
                factors: [None; MAX_FACTORS],
                ..one_rest
            }),
        },
        &mut buf,
    );
}

#[test]
fn from_parts_refuses_an_exponent_sum_past_i32() {
    // 2³²/127 + 1 entries of 3¹²⁷ᐟ²: the sum of exponents passes i32 (and wraps back into range in release)
    let none = Symbols::NONE;
    let three = PrimePower { prime: 3, exp: 127 };
    let n = (1usize << 32) / 127 + 1;
    assert_eq!(
        Magnitude::from_parts_x2(0, 0, none, core::iter::repeat_n(three, n)),
        Err(MagnitudeError::ExponentRange)
    );
}

/// A prime to the power 0 is a factor of 1, dropped: from its parts (serde, a hand-edited file) the magnitude is the
/// canonical one, equal to the same without it. It was kept as a slot of exponent 0, unequal to `ONE` and refused by
/// the wire's check (found by tests/props/wire.rs).
#[test]
fn a_prime_to_the_power_0_is_dropped() {
    let none = Symbols::NONE;
    let zero = PrimePower { prime: 3, exp: 0 };
    assert_eq!(
        Magnitude::from_parts_x2(0, 0, none, [zero]),
        Ok(Magnitude::ONE)
    );
    let seven = PrimePower { prime: 7, exp: 2 };
    assert_eq!(
        Magnitude::from_parts_x2(-6, 0, none, [zero, seven]),
        Magnitude::from_parts_x2(-6, 0, none, [seven])
    );
    assert_eq!(
        Magnitude::from_parts_x2(0, 0, none, [zero]).map(|m| m.primes().count()),
        Ok(0)
    );
}

#[test]
fn reduced_ratios_and_irrationals() {
    // a prime on both sides cancels, however it is spelled
    assert_eq!(Magnitude::ratio(7, 49), Magnitude::ratio(1, 7));
    assert_eq!(Magnitude::ratio(7, 49).to_ratio(), Some((1, 7)));
    assert_eq!(Magnitude::ratio(49, 7).to_ratio(), Some((7, 1)));
    assert_eq!(
        Magnitude::ratio(7 * 11, 49 * 13).to_ratio(),
        Some((11, 7 * 13))
    );
    // a prime and the 2 that sorts before it keep their own exponents
    assert_eq!(Magnitude::ratio(2 * 3, 1).to_ratio(), Some((6, 1)));
    assert_eq!(Magnitude::ratio(3, 2).to_ratio(), Some((3, 2)));
    // π, ln 2 and ln 10 are not fractions of integers, alone or with a prime
    assert_eq!(Magnitude::PI.to_ratio(), None);
    assert_eq!(Magnitude::LN2.to_ratio(), None);
    assert_eq!(Magnitude::LN10.to_ratio(), None);
    assert_eq!(Magnitude::PI.mul(Magnitude::ratio(3, 1)).to_ratio(), None);
    assert_eq!(
        Magnitude::PI.mul(Magnitude::PI.recip()).to_ratio(),
        Some((1, 1))
    );
    // a half exponent is not a fraction either
    assert_eq!(Magnitude::ratio(2, 1).sqrt().to_ratio(), None);
    assert_eq!(Magnitude::ratio(4, 1).sqrt().to_ratio(), Some((2, 1)));
}

#[test]
fn applied_to_floats_the_exact_path_is_taken_for_small_fractions() {
    // 0.1 · 7 as a fraction is 7/10 = 0.7; the product of the floats is 0.7000000000000001
    assert_eq!(Magnitude::pow10(-1).apply_f64(7.0).unwrap(), 0.7);
    assert_eq!(Magnitude::ratio(3, 10).apply_f64(10.0).unwrap(), 3.0);
    assert_eq!(Magnitude::ratio(1, 3).apply_f64(3.0).unwrap(), 1.0);
    // a numerator past 2⁵³ takes the float factor and stays finite
    let big = Magnitude::ratio(3u64.pow(40), 7);
    assert!(big.apply_f64(1e290).unwrap().is_finite());
    assert!((big.apply_f64(1.0).unwrap() / (3f64.powi(40) / 7.0) - 1.0).abs() < 1e-12);
}

/// The kernel's arithmetic by a magnitude: irrational ones and products past its room are refused, never a wrong
/// number or a panic; a power of ten alone takes the `u64` path and agrees with the general one.
#[test]
fn kernel_refuses_what_it_cannot_hold() {
    use crate::{Overflow, Rounding};
    assert_eq!(
        Magnitude::PI.apply_rounded::<i32, i32>(1, Rounding::Floor),
        None
    );
    assert_eq!(Magnitude::PI.cmp_scaled(1, 3), None);
    let root2 = Magnitude::pow2(1).sqrt();
    assert_eq!(
        root2.requantize::<u8, u8>(1, Rounding::Ceil, Overflow::Wrap),
        None
    );
    // 4294967291^31 (31 · 32 bits) is past the room of the limbs
    let big = Magnitude::ratio(4_294_967_291, 1).powi(31);
    assert_eq!(big.apply_rounded::<u8, u8>(1, Rounding::Floor), None);
    assert_eq!(big.cmp_scaled(1u8, 1u8), None);
    // 4294967291^5 isn't
    let fits = Magnitude::ratio(4_294_967_291, 1).powi(5);
    assert_eq!(
        fits.requantize::<u8, u8>(1, Rounding::Floor, Overflow::Saturate),
        Some(255)
    );
    assert_eq!(
        fits.recip()
            .apply_rounded::<u128, u8>(u128::MAX, Rounding::Floor),
        Some(0)
    );
    assert_eq!(
        fits.cmp_scaled(1u8, 255),
        Some(core::cmp::Ordering::Greater)
    );
    // a power of ten in u64, and the same through a factor of 3/3
    let milli = Magnitude::pow10(-3);
    let also = milli
        .mul(Magnitude::ratio(3, 1))
        .div(Magnitude::ratio(3, 1));
    for x in [0i64, 1, 499, 500, 501, 1500, -2500, i64::MAX, i64::MIN] {
        for r in [Rounding::Truncate, Rounding::NearestEven, Rounding::Ceil] {
            assert_eq!(
                milli.apply_rounded::<i64, i32>(x, r),
                Magnitude::ratio(1, 1000).apply_rounded::<i64, i32>(x, r),
                "{x} {r:?}"
            );
            assert_eq!(
                also.apply_rounded::<i64, i32>(x, r),
                milli.apply_rounded::<i64, i32>(x, r)
            );
        }
    }
}
