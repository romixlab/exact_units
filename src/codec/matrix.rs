//! [MatrixDef]: the Y′C′BC′R colour-difference matrix as data, exact rationals from `K_R` and `K_B` alone
//! (`K_G = 1 − K_R − K_B`). Not a [Codec](super::Codec) layer, as it mixes three components into three others
//! (a quantizer and a [Curve](super::Curve) each act on one signal alone).
//!
//! ITU-R BT.601, BT.709 and BT.2020 share the same analogue shape, only `K_R` and `K_B` differing
//! ([Bt601], [Bt709Matrix], [Bt2020]): `Y′ = K_R·R′ + K_G·G′ + K_B·B′`, `C′B = (B′ − Y′)/(2·(1 − K_B))`, `C′R = (R′
//! − Y′)/(2·(1 − K_R))`, and the inverse `R′ = Y′ + 2·(1 − K_R)·C′R`, `B′ = Y′ + 2·(1 − K_B)·C′B`, `G′ = (Y′ −
//! K_R·R′ − K_B·B′)/K_G`. Every step is exact `Fraction` arithmetic: no rounding is needed here, since R′G′B′ and
//! Y′C′BC′R are both unquantized signals; a client that wants a quantized code rounds separately, by a named
//! [Rounding](crate::Rounding), into whichever [Fixed](crate::Fixed) or [Codec](super::Codec) it targets.
//!
//! ```
//! use exact_units::codec::matrix::Bt709Matrix;
//! use exact_units::Fraction;
//!
//! // white (R'=G'=B'=1) is Y'=1, C'B=C'R=0, exactly
//! let [y, cb, cr] = Bt709Matrix::DEF.encode(Fraction::ONE, Fraction::ONE, Fraction::ONE).unwrap();
//! assert_eq!((y, cb, cr), (Fraction::ONE, Fraction::ZERO, Fraction::ZERO));
//! // and back
//! assert_eq!(Bt709Matrix::DEF.decode(y, cb, cr), Some((Fraction::ONE, Fraction::ONE, Fraction::ONE)));
//! ```

#[cfg(all(feature = "std", feature = "shrink_wrap"))]
use crate::MarkerIdOwned;
use crate::{Fraction, MarkerId};
#[cfg(feature = "shrink_wrap")]
use shrink_wrap::prelude::*;

/// The Y′C′BC′R matrix of a standard, as data: its id and `K_R`, `K_B` (`K_G` is derived, [k_g](Self::k_g)).
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
pub struct MatrixDef<'i> {
    /// The crate and marker that declare it.
    pub id: MarkerId<'i>,
    /// The weight of R′ in Y′.
    pub k_r: Fraction,
    /// The weight of B′ in Y′.
    pub k_b: Fraction,
}

impl MatrixDef<'_> {
    /// `K_G = 1 − K_R − K_B`, the weight of G′ in Y′.
    #[expect(
        clippy::panic,
        reason = "const fn evaluated only in a const item: the panic is a compile error naming an invalid K_R/K_B"
    )]
    #[must_use]
    pub const fn k_g(&self) -> Fraction {
        let Some(one_minus_r) = Fraction::ONE.checked_sub(self.k_r) else {
            panic!("a MatrixDef's K_R must subtract from 1 without overflow");
        };
        let Some(k_g) = one_minus_r.checked_sub(self.k_b) else {
            panic!("a MatrixDef's K_R + K_B must subtract from 1 without overflow");
        };
        k_g
    }

    /// R′G′B′ (each a fraction of reference white) to Y′C′BC′R, exactly: `None` only on an overflow of the
    /// (tiny, fixed) exact arithmetic, never a rounding loss.
    #[must_use]
    pub fn encode(&self, r: Fraction, g: Fraction, b: Fraction) -> Option<[Fraction; 3]> {
        let k_g = self.k_g();
        let y = self
            .k_r
            .checked_mul(r)?
            .checked_add(k_g.checked_mul(g)?)?
            .checked_add(self.k_b.checked_mul(b)?)?;
        let cb = b.checked_sub(y)?.checked_div(
            Fraction::ONE
                .checked_sub(self.k_b)?
                .checked_mul(Fraction::int(2))?,
        )?;
        let cr = r.checked_sub(y)?.checked_div(
            Fraction::ONE
                .checked_sub(self.k_r)?
                .checked_mul(Fraction::int(2))?,
        )?;
        Some([y, cb, cr])
    }

    /// Y′C′BC′R to R′G′B′, exactly: the inverse of [encode](Self::encode).
    #[must_use]
    pub fn decode(
        &self,
        y: Fraction,
        cb: Fraction,
        cr: Fraction,
    ) -> Option<(Fraction, Fraction, Fraction)> {
        let r = y.checked_add(
            Fraction::int(2)
                .checked_mul(Fraction::ONE.checked_sub(self.k_r)?)?
                .checked_mul(cr)?,
        )?;
        let b = y.checked_add(
            Fraction::int(2)
                .checked_mul(Fraction::ONE.checked_sub(self.k_b)?)?
                .checked_mul(cb)?,
        )?;
        let g = y
            .checked_sub(self.k_r.checked_mul(r)?)?
            .checked_sub(self.k_b.checked_mul(b)?)?
            .checked_div(self.k_g())?;
        Some((r, g, b))
    }
}

/// ITU-R BT.601's Y′C′BC′R matrix (625- and 525-line): `K_R = 0.299`, `K_B = 0.114` (so `K_G = 0.587`).
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct Bt601;

impl Bt601 {
    /// BT.601's matrix as data.
    pub const DEF: MatrixDef<'static> = MatrixDef {
        id: MarkerId {
            crate_name: "exact_units",
            name: "Bt601",
        },
        k_r: Fraction::new(299, 1000),
        k_b: Fraction::new(114, 1000),
    };
}

/// ITU-R BT.709's Y′C′BC′R matrix (HD): `K_R = 0.2126`, `K_B = 0.0722` (so `K_G = 0.7152`). Named `Bt709Matrix`,
/// not `Bt709`, which is [BT.709's inverse OETF](super::video::Bt709).
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct Bt709Matrix;

impl Bt709Matrix {
    /// BT.709's matrix as data.
    pub const DEF: MatrixDef<'static> = MatrixDef {
        id: MarkerId {
            crate_name: "exact_units",
            name: "Bt709Matrix",
        },
        k_r: Fraction::new(2126, 10_000),
        k_b: Fraction::new(722, 10_000),
    };
}

/// ITU-R BT.2020's Y′C′BC′R matrix (UHD, wide colour gamut): `K_R = 0.2627`, `K_B = 0.0593` (so `K_G = 0.6780`).
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct Bt2020;

impl Bt2020 {
    /// BT.2020's matrix as data.
    pub const DEF: MatrixDef<'static> = MatrixDef {
        id: MarkerId {
            crate_name: "exact_units",
            name: "Bt2020",
        },
        k_r: Fraction::new(2627, 10_000),
        k_b: Fraction::new(593, 10_000),
    };
}

#[cfg(test)]
mod tests {
    use super::{Bt601, Bt709Matrix, Bt2020};
    use crate::Fraction;

    /// `K_G` of every matrix here, against the standards' own printed value.
    #[test]
    fn k_g_of_every_matrix() {
        assert_eq!(Bt601::DEF.k_g(), Fraction::new(587, 1000));
        assert_eq!(Bt709Matrix::DEF.k_g(), Fraction::new(7152, 10_000));
        assert_eq!(Bt2020::DEF.k_g(), Fraction::new(678, 1000));
    }

    /// Every matrix round-trips a grid of R′G′B′ triples through `encode` and `decode`.
    #[test]
    fn round_trips() {
        let defs = [Bt601::DEF, Bt709Matrix::DEF, Bt2020::DEF];
        let values = [
            Fraction::ZERO,
            Fraction::new(1, 4),
            Fraction::new(1, 2),
            Fraction::new(3, 4),
            Fraction::ONE,
        ];
        for def in defs {
            for &r in &values {
                for &g in &values {
                    for &b in &values {
                        let [y, cb, cr] = def.encode(r, g, b).unwrap();
                        assert_eq!(def.decode(y, cb, cr), Some((r, g, b)));
                    }
                }
            }
        }
    }
}
