---
level: basic
---

# Reading levels

Every page has a small badge above its title. It says how much the page assumes. Read the Basic pages first and stop when you have what you need.

<div class="grid cards eu-level-cards" markdown>

-   <span class="eu-level eu-level--basic"><svg class="eu-level__icon" viewBox="0 0 14 12" aria-hidden="true"><rect x="0" y="7" width="3.2" height="5" rx="0.6"/><rect x="5.4" y="3.5" width="3.2" height="8.5" rx="0.6" class="eu-level__off"/><rect x="10.8" y="0" width="3.2" height="12" rx="0.6" class="eu-level__off"/></svg><span class="eu-level__label">Basic</span></span>

    ---
    What you need to use the crate: what a feature is for, the common case, a working example. Start here.

-   <span class="eu-level eu-level--advanced"><svg class="eu-level__icon" viewBox="0 0 14 12" aria-hidden="true"><rect x="0" y="7" width="3.2" height="5" rx="0.6"/><rect x="5.4" y="3.5" width="3.2" height="8.5" rx="0.6"/><rect x="10.8" y="0" width="3.2" height="12" rx="0.6" class="eu-level__off"/></svg><span class="eu-level__label">Advanced</span></span>

    ---
    Every option and when to pick it: rounding, overflow, fixed-point products, complex numbers. For when the defaults are not enough.

-   <span class="eu-level eu-level--nomicon"><svg class="eu-level__icon" viewBox="0 0 14 12" aria-hidden="true"><rect x="0" y="7" width="3.2" height="5" rx="0.6"/><rect x="5.4" y="3.5" width="3.2" height="8.5" rx="0.6"/><rect x="10.8" y="0" width="3.2" height="12" rx="0.6"/></svg><span class="eu-level__label">Nomicon</span></span>

    ---
    The inside, as in the Rustonomicon: exact magnitudes, how a conversion is worked out at compile time, how the crate is tested and proved.

</div>

A level says how much a page assumes, not how important it is.

In the navigation, each part lists its Basic pages first and groups the Advanced ones after them.

## For writers

- The badge comes from the page's front matter: `level: basic`, `level: advanced` or `level: nomicon`. `overrides/partials/level.html` draws it.
- Every Rust block on a page is a doctest (`src/book.rs` includes the page). A block that must not compile is fenced `rust,compile_fail`; the site labels it.
- Formulas are LaTeX between `$...$` (inline) or `$$...$$` (display), drawn by KaTeX.
- Every formula, equation, ratio of symbols and physical constant in prose is math, never plain text such as `m_u·c²`: write $m_{\mathrm{u}} c^2$. Quantity symbols are italic and a subscript that is a name is upright (`m_{\mathrm{u}}`, `V_{\mathrm{ref}}`), as SI Brochure 9th §5.1 asks. A unit inside math is `\unit{kHz}`, checked against `q!`'s table when the book is built (`$90\,\unit{kHz}$`); a unit in prose is `[[kHz]]`. What the reader types (a literal, a type, a time code) stays code, and code never holds math.
- A display formula's lines start at column 0: the page is also a doctest file, and an indented line would be read as Rust. Raw HTML (a figure's caption) takes `\(..\)`.
- A number's bit layout is a `bits` fence: one field per line with its kind (`sign`, `int`, `frac`, `quad`, `pad`), its bits and a label, then a `hex` line with the stored integer. The build fails when the bits don't make it.
- Drawings are SVG files in `docs/figures`, inlined with `--8<--` so they follow the light and dark scheme. Plots are computed by the crate itself (`tools/book_plots`) when the site is built; they are not in git.
- Text with colons that is no emoji (a time code `hh:mm:ss;ff`, `4:2:2`) goes in code style: in prose `:mm:` is drawn as a flag. `just docs-build` fails on any emoji or icon not on the list in `docs_ext/eu_docs_ext/check_emoji.py`.
