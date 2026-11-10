"""Guards the `\\unit{}` KaTeX macro (docs/javascripts/katex.js) the same way `[[..]]` guards prose: an unknown or
ambiguous argument fails the docs build. KaTeX renders `\\unit{}` client-side, so there is nothing for a Markdown
extension to check at parse time; this scans the built site's HTML instead (mirrors eu_docs_ext.check_hidden_lines),
where the macro's argument sits untouched as literal text inside pymdownx.arithmatex's `\\(..\\)` / `\\[..\\]` spans.

    uv run python -m eu_docs_ext.check_unit_macros [SITE_DIR]
"""

import re
import sys
from pathlib import Path

from eu_docs_ext.unit_catalog import UnitSymbolError, load_catalog, resolve_expression

_UNIT_MACRO = re.compile(r"\\unit\{([^{}]*)\}")


def bad_macros(site: Path):
    catalog = load_catalog()
    for page in sorted(site.rglob("*.html")):
        text = page.read_text()
        for m in _UNIT_MACRO.finditer(text):
            try:
                resolve_expression(m.group(1), catalog)
            except UnitSymbolError as e:
                yield page, m.group(1), str(e)


def main(argv: list[str]) -> None:
    site = Path(argv[1] if len(argv) > 1 else "site")
    found = list(bad_macros(site))
    for page, arg, reason in found:
        print(f"{page}: \\unit{{{arg}}}: {reason}", file=sys.stderr)
    if found:
        sys.exit(f"{len(found)} bad \\unit{{}} argument(s) in {site}")


if __name__ == "__main__":
    main(sys.argv)
