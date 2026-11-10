"""q!'s symbol table and its resolution (eu_docs_ext.unit_catalog), against the real docs/start/symbols.md so a
change to the generated table is caught here too, not only by the markup it feeds.
"""

import pytest

from eu_docs_ext.unit_catalog import UnitSymbolError, load_catalog, resolve_expression


@pytest.fixture(scope="module")
def catalog():
    return load_catalog()


def factors(parts):
    return [v.text for k, v in parts if k == "factor"]


def test_a_bare_symbol(catalog):
    parts = resolve_expression("Bq", catalog)
    assert parts == [("factor", parts[0][1])]
    assert parts[0][1].text == "Bq"
    assert parts[0][1].anchor == "sym-Bq"
    assert "becquerel" in parts[0][1].tooltip and "activity" in parts[0][1].tooltip


def test_a_prefixed_symbol(catalog):
    (kind, factor), = resolve_expression("keV", catalog)
    assert kind == "factor"
    assert factor.anchor == "sym-eV"
    assert "kilo" in factor.tooltip and "electronvolt" in factor.tooltip


def test_a_folded_symbol_reads_as_its_prefix_and_unit(catalog):
    (kind, factor), = resolve_expression("km", catalog)
    assert factor.anchor == "sym-km"
    assert "kilo" in factor.tooltip and "metre" in factor.tooltip


def test_a_compound_with_an_operator_and_an_exponent(catalog):
    parts = resolve_expression("mSv/h", catalog)
    assert factors(parts) == ["mSv", "h"]
    texts = [v for k, v in parts if k == "text"]
    assert "/" in texts


def test_m_per_s_squared(catalog):
    parts = resolve_expression("m/s²", catalog)
    assert factors(parts) == ["m", "s"]
    assert ("text", "²") in parts


def test_j_per_kg(catalog):
    parts = resolve_expression("J/kg", catalog)
    assert factors(parts) == ["J", "kg"]


def test_an_ambiguous_bare_symbol_is_refused(catalog):
    with pytest.raises(UnitSymbolError, match=r"\[\[min\]\]: reads more than one way"):
        resolve_expression("min", catalog)


def test_an_ambiguous_symbol_resolves_once_prefixed(catalog):
    # mcd is a millicandela, never a millicord: Cord takes no prefix, so only one candidate ever matches.
    (kind, factor), = resolve_expression("mcd", catalog)
    assert factor.anchor == "sym-cd"
    assert "candela" in factor.tooltip


def test_an_unknown_symbol_is_refused(catalog):
    with pytest.raises(UnitSymbolError, match="not a symbol"):
        resolve_expression("xyz", catalog)


def test_more_than_one_slash_is_refused(catalog):
    with pytest.raises(UnitSymbolError, match="more than one"):
        resolve_expression("m/s/s", catalog)


def test_every_curated_unit_marker_is_a_real_marker_in_the_table(catalog):
    from eu_docs_ext.unit_info import UNIT_INFO

    known_markers = {entry.marker for entry in catalog.units.values()}
    known_markers |= {prefix_word_marker.rsplit(" ", 1)[-1] for prefix_word_marker in catalog.folded.values()}
    for marker in UNIT_INFO:
        assert marker in known_markers, f"{marker!r} is curated in unit_info.py but not in symbols.md"
