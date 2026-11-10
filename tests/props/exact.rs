//! Property 11: the arithmetic of a `Fraction` against exact rationals. A `Fraction` is `±num/den · 10^exp` in a
//! canonical form, which `reference::canonical` finds by brute force over the exponents: on `u128` against
//! `BigRational`, a sum, difference, product or quotient is the exact value in that form, refused exactly where no
//! form fits (a sum whose aligned terms pass four limbs too, the window the book lists, with strategies aimed at it),
//! and the order is the exact one, across exponents too. The plain lowest terms (`numer`, `denom`) are the value's or
//! `None` past `u128`. Rounding to an integer is the reference's, `to_f64` is the nearest float (a tie to even,
//! subnormals and ±∞ included), `from_f64` the float's exact value or refused where no form fits. An `Exact` and a
//! `Fixed` convert and compare as the integers they are, and order as their exact values do for every value of the
//! `Fixed` (floats of any size, ±∞, NaN without an order, products with the scale past `u128`, exponents far apart).
//! On `u8` with an `i8` exponent (`Fraction8`, the same code) the same against `BigRational`: drawn, and walked over
//! every pair of `u8` numerators with denominators and exponents at both ends. The pieces (widening multiply, the
//! four-limb steps, divide, gcd, the 5s by the inverse) are Kani proofs on `u8` limbs in `src/exact/fraction.rs`; the
//! whole operations are too large a SAT problem for Kani.
#![expect(
    clippy::arithmetic_side_effects,
    reason = "a test computes with the values it chose, which fit; an overflow fails the test"
)]
#![expect(clippy::panic, reason = "a property fails by panicking")]
#![expect(clippy::expect_used, reason = "a property fails by panicking")]

#[cfg(not(kani))]
use super::reference;
#[cfg(not(kani))]
use exact_units::Fraction8;
#[cfg(not(kani))]
use num_bigint::BigInt;
#[cfg(not(kani))]
use num_rational::BigRational;

/// `±num/den · 10^exp`, `den > 0`.
pub(crate) type Parts8 = (bool, u8, u8, i8);

/// A form as the reference gives it: the sign, the mantissa and the exponent.
#[cfg(not(kani))]
pub(crate) type Form = (bool, BigInt, BigInt, i64);

/// The value of a `Fraction8`.
#[cfg(not(kani))]
fn big8(a: Fraction8) -> BigRational {
    let (n, d) = a.mantissa();
    reference::rational_exp10(a.is_negative(), n.into(), d.into(), a.exp10().into())
}

/// The form of a `Fraction8`.
#[cfg(not(kani))]
fn form8(a: Fraction8) -> Form {
    let (n, d) = a.mantissa();
    (a.is_negative(), n.into(), d.into(), a.exp10().into())
}

/// Whether a sum of `a` and `b` passes four limbs of `bits` once aligned: the term of the larger exponent times its
/// power of ten and the other's denominator over their common factor, or the sum of both terms of one sign. Where it
/// does, a refusal of a sum that would fit is the window the book lists.
#[cfg(not(kani))]
pub(crate) fn aligned_past(a: &Form, b: &Form, bits: u32) -> bool {
    use num_integer_free::gcd;
    let (a, b) = if a.3 >= b.3 { (a, b) } else { (b, a) };
    let g = gcd(&a.2, &b.2);
    let ten = BigInt::from(10).pow(u32::try_from(a.3 - b.3).unwrap_or(u32::MAX));
    let x = &a.1 * (&b.2 / &g) * ten;
    let y = &b.1 * (&a.2 / &g);
    let limit = BigInt::from(1) << (4 * bits);
    x >= limit || (a.0 == b.0 && x + y >= limit)
}

/// Euclid on big integers, written here.
#[cfg(not(kani))]
mod num_integer_free {
    use num_bigint::BigInt;

    pub(crate) fn gcd(a: &BigInt, b: &BigInt) -> BigInt {
        let (mut a, mut b) = (a.clone(), b.clone());
        while b != BigInt::from(0) {
            let r = &a % &b;
            (a, b) = (b, r);
        }
        a
    }
}

/// `got` is `want` in the canonical form of `bits` and the exponents `lo..=hi`, or `None` exactly where no form of
/// `want` fits; where `window`, a refusal of a form that fits is allowed (a sum in the window).
#[cfg(not(kani))]
pub(crate) fn exact_form(
    got: Option<Form>,
    want: &BigRational,
    (bits, lo, hi): (u32, i64, i64),
    window: bool,
) {
    match (got, reference::canonical(want, bits, lo, hi)) {
        (Some(r), Some(c)) => assert_eq!(r, c, "{want}"),
        (Some(r), None) => panic!("{r:?} for {want}, which has no form"),
        (None, Some(c)) => assert!(window, "{want} fits as {c:?}, refused"),
        (None, None) => {}
    }
}

/// The plain lowest terms of `want` against `numer` and `denom`: each the value's, `None` past `bits`.
#[cfg(not(kani))]
pub(crate) fn plain(numer: Option<u128>, denom: Option<u128>, want: &BigRational, bits: u32) {
    let limit = BigInt::from(1) << bits;
    let fit = |x: &BigInt| (x < &limit).then(|| u128::try_from(x).expect("below the limit"));
    assert_eq!(
        numer,
        fit(&want.numer().magnitude().clone().into()),
        "{want}"
    );
    assert_eq!(denom, fit(want.denom()), "{want}");
}

#[cfg(not(kani))]
const RANGE8: (u32, i64, i64) = (8, -128, 127);

/// Every operation of two `Fraction8`s against `BigRational`: exact or refused as documented, in the canonical form,
/// ordered exactly, rounded as the reference. A pair whose parts have a zero denominator or no form is skipped.
#[cfg(not(kani))]
pub(crate) fn ops8(a: Parts8, b: Parts8) {
    let made = |p: Parts8| {
        let x = Fraction8::from_parts_exp10(p.0, p.1, p.2, p.3);
        if p.2 != 0 {
            let want = reference::rational_exp10(p.0, p.1.into(), p.2.into(), p.3.into());
            exact_form(x.map(form8), &want, RANGE8, false);
        } else {
            assert_eq!(x, None);
        }
        x
    };
    let (Some(a), Some(b)) = (made(a), made(b)) else {
        return;
    };
    let (ra, rb) = (big8(a), big8(b));
    let (fa, fb) = (form8(a), form8(b));
    let neg = |f: &Form| {
        (
            !f.0 && f.1 != BigInt::from(0),
            f.1.clone(),
            f.2.clone(),
            f.3,
        )
    };
    exact_form(
        a.checked_add(b).map(form8),
        &(&ra + &rb),
        RANGE8,
        aligned_past(&fa, &fb, 8),
    );
    exact_form(
        a.checked_sub(b).map(form8),
        &(&ra - &rb),
        RANGE8,
        aligned_past(&fa, &neg(&fb), 8),
    );
    exact_form(a.checked_mul(b).map(form8), &(&ra * &rb), RANGE8, false);
    if b.is_zero() {
        assert_eq!(a.checked_div(b), None);
        assert_eq!(b.checked_recip(), None);
    } else {
        exact_form(a.checked_div(b).map(form8), &(&ra / &rb), RANGE8, false);
        exact_form(b.checked_recip().map(form8), &rb.recip(), RANGE8, false);
    }
    assert_eq!(a.compare(b), ra.cmp(&rb), "{a:?} against {b:?}");
    assert_eq!(a == b, ra == rb);
    assert_eq!(a.is_integer(), ra.is_integer(), "{a:?}");
    plain(a.numer().map(u128::from), a.denom().map(u128::from), &ra, 8);
    assert_eq!(a.negate().checked_add(a), Some(Fraction8::ZERO));
    compare_fast8(a, b);
}

/// The number of bits up to the highest one set, 0 for 0.
#[cfg(not(kani))]
fn len8(x: u8) -> i64 {
    x.checked_ilog2().map_or(0, |l| i64::from(l) + 1)
}

/// See [len8].
fn len(x: u128) -> i64 {
    x.checked_ilog2().map_or(0, |l| i64::from(l) + 1)
}

/// Whether the order of `a · b` and `c · d` (`x` and `y`, the sums of their factors' bit lengths) is proven by bit
/// length alone: a product of two has `x − 1` or `x` bits, so it is where `x` and `y` are 2 or more apart and no
/// factor is 0.
fn proven(x: i64, y: i64, any_zero: bool) -> bool {
    !any_zero && (x - 2 >= y || y - 2 >= x)
}

/// The fast path of a compare against the wide cross products alone: `compare` (the fast path on Thumb-1, the wide
/// products elsewhere) and `compare_fast` (fast path and fallback, on every target) are `compare_wide` for every
/// pair; `compare_by_bits` answers exactly where the signs differ or the bit lengths prove the order ([proven]), and
/// where it answers it is `compare_wide`'s answer.
macro_rules! compare_fast {
    ($(#[$meta:meta])* $name:ident, $f:ty, $len:ident) => {
        $(#[$meta])*
        pub(crate) fn $name(a: $f, b: $f) {
            let wide = a.compare_wide(b);
            assert_eq!(a.compare(b), wide, "{a:?} against {b:?}");
            assert_eq!(a.compare_fast(b), wide, "{a:?} against {b:?}");
            let fast = a.compare_by_bits(b);
            let ((an, ad), (bn, bd)) = (a.mantissa(), b.mantissa());
            let (x, y) = ($len(an) + $len(bd), $len(bn) + $len(ad));
            // of two exponents the fast path declines: the bit lengths of the mantissas alone say nothing
            let decides = a.exp10() == b.exp10()
                && (a.is_negative() != b.is_negative() || proven(x, y, a.is_zero() || b.is_zero()));
            assert_eq!(fast.is_some(), decides, "{a:?} against {b:?}: bit lengths {x}, {y}");
            if let Some(o) = fast {
                assert_eq!(o, wide, "{a:?} against {b:?}");
            }
        }
    };
}

compare_fast! {
    /// [compare_fast] on `u8`, walked whole with [ops8].
    #[cfg(not(kani))]
    compare_fast8, Fraction8, len8
}

compare_fast! {
    /// [compare_fast] on `u128`, drawn at the bit-length edge.
    #[cfg(not(kani))]
    compare_fast128, exact_units::Fraction, len
}

/// The order of a `Fixed`'s and an `Exact`'s magnitudes, `m · 2^k` steps of `sn/sd` against `en/ed` (all above 0),
/// fast path and fallback against the wide products alone: `magnitude` is `magnitude_wide`; `magnitude_by_bits`
/// answers exactly where the bit lengths prove the order, and then as `magnitude_wide` does. `a = m · sn · ed`
/// has between `la − 2` and `la` bits (`la` the sum of its factors'), `b = en · sd` between `lb − 1` and `lb`.
#[cfg(not(kani))]
pub(crate) fn magnitude_fast(
    m: u128,
    k: i32,
    s: (u128, u128),
    e: (u128, u128),
) -> core::cmp::Ordering {
    use exact_units::__order_parts::{magnitude, magnitude_by_bits, magnitude_wide};
    let wide = magnitude_wide(m, k, s, e);
    let at = format!("{m}·2^{k}·{}/{} against {}/{}", s.0, s.1, e.0, e.1);
    assert_eq!(magnitude(m, k, s, e), wide, "{at}");
    let fast = magnitude_by_bits(m, k, s, e);
    let (la, lb) = (
        len(m) + len(s.0) + len(e.1) + i64::from(k),
        len(e.0) + len(s.1),
    );
    let any_zero = [m, s.0, s.1, e.0, e.1].contains(&0);
    let decides = !any_zero && (la - 2 > lb || la < lb - 1);
    assert_eq!(fast.is_some(), decides, "{at}: bit lengths {la}, {lb}");
    if let Some(o) = fast {
        assert_eq!(o, wide, "{at}");
    }
    wide
}

#[cfg(not(kani))]
#[expect(clippy::expect_used, reason = "a property fails by panicking")]
pub(crate) mod big {
    use super::super::reference;
    use core::cmp::Ordering;
    use exact_units::prefix::{Kilo, Mega, One, Pow10, Quecto, Quetta, Rational};
    use exact_units::units::Hertz;
    use exact_units::{
        Bounds, Exact, Fixed, FixedScale, Fraction, MaybeNan, Repr, Rounding, Storage,
    };
    use num_rational::BigRational;

    pub(crate) fn big(x: Fraction) -> BigRational {
        let (n, d) = x.mantissa();
        reference::rational_exp10(x.is_negative(), n, d, x.exp10().into())
    }

    /// The form of a `Fraction`.
    pub(crate) fn form(x: Fraction) -> super::Form {
        let (n, d) = x.mantissa();
        (x.is_negative(), n.into(), d.into(), x.exp10().into())
    }

    /// The range of a `Fraction`'s form: `u128` mantissas, an `i32` exponent.
    const RANGE: (u32, i64, i64) = (128, i32::MIN as i64, i32::MAX as i64);

    /// `got` is `want` in the canonical form, or `None` exactly where no form of `want` fits; a sum in the window
    /// may be refused where `window`.
    fn exact(got: Option<Fraction>, want: &BigRational, window: bool) {
        super::exact_form(got.map(form), want, RANGE, window);
    }

    /// A `Fraction` of `want`'s canonical form, `None` where there is none.
    pub(crate) fn fraction_of(want: &BigRational) -> Option<Fraction> {
        let (neg, n, d, e) = reference::canonical(want, RANGE.0, RANGE.1, RANGE.2)?;
        let x = Fraction::from_parts_exp10(
            neg,
            (&n).try_into().expect("fits"),
            (&d).try_into().expect("fits"),
            i32::try_from(e).expect("in range"),
        );
        assert_eq!(x.map(form), Some((neg, n, d, e)), "{want}");
        x
    }

    /// Every operation of two fractions against `BigRational`, the plain terms, `is_integer`, rounding and the
    /// nearest float of each.
    pub(crate) fn arith(a: Fraction, b: Fraction) {
        let (ra, rb) = (big(a), big(b));
        exact(Some(a), &ra, false);
        let (fa, fb) = (form(a), form(b));
        let neg = |f: &super::Form| (!f.0 && f.1 != 0.into(), f.1.clone(), f.2.clone(), f.3);
        exact(
            a.checked_add(b),
            &(&ra + &rb),
            super::aligned_past(&fa, &fb, 128),
        );
        exact(
            a.checked_sub(b),
            &(&ra - &rb),
            super::aligned_past(&fa, &neg(&fb), 128),
        );
        exact(a.checked_mul(b), &(&ra * &rb), false);
        if b.is_zero() {
            assert_eq!(a.checked_div(b), None);
            assert_eq!(b.checked_recip(), None);
        } else {
            exact(a.checked_div(b), &(&ra / &rb), false);
            exact(b.checked_recip(), &rb.recip(), false);
        }
        assert_eq!(a.cmp(&b), ra.cmp(&rb), "{a:?} against {b:?}");
        assert_eq!(a == b, ra == rb);
        assert_eq!(a.is_integer(), ra.is_integer(), "{a:?}");
        super::plain(a.numer(), a.denom(), &ra, 128);
        round_any(a);
        nearest_f64(a);
    }

    /// `x · 10^shift` by its parts: the same mantissa, the exponent moved; `None` where it leaves `i32`.
    fn shifted(x: Fraction, shift: i32) -> Option<Fraction> {
        if x.is_zero() {
            return Some(x);
        }
        let (n, d) = x.mantissa();
        Fraction::from_parts_exp10(x.is_negative(), n, d, x.exp10().checked_add(shift)?)
    }

    /// Near the ends of the exponent's range, where no `BigRational` reaches: every operation of `a · 10^k` and
    /// `b · 10^k` is that of `a` and `b` shifted (a product's factors by about `k/2` each), the order the same. The
    /// canonical form doesn't depend on where the value sits, as long as the range doesn't clamp it.
    pub(crate) fn at_the_ends(a: Fraction, b: Fraction, k: i32) {
        let (Some(sa), Some(sb)) = (shifted(a, k), shifted(b, k)) else {
            return;
        };
        assert_eq!(shifted(sa, -k), Some(a));
        assert_eq!(
            sa.checked_add(sb),
            a.checked_add(b).and_then(|x| shifted(x, k)),
            "{sa:?} + {sb:?}"
        );
        assert_eq!(
            sa.checked_sub(sb),
            a.checked_sub(b).and_then(|x| shifted(x, k)),
            "{sa:?} - {sb:?}"
        );
        assert_eq!(sa.checked_div(sb), a.checked_div(b), "{sa:?} / {sb:?}");
        assert_eq!(sa.cmp(&sb), a.cmp(&b), "{sa:?} against {sb:?}");
        assert_eq!(sa.compare_fast(sb), a.cmp(&b), "{sa:?} against {sb:?}");
        if let Some(ab) = a.checked_mul(b) {
            let k2 = k / 2;
            if let (Some(ha), Some(hb)) = (shifted(a, k2), shifted(b, k - k2)) {
                assert_eq!(ha.checked_mul(hb), shifted(ab, k), "{ha:?} * {hb:?}");
            }
        }
        // past f64 and past i128 either way
        let f = sa.to_f64();
        if a.is_zero() {
            assert_eq!(f, 0.0);
        } else if k > 0 {
            assert!(
                f.is_infinite() && f.is_sign_negative() == a.is_negative(),
                "{sa:?}: {f}"
            );
            assert_eq!(sa.round(Rounding::NearestEven), None);
            assert_eq!(sa.numer(), None, "{sa:?}");
        } else {
            assert!(
                f == 0.0 && f.is_sign_negative() == a.is_negative(),
                "{sa:?}: {f}"
            );
            let floor = if a.is_negative() { -1 } else { 0 };
            assert_eq!(sa.round(Rounding::Floor), Some(floor), "{sa:?}");
            assert_eq!(sa.round(Rounding::NearestEven), Some(0), "{sa:?}");
            assert!(!sa.is_integer() && sa.denom().is_none(), "{sa:?}");
        }
    }

    /// `round` of any fraction is the reference's, `None` exactly past `i128`.
    pub(crate) fn round_any(a: Fraction) {
        let x = big(a);
        for r in reference::MODES {
            let want = reference::round_rational(&x, r);
            assert_eq!(a.round(r), i128::try_from(&want).ok(), "{a:?} {r:?}");
        }
    }

    /// A sum whose aligned terms are large but whose result fits: `a = s − b` for a small `s` and a large `b`, so
    /// `a + b` is `s`, `s − a` is `b`, each exact unless the terms pass four limbs. Skipped where `a` has no form.
    pub(crate) fn sum_in_the_window(s: Fraction, b: Fraction) {
        let Some(a) = fraction_of(&(&big(s) - &big(b))) else {
            return;
        };
        arith(a, b);
        if !super::aligned_past(&form(a), &form(b), 128) {
            assert_eq!(a.checked_add(b), Some(s), "{a:?} + {b:?}");
        }
        if !super::aligned_past(&form(s), &form(a.negate()), 128) {
            assert_eq!(s.checked_sub(a), Some(b), "{s:?} - {a:?}");
        }
    }

    /// `round` of `num/den` is the reference's.
    pub(crate) fn round(num: i64, den: u64, r: Rounding) {
        if den == 0 {
            return;
        }
        let x = Fraction::new(i128::from(num), u128::from(den));
        assert_eq!(
            x.round(r),
            Some(reference::round_div(i128::from(num), i128::from(den), r)),
            "{num}/{den} {r:?}"
        );
    }

    /// `x.rounded(places, r)` of `num/den` Hz: `{:#}` writes the reference's `round(x · 10^places)` with `places`
    /// digits after the point, and `{:e}` a mantissa in [1, 10) (or 0) with `places` digits whose value is the
    /// reference's rounding of `x / 10^k` for the exponent `k` it writes.
    pub(crate) fn rounded(num: i32, den: u32, places: u8, r: Rounding) {
        if den == 0 {
            return;
        }
        let x = Exact::<Hertz>::new(i128::from(num), u128::from(den));
        let p = u32::from(places);
        // the plain form, written here from the digits of the reference's integer
        let n = reference::round_div(i128::from(num) * 10i128.pow(p), i128::from(den), r);
        let digits = format!("{:0>width$}", n.unsigned_abs(), width = places as usize + 1);
        let (whole, frac) = digits.split_at(digits.len() - places as usize);
        let sign = if n < 0 { "-" } else { "" };
        let want = if places == 0 {
            format!("{sign}{whole} Hz")
        } else {
            format!("{sign}{whole}.{frac} Hz")
        };
        assert_eq!(
            format!("{:#}", x.rounded(places, r)),
            want,
            "{num}/{den} {places} {r:?}"
        );
        // the scientific form: parse it back and check it against the reference at the exponent it chose
        let text = format!("{:e}", x.rounded(places, r));
        let number = text.strip_suffix(" Hz").expect("the unit");
        let (mantissa, k) = number.split_once('e').expect("an exponent");
        let k: i32 = k.parse().expect("an integer exponent");
        let after = mantissa.split_once('.').map_or(0, |(_, f)| f.len());
        assert_eq!(after, places as usize, "{text}");
        let m: Fraction = mantissa.parse().expect("a decimal");
        let ten = Fraction::int(10);
        if num == 0 {
            assert!(m.is_zero() && k == 0, "{text}");
            return;
        }
        assert!(m.abs() >= Fraction::ONE && m.abs() < ten, "{text}");
        // round(x · 10^(places − k)) in i128: |x| < 2³¹ and k >= −10, places <= 6
        let shift = i32::from(places) - k;
        let (top, bottom) = if shift >= 0 {
            (
                i128::from(num) * 10i128.pow(shift.unsigned_abs()),
                i128::from(den),
            )
        } else {
            (
                i128::from(num),
                i128::from(den) * 10i128.pow(shift.unsigned_abs()),
            )
        };
        let want = reference::round_div(top, bottom, r);
        let got = m
            .checked_mul(Fraction::int(10i128.pow(p)))
            .and_then(|s| s.round(Rounding::Truncate));
        assert_eq!(got, Some(want), "{text} of {num}/{den} {r:?}");
    }

    /// `to_f64` is the float nearest the fraction, a tie to the even one, ±∞ from half a step past `f64::MAX`;
    /// `from_f64` of a finite one is that float exactly.
    pub(crate) fn nearest_f64(a: Fraction) {
        let x = big(a);
        let f = a.to_f64();
        assert_eq!(f.is_sign_negative(), a.is_negative(), "{a:?}: {f}");
        let abs = if a.is_negative() { -&x } else { x.clone() };
        if f.is_infinite() {
            // 2^1024 − 2^970: half a step past f64::MAX, where the nearest is ∞ (a tie goes to the even ∞)
            let edge = BigRational::from_integer(
                (num_bigint::BigInt::from(1) << 1024) - (num_bigint::BigInt::from(1) << 970),
            );
            assert!(abs >= edge, "{a:?}: ∞");
            return;
        }
        let off = |g: f64| {
            let g = BigRational::from_float(g).expect("finite");
            let d = &g - &x;
            if d < BigRational::from_integer(0.into()) {
                -d
            } else {
                d
            }
        };
        let (here, up, down) = (off(f), off(f.next_up()), off(f.next_down()));
        #[expect(
            clippy::float_cmp,
            reason = "the largest float exactly, whose next one up is ∞"
        )]
        if f.abs() == f64::MAX {
            assert!(here <= down, "{a:?}: {f}");
        } else {
            assert!(here <= up && here <= down, "{a:?}: {f}");
        }
        if here == up || here == down {
            assert_eq!(f.to_bits() % 2, 0, "{a:?}: a tie to {f}, not to even");
        }
        if let Some(back) = Fraction::from_f64(f) {
            assert_eq!(back.to_f64(), f);
        }
    }

    /// A finite float is `from_f64`'s fraction exactly, or refused where no form of it fits.
    pub(crate) fn from_f64(x: f64) {
        let got = Fraction::from_f64(x);
        match BigRational::from_float(x) {
            None => assert_eq!(got, None),
            Some(want) => {
                exact(got, &want, false);
                if let Some(r) = got {
                    assert_eq!(r.to_f64(), x);
                }
            }
        }
    }

    /// An `Exact` from a `Fixed` and back is the same, and two `Fixed`s of other scales compare with it as their
    /// integers in hertz do; rounding into MHz is the reference's.
    pub(crate) fn with_fixed(khz: i32, mhz: i16, r: Rounding) {
        let a = Fixed::<Hertz, Kilo, i32>::new(khz);
        let b = Fixed::<Hertz, Mega, i16>::new(mhz);
        let x = Exact::from_fixed(a).expect("kHz in i32 fits");
        assert_eq!(x, Exact::int(i128::from(khz) * 1000));
        assert_eq!(x.to::<Kilo, i32>(), Some(a));
        assert!(x == a, "Exact == Fixed");
        assert!(a == x, "Fixed == Exact");
        let (hz_a, hz_b) = (i128::from(khz) * 1000, i128::from(mhz) * 1_000_000);
        assert_eq!(x.partial_cmp(&b), Some(hz_a.cmp(&hz_b)));
        assert_eq!(b.partial_cmp(&x), Some(hz_b.cmp(&hz_a)));
        let want = reference::round_div(hz_a, 1_000_000, r);
        assert_eq!(
            x.to_rounded::<Mega, i16>(r).map(Fixed::get).map(i128::from),
            i16::try_from(want).ok().map(i128::from)
        );
        assert_eq!(
            x.to::<Mega, i32>().is_some(),
            hz_a % 1_000_000 == 0,
            "{khz} kHz in MHz"
        );
    }

    /// `f`'s exact value at a scale of `num/den` hertz a step, `None` for NaN, and ±∞ as `Err` of its sign.
    fn value<P: FixedScale, T: Storage, B: Bounds<T>>(
        f: Fixed<Hertz, P, T, B>,
        (num, den): (u128, u128),
    ) -> Option<Result<BigRational, Ordering>> {
        let x = match f.get().to_repr() {
            Repr::Int(i) => BigRational::from_integer(i.into()),
            Repr::Float(x) if x.is_nan() => return None,
            Repr::Float(x) if x.is_infinite() => {
                return Some(Err(if x > 0.0 {
                    Ordering::Greater
                } else {
                    Ordering::Less
                }));
            }
            Repr::Float(x) => BigRational::from_float(x).expect("a finite float"),
        };
        Some(Ok(x * reference::rational(false, num, den)))
    }

    /// A `Fixed` of `num/den` hertz a step and an `Exact` order as their exact values do, both ways: ±∞ is past
    /// every `Exact`, NaN (of a [MaybeNan]) alone has no order and gives the fallback of `gt_or` and its kin, the one
    /// value `not_nan` refuses. `==` is exactly where the order is `Equal`.
    pub(crate) fn order<P: FixedScale, T: Storage, B: Bounds<T>>(
        f: Fixed<Hertz, P, T, B>,
        scale: (u128, u128),
        e: Exact<Hertz>,
    ) {
        let want = value(f, scale).map(|v| v.map_or_else(|o| o, |x| x.cmp(&big(e.get()))));
        for nan in [false, true] {
            let or = |is: fn(Ordering) -> bool| want.map_or(nan, is);
            assert_eq!(f.lt_or(e, nan), or(Ordering::is_lt), "{f:?} < {e:?}");
            assert_eq!(f.le_or(e, nan), or(Ordering::is_le), "{f:?} <= {e:?}");
            assert_eq!(f.gt_or(e, nan), or(Ordering::is_gt), "{f:?} > {e:?}");
            assert_eq!(f.ge_or(e, nan), or(Ordering::is_ge), "{f:?} >= {e:?}");
            assert_eq!(f.eq_or(e, nan), or(Ordering::is_eq), "{f:?} == {e:?}");
        }
        let Some(f) = f.not_nan() else {
            assert_eq!(want, None, "{f:?} refused by not_nan");
            return;
        };
        assert_eq!(f.partial_cmp(&e), want, "{f:?} against {e:?}");
        assert_eq!(
            e.partial_cmp(&f),
            want.map(Ordering::reverse),
            "{e:?} against {f:?}"
        );
        assert_eq!(f == e, want == Some(Ordering::Equal), "{f:?} == {e:?}");
        assert_eq!(e == f, want == Some(Ordering::Equal), "{e:?} == {f:?}");
    }

    /// [magnitude_fast](super::magnitude_fast), and the order it gives is the exact one.
    pub(crate) fn magnitude(m: u128, k: i32, s: (u128, u128), e: (u128, u128)) {
        let got = super::magnitude_fast(m, k, s, e);
        let pow = BigRational::from_integer(num_bigint::BigInt::from(2).pow(k.unsigned_abs()));
        let pow = if k < 0 { pow.recip() } else { pow };
        let r = |n: u128, d: u128| BigRational::new(n.into(), d.into());
        let a = r(m, 1) * pow * r(s.0, s.1);
        assert_eq!(got, a.cmp(&r(e.0, e.1)), "{m}·2^{k}·{s:?} against {e:?}");
    }

    /// [order] at the scales that pass `u128` with a big storage (`10^±30`, `10^20`), at 1 and 1/3, on every storage
    /// kind: `x` as an `f64` and an `f32`, `i` as an `i64` and a `u64`.
    pub(crate) fn orders(x: f64, i: i64, e: Exact<Hertz>) {
        const Q: u128 = 10u128.pow(30);
        order(Fixed::<Hertz, One, f64, MaybeNan>::of(x), (1, 1), e);
        order(Fixed::<Hertz, Quetta, f64, MaybeNan>::of(x), (Q, 1), e);
        order(Fixed::<Hertz, Quecto, f64, MaybeNan>::of(x), (1, Q), e);
        order(
            Fixed::<Hertz, Rational<1, 3>, f64, MaybeNan>::of(x),
            (1, 3),
            e,
        );
        #[expect(clippy::cast_possible_truncation, reason = "any f32 will do")]
        order(
            Fixed::<Hertz, Kilo, f32, MaybeNan>::of(x as f32),
            (1000, 1),
            e,
        );
        order(Fixed::<Hertz, One, i64>::new(i), (1, 1), e);
        order(Fixed::<Hertz, Quetta, i64>::new(i), (Q, 1), e);
        order(Fixed::<Hertz, Quecto, i64>::new(i), (1, Q), e);
        order(
            Fixed::<Hertz, Pow10<20>, i64>::new(i),
            (10u128.pow(20), 1),
            e,
        );
        order(
            Fixed::<Hertz, Quetta, u64>::new(i.unsigned_abs()),
            (Q, 1),
            e,
        );
    }
}

#[cfg(not(any(kani, fuzzing)))]
mod tests {
    use super::super::strategy;
    use super::big;
    use proptest::prelude::*;

    fn parts8() -> impl Strategy<Value = super::Parts8> {
        (
            any::<bool>(),
            any::<u8>(),
            any::<u8>(),
            prop_oneof![
                2 => Just(0i8),
                2 => -4i8..=4,
                1 => any::<i8>(),
                1 => proptest::sample::select(&[i8::MIN, i8::MIN + 1, i8::MAX - 1, i8::MAX][..]),
            ],
        )
    }

    proptest! {
        #[test]
        fn ops_on_u8(a in parts8(), b in parts8()) {
            super::ops8(a, b);
        }

        #[test]
        fn ops_on_u128(a in strategy::fraction(), b in strategy::fraction()) {
            big::arith(a, b);
        }

        #[test]
        fn ops_at_the_ends(a in strategy::fraction(), b in strategy::fraction(), k in strategy::exp10_ends()) {
            big::at_the_ends(a, b, k);
        }

        #[test]
        fn ops_at_the_clamp(a in strategy::clamped(), b in strategy::fraction()) {
            big::arith(a, b);
            big::arith(b, a);
        }

        /// Two values of exponents `k` apart whose mantissas are near `10^k` apart: the order is decided by the
        /// aligned product, not the bit lengths, and the sum aligns by `k`.
        #[test]
        fn orders_across_exponents(
            neg in any::<bool>(), n in 1u128..=u128::MAX >> 4, d in 1u128..=u128::MAX,
            e in strategy::exp10s(), k in 1u32..=38, j in -3i128..=3,
        ) {
            let f = |n: u128, e: i32| exact_units::Fraction::from_parts_exp10(neg, n, d, e).unwrap_or_default();
            let a = f(n, e);
            if let Some(m) = n.checked_mul(10u128.pow(k)).and_then(|m| m.checked_add_signed(j)) {
                big::arith(a, f(m, e.saturating_sub(k.cast_signed())));
            }
        }

        /// Large denominators that share a factor with each other and none with the small sum: the cross terms pass
        /// 2¹²⁸.
        #[test]
        fn sums_in_the_window(
            s in (any::<bool>(), 0u128..=1000, 1u128..=1000),
            b in (any::<bool>(), (1u128 << 100)..=u128::MAX, (1u128 << 64)..=(1u128 << 120)),
        ) {
            let f = |(neg, n, d)| exact_units::Fraction::from_parts(neg, n, d).unwrap_or_default();
            big::sum_in_the_window(f(s), f(b));
        }

        /// The same with the exponents apart: a small sum and a large term whose aligned form passes four limbs or
        /// nears it (Δ about 77), so both sides of the window are drawn.
        #[test]
        fn sums_in_the_window_across_exponents(
            s in (any::<bool>(), 0u128..=1000, 1u128..=1000, -10i32..=10),
            b in (any::<bool>(), 1u128..=u128::MAX, 1u128..=u128::MAX, 60i32..=90),
        ) {
            let f = |(neg, n, d, e)| exact_units::Fraction::from_parts_exp10(neg, n, d, e).unwrap_or_default();
            big::sum_in_the_window(f(s), f(b));
            let (sn, sd, se) = (s.1, s.2, s.3);
            big::sum_in_the_window(f((s.0, sn, sd, se.saturating_sub(80))), f((b.0, b.1, b.2, -b.3)));
        }

        #[test]
        fn rounds_as_the_reference(num in strategy::i64(), den in any::<u64>(), r in strategy::mode()) {
            big::round(num, den, r);
        }

        #[test]
        fn rounded_as_the_reference(
            num in any::<i32>(), den in any::<u32>(), places in 0u8..=6, r in strategy::mode(),
        ) {
            big::rounded(num, den, places, r);
        }

        #[test]
        fn nearest_float(a in strategy::fraction()) {
            big::nearest_f64(a);
        }

        #[test]
        fn floats_exactly(bits in any::<u64>()) {
            big::from_f64(f64::from_bits(bits));
        }

        #[test]
        fn with_a_fixed(khz in strategy::i32(), mhz in strategy::i16(), r in strategy::mode()) {
            big::with_fixed(khz, mhz, r);
        }
    }

    /// Floats weighted to where an `Exact` ends: anything, the ends of `f64` and ±∞, NaN, powers of 2 across the whole
    /// range (subnormals to `2^1023`), and numbers near `2^±128`.
    fn floats() -> impl Strategy<Value = f64> {
        prop_oneof![
            3 => any::<u64>().prop_map(f64::from_bits),
            1 => proptest::sample::select(&[
                f64::INFINITY, f64::NEG_INFINITY, f64::NAN, 0.0, -0.0, f64::MAX, f64::MIN,
                f64::MIN_POSITIVE, 5e-324, -5e-324,
            ][..]),
            2 => (any::<bool>(), -1074i32..=1023).prop_map(|(neg, k)| {
                let x = 2f64.powi(k);
                if neg { -x } else { x }
            }),
            2 => (any::<i32>(), -160i32..=160).prop_map(|(m, k)| f64::from(m) * 2f64.powi(k)),
        ]
    }

    /// A number of exactly `l` bits (1 to 128): the least, the most, or between.
    fn of_bits(l: u32) -> impl Strategy<Value = u128> {
        let lo = 1u128 << (l - 1);
        let hi = lo - 1 + lo;
        prop_oneof![Just(lo), Just(hi), lo..=hi]
    }

    /// Bit lengths of 1 to 128 for `n` numbers, and `n` of those bits: the lengths are what decides.
    fn lengths<const N: usize>() -> impl Strategy<Value = [u32; N]> {
        proptest::array::uniform::<_, N>(prop_oneof![
            2 => 1u32..=128,
            1 => proptest::sample::select(&[1u32, 2, 31, 32, 33, 63, 64, 65, 95, 96, 97, 127, 128][..]),
        ])
    }

    /// The four cross factors of two fractions of one sign with `len(an) + len(bd)` and `len(bn) + len(ad)` at most
    /// 4 apart: both sides of the ±1-bit edge where the fast path just declines or just answers. The fractions
    /// are reduced, which may move a length.
    fn fractions_at_the_edge()
    -> impl Strategy<Value = (exact_units::Fraction, exact_units::Fraction)> {
        (lengths::<3>(), -4i64..=4, any::<bool>())
            .prop_flat_map(|([la, lb, lc], delta, neg)| {
                let ld = (i64::from(la) + i64::from(lb) - i64::from(lc) + delta).clamp(1, 128);
                let ld = u32::try_from(ld).unwrap_or(1);
                (
                    of_bits(la),
                    of_bits(ld),
                    of_bits(lc),
                    of_bits(lb),
                    Just(neg),
                )
            })
            .prop_map(|(an, ad, bn, bd, neg)| {
                let f = |n, d| exact_units::Fraction::from_parts(neg, n, d).unwrap_or_default();
                (f(an, ad), f(bn, bd))
            })
    }

    /// `m, k, (sn, sd), (en, ed)` of the order of magnitudes, with `k` set so that `len(m) + len(sn) + len(ed) + k`
    /// is within 4 of `len(en) + len(sd)`: both sides of the edges at −2 and +1 bits.
    fn magnitudes_at_the_edge() -> impl Strategy<Value = (u128, i32, (u128, u128), (u128, u128))> {
        (lengths::<5>(), -4i32..=4).prop_flat_map(|([lm, lsn, led, len, lsd], delta)| {
            let k = i32::try_from(len + lsd).unwrap_or(0)
                - i32::try_from(lm + lsn + led).unwrap_or(0)
                + delta;
            (
                of_bits(lm),
                Just(k),
                (of_bits(lsn), of_bits(lsd)),
                (of_bits(len), of_bits(led)),
            )
        })
    }

    proptest! {
        #[test]
        fn compares_by_bits_as_wide((a, b) in fractions_at_the_edge()) {
            super::compare_fast128(a, b);
            big::arith(a, b);
        }

        #[test]
        fn compares_any_by_bits_as_wide(a in strategy::fraction(), b in strategy::fraction()) {
            super::compare_fast128(a, b);
        }

        #[test]
        fn orders_magnitudes_by_bits_as_wide((m, k, s, e) in magnitudes_at_the_edge()) {
            big::magnitude(m, k, s, e);
        }
    }

    /// Pinned edges of the order of magnitudes: equal values, one step either side, the fast path's bounds met
    /// exactly (`len(a) = la − 2`, `len(b) = lb − 1`) and missed by one bit.
    #[test]
    fn magnitudes_at_known_edges() {
        let max = u128::MAX;
        for (m, k, s, e) in [
            (1, 0, (1, 1), (1, 1)),
            (1, 1, (1, 1), (1, 1)),
            (1, -1, (1, 1), (1, 1)),
            (1, 2, (1, 1), (3, 1)),
            (1, 1, (1, 1), (3, 1)),
            (2, 0, (2, 1), (7, 1)),
            (2, 0, (2, 1), (8, 1)),
            (3, 0, (3, 1), (8, 1)),
            (max, 0, (max, 1), (max, 1)),
            (max, 128, (max, max), (max, 1)),
            (max, -256, (1, 1), (1, max)),
            (1 << 127, -127, (1, 1), (1, 1)),
            (1 << 127, -128, (1, 1), (1, 1)),
            (1 << 127, -126, (1, 1), (1, 1)),
            (1 << 52, -1126, (1, 1), (1, max)),
        ] {
            big::magnitude(m, k, s, e);
        }
    }

    /// An `Exact` whose value is a float's: the orders that are `Equal`.
    fn as_exact(x: f64) -> exact_units::Exact<exact_units::units::Hertz> {
        exact_units::Exact::of(exact_units::Fraction::from_f64(x).unwrap_or_default())
    }

    proptest! {
        #[test]
        fn orders_with_a_fixed(x in floats(), i in strategy::i64(), e in strategy::fraction()) {
            big::orders(x, i, exact_units::Exact::of(e));
        }

        #[test]
        fn orders_with_its_own_value(x in floats(), i in strategy::i64()) {
            big::orders(x, i, as_exact(x));
            #[expect(clippy::cast_precision_loss, reason = "any float near i will do")]
            big::orders(i as f64, i, as_exact(i as f64));
        }
    }

    /// Orders that `from_fixed` couldn't give (its value past `u128`): huge and tiny floats, ±∞, and an `i64` at a
    /// scale whose product passes `u128`, against `Exact`s near `i128::MAX` and below `1/u128::MAX`.
    #[test]
    fn known_orders() {
        use exact_units::{Exact, Fraction};
        let max = Exact::of(Fraction::int(i128::MAX));
        let min = Exact::of(Fraction::int(i128::MIN));
        let tiny = Exact::of(Fraction::from_parts(false, 1, u128::MAX).unwrap());
        let atomic = Exact::of(Fraction::from_parts_exp10(false, 3, 7, -40).unwrap());
        let far = Exact::of(Fraction::from_parts_exp10(true, 1, 3, 400).unwrap());
        for e in [
            max,
            min,
            tiny,
            -tiny,
            Exact::ZERO,
            Exact::int(1),
            atomic,
            far,
            -far,
        ] {
            for x in [
                f64::MAX,
                f64::MIN,
                1e300,
                -1e300,
                1e-300,
                -1e-300,
                5e-324,
                f64::INFINITY,
                f64::NEG_INFINITY,
            ] {
                for i in [i64::MAX, i64::MIN, 1, -1, 0] {
                    big::orders(x, i, e);
                }
            }
        }
    }

    /// Every pair of `u8` numerators, each pair with signs, denominators at both ends (1, 3, 5, 128, 250, 255) and
    /// exponents near 0, far apart and at the ends of `i8` in turn (all of them over the walk): every refusal of a
    /// sum, difference, product and quotient against the exact value. Then every combination of those for a few
    /// numerators.
    #[test]
    fn walk_u8() {
        const DENS: [u8; 6] = [1, 3, 5, 128, 250, 255];
        const EXPS: [i8; 12] = [-128, -127, -60, -3, -1, 0, 1, 2, 3, 60, 126, 127];
        // the exponents of the second walk: both ends, the middle and one step past it
        const ENDS: [i8; 6] = [-128, -127, -1, 0, 1, 127];
        for an in 0..=u8::MAX {
            for bn in 0..=u8::MAX {
                let i = usize::from(an) * 256 + usize::from(bn);
                let (ad, bd) = (DENS[i % 6], DENS[(i / 6) % 6]);
                let (ae, be) = (EXPS[(i / 36) % 12], EXPS[(i / 432) % 12]);
                let (na, nb) = (i % 2 == 1, (i / 7) % 2 == 1);
                super::ops8((na, an, ad, ae), (nb, bn, bd, be));
            }
        }
        for an in [0u8, 1, 2, 5, 125, 128, 255] {
            for bn in [1u8, 3, 25, 64, 255] {
                for ad in DENS {
                    for bd in DENS {
                        for ae in ENDS {
                            for be in ENDS {
                                super::ops8((false, an, ad, ae), (true, bn, bd, be));
                            }
                        }
                    }
                }
            }
        }
    }

    /// The PLL values of an STM32H7 clock tree, which the planner works out.
    #[test]
    fn pll_values() {
        use exact_units::Fraction;
        let n = Fraction::int(173)
            .checked_add(Fraction::new(2730, 8192))
            .unwrap();
        big::arith(n, Fraction::new(24_000_000, 8));
        big::arith(Fraction::new(80_000_000, 3), Fraction::new(-80_000_000, 1));
        big::arith(
            Fraction::new(i128::MAX, u128::MAX),
            Fraction::new(i128::MIN, u128::MAX - 2),
        );
    }

    /// The ends of `i32`, where the range clamps: `1/10^MIN` is `10 · 10^MAX`, and the reverse; an `f64` and a
    /// `Fixed` compare with both, and the text reads back.
    #[test]
    fn exponents_at_the_ends() {
        use exact_units::{Exact, Fixed, Fraction, prefix::One, units::Hertz};
        let (lo, hi) = (Fraction::pow10(i32::MIN), Fraction::pow10(i32::MAX));
        assert_eq!(
            lo.checked_recip(),
            Fraction::from_parts_exp10(false, 10, 1, i32::MAX)
        );
        assert_eq!(hi.checked_recip(), Some(Fraction::pow10(i32::MIN + 1)));
        assert_eq!(
            Fraction::int(3).checked_div(hi),
            Fraction::from_parts_exp10(false, 3, 1, i32::MIN + 1)
        );
        assert_eq!(
            hi.checked_mul(Fraction::int(10)),
            Fraction::from_parts_exp10(false, 10, 1, i32::MAX)
        );
        assert_eq!(
            hi.checked_mul(Fraction::int(1 << 120))
                .and_then(|x| x.checked_mul(Fraction::int(1 << 120))),
            None
        );
        assert_eq!(lo.checked_mul(hi), Some(Fraction::new(1, 10)));
        // the floats with a form: every one of magnitude 2⁻¹⁰⁷ to below 2¹⁸³, powers of two out to 2⁻¹⁸²
        let full = |b: u64| f64::from_bits((b << 52) | ((1 << 52) - 1));
        let float = |x: f64| Fraction::from_f64(x);
        assert!(float(full(1023 - 107)).is_some() && float(full(1023 - 108)).is_none());
        assert!(
            float(full(1023 + 182)).is_some()
                && float(f64::from_bits((1023 + 183) << 52)).is_none()
        );
        let small = |k: u64| float(f64::from_bits((1023 - k) << 52));
        assert!(small(182).is_some() && small(183).is_none());
        assert_eq!((float(5e-324), float(f64::MAX)), (None, None));
        assert!(lo < small(182).unwrap() && hi > float(full(1023 + 182)).unwrap());
        assert_eq!(
            (lo.to_f64(), hi.to_f64(), (-hi).to_f64()),
            (0.0, f64::INFINITY, f64::NEG_INFINITY)
        );
        let tiny = Fixed::<Hertz, One, f64>::try_new(5e-324).unwrap();
        assert!(tiny > Exact::<Hertz>::of(lo) && tiny < Exact::<Hertz>::of(hi));
        assert_eq!(lo.to_string(), format!("1e{}/1", i32::MIN));
        assert_eq!(lo.to_string().parse::<Fraction>(), Ok(lo));
    }

    /// Found by the properties: a float whose last bit is below 2⁻¹²⁷ but whose mantissa ends in zeros.
    #[test]
    fn small_float_with_trailing_zeros() {
        big::from_f64(f64::from_bits(4_265_076_838_150_819_824));
    }
}
