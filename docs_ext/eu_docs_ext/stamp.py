"""Writes the build stamp the book's footer shows (overrides/partials/copyright.html includes it): the crate's
version, the git commit the site was built from (`-dirty` when the tree had changes) and the build time in UTC.
overrides/partials/stamp.html is not in git: `just docs` and `just docs-build` run this first.

    uv run python -m eu_docs_ext.stamp [REPO_DIR]
"""

import html
import subprocess
import sys
import tomllib
from datetime import UTC, datetime
from pathlib import Path


def version(repo: Path) -> str:
    """The crate's version, from Cargo.toml."""
    with (repo / "Cargo.toml").open("rb") as f:
        return tomllib.load(f)["package"]["version"]


def commit(repo: Path) -> str:
    """The short hash of HEAD, with `-dirty` when a tracked or untracked file differs from it."""

    def git(*args: str) -> str:
        return subprocess.run(["git", *args], cwd=repo, check=True, capture_output=True, text=True).stdout.strip()

    short = git("rev-parse", "--short", "HEAD")
    return f"{short}-dirty" if git("status", "--porcelain") else short


def stamp(repo: Path, now: datetime) -> str:
    """The footer line, as HTML."""
    when = now.astimezone(UTC).strftime("%Y-%m-%d %H:%M UTC")
    text = f"exact_units {version(repo)} · {commit(repo)} · built {when}"
    return f'<div class="eu-stamp">{html.escape(text)}</div>\n'


def main(argv: list[str]) -> None:
    repo = Path(argv[0]) if argv else Path()
    out = repo / "overrides/partials/stamp.html"
    out.write_text(stamp(repo, datetime.now(UTC)))
    print(f"{out}: {out.read_text().strip()}")


if __name__ == "__main__":
    main(sys.argv[1:])
