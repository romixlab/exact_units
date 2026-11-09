//! This crate's systems of units: [Si], and with the `systems` feature [Natural] (ħ = c = 1) and [Gaussian] (cm, g,
//! s, its electric quantities ESU's and its magnetic ones EMU's). [system](mod@crate::system) has the rules.

use crate::system::def_list;
use crate::{MarkerId, System, SystemDef};

#[cfg(feature = "systems")]
pub mod gaussian;
#[cfg(feature = "systems")]
pub mod natural;

#[cfg(feature = "systems")]
pub use gaussian::Gaussian;
#[cfg(feature = "systems")]
pub use natural::Natural;

/// The International System of Units: s, m, kg, A, K, mol, cd. Every unit of [units](crate::units) is one of SI's.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct Si;

impl System for Si {
    const ID: MarkerId<'static> = MarkerId {
        crate_name: "exact_units",
        name: "Si",
    };
    const BASES: &'static [&'static str] = &["s", "m", "kg", "A", "K", "mol", "cd"];
    const DEF: &'static SystemDef<'static> = &SystemDef {
        id: Self::ID,
        bases: def_list(Self::BASES),
        set_to_one: def_list(&[]),
        by_dim: def_list(&[]),
        by_quantity: def_list(&[]),
        no_rule: def_list(&[]),
    };
}

/// The definition of this crate's system `id`, `None` for another.
#[must_use]
pub const fn builtin(id: MarkerId<'_>) -> Option<&'static SystemDef<'static>> {
    use crate::dim::walk::same_str;
    if !same_str(id.crate_name, "exact_units") {
        return None;
    }
    if same_str(id.name, "Si") {
        return Some(Si::DEF);
    }
    #[cfg(feature = "systems")]
    {
        if same_str(id.name, "Natural") {
            return Some(Natural::DEF);
        }
        if same_str(id.name, "Gaussian") {
            return Some(Gaussian::DEF);
        }
    }
    None
}
