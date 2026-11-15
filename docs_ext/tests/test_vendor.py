"""The download cache and retries of eu_docs_ext.vendor (no real network: urlopen is monkeypatched throughout)."""

import hashlib
import io
import urllib.error

import pytest

from eu_docs_ext import vendor

URL = "https://example.invalid/thing"
DATA = b"the pinned bytes"
SHA256 = hashlib.sha256(DATA).hexdigest()
WRONG = hashlib.sha256(b"not it").hexdigest()


@pytest.fixture(autouse=True)
def _cache(tmp_path, monkeypatch):
    monkeypatch.setattr(vendor, "cache_dir", lambda: tmp_path)
    monkeypatch.setattr(vendor.time, "sleep", lambda _seconds: None)
    return tmp_path


def _ok_response(data: bytes):
    return io.BytesIO(data)


def test_cache_hit_uses_no_network(monkeypatch, _cache):
    (_cache / f"sha256-{SHA256}").write_bytes(DATA)

    def fail(*_args, **_kwargs):
        raise AssertionError("should not touch the network on a cache hit")

    monkeypatch.setattr(vendor.urllib.request, "urlopen", fail)
    assert vendor.download(URL, "sha256", SHA256) == DATA


def test_bad_cached_file_is_refetched(monkeypatch, _cache):
    (_cache / f"sha256-{SHA256}").write_bytes(b"stale garbage")
    calls = []

    def urlopen(url, timeout):
        calls.append(url)
        return _ok_response(DATA)

    monkeypatch.setattr(vendor.urllib.request, "urlopen", urlopen)
    assert vendor.download(URL, "sha256", SHA256) == DATA
    assert calls == [URL]
    assert (_cache / f"sha256-{SHA256}").read_bytes() == DATA


def test_503_then_success_is_retried(monkeypatch, _cache):
    attempts = []

    def urlopen(url, timeout):
        attempts.append(url)
        if len(attempts) == 1:
            raise urllib.error.HTTPError(url, 503, "Service Unavailable", None, None)
        return _ok_response(DATA)

    monkeypatch.setattr(vendor.urllib.request, "urlopen", urlopen)
    assert vendor.download(URL, "sha256", SHA256) == DATA
    assert len(attempts) == 2


def test_404_fails_without_retries(monkeypatch, _cache):
    attempts = []

    def urlopen(url, timeout):
        attempts.append(url)
        raise urllib.error.HTTPError(url, 404, "Not Found", None, None)

    monkeypatch.setattr(vendor.urllib.request, "urlopen", urlopen)
    with pytest.raises(SystemExit):
        vendor.download(URL, "sha256", SHA256)
    assert len(attempts) == 1


def test_hash_mismatch_is_refused_and_not_cached(monkeypatch, _cache):
    def urlopen(url, timeout):
        return _ok_response(DATA)

    monkeypatch.setattr(vendor.urllib.request, "urlopen", urlopen)
    with pytest.raises(SystemExit):
        vendor.download(URL, "sha256", WRONG)
    assert not (_cache / f"sha256-{WRONG}").exists()


def test_exhausted_retries_exit(monkeypatch, _cache):
    attempts = []

    def urlopen(url, timeout):
        attempts.append(url)
        raise urllib.error.HTTPError(url, 503, "Service Unavailable", None, None)

    monkeypatch.setattr(vendor.urllib.request, "urlopen", urlopen)
    with pytest.raises(SystemExit):
        vendor.download(URL, "sha256", SHA256)
    assert len(attempts) == vendor.RETRY_ATTEMPTS


def _ww_files(contents: dict[str, bytes]):
    return tuple((name, f"assets/{name}", hashlib.sha256(data).hexdigest()) for name, data in contents.items())


WW_CONTENTS = {
    "ww_highlight.js": b"// the highlighter",
    "ww_highlight.css": b".highlight .ww-unit {}",
    "ww_tokens.json": b"{}",
    "LICENSE": b"MIT License",
}


def test_ww_highlight_pins_the_three_files_and_the_licence():
    assert len(vendor.WW_LSP_COMMIT) == 40
    assert vendor.WW_LSP_COMMIT in vendor.WW_LSP_RAW
    names = [name for name, _path, _sha256 in vendor.WW_HIGHLIGHT_FILES]
    assert names == ["ww_highlight.js", "ww_highlight.css", "ww_tokens.json", "LICENSE"]
    for name, path, sha256 in vendor.WW_HIGHLIGHT_FILES:
        assert len(sha256) == 64
        int(sha256, 16)
        assert path.endswith(name)


def test_ww_highlight_files_are_checked_and_written_beside_each_other(monkeypatch, tmp_path):
    monkeypatch.setattr(vendor, "WW_HIGHLIGHT_FILES", _ww_files(WW_CONTENTS))
    fetched = []

    def urlopen(url, timeout):
        fetched.append(url)
        return _ok_response(WW_CONTENTS[url.rsplit("/", 1)[1]])

    monkeypatch.setattr(vendor.urllib.request, "urlopen", urlopen)
    vendor.fetch_ww_highlight(tmp_path)
    out = tmp_path / "vendor" / "ww_highlight"
    for name, data in WW_CONTENTS.items():
        assert (out / name).read_bytes() == data
    assert (out / "VERSION").read_text().strip() == vendor.WW_LSP_COMMIT
    assert all(url.startswith(vendor.WW_LSP_RAW + "/") for url in fetched)
    assert len(fetched) == len(WW_CONTENTS)

    # pinned commit there already: nothing fetched again
    vendor.fetch_ww_highlight(tmp_path)
    assert len(fetched) == len(WW_CONTENTS)


def test_ww_highlight_file_with_a_wrong_hash_is_refused(monkeypatch, tmp_path):
    monkeypatch.setattr(vendor, "WW_HIGHLIGHT_FILES", _ww_files(WW_CONTENTS))

    def urlopen(url, timeout):
        return _ok_response(b"tampered")

    monkeypatch.setattr(vendor.urllib.request, "urlopen", urlopen)
    with pytest.raises(SystemExit):
        vendor.fetch_ww_highlight(tmp_path)
    assert not (tmp_path / "vendor" / "ww_highlight" / "VERSION").exists()
