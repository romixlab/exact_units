//! Property 4: saturating and wrapping arithmetic is the exact result, clamped to the storage or wrapped modulo
//! 2^bits; a product rounded into a scale (`times_into`, `times_requantize`) is the exact product rounded once.
#![expect(
    clippy::arithmetic_side_effects,
    reason = "a test computes with the values it chose, which fit; an overflow fails the test"
)]

use super::reference::{assert_rounded, round_div, settle};
use super::rounding::Int;
use exact_units::units::Ratio;
use exact_units::{Fixed, FixedScale, Overflow, Rounding, Saturating, Wrapping};

/// `+`, `-` and `* n` of a [Saturating] value are the exact results clamped to `T`.
pub(crate) fn saturating<P: FixedScale, T: Int>(a: T, b: T) {
    let (x, y) = (
        Fixed::<Ratio, P, T, Saturating>::of(a),
        Fixed::<Ratio, P, T, Saturating>::of(b),
    );
    let (ea, eb) = (a.into(), b.into());
    let clamp = |v: i128| v.clamp(T::MIN_INT, T::MAX_INT);
    assert_eq!((x + y).get().into(), clamp(ea + eb), "{a:?} + {b:?}");
    assert_eq!((x - y).get().into(), clamp(ea - eb), "{a:?} - {b:?}");
    assert_eq!((x * b).get().into(), clamp(ea * eb), "{a:?} * {b:?}");
}

/// `+`, `-` and `* n` of a [Wrapping] value are the exact results modulo 2^bits.
pub(crate) fn wrapping<P: FixedScale, T: Int>(a: T, b: T) {
    let (x, y) = (
        Fixed::<Ratio, P, T, Wrapping>::of(a),
        Fixed::<Ratio, P, T, Wrapping>::of(b),
    );
    let (ea, eb) = (a.into(), b.into());
    let wrap = |v: i128| settle(v, T::MIN_INT, T::MAX_INT, Overflow::Wrap);
    assert_eq!(Some((x + y).get().into()), wrap(ea + eb), "{a:?} + {b:?}");
    assert_eq!(Some((x - y).get().into()), wrap(ea - eb), "{a:?} - {b:?}");
    assert_eq!(Some((x * b).get().into()), wrap(ea * eb), "{a:?} * {b:?}");
}

/// `a · b` of steps `P` into steps `P3` of `T3` (the factor `P² / P3` is `num / den`), rounded by `r` and settled
/// by `o`: the exact product rounded once; `times_into` is the checked form.
pub(crate) fn times<P: FixedScale, P3: FixedScale, T: Int, T3: Int>(
    a: T,
    b: T,
    num: i128,
    den: i128,
    r: Rounding,
    o: Overflow,
) {
    let (x, y) = (
        Fixed::<Ratio, P, T, Saturating>::of(a),
        Fixed::<Ratio, P, T, Saturating>::of(b),
    );
    let got = x
        .times_requantize::<_, _, _, P3, T3>(y, r, o)
        .map(|f| f.get().into());
    let p = a.into() * b.into() * num;
    let q = round_div(p, den, r);
    let (min, max) = (T3::MIN_INT, T3::MAX_INT);
    match (o, got) {
        (Overflow::Checked, Some(v)) => {
            assert_rounded(p, den, r, v);
            assert!((min..=max).contains(&v));
        }
        (Overflow::Checked, None) => assert!(!(min..=max).contains(&q), "{a:?} · {b:?}: {q} fits"),
        (_, got) => assert_eq!(got, settle(q, min, max, o), "{a:?} · {b:?} {r:?} {o:?}"),
    }
    if o == Overflow::Checked {
        assert_eq!(
            x.times_into::<_, _, _, P3, T3>(y, r)
                .map(|f| f.get().into()),
            got
        );
    }
}

#[cfg(not(kani))]
mod tests {
    use crate::reference::{MODES, POLICIES};
    use crate::strategy;
    use exact_units::prefix::{Micro, Milli, One, Pow2};
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn q15_saturates(a in strategy::i16(), b in strategy::i16()) {
            super::saturating::<Pow2<-15>, i16>(a, b);
        }

        #[test]
        fn q31_saturates(a in strategy::i32(), b in strategy::i32()) {
            super::saturating::<Pow2<-31>, i32>(a, b);
        }

        #[test]
        fn i64_saturates(a in strategy::i64(), b in strategy::i64()) {
            super::saturating::<Milli, i64>(a, b);
        }

        #[test]
        fn bam16_wraps(a: u16, b: u16) {
            super::wrapping::<Pow2<-16>, u16>(a, b);
        }

        #[test]
        fn i32_wraps(a in strategy::i32(), b in strategy::i32()) {
            super::wrapping::<Milli, i32>(a, b);
        }

        #[test]
        fn i64_wraps(a in strategy::i64(), b in strategy::i64()) {
            super::wrapping::<Pow2<-63>, i64>(a, b);
        }

        /// Q15 · Q15 back into Q15: the product's 2⁻³⁰ steps into 2⁻¹⁵.
        #[test]
        fn q15_times(a in strategy::i16(), b in strategy::i16(), r in strategy::mode(), o in strategy::policy()) {
            super::times::<Pow2<-15>, Pow2<-15>, i16, i16>(a, b, 1, 1 << 15, r, o);
        }

        #[test]
        fn q31_times(a in strategy::i32(), b in strategy::i32(), r in strategy::mode(), o in strategy::policy()) {
            super::times::<Pow2<-31>, Pow2<-31>, i32, i32>(a, b, 1, 1 << 31, r, o);
        }

        /// mV · mA in mW: µW steps into mW, the 64-bit path.
        #[test]
        fn milli_times(a in strategy::i32(), b in strategy::i32(), r in strategy::mode(), o in strategy::policy()) {
            super::times::<Milli, Milli, i32, i32>(a, b, 1, 1000, r, o);
        }

        /// µ · µ into whole steps of an i64: the 128-bit path.
        #[test]
        fn micro_times(a in strategy::i64(), b in strategy::i64(), r in strategy::mode(), o in strategy::policy()) {
            super::times::<Micro, One, i64, i64>(a, b, 1, 1_000_000_000_000, r, o);
        }
    }

    /// Every pair of i8 in steps of 2⁻⁷ (Q7): add, subtract, multiply by a number and the rounded product, in full.
    #[test]
    fn every_q7_pair() {
        for a in i8::MIN..=i8::MAX {
            for b in i8::MIN..=i8::MAX {
                super::saturating::<Pow2<-7>, i8>(a, b);
                super::wrapping::<Pow2<-7>, i8>(a, b);
                for r in MODES {
                    for o in POLICIES {
                        super::times::<Pow2<-7>, Pow2<-7>, i8, i8>(a, b, 1, 1 << 7, r, o);
                    }
                }
            }
        }
    }
}

#[cfg(kani)]
mod proofs {
    use crate::reference::{MODES, POLICIES};
    use exact_units::prefix::{Milli, Pow2};

    /// Every pair of Q15: add, subtract and multiply by a number saturate.
    #[kani::proof]
    fn q15_saturates() {
        super::saturating::<Pow2<-15>, i16>(kani::any(), kani::any());
    }

    /// Every pair of Q15, every mode and policy: the product rounded back into Q15.
    #[kani::proof]
    fn q15_times() {
        let r = MODES[usize::from(kani::any_where(|i: &u8| usize::from(*i) < MODES.len()))];
        let o = POLICIES[usize::from(kani::any_where(|i: &u8| *i < 3))];
        super::times::<Pow2<-15>, Pow2<-15>, i16, i16>(kani::any(), kani::any(), 1, 1 << 15, r, o);
    }

    /// Every pair of i16 mV · mA into mW of an i16, every mode, under one policy: 30 s to 3 minutes each, 6 minutes
    /// for the three against 10 for all policies in one proof; a pair of i32 takes over 10 minutes.
    fn milli_times_i16(o: exact_units::Overflow) {
        let r = MODES[usize::from(kani::any_where(|i: &u8| usize::from(*i) < MODES.len()))];
        super::times::<Milli, Milli, i16, i16>(kani::any(), kani::any(), 1, 1000, r, o);
    }

    #[kani::proof]
    fn milli_times_i16_checked() {
        milli_times_i16(POLICIES[0]);
    }

    #[kani::proof]
    fn milli_times_i16_saturating() {
        milli_times_i16(POLICIES[1]);
    }

    #[kani::proof]
    fn milli_times_i16_wrapping() {
        milli_times_i16(POLICIES[2]);
    }
}
