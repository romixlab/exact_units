---
level: advanced
---

# In your editor

A WireWeaver API names its types in Rust: `Fixed<Volt, Milli, i16>`. That is exact, but it is not what you think while you read a trait. You think "millivolts, 16 bits, up to 32.767 [[V]]".

[`ww_lsp`](https://github.com/romixlab/ww_lsp) is a language server that writes that for you. It runs beside rust-analyzer and shows the exact_units type of each API item as an inlay hint, with the details on hover.

!!! warning "A prototype"
    [`ww_lsp`](https://github.com/romixlab/ww_lsp) is a prototype. It is not on crates.io yet, there are no releases, and what it does may change. This page shows what it does today.

## Inlay hints

<figure class="eu-figure" markdown="0">
<pre class="eu-editor" aria-label="A WireWeaver trait with inlay hints after each type"><code><span class="c">#[ww_trait]</span>
<span class="k">pub trait</span> Output {
    property!(rw v_set: Fixed&lt;Volt, Milli, i16&gt;)<span class="h">mV · i16 · ±32.767 V</span>;
    property!(rw i_limit: Fixed&lt;Ampere, Milli, u16, Range&lt;0, 5000&gt;&gt;)<span class="h">mA · u16 · 0..5 A</span>;
    property!(rw ramp: Millis&lt;u32&gt;)<span class="h">ms · u32</span>;
    property!(rw trim: Q15)<span class="h">Q15 · −1..1</span>;
    property!(ro power: Fixed&lt;Watt, Pow2&lt;-8&gt;, u32&gt;, MeterError)<span class="h">UQ24.8 W</span>;
    stream!(i_out: Fixed&lt;Ampere, Rational&lt;1, 4096&gt;, i16&gt;)<span class="h">Q3.12 A · −8..8 A</span>;
    property!(ro+observe heatsink: Point&lt;Celsius, Fixed&lt;Kelvin, Centi, i16&gt;&gt;)<span class="h">°C</span>;
}</code></pre>
<figcaption>The hints are the grey boxes. Text from <code>ww_lsp hints</code> on a bench power supply API.</figcaption>
</figure>

A hint reads: unit with its prefix, storage, and the range in the unit.

- `mV · i16 · ±32.767 V`: millivolts in an `i16`. The range is all of `i16`, so it is symmetric.
- `mA · u16 · 0..5 A`: the `Range<0, 5000>` bound shows as the range.
- `Q15 · −1..1`: a scale of $2^{-n}$ on an integer is named as a Q format.
- `UQ24.8 W` and `Q3.12 A`: the same for other widths.
- `°C`: a `Point` shows its origin's symbol.

The aliases (`Millis`, `Hz`, `Percent`, `Duty16`, `Bam16`, `Q15<U>` and the others) are read as the types they are. A type `ww_lsp` does not know gets no hint.

## Hover

Hover on a type to see the facts. For `Fixed<Volt, Milli, i16>`:

```text
mV · i16 · ±32.767 V

- unit: Volt (V), kg·m²·s⁻³·A⁻¹
- scale: 10⁻³ (Milli), exact
- step: 0.001 V
- range: −32.768 ..= 32.767 V (Unbounded, -32768 ..= 32767 steps)
- storage: i16, 2 bytes on the wire
```

It gives:

1. The unit, with its name and dimension.
2. The exact scale. A rounded value is marked `≈`.
3. The step, and the whole range to the last digit.
4. The bounds.
5. The storage and its size on the wire.

## Every number comes from exact_units

`ww_lsp` has no table of its own for units and scales. It reads the unit symbols, dimensions, `Magnitude`s, storage limits and wire sizes from exact_units itself. When the crate changes, the hints change with it, and they cannot disagree with what the compiler does.

## Install

You need a Rust toolchain. This builds `ww_lsp` from its repository and puts it in `~/.cargo/bin`:

```sh
cargo install --locked --git https://github.com/romixlab/ww_lsp ww_lsp
ww_lsp --version
```

Or clone the repository and run `just install` in it.

## Try it without an editor

`ww_lsp` prints the same analysis for a terminal. Give it any Rust file with a WireWeaver API:

```sh
ww_lsp hints my_api.rs            # FILE:LINE:COL  HINT  (ITEM)
ww_lsp hover my_api.rs:30:25      # the hover text at a line and column
```

The repository has a bench power supply API to try, `crates/ww_lsp_core/tests/fixtures/psu_api.rs`.

## Try it in an editor

The server is `ww_lsp serve` on stdin and stdout. Attach it to Rust files beside rust-analyzer and turn inlay hints on. Both servers stay attached to the buffer, so rust-analyzer is unchanged.

Neovim 0.11 and later:

```lua
vim.lsp.config('ww_lsp', {
  cmd = { 'ww_lsp', 'serve' },
  filetypes = { 'rust' },
  root_markers = { 'Cargo.toml', '.git' },
})
vim.lsp.enable('ww_lsp')
vim.lsp.inlay_hint.enable(true)
```

Helix, in `languages.toml`:

```toml
[language-server.ww_lsp]
command = "ww_lsp"
args = ["serve"]

[[language]]
name = "rust"
language-servers = ["rust-analyzer", "ww_lsp"]
```

and `display-inlay-hints = true` under `[editor.lsp]` in `config.toml`.

Zed starts only servers that an extension registers, so `ww_lsp` comes with a small extension, `editors/zed` in its repository. It is not in Zed's registry yet. The README of `ww_lsp` has the steps for it and for older Neovim.

## What it does not do yet

- It reads one file. Types from other files and crates, and aliases defined there, get no hint.
- While the file does not parse, in the middle of typing, the hints of the last good text stay.
- Chemical, photometric, radiation, counting and customary units get no hint yet.
- No diagnostics and no completion yet.
