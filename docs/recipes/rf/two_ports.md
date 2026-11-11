---
level: advanced
---

# Two-ports: S, Z, Y and ABCD

A matching network, a filter or an attenuator is a two-port. On the bench it is measured as S-parameters against a reference impedance; for design it is easiest as an ABCD matrix, because a cascade of sections is the product of their matrices. The conversions between S, Z, Y and ABCD (Pozar, Microwave Engineering, 4th ed., Table 4.2) are rational functions of the entries, so for rational component values everything is exact: a network that matches, matches with $\Gamma = 0$, not $10^{-17}$.

| Section | ABCD |
|---|---|
| Series impedance $Z$ | $\begin{pmatrix} 1 & Z \\ 0 & 1 \end{pmatrix}$ |
| Shunt admittance $Y$ | $\begin{pmatrix} 1 & 0 \\ Y & 1 \end{pmatrix}$ |

and with $\Delta = A + B/Z_0 + C Z_0 + D$ over a real $Z_0$:

$$
S_{11} = \frac{A + B/Z_0 - C Z_0 - D}{\Delta}, \quad
S_{12} = \frac{2(AD - BC)}{\Delta}, \quad
S_{21} = \frac{2}{\Delta}, \quad
S_{22} = \frac{-A + B/Z_0 - C Z_0 + D}{\Delta}.
$$

## The crate has no complex rational yet

`Complex` takes integer and float storages; there is no `Complex` over exact fractions (CPX-8), and no two-port type with a unit per entry (S is a ratio, Z ohms, Y siemens, and ABCD mixes them: A and D ratios, B ohms, C siemens). The example writes a small `Cq`, a pair of `Fraction`s, and 2×2 arrays of it; the code blocks below carry the same, hidden. A two-port type belongs with glossator_value's model of measured networks, once CPX-8 gives it the numbers.

## An L-match that matches exactly

A 100 [[Ω]] load on a 50 [[Ω]] source: the textbook L-network has $Q = \sqrt{100/50 - 1} = 1$, a shunt susceptance of $Q/R_{\mathrm{L}} = 1/100$ [[S]] across the load and a series reactance of $Q R_{\mathrm{S}} = 50$ [[Ω]] toward the source. In ABCD, source side first:

```rust
# use exact_units::Fraction;
# #[derive(Copy, Clone, Debug, PartialEq)]
# struct Cq { re: Fraction, im: Fraction }
# impl Cq {
#     const fn int(re: i128, im: i128) -> Cq { Cq { re: Fraction::int(re), im: Fraction::int(im) } }
#     fn new(re: Fraction, im: Fraction) -> Cq { Cq { re, im } }
#     fn add(self, o: Cq) -> Option<Cq> { Some(Cq::new(self.re.checked_add(o.re)?, self.im.checked_add(o.im)?)) }
#     fn sub(self, o: Cq) -> Option<Cq> { Some(Cq::new(self.re.checked_sub(o.re)?, self.im.checked_sub(o.im)?)) }
#     fn mul(self, o: Cq) -> Option<Cq> {
#         Some(Cq::new(self.re.checked_mul(o.re)?.checked_sub(self.im.checked_mul(o.im)?)?,
#                      self.re.checked_mul(o.im)?.checked_add(self.im.checked_mul(o.re)?)?))
#     }
#     fn div(self, o: Cq) -> Option<Cq> {
#         let d = o.re.checked_mul(o.re)?.checked_add(o.im.checked_mul(o.im)?)?;
#         let n = self.mul(Cq::new(o.re, o.im.negate()))?;
#         Some(Cq::new(n.re.checked_div(d)?, n.im.checked_div(d)?))
#     }
#     fn scale(self, k: Fraction) -> Option<Cq> { Some(Cq::new(self.re.checked_mul(k)?, self.im.checked_mul(k)?)) }
# }
# type M2 = [[Cq; 2]; 2];
# fn cascade(a: &M2, b: &M2) -> Option<M2> {
#     let ([[a11, a12], [a21, a22]], [[b11, b12], [b21, b22]]) = (*a, *b);
#     let e = |x: Cq, y: Cq, z: Cq, w: Cq| x.mul(y)?.add(z.mul(w)?);
#     Some([[e(a11, b11, a12, b21)?, e(a11, b12, a12, b22)?], [e(a21, b11, a22, b21)?, e(a21, b12, a22, b22)?]])
# }
# fn s_of_abcd(m: &M2, z0: Fraction) -> Option<M2> {
#     let [[a, b], [c, d]] = *m;
#     let (bz, cz) = (b.scale(z0.checked_recip()?)?, c.scale(z0)?);
#     let den = a.add(bz)?.add(cz)?.add(d)?;
#     let two = Cq::int(2, 0);
#     let det = a.mul(d)?.sub(b.mul(c)?)?;
#     Some([[a.add(bz)?.sub(cz)?.sub(d)?.div(den)?, two.mul(det)?.div(den)?],
#           [two.div(den)?, Cq::int(0, 0).sub(a)?.add(bz)?.sub(cz)?.add(d)?.div(den)?]])
# }
# let series = |z: Cq| [[Cq::int(1, 0), z], [Cq::int(0, 0), Cq::int(1, 0)]];
# let shunt = |y: Cq| [[Cq::int(1, 0), Cq::int(0, 0)], [y, Cq::int(1, 0)]];
let j50 = Cq::int(0, 50); // +j50 Ω: an inductor
let j_100th = Cq::new(Fraction::int(0), Fraction::new(1, 100)); // +j/100 S: a capacitor
let net = cascade(&series(j50), &shunt(j_100th)).unwrap();
assert_eq!(net[0][0], Cq::new(Fraction::new(1, 2), Fraction::int(0))); // A = 1/2

let s = s_of_abcd(&net, Fraction::int(50)).unwrap();
// Γ at the input with 100 Ω on port 2 (Γ_L = 1/3): S₁₁ + S₁₂S₂₁Γ_L/(1 − S₂₂Γ_L)
let gamma_l = Cq::new(Fraction::new(1, 3), Fraction::int(0));
let num = s[0][1].mul(s[1][0]).unwrap().mul(gamma_l).unwrap();
let den = Cq::int(1, 0).sub(s[1][1].mul(gamma_l).unwrap()).unwrap();
let gamma_in = s[0][0].add(num.div(den).unwrap()).unwrap();
assert_eq!(gamma_in, Cq::int(0, 0)); // matched, exactly
```

At 100 [[MHz]] the parts are $L = 50/(2\pi f)$ = 79.6 [[nH]] and $C = (1/100)/(2\pi f)$ = 15.9 [[pF]]: $\pi$ enters only when reactances become component values, and a `Magnitude` holds it exactly until a value is asked for. The E12 parts nearest them (82 [[nH]], 15 [[pF]]) are a design choice; their $\Gamma_{\mathrm{in}}$ goes through $\pi$, so it is a named float edge.

## Attenuator pads in cascade

A 50 [[Ω]] T-pad of voltage ratio $K = 2$ has arms of $50(K-1)/(K+1) = 50/3$ [[Ω]] and a leg of $50 \cdot 2K/(K^2 - 1) = 200/3$ [[Ω]]. Its $S_{21}$ is exactly 1/2, the "6 dB pad" that is $20 \lg 2 = 6.0206$ dB, and two in cascade give exactly 1/4 with $S_{11} = 0$. The example builds both from the sections above and checks it with `==`.

## Going deeper

**Refusals have a meaning.** A series element has no Z matrix (its $C$ is 0, so $Z_{21} = 1/C$ is the point at infinity), and a shunt element has no Y matrix ($B = 0$). The conversions return `None` there, as `Impedance::to_admittance` returns `Extended::Infinity` for a short ([Impedance](../../rf/impedance.md#shorts-opens-and-the-extended-plane)). With exact arithmetic the refusal happens exactly where the matrix doesn't exist, never "nearly singular". A product past `Fraction`'s `u128` is also refused, never rounded.

**What the properties check.** `ladder` draws up to four series and shunt sections with complex values in eighths, cascades them, and checks the ABCD matrix and its S-parameters against Pozar's Table 4.2 written out on `i128` rationals; S back to ABCD gives the same matrix whenever the arithmetic fits, and Z and Y are refused exactly where C and B are 0. `line` checks a lossless line's input impedance against the reference and that $|\Gamma|$ at its input is the load's ([Skin depth and line input impedance](skin_depth.md)).

**Units per entry.** ABCD is where a unit system earns its keep: $A$ and $D$ are ratios, $B$ is an impedance and $C$ an admittance, so $AD - BC$ is a ratio only because [[Ω]]·[[S]] is one. The example keeps the units in the names (`series(z)` in ohms, `shunt(y)` in siemens) because a matrix of `Exact<U>` with a different `U` per entry is a type the crate doesn't have yet.
