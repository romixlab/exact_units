//! Property: a [OneOf] value's set check (`try_new`) and its [position](Fixed::position) against a walk of the set
//! in `i128` (`reference::set_position`), for sets of every shape the search takes apart: a leading run of
//! consecutive values and a tail (CAN FD's lengths), no run, a run of every value of the storage (`u8` and `i8`,
//! offsets up to 255), more than the 256 values the unrolled steps cover (`u16`, the loop after them), one value, and
//! the ends of `i64` and `u64`, where a run's offset and a tail's search must not overflow. The 8- and 16-bit sets are
//! walked whole; the 64-bit ones drawn, weighted to their values and the numbers either side.

use crate::reference;
use exact_units::prefix::One;
use exact_units::units::Ratio;
use exact_units::{Fixed, OneOf, ValueSet};

/// Declares a set marker of `$ty` with `$values`.
macro_rules! set {
    ($(#[$doc:meta])* $name:ident: $ty:ty = $values:expr) => {
        $(#[$doc])*
        #[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
        pub(crate) struct $name;

        impl ValueSet<$ty> for $name {
            const VALUES: &'static [$ty] = $values;
        }
    };
}

set!(
    /// A run of four, then gaps up to the storage's greatest.
    Steps: u8 = &[1, 2, 3, 4, 8, 16, 254, 255]
);
set!(
    /// No run: powers of two.
    Powers: u8 = &[1, 2, 4, 8, 16, 32, 64, 128]
);
set!(
    /// Every `u8`: one run of 256.
    EveryU8: u8 = &EVERY_U8
);
set!(
    /// Every `i8`: one run of 256 from −128, so the offset of 127 is 255.
    EveryI8: i8 = &EVERY_I8
);
set!(
    /// 400 values, every third from 0, no run: past the eight unrolled steps.
    Thirds: u16 = &THIRDS
);
set!(
    /// A run of 100 and 300 values after it: the tail past the eight steps too.
    RunThenThirds: u16 = &RUN_THEN_THIRDS
);
set!(
    /// One value.
    Single: i16 = &[-5]
);
set!(
    /// A run of two at `i64`'s low end, then gaps up to its high end.
    EndsI64: i64 = &[i64::MIN, i64::MIN + 1, -1, 0, i64::MAX - 1, i64::MAX]
);
set!(
    /// A run of three from 0, then values up to `u64`'s high end, a pair of neighbours past the run.
    EndsU64: u64 = &[0, 1, 2, 1 << 32, u64::MAX - 1, u64::MAX]
);

/// Every `u8`, ascending.
#[expect(
    clippy::indexing_slicing,
    clippy::cast_possible_truncation,
    reason = "a const item: i < 256 indexes the table and fits a u8"
)]
const EVERY_U8: [u8; 256] = {
    let mut t = [0; 256];
    let mut i = 0;
    while i < 256 {
        t[i] = i as u8;
        i += 1;
    }
    t
};

/// Every `i8`, ascending.
#[expect(
    clippy::indexing_slicing,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    reason = "a const item: i < 256 indexes the table, and i - 128 is -128..=127, an i8"
)]
const EVERY_I8: [i8; 256] = {
    let mut t = [0; 256];
    let mut i = 0;
    while i < 256 {
        t[i] = (i as i16 - 128) as i8;
        i += 1;
    }
    t
};

/// 0, 3, 6, .. 1197.
#[expect(
    clippy::indexing_slicing,
    clippy::cast_possible_truncation,
    reason = "a const item: i < 400 indexes the table, and 3 * i <= 1197 fits a u16"
)]
const THIRDS: [u16; 400] = {
    let mut t = [0; 400];
    let mut i = 0;
    while i < 400 {
        t[i] = 3 * i as u16;
        i += 1;
    }
    t
};

/// 0..=99, then 102, 105, .. 999.
#[expect(
    clippy::indexing_slicing,
    clippy::cast_possible_truncation,
    reason = "a const item: i < 400 indexes the table, and 3 * (i - 66) <= 999 fits a u16"
)]
const RUN_THEN_THIRDS: [u16; 400] = {
    let mut t = [0; 400];
    let mut i = 0;
    while i < 400 {
        t[i] = if i < 100 {
            i as u16
        } else {
            3 * (i as u16 - 66)
        };
        i += 1;
    }
    t
};

/// A property per set: `x` is taken exactly when the walk finds it, at the position the walk finds.
macro_rules! property {
    ($($fn:ident: $set:ident, $ty:ty;)*) => {$(
        #[doc = concat!("`x` in [", stringify!($set), "]: taken where the walk finds it, at its position.")]
        pub(crate) fn $fn(x: $ty) {
            type V = Fixed<Ratio, One, $ty, OneOf<$set>>;
            let set: Vec<i128> = $set::VALUES.iter().map(|&v| i128::from(v)).collect();
            let want = reference::set_position(&set, i128::from(x));
            assert_eq!(V::try_new(x).ok().map(V::position), want, "{x}");
            if let Some(at) = want {
                assert_eq!($set::VALUES.get(at), Some(&x), "{x}");
            }
        }
    )*};
}

property! {
    steps: Steps, u8;
    powers: Powers, u8;
    every_u8: EveryU8, u8;
    every_i8: EveryI8, i8;
    thirds: Thirds, u16;
    run_then_thirds: RunThenThirds, u16;
    single: Single, i16;
    ends_i64: EndsI64, i64;
    ends_u64: EndsU64, u64;
}

#[cfg(not(any(kani, fuzzing)))]
mod tests {
    use super::{EndsI64, EndsU64, ValueSet};
    use proptest::prelude::*;
    use proptest::sample::select;

    #[test]
    fn every_8_and_16_bit_value() {
        for x in 0..=u8::MAX {
            super::steps(x);
            super::powers(x);
            super::every_u8(x);
        }
        for x in i8::MIN..=i8::MAX {
            super::every_i8(x);
        }
        for x in 0..=u16::MAX {
            super::thirds(x);
            super::run_then_thirds(x);
        }
        for x in i16::MIN..=i16::MAX {
            super::single(x);
        }
    }

    /// A value of `values`, or one either side of it.
    fn near<T: Copy + core::fmt::Debug + 'static>(
        values: &'static [T],
        step: fn(T, i8) -> T,
    ) -> impl Strategy<Value = T> {
        (select(values), -1_i8..=1).prop_map(move |(v, d)| step(v, d))
    }

    proptest! {
        #[test]
        fn ends_i64(x in prop_oneof![
            1 => any::<i64>(),
            3 => near(EndsI64::VALUES, |v, d| v.wrapping_add(i64::from(d))),
        ]) {
            super::ends_i64(x);
        }

        #[test]
        fn ends_u64(x in prop_oneof![
            1 => any::<u64>(),
            3 => near(EndsU64::VALUES, |v, d| v.wrapping_add_signed(i64::from(d))),
        ]) {
            super::ends_u64(x);
        }
    }
}

#[cfg(kani)]
mod proofs {
    /// Every `u8` against a set with a run and a tail, and one without a run.
    #[kani::proof]
    #[kani::unwind(10)]
    fn u8_sets() {
        let x: u8 = kani::any();
        super::steps(x);
        super::powers(x);
    }

    /// Every `i8` against the set of all of them: the run's offset at both ends.
    #[kani::proof]
    #[kani::unwind(257)]
    fn every_i8() {
        super::every_i8(kani::any());
    }
}
