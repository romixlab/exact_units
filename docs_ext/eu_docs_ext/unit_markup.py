"""The `[[symbol]]` markup: a unit symbol in prose gets its own style (upright, a subtle tint, `.eu-unit` in
docs/stylesheets/book.css), a tooltip with its name and quantity, and a link to its row in docs/start/symbols.md.
`[[keV]]`, `[[mSv/h]]` and `[[J/kg]]` are read the way `q!` reads them (eu_docs_ext.unit_catalog); an unknown or
ambiguous symbol is a `UnitSymbolError`, uncaught, which fails `zensical build` the same way a bad `rust,ignore`
fence fails it in eu_docs_ext.rustdoc.

Each row of docs/start/symbols.md's generated tables gets an `id` (`SymbolAnchors`, matching the rendered table
by its header text alone, so it runs on whichever page has it) so a `[[..]]` elsewhere in the book can link
straight to it, with no change to the Rust generator that writes the page.
"""

from __future__ import annotations

import html
import re
import xml.etree.ElementTree as etree

from markdown import Extension
from markdown.inlinepatterns import InlineProcessor
from markdown.postprocessors import Postprocessor

from eu_docs_ext.unit_catalog import Factor, load_catalog, resolve_expression, slugify_row

_PATTERN = r"\[\[([^\[\]]+)\]\]"

# The book is mounted at the site root (zensical.toml's site_url); an absolute path always finds
# docs/start/symbols.md, with no per-page relative-path bookkeeping.
SYMBOLS_PAGE_HREF = "/start/symbols/"


class UnitSymbol(InlineProcessor):
    def handleMatch(self, m, data):
        parts = resolve_expression(m.group(1), load_catalog())
        span = etree.Element("span", {"class": "eu-unit-expr"})
        current: etree.Element | None = None
        for kind, value in parts:
            if kind == "factor":
                assert isinstance(value, Factor)
                current = etree.SubElement(
                    span,
                    "a",
                    {"class": "eu-unit", "href": f"{SYMBOLS_PAGE_HREF}#{value.anchor}", "title": value.tooltip},
                )
                current.text = value.text
            elif current is None:
                span.text = (span.text or "") + value
            else:
                current.tail = (current.tail or "") + value
        return span, m.start(0), m.end(0)


_TAG = re.compile(r"<[^>]+>")
_TABLE = re.compile(
    r'(<table>\s*<thead>\s*<tr>\s*<th>(?:Symbol|Origin)</th>.*?</thead>\s*<tbody>)(.*?)(</tbody>\s*</table>)',
    re.DOTALL,
)
_ROW = re.compile(r"(<tr>\s*<td>)(.*?)(</td>)", re.DOTALL)


class SymbolAnchors(Postprocessor):
    """Gives every row of a `Symbol`/`Origin` table (docs/start/symbols.md's) an id from its first cell, so
    `UnitSymbol`'s links resolve. Matches by the header text alone, not the page, so it needs no page context.

    A postprocessor, not a treeprocessor: pymdownx.inlinehilite stashes a backtick span's HTML (so Pygments'
    markup survives further Markdown processing untouched) and only substitutes it back in during Markdown's own
    raw-HTML postprocessing, which runs after every treeprocessor. A treeprocessor here would see the stash
    placeholder, not the symbol's text, and slug it on that instead.
    """

    def run(self, text: str) -> str:
        def tag_table(m: "re.Match[str]") -> str:
            def tag_row(rm: "re.Match[str]") -> str:
                cell, close_td = rm.group(2), rm.group(3)
                slug = slugify_row(html.unescape(_TAG.sub("", cell)))
                if not slug:
                    return rm.group(0)
                return f'<tr id="sym-{slug}">\n<td>{cell}{close_td}'

            return m.group(1) + _ROW.sub(tag_row, m.group(2)) + m.group(3)

        return _TABLE.sub(tag_table, text)


class UnitMarkupExtension(Extension):
    def extendMarkdown(self, md):
        md.inlinePatterns.register(UnitSymbol(_PATTERN, md), "eu_unit_symbol", 175)
        md.postprocessors.register(SymbolAnchors(md), "eu_symbol_anchors", 5)


def makeExtension(**kwargs):
    return UnitMarkupExtension(**kwargs)
