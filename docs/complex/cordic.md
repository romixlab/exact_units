---
level: advanced
---

# CORDIC

Angles and roots in integer arithmetic only: no float, no table but the constant ones, the same on a Cortex-M0 as anywhere.

## How it works

CORDIC turns a vector by $\pm\arctan 2^{-i}$ in step $i$. A turn by such an angle needs only a shift and an add:

$$
\begin{aligned}
x_{i+1} &= x_i - d_i\, y_i\, 2^{-i} \\
y_{i+1} &= y_i + d_i\, x_i\, 2^{-i} \\
z_{i+1} &= z_i - d_i \arctan 2^{-i}
\end{aligned}
\qquad d_i = \pm 1
$$

- **Vectoring** (for atan2): pick $d_i$ to drive $y$ to 0. The angles add up to the vector's angle.
- **Rotation** (for sin and cos): pick $d_i$ to drive $z$ to 0, starting from the angle. $(x, y)$ ends at $(\cos, \sin)$.

Each step stretches the vector by $\sqrt{1 + 2^{-2i}}$; after many steps by $K = \prod_i \sqrt{1 + 2^{-2i}} \approx 1.6468$, which the code divides out with a constant.

<figure class="eu-figure" markdown="0">
--8<-- "figures/cordic_vectoring.svg"
<figcaption>Vectoring: each step turns the vector closer to the x axis, by a smaller angle.</figcaption>
</figure>

## The functions

| Function | In | Out | Steps | Wide type |
|---|---|---|---|---|
| `atan2_16` | `i16, i16` | `Bam16` | 24 | `i32` |
| `atan2_32` | `i32, i32` | `Bam32` | 40 | `i64` |
| `sin_cos_16` | `Bam16` | `Q15`, `Q15` | 24 | `i32` |
| `sin_cos_32` | `Bam32` | `Q31`, `Q31` | 40 | `i64` |
| `isqrt_u32`, `_u64`, `_u128` | integer | integer, by a `Rounding` | | |

```rust
use exact_units::{Bam16, Rounding, cordic};

// the angle of (x, y) = (−1, 1): 3/8 of a turn, 135°
assert_eq!(cordic::atan2_16(1, -1), Bam16::of(0x6000));
// sin and cos of 30°: 1/2 and √3/2 in Q15
let (sin, cos) = cordic::sin_cos_16(Bam16::of(5461));
assert_eq!((sin.get(), cos.get()), (16383, 28378));
// √2 · 2¹⁵, rounded
assert_eq!(cordic::isqrt_u32(2 << 30, Rounding::NearestEven), 46341);
```

All of them are `const fn`.

## How exact

Every result is within 0.51 of a step of the exact value: half a step from the one rounding at the end, less than a hundredth from the iterations. The sine of `sin_cos_16` against the exact sine, over every angle of a turn:

<figure class="eu-figure" markdown="0">
--8<-- "plots/cordic_error.svg"
<figcaption>The largest error in each 1/512 of a turn, in Q15 steps: half a step from the final rounding, and a hair more where the exact sine lies near a tie. Computed by <code>tools/book_plots</code> with <code>cordic::sin_cos_16</code>.</figcaption>
</figure>

- The axes and the multiples of an eighth of a turn are exact.
- 1 is not a Q15 value, so $\cos 0$ is $1 - 2^{-15}$. −1 is exact.

The [polar form](complex.md#polar-form) of a complex number and the [NCO](../rf/iq.md#an-nco-and-a-mixer) are built on these.
