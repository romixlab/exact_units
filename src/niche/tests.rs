use super::{
    BothRails, Clip, Clipped, HighRail, J1939U8, J1939U16, J1939U32, Niched, Null, Reading,
};
use crate::prefix::{One, Pow2, Rational};
use crate::units::{Kelvin, Ratio, Volt};
use crate::{Fixed, Offset, OutOfBounds, Point, Range, Saturating, origin, origins::Celsius};
use core::mem::size_of;

type Pct = Fixed<Ratio, One, u8, Range<0, 250>>;
type Load = Niched<Pct, J1939U8>;

#[test]
fn j1939_byte_every_code() {
    for raw in 0..=u8::MAX {
        let n = Load::try_from_raw(raw).unwrap();
        let want = match raw {
            0..=250 => Reading::Value(Pct::bounded(raw)),
            251..=253 => Reading::Niche(J1939U8::Reserved(raw)),
            0xFE => Reading::Niche(J1939U8::Error),
            0xFF => Reading::Niche(J1939U8::NotAvailable),
        };
        assert_eq!(n.get(), want, "{raw}");
        assert_eq!(n.raw(), raw);
        assert_eq!(Load::from_reading(n.get()).map(Load::raw), Ok(raw));
    }
    assert_eq!(size_of::<Load>(), 1);
}

#[test]
fn j1939_word_and_dword_edges() {
    type S = Fixed<Ratio, Rational<1, 256>, u16, Range<0, 0xFAFF>>;
    type Speed = Niched<S, J1939U16>;
    let get = |raw| Speed::try_from_raw(raw).map(Speed::get);
    assert_eq!(get(0xFAFF), Ok(Reading::Value(S::bounded(0xFAFF))));
    assert_eq!(get(0xFB00), Ok(Reading::Niche(J1939U16::Reserved(0xFB00))));
    assert_eq!(get(0xFDFF), Ok(Reading::Niche(J1939U16::Reserved(0xFDFF))));
    assert_eq!(get(0xFE00), Ok(Reading::Niche(J1939U16::Error(0xFE00))));
    assert_eq!(get(0xFEFF), Ok(Reading::Niche(J1939U16::Error(0xFEFF))));
    assert_eq!(
        get(0xFF00),
        Ok(Reading::Niche(J1939U16::NotAvailable(0xFF00)))
    );
    assert_eq!(
        get(0xFFFF),
        Ok(Reading::Niche(J1939U16::NotAvailable(0xFFFF)))
    );
    // a range meaning holding a code outside its range has no code
    assert_eq!(Speed::niche(J1939U16::Error(0x1234)), Err(OutOfBounds));
    assert_eq!(
        Speed::niche(J1939U16::Error(0xFE42)).map(Speed::raw),
        Ok(0xFE42)
    );
    assert_eq!(size_of::<Speed>(), 2);

    type D = Fixed<Ratio, One, u32, Range<0, 0xFAFF_FFFF>>;
    type Distance = Niched<D, J1939U32>;
    let get = |raw| Distance::try_from_raw(raw).map(Distance::get);
    assert_eq!(
        get(0xFAFF_FFFF),
        Ok(Reading::Value(D::bounded(0xFAFF_FFFF)))
    );
    assert_eq!(
        get(0xFB00_0000),
        Ok(Reading::Niche(J1939U32::Reserved(0xFB00_0000)))
    );
    assert_eq!(
        get(0xFEFF_FFFF),
        Ok(Reading::Niche(J1939U32::Error(0xFEFF_FFFF)))
    );
    assert_eq!(
        get(u32::MAX),
        Ok(Reading::Niche(J1939U32::NotAvailable(u32::MAX)))
    );
    assert_eq!(size_of::<Distance>(), 4);
}

crate::niches! {
    /// SunSpec's `int16`: 0x8000 is a point the device doesn't implement.
    pub(crate) enum SunSpecI16: i16 {
        /// Not implemented.
        NotImplemented = -32768,
    }
}

crate::niches! {
    /// The RSSI of a BLE HCI event: 127 is not available.
    pub(crate) enum HciRssi: i8 {
        /// Not available.
        NotAvailable = 127,
    }
}

#[test]
fn sunspec_and_rssi() {
    type W = Fixed<crate::units::Watt, One, i16, Range<-32767, 32767>>;
    type Watts = Niched<W, SunSpecI16>;
    assert_eq!(
        Watts::try_from_raw(i16::MIN).map(Watts::get),
        Ok(Reading::Niche(SunSpecI16::NotImplemented))
    );
    assert_eq!(
        Watts::try_from_raw(-32767).unwrap().value(),
        Some(W::bounded(-32767))
    );
    assert_eq!(
        Watts::niche(SunSpecI16::NotImplemented).map(Watts::raw),
        Ok(i16::MIN)
    );

    type R = Fixed<Ratio, One, i8, Range<-127, 20>>;
    type Rssi = Niched<R, HciRssi>;
    assert_eq!(
        Rssi::try_from_raw(127).unwrap().niche_meaning(),
        Some(HciRssi::NotAvailable)
    );
    assert_eq!(
        Rssi::try_from_raw(-60).unwrap().value(),
        Some(R::bounded(-60))
    );
    // neither a value of -127..=20 nor the code
    assert_eq!(Rssi::try_from_raw(50), Err(OutOfBounds));
    assert_eq!(Rssi::try_from_raw(-128), Err(OutOfBounds));
}

#[test]
fn matter_null_is_an_option() {
    type L = Fixed<Ratio, One, u8, Range<0, 254>>;
    type Level = Niched<L, Null<0xFF>>;
    assert_eq!(Level::null().raw(), 0xFF);
    assert_eq!(Option::from(Level::null()), None::<L>);
    assert_eq!(Level::from(Some(L::bounded(7))).raw(), 7);
    assert_eq!(Level::from_option(None).get(), Reading::Niche(Null));
    assert_eq!(size_of::<Level>(), 1);

    type T = Fixed<Ratio, One, i8, Range<-127, 127>>;
    type Temp = Niched<T, Null<-128>>;
    assert_eq!(Temp::null().raw(), -128);
    assert_eq!(Temp::try_from_raw(-128).unwrap().value(), None);
    assert_eq!(
        Temp::try_from_raw(-127).unwrap().value(),
        Some(T::bounded(-127))
    );

    // the top of a u64 needs the i128 code
    type Big = Niched<
        Fixed<Ratio, One, u64, crate::RangeU64<0, { u64::MAX - 1 }>>,
        Null<0xFFFF_FFFF_FFFF_FFFF>,
    >;
    assert_eq!(Big::null().raw(), u64::MAX);
    assert_eq!(
        Big::try_from_raw(u64::MAX - 1)
            .unwrap()
            .value()
            .map(Fixed::get),
        Some(u64::MAX - 1)
    );
}

#[test]
fn display_and_debug() {
    let v = Load::try_from_raw(80).unwrap();
    assert_eq!(format!("{v}"), "80");
    assert_eq!(
        format!("{:?}", Load::try_from_raw(0xFF).unwrap()),
        "NotAvailable"
    );
    assert_eq!(
        format!("{}", Load::try_from_raw(252).unwrap()),
        "Reserved(252)"
    );
}

#[test]
fn clipped_adc_and_q15() {
    type A = Fixed<Volt, Rational<33, 40950>, u16, Range<0, 4095>>;
    type Adc = Clipped<A>;
    assert_eq!(
        Adc::try_from_raw(1).map(Adc::get),
        Ok(Clip::Exact(A::bounded(1)))
    );
    assert_eq!(
        Adc::try_from_raw(4094).map(Adc::get),
        Ok(Clip::Exact(A::bounded(4094)))
    );
    assert_eq!(
        Adc::try_from_raw(4095).map(Adc::get),
        Ok(Clip::AtLeast(A::bounded(4095)))
    );
    assert_eq!(
        Adc::try_from_raw(0).map(Adc::get),
        Ok(Clip::AtMost(A::bounded(0)))
    );
    assert_eq!(Adc::try_from_raw(4096), Err(OutOfBounds));
    assert!(Adc::try_from_raw(4095).unwrap().get().is_clipped());
    assert_eq!(size_of::<Adc>(), 2);
    // a rail mark encodes only at its rail
    assert_eq!(
        Adc::from_clip(Clip::AtLeast(A::bounded(4095))).map(Adc::raw),
        Ok(4095)
    );
    assert_eq!(
        Adc::from_clip(Clip::AtLeast(A::bounded(4000))),
        Err(OutOfBounds)
    );
    assert_eq!(
        Adc::from_clip(Clip::AtMost(A::bounded(4095))),
        Err(OutOfBounds)
    );
    assert_eq!(
        Adc::from_clip(Clip::Exact(A::bounded(4095))).map(Adc::raw),
        Ok(4095)
    );

    // Q15 after saturation: the ends of i16
    type Q = Clipped<Fixed<Ratio, Pow2<-15>, i16, Saturating>>;
    let q = |x| Q::new(Fixed::of(x)).get();
    assert_eq!(q(i16::MAX), Clip::AtLeast(Fixed::of(i16::MAX)));
    assert_eq!(q(i16::MIN), Clip::AtMost(Fixed::of(i16::MIN)));
    assert_eq!(q(0), Clip::Exact(Fixed::of(0)));
    assert_eq!(format!("{}", Q::new(Fixed::of(i16::MAX))), "≥ 32767·2⁻¹⁵");

    // only the high end a rail
    type H = Fixed<Ratio, One, u8, Range<0, 100>>;
    type Top = Clipped<H, HighRail>;
    assert_eq!(
        Top::try_from_raw(0).map(Top::get),
        Ok(Clip::Exact(H::bounded(0)))
    );
    assert_eq!(
        Top::try_from_raw(100).map(Top::get),
        Ok(Clip::AtLeast(H::bounded(100)))
    );
}

origin! {
    /// J1939 engine coolant temperature (SPN 110): 1 °C per bit, −40 °C offset.
    pub(crate) struct J1939Temp { above: Celsius, offset: Offset::int(-40) }
}

#[test]
fn j1939_coolant_niches_and_rails() {
    type Raw = Point<J1939Temp, Fixed<Kelvin, One, u8, Range<0, 250>>>;
    type Coolant = Niched<Clipped<Raw, BothRails>, J1939U8>;
    let read = |raw| Coolant::try_from_raw(raw).unwrap().get();
    let celsius = |c: Clipped<Raw, BothRails>| c.value().to::<Celsius, One, i16>().map(Point::raw);
    let Reading::Value(c) = read(130) else {
        panic!()
    };
    assert_eq!(celsius(c), Some(90));
    assert!(!c.get().is_clipped());
    let Reading::Value(c) = read(0) else { panic!() };
    assert_eq!(celsius(c), Some(-40));
    assert!(matches!(c.get(), Clip::AtMost(_)));
    assert_eq!(read(0xFE), Reading::Niche(J1939U8::Error));
    assert_eq!(read(0xFF), Reading::Niche(J1939U8::NotAvailable));
    assert_eq!(size_of::<Coolant>(), 1);
}

#[cfg(feature = "shrink_wrap")]
#[test]
fn wire_bytes_are_the_storage() {
    use shrink_wrap::{DeserializeShrinkWrap, SerializeShrinkWrap};

    type L = Fixed<Ratio, One, u8, Range<0, 100>>;
    type Level = Niched<L, Null<0xFF>>;
    let mut buf = [0u8; 8];
    let bytes = Level::null().to_ww_bytes(&mut buf).unwrap();
    assert_eq!(bytes, [0xFF]);
    let bytes = Level::from(Some(L::bounded(42)))
        .to_ww_bytes(&mut buf)
        .unwrap();
    assert_eq!(bytes, [42]);
    assert_eq!(Level::from_ww_bytes(&[0xFF]).unwrap(), Level::null());
    assert_eq!(
        Level::from_ww_bytes(&[42]).unwrap().value().map(Fixed::get),
        Some(42)
    );
    // neither a value of 0..=100 nor the code
    assert!(Level::from_ww_bytes(&[150]).is_err());

    type Adc = Clipped<Fixed<Volt, Rational<33, 40950>, u16, Range<0, 4095>>>;
    let bytes = Adc::try_from_raw(4095)
        .unwrap()
        .to_ww_bytes(&mut buf)
        .unwrap()
        .to_vec();
    assert_eq!(bytes, 4095u16.to_ww_bytes(&mut [0u8; 8]).unwrap());
    assert!(Adc::from_ww_bytes(&4096u16.to_le_bytes()).is_err());
}
