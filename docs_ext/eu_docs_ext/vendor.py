"""Puts KaTeX into docs/vendor/katex and IBM Plex Sans/Mono into docs/vendor/fonts, so the site serves its own
copies and needs no CDN (no Google Fonts, either: zensical.toml turns that off).

Each release is pinned with a hash; the download is checked against it before anything is written. The fonts come
from IBM's own released woff2 (the IBM/plex GitHub releases), unmodified apart from npm not publishing a hash for
GitHub release assets, so FONT_*_SHA256 is a plain sha256 of the zip this script downloaded, pinned by hand.
docs/vendor/ is not in git: `just docs` and `just docs-build` run this first, and it does nothing when the pinned
version is there already.

Every fetch goes through `download()`: a local cache outside the repo, content-addressed by the pinned hash
(`$EU_VENDOR_CACHE`, default `$XDG_CACHE_HOME/exact_units/book-vendor`, itself defaulting to `~/.cache/...`), so a
warm cache needs no network at all, and a timeout, a connection error or an HTTP 5xx/429 from npm, GitHub or
Google's font subsetter is retried with backoff instead of failing the gate outright. Any other HTTP error, or a
hash mismatch (cached or freshly fetched), fails at once.

    uv run python -m eu_docs_ext.vendor [DOCS_DIR]
"""

import base64
import hashlib
import io
import os
import sys
import tarfile
import tempfile
import time
import urllib.error
import urllib.request
import zipfile
from pathlib import Path

VERSION = "0.19.1"
TARBALL = f"https://registry.npmjs.org/katex/-/katex-{VERSION}.tgz"
INTEGRITY = "sha512-jG8uKntjSEcm2EpQOh0XturpPuuC9r3Sq5HLHdsFdWZT27E672SF19QEi+of0/YeOy15lDgRhKvr5614uRrEfA=="
# what the site loads (docs/javascripts/katex.js, zensical.toml) and the fonts the CSS points at
KEEP = ("dist/katex.min.js", "dist/katex.min.css", "dist/contrib/auto-render.min.js", "dist/fonts/", "LICENSE")

# the weights book.css's @font-face rules declare (start/why.md and friends don't need more)
FONT_SANS_VERSION = "1.1.0"
FONT_SANS_URL = f"https://github.com/IBM/plex/releases/download/%40ibm/plex-sans%40{FONT_SANS_VERSION}/ibm-plex-sans.zip"
FONT_SANS_SHA256 = "fb365d910566e6d199cc2c15579a7dd9a267128e18431a394ed81f1970c69200"
FONT_SANS_WEIGHTS = ("Light", "LightItalic", "Regular", "Italic", "Medium", "MediumItalic", "Bold", "BoldItalic")

FONT_MONO_VERSION = "2.5.0"
FONT_MONO_URL = f"https://github.com/IBM/plex/releases/download/%40ibm/plex-mono%40{FONT_MONO_VERSION}/ibm-plex-mono.zip"
FONT_MONO_SHA256 = "6d23f01257663d8cc49a0d64c22ced630b79e0e2a0ac08a0da86e9a38bbc481c"
FONT_MONO_WEIGHTS = ("Regular", "Italic", "Bold", "BoldItalic")

# Neither Plex face has U+2071 or U+207A-207F (the superscript +, -, =, (, ), n that U+2070-2079's digits lack):
# Noto Sans / Noto Sans Mono do, at a matching height. The URLs below are Google's own on-demand subsetter
# (css2's `text=`, with those seven codepoints) cutting a woff2 of just those glyphs; fetched once here instead
# of at every page load.
FALLBACK_SANS_URL = (
    "https://fonts.gstatic.com/l/font?kit=o-0mIpQlx3QUlC5A4PNB6Ryti20_6n1iPHjcz6L1SoM-jCpoiyD9A99Y-0IQTNK7Tw"
    "lMdyB37aqBaTkKmg9EcSE&skey=2b960fe17823056f&v=v42"
)
FALLBACK_SANS_SHA256 = "de49ba0896cab5349c2d5133dd2a7019f2988d9e456d4793ce1424459ac75f03"
FALLBACK_MONO_URL = (
    "https://fonts.gstatic.com/l/font?kit=BngrUXNETWXI6LwhGYvaxZikqZqK6fBq6kPvUce2oAZcdthSBUsYck4-_FNJ49_PRa6"
    "QhiWsixuLlH1PMpMfxX39FDGIEQ&skey=41eaca710d6906c9&v=v37"
)
FALLBACK_MONO_SHA256 = "7b28f5333f4f0d334e32c758ec05a412ce1060c4f2bf6ba87e3d948bf44fa64f"

# U+264F (the minim's symbol, catalogue.rs) renders as the Scorpio emoji in some browsers even with the U+FE0E
# text-presentation selector appended (Firefox and older WebKit ignore it): Noto Sans Symbols has a plain text
# glyph for it (checked with fontTools: no COLR/CPAL/SVG table, one outline contour), so docs/customary.md's
# `<code>` cell (IBM Plex Mono) gets it from there instead of whatever emoji font the browser would fall back to.
FALLBACK_MINIM_URL = (
    "https://fonts.gstatic.com/l/font?kit=rP2up3q65FkAtHfwd-eIS2brbDN6gxP34F9jRRCe4W3gfQ8gb-1CrNR8"
    "&skey=e59169fb45ece077&v=v47"
)
FALLBACK_MINIM_SHA256 = "5a00cc5c87afe8ae4bbf0da86f47d63c4b79a957a282df34b4966428617238ea"
FALLBACK_LICENSE_URL = "https://raw.githubusercontent.com/notofonts/notofonts.github.io/main/fonts/LICENSE"
FALLBACK_LICENSE_SHA256 = "f2095b08bed08b23a6fe26112fcd679a2bee3f002eef077eb05d215ed1051bd8"

# A timeout, a connection error and these statuses are worth retrying (a 503 from GitHub, a 429 from the font
# subsetter); any other HTTP status is treated as permanent (a 404 means the pinned URL moved, not a glitch).
RETRYABLE_STATUSES = {429, 500, 502, 503, 504}
RETRY_ATTEMPTS = 4
RETRY_BASE_SECONDS = 1.0


def cache_dir() -> Path:
    """Where `download()` keeps fetched files, content-addressed by their pinned hash. `EU_VENDOR_CACHE` names the
    directory outright; otherwise it is `$XDG_CACHE_HOME/exact_units/book-vendor`, with `~/.cache` when
    `XDG_CACHE_HOME` isn't set either."""
    override = os.environ.get("EU_VENDOR_CACHE")
    if override:
        return Path(override)
    base = os.environ.get("XDG_CACHE_HOME")
    return (Path(base) if base else Path.home() / ".cache") / "exact_units" / "book-vendor"


def _write_atomic(path: Path, data: bytes) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    fd, tmp_name = tempfile.mkstemp(dir=path.parent, prefix=f".{path.name}.")
    try:
        with os.fdopen(fd, "wb") as f:
            f.write(data)
        os.replace(tmp_name, path)
    except Exception:
        Path(tmp_name).unlink(missing_ok=True)
        raise


def _fetch(url: str) -> bytes:
    """`url`'s bytes, retrying a timeout, a connection error or one of `RETRYABLE_STATUSES` with backoff. Any other
    HTTP error exits at once: it means the pinned URL itself is wrong, not a glitch worth waiting out."""
    last: Exception | None = None
    for attempt in range(RETRY_ATTEMPTS):
        try:
            with urllib.request.urlopen(url, timeout=60) as r:
                return r.read()
        except urllib.error.HTTPError as e:
            if e.code not in RETRYABLE_STATUSES:
                sys.exit(f"{url}: HTTP {e.code}, not retrying; check the URL or the pin")
            last = e
        except (urllib.error.URLError, TimeoutError, ConnectionError) as e:
            last = e
        if attempt < RETRY_ATTEMPTS - 1:
            time.sleep(RETRY_BASE_SECONDS * 2**attempt)
    sys.exit(f"{url}: still failing after {RETRY_ATTEMPTS} attempts ({last}); check network access and retry")


def download(url: str, algo: str, want: str) -> bytes:
    """`url`'s bytes, verified against `want` (a hex digest under `algo`, a `hashlib` name). A cached copy whose
    hash already matches is returned with no network; otherwise `url` is fetched (`_fetch`, with retries), checked
    against `want`, cached atomically and returned. A cached file with the wrong hash is ignored and refetched; a
    freshly fetched one with the wrong hash exits without being cached."""
    path = cache_dir() / f"{algo}-{want}"
    if path.exists():
        cached = path.read_bytes()
        if hashlib.new(algo, cached).hexdigest() == want:
            return cached
    data = _fetch(url)
    got = hashlib.new(algo, data).hexdigest()
    if got != want:
        sys.exit(f"{url}: the download's {algo} is {got}, the pinned one {want}")
    _write_atomic(path, data)
    return data


def fetch(docs: Path) -> None:
    out = docs / "vendor" / "katex"
    stamp = out / "VERSION"
    if stamp.exists() and stamp.read_text().strip() == VERSION:
        return
    algo, want_b64 = INTEGRITY.split("-", 1)
    data = download(TARBALL, algo, base64.b64decode(want_b64).hex())
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


def fetch_fonts(docs: Path) -> None:
    out = docs / "vendor" / "fonts"
    stamp = out / "VERSION"
    want_stamp = (
        f"sans-{FONT_SANS_VERSION} mono-{FONT_MONO_VERSION} "
        f"fallback-{FALLBACK_SANS_SHA256[:8]} minim-{FALLBACK_MINIM_SHA256[:8]}"
    )
    if stamp.exists() and stamp.read_text().strip() == want_stamp:
        return
    out.mkdir(parents=True, exist_ok=True)
    for family, prefix, url, sha256, weights in (
        ("ibm-plex-sans", "IBMPlexSans", FONT_SANS_URL, FONT_SANS_SHA256, FONT_SANS_WEIGHTS),
        ("ibm-plex-mono", "IBMPlexMono", FONT_MONO_URL, FONT_MONO_SHA256, FONT_MONO_WEIGHTS),
    ):
        data = download(url, "sha256", sha256)
        with zipfile.ZipFile(io.BytesIO(data)) as z:
            (out / "LICENSE.txt").write_bytes(z.read(f"{family}/LICENSE.txt"))
            for weight in weights:
                name = f"{prefix}-{weight}.woff2"
                data_font = z.read(f"{family}/fonts/complete/woff2/{name}")
                (out / name).write_bytes(data_font)
    for name, url, sha256 in (
        ("NotoSans-Superscripts.woff2", FALLBACK_SANS_URL, FALLBACK_SANS_SHA256),
        ("NotoSansMono-Superscripts.woff2", FALLBACK_MONO_URL, FALLBACK_MONO_SHA256),
        ("NotoSansSymbols-Minim.woff2", FALLBACK_MINIM_URL, FALLBACK_MINIM_SHA256),
        ("LICENSE-noto.txt", FALLBACK_LICENSE_URL, FALLBACK_LICENSE_SHA256),
    ):
        data = download(url, "sha256", sha256)
        (out / name).write_bytes(data)
    stamp.write_text(want_stamp + "\n")


if __name__ == "__main__":
    docs_dir = Path(sys.argv[1] if len(sys.argv) > 1 else "docs")
    fetch(docs_dir)
    fetch_fonts(docs_dir)
