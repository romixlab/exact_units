"""The emoji check (eu_docs_ext.check_emoji), on pages rendered with the book's own emoji setup (zensical.toml)."""

import markdown
from zensical.extensions.emoji import twemoji

from eu_docs_ext import emoji
from eu_docs_ext.check_emoji import stray_emoji

# zensical.toml's dotted names, which Zensical resolves before it hands them to pymdownx.emoji
_EMOJI = {"pymdownx.emoji": {"emoji_generator": emoji.to_svg, "emoji_index": twemoji}}


def page(tmp_path, text):
    html = markdown.markdown(text, extensions=["pymdownx.emoji", "pymdownx.inlinehilite"], extension_configs=_EMOJI)
    (tmp_path / "index.html").write_text(html)
    return html


def test_a_time_code_in_prose_is_a_stray_flag(tmp_path):
    html = page(tmp_path, "A time code hh:mm:ss;ff.")
    assert 'class="twemoji"' in html
    [(found, shortcode)] = stray_emoji(tmp_path)
    assert found == tmp_path / "index.html"
    assert shortcode == ":mm:"


def test_a_time_code_in_code_style_is_silent(tmp_path):
    html = page(tmp_path, "A time code `hh:mm:ss;ff`.")
    assert "twemoji" not in html
    assert list(stray_emoji(tmp_path)) == []


def test_an_allowed_icon_is_silent(tmp_path):
    html = page(tmp_path, "Careful :material-alert: here, and :smile: there.")
    assert 'data-emoji=":material-alert:"' in html
    [(_page, shortcode)] = stray_emoji(tmp_path, allowed=frozenset({":material-alert:"}))
    assert shortcode == ":smile:"
