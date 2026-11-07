"""The footer's build stamp (eu_docs_ext.stamp)."""

import subprocess
from datetime import UTC, datetime, timedelta, timezone

from eu_docs_ext.stamp import stamp


def git(repo, *args):
    subprocess.run(["git", "-c", "user.name=t", "-c", "user.email=t@t", *args], cwd=repo, check=True, capture_output=True)


def test_version_commit_and_utc_time(tmp_path):
    (tmp_path / "Cargo.toml").write_text('[package]\nname = "exact_units"\nversion = "1.2.3"\n')
    git(tmp_path, "init", "-q")
    git(tmp_path, "add", "Cargo.toml")
    git(tmp_path, "commit", "-qm", "x")
    short = subprocess.run(["git", "rev-parse", "--short", "HEAD"], cwd=tmp_path, capture_output=True, text=True)
    now = datetime(2026, 11, 7, 19, 5, tzinfo=timezone(timedelta(hours=2)))
    out = stamp(tmp_path, now)
    assert out == f'<div class="eu-stamp">exact_units 1.2.3 · {short.stdout.strip()} · built 2026-11-07 17:05 UTC</div>\n'

    (tmp_path / "new.md").write_text("x")
    assert "-dirty · built" in stamp(tmp_path, datetime(2026, 1, 1, tzinfo=UTC))
