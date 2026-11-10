---
level: basic
---

# Examples

The crate's `examples/` are short stories, in reading order. Each is a small device or tool, the bug a plain `u32` or `f32` lets through there, how the type stops it, and what it costs at run time. CI builds them all.

Run one with the line at the top of its file, for example:

```sh
cargo run --example 01_timer --no-default-features --features time
```

| # | Example | Features | The story |
|---|---|---|---|
| 1 | [`01_timer`][ex01] | `time` | a debounce timeout set in ms and armed in µs |
| 2 | [`02_thermostat`][ex02] | `thermal` | °C and °F setpoints; 1 °F of hysteresis is not −17.2 °C |
| 3 | [`03_battery`][ex03] | `electrical,dimensionless,time` | bounded readings, a broken wire's 65535 refused, a counter that saturates |
| 4 | [`04_motor_power`][ex04] | `electrical` | phase current through a shunt; mV/mΩ is A, mV·mA is µW |
| 5 | [`05_light_sensor`][ex05] | `electrical,dimensionless` | kinds: a V/V gain is not a PWM duty, a TIA's V/A is not a resistor |
| 6 | [`06_audio_limiter`][ex06] | `electrical,libm` | a limiter threshold in dBFS, worked out at compile time |
| 7 | [`07_tone_generator`][ex07] | `dimensionless,angle` | a phase that wraps (`Bam16`) and a mix that saturates (`Q15`) |
| 8 | [`08_data_logger`][ex08] | `counting,time` | channels from 1, ring slots from 0, packet numbers that wrap |
| 9 | [`09_capture_buffer`][ex09] | `counting,time,information` | samples, bytes and cycles as dimensions; KiB is not kB |
| 10 | [`10_adc_noise`][ex10] | `electrical,time` | an amplifier's nV/√Hz over a bandwidth |
| 11 | [`11_us_customer`][ex11] | `customary` | setpoints in psi and gallons, US or imperial named |
| 12 | [`12_wire`][ex12] | `shrink_wrap,time,electrical` | a reading sent to a PC tool, the same bytes as plain integers |
| 13 | [`13_impedance_match`][ex13] | `electrical,angle,dimensionless,libm` | an antenna's Γ, VSWR and return loss; a dB limit as an integer compare |
| 14 | [`14_iq_downconvert`][ex14] | `angle,dimensionless,electrical` | Q15 IQ samples mixed down by an NCO on a Cortex-M0 |
| 15 | [`15_nuclear`][ex15] | `radiation,systems,time,chemical,dimensionless,mechanical` | AME2020 masses to binding energies and Q-values, a gram of radium in Bq, Gy against Sv ([Nuclear physics](nuclear.md)) |

The lines that don't compile are in the files too, each with the compiler's real message.

[ex01]: https://github.com/romixlab/exact_units/blob/main/examples/01_timer.rs
[ex02]: https://github.com/romixlab/exact_units/blob/main/examples/02_thermostat.rs
[ex03]: https://github.com/romixlab/exact_units/blob/main/examples/03_battery.rs
[ex04]: https://github.com/romixlab/exact_units/blob/main/examples/04_motor_power.rs
[ex05]: https://github.com/romixlab/exact_units/blob/main/examples/05_light_sensor.rs
[ex06]: https://github.com/romixlab/exact_units/blob/main/examples/06_audio_limiter.rs
[ex07]: https://github.com/romixlab/exact_units/blob/main/examples/07_tone_generator.rs
[ex08]: https://github.com/romixlab/exact_units/blob/main/examples/08_data_logger.rs
[ex09]: https://github.com/romixlab/exact_units/blob/main/examples/09_capture_buffer.rs
[ex10]: https://github.com/romixlab/exact_units/blob/main/examples/10_adc_noise.rs
[ex11]: https://github.com/romixlab/exact_units/blob/main/examples/11_us_customer.rs
[ex12]: https://github.com/romixlab/exact_units/blob/main/examples/12_wire.rs
[ex13]: https://github.com/romixlab/exact_units/blob/main/examples/13_impedance_match.rs
[ex14]: https://github.com/romixlab/exact_units/blob/main/examples/14_iq_downconvert.rs
[ex15]: https://github.com/romixlab/exact_units/blob/main/examples/15_nuclear.rs
