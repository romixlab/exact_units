"""Guards against an emoji the book never meant: pymdownx.emoji turns any `:name:` of prose into a picture, so
`hh:mm:ss` becomes `hh`, the flag of Myanmar (`:mm:`) and `ss`. Scans the built site's HTML for every element the
emoji extension produced (`class="twemoji"`; eu_docs_ext.emoji keeps the shortcode as `data-emoji`) and fails on any
whose shortcode is not in `ALLOWED`. A colon pair in prose that is no emoji goes in code style (`` `hh:mm:ss` ``),
where the extension never looks.

    uv run python -m eu_docs_ext.check_emoji [SITE_DIR]
"""

import re
import sys
from pathlib import Path

# The emoji and icons the book's pages mean, by shortcode with its colons (`:material-alert:`). None today: a page
# that means one adds its shortcode here.
ALLOWED: frozenset[str] = frozenset()

_EMOJI = re.compile(r'<(?:img|span)\b[^>]*\bclass="twemoji\b[^"]*"[^>]*>')
_DATA = re.compile(r'\bdata-emoji="([^"]*)"')


def stray_emoji(site: Path, allowed: frozenset[str] = ALLOWED):
    for page in sorted(site.rglob("*.html")):
        for m in _EMOJI.finditer(page.read_text()):
            tag = m.group(0)
            data = _DATA.search(tag)
            # no `data-emoji`: drawn by some other generator, so not one the book means either
            shortcode = data.group(1) if data else tag
            if shortcode not in allowed:
                yield page, shortcode


def main(argv: list[str]) -> None:
    site = Path(argv[1] if len(argv) > 1 else "site")
    found = list(stray_emoji(site))
    for page, shortcode in found:
        print(f"{page}: emoji {shortcode}: not in eu_docs_ext.check_emoji.ALLOWED", file=sys.stderr)
    if found:
        sys.exit(f"{len(found)} emoji the book doesn't mean in {site}: write the text in code style")


if __name__ == "__main__":
    main(sys.argv)
