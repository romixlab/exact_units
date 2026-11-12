//! serde of a [Fraction] and an [Exact] as the number alone, in the unit itself: in a human-readable format the
//! text `{:#}` writes for the number (`"333.[3]"`, `"80000000"`, `"1000/7919"`), an integer read too; in a binary
//! one the quadruple `(negative, numerator, denominator, exponent)`, the canonical form ([Fraction::mantissa],
//! [Fraction::exp10]).

use crate::{Exact, ExactNum, Fraction};
use ::serde::de::{self, Deserialize, Deserializer, SeqAccess, Visitor};
use ::serde::ser::{Serialize, Serializer};
use core::fmt;

impl Serialize for Fraction {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        if serializer.is_human_readable() {
            serializer.collect_str(self)
        } else {
            let (num, den) = self.mantissa();
            (self.is_negative(), num, den, self.exp10()).serialize(serializer)
        }
    }
}

impl<'de> Deserialize<'de> for Fraction {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        if deserializer.is_human_readable() {
            deserializer.deserialize_any(TextVisitor)
        } else {
            deserializer.deserialize_tuple(4, PartsVisitor)
        }
    }
}

impl<U, R: ExactNum + Serialize> Serialize for Exact<U, R> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.num().serialize(serializer)
    }
}

impl<'de, U, R: ExactNum + Deserialize<'de>> Deserialize<'de> for Exact<U, R> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        R::deserialize(deserializer).map(Exact::of)
    }
}

struct TextVisitor;

impl Visitor<'_> for TextVisitor {
    type Value = Fraction;

    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("an exact number as text (\"1.5\", \"333.[3]\", \"1000/3\") or an integer")
    }

    fn visit_str<E: de::Error>(self, s: &str) -> Result<Fraction, E> {
        s.parse().map_err(|e| E::custom(format_args!("{s:?}: {e}")))
    }

    fn visit_i64<E: de::Error>(self, v: i64) -> Result<Fraction, E> {
        Ok(Fraction::int(i128::from(v)))
    }

    fn visit_u64<E: de::Error>(self, v: u64) -> Result<Fraction, E> {
        Ok(Fraction::int(i128::from(v)))
    }

    fn visit_i128<E: de::Error>(self, v: i128) -> Result<Fraction, E> {
        Ok(Fraction::int(v))
    }

    fn visit_u128<E: de::Error>(self, v: u128) -> Result<Fraction, E> {
        Fraction::from_parts(false, v, 1)
            .ok_or_else(|| E::custom("unreachable: a denominator of 1"))
    }

    /// A float's digits were rounded by the format's parser before it got here: refused, rule 2.
    fn visit_f64<E: de::Error>(self, v: f64) -> Result<Fraction, E> {
        Err(E::custom(format_args!(
            "{v} is a float, rounded when it was read: write the exact value as text, \"{v}\" or a fraction \"1/3\""
        )))
    }
}

struct PartsVisitor;

impl<'de> Visitor<'de> for PartsVisitor {
    type Value = Fraction;

    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("(negative, numerator, denominator, exponent) in the canonical form")
    }

    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Fraction, A::Error> {
        let missing = || de::Error::custom("(negative, numerator, denominator, exponent): too few");
        let neg: bool = seq.next_element()?.ok_or_else(missing)?;
        let num: u128 = seq.next_element()?.ok_or_else(missing)?;
        let den: u128 = seq.next_element()?.ok_or_else(missing)?;
        let exp: i32 = seq.next_element()?.ok_or_else(missing)?;
        let x = Fraction::from_parts_exp10(neg, num, den, exp)
            .ok_or_else(|| de::Error::custom("a fraction over 0, or one no form of which fits"))?;
        // one value has one form, as on the wire
        if (x.is_negative(), x.mantissa(), x.exp10()) != (neg, (num, den), exp) {
            return Err(de::Error::custom(format_args!(
                "{}{num}e{exp}/{den} is not the canonical form (lowest terms, the power of ten the value's power of 5 \
                 where it fits, a zero +0/1e0)",
                if neg { "-" } else { "" }
            )));
        }
        Ok(x)
    }
}

/// A [BigFraction](crate::BigFraction): in a human-readable format its text as [Fraction]'s (`"333.[3]"`,
/// `"1/7919"`), an integer read too; in a binary one the triple `(negative, numerator, denominator)`, each integer
/// its little-endian bytes without trailing zeros (0 none), in lowest terms: one value, one form.
#[cfg(feature = "bignum")]
mod big {
    use crate::BigFraction;
    use ::serde::de::{self, Deserialize, Deserializer, SeqAccess, Visitor};
    use ::serde::ser::{Serialize, Serializer};
    use alloc::vec::Vec;
    use core::fmt;
    use dashu_int::UBig;

    /// Bytes as serde's bytes, not a sequence of `u8`s.
    struct Bytes<'a>(&'a [u8]);

    impl Serialize for Bytes<'_> {
        fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
            serializer.serialize_bytes(self.0)
        }
    }

    impl Serialize for BigFraction {
        fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
            if serializer.is_human_readable() {
                serializer.collect_str(self)
            } else {
                let (num, den) = (self.numer().to_le_bytes(), self.denom().to_le_bytes());
                (self.is_negative(), Bytes(&num), Bytes(&den)).serialize(serializer)
            }
        }
    }

    impl<'de> Deserialize<'de> for BigFraction {
        fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
            if deserializer.is_human_readable() {
                deserializer.deserialize_any(TextVisitor)
            } else {
                deserializer.deserialize_tuple(3, PartsVisitor)
            }
        }
    }

    struct TextVisitor;

    impl Visitor<'_> for TextVisitor {
        type Value = BigFraction;

        fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.write_str("an exact number as text (\"1.5\", \"333.[3]\", \"1000/3\") or an integer")
        }

        fn visit_str<E: de::Error>(self, s: &str) -> Result<BigFraction, E> {
            s.parse().map_err(|e| E::custom(format_args!("{s:?}: {e}")))
        }

        fn visit_i64<E: de::Error>(self, v: i64) -> Result<BigFraction, E> {
            Ok(BigFraction::int(i128::from(v)))
        }

        fn visit_u64<E: de::Error>(self, v: u64) -> Result<BigFraction, E> {
            Ok(BigFraction::int(i128::from(v)))
        }

        fn visit_i128<E: de::Error>(self, v: i128) -> Result<BigFraction, E> {
            Ok(BigFraction::int(v))
        }

        fn visit_u128<E: de::Error>(self, v: u128) -> Result<BigFraction, E> {
            BigFraction::from_parts(false, UBig::from(v), UBig::ONE)
                .ok_or_else(|| E::custom("unreachable: a denominator of 1"))
        }

        /// A float's digits were rounded by the format's parser before it got here: refused, rule 2.
        fn visit_f64<E: de::Error>(self, v: f64) -> Result<BigFraction, E> {
            Err(E::custom(format_args!(
                "{v} is a float, rounded when it was read: write the exact value as text, \"{v}\" or a fraction \"1/3\""
            )))
        }
    }

    /// An integer's little-endian bytes, as bytes or as a sequence of `u8`s.
    struct BytesVisitor;

    impl<'de> Visitor<'de> for BytesVisitor {
        type Value = Vec<u8>;

        fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.write_str("an integer's little-endian bytes")
        }

        fn visit_bytes<E: de::Error>(self, v: &[u8]) -> Result<Vec<u8>, E> {
            Ok(v.to_vec())
        }

        fn visit_byte_buf<E: de::Error>(self, v: Vec<u8>) -> Result<Vec<u8>, E> {
            Ok(v)
        }

        fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Vec<u8>, A::Error> {
            let mut v = Vec::new();
            while let Some(b) = seq.next_element()? {
                v.push(b);
            }
            Ok(v)
        }
    }

    /// [BytesVisitor] as a [Deserialize].
    struct LeBytes(Vec<u8>);

    impl<'de> Deserialize<'de> for LeBytes {
        fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
            deserializer.deserialize_bytes(BytesVisitor).map(LeBytes)
        }
    }

    struct PartsVisitor;

    impl<'de> Visitor<'de> for PartsVisitor {
        type Value = BigFraction;

        fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.write_str("(negative, numerator, denominator) as little-endian bytes in lowest terms")
        }

        fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<BigFraction, A::Error> {
            let missing = || de::Error::custom("(negative, numerator, denominator): too few");
            let neg: bool = seq.next_element()?.ok_or_else(missing)?;
            let LeBytes(num) = seq.next_element()?.ok_or_else(missing)?;
            let LeBytes(den) = seq.next_element()?.ok_or_else(missing)?;
            let (n, d) = (UBig::from_le_bytes(&num), UBig::from_le_bytes(&den));
            let x = BigFraction::from_parts(neg, n.clone(), d.clone())
                .ok_or_else(|| de::Error::custom("a fraction over 0"))?;
            // one value has one form: lowest terms, no trailing zero bytes, a zero +0/1
            let canonical = num.last() != Some(&0)
                && den.last() != Some(&0)
                && x.numer() == n
                && x.denom() == d
                && x.is_negative() == neg;
            if !canonical {
                return Err(de::Error::custom(
                    "not the canonical form (lowest terms, no trailing zero bytes, a zero +0/1)",
                ));
            }
            Ok(x)
        }
    }
}
