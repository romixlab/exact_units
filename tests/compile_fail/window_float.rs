// A window from a tolerance, and a tolerance between two stored values, are integer arithmetic: on a float storage
// they don't compile.
use exact_units::{Fit, Fixed, MaybeNan, Tolerance, prefix::Milli, units::Ratio};

type F = Fixed<Ratio, Milli, f32, MaybeNan>;

fn main() {
    let _ = F::of(1.0).window(Tolerance::percent(1), Fit::Inside);
    let _ = F::of(1.0).is_within(F::of(1.0), Tolerance::percent(1));
}
