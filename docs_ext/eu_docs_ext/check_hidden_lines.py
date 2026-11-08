"""Guards against a rustdoc hidden line reaching the built site: eu_docs_ext.rustdoc strips them from Rust fences
before the Markdown is rendered, but a page outside its reach (a fence it doesn't recognise, a future bug in it)
would otherwise show `# #[cfg(..)] {` or a lone `# }` to the reader. Scans only the blocks pymdownx.highlight
rendered as Rust (`pygments_lang_class = true` in zensical.toml puts `language-rust` first in the wrapping div's
class, as rustdoc itself would run these and no others), with their HTML tags stripped back to the line text
rustdoc would have hidden, for a line that is `#` alone or starts with `# ` after its own leading whitespace - the
same rule eu_docs_ext.rustdoc applies when it builds the page. A `# comment` in a `sh` or `toml` block, or a lone
`#` in an inline `` `#` `` span, is someone else's fence and is left alone.

    uv run python -m eu_docs_ext.check_hidden_lines [SITE_DIR]
"""

import html
import re
import sys
from pathlib import Path

_CODE_BLOCK = re.compile(r'<div class="language-rust\b[^"]*">.*?<code>(.*?)</code>', re.DOTALL)
_TAG = re.compile(r"<[^>]+>")


def hidden_lines(site: Path):
    for page in sorted(site.rglob("*.html")):
        for block in _CODE_BLOCK.findall(page.read_text()):
            content = html.unescape(_TAG.sub("", block))
            for n, line in enumerate(content.split("\n"), 1):
                stripped = line.strip()
                if stripped == "#" or stripped.startswith("# "):
                    yield page, n, line


def main(argv: list[str]) -> None:
    site = Path(argv[1] if len(argv) > 1 else "site")
    found = list(hidden_lines(site))
    for page, n, line in found:
        print(f"{page}: code block line {n}: {line!r}", file=sys.stderr)
    if found:
        sys.exit(f"{len(found)} rustdoc hidden line(s) leaked into {site}")


if __name__ == "__main__":
    main(sys.argv)
