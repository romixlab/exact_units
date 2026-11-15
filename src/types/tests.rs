//! The tables of [types](super) and [origins::ALL](crate::origins::ALL) against this crate's source: every alias,
//! wrapper and origin it declares is listed, with the parameters its declaration has.

use super::{ALIASES, AliasParam, UNIT_FORMS, WRAPPERS};
use std::string::{String, ToString};
use std::vec::Vec;

/// The files that declare the aliases of `Fixed` and `Complex`.
const ALIAS_SOURCES: [&str; 3] = [
    include_str!("../fixed.rs"),
    include_str!("../complex.rs"),
    include_str!("../complex/iq.rs"),
];

/// A `pub type` declaration: its name, its parameters (name, has a default) and the type it names.
struct Decl {
    name: String,
    params: Vec<(String, bool)>,
    of: String,
}

/// Every `pub type Name<..> = Of<..>;` of `src`, a declaration on one line or two.
fn decls(src: &str) -> Vec<Decl> {
    let joined = src.replace("=\n    ", "= ");
    joined
        .lines()
        .filter_map(|line| line.strip_prefix("pub type "))
        .map(|rest| {
            // a parameter's default has a ` = ` too: the declaration's is the one after the parameters
            let generic = rest.find('<') < rest.find(" = ");
            let (head, of) = if generic {
                rest.split_once("> = ").expect("a generic alias has `> = `")
            } else {
                rest.split_once(" = ").expect("a type alias has ` = `")
            };
            let (name, params) = head.split_once('<').unwrap_or((head, ""));
            let params = params
                .split(", ")
                .filter(|p| !p.is_empty())
                .map(|p| match p.split_once(" = ") {
                    Some((p, _)) => (p.to_string(), true),
                    None => (p.to_string(), false),
                })
                .collect();
            Decl {
                name: name.to_string(),
                params,
                of: of.split('<').next().unwrap_or(of).to_string(),
            }
        })
        .collect()
}

#[cfg(feature = "quantities")]
#[test]
fn every_alias_is_listed_with_its_parameters() {
    let decls: Vec<Decl> = ALIAS_SOURCES
        .iter()
        .flat_map(|src| decls(src))
        .filter(|d| d.of == "Fixed" || d.of == "Complex")
        .collect();
    assert!(decls.len() > 20, "the scan finds the aliases");
    for d in &decls {
        let def = ALIASES
            .iter()
            .find(|a| a.name == d.name)
            .unwrap_or_else(|| panic!("{} is missing from types::ALIASES", d.name));
        assert_eq!(def.wrapper, d.of, "{}", d.name);
        assert!(WRAPPERS.contains(&def.wrapper), "{}", d.name);
        let kinds: Vec<AliasParam> = d
            .params
            .iter()
            .map(|(p, _)| match p.as_str() {
                "U" | "Of" => AliasParam::U,
                "P" => AliasParam::P,
                "T" => AliasParam::T,
                "B" => AliasParam::B,
                other => panic!("{}: a parameter {other}", d.name),
            })
            .collect();
        assert_eq!(def.params, kinds.as_slice(), "{}", d.name);
        for ((_, default), kind) in d.params.iter().zip(&kinds) {
            let given = match kind {
                AliasParam::U => def.unit.is_some(),
                AliasParam::P => def.scale.is_some(),
                AliasParam::T => def.storage.is_some(),
                AliasParam::B => def.bounds.is_some(),
            };
            assert_eq!(
                given, *default,
                "{} {kind:?}: a default only where declared",
                d.name
            );
        }
    }
    assert_eq!(
        ALIASES.len(),
        decls.len(),
        "no alias listed twice or made up"
    );
}

#[cfg(feature = "quantities")]
#[test]
fn alias_parts_are_the_types() {
    let q15 = ALIASES.iter().find(|a| a.name == "Q15").expect("Q15");
    assert_eq!(
        q15.unit.map(|u| u.name().to_string()).as_deref(),
        Some("Ratio")
    );
    assert_eq!(q15.scale.map(|s| s.name), Some("Pow2"));
    assert_eq!(q15.storage, Some("i16"));
    assert_eq!(q15.bounds, Some("Saturating"));
    let bam = ALIASES.iter().find(|a| a.name == "Bam16").expect("Bam16");
    assert_eq!(bam.bounds, Some("Wrapping"));
    let count = ALIASES.iter().find(|a| a.name == "Count").expect("Count");
    assert_eq!((count.unit, count.storage), (None, None));
    assert_eq!(count.scale.map(|s| s.name), Some("One"));
    assert_eq!(count.bounds, Some("Unbounded"));
}

#[cfg(feature = "quantities")]
#[test]
fn wrappers_and_unit_forms_are_types_of_the_crate() {
    let src = [
        include_str!("../fixed.rs"),
        include_str!("../complex.rs"),
        include_str!("../complex/polar.rs"),
        include_str!("../point.rs"),
        include_str!("../exact.rs"),
        include_str!("../scaled.rs"),
        include_str!("../index.rs"),
        include_str!("../units.rs"),
    ]
    .concat();
    for name in WRAPPERS.iter().chain(UNIT_FORMS) {
        let declared = src.contains(&std::format!("pub struct {name}<"))
            || src.contains(&std::format!("pub type {name}<"));
        assert!(declared, "{name} is no generic type of the crate");
    }
}

#[cfg(feature = "quantities")]
#[test]
fn every_origin_is_listed() {
    let src = include_str!("../point/origins.rs");
    let declared: Vec<&str> = src
        .lines()
        .map(str::trim)
        .filter_map(|l| l.strip_prefix("pub struct "))
        .filter(|l| l.ends_with('{') || l.contains(" { unit:") || l.contains(" { above:"))
        .filter_map(|l| l.split([' ', '{']).next())
        .collect();
    assert!(declared.len() >= 13, "the scan finds the origins");
    let listed: Vec<&str> = crate::origins::ALL.iter().map(|o| o.name).collect();
    for name in &declared {
        assert!(listed.contains(name), "{name} is missing from origins::ALL");
    }
    let angles = listed.iter().filter(|n| **n == "Angle").count();
    assert_eq!(angles, 3, "Angle in turns, degrees and radians");
    assert_eq!(listed.len(), declared.len() + angles, "nothing made up");
}
