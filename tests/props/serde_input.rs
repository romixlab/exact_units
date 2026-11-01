//! Property 10: serde (feature `serde`, RON snapshots of APIs) of any text at all. A `Magnitude`, `Offset`, `DimOwned`
//! or `ProvenanceOwned` read from JSON is an error or a value that writes and reads back the same. A magnitude and an
//! offset go on the wire as they are (serde rebuilds them canonical); a dimension does where it is canonical (serde
//! takes its fields as written), a provenance unless it is `Unknown` with the tag of a known kind. Never a panic.
//! Strings, so no Kani proof; `fuzz/` runs the same function.
#![expect(clippy::panic, reason = "a property fails by panicking")]

use super::wire::round_trip;
use core::fmt::Debug;
use exact_units::{DimOwned, Magnitude, Offset, ProvenanceOwned};
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
        // serde takes any fields (unlike Magnitude's): a dimension that isn't canonical, or has an exponent past 4
        // bits, reads but doesn't go on the wire; a canonical one does, as it is
        let canonical = d.as_dim().is_canonical();
        match d.to_ww_bytes_owned() {
            Ok(bytes) => {
                let back = DimOwned::from_ww_bytes_owned(&bytes).ok();
                assert_eq!(back.is_some(), canonical, "{s:?}: {d:?}");
                if let Some(back) = back {
                    assert_eq!(back, d, "{s:?}");
                }
            }
            Err(_) => assert!(
                d.as_dim()
                    .exps()
                    .into_iter()
                    .any(|e| !(-8..=7).contains(&e))
                    || d.counted
                        .iter()
                        .flatten()
                        .any(|c| !(-8..=7).contains(&c.exp))
                    || !canonical,
                "{s:?}: {d:?} doesn't encode"
            ),
        }
        let _ = d.as_dim().to_string();
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

#[cfg(not(any(kani, fuzzing)))]
mod tests {
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
            r#"{"Conventional":{"source":"NIST SP 811","year":2008}}"#,
        ] {
            super::any_json(s);
        }
    }

    proptest! {
        #[test]
        fn json_text(s in json()) {
            super::any_json(&s);
        }
    }
}
