"""Rust code blocks written for rustdoc, shown by the site.

The book's pages run as doctests (src/book.rs), so a block that must not compile is fenced as rustdoc wants it,
`rust,compile_fail`. Python-Markdown's fences don't take that info string; this extension rewrites it before they
read the page: the block is highlighted as Rust and gets the class `eu-no-compile`, which docs/stylesheets/book.css
labels "doesn't compile". Other rustdoc attributes are not used in the book and are refused, so a page can't
silently show a block that rustdoc skips or treats differently.

A fence rustdoc runs as Rust (bare ```rust, ```rust,compile_fail, or a bare ``` with no language at all - rustdoc's
default) may use rustdoc's own hidden-line convention ("Hiding portions of the example" in the rustdoc book): a
line is part of the doctest but not shown to the reader when, after stripping leading whitespace, it is `#` alone
or starts with `# `; a line starting with `##` is shown with one `#` removed (so a visible line that itself starts
with `#` can be written without being hidden). rustdoc strips these when it renders the example; Python-Markdown
does not, so this extension does the same stripping while rewriting the page, leaving the doctest itself (the
fenced text `cargo test --doc` sees) untouched.
"""

import re

from markdown import Extension
from markdown.preprocessors import Preprocessor

_FENCE_OPEN = re.compile(r"^(?P<indent>\s*)(?P<fence>`{3,}|~{3,})(?P<info>.*)$")
_FENCE_CLOSE = re.compile(r"^\s*(?P<fence>`{3,}|~{3,})\s*$")


def _unhide(line: str) -> str | None:
    stripped = line.lstrip(" \t")
    indent = line[: len(line) - len(stripped)]
    if stripped.rstrip() == "#":
        return None
    if stripped.startswith("# "):
        return None
    if stripped.startswith("##"):
        return indent + stripped[1:]
    return line


class RustdocFences(Preprocessor):
    def run(self, lines):
        out = []
        in_fence = False
        is_rust = False
        fence_char = ""
        fence_len = 0
        for n, line in enumerate(lines, 1):
            if in_fence:
                close = _FENCE_CLOSE.match(line)
                if close and close.group("fence")[0] == fence_char and len(close.group("fence")) >= fence_len:
                    in_fence = False
                    out.append(line)
                    continue
                if is_rust:
                    shown = _unhide(line)
                    if shown is not None:
                        out.append(shown)
                else:
                    out.append(line)
                continue

            m = _FENCE_OPEN.match(line)
            if m:
                info = m.group("info").strip()
                parts = [p.strip() for p in info.split(",")] if info else []
                is_rust = not parts or parts[0] == "rust"
                attrs = parts[1:] if is_rust and parts else []
                if is_rust:
                    if attrs and attrs != ["compile_fail"]:
                        raise ValueError(f"line {n}: rust fence with {attrs}, the book uses only rust,compile_fail")
                    if attrs == ["compile_fail"]:
                        line = f"{m.group('indent')}{m.group('fence')}{{.rust .eu-no-compile}}"
                in_fence = True
                fence_char = m.group("fence")[0]
                fence_len = len(m.group("fence"))
            out.append(line)
        return out


class RustdocExtension(Extension):
    def extendMarkdown(self, md):
        # before pymdownx.superfences reads the fences (its preprocessor runs at priority 32)
        md.preprocessors.register(RustdocFences(md), "eu_rustdoc_fences", 40)


def makeExtension(**kwargs):
    return RustdocExtension(**kwargs)
