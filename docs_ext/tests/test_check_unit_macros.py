"""The `\\unit{}` build-time check (eu_docs_ext.check_unit_macros)."""

from eu_docs_ext.check_unit_macros import bad_macros


def test_a_good_macro_is_silent(tmp_path):
    (tmp_path / "a.html").write_text(r"<p>\(v = 3\times10^8\ \unit{m/s}\)</p>")
    assert list(bad_macros(tmp_path)) == []


def test_an_unknown_macro_is_reported(tmp_path):
    (tmp_path / "a.html").write_text(r"<p>\(\unit{xyz}\)</p>")
    [(page, arg, reason)] = bad_macros(tmp_path)
    assert page == tmp_path / "a.html"
    assert arg == "xyz"
    assert "not a symbol" in reason


def test_an_ambiguous_macro_is_reported(tmp_path):
    (tmp_path / "a.html").write_text(r"<p>\(\unit{min}\)</p>")
    [(_page, arg, reason)] = bad_macros(tmp_path)
    assert arg == "min"
    assert "reads more than one way" in reason
