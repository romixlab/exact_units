use exact_units::systems::Gaussian;
use exact_units::systems::gaussian::{Erg, GaussianCentimeter, GaussianSecond};
use exact_units::units::{MagneticFieldStrength, Meter, Per, Times, Volt};
use exact_units::{Fixed, prefix::One};

fn main() {
    // SI's E×H, unnamed: a product of units that cross only by rows
    let p = Fixed::<Per<Volt, Meter>, One, i64>::new(1) * Fixed::<MagneticFieldStrength, One, i64>::new(1);
    type Flux = Per<Erg, Times<GaussianSecond, Times<GaussianCentimeter, GaussianCentimeter>>>;
    let _ = p.to_system::<Gaussian>().convert::<Flux, One, i64>();
}
