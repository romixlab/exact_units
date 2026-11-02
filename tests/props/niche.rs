//! Property 12: niche codes. Every raw number is a value within the bounds, a niche code or refused, as a reference
//! written from the standard's table says; `get` never answers a code as a value; encoding what `get` answered gives
//! back the same raw number; every code of a set decodes and is outside the bounds (the compile-time check, seen from
//! run time). Rails: a `Clipped` marks exactly its ends. The 8- and 16-bit storages are walked whole, J1939's 4-byte
//! set sampled; Kani proves the u8 round trip.
#![expect(
    clippy::arithmetic_side_effects,
    reason = "a test computes with the values it chose, which fit; an overflow fails the test"
)]
#![expect(clippy::panic, reason = "a property fails by panicking")]
#![cfg_attr(
    not(any(kani, fuzzing)),
    expect(
        clippy::indexing_slicing,
        reason = "a table of four the strategy picks from by a number below four"
    )
)]

use exact_units::niche::{
    BothRails, Clip, Clipped, J1939U8, J1939U16, J1939U32, Niched, Niches, Null, Reading,
};
use exact_units::prefix::One;
use exact_units::units::Ratio;
use exact_units::{Fixed, Range};

/// A J1939 1-byte parameter.
pub(crate) type Byte = Niched<Fixed<Ratio, One, u8, Range<0, 250>>, J1939U8>;

/// A J1939 2-byte parameter.
pub(crate) type Word = Niched<Fixed<Ratio, One, u16, Range<0, 0xFAFF>>, J1939U16>;

/// A J1939 4-byte parameter.
pub(crate) type Dword = Niched<Fixed<Ratio, One, u32, Range<0, 0xFAFF_FFFF>>, J1939U32>;

/// A BLE HCI RSSI with a narrow range: -127..=20 dBm, 127 not available, the rest refused.
pub(crate) type Rssi = Niched<Fixed<Ratio, One, i8, Range<-127, 20>>, Null<127>>;

/// A Matter nullable int16s: -32768 null.
pub(crate) type Int16 = Niched<Fixed<Ratio, One, i16, Range<-32767, 32767>>, Null<-32768>>;

/// The J1939 class of a raw number of `bytes` bytes, from the table: 0 a value, 1 reserved, 2 error, 3 not available.
fn j1939_class(raw: u32, bytes: u32) -> u8 {
    let top = raw >> (8 * (bytes - 1));
    match top {
        0..=0xFA => 0,
        0xFB..=0xFD => 1,
        0xFE => 2,
        _ => 3,
    }
}

/// The class of a J1939 reading: 0 a value, 1 reserved, 2 error, 3 not available.
macro_rules! class {
    ($r:expr, $n:ident, $v:ident) => {
        match $r {
            Reading::Value($v) => {
                let _ = $v;
                0
            }
            Reading::Niche($n::Reserved(..)) => 1,
            Reading::Niche($n::Error { .. }) => 2,
            Reading::Niche($n::NotAvailable { .. }) => 3,
        }
    };
}

/// A J1939 1-byte raw number: every one is a value or a code, of the class the table says, the value's number is the
/// raw number, and it encodes back to itself.
pub(crate) fn j1939_byte(raw: u8) {
    let Ok(n) = Byte::try_from_raw(raw) else {
        panic!("{raw} refused")
    };
    let r = n.get();
    assert_eq!(
        class!(r, J1939U8, v),
        j1939_class(u32::from(raw), 1),
        "{raw}"
    );
    if let Reading::Value(v) = r {
        assert_eq!(v.get(), raw);
    }
    if let Reading::Niche(J1939U8::Reserved(c)) = r {
        assert_eq!(c, raw);
    }
    assert_eq!(Byte::from_reading(r).map(Byte::raw), Ok(raw), "{raw}");
    assert_eq!(n.raw(), raw);
}

/// As [j1939_byte], 2 bytes.
pub(crate) fn j1939_word(raw: u16) {
    let Ok(n) = Word::try_from_raw(raw) else {
        panic!("{raw} refused")
    };
    let r = n.get();
    assert_eq!(
        class!(r, J1939U16, v),
        j1939_class(u32::from(raw), 2),
        "{raw}"
    );
    if let Reading::Value(v) = r {
        assert_eq!(v.get(), raw);
    }
    assert_eq!(Word::from_reading(r).map(Word::raw), Ok(raw), "{raw:#x}");
}

/// As [j1939_byte], 4 bytes.
pub(crate) fn j1939_dword(raw: u32) {
    let Ok(n) = Dword::try_from_raw(raw) else {
        panic!("{raw} refused")
    };
    let r = n.get();
    assert_eq!(class!(r, J1939U32, v), j1939_class(raw, 4), "{raw}");
    if let Reading::Value(v) = r {
        assert_eq!(v.get(), raw);
    }
    assert_eq!(Dword::from_reading(r).map(Dword::raw), Ok(raw), "{raw:#x}");
}

/// An RSSI byte: a value of -127..=20, the code 127, or refused; what reads round-trips.
pub(crate) fn rssi(raw: i8) {
    match (raw, Rssi::try_from_raw(raw)) {
        (-127..=20, Ok(n)) => {
            assert_eq!(n.value().map(Fixed::get), Some(raw));
            assert_eq!(Rssi::from_reading(n.get()).map(Rssi::raw), Ok(raw));
        }
        (127, Ok(n)) => {
            assert_eq!(n.get(), Reading::Niche(Null));
            assert_eq!(
                Option::from(n),
                None::<Fixed<Ratio, One, i8, Range<-127, 20>>>
            );
        }
        (_, Err(_)) if !(-127..=20).contains(&raw) && raw != 127 => {}
        (_, r) => panic!("{raw}: {r:?}"),
    }
}

/// A Matter int16s: every number but -32768 a value, -32768 null, and `Option` round-trips.
pub(crate) fn int16(raw: i16) {
    let Ok(n) = Int16::try_from_raw(raw) else {
        panic!("{raw} refused")
    };
    let o: Option<_> = n.into();
    assert_eq!(o.is_none(), raw == i16::MIN, "{raw}");
    assert_eq!(Int16::from(o).raw(), raw);
}

/// The codes of `N` on the storage `T`: every code in `CODES` decodes and encodes back, a number between two code
/// ranges doesn't decode, and none is a value of `0..=max`.
fn codes_are_outside<T, N>(max: i128)
where
    T: exact_units::Storage + TryFrom<i128> + core::fmt::Debug,
    N: Niches<T>,
{
    for &(lo, hi) in N::CODES {
        assert!(lo > max || hi < 0, "{lo}..={hi} inside 0..={max}");
        for c in [lo, lo.midpoint(hi), hi] {
            let Ok(raw) = T::try_from(c) else {
                panic!("{c} doesn't fit")
            };
            let Some(m) = N::decode(raw) else {
                panic!("{c} doesn't decode")
            };
            assert_eq!(m.encode(), Some(raw), "{c}");
        }
        if let Ok(raw) = T::try_from(lo - 1)
            && !N::CODES.iter().any(|&(l, h)| (l..=h).contains(&(lo - 1)))
        {
            assert!(N::decode(raw).is_none(), "{} decodes", lo - 1);
        }
    }
}

/// Every shipped set: its codes decode, encode back and are outside the bounds it documents.
pub(crate) fn shipped_sets() {
    codes_are_outside::<u8, J1939U8>(250);
    codes_are_outside::<u16, J1939U16>(0xFAFF);
    codes_are_outside::<u32, J1939U32>(0xFAFF_FFFF);
}

/// A 12-bit ADC's rails: 0 at most, 4095 at least, the rest exact, and each mark encodes back to its raw number.
pub(crate) fn adc_rails(raw: u16) {
    type Adc = Clipped<Fixed<Ratio, One, u16, Range<0, 4095>>, BothRails>;
    match (raw, Adc::try_from_raw(raw)) {
        (0, Ok(a)) => assert!(matches!(a.get(), Clip::AtMost(_))),
        (4095, Ok(a)) => assert!(matches!(a.get(), Clip::AtLeast(_))),
        (1..=4094, Ok(a)) => assert!(matches!(a.get(), Clip::Exact(_))),
        (4096.., Err(_)) => return,
        (_, r) => panic!("{raw}: {r:?}"),
    }
    let Ok(a) = Adc::try_from_raw(raw) else {
        return;
    };
    assert_eq!(Adc::from_clip(a.get()).map(Adc::raw), Ok(raw));
}

#[cfg(not(any(kani, fuzzing)))]
mod tests {
    use proptest::prelude::*;

    #[test]
    fn every_8_and_16_bit_raw_number() {
        for raw in 0..=u8::MAX {
            super::j1939_byte(raw);
        }
        for raw in i8::MIN..=i8::MAX {
            super::rssi(raw);
        }
        for raw in 0..=u16::MAX {
            super::j1939_word(raw);
            super::adc_rails(raw);
        }
        for raw in i16::MIN..=i16::MAX {
            super::int16(raw);
        }
        super::shipped_sets();
    }

    /// 4-byte numbers weighted to the edges of J1939's classes.
    fn dword() -> impl Strategy<Value = u32> {
        prop_oneof![
            2 => any::<u32>(),
            1 => (0xF9u32..=0xFF, 0u32..4).prop_map(|(top, d)| {
                let edge = top << 24;
                [edge, edge | 1, edge | 0x00FF_FFFF, edge | 0x00FF_FFFE][d as usize]
            }),
        ]
    }

    proptest! {
        #[test]
        fn j1939_dword(raw in dword()) {
            super::j1939_dword(raw);
        }
    }
}

#[cfg(kani)]
mod proofs {
    /// Every u8 as a J1939 1-byte parameter: a value or a code of its class, encoding back to itself.
    #[kani::proof]
    fn j1939_byte() {
        super::j1939_byte(kani::any());
    }

    /// Every i8 as an RSSI: a value, the code, or refused.
    #[kani::proof]
    fn rssi() {
        super::rssi(kani::any());
    }
}
