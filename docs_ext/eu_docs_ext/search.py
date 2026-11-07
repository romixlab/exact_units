"""Queries a built book's search index the way the site's search box does: the site's own search worker
(site/assets/javascripts/workers/search.*.min.js, the one Zensical ships), run in V8 through mini-racer, fed
site/search.json and the query object the page posts to it. The ranks it prints are the ranks a reader sees.

    uv run --group dev python -m eu_docs_ext.search [--site site] QUERY...
"""

import json
import sys
import time
from dataclasses import dataclass
from pathlib import Path

from py_mini_racer import MiniRacer

# What the page posts for a query typed in the box (Zensical UI's bundle.js: `{type: 2, data: query}`, with no
# tag selected)
EMPTY_FILTER = {
    "input": {"type": "operator", "data": {"operator": "and", "operands": []}},
    "aggregation": {"input": [{"type": "term", "data": {"field": "tags"}}]},
}


@dataclass
class Hit:
    """One result row: the section's location (`page/#anchor`) and title."""

    location: str
    title: str

    @property
    def page(self) -> str:
        """The page the section is on (`start/macros/`; `` for the home page)."""
        return self.location.split("#", 1)[0]


class Search:
    """The site's search worker over one built site."""

    def __init__(self, site: Path):
        workers = sorted((site / "assets/javascripts/workers").glob("search.*.min.js"))
        if len(workers) != 1:
            raise FileNotFoundError(f"one search worker in {site}/assets/javascripts/workers, found {workers}")
        index = (site / "search.json").read_text()
        self.items = json.loads(index)["items"]
        self.ctx = MiniRacer()
        self.ctx.eval("var out = []; var self = {postMessage: m => out.push(m)};")
        self.ctx.eval(workers[0].read_text())
        self.ctx.eval(f"self.onmessage({{data: {{type: 0, data: {index}}}}})")
        self._wait(1)

    def _wait(self, kind: int):
        # the worker answers from a promise; mini-racer runs its microtasks between evals
        for _ in range(200):
            out = json.loads(self.ctx.eval("JSON.stringify(out)"))
            if out:
                self.ctx.eval("out = []")
                if out[-1]["type"] != kind:
                    raise RuntimeError(f"search worker answered {out}")
                return out[-1].get("data")
            time.sleep(0.01)
        raise TimeoutError("search worker did not answer")

    def query(self, text: str) -> list[Hit]:
        """The first page of results (10 rows, as the site shows them) for `text`."""
        query = json.dumps({"input": text, "filter": EMPTY_FILTER})
        self.ctx.eval(f"self.onmessage({{data: {{type: 2, data: {query}}}}})")
        data = self._wait(3)
        return [Hit(self.items[i["id"]]["location"], self.items[i["id"]]["title"]) for i in data["items"]]

    def rank(self, text: str, page: str) -> int | None:
        """The 1-based row of the first result on `page`, None when it's not on the first page of results."""
        for row, hit in enumerate(self.query(text), 1):
            if hit.page == page:
                return row
        return None


def main(argv: list[str]) -> None:
    site = Path("site")
    if argv[:1] == ["--site"]:
        site, argv = Path(argv[1]), argv[2:]
    search = Search(site)
    for text in argv:
        print(f"{text}:")
        for row, hit in enumerate(search.query(text), 1):
            print(f"  {row:2} {hit.location}  {hit.title}")


if __name__ == "__main__":
    main(sys.argv[1:])
