"""The built book's search index (site/search.json): a macro's name is a term of its own. Each item's title and text
are split with the separator zensical.toml gives the site's search, as the search does. Needs a built site, so it
runs from `just docs-search` (and CI's book job after `just docs-build`), with EU_SITE naming it."""

import html
import json
import os
import re
import tomli
from collections import Counter
from pathlib import Path

import pytest

ROOT = Path(__file__).parents[2]
SITE = os.environ.get("EU_SITE")
MACROS = ["fixed!", "q!", "db!", "np!", "unit!", "scale!", "origin!", "niches!", "counted!"]

with (ROOT / "zensical.toml").open("rb") as f:
    # tomli, as Zensical reads it: zensical.toml is TOML 1.1 (multi-line inline tables), which tomllib refuses
    SEPARATOR = tomli.load(f)["project"]["plugins"]["search"]["separator"]


def terms(text: str) -> list[str]:
    """`text` (HTML, as search.json holds it) split into lower-case terms."""
    plain = html.unescape(re.sub(r"<[^>]*>", " ", text)).lower()
    return [t for t in re.split(SEPARATOR, plain) if t]


def test_bang_stays_in_a_term():
    assert terms("<code>db!(6)</code>") == ["db!", "6"]
    assert terms("q!(100 m/s) and fixed!(0.5)") == ["q!", "100", "m", "s", "and", "fixed!", "0.5"]


def test_not_equal_still_splits():
    assert terms("a != b") == ["a", "b"]
    assert terms("x!=y") == ["x", "y"]


needs_site = pytest.mark.skipif(not SITE, reason="needs a built site: EU_SITE=site, see `just docs-search`")


@pytest.fixture(scope="module")
def pages() -> dict[str, Counter]:
    """Each page of the built site (`levels/decibels/`) and the terms of its sections."""
    index = json.loads((Path(SITE) / "search.json").read_text())
    assert index["config"]["separator"] == SEPARATOR, "the site was built with another separator"
    out: dict[str, Counter] = {}
    for item in index["items"]:
        page = item["location"].split("#", 1)[0]
        out.setdefault(page, Counter()).update(terms(item["title"]) + terms(item["text"]))
    return out


@pytest.fixture(scope="module")
def plain() -> str:
    """All titles and texts of the built site as plain lower-case text."""
    index = json.loads((Path(SITE) / "search.json").read_text())
    parts = (part for item in index["items"] for part in (item["title"], item["text"]))
    return " ".join(html.unescape(re.sub(r"<[^>]*>", " ", part)).lower() for part in parts)


@needs_site
@pytest.mark.parametrize(
    ("page", "macros"),
    [
        ("levels/decibels/", ["db!", "np!"]),
        ("start/literals/", ["q!", "fixed!"]),
        ("start/macros/", ["db!", "np!", "q!", "fixed!"]),
    ],
)
def test_macros_are_terms_on_their_pages(pages, page, macros):
    for macro in macros:
        assert pages[page][macro] > 0, f"{macro} is not a term of {page}"


@needs_site
def test_db_alone_no_longer_swallows_db_bang(pages, plain):
    # each `db!` written in the book is a db! term, so none of them adds to the `db` (the unit dB) a query for db! hit
    written = len(re.findall(r"(?<![\w!])db!(?!=)", plain))
    assert written > 10
    assert sum(p["db!"] for p in pages.values()) == written


@needs_site
def test_macros_page_names_every_macro(pages):
    macros = pages["start/macros/"]
    assert macros["macro"] + macros["macros"] > 0
    for macro in MACROS:
        assert macros[macro] > 0, f"{macro} is not on the Macros page"
