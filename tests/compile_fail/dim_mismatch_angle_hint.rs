// Hertz (s⁻¹) is not an angular velocity (rad·s⁻¹): they differ by angle the other way round from
// dim_mismatch.rs's torque/joule case, so the hint's operator flips too (P2401#5nov-0954).
use exact_units::{Fixed, prefix::One, units::{AngularVelocity, Hertz}};

fn main() {
    let _ = Fixed::<Hertz, One, i32>::new(5).convert::<AngularVelocity, One, i32>();
}
