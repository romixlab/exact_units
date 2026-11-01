"""The `bits` fence: how a number's bits are laid out, drawn as coloured bit cells with a table of the fields.

Adapted from WireWeaver's fence of the same name (docs_ext/ww_docs_ext/bits.py in vhrdtech/wire_weaver), which draws
wire layouts. Here the fields are the parts of one stored integer (a sign, integer bits, fraction bits, the quadrant
of a binary angle), so the kinds are those of a number, and bits are numbered as the integer numbers them: the first
one written is the most significant, bit n-1, the last bit 0.

One field per line: its kind, its bits (groups of `0` and `1`), then its label, which starts at the first word that
isn't only `0` and `1` and may have `code`. A `hex` line gives the stored integer the bits make, in hex; the build
fails when they don't make it, so take it from a real value (a doctest on the same page), never from the bits by hand.

    ```bits
    sign 0                 +
    frac 100000000000000   2⁻¹
    hex  4000
    ```

Kinds: `sign` (the sign bit of two's complement), `int` (integer bits), `frac` (fraction bits), `quad` (the quadrant
bits of a binary angle), `pad` (bits that are not part of the value). Wired up in zensical.toml as a custom fence of
pymdownx.superfences; the colours are in docs/stylesheets/book.css (`.eu-bits`).
"""

import html
import re

from pymdownx.superfences import SuperFencesException

KINDS = {
    "sign": "sign",
    "int": "integer bits",
    "frac": "fraction bits",
    "quad": "quadrant",
    "pad": "not part of the value",
}

_BITS = re.compile(r"^[01]+$")
_CODE = re.compile(r"`([^`]+)`")


class Field:
    def __init__(self, kind, groups, label):
        self.kind = kind
        self.groups = groups
        self.label = label

    @property
    def bits(self):
        return "".join(self.groups)


def parse(source):
    """The fields and the expected integer (or None) of a fence's source."""
    fields = []
    expected = None
    for n, raw in enumerate(source.splitlines(), 1):
        line = raw.strip()
        if not line or line.startswith("#"):
            continue
        words = line.split()
        kind, rest = words[0], words[1:]
        if kind == "hex":
            try:
                expected = int("".join(rest), 16)
            except ValueError as e:
                raise SuperFencesException(f"bits fence, line {n}: bad hex in {line!r}: {e}") from e
            continue
        if kind not in KINDS:
            raise SuperFencesException(
                f"bits fence, line {n}: unknown kind {kind!r}, one of {', '.join(KINDS)} or hex"
            )
        groups = []
        while rest and _BITS.match(rest[0]):
            groups.append(rest.pop(0))
        if not groups:
            raise SuperFencesException(f"bits fence, line {n}: no bits in {line!r}")
        fields.append(Field(kind, groups, " ".join(rest)))
    if not fields:
        raise SuperFencesException("bits fence: no fields")
    return fields, expected


def _label(text):
    """A label as HTML: escaped, `code` spans kept."""
    return _CODE.sub(lambda m: f"<code>{m.group(1)}</code>", html.escape(text, quote=False))


def _span(top, bottom):
    return str(top) if top == bottom else f"{top}–{bottom}"


def render(source):
    fields, expected = parse(source)
    bits = "".join(f.bits for f in fields)
    width = len(bits)
    value = int(bits, 2)
    if expected is not None and value != expected:
        raise SuperFencesException(f"bits fence: the fields make {value:x}, the hex line says {expected:x}")

    out = ['<div class="eu-bits">']
    summary = f"{width} bits, 0x{value:0{(width + 3) // 4}x}"
    out.append(f'<div class="eu-bits__strip" role="img" aria-label="{html.escape(summary)}">')
    k = 0
    for f in fields:
        for i, b in enumerate(f.bits):
            pos = width - 1 - k
            start = " eu-bit--start" if i == 0 else ""
            title = html.escape(f"bit {pos}: {f.label.replace('`', '') or KINDS[f.kind]}")
            out.append(
                f'<span class="eu-bit eu-bit--{f.kind}{start}" title="{title}">'
                f'<span class="eu-bit__no">{pos}</span>{b}</span>'
            )
            k += 1
    out.append("</div>")

    used = [k for k in KINDS if any(f.kind == k for f in fields)]
    out.append('<div class="eu-bits__legend">')
    for kind in used:
        out.append(f'<span class="eu-bits__key"><span class="eu-bit eu-bit--{kind}"></span>{KINDS[kind]}</span>')
    out.append(f'<span class="eu-bits__key">stored: <code>0x{value:0{(width + 3) // 4}x}</code></span>')
    out.append("</div>")

    out.append('<table class="eu-bits__fields"><thead><tr><th>bits</th><th></th><th>field</th></tr></thead><tbody>')
    top = width - 1
    for f in fields:
        bottom = top - len(f.bits) + 1
        chips = " ".join(html.escape(g) for g in f.groups)
        out.append(
            f'<tr><td class="eu-bits__at">{_span(top, bottom)}</td>'
            f'<td><span class="eu-bits__chip eu-bit--{f.kind}">{chips}</span></td>'
            f"<td>{_label(f.label)}</td></tr>"
        )
        top = bottom - 1
    out.append("</tbody></table>")
    out.append("</div>")
    return "".join(out)


def fence(source, language, class_name, options, md, **kwargs):
    """The pymdownx.superfences format function of the `bits` fence."""
    return render(source)
