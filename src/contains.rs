//! [Contains], a set of values a value is in or not, and `is_in` on the values: `v.is_in(q!(4.9 V ..= 5.1 V))`.

use crate::fixed::Storage;
use crate::{Bounds, Exact, Fixed, Point};

/// A set of values of `V` that a value is in or not: a `q!` range, a window. Which values are in is the set's own
/// rule; none of them holds NaN.
pub trait Contains<V> {
    /// Whether `v` is in the set.
    fn contains(&self, v: &V) -> bool;
}

impl<U, P, T: Storage, B: Bounds<T>> Fixed<U, P, T, B> {
    /// Whether this value is in `set`: `set.contains(&self)`, in the order a check reads.
    #[inline]
    #[must_use]
    #[expect(
        clippy::needless_pass_by_value,
        reason = "a set is a Copy literal or window, taken by value so a check reads v.is_in(q!(..))"
    )]
    pub fn is_in<R: Contains<Self>>(&self, set: R) -> bool {
        set.contains(self)
    }
}

impl<U> Exact<U> {
    /// Whether this value is in `set`: `set.contains(&self)`, in the order a check reads.
    #[inline]
    #[must_use]
    #[expect(
        clippy::needless_pass_by_value,
        reason = "a set is a Copy literal or window, taken by value so a check reads v.is_in(q!(..))"
    )]
    pub fn is_in<R: Contains<Self>>(&self, set: R) -> bool {
        set.contains(self)
    }
}

impl<O, V> Point<O, V> {
    /// Whether this point is in `set`: `set.contains(&self)`, in the order a check reads.
    #[inline]
    #[must_use]
    #[expect(
        clippy::needless_pass_by_value,
        reason = "a set is a Copy literal or window, taken by value so a check reads v.is_in(q!(..))"
    )]
    pub fn is_in<R: Contains<Self>>(&self, set: R) -> bool {
        set.contains(self)
    }
}
