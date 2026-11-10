"""The `[[symbol]]` markup (eu_docs_ext.unit_markup)."""

import markdown
import pytest

from eu_docs_ext.unit_catalog import UnitSymbolError


def html(text):
    return markdown.markdown(text, extensions=["eu_docs_ext.unit_markup", "tables"])


def test_a_bare_symbol_is_a_tooltipped_link():
    out = html("Activity in [[Bq]].")
    assert '<a class="eu-unit" href="/start/symbols/#sym-Bq" title=' in out
    assert "becquerel" in out and "activity" in out
    assert ">Bq</a>" in out


def test_a_compound_keeps_each_factor_linked():
    out = html("[[mSv/h]]")
    assert 'href="/start/symbols/#sym-Sv"' in out
    assert 'href="/start/symbols/#sym-h"' in out
    assert ">mSv</a>" in out and ">h</a>" in out
    assert "/" in out


def test_an_exponent_stays_outside_the_link():
    out = html("[[m/s²]]")
    assert ">m</a>" in out
    assert "</a>²" in out or "²<" in out


def test_an_unknown_symbol_fails_the_build():
    with pytest.raises(UnitSymbolError, match="not a symbol"):
        html("[[xyz]]")


def test_an_ambiguous_symbol_fails_the_build():
    with pytest.raises(UnitSymbolError, match="reads more than one way"):
        html("[[min]]")


def test_symbols_md_table_rows_get_an_id():
    page = (
        "| Symbol | Marker | Feature | Prefixes |\n"
        "|---|---|---|---|\n"
        "| `Bq` | `Becquerel` | `radiation` | SI |\n"
    )
    out = html(page)
    assert 'id="sym-Bq"' in out


def test_an_unrelated_table_gets_no_id():
    page = "| Name | Value |\n|---|---|\n| a | 1 |\n"
    out = html(page)
    assert "id=" not in out


def test_an_id_strips_the_module_path_even_through_inlinehilite():
    # pymdownx.inlinehilite stashes a backtick span's HTML and only substitutes it back during Markdown's own
    # raw-HTML postprocessing, after every treeprocessor: a treeprocessor here would slug the stash placeholder,
    # not "origins::Celsius". Reproduces the real book's extension set (zensical.toml), not just "tables".
    page = (
        "| Origin | Feature | Written bare | Written in quotes |\n"
        "|---|---|---|---|\n"
        '| `origins::Celsius` | `thermal` | `degC` | `"°C"` |\n'
    )
    out = markdown.markdown(page, extensions=["eu_docs_ext.unit_markup", "tables", "pymdownx.inlinehilite"])
    assert 'id="sym-Celsius"' in out
