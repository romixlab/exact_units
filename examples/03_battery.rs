//! 3. A battery monitor: readings that carry their valid range, and arithmetic that says what happens at the edge.
//!
//! A cell monitor reports each Li-ion cell in mV as a `u16`. Two bugs live in that `u16`. A broken sense wire reads
//! 0 or 65535, and nothing stops 65535 mV from flowing into the state of charge. And the headroom above the cutoff,
//! `cell - cutoff`, underflows when a deep-discharged cell sits below it: in a release build 2400 − 2500 is 65436 mV
//! of headroom, and the pack reports itself full just before it dies.
//!
//! Bounds in the type (`Range<2000, 4300>`) are checked where a number enters (`try_new`, and on every read from
//! the wire), so a value inside the program is known to be in range. Arithmetic names its overflow policy:
//! `checked_*` gives `None`, `saturating_*` clamps to the range, never a silent wrap. Rounding into a coarser scale
//! names its rounding, and when the bounds prove the result fits, it can't fail at all.
//!
//! `cargo run --example 03_battery --features electrical,dimensionless`

use exact_units::prefix::{Centi, Deci, Milli};
use exact_units::units::{Ampere, Ratio, Second, Volt};
use exact_units::{Fixed, Range, RangeDec, Rounding};

/// A cell as the monitor reports it: 2.0 to 4.3 V in mV, anything else is a fault, not a voltage.
type Cell = Fixed<Volt, Milli, u16, Range<2000, 4300>>;
/// A threshold: the same unit, scale and storage, no bounds of its own.
type Threshold = Fixed<Volt, Milli, u16>;

const CUTOFF: Threshold = Fixed::new(2500);
const LOW_ALARM: Threshold = Fixed::new(3300);

fn main() {
    // the register values, as read: two good cells, one deep-discharged, one broken sense wire
    for raw in [3712, 3249, 2400, 65535] {
        let Ok(cell) = Cell::try_new(raw) else {
            println!("{raw:>5}: outside 2000..=4300 mV, a fault, not a reading");
            continue;
        };
        // a reading compares with a threshold across bounds: the same integer compare, the unit still checked
        let alarm = cell < LOW_ALARM;
        // headroom above the cutoff: `checked_sub` says when there is none instead of wrapping to 65436 mV
        let headroom = cell.unbounded().checked_sub(CUTOFF);
        // for the display, 0.1 V steps in a u8: the bounds prove 4300 mV fits, so this can't fail
        let shown: Fixed<Volt, Deci, u8> = cell.into_rounded(Rounding::NearestAway);
        println!(
            "{raw:>5}: shown as {shown:#}, headroom {}, {}",
            match headroom {
                Some(h) => format!("{h:#}"),
                None => "none, below the cutoff".into(),
            },
            if alarm { "LOW" } else { "ok" }
        );
    }
    // the plain-u16 version of the third line, as a release build computes it
    println!(
        "plain u16: 2400 - 2500 = {} mV of headroom",
        2400u16.wrapping_sub(2500)
    );

    // `/ 100` on a plain integer truncates 3.99 V to 3.9 V; the display above rounds, and says how
    let cell = Cell::bounded(3990); // `bounded` is the const constructor: outside the range, a compile error
    assert_eq!(
        cell.into_rounded::<Deci, u8>(Rounding::NearestAway).get(),
        40
    );
    assert_eq!(cell.into_rounded::<Deci, u8>(Rounding::Truncate).get(), 39);

    // the charge counter: 3000 mAh of capacity, kept in mA·s (1 mAh is 3600 mA·s); charging adds, and a full cell
    // stays full instead of counting past its capacity (saturating) or wrapping to empty (a u32 at its edge)
    type Charge = Fixed<exact_units::units::Coulomb, Milli, u32, Range<0, 10_800_000>>;
    let near_full = Charge::bounded(10_780_000);
    let one_minute_at_1a = Charge::bounded(60 * 1000);
    println!(
        "{near_full:#} + {one_minute_at_1a:#}: checked {:?}, saturating {:#}",
        near_full.checked_add(one_minute_at_1a).map(Fixed::get),
        near_full.saturating_add(one_minute_at_1a)
    );
    // (a current times a time is a charge too: 1 A for 60 s, one multiply, see 04_motor_power)
    let q = Fixed::<Ampere, Milli, u32>::new(1000)
        * Fixed::<Second, exact_units::prefix::One, u32>::new(60);
    assert_eq!(q.get(), one_minute_at_1a.get());

    // a float bound is decimal (floats can't be const parameters): a state-of-charge estimate from a filter is
    // 0.0 to 100.0 %, and an estimate that drifts past it is refused, not shown as 100.4 %
    type Soc = Fixed<Ratio, Centi, f32, RangeDec<0, 1000, -1>>;
    println!(
        "SoC 87.5 %: {:?}, 100.4 %: {:?}",
        Soc::try_new(87.5).map(Fixed::get),
        Soc::try_new(100.4)
    );

    // what it costs: a bound is a compile-time constant, a range check is the two compares a careful plain
    // version would write (the same instructions, measured on Cortex-M0/M4F/M33, see the README's Zero cost),
    // and the type is the u16
    assert_eq!(size_of::<Cell>(), size_of::<u16>());

    // a cell outside its range in const code doesn't compile (a real build, the message copied from it):
    //
    // error[E0080]: evaluation panicked: value above its bound
    //   --> examples/03_battery.rs:84:23
    //    |
    // 84 |     const BAD: Cell = Cell::bounded(5000);
    //    |                       ^^^^^^^^^^^^^^^^^^^ evaluation of `main::BAD` failed inside this call
    //
    // const BAD: Cell = Cell::bounded(5000);
}
