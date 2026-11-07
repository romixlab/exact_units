"""The built book's search (eu_docs_ext.search): a query for a macro finds the page about it first. Needs a built
site, so it runs from `just docs-search` (and CI's book job after `just docs-build`), with EU_SITE naming it."""

import os
from pathlib import Path

import pytest

from eu_docs_ext.search import Search

SITE = os.environ.get("EU_SITE")
pytestmark = pytest.mark.skipif(not SITE, reason="needs a built site: EU_SITE=site, see `just docs-search`")


@pytest.fixture(scope="module")
def search():
    return Search(Path(SITE))


# Each query and the page or section its first result is on. `!` is not a separator (zensical.toml), so `db!` is a
# term of its own and not every "dB" of the book.
FIRST = [
    ("db!", "levels/decibels/#db-a-level-as-a-constant"),
    ("np!", "levels/decibels/#neper"),
    ("q!", "start/literals/#q"),
    ("fixed!", "start/literals/#fixed"),
    ("macro", "start/macros/"),
    ("macros", "start/macros/"),
]


@pytest.mark.parametrize(("query", "location"), FIRST)
def test_first_result(search, query, location):
    hits = search.query(query)
    assert hits, f"{query}: no results"
    assert hits[0].location == location, f"{query}: {[h.location for h in hits]}"


@pytest.mark.parametrize(("query", "page"), [("db", "levels/decibels/"), ("fix", "start/literals/")])
def test_a_prefix_still_finds_the_macro(search, query, page):
    assert search.rank(query, page) == 1
