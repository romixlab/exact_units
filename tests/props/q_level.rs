//! Property 10: a linear gain against a `q!` level is the exact order of the two, never a rounded one. A stored value
//! `t` in steps of `num_s / den_s` against a gain of `10^(x/20)` (root-power) or `10^(x/10)` (power) for `x` dB, and
//! of `e^x` or `e^(2x)` for `x` Np, compared exactly: `a^d` against `10^n` in big integers, and `a` against the
//! partial sums of `e^y`'s series, refined until they decide (`reference.rs`). `<`, `<=` and `==` agree with that
//! order for every integer storage, `<`, `<=`, `==`, `>` and `>=` for every float one (none of them for NaN), for the
//! fold a compare makes at compile time (reached at run time through `level_order`) and for `q!` itself, walked over
//! every `u16` and every `f32` whose low 16 bits are 0.
//!
//! An `Exact` against a `q!` literal is the exact order of two fractions, against `num · 10^exp` times the factor
//! from the literal's unit to the `Exact`'s, in big rationals.
#![expect(
    clippy::arithmetic_side_effects,
    reason = "a test computes with the values it chose, which fit; an overflow fails the test"
)]

use super::reference::{order_exp, order_pow10};
use core::cmp::Ordering;
use exact_units::literal::{Db, Decimal, Np};
use exact_units::prefix::{Micro, Milli, One, Pow2};
use exact_units::units::{Kilometre, Meter, Per, Volt, Watt};
use exact_units::{Exact, Fixed, MaybeNan};
use num_bigint::BigInt;
use num_rational::BigRational;

/// `num · 10^exp` exactly.
fn decimal(num: i64, exp: i16) -> BigRational {
    let ten = BigRational::from_integer(BigInt::from(10)).pow(i32::from(exp));
    BigRational::from_integer(BigInt::from(num)) * ten
}

/// `t` steps of `num_s / den_s` against the gain of `num · 10^exp` dB, `per` dB a decade (20 root-power, 10 power).
pub(crate) fn order_db(t: &BigRational, num: i64, exp: i16, per: i64) -> Ordering {
    // x / per = n / d
    let (n, d) = if exp >= 0 {
        (num * 10_i64.pow(exp.unsigned_abs().into()), per)
    } else {
        (num, per * 10_i64.pow(exp.unsigned_abs().into()))
    };
    order_pow10(t, n, u32::try_from(d).unwrap_or(u32::MAX))
}

/// `t` against the gain of `num · 10^exp` Np, `times` 1 (root-power, `e^x`) or 2 (power, `e^(2x)`).
pub(crate) fn order_np(t: &BigRational, num: i64, exp: i16, times: i64) -> Ordering {
    order_exp(
        t,
        &(decimal(num, exp) * BigRational::from_integer(times.into())),
    )
}

/// `[<, <=, ==]` as an order has them.
fn ops3(o: Ordering) -> [bool; 3] {
    [o.is_lt(), o.is_le(), o.is_eq()]
}

/// `[<, <=, ==, >, >=]` as an order has them, all false without one (NaN).
fn ops5(o: Option<Ordering>) -> [bool; 5] {
    o.map_or([false; 5], |o| {
        [o.is_lt(), o.is_le(), o.is_eq(), o.is_gt(), o.is_ge()]
    })
}

/// A level the properties take: `|num · 10^exp|` at most 300 dB (or 30 Np), whose gain stays inside `i128` steps
/// of the finest scale below.
fn in_range(num: i64, exp: i16, neper: bool) -> bool {
    let limit: i64 = if neper { 30 } else { 300 };
    (-2..=0).contains(&exp)
        && num.unsigned_abs() <= (limit * 10_i64.pow(exp.unsigned_abs().into())).unsigned_abs()
}

macro_rules! int_orders {
    ($($name:ident: $ty:ty, $raw:ty, $l:ty, $num_s:expr, $den_s:expr, $order:ident, $f:expr;)*) => {$(
        /// The fold's `[<, <=, ==]` of stored value `t` against a level is the exact order.
        pub(crate) fn $name(t: $raw, num: i64, exp: i16) {
            if in_range(num, exp, stringify!($order) == "order_np") {
                let a = BigRational::new(BigInt::from(t) * BigInt::from($num_s), BigInt::from($den_s));
                let want = $order(&a, num, exp, $f);
                let got = <$ty>::new(t).level_order::<$l>(Decimal::new(num, exp));
                assert_eq!(got, ops3(want), "{t} against {num}e{exp}");
            }
        }
    )*};
}

int_orders! {
    db_root_milli_u16: Fixed<Per<Volt, Volt>, Milli, u16>, u16, Db, 1, 1000, order_db, 20;
    db_root_q15_i16: Fixed<Per<Volt, Volt>, Pow2<-15>, i16>, i16, Db, 1, 32768, order_db, 20;
    db_power_milli_i32: Fixed<Per<Watt, Watt>, Milli, i32>, i32, Db, 1, 1000, order_db, 10;
    db_power_kilo_u64: Fixed<Per<Watt, Watt>, One, u64>, u64, Db, 1, 1, order_db, 10;
    np_root_micro_u32: Fixed<Per<Volt, Volt>, Micro, u32>, u32, Np, 1, 1_000_000, order_np, 1;
    np_power_milli_i16: Fixed<Per<Watt, Watt>, Milli, i16>, i16, Np, 1, 1000, order_np, 2;
}

macro_rules! float_orders {
    ($($name:ident: $ty:ty, $raw:ty, $l:ty, $order:ident, $f:expr;)*) => {$(
        /// The fold's `[<, <=, ==, >, >=]` of stored float `t` against a level is the exact order, none for NaN.
        pub(crate) fn $name(t: $raw, num: i64, exp: i16) {
            if in_range(num, exp, stringify!($order) == "order_np") {
                let want = if t.is_nan() {
                    None
                } else if t.is_infinite() {
                    Some(if t > 0.0 { Ordering::Greater } else { Ordering::Less })
                } else {
                    BigRational::from_float(t).map(|a| $order(&a, num, exp, $f))
                };
                let got = <$ty>::of(t).level_order::<$l>(Decimal::new(num, exp));
                assert_eq!(got, ops5(want), "{t:e} against {num}e{exp}");
            }
        }
    )*};
}

float_orders! {
    db_root_f32: Fixed<Per<Volt, Volt>, One, f32, MaybeNan>, f32, Db, order_db, 20;
    db_power_f64: Fixed<Per<Watt, Watt>, One, f64, MaybeNan>, f64, Db, order_db, 10;
    np_root_f64: Fixed<Per<Volt, Volt>, One, f64, MaybeNan>, f64, Np, order_np, 1;
}

/// An `Exact` of `n / d` m against `num · 10^exp` of a literal in m (`k` 1) or in km (`k` 1000): the exact order of
/// the two fractions, wherever the literal is one (`num · 10^exp · k` within `u128`).
pub(crate) fn exact_against_literal(n: i64, d: u64, num: i64, exp: i16) {
    if d == 0 || !(-15..=15).contains(&exp) {
        return;
    }
    let e = Exact::<Meter>::new(n.into(), d.into());
    let want = BigRational::new(n.into(), d.into()).cmp(&decimal(num, exp));
    assert_eq!(
        e.literal_order::<Meter>(Decimal::new(num, exp)),
        want,
        "{n}/{d} against {num}e{exp} m"
    );
    let want = BigRational::new(n.into(), d.into())
        .cmp(&(decimal(num, exp) * BigRational::from_integer(1000.into())));
    assert_eq!(
        e.literal_order::<Kilometre>(Decimal::new(num, exp)),
        want,
        "{n}/{d} against {num}e{exp} km"
    );
}

#[cfg(not(any(kani, fuzzing)))]
mod tests {
    use super::*;
    use crate::strategy;
    use exact_units::q;
    use exact_units::units::Hertz;
    use proptest::prelude::*;

    /// Levels: hundredths, tenths and whole dB (or Np) inside the properties' range, weighted to the small ones.
    fn level() -> impl Strategy<Value = (i64, i16)> {
        prop_oneof![
            3 => (-600i64..=600, Just(-1i16)),
            2 => (-30_000i64..=30_000, Just(-2i16)),
            1 => (-300i64..=300, Just(0i16)),
        ]
    }

    proptest! {
        #[test]
        fn integer_gains(t16 in strategy::i16(), u in any::<u16>(), t32 in strategy::i32(), u32_ in any::<u32>(), u64_ in any::<u64>(), (num, exp) in level()) {
            db_root_milli_u16(u, num, exp);
            db_root_q15_i16(t16, num, exp);
            db_power_milli_i32(t32, num, exp);
            db_power_kilo_u64(u64_, num, exp);
            np_root_micro_u32(u32_, num / 10, exp);
            np_power_milli_i16(t16, num / 10, exp);
        }

        #[test]
        fn float_gains(t32 in any::<f32>(), t64 in any::<f64>(), (num, exp) in level()) {
            db_root_f32(t32, num, exp);
            db_power_f64(t64, num, exp);
            np_root_f64(t64, num / 10, exp);
        }

        /// Floats next to the gain, where an off-by-one in the fold would show.
        #[test]
        fn floats_next_to_the_gain((num, exp) in level(), k in -3i32..=3) {
            #[expect(clippy::cast_precision_loss, reason = "|num| <= 30000, exact in f64")]
            let x = (num as f64) * 10f64.powi(exp.into());
            let g = 10f64.powf(x / 20.0);
            #[expect(clippy::cast_possible_truncation, reason = "a float near the gain: its last bits don't matter")]
            let g32 = 10f32.powf((x / 20.0) as f32);
            let mut b = g.to_bits().cast_signed();
            let mut b32 = g32.to_bits().cast_signed();
            b += i64::from(k);
            b32 += k;
            db_root_f32(f32::from_bits(b32.cast_unsigned()), num, exp);
            np_root_f64(f64::from_bits(b.cast_unsigned()), num / 10, exp);
            let p = 10f64.powf(x / 10.0).to_bits().cast_signed() + i64::from(k);
            db_power_f64(f64::from_bits(p.cast_unsigned()), num, exp);
        }

        #[test]
        fn exact_against_literals(n in strategy::i64(), d in any::<u64>(), num in strategy::i64(), exp in -15i16..=15) {
            exact_against_literal(n, d, num, exp);
        }
    }

    /// Every operator of stored value `t` against a `q!` level, both ways round, is order `ord` (no `==`).
    macro_rules! orders_only {
        ($t:expr, $lit:expr, $ord:expr) => {{
            let (t, lit, ord): (_, _, Ordering) = ($t, $lit, $ord);
            assert_eq!(t.partial_cmp(&lit), Some(ord), "{t:?}");
            assert_eq!(t < lit, ord.is_lt());
            assert_eq!(t <= lit, ord.is_le());
            assert_eq!(t > lit, ord.is_gt());
            assert_eq!(t >= lit, ord.is_ge());
            assert_eq!(lit.partial_cmp(&t), Some(ord.reverse()));
            assert_eq!(lit < t, ord.is_gt());
            assert_eq!(lit <= t, ord.is_ge());
            assert_eq!(lit > t, ord.is_lt());
            assert_eq!(lit >= t, ord.is_le());
        }};
    }

    /// As `orders_only!`, `==` and `!=` too: a level whose gain is a stored value.
    macro_rules! agrees {
        ($t:expr, $lit:expr, $ord:expr) => {{
            let (t, lit, ord): (_, _, Ordering) = ($t, $lit, $ord);
            orders_only!(t, lit, ord);
            assert_eq!(t == lit, ord.is_eq());
            assert_eq!(t != lit, ord.is_ne());
            assert_eq!(lit == t, ord.is_eq());
            assert_eq!(lit != t, ord.is_ne());
        }};
    }

    fn milli(t: impl Into<BigInt>) -> BigRational {
        BigRational::new(t.into(), 1000.into())
    }

    /// `q!` itself over every u16 in steps of 1 mV/V and of 1 mW/W.
    #[test]
    fn every_u16_against_levels() {
        for t in u16::MIN..=u16::MAX {
            let v = Fixed::<Per<Volt, Volt>, Milli, u16>::new(t);
            let a = milli(t);
            orders_only!(v, q!(-3 dB), order_db(&a, -3, 0, 20));
            orders_only!(v, q!(-3.01 dB), order_db(&a, -301, -2, 20));
            orders_only!(v, q!("-3 dB"), order_db(&a, -3, 0, 20));
            orders_only!(v, q!(-3 "dB"), order_db(&a, -3, 0, 20));
            orders_only!(v, q!(6 dB), order_db(&a, 6, 0, 20));
            agrees!(v, q!(0 dB), t.cmp(&1000));
            agrees!(v, q!(20 dB), t.cmp(&10_000));
            agrees!(v, q!(-20 dB), t.cmp(&100));
            agrees!(v, q!(-60 dB), t.cmp(&1));
            // 10^(−80/20) is 0.1 mV/V: between 0 and 1
            orders_only!(
                v,
                q!(-80 dB),
                if t == 0 {
                    Ordering::Less
                } else {
                    Ordering::Greater
                }
            );
            // 10^(100/20) is 10^5 V/V, past every u16
            orders_only!(v, q!(100 dB), Ordering::Less);
            orders_only!(v, q!(-0.5 Np), order_np(&a, -5, -1, 1));
            agrees!(v, q!(0 Np), t.cmp(&1000));
            let p = Fixed::<Per<Watt, Watt>, Milli, u16>::new(t);
            orders_only!(p, q!(-3 dB), order_db(&a, -3, 0, 10));
            agrees!(p, q!(10 dB), t.cmp(&10_000));
            orders_only!(p, q!(-0.35 Np), order_np(&a, -35, -2, 2));
        }
    }

    /// `q!` over every `f32` whose low 16 bits are 0 and its neighbours.
    #[test]
    fn every_f32_against_levels() {
        for high in 0..=u32::from(u16::MAX) {
            let b = high << 16;
            for b in [b.wrapping_sub(1), b, b + 1] {
                let t = f32::from_bits(b);
                let m = Fixed::<Per<Volt, Volt>, One, f32, MaybeNan>::of(t);
                let want = |num, exp| {
                    if t.is_nan() {
                        None
                    } else if t.is_infinite() {
                        Some(if t > 0.0 {
                            Ordering::Greater
                        } else {
                            Ordering::Less
                        })
                    } else {
                        BigRational::from_float(t).map(|a| order_db(&a, num, exp, 20))
                    }
                };
                // a MaybeNan compares by naming what NaN gives
                for nan in [false, true] {
                    let or = |is: fn(Ordering) -> bool| want(-3, 0).map_or(nan, is);
                    assert_eq!(m.lt_or(q!(-3 dB), nan), or(Ordering::is_lt), "{t:e}");
                    assert_eq!(m.le_or(q!(-3 dB), nan), or(Ordering::is_le), "{t:e}");
                    assert_eq!(m.gt_or(q!(-3 dB), nan), or(Ordering::is_gt), "{t:e}");
                    assert_eq!(m.ge_or(q!(-3 dB), nan), or(Ordering::is_ge), "{t:e}");
                    let or = |is: fn(Ordering) -> bool| want(20, 0).map_or(nan, is);
                    assert_eq!(m.eq_or(q!(20 dB), nan), or(Ordering::is_eq), "{t:e}");
                }
                let Some(g) = m.not_nan() else {
                    assert_eq!(want(-3, 0), None, "{t:e}");
                    continue;
                };
                let got = |lt, le, gt, ge| [lt, le, false, gt, ge];
                let mut w = ops5(want(-3, 0));
                w[2] = false;
                assert_eq!(
                    got(g < q!(-3 dB), g <= q!(-3 dB), g > q!(-3 dB), g >= q!(-3 dB)),
                    w,
                    "{t:e}"
                );
                let w = ops5(want(20, 0));
                assert_eq!(
                    [
                        g < q!(20 dB),
                        g <= q!(20 dB),
                        g == q!(20 dB),
                        g > q!(20 dB),
                        g >= q!(20 dB)
                    ],
                    w,
                    "{t:e}"
                );
                assert_eq!(g.partial_cmp(&q!(-3 dB)), want(-3, 0), "{t:e}");
            }
        }
    }

    /// `level_order` is the fold `q!` makes.
    #[test]
    fn level_order_is_what_q_compares() {
        for t in u16::MIN..=u16::MAX {
            let v = Fixed::<Per<Volt, Volt>, Milli, u16>::new(t);
            assert_eq!(
                [v < q!(-3.01 dB), v <= q!(-3.01 dB), false],
                v.level_order::<Db>(Decimal::new(-301, -2))
            );
            assert_eq!(
                [v < q!(20 dB), v <= q!(20 dB), v == q!(20 dB)],
                v.level_order::<Db>(Decimal::new(20, 0))
            );
            assert_eq!(
                [v < q!(-0.5 Np), v <= q!(-0.5 Np), false],
                v.level_order::<Np>(Decimal::new(-5, -1))
            );
        }
    }

    /// `q!` against an `Exact`: the PLL output of the crate's example, and both sides round.
    #[test]
    fn exact_against_q() {
        let f = Exact::<Hertz>::new(80_000_000, 3);
        assert!(f > q!(26.666666 MHz) && f < q!(26.666667 MHz));
        assert!(q!(26.666666 MHz) < f && q!(26.666667 MHz) > f);
        assert_eq!(f.partial_cmp(&q!(26666666.666 Hz)), Some(Ordering::Greater));
        assert_eq!(q!(26666666.666 Hz).partial_cmp(&f), Some(Ordering::Less));
        let g = (f * 3).unwrap();
        assert_eq!(g, q!(80 MHz));
        assert_eq!(q!(80 MHz), g);
        assert_ne!(g, q!(80.000001 MHz));
        let m = Exact::<Meter>::new(-1, 3);
        assert!(m < q!(-0.333 m) && m > q!(-0.334 m));
        assert_eq!(Exact::<Meter>::ZERO, q!(0 km));
        assert_eq!(Exact::<Meter>::int(1500), q!(1.5 km));
    }
}
