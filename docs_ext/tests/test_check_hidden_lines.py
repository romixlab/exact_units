"""eu_docs_ext.check_hidden_lines: only a block pymdownx.highlight rendered as Rust is in scope."""

from eu_docs_ext.check_hidden_lines import hidden_lines

_RUST_BLOCK = (
    '<div class="language-rust highlight"><pre><span></span><code>'
    "<span>use foo;\n</span>"
    '<span><span class="p">#</span><span class="w"> </span><span class="p">}</span>\n</span>'
    "</code></pre></div>"
)

_SH_BLOCK = (
    '<div class="language-sh highlight"><pre><span></span><code>'
    "<span>cargo build\n</span>"
    '<span><span class="c1"># comment</span>\n</span>'
    "</code></pre></div>"
)


def test_a_rust_block_fails(tmp_path):
    (tmp_path / "page.html").write_text(f"<html><body>{_RUST_BLOCK}</body></html>")
    assert len(list(hidden_lines(tmp_path))) == 1


def test_a_sh_block_with_a_comment_passes(tmp_path):
    (tmp_path / "page.html").write_text(f"<html><body>{_SH_BLOCK}</body></html>")
    assert not list(hidden_lines(tmp_path))


def test_a_sh_block_beside_a_rust_block_only_flags_the_rust_one(tmp_path):
    (tmp_path / "page.html").write_text(f"<html><body>{_SH_BLOCK}{_RUST_BLOCK}</body></html>")
    assert len(list(hidden_lines(tmp_path))) == 1
