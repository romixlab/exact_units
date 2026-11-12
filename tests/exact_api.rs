//! One API over every number an [Exact] holds (NUM-40, NUM-41): the same body of calls compiled and checked against
//! each `R`, so that a name one `Exact<U, R>` has and another lacks fails the build. The `R`s are listed at the
//! bottom: `Fraction`, and `BigFraction` with the feature `bignum` (its API through the trait `BigExact`). The calls
//! name `R` only where a type says which `Exact` it is (`E::<Hertz>::int`), as code written for `Exact<Hertz>` does.
#![expect(
    clippy::arithmetic_side_effects,
    reason = "Exact's operators are checked: each gives an Option, which the test unwraps"
)]

use exact_units::prefix::{Kilo, Mega, One};
use exact_units::quantities::{Activity, Height, Width};
use exact_units::systems::Si;
use exact_units::units::{Becquerel, Hertz, Meter, Of, Per, Ratio, Second, Volt};
#[cfg(feature = "bignum")]
use exact_units::{BigExact, BigFraction};
use exact_units::{Exact, ExactNum, Fixed, FixedScale, Fraction, Prefix, Rounding, Tolerance};

/// Code of one's own over any `R`: the operators, `Display`, parse, the order against a `Fixed` and `convert`, with
/// no `R` named. The `const fn`s (`get`, `to_system`, `as_quantity`) are on each concrete `Exact<U, R>`, not here.
fn period_text<R: ExactNum>(f: &Exact<Hertz, R>) -> Option<String> {
    let doubled = (f.clone() + f.clone())?;
    let back = (doubled / 2)?;
    assert_eq!(&back, f);
    assert!(back.clone() > Fixed::<Hertz, One, u32>::new(0));
    let t: Exact<Second, R> = back.recip()?;
    let ms: Exact<exact_units::units::Minute, R> = t.convert()?;
    let read: Exact<Second, R> = ms.convert::<Second>()?.to_string().parse().ok()?;
    Some(read.to_string())
}

macro_rules! api {
    ($m:ident, $r:ty) => {
        mod $m {
            use super::*;

            type R = $r;
            type E<U> = Exact<U, R>;

            fn num(n: i128, d: u128) -> R {
                R::from_fraction(Fraction::new(n, d))
            }

            #[test]
            fn constructors() {
                assert_eq!(E::<Hertz>::new(1, 3).get(), num(1, 3));
                assert_eq!(E::<Hertz>::of(num(2, 1)), E::<Hertz>::int(2));
                assert_eq!(E::<Hertz>::checked_new(1, 0), None);
                assert_eq!(E::<Hertz>::new_in::<Mega>(24, 1), E::int(24_000_000));
                assert_eq!(E::<Hertz>::checked_new_in::<Mega>(1, 0), None);
                assert_eq!(
                    E::<Hertz>::from_scaled(Fraction::new(1, 2), &Kilo::SCALE),
                    Some(E::int(500))
                );
                assert_eq!(
                    E::<Hertz>::from_prefix(num(24, 1), Prefix::Mega),
                    Some(E::int(24_000_000))
                );
                let f = Fixed::<Hertz, Kilo, u32>::new(3);
                assert_eq!(E::<Hertz>::from_fixed(f), Some(E::int(3000)));
                assert_eq!(E::<Hertz>::ZERO, E::default());
            }

            #[test]
            fn arithmetic() {
                let a = E::<Hertz>::new(1, 3);
                let b = E::<Hertz>::new(1, 6);
                assert_eq!(a.clone().checked_add(b.clone()), Some(E::new(1, 2)));
                assert_eq!(a.clone().checked_sub(b.clone()), Some(b.clone()));
                assert_eq!(
                    a.clone().checked_scale(Fraction::new(1, 2)),
                    Some(b.clone())
                );
                assert_eq!(
                    b.clone().checked_unscale(Fraction::new(1, 2)),
                    Some(a.clone())
                );
                let t = E::<Second>::int(3);
                assert_eq!(
                    a.clone().checked_times(t.clone()).map(E::get),
                    Some(num(1, 1))
                );
                assert_eq!(
                    a.clone().checked_per(t.clone()).map(E::get),
                    Some(num(1, 9))
                );
                assert_eq!(a.clone() + b.clone(), Some(E::new(1, 2)));
                assert_eq!(a.clone() - b.clone(), Some(b.clone()));
                assert_eq!(-a.clone(), E::new(-1, 3));
                assert_eq!(a.clone() * 3, Some(E::int(1)));
                assert_eq!(a.clone() / 2, Some(b.clone()));
                assert_eq!(a.clone() * num(1, 2), Some(b.clone()));
                assert_eq!(b.clone() / num(1, 2), Some(a.clone()));
                assert_eq!((a.clone() * t.clone()).map(E::get), Some(num(1, 1)));
                assert_eq!((a.clone() / t).map(E::get), Some(num(1, 9)));
                assert_eq!(a.clone() / 0, None);
                assert!((-a.clone()).is_negative() && !a.clone().is_zero());
                assert_eq!((-a.clone()).abs(), a);
                assert!(b < a && E::<Hertz>::ZERO.is_zero());
                // on references, so a large value isn't cloned for each step
                assert_eq!(&a + &b, Some(E::new(1, 2)));
                assert_eq!(&a - &b, Some(b.clone()));
                assert_eq!(-&a, E::new(-1, 3));
                assert_eq!(&a * 3, Some(E::int(1)));
                assert_eq!(&a / 2, Some(b.clone()));
                assert_eq!(&a * &num(1, 2), Some(b.clone()));
                assert_eq!(&b / &num(1, 2), Some(a.clone()));
                assert_eq!((&a * &E::<Second>::int(3)).map(E::get), Some(num(1, 1)));
                assert_eq!((&a / &E::<Second>::int(3)).map(E::get), Some(num(1, 9)));
            }

            #[test]
            fn units_and_fixed() {
                let f = E::<Hertz>::new(80_000_000, 3);
                assert_eq!(f.clone().to::<One, u64>(), None);
                assert_eq!(
                    f.clone().to_rounded::<One, u64>(Rounding::Floor),
                    Some(Fixed::new(26_666_666))
                );
                assert!(f.clone() < Fixed::<Hertz, Kilo, u32>::new(26_667));
                assert!(Fixed::<Hertz, Kilo, u32>::new(26_666) < f.clone());
                assert!(Fixed::<Hertz, One, u32>::new(5) == E::<Hertz>::int(5));
                assert_eq!(
                    E::<Second>::int(60).convert::<exact_units::units::Minute>(),
                    Some(E::int(1))
                );
                let period: Option<E<Second>> = E::<Hertz>::int(1000).recip();
                assert_eq!(period, Some(E::new(1, 1000)));
                assert!((f.clone().to_f64() - 26_666_666.666_666_668).abs() < 1e-6);
                assert_eq!(f.clone().auto_prefix(), Prefix::Mega);
                assert_eq!(
                    f.clone().in_prefix(Prefix::Kilo).to_string(),
                    "26666.[6] kHz"
                );
                assert_eq!(f.to_string(), "26.[6] MHz");
                assert_eq!(format!("{:#}", E::<Hertz>::new(1, 2)), "0.5 Hz");
                assert_eq!(format!("{:?}", E::<Hertz>::int(2)), "Exact(2/1 Hz)");
                assert_eq!("26.[6] MHz".parse::<E<Hertz>>(), Ok(E::new(80_000_000, 3)));
                assert!("1 m".parse::<E<Hertz>>().is_err());
                let r = E::<Hertz>::new(1, 3).rounded(2, Rounding::NearestEven);
                assert_eq!(r.to_string(), "333.33 mHz");
            }

            #[test]
            fn quantities_and_ratios() {
                let rate = E::<Per<Ratio, Second>>::int(100);
                let a = rate.as_quantity::<Activity>().convert::<Becquerel>();
                assert_eq!(a, Some(E::int(100)));
                let h = E::<Of<Height, Meter>>::new(12, 10);
                let w = E::<Of<Width, Meter>>::new(3, 10);
                let l: Option<E<Meter>> = h.up() + w.up();
                assert_eq!(l, Some(E::new(3, 2)));
                let s = E::<Second>::int(2).to_system::<Si>().convert::<Second>();
                assert_eq!(s, Some(E::int(2)));
                let gain = (E::<Volt>::int(3) / E::<Volt>::int(2)).unwrap();
                assert_eq!(gain.as_ratio(), E::<Ratio>::new(3, 2));
                assert_eq!(
                    E::<Ratio>::new(3, 2).as_gain::<Volt>().as_ratio(),
                    E::new(3, 2)
                );
                assert_eq!(E::<Volt>::int(2).scaled(E::new(3, 2)), Some(E::int(3)));
            }

            #[test]
            fn tolerances() {
                let fdcan = E::<Hertz>::new(2_559_999_375, 32);
                let target = E::<Hertz>::new_in::<Mega>(80, 1);
                let e = fdcan.clone().rel_error(target.clone()).unwrap();
                assert_eq!(e.to_string(), "-0.244140625 ppm");
                assert!(fdcan.within(target, Tolerance::ppm(1)).is_ok());
            }

            #[test]
            fn generic_code() {
                assert_eq!(
                    period_text(&E::<Hertz>::int(3)).as_deref(),
                    Some("333.[3] ms")
                );
            }
        }
    };
}

api!(fraction, Fraction);
#[cfg(feature = "bignum")]
api!(big_fraction, BigFraction);
