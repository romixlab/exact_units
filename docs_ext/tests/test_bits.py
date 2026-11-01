"""The `bits` fence of the book (eu_docs_ext.bits)."""

import pytest
from pymdownx.superfences import SuperFencesException

from eu_docs_ext.bits import parse, render

# Q15 one half: the sign, then 15 fraction bits, 0x4000 (docs/fixed_point/q_formats.md)
HALF = """
sign 0                 +
frac 100000000000000   2⁻¹, `16384` / 2¹⁵
hex  4000
"""


def test_fields_and_hex():
    fields, expected = parse(HALF)
    assert [f.kind for f in fields] == ["sign", "frac"]
    assert fields[1].label == "2⁻¹, `16384` / 2¹⁵"
    assert expected == 0x4000


def test_render_numbers_bits_from_the_msb():
    out = render(HALF)
    assert out.count('title="bit ') == 16
    assert 'title="bit 15: +"' in out
    assert '<td class="eu-bits__at">14–0</td>' in out
    assert "<code>16384</code>" in out
    assert "<code>0x4000</code>" in out


def test_a_wrong_hex_line_fails_the_build():
    with pytest.raises(SuperFencesException, match="the fields make 4000, the hex line says 4001"):
        render(HALF.replace("hex  4000", "hex  4001"))


def test_an_unknown_kind_fails_the_build():
    with pytest.raises(SuperFencesException, match="unknown kind 'head'"):
        render("head 0101 n")
