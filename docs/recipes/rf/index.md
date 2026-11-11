---
level: basic
---

# Recipes: RF

Eight everyday RF calculations, each worked with the crate and checked exactly. They are the questions a bench or a firmware asks: is the antenna matched, does the link close, how noisy is the receiver, which register values give this channel. Each page starts with the task and the numbers, and ends with a "Going deeper" section on what is exact, what is not, and why.

The example [`19_rf`](../examples.md) does all eight in one program, one function per page, and `tests/props/rf.rs` holds the properties that check them against exact references.

1. [Mismatch: Γ, VSWR and return loss](mismatch.md): a load on 50 [[Ω]], a VSWR spec, and why Γ = 1/2 is exactly 20·lg 2 dB of return loss.
2. [Link budget](link_budget.md): EIRP, the free-space path loss $20 \lg(4\pi d/\lambda)$, dBm and dBW, and dBi against dBd.
3. [Friis noise cascade](noise_cascade.md): noise figures in dB through a receiver chain, certified, and the −174 dBm/[[Hz]] that is a rounding.
4. [Fractional-N PLL](fractional_n.md): a synthesizer's register values, its frequency error and its integer-boundary spur, exact.
5. [Two-ports: S, Z, Y and ABCD](two_ports.md): an L-match that matches exactly, attenuator pads in cascade.
6. [Free-space impedance](free_space.md): $\eta_0 = \mu_0 c$ from a named CODATA set, and where 120π matters and where it doesn't.
7. [Skin depth and line input impedance](skin_depth.md): $\delta = \sqrt{\rho/(\pi f \mu_0)}$ as an exact scale, and a stub's reactance through tan.
8. [Phase noise to jitter](phase_noise.md): a clock's phase noise integrated over a band, and its RMS jitter.

The API these pages use is described in the RF chapter: [Impedance](../../rf/impedance.md), [Reflection](../../rf/reflection.md), [Levels and dBm](../../rf/levels.md). The rules behind "exact" are on [What exact means](../../start/exact.md).

## Where the crate has no place yet

These pages found five gaps. The example writes what it needs and says so where it does:

- **CPX-8**: no complex rational, so S, Z, Y and ABCD are exact only in the example's own `Cq`.
- **LVL-17**: no certified level (dB) of an `Exact` or a `Magnitude` at run time; `to_level` works in `f64`.
- **LVL-18**: no level of a spectral density (dBm/[[Hz]], dBc/[[Hz]]) and no integral of one over a band.
- **LVL-19**: no antenna gain against a named reference (dBi, dBd).
- **NUM-45**: no public interval, so the example brackets tan and lg with a few lines of its own.
