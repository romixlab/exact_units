// A torque (N·m, kg·m²·s⁻²·rad⁻¹) is not a joule (J, kg·m²·s⁻²): they differ by an angle (P2401#5nov-0954).
use exact_units::{Fixed, prefix::One, units::{Joule, Torque}};

fn main() {
    let _ = Fixed::<Torque, One, i32>::new(5).convert::<Joule, One, i32>();
}
