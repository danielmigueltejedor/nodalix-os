"""Apply an AppImage update when the file itself names a verifiable channel.

GitHub release channels can be downloaded over HTTPS and checked for the ELF
header before the old file is replaced. Plain zsync channels are reported and
left untouched: NXPM does not guess how to reconstruct them.
"""

from __future__ import annotations

import hashlib
import json
import re
import urllib.request
from pathlib import Path
from urllib.parse import urlparse

from nxpm.errors import NxpmError

_GITHUB = re.compile(
    r"^gh-releases-zsync\|([A-Za-z0-9._-]+)\|([A-Za-z0-9._-]+)\|([A-Za-z0-9._-]+)\|([A-Za-z0-9._*+-]+)$"
)
_ALLOWED_HOSTS = {
    "github.com",
    "release-assets.githubusercontent.com",
    "objects.githubusercontent.com",
    "github-releases.githubusercontent.com",
}
_MAX_DOWNLOAD = 8 * 1024 * 1024 * 1024


def describe_channel(information: str) -> str:
    text = (information or "").strip()
    if _GITHUB.match(text):
        return "github"
    if text.startswith("zsync|https://"):
        return "zsync"
    return ""


def apply_update(path: Path, information: str, opener=None) -> str:
    channel = describe_channel(information)
    if channel == "zsync":
        raise NxpmError(
            "unsupported",
            "This AppImage uses zsync. NXPM will not replace it without a verified updater.",
        )
    if channel != "github":
        raise NxpmError("unsupported", "This AppImage does not publish an update channel NXPM can verify.")
    owner, repo, _release, pattern = _GITHUB.match(information.strip()).groups()
    payload = _download_release_asset(owner, repo, pattern, opener or _opener())
    if not payload.startswith(b"\x7fELF"):
        raise NxpmError("failed", "The published file is not an AppImage NXPM can install.")
    current = _sha256(path)
    if hashlib.sha256(payload).hexdigest() == current:
        return "The published AppImage matches the installed file."
    temporary = path.with_name(path.name + ".partial")
    temporary.write_bytes(payload)
    temporary.chmod(0o755)
    temporary.replace(path)
    path.chmod(0o755)
    return "The AppImage was replaced with the file published on GitHub."


def _download_release_asset(owner: str, repo: str, pattern: str, opener) -> bytes:
    request = urllib.request.Request(
        f"https://api.github.com/repos/{owner}/{repo}/releases/latest",
        headers={"Accept": "application/vnd.github+json", "User-Agent": "NXPM/0.1"},
    )
    try:
        with opener.open(request, timeout=30) as response:
            payload = json.loads(response.read().decode("utf-8"))
    except (OSError, json.JSONDecodeError, UnicodeError) as exc:
        raise NxpmError("failed", "The GitHub release could not be read.") from exc
    matches = []
    for asset in payload.get("assets") or []:
        if not isinstance(asset, dict):
            continue
        name = str(asset.get("name") or "")
        link = str(asset.get("browser_download_url") or "")
        if _name_matches(pattern, name) and _https_asset(link):
            matches.append(link)
    if len(matches) != 1:
        raise NxpmError("failed", "The release does not contain one matching AppImage.")
    download = urllib.request.Request(matches[0], headers={"User-Agent": "NXPM/0.1"})
    try:
        with opener.open(download, timeout=60) as response:
            return _read_limited(response)
    except OSError as exc:
        raise NxpmError("failed", "The published AppImage could not be downloaded.") from exc


def _sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def _name_matches(pattern: str, name: str) -> bool:
    import fnmatch

    if not name.endswith(".AppImage"):
        return False
    candidate = pattern[: -len(".zsync")] if pattern.endswith(".zsync") else pattern
    return fnmatch.fnmatch(name, candidate)


def _https_asset(url: str) -> bool:
    parsed = urlparse(url)
    return parsed.scheme == "https" and parsed.hostname in _ALLOWED_HOSTS


def _read_limited(response) -> bytes:
    chunks = []
    total = 0
    while True:
        block = response.read(1024 * 1024)
        if not block:
            break
        total += len(block)
        if total > _MAX_DOWNLOAD:
            raise NxpmError("failed", "The published AppImage is larger than NXPM will download.")
        chunks.append(block)
    return b"".join(chunks)


def _opener():
    class Redirect(urllib.request.HTTPRedirectHandler):
        def redirect_request(self, req, fp, code, msg, headers, newurl):
            if not str(newurl).startswith("https://"):
                raise NxpmError("failed", "The download left HTTPS.")
            return super().redirect_request(req, fp, code, msg, headers, newurl)

    return urllib.request.build_opener(Redirect)
