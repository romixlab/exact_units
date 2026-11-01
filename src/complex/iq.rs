//! IQ fixed-point DSP (STD-91): Q15 and Q31 IQ samples ([CQ15], [CQ31]), a complex multiply into Q15 or Q31 that
//! rounds once and saturates per part, the conjugate multiply, a multiply-accumulate in the wide type ([Mac]) and
//! the dot products built on it, safe adaptors for interleaved sample buffers ([Interleaved]), and an NCO ([Nco])
//! whose phase accumulator drives a unit phasor by CORDIC.

use super::{Complex, ComplexStorage};
#[cfg(all(feature = "angle", feature = "dimensionless"))]
use crate::Bam32;
use crate::fixed::{check_product, round_div_i128, settle_i128};
use crate::prefix::{Pow2, Product};
#[cfg(feature = "dimensionless")]
use crate::units::Ratio;
use crate::units::Times;
use crate::{FixedScale, Overflow, Rounding, Saturating, Unbounded, Unit, Unlimited};
use core::fmt;
use core::iter::FusedIterator;
use core::marker::PhantomData;
use core::slice::Iter;

/// A Q15 IQ sample: `re` (in-phase) and `im` (quadrature) as `i16` in steps of 2⁻¹⁵, -1 to 1 − 2⁻¹⁵ each,
/// saturating. `CQ15<Volt>` is a voltage phasor in Q15 of a full scale.
#[cfg(feature = "dimensionless")]
pub type CQ15<U = Ratio> = Complex<U, Pow2<-15>, i16, Saturating>;

/// A Q31 IQ sample: `i32` parts in steps of 2⁻³¹, saturating.
#[cfg(feature = "dimensionless")]
pub type CQ31<U = Ratio> = Complex<U, Pow2<-31>, i32, Saturating>;

mod sealed {
    pub trait Sealed {}
    impl Sealed for i16 {}
    impl Sealed for i32 {}
}

/// The storage of an IQ sample, `i16` or `i32`: the accumulator a [Mac] sums its products in, how many products it
/// holds, and the unit phasor of an [Nco].
pub trait IqStorage: ComplexStorage + sealed::Sealed {
    /// The Q format whose full scale is ±1: `Pow2<-15>` for `i16`, `Pow2<-31>` for `i32`.
    type Q: FixedScale;
    /// The accumulator: `i64` for `i16`, `i128` for `i32`.
    type Acc: Copy + Default + Eq + fmt::Debug;
    /// The number of complex products a [Mac] holds exactly, whatever their values: 2³² − 1 for `i16`, 2⁶⁴ − 1 for
    /// `i32` (see [Mac] for the proof).
    const HEADROOM: u64;
    /// `acc + (a·b ± c·d)`, exact in the widened type and added modulo 2^bits of the accumulator.
    #[doc(hidden)]
    fn mac(acc: Self::Acc, ab: (Self, Self), cd: (Self, Self), neg: bool) -> Self::Acc;
    #[doc(hidden)]
    fn acc_to_i128(acc: Self::Acc) -> i128;
    /// `(cos a, sin a)` in [Q](Self::Q), by [CORDIC](crate::cordic).
    #[cfg(all(feature = "angle", feature = "dimensionless"))]
    #[doc(hidden)]
    fn unit_phasor(a: Bam32) -> (Self, Self);
}

impl IqStorage for i16 {
    type Q = Pow2<-15>;
    type Acc = i64;
    const HEADROOM: u64 = u32::MAX as u64;
    #[inline(always)]
    fn mac(acc: i64, (a, b): (i16, i16), (c, d): (i16, i16), neg: bool) -> i64 {
        // |a·b ± c·d| ≤ 2³¹, an i64 holds it
        let (ab, cd) = (a as i64 * b as i64, c as i64 * d as i64);
        acc.wrapping_add(if neg { ab - cd } else { ab + cd })
    }
    #[inline(always)]
    fn acc_to_i128(acc: i64) -> i128 {
        acc as i128
    }
    #[cfg(all(feature = "angle", feature = "dimensionless"))]
    #[inline(always)]
    fn unit_phasor(a: Bam32) -> (i16, i16) {
        let (s, c) = crate::cordic::sin_cos_32_q15(a);
        (c.get(), s.get())
    }
}

impl IqStorage for i32 {
    type Q = Pow2<-31>;
    type Acc = i128;
    const HEADROOM: u64 = u64::MAX;
    #[inline(always)]
    fn mac(acc: i128, (a, b): (i32, i32), (c, d): (i32, i32), neg: bool) -> i128 {
        // each product fits an i64 (|a·b| ≤ 2⁶²), their sum (2⁶³) only an i128
        let (ab, cd) = ((a as i64 * b as i64) as i128, (c as i64 * d as i64) as i128);
        acc.wrapping_add(if neg { ab - cd } else { ab + cd })
    }
    #[inline(always)]
    fn acc_to_i128(acc: i128) -> i128 {
        acc
    }
    #[cfg(all(feature = "angle", feature = "dimensionless"))]
    #[inline(always)]
    fn unit_phasor(a: Bam32) -> (i32, i32) {
        let (s, c) = crate::cordic::sin_cos_32(a);
        (c.get(), s.get())
    }
}

/// Products into the operands' own scale and storage, rounded once and saturated per part.
impl<U: Unit, P: FixedScale, T: ComplexStorage, B: Unlimited<T>> Complex<U, P, T, B> {
    /// `self · rhs` in the scale and storage of both, each part rounded once by `r` from the exact sum in the
    /// widened type and saturated at the ends of `T`: the Q15 · Q15 → Q15 complex multiply. Only −1 · −1 and its
    /// relatives reach past Q15: (−1 − j)² = 2j is `0 + j·(1 − 2⁻¹⁵)`, (−1)·(−1) is `1 − 2⁻¹⁵`.
    ///
    /// ```
    /// # #[cfg(feature = "dimensionless")] {
    /// use exact_units::{CQ15, Complex, Rounding};
    /// let a: CQ15 = Complex::of(16384, 16384); // 0.5 + 0.5j
    /// assert_eq!(a.mul_round(a, Rounding::NearestEven).get(), (0, 16384)); // 0.5j
    /// let m: CQ15 = Complex::of(-32768, -32768); // −1 − j
    /// assert_eq!(m.mul_round(m, Rounding::NearestEven).get(), (0, 32767)); // 2j, saturated
    /// # }
    /// ```
    #[inline(always)]
    #[allow(clippy::type_complexity)] // the written form of the result is the point
    pub fn mul_round<U2: Unit, B2: Unlimited<T>>(
        self,
        rhs: Complex<U2, P, T, B2>,
        r: Rounding,
    ) -> Complex<Times<U, U2>, P, T, Saturating> {
        let (re, im) = self.saturating_product::<U2, P, B2>(rhs, false, r);
        Complex::of(re, im)
    }

    /// `self · conj(rhs)` as [mul_round](Self::mul_round) does `self · rhs`: rounded once, saturated per part, the
    /// conjugate never formed in `T`. (−1 − j)·conj(−1 − j) = |−1 − j|² = 2 is `1 − 2⁻¹⁵`.
    ///
    /// ```
    /// # #[cfg(feature = "dimensionless")] {
    /// use exact_units::{CQ15, Complex, Rounding};
    /// let a: CQ15 = Complex::of(0, -32768); // −j: its conjugate j isn't a Q15 value, the product is exact anyway
    /// let b: CQ15 = Complex::of(0, 16384); // 0.5j
    /// assert_eq!(b.mul_conj_round(a, Rounding::NearestEven).get(), (-16384, 0)); // 0.5j · j = −0.5
    /// # }
    /// ```
    #[inline(always)]
    #[allow(clippy::type_complexity)]
    pub fn mul_conj_round<U2: Unit, B2: Unlimited<T>>(
        self,
        rhs: Complex<U2, P, T, B2>,
        r: Rounding,
    ) -> Complex<Times<U, U2>, P, T, Saturating> {
        let (re, im) = self.saturating_product::<U2, P, B2>(rhs, true, r);
        Complex::of(re, im)
    }

    /// The parts of `self · rhs` (or `· conj(rhs)`) in `P`, `T`, saturated.
    #[inline(always)]
    fn saturating_product<U2: Unit, P2: FixedScale, B2: Unlimited<T>>(
        self,
        rhs: Complex<U2, P2, T, B2>,
        conj: bool,
        r: Rounding,
    ) -> (T, T) {
        // `Saturate` always settles: the zero is never taken
        self.wide_product::<U2, P2, B2, P, T>(rhs, conj, r, Overflow::Saturate)
            .unwrap_or((T::ZERO, T::ZERO))
    }
}

/// Mixing: a product with a [Ratio], a phasor of a local oscillator, keeps the unit (48 V · 0.5 = 24 V).
#[cfg(feature = "dimensionless")]
impl<U: Unit, P: FixedScale, T: ComplexStorage, B: Unlimited<T>> Complex<U, P, T, B> {
    /// `self · lo` for a plain ratio `lo` in any scale `PL` (a Q15 phasor of an oscillator): the result in `self`'s
    /// unit, scale and storage, each part rounded once by `r` and saturated. The up-conversion of an IQ sample by
    /// e^(jθ), a gain, a rotation.
    ///
    /// ```
    /// # #[cfg(feature = "dimensionless")] {
    /// use exact_units::{CQ15, Complex, Rounding, units::Volt};
    /// let x: CQ15<Volt> = Complex::of(16384, 0); // 0.5 V of full scale, in phase
    /// let j: CQ15 = Complex::of(0, 32767); // a quarter turn, as close to j as Q15 gets
    /// let y: CQ15<Volt> = x.mix(j, Rounding::NearestEven);
    /// assert_eq!(y.get(), (0, 16384));
    /// # }
    /// ```
    ///
    /// The oscillator is a plain ratio; a quantity mixed with a quantity is [mul_round](Self::mul_round)'s product:
    ///
    /// ```compile_fail
    /// use exact_units::{CQ15, Complex, Rounding, units::Volt};
    /// let x: CQ15<Volt> = Complex::of(16384, 0);
    /// let _ = x.mix(x, Rounding::NearestEven);
    /// ```
    #[inline(always)]
    pub fn mix<PL: FixedScale, BL: Unlimited<T>>(
        self,
        lo: Complex<Ratio, PL, T, BL>,
        r: Rounding,
    ) -> Complex<U, P, T, Saturating> {
        let (re, im) = self.saturating_product::<Ratio, PL, BL>(lo, false, r);
        Complex::of(re, im)
    }

    /// `self · conj(lo)`: the down-conversion of an IQ sample by e^(−jθ) when `lo` is e^(jθ), as [mix](Self::mix).
    #[inline(always)]
    pub fn mix_conj<PL: FixedScale, BL: Unlimited<T>>(
        self,
        lo: Complex<Ratio, PL, T, BL>,
        r: Rounding,
    ) -> Complex<U, P, T, Saturating> {
        let (re, im) = self.saturating_product::<Ratio, PL, BL>(lo, true, r);
        Complex::of(re, im)
    }
}

/// A sum of complex products, exact, in the wide type of `T` ([IqStorage::Acc]: `i64` for Q15, `i128` for Q31), in
/// the unit `U` and scale `P` of the products (`Times<U1, U2>`, `Product<P1, P2>`: Q30 for two Q15). Nothing is
/// rounded until [result](Self::result) rounds the sum once into the scale and storage asked for.
///
/// **Headroom.** A part of one product of `n`-bit numbers, `ac − bd`, `ad + bc` or their conjugate forms, lies in
/// −2²ⁿ⁻¹ + 2ⁿ⁻¹ ..= 2²ⁿ⁻¹, the top reached by `ad + bc` with all four at `MIN` ((−1 − j)², `2j`). So `N` products
/// sum to at most N · 2²ⁿ⁻¹ and the accumulator of 4n bits holds them while N · 2²ⁿ⁻¹ ≤ 2⁴ⁿ⁻¹ − 1, that is
/// N ≤ 2²ⁿ − 1 ([IqStorage::HEADROOM]):
///
/// | samples | products | accumulator | headroom, products |
/// |---|---|---|---|
/// | [CQ15] | Q30, ≤ 2³¹ | `i64` | 2³² − 1 |
/// | [CQ31] | Q62, ≤ 2⁶³ | `i128` | 2⁶⁴ − 1 |
///
/// A slice on a 32-bit target holds fewer than 2³² samples, so a [dot](Self::dot) of two Q15 slices never
/// overflows there. The sums are added modulo 2^bits of the accumulator, and two's complement addition is exact
/// modulo 2^bits: partial sums may pass the ends on the way and the total is still exact wherever the total itself
/// fits, so the headroom bounds the total, not the path to it.
///
/// ```
/// # #[cfg(feature = "dimensionless")] {
/// use exact_units::{CQ15, Complex, Mac, Overflow, Rounding, prefix::Pow2};
/// let x: [CQ15; 3] = [Complex::of(16384, 0), Complex::of(0, 16384), Complex::of(-16384, 0)]; // 0.5, 0.5j, −0.5
/// let mut acc = Mac::new();
/// for s in x {
///     acc.mac_conj(s, s); // |s|², each 0.25
/// }
/// assert_eq!(acc.get(), (3 << 28, 0)); // 0.75 in Q30
/// let energy = acc.result::<Pow2<-15>, i16>(Rounding::NearestEven, Overflow::Saturate).unwrap();
/// assert_eq!(energy.get(), (24576, 0)); // 0.75 in Q15
/// assert_eq!(Mac::dot_conj(&x, &x), acc);
/// # }
/// ```
///
/// The result goes to a scale the products' is a whole multiple of, not to a finer one:
///
/// ```compile_fail
/// use exact_units::{CQ15, Complex, Mac, Overflow, Rounding, prefix::Pow2};
/// let x: CQ15 = Complex::of(1, 0);
/// let mut acc = Mac::new();
/// acc.mac(x, x);
/// let _ = acc.result::<Pow2<-31>, i64>(Rounding::Floor, Overflow::Checked);
/// ```
pub struct Mac<U, P, T: IqStorage> {
    re: T::Acc,
    im: T::Acc,
    _unit: PhantomData<(U, P)>,
}

impl<U, P, T: IqStorage> Mac<U, P, T> {
    /// An empty sum.
    #[inline(always)]
    pub fn new() -> Self {
        Self::default()
    }

    /// The two sums in units of `P`, `(re, im)`.
    #[inline(always)]
    pub fn get(&self) -> (T::Acc, T::Acc) {
        (self.re, self.im)
    }

    /// Back to an empty sum.
    #[inline(always)]
    pub fn clear(&mut self) {
        *self = Self::default();
    }
}

impl<U: Unit, P: FixedScale, T: IqStorage> Mac<U, P, T> {
    /// The sum in the scale `P3` and storage `T3`, each part rounded once by `r` and settled by `o`; `None` only for
    /// [Overflow::Checked] where a part doesn't fit `T3`. `P3` is a whole multiple of `P` (Q30 to Q15: 2¹⁵ steps),
    /// checked at compile time: a finer scale than the products' would add nothing.
    #[inline]
    pub fn result<P3: FixedScale, T3: ComplexStorage>(
        &self,
        r: Rounding,
        o: Overflow,
    ) -> Option<Complex<U, P3, T3>> {
        let den = const {
            match P::SCALE.div(P3::SCALE).to_ratio() {
                Some((1, den)) if den <= i128::MAX as u128 => den as i128,
                _ => panic!(
                    "a MAC result's scale is a whole multiple of the products' scale (Q30 to Q15, not Q15 to Q30)"
                ),
            }
        };
        let part = |x: T::Acc| {
            let x = T::acc_to_i128(x);
            let y = if den == 1 {
                x
            } else {
                round_div_i128(x, den, r)
            };
            settle_i128::<T3>(y, o)
        };
        Some(Complex::new(part(self.re)?, part(self.im)?))
    }

    /// Whether the mean of the real sum over `n` products is at least `threshold`, `Σ re ≥ threshold · n`, exact:
    /// a power detector's test of Σ|y|² (`mac_conj(y, y)`) against a level in the products' unit and scale,
    /// `db!(norm_sqr -10)` or `fixed!(0.1)`. One multiply and one compare of integers, no division. The imaginary
    /// sum takes no part (a power sum has none); no products (`n = 0`) are no signal, `false`. The threshold is an
    /// integer storage.
    ///
    /// ```
    /// # #[cfg(feature = "dimensionless")] {
    /// use exact_units::{CQ15, Complex, Fixed, Mac, fixed, prefix::{Pow2, Product}, units::{Ratio, Times}};
    /// type PowerQ30 = Fixed<Times<Ratio, Ratio>, Product<Pow2<-15>, Pow2<-15>>, u32>;
    /// const BUSY: PowerQ30 = fixed!(0.1);
    /// let y: CQ15 = Complex::of(16384, 0); // 0.5, a power of 0.25
    /// let mut power = Mac::new();
    /// power.mac_conj(y, y);
    /// power.mac_conj(y, y);
    /// assert!(power.mean_at_least(BUSY, 2));
    /// assert!(!power.mean_at_least(BUSY, 6)); // 0.5 over 6 is 0.083
    /// # }
    /// ```
    #[inline]
    pub fn mean_at_least<T2: crate::Storage, B2: crate::Bounds<T2>>(
        &self,
        threshold: crate::Fixed<U, P, T2, B2>,
        n: usize,
    ) -> bool {
        const { assert!(!T2::IS_FLOAT, "a Mac's threshold is an integer storage") };
        let t = match threshold.get().to_repr() {
            crate::Repr::Int(t) => t,
            crate::Repr::Float(_) => unreachable!(),
        };
        if n == 0 {
            return false;
        }
        match t.checked_mul(n as i128) {
            Some(tn) => T::acc_to_i128(self.re) >= tn,
            // beyond i128, which no sum reaches: above it for a positive threshold, below for a negative one
            None => t < 0,
        }
    }
}

impl<U1: Unit, P1: FixedScale, U2: Unit, P2: FixedScale, T: IqStorage>
    Mac<Times<U1, U2>, Product<P1, P2>, T>
{
    /// Adds `a · b`, exactly.
    #[inline(always)]
    pub fn mac<B1: Unlimited<T>, B2: Unlimited<T>>(
        &mut self,
        a: Complex<U1, P1, T, B1>,
        b: Complex<U2, P2, T, B2>,
    ) {
        const { check_product::<U1, U2, P1, P2>(false) };
        let (a, b, c, d) = (a.re, a.im, b.re, b.im);
        self.re = T::mac(self.re, (a, c), (b, d), true);
        self.im = T::mac(self.im, (a, d), (b, c), false);
    }

    /// Adds `a · conj(b)`, exactly: a correlation, an energy (`a · conj(a)`), a phase difference.
    #[inline(always)]
    pub fn mac_conj<B1: Unlimited<T>, B2: Unlimited<T>>(
        &mut self,
        a: Complex<U1, P1, T, B1>,
        b: Complex<U2, P2, T, B2>,
    ) {
        const { check_product::<U1, U2, P1, P2>(false) };
        let (a, b, c, d) = (a.re, a.im, b.re, b.im);
        self.re = T::mac(self.re, (a, c), (b, d), false);
        self.im = T::mac(self.im, (b, c), (a, d), true);
    }

    /// Σ `a[i] · b[i]`, exactly.
    ///
    /// # Panics
    ///
    /// If the slices differ in length, as `copy_from_slice` does: a dot product of a slice with a shorter one is a
    /// bug, not a shorter sum.
    pub fn dot<B1: Unlimited<T>, B2: Unlimited<T>>(
        a: &[Complex<U1, P1, T, B1>],
        b: &[Complex<U2, P2, T, B2>],
    ) -> Self {
        assert_eq!(
            a.len(),
            b.len(),
            "a dot product of slices of different lengths"
        );
        let mut acc = Self::new();
        for (&a, &b) in a.iter().zip(b) {
            acc.mac(a, b);
        }
        acc
    }

    /// Σ `a[i] · conj(b[i])`, exactly: the correlation of two signals at lag 0, the energy of one with itself.
    ///
    /// # Panics
    ///
    /// If the slices differ in length, as [dot](Self::dot).
    pub fn dot_conj<B1: Unlimited<T>, B2: Unlimited<T>>(
        a: &[Complex<U1, P1, T, B1>],
        b: &[Complex<U2, P2, T, B2>],
    ) -> Self {
        assert_eq!(
            a.len(),
            b.len(),
            "a dot product of slices of different lengths"
        );
        let mut acc = Self::new();
        for (&a, &b) in a.iter().zip(b) {
            acc.mac_conj(a, b);
        }
        acc
    }
}

impl<U, P, T: IqStorage> Clone for Mac<U, P, T> {
    #[inline(always)]
    fn clone(&self) -> Self {
        *self
    }
}

impl<U, P, T: IqStorage> Copy for Mac<U, P, T> {}

impl<U, P, T: IqStorage> Default for Mac<U, P, T> {
    #[inline(always)]
    fn default() -> Self {
        Mac {
            re: T::Acc::default(),
            im: T::Acc::default(),
            _unit: PhantomData,
        }
    }
}

impl<U, P, T: IqStorage> PartialEq for Mac<U, P, T> {
    #[inline(always)]
    fn eq(&self, other: &Self) -> bool {
        self.re == other.re && self.im == other.im
    }
}

impl<U, P, T: IqStorage> Eq for Mac<U, P, T> {}

/// `Mac(3+0j, i64)`: the two sums in units of the products' scale, and the accumulator.
impl<U, P, T: IqStorage> fmt::Debug for Mac<U, P, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Mac({:?}, {:?})", self.re, self.im)
    }
}

/// The complex samples of an interleaved buffer `[re₀, im₀, re₁, im₁, ..]`, two numbers at a time: made by
/// [Complex::from_interleaved]. An odd number at the end is no sample; [remainder](Self::remainder) has it.
pub struct Interleaved<'a, U, P, T: ComplexStorage, B: Unlimited<T> = Unbounded> {
    pairs: Iter<'a, [T; 2]>,
    rest: &'a [T],
    _value: PhantomData<(U, P, B)>,
}

impl<'a, U, P, T: ComplexStorage, B: Unlimited<T>> Interleaved<'a, U, P, T, B> {
    /// The number left over at the end of an odd buffer, or nothing.
    #[inline(always)]
    pub fn remainder(&self) -> &'a [T] {
        self.rest
    }
}

impl<U, P, T: ComplexStorage, B: Unlimited<T>> Clone for Interleaved<'_, U, P, T, B> {
    fn clone(&self) -> Self {
        Interleaved {
            pairs: self.pairs.clone(),
            rest: self.rest,
            _value: PhantomData,
        }
    }
}

impl<U, P, T: ComplexStorage, B: Unlimited<T>> Iterator for Interleaved<'_, U, P, T, B> {
    type Item = Complex<U, P, T, B>;
    #[inline(always)]
    fn next(&mut self) -> Option<Self::Item> {
        self.pairs.next().map(|&[re, im]| Complex::of(re, im))
    }
    #[inline(always)]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.pairs.size_hint()
    }
}

impl<U, P, T: ComplexStorage, B: Unlimited<T>> DoubleEndedIterator for Interleaved<'_, U, P, T, B> {
    #[inline(always)]
    fn next_back(&mut self) -> Option<Self::Item> {
        self.pairs.next_back().map(|&[re, im]| Complex::of(re, im))
    }
}

impl<U, P, T: ComplexStorage, B: Unlimited<T>> ExactSizeIterator for Interleaved<'_, U, P, T, B> {}

impl<U, P, T: ComplexStorage, B: Unlimited<T>> FusedIterator for Interleaved<'_, U, P, T, B> {}

/// Interleaved buffers `[re₀, im₀, re₁, im₁, ..]`, the layout of an ADC's or a radio's IQ stream, in and out
/// without `unsafe`: two numbers at a time as `[T; 2]` by `as_chunks`.
impl<U, P, T: ComplexStorage, B: Unlimited<T>> Complex<U, P, T, B> {
    /// The samples of an interleaved buffer.
    ///
    /// ```
    /// # #[cfg(feature = "dimensionless")] {
    /// use exact_units::{CQ15, units::Ratio};
    /// let buf = [100i16, -200, 300, -400, 7];
    /// let mut it = CQ15::<Ratio>::from_interleaved(&buf);
    /// assert_eq!(it.len(), 2);
    /// assert_eq!(it.next().map(CQ15::get), Some((100, -200)));
    /// assert_eq!(it.next().map(CQ15::get), Some((300, -400)));
    /// assert_eq!(it.next(), None);
    /// assert_eq!(it.remainder(), &[7]);
    /// # }
    /// ```
    #[inline(always)]
    pub fn from_interleaved(buf: &[T]) -> Interleaved<'_, U, P, T, B> {
        let (pairs, rest) = buf.as_chunks::<2>();
        Interleaved {
            pairs: pairs.iter(),
            rest,
            _value: PhantomData,
        }
    }

    /// Writes `src` into the interleaved buffer `out`, as many samples as fit (`out.len() / 2`), and returns how
    /// many it wrote.
    ///
    /// ```
    /// # #[cfg(feature = "dimensionless")] {
    /// use exact_units::{CQ15, Complex};
    /// let mut out = [0i16; 4];
    /// let src: [CQ15; 3] = [Complex::of(1, 2), Complex::of(3, 4), Complex::of(5, 6)];
    /// let n = Complex::write_interleaved(&src, &mut out);
    /// assert_eq!((n, out), (2, [1, 2, 3, 4]));
    /// # }
    /// ```
    #[inline]
    pub fn write_interleaved(src: &[Self], out: &mut [T]) -> usize {
        let mut n = 0;
        for (o, s) in out.as_chunks_mut::<2>().0.iter_mut().zip(src) {
            *o = [s.re, s.im];
            n += 1;
        }
        n
    }

    /// Replaces every sample of the interleaved buffer `buf` by `f` of it, in place, and returns how many there were;
    /// an odd number at the end stays as it is. A mixer or a filter over a DMA buffer, without a copy.
    ///
    /// ```
    /// # #[cfg(feature = "dimensionless")] {
    /// use exact_units::{CQ15, units::Ratio};
    /// let mut buf = [100i16, -200, 300, -400];
    /// assert_eq!(CQ15::<Ratio>::map_interleaved(&mut buf, CQ15::conj), 2);
    /// assert_eq!(buf, [100, 200, 300, 400]);
    /// # }
    /// ```
    #[inline]
    pub fn map_interleaved(buf: &mut [T], mut f: impl FnMut(Self) -> Self) -> usize {
        let mut n = 0;
        for c in buf.as_chunks_mut::<2>().0 {
            let y = f(Complex::of(c[0], c[1]));
            *c = [y.re, y.im];
            n += 1;
        }
        n
    }
}

/// A numerically controlled oscillator: a [Bam32] phase accumulator that advances by a step every sample and wraps
/// at a turn, and the unit phasor e^(jθ) of its phase by [CORDIC](crate::cordic), no table and no float.
/// [mix_down](Self::mix_down) takes a signal at the NCO's frequency to 0 Hz (`x · e^(−jθ)`), [mix_up](Self::mix_up)
/// the other way.
///
/// The phasor is a [CQ15] (or [CQ31]) of `Ratio`, each part within 0.51 of a step of the exact cos θ and sin θ, all
/// 32 bits of the phase used; 1 itself is no Q15 value, so the phasor of 0 is `1 − 2⁻¹⁵`. The frequency resolution
/// is fs / 2³²: 11 mHz at 48 kHz.
///
/// ```
/// # #[cfg(all(feature = "angle", feature = "dimensionless"))] {
/// use exact_units::{Bam32, CQ15, Complex, Nco, Rounding};
/// let mut nco = Nco::new(Bam32::of(1 << 30)); // fs / 4: a quarter turn per sample
/// let p: Vec<(i16, i16)> = (0..4).map(|_| nco.tick::<i16>().get()).collect();
/// assert_eq!(p, [(32767, 0), (0, 32767), (-32768, 0), (0, -32768)]);
/// // a tone at fs / 4, mixed down to 0 Hz: a constant
/// let tone: [CQ15; 4] = [Complex::of(16384, 0), Complex::of(0, 16384), Complex::of(-16384, 0), Complex::of(0, -16384)];
/// let mut nco = Nco::new(Bam32::of(1 << 30));
/// for x in tone {
///     assert_eq!(nco.mix_down(x, Rounding::NearestEven).get(), (16384, 0));
/// }
/// # }
/// ```
#[cfg(all(feature = "angle", feature = "dimensionless"))]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Nco {
    phase: Bam32,
    step: Bam32,
}

#[cfg(all(feature = "angle", feature = "dimensionless"))]
impl Nco {
    /// An NCO at phase 0 that advances by `step` (a turn is 2³² steps) per sample. A negative frequency is the step
    /// negated, modulo a turn.
    #[inline(always)]
    pub const fn new(step: Bam32) -> Self {
        Nco {
            phase: Bam32::of(0),
            step,
        }
    }

    /// The step of a frequency `f` at the sample rate `fs` (in one unit, Hz or other): f / fs of a turn, rounded to
    /// the nearest of 2³² steps. `f` at or above `fs` aliases, as it does on the wire; `fs` of 0 panics.
    ///
    /// ```
    /// # #[cfg(all(feature = "angle", feature = "dimensionless"))] {
    /// use exact_units::Nco;
    /// assert_eq!(Nco::tuned(12_000, 48_000).step().get(), 1 << 30);
    /// assert_eq!(Nco::tuned(1_000, 48_000).step().get(), 89_478_485); // 2³² / 48, rounded
    /// # }
    /// ```
    #[inline]
    pub const fn tuned(f: u32, fs: u32) -> Self {
        let step = (((f as u64) << 32) + fs as u64 / 2) / fs as u64;
        Self::new(Bam32::of(step as u32))
    }

    /// The same NCO, now at `phase`.
    #[inline(always)]
    pub const fn with_phase(self, phase: Bam32) -> Self {
        Nco { phase, ..self }
    }

    /// The phase of the next sample.
    #[inline(always)]
    pub const fn phase(&self) -> Bam32 {
        self.phase
    }

    /// The step per sample.
    #[inline(always)]
    pub const fn step(&self) -> Bam32 {
        self.step
    }

    /// e^(jθ) of the current phase θ, `cos θ + j·sin θ`, in `T`'s Q format; the phase stays.
    #[inline(always)]
    pub fn phasor<T: IqStorage>(&self) -> Complex<Ratio, T::Q, T, Saturating> {
        let (c, s) = T::unit_phasor(self.phase);
        Complex::of(c, s)
    }

    /// e^(jθ) of the current phase, then a step on.
    #[inline(always)]
    pub fn tick<T: IqStorage>(&mut self) -> Complex<Ratio, T::Q, T, Saturating> {
        let p = self.phasor::<T>();
        self.phase = Bam32::of(self.phase.get().wrapping_add(self.step.get()));
        p
    }

    /// `x · e^(−jθ)`, then a step on: one sample of a down-converter, rounded once by `r`, saturated per part, in
    /// `x`'s unit, scale and storage ([Complex::mix_conj]).
    #[inline(always)]
    pub fn mix_down<U: Unit, P: FixedScale, T: IqStorage, B: Unlimited<T>>(
        &mut self,
        x: Complex<U, P, T, B>,
        r: Rounding,
    ) -> Complex<U, P, T, Saturating> {
        x.mix_conj(self.tick::<T>(), r)
    }

    /// `x · e^(jθ)`, then a step on: one sample of an up-converter ([Complex::mix]).
    #[inline(always)]
    pub fn mix_up<U: Unit, P: FixedScale, T: IqStorage, B: Unlimited<T>>(
        &mut self,
        x: Complex<U, P, T, B>,
        r: Rounding,
    ) -> Complex<U, P, T, Saturating> {
        x.mix(self.tick::<T>(), r)
    }
}

#[cfg(test)]
mod tests;
