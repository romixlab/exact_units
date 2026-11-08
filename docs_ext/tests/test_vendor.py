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
