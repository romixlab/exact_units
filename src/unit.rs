//! What a unit is: its dimension, how its values become coherent SI, and how it is written.

use crate::Kind;
#[cfg(feature = "shrink_wrap")]
use crate::SIExpr;
use crate::tree;
use core::fmt;
#[cfg(feature = "shrink_wrap")]
use shrink_wrap::prelude::*;

/// Most counted bases a [Dim] has at once (`Sample·Frame⁻¹` has two).
pub const MAX_COUNTED: usize = 4;

/// A counted base of a [Dim] and its exponent: what is counted, named by the crate and the marker that define it
/// (`exact_units` `Sample`, an API crate's own `Bucket`), and how often it is in the dimension (`Sample/s` has `Sample` to
/// the 1st power).
///
/// A base is told apart by crate and marker name together: two crates' `Sample` markers are different bases, and a
/// marker stays the same base when it moves between modules of its crate. Shown by the marker name alone.
#[cfg_attr(
    feature = "shrink_wrap",
    derive_shrink_wrap(
        borrowed,
        owned(feature = "std"),
        derive_owned(Clone, Debug, PartialEq, Eq, Hash),
        cfg_attr_owned(feature = "serde", derive(serde::Serialize, serde::Deserialize)),
        final_structure
    )
)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Counted<'i> {
    /// The crate that defines the marker, as Rust names it (`exact_units`, `my_api`): `CARGO_CRATE_NAME`.
    pub crate_name: &'i str,
    /// The marker's name: `Sample`.
    pub name: &'i str,
    /// The exponent: `Sample/s` has 1.
    #[cfg_attr(feature = "shrink_wrap", wire(I4))]
    pub exp: i8,
}

/// The dimension of a unit: exponents of the 7 SI base units s, m, kg, A, K, mol, cd, of angle (rad) and of
/// information (bit), each from -8 to 7, and of up to [MAX_COUNTED] counted bases (`Sample`, `Block`, `Item`, ..).
/// Volt is kg·m²·s⁻³·A⁻¹: `Dim::new(-3, 2, 1, -1, 0, 0, 0)`; rad/s is `Dim::new(-1, 0, 0, 0, 0, 0, 0).with_rad(1)`;
/// samples per second is `Dim::counted("exact_units", "Sample").div(Dim::new(1, 0, 0, 0, 0, 0, 0))`.
///
/// With `root` set, the dimension is the square root of the one the exponents describe: a noise density in V/√Hz
/// is kg²·m⁴·s⁻⁵·A⁻² with `root` (`Volt` squared over hertz), shown as `kg·m²·s⁻⁵ᐟ²·A⁻¹`. Only half-integer
/// powers are written this way, so `root` is set only when some exponent is odd ([sqrt](Self::sqrt) and
/// [mul](Self::mul) keep it so).
///
/// Values convert into each other and add only with the same dimension: bytes never become a ratio, an angle never
/// a frequency, samples never bytes (that takes a `Per<Byte, Sample>`).
///
/// On the wire nine 4-bit signed numbers in the order above, the `root` bit, then for each of the [MAX_COUNTED]
/// slots a bit for whether it is used, and a used one's crate name, name and exponent. Used slots come first, sorted
/// by crate and name, with exponents that aren't 0, and `root` only with an odd exponent (a value that isn't so
/// doesn't decode), so two equal dimensions have the same bytes: Volt is `d2 1f 00 00 00 00` (41 bits).
#[cfg_attr(
    feature = "shrink_wrap",
    derive_shrink_wrap(
        borrowed,
        owned(feature = "std"),
        derive_owned(Clone, Debug, PartialEq, Eq, Hash),
        cfg_attr_owned(feature = "serde", derive(serde::Serialize, serde::Deserialize)),
        // serde reads the fields into DimFields and keeps only a canonical dimension, as the wire does
        cfg_attr_owned(feature = "serde", serde(try_from = "DimFields")),
        final_structure,
        validate = Self::validate
    )
)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Dim<'i> {
    /// Time, second.
    #[cfg_attr(feature = "shrink_wrap", wire(I4))]
    pub s: i8,
    /// Length, metre.
    #[cfg_attr(feature = "shrink_wrap", wire(I4))]
    pub m: i8,
    /// Mass, kilogram.
    #[cfg_attr(feature = "shrink_wrap", wire(I4))]
    pub kg: i8,
    /// Electric current, ampere.
    #[cfg_attr(feature = "shrink_wrap", wire(I4))]
    pub a: i8,
    /// Thermodynamic temperature, kelvin.
    #[cfg_attr(feature = "shrink_wrap", wire(I4))]
    pub k: i8,
    /// Amount of substance, mole.
    #[cfg_attr(feature = "shrink_wrap", wire(I4))]
    pub mol: i8,
    /// Luminous intensity, candela.
    #[cfg_attr(feature = "shrink_wrap", wire(I4))]
    pub cd: i8,
    /// Plane angle, radian.
    #[cfg_attr(feature = "shrink_wrap", wire(I4))]
    pub rad: i8,
    /// Information, bit.
    #[cfg_attr(feature = "shrink_wrap", wire(I4))]
    pub bit: i8,
    /// The square root of what the exponents describe, see above.
    pub root: bool,
    /// Counted bases, see [Counted]: used slots first, sorted by crate and name.
    pub counted: [Option<Counted<'i>>; 4],
}

const _: () = assert!(MAX_COUNTED == 4, "the array length in Dim");

/// Number of fixed exponents: s, m, kg, A, K, mol, cd, rad, bit.
const FIXED: usize = 9;

/// An exponent in `const` code, a compile error (a panic at run time) when out of -8..=7.
#[must_use]
pub const fn exp(e: i8) -> i8 {
    assert!(in_range(e), "exponent of a dimension out of -8..=7");
    e
}

/// An exponent a [Dim] holds: -8..=7, a 4-bit signed number on the wire.
const fn in_range(e: i8) -> bool {
    e >= -8 && e <= 7
}

#[expect(
    clippy::cast_possible_truncation,
    reason = "e is checked to be in i8's range just above"
)]
#[expect(
    clippy::as_conversions,
    reason = "const fn: From is not const on stable"
)]
const fn checked_exp(e: i16) -> Option<i8> {
    if e < i8::MIN as i16 || e > i8::MAX as i16 {
        return None;
    }
    if in_range(e as i8) {
        Some(e as i8)
    } else {
        None
    }
}

/// Byte-wise order of two names, in `const` code.
#[expect(
    clippy::indexing_slicing,
    reason = "i < the shorter length, the loop bound"
)]
#[expect(
    clippy::arithmetic_side_effects,
    reason = "i < the shorter length, the loop bound, so i + 1 fits usize"
)]
const fn cmp_names(a: &str, b: &str) -> core::cmp::Ordering {
    use core::cmp::Ordering;
    let (a, b) = (a.as_bytes(), b.as_bytes());
    let mut i = 0;
    while i < a.len() && i < b.len() {
        if a[i] != b[i] {
            return if a[i] < b[i] {
                Ordering::Less
            } else {
                Ordering::Greater
            };
        }
        i += 1;
    }
    if a.len() < b.len() {
        Ordering::Less
    } else if a.len() > b.len() {
        Ordering::Greater
    } else {
        Ordering::Equal
    }
}

/// Order of two counted bases: by crate, then by marker name.
const fn cmp_bases(a: (&str, &str), b: (&str, &str)) -> core::cmp::Ordering {
    match cmp_names(a.0, b.0) {
        core::cmp::Ordering::Equal => cmp_names(a.1, b.1),
        o => o,
    }
}

/// Most counted bases while two dimensions are combined, before zero exponents are dropped.
const MERGED: usize = 2 * MAX_COUNTED;

impl Dim<'static> {
    /// Dimensionless: a ratio.
    pub const ONE: Dim<'static> = Dim::new(0, 0, 0, 0, 0, 0, 0);
}

impl<'i> Dim<'i> {
    /// From the exponents of s, m, kg, A, K, mol, cd, without angle, information or counted bases (add them with
    /// [with_rad](Self::with_rad), [with_bit](Self::with_bit), [mul](Self::mul) by [counted](Self::counted)). Panics
    /// (a compile error in `const` code) when one is out of -8..=7, [checked_new](Self::checked_new) doesn't.
    #[must_use]
    pub const fn new(s: i8, m: i8, kg: i8, a: i8, k: i8, mol: i8, cd: i8) -> Dim<'i> {
        Dim {
            s: exp(s),
            m: exp(m),
            kg: exp(kg),
            a: exp(a),
            k: exp(k),
            mol: exp(mol),
            cd: exp(cd),
            rad: exp(0),
            bit: exp(0),
            root: false,
            counted: [None; MAX_COUNTED],
        }
    }

    /// From the exponents of s, m, kg, A, K, mol, cd, rad, bit, `None` when one is out of -8..=7.
    #[must_use]
    #[expect(clippy::indexing_slicing, reason = "i < FIXED, the loop bound")]
    #[expect(clippy::arithmetic_side_effects, reason = "i < FIXED, the loop bound")]
    pub const fn checked_new(exps: [i8; FIXED]) -> Option<Dim<'i>> {
        let mut i = 0;
        while i < FIXED {
            if !in_range(exps[i]) {
                return None;
            }
            i += 1;
        }
        Some(Dim::from_fixed(exps))
    }

    const fn from_fixed(e: [i8; FIXED]) -> Dim<'i> {
        Dim::new(e[0], e[1], e[2], e[3], e[4], e[5], e[6])
            .with_rad(e[7])
            .with_bit(e[8])
    }

    /// A count of the marker `name` of `crate_name` to the 1st power: the dimension of a counted unit marker
    /// (`Dim::counted("exact_units", "Sample")`). In an API crate, [counted!](crate::counted) fills in the crate.
    #[must_use]
    pub const fn counted(crate_name: &'i str, name: &'i str) -> Dim<'i> {
        let mut d = Dim::new(0, 0, 0, 0, 0, 0, 0);
        d.counted[0] = Some(Counted {
            crate_name,
            name,
            exp: exp(1),
        });
        d
    }

    /// The same with the exponent of angle (rad) set to `e`.
    #[must_use]
    pub const fn with_rad(mut self, e: i8) -> Dim<'i> {
        self.rad = exp(e);
        self
    }

    /// The same with the exponent of information (bit) set to `e`.
    #[must_use]
    pub const fn with_bit(mut self, e: i8) -> Dim<'i> {
        self.bit = exp(e);
        self
    }

    /// The same without angle, as SI writes it (rad = 1): `Torque` (kg·m²·s⁻²·rad⁻¹) becomes the joule's
    /// kg·m²·s⁻², `AngularVelocity` s⁻¹. The explicit bridge for exports to systems without an angle dimension
    /// (UCUM, UNECE, Pint, uom) and for formulas like v = ωr; never applied implicitly.
    #[must_use]
    pub const fn strip_radian(self) -> Dim<'i> {
        self.with_rad(0)
    }

    /// The same without the level bases, as SI writes it (Np = 1): `Per<Neper, Second>` (a damping coefficient)
    /// becomes the hertz's s⁻¹, `Per<Decibel, Meter>` m⁻¹. Drops the gain base `Neper` and the loss base `NeperLoss`
    /// of this crate alike, so the sign of a loss is the caller's to keep. The explicit bridge for exports to systems
    /// without a level dimension (UCUM, Pint, uom), like [strip_radian](Self::strip_radian); never applied
    /// implicitly.
    #[must_use]
    #[expect(
        clippy::indexing_slicing,
        reason = "i and kept are below MAX_COUNTED, the loop bound (kept <= i)"
    )]
    #[expect(clippy::arithmetic_side_effects, reason = "i, kept < MAX_COUNTED")]
    pub const fn strip_neper(self) -> Dim<'i> {
        let mut out = self;
        out.counted = [None; MAX_COUNTED];
        let (mut i, mut kept) = (0, 0);
        while i < MAX_COUNTED {
            if let Some(c) = self.counted[i]
                && !(crate::kind::same_str(c.crate_name, "exact_units")
                    && (crate::kind::same_str(c.name, "Neper")
                        || crate::kind::same_str(c.name, "NeperLoss")))
            {
                out.counted[kept] = Some(c);
                kept += 1;
            }
            i += 1;
        }
        // canonical again: a root whose odd exponent was a level's halves
        match out.combine(Dim::ONE, 0, 1) {
            Some(d) => d,
            None => out,
        }
    }

    /// The exponents of s, m, kg, A, K, mol, cd, rad, bit.
    #[must_use]
    pub const fn exps(&self) -> [i8; FIXED] {
        [
            self.s, self.m, self.kg, self.a, self.k, self.mol, self.cd, self.rad, self.bit,
        ]
    }

    /// The marker names of the counted bases and their exponents, in the order of the wire (by crate and name), as
    /// they are shown; [counted](Self::counted) has their crates.
    pub fn counted_bases(&self) -> impl Iterator<Item = (&'i str, i8)> + '_ {
        self.counted
            .iter()
            .map_while(|c| c.map(|c| (c.name, c.exp)))
    }

    /// Without any exponent: a ratio.
    #[must_use]
    pub const fn is_one(&self) -> bool {
        self.same(Dim::ONE)
    }

    /// `self^times · o^sign`, `None` when an exponent leaves -8..=7 or more than [MAX_COUNTED] counted bases are left.
    /// A side without `root` is squared first when the other has it; the result is made canonical (halved, without
    /// `root`, when every exponent is even).
    #[expect(
        clippy::too_many_lines,
        reason = "one const fn walks both sides' bases in step; split, it would pass the half-built result around"
    )]
    #[expect(
        clippy::as_conversions,
        reason = "const fn: From is not const on stable"
    )]
    #[expect(
        clippy::indexing_slicing,
        reason = "indices below FIXED and below the slot counts, the loop bounds"
    )]
    #[expect(
        clippy::arithmetic_side_effects,
        reason = "counters below FIXED, MAX_COUNTED and MERGED (n grows at most once per step of i or j); sign is -1, 0 or 1 and times is 1 but in checked_pow, where o's exponents are 0 and fa is 1: i8 * 2 + i8 * 2 and i8 * i8 fit i16"
    )]
    const fn combine(self, o: Dim<'i>, sign: i16, times: i16) -> Option<Dim<'i>> {
        // with only one side a root, the other is squared so both describe the square of what they are
        let (fa, fb) = match (self.root, o.root) {
            (false, true) => (2, 1),
            (true, false) => (1, 2),
            _ => (1, 1),
        };
        let a = self.exps();
        let b = o.exps();
        let mut fixed = [0i16; FIXED];
        let mut i = 0;
        while i < FIXED {
            fixed[i] = a[i] as i16 * fa * times + sign * b[i] as i16 * fb;
            i += 1;
        }
        // both lists are sorted: merge them, adding the exponents of a base in both
        let mut merged: [Option<(&'i str, &'i str, i16)>; MERGED] = [None; MERGED];
        let (mut i, mut j, mut n) = (0, 0, 0);
        loop {
            let (c, e) = match (self.slot(i), o.slot(j)) {
                (None, None) => break,
                (Some((ca, ea)), None) => {
                    i += 1;
                    (ca, ea as i16 * fa * times)
                }
                (None, Some((cb, eb))) => {
                    j += 1;
                    (cb, sign * eb as i16 * fb)
                }
                (Some((ca, ea)), Some((cb, eb))) => {
                    match cmp_bases((ca.crate_name, ca.name), (cb.crate_name, cb.name)) {
                        core::cmp::Ordering::Less => {
                            i += 1;
                            (ca, ea as i16 * fa * times)
                        }
                        core::cmp::Ordering::Greater => {
                            j += 1;
                            (cb, sign * eb as i16 * fb)
                        }
                        core::cmp::Ordering::Equal => {
                            i += 1;
                            j += 1;
                            (ca, ea as i16 * fa * times + sign * eb as i16 * fb)
                        }
                    }
                }
            };
            if e != 0 {
                merged[n] = Some((c.crate_name, c.name, e));
                n += 1;
            }
        }
        let mut root = self.root || o.root;
        if root {
            // a square root of exponents that are all even is a plain dimension
            let mut all_even = true;
            let mut i = 0;
            while i < FIXED {
                all_even &= fixed[i] % 2 == 0;
                i += 1;
            }
            let mut i = 0;
            while i < n {
                if let Some((_, _, e)) = merged[i] {
                    all_even &= e % 2 == 0;
                }
                i += 1;
            }
            if all_even {
                root = false;
                let mut i = 0;
                while i < FIXED {
                    fixed[i] /= 2;
                    i += 1;
                }
                let mut i = 0;
                while i < n {
                    if let Some((c, name, e)) = merged[i] {
                        merged[i] = Some((c, name, e / 2));
                    }
                    i += 1;
                }
            }
        }
        if n > MAX_COUNTED {
            return None;
        }
        let mut exps = [0i8; FIXED];
        let mut i = 0;
        while i < FIXED {
            match checked_exp(fixed[i]) {
                Some(e) => exps[i] = e,
                None => return None,
            }
            i += 1;
        }
        let mut out = Dim::from_fixed(exps);
        out.root = root;
        let mut i = 0;
        while i < n {
            if let Some((crate_name, name, e)) = merged[i] {
                let Some(exp) = checked_exp(e) else {
                    return None;
                };
                out.counted[i] = Some(Counted {
                    crate_name,
                    name,
                    exp,
                });
            }
            i += 1;
        }
        Some(out)
    }

    #[expect(
        clippy::indexing_slicing,
        reason = "i < the slots in use, checked first"
    )]
    const fn slot(&self, i: usize) -> Option<(Counted<'i>, i8)> {
        if i >= MAX_COUNTED {
            return None;
        }
        match self.counted[i] {
            Some(c) => Some((c, c.exp)),
            None => None,
        }
    }

    /// The dimension of a product: exponents added. `None` when one leaves -8..=7 or there are more than
    /// [MAX_COUNTED] counted bases.
    #[must_use]
    pub const fn checked_mul(self, o: Dim<'i>) -> Option<Dim<'i>> {
        self.combine(o, 1, 1)
    }

    /// The dimension of a quotient: exponents subtracted. `None` like [checked_mul](Self::checked_mul).
    #[must_use]
    pub const fn checked_div(self, o: Dim<'i>) -> Option<Dim<'i>> {
        self.combine(o, -1, 1)
    }

    /// The dimension of a power: exponents multiplied by `n`. `None` when one leaves -8..=7.
    #[must_use]
    #[expect(
        clippy::as_conversions,
        reason = "const fn: From is not const on stable"
    )]
    pub const fn checked_pow(self, n: i8) -> Option<Dim<'i>> {
        self.combine(Dim::new(0, 0, 0, 0, 0, 0, 0), 0, n as i16)
    }

    /// The dimension of a square root: halved exponents when they are all even, else the same exponents with
    /// `root`. `None` for the root of a root (a fourth root isn't a `Dim`).
    #[must_use]
    pub const fn checked_sqrt(self) -> Option<Dim<'i>> {
        if self.root {
            return None;
        }
        let mut squared_root = self;
        squared_root.root = true;
        // canonical form through combine: halves when every exponent is even
        squared_root.combine(Dim::new(0, 0, 0, 0, 0, 0, 0), 0, 1)
    }

    /// The dimension of a square root, for unit markers: V/√Hz is `Volt.div(Hertz.sqrt())`. Panics (a compile
    /// error in `const` code) where [checked_sqrt](Self::checked_sqrt) is `None`.
    #[must_use]
    #[expect(
        clippy::panic,
        reason = "a compile error in const code (a panic at run time), as documented: a dimension that is already a root"
    )]
    pub const fn sqrt(self) -> Dim<'i> {
        match self.checked_sqrt() {
            Some(d) => d,
            None => panic!("square root of a dimension that is already a root"),
        }
    }

    /// The dimension of a product, for unit markers: panics (a compile error in `const` code) when an exponent leaves
    /// -8..=7 or there are more than [MAX_COUNTED] counted bases.
    #[must_use]
    #[expect(
        clippy::panic,
        reason = "a compile error in const code (a panic at run time), as documented: an exponent out of -8..=7 or too many counted bases"
    )]
    pub const fn mul(self, o: Dim<'i>) -> Dim<'i> {
        match self.checked_mul(o) {
            Some(d) => d,
            None => panic!("exponent of a dimension out of -8..=7, or too many counted bases"),
        }
    }

    /// The dimension of a quotient, for unit markers: panics like [mul](Self::mul).
    #[must_use]
    #[expect(
        clippy::panic,
        reason = "a compile error in const code (a panic at run time), as documented: an exponent out of -8..=7 or too many counted bases"
    )]
    pub const fn div(self, o: Dim<'i>) -> Dim<'i> {
        match self.checked_div(o) {
            Some(d) => d,
            None => panic!("exponent of a dimension out of -8..=7, or too many counted bases"),
        }
    }

    /// `==` in `const` code.
    #[must_use]
    #[expect(
        clippy::indexing_slicing,
        reason = "indices below FIXED and below the slot counts, the loop bounds"
    )]
    #[expect(
        clippy::arithmetic_side_effects,
        reason = "i < FIXED and i < MAX_COUNTED, the loop bounds"
    )]
    pub const fn same(self, o: Dim<'_>) -> bool {
        let a = self.exps();
        let b = o.exps();
        if self.root != o.root {
            return false;
        }
        let mut i = 0;
        while i < FIXED {
            if a[i] != b[i] {
                return false;
            }
            i += 1;
        }
        let mut i = 0;
        while i < MAX_COUNTED {
            match (self.counted[i], o.counted[i]) {
                (None, None) => {}
                (Some(x), Some(y)) => {
                    let same = matches!(
                        cmp_bases((x.crate_name, x.name), (y.crate_name, y.name)),
                        core::cmp::Ordering::Equal
                    );
                    if !same || x.exp != y.exp {
                        return false;
                    }
                }
                _ => return false,
            }
            i += 1;
        }
        true
    }

    /// Whether the dimension is as it must be on the wire: every exponent in -8..=7, counted bases in used slots
    /// first, sorted by crate and name without duplicates, exponents not 0; `root` only with an odd exponent. Every
    /// decoder (the wire, borrowed and owned, and serde) accepts only such a dimension, so two equal dimensions are
    /// equal field by field.
    #[must_use]
    #[expect(
        clippy::indexing_slicing,
        reason = "indices below FIXED and below the slot counts, the loop bounds"
    )]
    #[expect(
        clippy::arithmetic_side_effects,
        reason = "i < FIXED and i < MAX_COUNTED, the loop bounds; i - 1 only after i > 0"
    )]
    pub const fn is_canonical(&self) -> bool {
        let e = self.exps();
        let mut i = 0;
        while i < FIXED {
            if !in_range(e[i]) {
                return false;
            }
            i += 1;
        }
        let mut i = 0;
        while i < MAX_COUNTED {
            if let Some(c) = self.counted[i]
                && !in_range(c.exp)
            {
                return false;
            }
            i += 1;
        }
        if self.root {
            let mut odd = false;
            let mut i = 0;
            while i < FIXED {
                odd |= e[i] % 2 != 0;
                i += 1;
            }
            let mut i = 0;
            while i < MAX_COUNTED {
                if let Some(c) = self.counted[i] {
                    odd |= c.exp % 2 != 0;
                }
                i += 1;
            }
            if !odd {
                return false;
            }
        }
        let mut i = 0;
        let mut ended = false;
        while i < MAX_COUNTED {
            match self.counted[i] {
                None => ended = true,
                Some(c) => {
                    if ended || c.exp == 0 {
                        return false;
                    }
                    if i > 0
                        && let Some(prev) = self.counted[i - 1]
                    {
                        let order = cmp_bases((prev.crate_name, prev.name), (c.crate_name, c.name));
                        if !matches!(order, core::cmp::Ordering::Less) {
                            return false;
                        }
                    }
                }
            }
            i += 1;
        }
        true
    }

    #[cfg(feature = "shrink_wrap")]
    fn validate(&self) -> Result<(), ShrinkWrapError> {
        if self.is_canonical() {
            Ok(())
        } else {
            Err(ShrinkWrapError::SubtypeOutOfRange)
        }
    }
}

/// Written the SI way, base units in the order kg, m, s, A, K, mol, cd, then rad, bit and the counted bases:
/// `kg·m²·s⁻³·A⁻¹`, `rad·s⁻¹`, `Sample·s⁻¹`, `1` when dimensionless; a root with halves: `kg·m²·s⁻⁵ᐟ²·A⁻¹`.
impl fmt::Display for Dim<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let parts = [
            ("kg", self.kg),
            ("m", self.m),
            ("s", self.s),
            ("A", self.a),
            ("K", self.k),
            ("mol", self.mol),
            ("cd", self.cd),
            ("rad", self.rad),
            ("bit", self.bit),
        ];
        let mut first = true;
        for (symbol, exp) in parts.into_iter().chain(self.counted_bases()) {
            if exp == 0 {
                continue;
            }
            if !first {
                f.write_str("·")?;
            }
            first = false;
            f.write_str(symbol)?;
            if self.root && exp % 2 != 0 {
                write_superscript(f, i32::from(exp))?;
                f.write_str("ᐟ²")?;
            } else {
                let exp = if self.root { exp / 2 } else { exp };
                if exp != 1 {
                    write_superscript(f, i32::from(exp))?;
                }
            }
        }
        if first {
            f.write_str("1")?;
        }
        Ok(())
    }
}

#[cfg(all(feature = "std", feature = "shrink_wrap"))]
impl DimOwned {
    fn validate(&self) -> Result<(), ShrinkWrapError> {
        self.as_dim().validate()
    }

    /// The same dimension, borrowing the names of counted bases.
    #[must_use]
    pub fn as_dim(&self) -> Dim<'_> {
        let mut counted = [None; MAX_COUNTED];
        for (slot, c) in counted.iter_mut().zip(&self.counted) {
            *slot = c.as_ref().map(|c| Counted {
                crate_name: c.crate_name.as_str(),
                name: c.name.as_str(),
                exp: c.exp,
            });
        }
        Dim {
            s: self.s,
            m: self.m,
            kg: self.kg,
            a: self.a,
            k: self.k,
            mol: self.mol,
            cd: self.cd,
            rad: self.rad,
            bit: self.bit,
            root: self.root,
            counted,
        }
    }
}

#[cfg(all(feature = "std", feature = "shrink_wrap"))]
impl From<Dim<'_>> for DimOwned {
    fn from(d: Dim<'_>) -> Self {
        DimOwned {
            s: d.s,
            m: d.m,
            kg: d.kg,
            a: d.a,
            k: d.k,
            mol: d.mol,
            cd: d.cd,
            rad: d.rad,
            bit: d.bit,
            root: d.root,
            counted: d.counted.map(|c| {
                c.map(|c| CountedOwned {
                    crate_name: c.crate_name.into(),
                    name: c.name.into(),
                    exp: c.exp,
                })
            }),
        }
    }
}

/// A [DimOwned] as serde reads it, before the check: the same fields.
#[cfg(all(feature = "serde", feature = "std", feature = "shrink_wrap"))]
#[derive(serde::Deserialize)]
struct DimFields {
    s: i8,
    m: i8,
    kg: i8,
    a: i8,
    k: i8,
    mol: i8,
    cd: i8,
    rad: i8,
    bit: i8,
    root: bool,
    counted: [Option<CountedOwned>; MAX_COUNTED],
}

/// A dimension serde read that isn't [canonical](Dim::is_canonical).
#[cfg(all(feature = "serde", feature = "std", feature = "shrink_wrap"))]
struct NotCanonical;

#[cfg(all(feature = "serde", feature = "std", feature = "shrink_wrap"))]
impl fmt::Display for NotCanonical {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(
            "a dimension that isn't canonical: exponents in -8..=7, counted bases first, sorted by crate and name \
             without duplicates, exponents not 0, root only with an odd exponent",
        )
    }
}

#[cfg(all(feature = "serde", feature = "std", feature = "shrink_wrap"))]
impl TryFrom<DimFields> for DimOwned {
    type Error = NotCanonical;

    fn try_from(f: DimFields) -> Result<Self, NotCanonical> {
        let d = DimOwned {
            s: f.s,
            m: f.m,
            kg: f.kg,
            a: f.a,
            k: f.k,
            mol: f.mol,
            cd: f.cd,
            rad: f.rad,
            bit: f.bit,
            root: f.root,
            counted: f.counted,
        };
        if d.as_dim().is_canonical() {
            Ok(d)
        } else {
            Err(NotCanonical)
        }
    }
}

/// `exp` in superscript digits, `⁻` before a negative one.
#[expect(
    clippy::arithmetic_side_effects,
    reason = "p * 10 only while p <= exp / 10, so it stays at most exp; p is at least 1, so never a division by 0"
)]
pub(crate) fn write_superscript(f: &mut fmt::Formatter<'_>, exp: i32) -> fmt::Result {
    if exp < 0 {
        f.write_str("⁻")?;
    }
    let exp = exp.unsigned_abs();
    // the largest power of ten not above exp (1 for 0)
    let mut p = 1;
    while p <= exp / 10 {
        p *= 10;
    }
    loop {
        f.write_str(superscript_digit(exp / p % 10))?;
        if p == 1 {
            return Ok(());
        }
        p /= 10;
    }
}

/// A digit (`d` below 10) in superscript.
pub(crate) const fn superscript_digit(d: u32) -> &'static str {
    match d {
        0 => "⁰",
        1 => "¹",
        2 => "²",
        3 => "³",
        4 => "⁴",
        5 => "⁵",
        6 => "⁶",
        7 => "⁷",
        8 => "⁸",
        _ => "⁹",
    }
}

/// Who a marker is: the crate that declares it, as Rust names it (`CARGO_CRATE_NAME`: `exact_units`, `my_api`), and
/// the marker's name (`Rpm`). Every marker kind has one: a unit ([UnitDef]), an origin ([OriginDef](crate::OriginDef)),
/// a scale ([ScaleDef](crate::ScaleDef)), a [Reference](crate::Reference).
///
/// Two crates' `Rpm` are two units, which print the same. The name is the marker's own, not its module path: a
/// marker stays the same unit when it moves between modules of its crate, so a crate has one marker of a kind per name.
#[cfg_attr(
    feature = "shrink_wrap",
    derive_shrink_wrap(
        borrowed,
        owned(feature = "std"),
        derive_owned(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord),
        cfg_attr_owned(feature = "serde", derive(serde::Serialize, serde::Deserialize)),
        final_structure
    )
)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct MarkerId<'i> {
    /// The crate that declares the marker: `exact_units`.
    pub crate_name: &'i str,
    /// The marker's name: `Volt`.
    pub name: &'i str,
}

/// `crate::Name`: `exact_units::Volt`.
impl fmt::Display for MarkerId<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}::{}", self.crate_name, self.name)
    }
}

/// How a unit is written. Only the symbol for now; spellings, an ASCII form and the prefixes it takes join it (IO-9).
#[cfg_attr(
    feature = "shrink_wrap",
    derive_shrink_wrap(
        borrowed,
        owned(feature = "std"),
        derive_owned(Clone, Debug, PartialEq, Eq, Hash),
        cfg_attr_owned(feature = "serde", derive(serde::Serialize, serde::Deserialize))
    )
)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct UnitNames<'i> {
    /// As written after a number: `V`, empty for [Ratio](crate::units::Ratio).
    pub symbol: &'i str,
}

/// A named unit as data: who it is, how it is written, its dimension, its kind of quantity, its exact factor to
/// coherent SI and where that factor comes from. [unit!](crate::unit) builds one per marker ([UnitExpr::Named]), and the
/// same record is a unit's definition in an API model and in a run-time [SIExpr].
///
/// The scale is a [Magnitude](crate::Magnitude), always: a unit has no offset (°C is the origin
/// [Celsius](crate::origins::Celsius) of kelvins) and no float factor.
#[cfg_attr(
    feature = "shrink_wrap",
    derive_shrink_wrap(
        borrowed,
        owned(feature = "std"),
        derive_owned(Clone, Debug, PartialEq),
        cfg_attr_owned(feature = "serde", derive(serde::Serialize, serde::Deserialize))
    )
)]
#[derive(Copy, Clone, Debug, PartialEq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct UnitDef<'i> {
    /// The crate and marker that declare it.
    pub id: MarkerId<'i>,
    /// How it is written.
    pub names: UnitNames<'i>,
    /// Its dimension.
    pub dim: Dim<'i>,
    /// The kind of quantity of its own, by its ISO 80000 name (`"resistance"` for [Ohm](crate::units::Ohm)), `None`
    /// for a plain unit ([Kind::Named](crate::Kind)). A quantity reference once quantity specs exist (UNIT-8).
    pub quantity: Option<&'i str>,
    /// The exact factor to coherent SI: ° is π/180 rad, eV 1.602176634·10⁻¹⁹ J.
    pub scale: crate::Magnitude,
    /// Whether the scale is the unit's definition or a value a source prints.
    pub provenance: Provenance<'i>,
}

impl UnitDef<'_> {
    /// The [Kind] of a unit with this definition.
    #[must_use]
    pub const fn kind(&self) -> Kind<'_> {
        match self.quantity {
            Some(name) => Kind::Named(name),
            None => Kind::Plain,
        }
    }
}

/// Where the definition of a unit named by its [MarkerId] is found: a run-time [SIExpr](crate::SIExpr)'s `Ref` leaf
/// resolves through one, and [compact](crate::SIExpr::compact_in) turns a definition a table lists into a `Ref`.
///
/// A table is never a global: each walk is given the one it reads. This crate's own units are
/// [units::TABLE](crate::units::TABLE), a crate's `unit! { @table static UNITS; .. }` list is one (`[&UnitExpr]`), an
/// API model is one (WireWeaver's schema bundle), and a slice of tables chains them, the first that lists an id
/// answering:
///
/// ```
/// # #[cfg(feature = "electrical")] {
/// use exact_units::units::{self, Volt};
/// use exact_units::{MarkerId, Unit, UnitTable};
///
/// exact_units::unit! {
///     @table
///     static MINE;
///     /// Shaft speed in revolutions per minute.
///     pub struct Rpm = "rpm", [rad 1, s -1], Exact(1 / 30, pi 1);
/// }
///
/// let tables: [&dyn UnitTable; 2] = [&units::TABLE, &MINE];
/// let volt = tables.unit(MarkerId { crate_name: "exact_units", name: "Volt" });
/// assert_eq!(volt.as_ref(), Volt::EXPR.def());
/// let rpm = tables.unit(MarkerId { crate_name: env!("CARGO_CRATE_NAME"), name: "Rpm" });
/// assert_eq!(rpm.map(|d| d.names.symbol), Some("rpm"));
/// assert_eq!(tables.unit(MarkerId { crate_name: "other", name: "Rpm" }), None);
/// # }
/// ```
pub trait UnitTable {
    /// The definition of the unit `id`, `None` when the table doesn't list it.
    fn unit(&self, id: MarkerId<'_>) -> Option<UnitDef<'_>>;
}

/// The table that lists nothing: a walk over [SIExpr](crate::SIExpr) whose `Ref` leaves stay unresolved.
impl UnitTable for () {
    fn unit(&self, _: MarkerId<'_>) -> Option<UnitDef<'_>> {
        None
    }
}

impl<T: UnitTable + ?Sized> UnitTable for &T {
    fn unit(&self, id: MarkerId<'_>) -> Option<UnitDef<'_>> {
        (**self).unit(id)
    }
}

/// Tables chained: the first that lists `id` answers.
impl<T: UnitTable> UnitTable for [T] {
    fn unit(&self, id: MarkerId<'_>) -> Option<UnitDef<'_>> {
        self.iter().find_map(|t| t.unit(id))
    }
}

/// Tables chained, like the slice.
impl<T: UnitTable, const N: usize> UnitTable for [T; N] {
    fn unit(&self, id: MarkerId<'_>) -> Option<UnitDef<'_>> {
        self.as_slice().unit(id)
    }
}

/// A definition is the table of its own unit.
impl UnitTable for UnitDef<'_> {
    fn unit(&self, id: MarkerId<'_>) -> Option<UnitDef<'_>> {
        (self.id == id).then_some(*self)
    }
}

/// The named units of the tree: `Per<Rpm, Second>` lists `Rpm` and `Second`.
impl UnitTable for UnitExpr {
    fn unit(&self, id: MarkerId<'_>) -> Option<UnitDef<'_>> {
        // a UnitExpr of a type is at most MAX_UNIT_DEPTH deep (Unit::DEPTH); one built by hand is followed as far
        fn find(e: &UnitExpr, id: MarkerId<'_>, depth: usize) -> Option<UnitDef<'static>> {
            let d = depth
                .checked_add(1)
                .filter(|d| *d <= crate::MAX_UNIT_DEPTH)?;
            match *e {
                UnitExpr::Named(def) => (def.id == id).then_some(*def),
                UnitExpr::Mul(a, b) | UnitExpr::Div(a, b) => {
                    find(a, id, d).or_else(|| find(b, id, d))
                }
                UnitExpr::Pow(a, _) | UnitExpr::Sqrt(a) => find(a, id, d),
            }
        }
        find(self, id, 0)
    }
}

#[cfg(all(feature = "std", feature = "shrink_wrap"))]
mod owned_defs {
    use super::{
        MarkerId, MarkerIdOwned, Provenance, ProvenanceOwned, UnitDef, UnitDefOwned, UnitNames,
        UnitNamesOwned,
    };

    impl MarkerIdOwned {
        /// The same identity, borrowed.
        #[must_use]
        pub fn as_id(&self) -> MarkerId<'_> {
            MarkerId {
                crate_name: &self.crate_name,
                name: &self.name,
            }
        }
    }

    impl From<MarkerId<'_>> for MarkerIdOwned {
        fn from(id: MarkerId<'_>) -> Self {
            MarkerIdOwned {
                crate_name: id.crate_name.into(),
                name: id.name.into(),
            }
        }
    }

    impl From<Provenance<'_>> for ProvenanceOwned {
        fn from(p: Provenance<'_>) -> Self {
            match p {
                Provenance::Defined => ProvenanceOwned::Defined,
                Provenance::Conventional { source, year } => ProvenanceOwned::Conventional {
                    source: source.into(),
                    year,
                },
                Provenance::Unknown(v) => ProvenanceOwned::Unknown(v),
            }
        }
    }

    impl UnitDefOwned {
        /// The same definition, borrowed.
        #[must_use]
        pub fn as_def(&self) -> UnitDef<'_> {
            UnitDef {
                id: self.id.as_id(),
                names: UnitNames {
                    symbol: &self.names.symbol,
                },
                dim: self.dim.as_dim(),
                quantity: self.quantity.as_deref(),
                scale: self.scale,
                provenance: match &self.provenance {
                    ProvenanceOwned::Defined => Provenance::Defined,
                    ProvenanceOwned::Conventional { source, year } => Provenance::Conventional {
                        source,
                        year: *year,
                    },
                    ProvenanceOwned::Unknown(v) => Provenance::Unknown(*v),
                },
            }
        }
    }

    impl From<UnitDef<'_>> for UnitDefOwned {
        fn from(d: UnitDef<'_>) -> Self {
            UnitDefOwned {
                id: d.id.into(),
                names: UnitNamesOwned {
                    symbol: d.names.symbol.into(),
                },
                dim: d.dim.into(),
                quantity: d.quantity.map(Into::into),
                scale: d.scale,
                provenance: d.provenance.into(),
            }
        }
    }
}

/// Where a named unit's scale comes from. On the wire (in an API model's `UnitDef`) an evolvable enum, so a later
/// kind of source can be added.
///
/// There is no `Default`: a unit read from data says where its scale comes from, and one that doesn't is refused
/// rather than taken for a definition.
///
/// ```compile_fail
/// let p = exact_units::Provenance::default();
/// ```
#[cfg_attr(
    feature = "shrink_wrap",
    derive_shrink_wrap(
        borrowed,
        owned(feature = "std"),
        derive_owned(Clone, Debug, PartialEq, Eq, Hash),
        cfg_attr_owned(feature = "serde", derive(serde::Serialize, serde::Deserialize)),
        cfg_attr_owned(
            feature = "serde",
            expect(clippy::unsafe_derive_deserialize, reason = "the unsafe is shrink_wrap's, in the generated owned type")
        ),
        ww_repr = unib32
    )
)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum Provenance<'i> {
    /// The scale is the unit's definition: exact by law or by the SI (a foot is 0.3048 m).
    Defined,
    /// The unit is fixed by a physical property (mercury at 32 °F, a calorie at 15 °C): its scale is the value a
    /// source prints, exact in this crate but only by convention, and a newer measurement may differ.
    Conventional {
        /// The document the value is from: `"NIST SP 811"`.
        source: &'i str,
        /// Its edition.
        year: u16,
    },
}

/// A unit as a tree: named units, and products and quotients of them (`Per<Torque, AngularVelocity>` is
/// `Div(Torque, AngularVelocity)`). Every [Unit] has one in [Unit::EXPR]; [units::by_name](crate::units::by_name)
/// finds the named ones of this crate.
#[derive(Debug, PartialEq)]
pub enum UnitExpr {
    /// A unit with a marker of its own.
    Named(&'static UnitDef<'static>),
    /// `A · B`, [Times](crate::units::Times).
    Mul(&'static UnitExpr, &'static UnitExpr),
    /// `A / B`, [Per](crate::units::Per).
    Div(&'static UnitExpr, &'static UnitExpr),
    /// A unit to an integer power: m³ as `Pow(Meter, 3)`. No marker of this crate is one, a user's marker can be.
    Pow(&'static UnitExpr, i8),
    /// The square root of a unit: √Hz in V/√Hz, [Sqrt](crate::units::Sqrt). Its dimension has the root flag
    /// ([Dim::sqrt]) unless every exponent is even.
    Sqrt(&'static UnitExpr),
}

impl UnitExpr {
    /// Levels of the tree: 1 for a named unit, one more for each `Mul`, `Div`, `Pow` or `Sqrt` above the deepest
    /// one. A typed unit has at most [MAX_UNIT_DEPTH](crate::MAX_UNIT_DEPTH) ([Unit::DEPTH]).
    #[must_use]
    pub const fn depth(&self) -> usize {
        let below = match self {
            UnitExpr::Named(_) => return 1,
            UnitExpr::Mul(a, b) | UnitExpr::Div(a, b) => {
                let (a, b) = (a.depth(), b.depth());
                if a > b { a } else { b }
            }
            UnitExpr::Pow(a, _) | UnitExpr::Sqrt(a) => a.depth(),
        };
        below.saturating_add(1)
    }

    /// The dimension of the whole expression. Panics (a compile error in `const` code) when an exponent leaves
    /// -8..=7, like [Dim::mul].
    #[must_use]
    #[expect(
        clippy::panic,
        reason = "a compile error in const code (a panic at run time), as documented: a unit expression whose dimension does not combine"
    )]
    pub const fn dim(&self) -> Dim<'static> {
        match self {
            UnitExpr::Named(def) => def.dim,
            UnitExpr::Mul(a, b) => a.dim().mul(b.dim()),
            UnitExpr::Div(a, b) => a.dim().div(b.dim()),
            UnitExpr::Pow(a, n) => match a.dim().checked_pow(*n) {
                Some(d) => d,
                None => panic!("exponent of a dimension out of -8..=7"),
            },
            UnitExpr::Sqrt(a) => a.dim().sqrt(),
        }
    }

    /// The factor to coherent SI of the whole expression, as a float: [magnitude](Self::magnitude) is the exact one.
    ///
    /// # Panics
    ///
    /// Where a unit's scale keeps μ₀ or G ([Magnitude::to_f64](crate::Magnitude::to_f64)): at compile time in a
    /// `const`, which is where the crate asks for it.
    #[must_use]
    pub const fn factor(&self) -> f64 {
        match self {
            UnitExpr::Named(def) => def.scale.to_f64(),
            UnitExpr::Mul(a, b) => a.factor() * b.factor(),
            UnitExpr::Div(a, b) => a.factor() / b.factor(),
            UnitExpr::Pow(a, n) => crate::num::powi_f64(a.factor(), *n),
            UnitExpr::Sqrt(a) => crate::num::sqrt_f64(a.factor()),
        }
    }

    /// The exact factor to coherent SI of the whole expression: `None` when a product leaves what a
    /// [Magnitude](crate::Magnitude) holds. A pound-force second is
    /// 0.45359237 · 9.80665 N·s.
    #[must_use]
    #[expect(
        clippy::arithmetic_side_effects,
        reason = "i < n.unsigned_abs(), at most 128, the loop bound"
    )]
    pub const fn magnitude(&self) -> Option<crate::Magnitude> {
        use crate::Magnitude;
        match self {
            UnitExpr::Named(def) => Some(def.scale),
            UnitExpr::Mul(a, b) => match (a.magnitude(), b.magnitude()) {
                (Some(a), Some(b)) => ok(a.checked_mul(b)),
                _ => None,
            },
            UnitExpr::Div(a, b) => match (a.magnitude(), b.magnitude()) {
                (Some(a), Some(b)) => ok(a.checked_div(b)),
                _ => None,
            },
            UnitExpr::Pow(a, n) => match a.magnitude() {
                Some(a) => {
                    // powi panics out of range: go through checked products
                    let mut r = Magnitude::ONE;
                    let mut i = 0;
                    while i < n.unsigned_abs() {
                        r = match r.checked_mul(a) {
                            Ok(r) => r,
                            Err(_) => return None,
                        };
                        i += 1;
                    }
                    if *n < 0 {
                        ok(Magnitude::ONE.checked_div(r))
                    } else {
                        Some(r)
                    }
                }
                None => None,
            },
            UnitExpr::Sqrt(a) => match a.magnitude() {
                Some(a) => ok(a.checked_sqrt()),
                None => None,
            },
        }
    }

    /// The definition of a named unit, `None` for a compound one.
    #[must_use]
    pub const fn def(&self) -> Option<&'static UnitDef<'static>> {
        match self {
            UnitExpr::Named(def) => Some(def),
            _ => None,
        }
    }

    /// The symbol as written after a number: `V`, `N·m/(rad/s)`, empty for a ratio. Same as `Display`.
    #[must_use]
    pub fn symbol(&self) -> impl fmt::Display + '_ {
        self
    }

    /// The type name: `Volt`, `Per<Torque, AngularVelocity>`.
    #[must_use]
    pub fn name(&self) -> impl fmt::Display + '_ {
        tree::name_of(self)
    }
}

/// `Result::ok` in `const` code.
const fn ok(r: Result<crate::Magnitude, crate::MagnitudeError>) -> Option<crate::Magnitude> {
    match r {
        Ok(m) => Some(m),
        Err(_) => None,
    }
}

/// The symbol: `V`, `N·m/(rad/s)`, `(rad/s)·(rad/s)`.
impl fmt::Display for UnitExpr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&tree::symbol(&self), f)
    }
}

/// A unit marker: a zero-size type that is never sent, only named in types (`Fixed<units::Volt, Milli, u16>`). The markers of
/// this crate are in [units](crate::units); an API crate declares its own the same way, with [unit!](crate::unit):
///
/// ```
/// use exact_units::{Fixed, Unit, prefix::One};
///
/// exact_units::unit! {
///     /// Revolutions per minute: 2π rad / 60 s, exactly.
///     pub struct Rpm = "rpm", [rad 1, s -1], Exact(1 / 30, pi 1);
/// }
///
/// # use core::f64::consts::TAU;
/// let speed = Fixed::<Rpm, One, u8>::new(60);
/// assert_eq!(format!("{speed}"), "60 rpm");
/// assert!((<Rpm as Unit>::EXPR.factor() * 60.0 - TAU).abs() < 1e-12); // rad/s
/// ```
///
/// A hand-written impl compiles too, but WireWeaver refuses an API that uses such a marker: its walker reads a unit's
/// definition from the `unit!` line, so that clients know it (INT-13).
pub trait Unit: Copy + Default + 'static {
    /// The dimension: SI base-unit exponents, angle, information and counted bases.
    const DIM: Dim<'static>;

    /// The kind of quantity beyond the dimension ([Kind]): plain, unless the `unit!` line names one (`Ohm` is a
    /// resistance), or what a quotient kept (`Per<Volt, Volt>` is a ratio of voltage). [Fixed::convert](crate::Fixed::convert)
    /// needs the same dimension and kind.
    const KIND: Kind<'static> = Kind::Plain;

    /// Name, symbol and scale, or the tree of a compound unit.
    const EXPR: &'static UnitExpr;

    /// Levels of [EXPR](Self::EXPR), [UnitExpr::depth]. [Per](crate::units::Per), [Times](crate::units::Times) and
    /// [Sqrt](crate::units::Sqrt) count it from their parts and refuse more than
    /// [MAX_UNIT_DEPTH](crate::MAX_UNIT_DEPTH), a compile error where the unit's tree is used: a run-time unit
    /// ([SIExpr]) is followed that deep, so a typed one is never deeper.
    const DEPTH: usize = Self::EXPR.depth();

    /// The same unit as it goes on the wire in an `ww_si::AnyQuantity`. Made from [EXPR](Self::EXPR)
    /// for a named unit; a compound marker of its own (not `Per` / `Times`) writes its tree here too. With the
    /// `shrink_wrap` feature.
    #[cfg(feature = "shrink_wrap")]
    const SI_EXPR: &'static SIExpr<'static> = &SIExpr::named(Self::EXPR);
}

/// `Self · R` is a `Self::Output`: filled in by this crate's product table (`Volt · Ampere = Watt`), and for any unit
/// times a [Ratio](crate::units::Ratio).
pub trait UnitMul<R: Unit>: Unit {
    /// The unit of the product.
    type Output: Unit;
}

/// `Self / R` is a `Self::Output`: filled in by this crate's product table (`Watt / Volt = Ampere`), and for any unit
/// divided by a [Ratio](crate::units::Ratio).
pub trait UnitDiv<R: Unit>: Unit {
    /// The unit of the quotient.
    type Output: Unit;
}
