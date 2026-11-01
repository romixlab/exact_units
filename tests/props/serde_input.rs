//! Property 10: serde (feature `serde`, RON snapshots of APIs) of any text at all. A `Magnitude`, `Offset`, `DimOwned`
//! or `ProvenanceOwned` read from JSON is an error or a value that writes and reads back the same. A magnitude and an
//! offset go on the wire as they are (serde rebuilds them canonical), a dimension too (serde refuses one that isn't
//! canonical, as the wire does), a provenance unless it is `Unknown` with the tag of a known kind. Never a panic.
//! Strings, so no Kani proof; `fuzz/` runs the same function.
#![expect(clippy::panic, reason = "a property fails by panicking")]

use super::wire::round_trip;
use core::fmt::Debug;
use exact_units::{Dim, DimOwned, Magnitude, Offset, ProvenanceOwned};
use serde::Serialize;
use serde::de::DeserializeOwned;
use shrink_wrap::prelude::*;

/// `s` read as `T`: an error, or a value that writes to JSON and reads back the same.
#[expect(clippy::expect_used, reason = "a property fails by panicking")]
fn read<T: DeserializeOwned + Serialize + PartialEq + Debug>(s: &str) -> Option<T> {
    let x = serde_json::from_str::<T>(s).ok()?;
    let again = serde_json::to_string(&x).expect("writes");
    assert_eq!(
        serde_json::from_str::<T>(&again).ok().as_ref(),
        Some(&x),
        "{s:?} wrote {again}"
    );
    Some(x)
}

/// JSON text as each serde type: an error, or a value that round-trips through JSON and through the wire.
pub(crate) fn any_json(s: &str) {
    if let Some(m) = read::<Magnitude>(s) {
        round_trip(&m);
        let _ = (m.to_string(), m.to_f64());
    }
    if let Some(o) = read::<Offset>(s) {
        round_trip(&o);
        let _ = (o.to_string(), o.to_f64(), o.to_ratio());
    }
    if let Some(d) = read::<DimOwned>(s) {
        // serde keeps only a canonical dimension, as the wire reader does: every one read goes on the wire
        assert!(
            d.as_dim().is_canonical(),
            "{s:?}: {d:?} reads, not canonical"
        );
        dim(&d);
    }
    if let Some(p) = read::<ProvenanceOwned>(s) {
        // `Unknown(tag)`, a kind from a newer version, goes on the wire with a tag no kind of this version has
        match (p.to_ww_bytes_owned(), &p) {
            (Ok(bytes), _) => {
                assert_eq!(ProvenanceOwned::from_ww_bytes_owned(&bytes).ok(), Some(p));
            }
            (Err(_), ProvenanceOwned::Unknown(tag)) => {
                assert!(*tag < 2, "{s:?}: {p:?} doesn't encode");
            }
            (Err(e), _) => panic!("{s:?}: {p:?} doesn't encode: {e:?}"),
        }
    }
}

/// A canonical dimension round-trips: JSON to the same value and the same text, the wire (owned and borrowed) to the
/// same value, and what the wire gives back writes the same bytes and the same JSON.
#[expect(clippy::expect_used, reason = "a property fails by panicking")]
pub(crate) fn dim(d: &DimOwned) {
    let json = serde_json::to_string(d).expect("writes");
    let back: DimOwned = serde_json::from_str(&json).expect("reads");
    assert_eq!(&back, d, "{json}");
    let bytes = d.to_ww_bytes_owned().expect("encodes");
    let owned = DimOwned::from_ww_bytes_owned(&bytes).expect("decodes");
    assert_eq!(&owned, d);
    assert_eq!(Dim::from_ww_bytes(&bytes).ok(), Some(d.as_dim()));
    assert_eq!(owned.to_ww_bytes_owned().expect("encodes"), bytes);
    assert_eq!(serde_json::to_string(&owned).expect("writes"), json);
}

#[cfg(not(any(kani, fuzzing)))]
mod tests {
    use exact_units::Dim;
    use proptest::prelude::*;

    /// JSON shaped like the types: their field names, small and edge numbers, lists of pairs.
    fn json() -> impl Strategy<Value = String> {
        let num = prop_oneof![
            "-?[0-9]{1,3}",
            Just("127".to_string()),
            Just("-128".to_string()),
            Just("4294967291".into())
        ];
        let field = prop_oneof![
            num.clone().prop_map(|n| format!("\"pow10\":{n}")),
            num.clone().prop_map(|n| format!("\"pow2\":{n}")),
            num.clone().prop_map(|n| format!("\"pi\":{n}")),
            num.clone().prop_map(|n| format!("\"ln10\":{n}")),
            proptest::collection::vec((num.clone(), num.clone()), 0..6)
                .prop_map(|v| format!("\"primes\":[{}]", v.iter().map(|(p, e)| format!("[{p},{e}]")).collect::<Vec<_>>().join(","))),
            any::<bool>().prop_map(|b| format!("\"negative\":{b}")),
            num.clone().prop_map(|n| format!("\"magnitude\":{{\"pow10\":{n}}}")),
            num.clone().prop_map(|n| format!("\"s\":{n}")),
            num.clone().prop_map(|n| format!("\"m\":{n}")),
            any::<bool>().prop_map(|b| format!("\"root\":{b}")),
            num.prop_map(|n| format!("\"counted\":[{{\"crate_name\":\"exact_units\",\"name\":\"Sample\",\"exp\":{n}}},null,null,null]")),
        ];
        prop_oneof![
            4 => proptest::collection::vec(field, 0..8).prop_map(|f| format!("{{{}}}", f.join(","))),
            1 => any::<String>(),
        ]
    }

    #[test]
    fn edges() {
        for s in [
            "{}",
            "null",
            r#"{"pow10":0,"primes":[[3,0]]}"#,
            r#"{"pow10":0,"primes":[[4,2]]}"#,
            r#"{"pow10":127,"pow2":-128}"#,
            r#"{"negative":true}"#,
            r#"{"negative":true,"magnitude":{"pow10":0}}"#,
            r#""Defined""#,
            r#"{"Unknown":0}"#,
            r#"{"Unknown":1}"#,
            r#"{"Unknown":2}"#,
            r#"{"s":0,"m":0,"kg":0,"a":0,"k":0,"mol":0,"cd":0,"rad":0,"bit":0,"root":false,"counted":[null,null,null,null]}"#,
            r#"{"s":9,"m":0,"kg":0,"a":0,"k":0,"mol":0,"cd":0,"rad":0,"bit":0,"root":false,"counted":[null,null,null,null]}"#,
            r#"{"s":0,"m":0,"kg":0,"a":0,"k":0,"mol":0,"cd":0,"rad":0,"bit":0,"root":true,"counted":[null,null,null,null]}"#,
            r#"{"s":0,"m":0,"kg":0,"a":0,"k":0,"mol":0,"cd":0,"rad":0,"bit":0,"root":false,"counted":[{"crate_name":"exact_units","name":"Sample","exp":1},{"crate_name":"exact_units","name":"Frame","exp":-1},null,null]}"#,
            r#"{"s":0,"m":0,"kg":0,"a":0,"k":0,"mol":0,"cd":0,"rad":0,"bit":0,"root":false,"counted":[null,{"crate_name":"exact_units","name":"Sample","exp":1},null,null]}"#,
            r#"{"Conventional":{"source":"NIST SP 811","year":2008}}"#,
        ] {
            super::any_json(s);
        }
    }

    /// A canonical dimension built the way markers build one: SI exponents, counted bases multiplied in (several
    /// crates and names, so the order matters), maybe a square root.
    fn dim() -> impl Strategy<Value = Dim<'static>> {
        const BASES: [(&str, &str); 5] = [
            ("exact_units", "Sample"),
            ("exact_units", "Frame"),
            ("my_api", "Bucket"),
            ("a", "Sample"),
            ("my_api", "Frame"),
        ];
        let e = -8i8..=7;
        (
            [
                e.clone(),
                e.clone(),
                e.clone(),
                e.clone(),
                e.clone(),
                e.clone(),
                e,
            ],
            (-8i8..=7, -8i8..=7),
            proptest::collection::vec((proptest::sample::select(&BASES[..]), any::<bool>()), 0..8),
            any::<bool>(),
        )
            .prop_map(|([s, m, kg, a, k, mol, cd], (rad, bit), counted, root)| {
                let mut d = Dim::new(s, m, kg, a, k, mol, cd)
                    .with_rad(rad)
                    .with_bit(bit);
                for ((c, n), up) in counted {
                    let b = Dim::counted(c, n);
                    d = if up {
                        d.checked_mul(b)
                    } else {
                        d.checked_div(b)
                    }
                    .unwrap_or(d);
                }
                if root {
                    d = d.checked_sqrt().unwrap_or(d);
                }
                d
            })
    }

    proptest! {
        #[test]
        fn json_text(s in json()) {
            super::any_json(&s);
        }

        #[test]
        fn dim_round_trips(d in dim()) {
            prop_assert!(d.is_canonical(), "{d:?}");
            super::dim(&d.into());
        }
    }
}
