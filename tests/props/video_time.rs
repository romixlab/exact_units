//! Properties of video time (PT-8, `examples/16_video_time.rs`, the book's video time recipe): conversions between
//! the clocks of H.264, H.265 and RFC 6184 against the exact quotient in every rounding and overflow policy (a frame
//! at 30000/1001 Hz is 3003 ticks of 90 kHz, an H.264 tick 1501.5, an H.265 25 Hz tick of a 27 MHz clock is 1/25 s
//! and free to convert), the 48 kHz cadence of each rounding (every 5 frames 8008 samples, each frame 1601 or 1602),
//! and points on clocks that wrap: RFC 6184 5.1's 32-bit timestamp, whose period is the storage's own 2³², and a
//! 33-bit count in a `u64`, which wraps by `rem_euclid`. `+` lands on `(a + v) mod m` and `b − a` is the shortest
//! way round, `[-m/2, m/2)`, against [modulo]. Kani proves the same two shapes on `u16` (a period of 2¹⁶ and of
//! 2¹⁵ steps), as 32- and 64-bit storages take it too long.
#![expect(
    clippy::arithmetic_side_effects,
    reason = "a test computes with the values it chose, which fit; an overflow fails the test"
)]

use super::reference::modulo;
use exact_units::prefix::Rational;
use exact_units::units::Second;
use exact_units::{Fixed, Modulo, Offset, Point};

/// An H.264 tick at 30000/1001 frames/s: num_units_in_tick 1001, time_scale 60 000 (H.264 E.2.1).
pub(crate) type Tick60k = Rational<1001, 60_000>;

/// An H.264 frame at that rate, 2·tc (Table E-6).
pub(crate) type Frame30k = Rational<1001, 30_000>;

/// A tick of RFC 6184's 90 kHz clock.
pub(crate) type Tick90k = Rational<1, 90_000>;

/// A sample of 48 kHz audio.
pub(crate) type Sample48k = Rational<1, 48_000>;

macro_rules! clocks {
    ($($name:ident { modulus: $m:ty }),* $(,)?) => {$(
        exact_units::origin! {
            /// A clock of the property.
            pub struct $name { unit: Second, symbol: "", offset: Offset::ZERO, modulus: Modulo<$m> }
        }
    )*};
}

clocks! {
    Rtp32 { modulus: Rational<4_294_967_296, 90_000> },
    Pts33 { modulus: Rational<8_589_934_592, 90_000> },
    Rtp16 { modulus: Rational<65_536, 90_000> },
    Pts15 { modulus: Rational<32_768, 90_000> },
}

macro_rules! wraps {
    ($($name:ident: $origin:ty, $t:ty, $bits:literal;)*) => {$(
        /// `a + v` and `b − a` on the clock against [modulo], for `a` and `b` on the circle.
        pub(crate) fn $name(a: $t, b: $t, v: $t) {
            type P = Point<$origin, Fixed<Second, Tick90k, $t>>;
            let m = 1i128 << $bits;
            let (a, b) = (modulo(a.into(), m, false), modulo(b.into(), m, false));
            let (pa, pb) = (P::from_raw(a as $t), P::from_raw(b as $t));
            assert_eq!(i128::from((pa + Fixed::new(v)).raw()), modulo(a + i128::from(v), m, false), "{a} + {v}");
            assert_eq!(i128::from((pa - Fixed::new(v)).raw()), modulo(a - i128::from(v), m, false), "{a} - {v}");
            assert_eq!(i128::from((pb - pa).get()), modulo(b - a, m, true), "{b} - {a}");
            assert_eq!(pa.checked_add(Fixed::new(v)), Some(pa + Fixed::new(v)));
            // a point moved by the difference lands on the other
            let d = pb - pa;
            let back = if d.get() >= 0 { pa + Fixed::new(d.get().unsigned_abs()) } else { pa - Fixed::new(d.get().unsigned_abs()) };
            assert_eq!(back, pb);
        }
    )*};
}

wraps! {
    rtp32_wraps: Rtp32, u32, 32;
    pts33_wraps: Pts33, u64, 33;
    rtp16_wraps: Rtp16, u16, 16;
    pts15_wraps: Pts15, u16, 15;
}

/// The samples of frames `k + 1 ..= k + 5` at 48 kHz, each the difference of two running totals rounded by `r`: 8008
/// in all, each 1601 or 1602, and the same as the reference's quotients.
pub(crate) fn cadence(k: u64, r: exact_units::Rounding) {
    let total = |k: u64| {
        let samples = Fixed::<Second, Frame30k, u64>::new(k).to_rounded::<Sample48k, u64>(r);
        let got = i128::from(samples.unwrap().get());
        assert_eq!(got, super::reference::round_div(i128::from(k) * 8008, 5, r), "frame {k} {r:?}");
        got
    };
    let counts = [1, 2, 3, 4, 5].map(|i| total(k + i) - total(k + i - 1));
    assert_eq!(counts.iter().sum::<i128>(), 8008, "{counts:?}");
    assert!(counts.iter().all(|c| (1601..=1602).contains(c)), "{counts:?}");
}

#[cfg(not(kani))]
mod tests {
    use super::*;
    use crate::reference::MODES;
    use crate::strategy;
    use exact_units::FixedScale;
    use proptest::prelude::*;
    use proptest::sample::select;

    /// Anything, the ends, and a window either side of `2^bits` and of the half turn, where the clocks wrap.
    fn around(bits: u32) -> impl Strategy<Value = u64> {
        let m = 1u64 << bits;
        prop_oneof![
            3 => any::<u64>(),
            2 => (0..=2000u64).prop_map(move |d| m.wrapping_sub(1000).wrapping_add(d)),
            2 => (0..=2000u64).prop_map(move |d| (m / 2).wrapping_sub(1000).wrapping_add(d)),
            1 => select(&[0, 1, u64::MAX - 1, u64::MAX][..]),
        ]
    }

    #[expect(clippy::cast_possible_truncation, reason = "a u32 clock takes the low 32 bits of a drawn value")]
    fn u32_around() -> impl Strategy<Value = u32> {
        around(32).prop_map(|x| x as u32)
    }

    proptest! {
        #[test]
        fn rtp32(a in u32_around(), b in u32_around(), v in u32_around()) {
            rtp32_wraps(a, b, v);
        }

        #[test]
        fn pts33(a in around(33), b in around(33), v in around(33)) {
            pts33_wraps(a, b, v);
        }

        /// A field of 60000/1001 Hz into 90 kHz ticks: × 3003/2, never exact for an odd count.
        #[test]
        fn field_to_90k(x in around(32), r in strategy::mode(), o in strategy::policy()) {
            crate::rounding::requantizes::<Tick60k, Tick90k, u64, u64>(x, 3003, 2, r, o);
        }

        /// 90 kHz ticks into frames of 30000/1001 Hz: ÷ 3003, a u64 count into a u32.
        #[test]
        fn rtp_to_frames(x in around(32), r in strategy::mode(), o in strategy::policy()) {
            crate::rounding::requantizes::<Tick90k, Frame30k, u64, u32>(x, 1, 3003, r, o);
        }

        /// Frames into 48 kHz samples: × 8008/5.
        #[test]
        fn frames_to_samples(x in u32_around(), r in strategy::mode(), o in strategy::policy()) {
            crate::rounding::requantizes::<Frame30k, Sample48k, u32, u64>(x, 8008, 5, r, o);
        }

        /// H.265's 25 Hz tick of a 27 MHz clock is 1/25 s: the same scale, so the same raw value.
        #[test]
        fn tick_27m_is_a_25th(x in any::<u32>()) {
            let tick = Fixed::<Second, Rational<1_080_000, 27_000_000>, u32>::new(x);
            prop_assert_eq!(tick.to::<Rational<1, 25>, u32>().map(Fixed::get), Some(x));
        }
    }

    #[test]
    fn scales() {
        assert_eq!(<Rational<1_080_000, 27_000_000>>::SCALE, <Rational<1, 25>>::SCALE);
        assert_eq!(Frame30k::SCALE, Tick60k::SCALE.mul(exact_units::Magnitude::pow2(1)));
    }

    /// Every frame of an hour at 30000/1001 Hz, every rounding: the cadence of 5 frames from each.
    #[test]
    fn cadence_an_hour() {
        for r in MODES {
            for k in 0..107_892 {
                cadence(k, r);
            }
        }
    }

    /// Every pair of u16 points and every move, on both u16 clocks: what the Kani proofs take symbolic, walked whole
    /// for the moves from a few points.
    #[test]
    fn u16_clocks_walked() {
        for a in [0, 1, 0x3fff, 0x4000, 0x7fff, 0x8000, 0xffff] {
            for b in 0..=u16::MAX {
                rtp16_wraps(a, b, b);
                pts15_wraps(a, b, b);
            }
        }
    }
}

#[cfg(kani)]
mod proofs {
    /// Every u16 point and move on a clock of 2¹⁶ ticks, the storage's own wrapping.
    #[kani::proof]
    fn rtp16() {
        super::rtp16_wraps(kani::any(), kani::any(), kani::any());
    }

    /// Every u16 point and move on a clock of 2¹⁵ ticks, by `rem_euclid`.
    #[kani::proof]
    fn pts15() {
        super::pts15_wraps(kani::any(), kani::any(), kani::any());
    }
}
