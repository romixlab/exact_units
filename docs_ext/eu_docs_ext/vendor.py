"""Puts KaTeX into docs/vendor/katex, so the site serves its own copy and needs no CDN.

The release is pinned with its npm integrity hash; the download is checked against it before anything is written.
docs/vendor/ is not in git: `just docs` and `just docs-build` run this first, and it does nothing when the pinned
version is there already.

    uv run python -m eu_docs_ext.vendor [DOCS_DIR]
"""

import base64
import hashlib
import io
import sys
import tarfile
import urllib.request
from pathlib import Path

VERSION = "0.19.1"
TARBALL = f"https://registry.npmjs.org/katex/-/katex-{VERSION}.tgz"
INTEGRITY = "sha512-jG8uKntjSEcm2EpQOh0XturpPuuC9r3Sq5HLHdsFdWZT27E672SF19QEi+of0/YeOy15lDgRhKvr5614uRrEfA=="
# what the site loads (docs/javascripts/katex.js, zensical.toml) and the fonts the CSS points at
KEEP = ("dist/katex.min.js", "dist/katex.min.css", "dist/contrib/auto-render.min.js", "dist/fonts/", "LICENSE")


def fetch(docs: Path) -> None:
    out = docs / "vendor" / "katex"
    stamp = out / "VERSION"
    if stamp.exists() and stamp.read_text().strip() == VERSION:
        return
    with urllib.request.urlopen(TARBALL, timeout=60) as r:
        data = r.read()
    algo, want = INTEGRITY.split("-", 1)
    got = base64.b64encode(hashlib.new(algo, data).digest()).decode()
    if got != want:
        sys.exit(f"katex {VERSION}: the download's {algo} is {got}, the pinned one {want}")
    with tarfile.open(fileobj=io.BytesIO(data), mode="r:gz") as tar:
        for member in tar.getmembers():
            name = member.name.removeprefix("package/")
            if not member.isfile() or not name.startswith(KEEP):
                continue
            # every browser the theme supports reads woff2: the .ttf and .woff copies stay out
            if name.startswith("dist/fonts/") and not name.endswith(".woff2"):
                continue
            target = out / name.removeprefix("dist/")
            target.parent.mkdir(parents=True, exist_ok=True)
            target.write_bytes(tar.extractfile(member).read())
    stamp.write_text(VERSION + "\n")


if __name__ == "__main__":
    fetch(Path(sys.argv[1] if len(sys.argv) > 1 else "docs"))
