"""The rustdoc fences of the book (eu_docs_ext.rustdoc)."""

import re

import markdown
import pytest

_TAG = re.compile(r"<[^>]+>")


def html(text):
    return markdown.markdown(text, extensions=["eu_docs_ext.rustdoc", "pymdownx.superfences", "attr_list"])


def code_text(out):
    """The shown code, with pygments' per-token spans stripped back to plain text."""
    return _TAG.sub("", out)


def test_compile_fail_is_rust_with_a_class():
    out = html("```rust,compile_fail\nlet a: u8 = \"x\";\n```\n")
    assert "eu-no-compile" in out
    assert "```" not in out


def test_other_rustdoc_attributes_are_refused():
    with pytest.raises(ValueError, match="only rust,compile_fail"):
        html("```rust,ignore\nlet a = 1;\n```\n")


def test_hidden_line_is_dropped():
    out = code_text(html("```rust\n# #[cfg(feature = \"angle\")] {\nlet a = 1;\n# }\n```\n"))
    assert "cfg(feature" not in out
    assert "let a = 1;" in out


def test_bare_hash_line_is_dropped():
    out = code_text(html("```rust\nlet a = 1;\n#\nlet b = 2;\n```\n"))
    assert "let a = 1;" in out and "let b = 2;" in out


def test_doubled_hash_shows_one_hash():
    out = code_text(html("```rust\n## this stays, minus one #\nlet a = 1;\n```\n"))
    assert "# this stays, minus one #" in out
    assert "## this stays" not in out


def test_derive_attribute_is_not_hidden():
    out = html("```rust\n#[derive(Debug)]\nstruct A;\n```\n")
    assert "derive(Debug)" in out


def test_compile_fail_hidden_line_is_dropped_too():
    out = html("```rust,compile_fail\n# #[cfg(feature = \"angle\")] {\nlet a: u8 = \"x\";\n# }\n```\n")
    assert "cfg(feature" not in out
    assert "eu-no-compile" in out
