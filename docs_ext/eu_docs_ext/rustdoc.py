"""Rust code blocks written for rustdoc, shown by the site.

The book's pages run as doctests (src/book.rs), so a block that must not compile is fenced as rustdoc wants it,
`rust,compile_fail`. Python-Markdown's fences don't take that info string; this extension rewrites it before they
read the page: the block is highlighted as Rust and gets the class `eu-no-compile`, which docs/stylesheets/book.css
labels "doesn't compile". Other rustdoc attributes are not used in the book and are refused, so a page can't
silently show a block that rustdoc skips or treats differently.
"""

import re

from markdown import Extension
from markdown.preprocessors import Preprocessor

_FENCE = re.compile(r"^(?P<indent>\s*)(?P<fence>`{3,}|~{3,})rust,(?P<attrs>[\w,]+)\s*$")


class RustdocFences(Preprocessor):
    def run(self, lines):
        out = []
        for n, line in enumerate(lines, 1):
            m = _FENCE.match(line)
            if m:
                attrs = m.group("attrs").split(",")
                if attrs != ["compile_fail"]:
                    raise ValueError(f"line {n}: rust fence with {attrs}, the book uses only rust,compile_fail")
                line = f"{m.group('indent')}{m.group('fence')}{{.rust .eu-no-compile}}"
            out.append(line)
        return out


class RustdocExtension(Extension):
    def extendMarkdown(self, md):
        # before pymdownx.superfences reads the fences (its preprocessor runs at priority 32)
        md.preprocessors.register(RustdocFences(md), "eu_rustdoc_fences", 40)


def makeExtension(**kwargs):
    return RustdocExtension(**kwargs)
