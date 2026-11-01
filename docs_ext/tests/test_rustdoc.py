"""The rustdoc fences of the book (eu_docs_ext.rustdoc)."""

import markdown
import pytest


def html(text):
    return markdown.markdown(text, extensions=["eu_docs_ext.rustdoc", "pymdownx.superfences", "attr_list"])


def test_compile_fail_is_rust_with_a_class():
    out = html("```rust,compile_fail\nlet a: u8 = \"x\";\n```\n")
    assert "eu-no-compile" in out
    assert "```" not in out


def test_other_rustdoc_attributes_are_refused():
    with pytest.raises(ValueError, match="only rust,compile_fail"):
        html("```rust,ignore\nlet a = 1;\n```\n")
