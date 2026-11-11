---
level: basic
---

# Recipes: image and video codecs

Image and video codecs compute in integers on purpose: a decoder must give the same bits on every machine. So their standards are full of the things "exact" is about. Each formula has a rounding, named or not. Lossless transforms must give every sample back. Scales are an integer times a power of two known only at run time. Irrational constants are printed as decimals. This page works through JPEG (ITU-T T.81, T.871), JPEG 2000 (T.800, the 08/2002 edition), H.264, H.265, H.273 and AV1, and says for each step which rounding it is and what the crate checks. The example [`17_image_codecs`](examples.md) does all of it in one program, and `tests/props/image_codecs.rs` holds the properties.

| Step | Exact | Rounding | Float edge |
|---|---|---|---|
| JPEG quantization, $S_{\mathrm{q}} = \mathrm{round}(S/Q)$ (T.81 A.3.4) | the quotient | nearest, the tie unnamed | |
| JPEG dequantization, $R = S_{\mathrm{q}} \cdot Q$ | yes | | |
| Point transform (T.81 A.4, K.10) | | `Truncate` (AC, lossless), `Floor` (DC) | |
| JFIF's Round, AV1's Round2, H.264/H.265 scaling | | `NearestCeil` | |
| H.273's Round (eq. 8) | | `NearestAway` | |
| 5/3 wavelet, RCT, YCgCo-R | lossless | `Floor` | |
| T.800's step size, H.265's QP scale, HRD bit rate and CPB size | yes, `CodecShape::MantissaExponent` | | |
| 9/7 wavelet taps (T.800 Table F.4) | | | $\sqrt{15}$ and cube roots |
| AV1's Cos128_Lookup (7.13.2) | | nearest | $\cos(k\pi/128)$ |
| Sample aspect ratio (H.264 Table E-1) | a table over $1/198$ | | |
| Y′CbCr matrices from $K_{\mathrm{R}}$ and $K_{\mathrm{B}}$ | yes | | |

## Four roundings in one chain

JPEG quantizes each DCT coefficient as $S_{\mathrm{q}} = \mathrm{round}(S/Q)$, and T.81 A.3.4 says only "Rounding is to the nearest integer". A tie, $S/Q = 2.5$, is left to the encoder. The crate never picks a tie for the caller, so the call names it:

```rust
use exact_units::{Exact, Rounding, prefix::One, units::Ratio};

// Q is one of 64 run-time entries of a quantization table, so S/Q is an Exact, not a compile-time scale
let s_over_q = Exact::<Ratio>::new(20, 8);
let sq = |r| s_over_q.to_rounded::<One, i16>(r).map(|v| v.get());
assert_eq!(sq(Rounding::NearestEven), Some(2));
assert_eq!(sq(Rounding::NearestAway), Some(3));
```

JPEG's point transform divides by $2^{\mathrm{Pt}}$ in two ways (T.81 A.4). AC coefficients and lossless samples get "an integer divide", which truncates toward zero. DC coefficients get "an arithmetic-shift-right", which floors. K.10 shows the difference in a table, and every row of it is a test. $\mathrm{Pt}$ is known only at run time, so the divide goes through a run-time `Prefix` and `Prefix::rescale`:

```rust
use exact_units::{Overflow, Prefix, Rounding};

let by_2_pt = |x: i32, pt: i8, r| Prefix::from_pow2(-pt)?.rescale::<i32, i32>(x, Prefix::Unit, r, Overflow::Checked);
// K.10, Pt = 1: -1 is 0 divided, -1 shifted; -3 is -1 divided, -2 shifted
assert_eq!(by_2_pt(-1, 1, Rounding::Truncate), Some(0));
assert_eq!(by_2_pt(-1, 1, Rounding::Floor), Some(-1));
assert_eq!(by_2_pt(-3, 1, Rounding::Truncate), Some(-1));
assert_eq!(by_2_pt(-3, 1, Rounding::Floor), Some(-2));
```

JFIF defines $\mathrm{Round}(x) = \lfloor x + 0.5 \rfloor$ (T.871 clause 7): a half goes up, toward $+\infty$, also below zero. AV1's $\mathrm{Round2}(x, n) = (x + 2^{n-1}) \gg n$ (section 4.7) is the same rounding, and so are H.265's scaling (eq. 8-309) and H.264's (eq. 8-337), which add half a step before the shift. This is `NearestCeil`, a tie toward $+\infty$, and none of the other five: `Ceil` agrees on halves but not on quarters, `NearestEven` and `NearestAway` part on −0.5. The properties check it against $\lfloor x + 0.5 \rfloor$ on every quarter from −1.5 to 1.5, and that each other mode differs there.

H.273 defines $\mathrm{Round}(x) = \mathrm{Sign}(x) \cdot \mathrm{Floor}(\mathrm{Abs}(x) + 0.5)$ (eq. 8): a half away from zero, `NearestAway`. Its YCgCo (eq. 51 to 53) rounds so, and its NOTE 3 warns that Cb and Cr can reach $1 \ll \mathrm{BitDepth}_{\mathrm{C}}$, one past their range. The properties walk every 8-bit colour and find it exactly where NOTE 3 says: $\mathrm{Cb} = 256$ for pure green. A `u8` refuses it instead of wrapping it to 0.

## Lossless transforms

JPEG 2000's reversible 5/3 wavelet (T.800 F.3.8.1, eq. F-5 and F-6) and its reversible colour transform (G.2, eq. G-3 to G-8), and H.273's YCgCo-R (eq. 58 to 65), are lifting steps with floors: every sample comes back. Each `⌊·/4⌋` is a `Floor` from a scale of $2^{-2}$:

```rust
use exact_units::{Fixed, Rounding, prefix::{One, Pow2}, units::Ratio};

let quarter = |x: i32| Fixed::<Ratio, Pow2<-2>, i32>::new(x).into_rounded::<One, i32>(Rounding::Floor).get();
// the RCT forward (G-3 to G-5) and back (G-6 to G-8)
let (i0, i1, i2) = (72, -98, 127);
let (y0, y1, y2) = (quarter(i0 + 2 * i1 + i2), i2 - i1, i0 - i1);
let back1 = y0 - quarter(y2 + y1);
assert_eq!((y2 + back1, back1, y1 + back1), (i0, i1, i2));
```

The properties check the three transforms against the standards' formulas written out in `i128`. They walk every pair of 8-bit samples (every 8-bit colour for YCgCo-R), draw 16-bit ones with proptest, and prove them for 16-bit samples with Kani (`just verify`).

The forward 5/3 is informative (F.4.8.1), and in the 08/2002 edition its first step, F-9, is printed with $\lfloor (X(2n) + X(2n+2))/4 \rfloor$. Its inverse F-6 adds back $\lfloor (X(2n) + X(2n+2))/2 \rfloor$, so with a 4 the inverse doesn't undo the forward: `[2, 0]` comes back as `[2, 1]`, and of the 65 536 two-sample 8-bit signals only 512 survive. A 2 is meant, and Technical Corrigendum 4 (10/2014) corrects the divisor to 2; the 06/2019 edition prints it right. The current edition, 07/2024, is not freely available and was not checked.

## Mantissa times a power of two

Three scales in these standards have one shape: an integer mantissa times 2 to an exponent read from the stream.

- JPEG 2000's step size $\Delta_b = 2^{R_b - \varepsilon_b} (1 + \mu_b/2^{11})$ (T.800 E.1.1.1, eq. E-3), with $\varepsilon_b$ in 5 bits and $\mu_b$ in 11 (Table A.30).
- H.265's dequantization scale, $\mathrm{levelScale}[\mathrm{qP} \bmod 6] \ll (\mathrm{qP}/6)$ with $\mathrm{levelScale} = \{40, 45, 51, 57, 64, 72\}$ (8.6.3, eq. 8-309); H.264's is the same with its $v$ tables (8.5.9, eq. 8-315 and 8-318; 8.5.12.1, eq. 8-336).
- The HRD's $\mathrm{BitRate} = (\mathrm{bit\_rate\_value\_minus1} + 1) \cdot 2^{6 + \mathrm{bit\_rate\_scale}}\,\unit{bit/s}$ and $\mathrm{CpbSize} = (\mathrm{cpb\_size\_value\_minus1} + 1) \cdot 2^{4 + \mathrm{cpb\_size\_scale}}\,\unit{bit}$ (H.264 E.2.2, eq. E-81 and E-82; H.265 E.3.3, eq. E-87 and E-88).

Each is a codec of `CodecShape::MantissaExponent`: the code splits into an exponent and a mantissa, and the value is `(mantissa + offset) · 2^(zero + step · exponent)`, exact. `J2kStepSize` decodes T.800's 16-bit code to $\Delta_b / 2^{R_b}$, `HevcLevelScale` H.265's qP to its scale, and `HrdBitRate` and `HrdCpbSize` the HRD's two fields, packed as `scale << 32 | value_minus1`, to [[bit/s]] and [[bit]]. Encoding goes back: a value between two codes takes the one a named rounding picks, and a value that several codes give takes the lowest.

```rust
use exact_units::{Exact, Rounding, codec::{Codec, HevcLevelScale, HrdBitRate, J2kStepSize}, units::Ratio};

// εb = 8, μb = 1024: Δb = 2^(Rb − 8) · (1 + 1024/2048) = 2^Rb · 3/512
let step = J2kStepSize::decode((8 << 11) | 1024).unwrap();
assert_eq!(Exact::from_fixed(step), Exact::<Ratio>::checked_new(3, 512));

// qP 26: levelScale[26 % 6] << (26 / 6) = 51 << 4
assert_eq!(HevcLevelScale::decode(26).unwrap().get(), 816);

// bit_rate_value_minus1 = 15 624, bit_rate_scale = 4: 16 Mbit/s
let rate = HrdBitRate::decode((4 << 32) | 0x3D08).unwrap();
assert_eq!(rate.get(), 16_000_000);
// 250 000 · 2⁶ at scale 0 is the same rate: the lowest code
assert_eq!(HrdBitRate::encode(rate, Rounding::NearestEven), Some(249_999));
```

The properties walk every code of the first two and every scale of the HRD codes, and encode values on and between codes by every rounding.

H.264's NOTE 2 in 8.5.8 says the scale "doubles for every increment of 6 in QPY" and grows "approximately 12 % for each increase of 1". The doubling is exact. The steps in between are the tables' own ratios, $45/40 = 1.125$, $51/45 = 1.133\ldots$, $57/51 = 1.118\ldots$, never $2^{1/6} = 1.122\ldots$, which no formula of either standard contains. The properties check every qP from 0 to 51: H.265's steps are within 0.012 of $2^{1/6}$, H.264's within 0.089 ($19/18$ in the 8×8 table).

## Irrational constants

JPEG 2000's irreversible 9/7 wavelet lifts by $\alpha$, $\beta$, $\gamma$, $\delta$ and $K$ (T.800 Table F.4). Tables F.5 to F.7 give their closed forms, built from $\sqrt{15}$ and two cube roots, and print each "including approximations up to 15 decimal points" (F.3.8.2.1). These are algebraic numbers, not a [Magnitude](../nomicon/magnitude.md), so a 9/7 transform is a float edge by necessity.

The property computes every one of the 19 parameters as an interval of exact rationals, about $10^{-22}$ wide, and rounds both ends to 15 decimals. 17 are the printed digits, in the 08/2002 edition. Two round differently from the exact value, one off in the 15th decimal, unchanged in the 06/2019 edition (the current edition, 07/2024, is not freely available and was not checked):

- $r_0$ (Table F.6) is printed 1.449 513 704 087 943; it is 1.449 513 704 087 942 3….
- $\gamma$ (Table F.4) is printed 0.882 911 075 530 934; it is 0.882 911 075 530 932 9….

AV1's inverse DCT rotates by Cos128_Lookup (7.13.2), whose note says it "implements the expression 4096 * cos( angle * pi / 128 ) rounded to the nearest integer". The tie is unnamed again, but it never matters: the property bounds each of the 65 cosines with $\pi$ between two 50-digit decimals and a Taylor series, and every entry is the nearest integer by `NearestEven` and `NearestAway` alike.

## Tables and matrices

H.264's sample aspect ratio indicator (Table E-1, H.265 Table E.1 the same) has 16 ratios from 1:1 to 160:99. All are whole numbers of $1/198$, so the indicator is a [codec](../fixed_point/codecs.md) with `CodecShape::Table` and `Linear = Fixed<Ratio, Rational<1, 198>, u16>`. Code 0 (Unspecified), 17 to 254 (Reserved) and 255 (Extended_SAR) are spare codes, and a [niche](../maths/niches.md) enum names 0 and 255. Extended_SAR means "the ratio follows in two fields of their own", which no codec can decode from the byte alone.

JFIF's and JPEG 2000's Y′CbCr matrices come from BT.601's $K_{\mathrm{R}} = 0.299$ and $K_{\mathrm{B}} = 0.114$. Every entry is then a rational: Cb divides by $2(1 - K_{\mathrm{B}}) = 1.772$ and Cr by $2(1 - K_{\mathrm{R}}) = 1.402$ (T.871 clause 7). The property builds both matrices exactly, checks that each undoes the other, and rounds the entries to the printed places:

- JFIF's four-decimal entries (0.1687, 0.3313, 0.4187, 0.0813, 0.3441, 0.7141) are all right.
- T.800's 08/2002 edition's G-10 prints 0,16875, which rounds differently from the exact $K_{\mathrm{R}}$/$K_{\mathrm{B}}$ value, 0.16874: one off in the last place. The 06/2019 edition prints it the same.
- T.800's 08/2002 edition's G-10 prints 0,331260, which rounds differently from the exact value, 0.331264 ($0.587/1.772 = 0.331\,264\,1\ldots$): four off in the sixth decimal. As 0,33126 it would match to five places. The 06/2019 edition prints it the same.
- T.800's 08/2002 edition's G-13 prints 0,34413, which rounds differently from the exact value, 0.34414: one off in the last place. The 06/2019 edition prints it the same. T.800 says directly of G-12 to G-14's coefficients, "Equations (G-12), (G-13) and (G-14) do not imply a required precision for the coefficients" (06/2019 edition, p. 135); G-10's forward coefficients are printed to the same kind of approximation. The current edition, 07/2024, is not freely available and was not checked.
- T.800's 08/2002 edition's G-9 is a formula error, not a rounding difference: it prints $Y_0(x, y) = -0{,}299\, I_0(x, y) - 0{,}587\, I_1(x, y) + 0{,}114\, I_2(x, y)$, where its inverse, G-12 to G-14, undoes the matrix with plus signs, not that one. Technical Corrigendum 3 (10/2014) corrects G-9 to the plus signs, and the 06/2019 edition prints it right.

JFIF's `units` byte (T.871 10.1) picks what its densities mean. 0 means only the pixel aspect ratio, Vdensity:Hdensity. 1 means dots per inch, "dots per 2.54 cm". 2 means dots per cm. In dots per metre, 72 dpi is $360\,000/127$ exactly. One byte choosing among three kinds of value is something a codec, with its one `Linear`, can't say; the example uses an enum.

## Where the crate falls short here

- **A mantissa times a power of ten has no codec shape.** `MantissaExponent` is base 2; IEEE 11073's SFLOAT has the same shape in base 10.
- **A code that picks the unit has no codec.** JFIF's `units` byte decodes to one of three kinds of value.
- **No counted pixel or dot unit.** "Dots per inch" is a `Per<Ratio, Inch>` here.
- **A quantization table is 64 run-time scales.** A `Fixed` has one compile-time scale, so a dequantized coefficient is an integer and its step a separate run-time value.
- **A code meaning "the value follows elsewhere"** (Extended_SAR) is a spare code with a niche name, not a meaning the codec knows.
