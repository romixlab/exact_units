// `q!` ranges and tolerances the macro refuses: an inverted or empty range, a tolerance that reads two ways or is
// of nothing, units that differ, a fit without a range.
use exact_units::q;

fn main() {
    let _ = q!(5.1 V ..= 4.9 V);
    let _ = q!(5 V .. 5 V);
    let _ = q!(..= 5 V);
    let _ = q!(5 % +- 1 %);
    let _ = q!(25 degC +- 1 %);
    let _ = q!(0 V +- 1 %);
    let _ = q!(1 in +- 0.1 mm);
    let _ = q!(5 V +- -1 %);
    let _ = q!(5 V + 1 %);
    let _ = q!(5 V, Inside);
    let _ = q!(4.9 V ..= 5.1 V, Middle);
    let _ = q!(20 degC ..= 25 K);
    let _ = q!(9... V);
    let _ = q!(...5 V);
    let _ = q!(9.5e3... V);
    let _ = q!(9.5... V, Inside);
    let _ = q!(-0.0... V);
    let _ = q!(4.9 ... 5.1 V);
    let _ = q!(9.5... V ..= 10 V);
    let _ = q!("9.5... V ± 1 %");
}
