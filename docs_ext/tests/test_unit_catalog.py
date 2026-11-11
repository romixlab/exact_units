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


def test_every_marker_of_the_table_has_a_generated_entry(catalog):
    # unit_info.json is generated (just q-symbols): every marker q!'s table reads has an entry there
    from eu_docs_ext.unit_info import LEVEL_INFO, ORIGIN_INFO, UNIT_INFO

    markers = {entry.marker for entry in catalog.units.values()}
    markers |= {read_as.rsplit(" ", 1)[-1] for read_as in catalog.folded.values()}
    for marker in markers:
        assert marker in UNIT_INFO, f"{marker!r} of symbols.md has no entry in unit_info.json"
    for origin in set(catalog.origins.values()):
        assert origin in ORIGIN_INFO, origin
    for level in set(catalog.levels.values()):
        assert level in LEVEL_INFO, level


def test_unit_info_is_not_curated_by_hand():
    # the module reads the generated file and holds no entries of its own
    import eu_docs_ext.unit_info as unit_info

    source = open(unit_info.__file__, encoding="utf-8").read()
    assert 'Info("' not in source and 'PrefixEntry("' not in source


def test_tooltip_is_the_generated_entry_line_by_line(catalog):
    (_, becquerel), = resolve_expression("kBq", catalog)
    lines = becquerel.tooltip.split("\n")
    assert lines[0] == "kilo- becquerel · activity"
    assert lines[1].startswith("Radioactivity in becquerels")
    assert lines[-1] == "Related: Ci"


def test_tooltip_writes_a_long_exact_scale_as_a_power_of_ten(catalog):
    (_, ev), = resolve_expression("eV", catalog)
    assert "1 eV = 1.602176634·10⁻¹⁹ kg·m²·s⁻²" in ev.tooltip.split("\n")


def test_a_marker_the_source_names_no_quantity_of_shows_less(catalog):
    (_, item), = resolve_expression("item", catalog)
    assert item.tooltip.split("\n")[0] == "item"
