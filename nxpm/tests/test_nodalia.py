import hashlib
import json
from pathlib import Path

import pytest

from nxpm.applications import applications_dir, load_catalog
from nxpm.backends.nodalia import NodaliaBackend, load_entries, release_version
from nxpm.backends import nodalia as nodalia_module
from nxpm.config import load_config
from nxpm.engine import Engine
from nxpm.errors import NxpmError
from nxpm.model import parse_package_id

from test_applications import elf_with_update

APPIMAGE = elf_with_update("gh-releases-zsync|a|b|latest|X-*.AppImage.zsync")
DIGEST = hashlib.sha256(APPIMAGE).hexdigest()


class Body:
    def __init__(self, payload: bytes):
        self.payload = payload
        self.position = 0
        self.headers = {"Content-Length": str(len(payload))}

    def read(self, amount=-1):
        if self.position >= len(self.payload):
            return b""
        if amount is None or amount < 0:
            amount = len(self.payload) - self.position
        chunk = self.payload[self.position : self.position + amount]
        self.position += len(chunk)
        return chunk

    def __enter__(self):
        return self

    def __exit__(self, *_args):
        return False


class Opener:
    """A GitHub that publishes one release; ``sums`` and ``image`` can be swapped."""

    def __init__(self, tag="v0.3.0", image=APPIMAGE, sums=None, with_sums=True):
        self.tag = tag
        self.image = image
        self.sums = sums if sums is not None else f"{DIGEST}  Brujula-0.3.0-x86_64.AppImage\n".encode()
        self.with_sums = with_sums
        self.calls = []

    def open(self, request, timeout=30):
        url = request.full_url
        self.calls.append(url)
        if "api.github.com" in url:
            assets = [
                {
                    "name": "Brujula-0.3.0-x86_64.AppImage",
                    "size": len(self.image),
                    "browser_download_url": "https://github.com/getnodalia/brujula/releases/download/v0.3.0/Brujula-0.3.0-x86_64.AppImage",
                },
                {
                    "name": "Source.tar.gz",
                    "browser_download_url": "https://github.com/getnodalia/brujula/releases/download/v0.3.0/Source.tar.gz",
                },
            ]
            if self.with_sums:
                assets.append(
                    {
                        "name": "SHA256SUMS",
                        "browser_download_url": "https://github.com/getnodalia/brujula/releases/download/v0.3.0/SHA256SUMS",
                    }
                )
            return Body(json.dumps({"tag_name": self.tag, "assets": assets}).encode())
        if url.endswith("SHA256SUMS"):
            return Body(self.sums)
        return Body(self.image)


@pytest.fixture
def home(monkeypatch, tmp_path):
    monkeypatch.setenv("NXPM_APPLICATIONS_DIR", str(tmp_path / "Applications"))
    monkeypatch.setenv("XDG_STATE_HOME", str(tmp_path / "state"))
    monkeypatch.setenv("XDG_DATA_HOME", str(tmp_path / "data"))
    monkeypatch.setenv("NXPM_NODALIX_APPS_ROOT", str(tmp_path / "opt"))
    return tmp_path


@pytest.fixture
def catalog(home):
    path = home / "nodalia-apps.json"
    path.write_text(
        json.dumps(
            {
                "apps": [
                    {
                        "id": "brujula",
                        "name": "Brújula",
                        "summary": "Navegador",
                        "repo": "getnodalia/brujula",
                        "asset": "Brujula-*-x86_64.AppImage",
                        "checksums": "SHA256SUMS",
                        "icon": "dev.danielmiguelt.Brujula",
                    },
                    {
                        "id": "vinilo",
                        "name": "Vinilo",
                        "summary": "Música",
                        "kind": "nodalix-apps",
                        "manager": "vinilo",
                        "repo": "danielmigueltejedor/vinilo",
                    },
                    {"id": "Bad Slug", "repo": "x/y", "asset": "a.AppImage"},
                    {"id": "noasset", "repo": "x/y", "asset": "a.zip"},
                    {"id": "badrepo", "repo": "../etc", "asset": "a.AppImage"},
                    {"id": "badkind", "repo": "x/y", "kind": "shell", "asset": "a.AppImage"},
                ]
            }
        ),
        encoding="utf-8",
    )
    return path


def run(backend, package_id, operation):
    messages = []
    backend.apply(package_id, operation, lambda progress, message: messages.append(message))
    return messages


def test_shipped_catalogue_is_well_formed():
    shipped = Path(__file__).resolve().parents[1] / "share" / "nodalia-apps.json"
    data = json.loads(shipped.read_text(encoding="utf-8"))
    assert len(load_entries(shipped)) == len(data["apps"])


def test_ids_and_versions():
    assert parse_package_id("nodalia:brujula") == ("nodalia", "brujula")
    for bad in ("nodalia:Bad", "nodalia:../x", "nodalia:"):
        with pytest.raises(NxpmError):
            parse_package_id(bad)
    assert release_version("v0.3.0") == "0.3.0"
    assert release_version("2024.1") == "2024.1"


def test_malformed_entries_are_dropped(catalog):
    assert sorted(load_entries(catalog)) == ["brujula", "vinilo"]


def test_install_downloads_checks_and_registers_the_app(home, catalog):
    opener = Opener()
    backend = NodaliaBackend(catalog, opener)
    assert [p.id for p in backend.search("navegador")] == ["nodalia:brujula"]
    assert backend.list_installed() == []
    run(backend, "nodalia:brujula", "install")
    stored = applications_dir() / "brujula.AppImage"
    assert stored.read_bytes() == APPIMAGE
    assert stored.stat().st_mode & 0o111
    assert not (applications_dir() / "brujula.AppImage.partial").exists()
    menu = (home / "data" / "applications" / "os.nodalix.nxpm.brujula.desktop").read_text(encoding="utf-8")
    assert f'Exec="{stored}"' in menu
    assert "Icon=dev.danielmiguelt.Brujula" in menu
    record = load_catalog()["apps"]["brujula"]
    assert record["origin"] == "nodalia" and record["version"] == "0.3.0"
    (package,) = backend.list_installed()
    assert package.installed and package.version == "0.3.0" and package.update is False
    with pytest.raises(NxpmError):
        run(backend, "nodalia:brujula", "install")


def test_a_wrong_checksum_installs_nothing(home, catalog):
    backend = NodaliaBackend(catalog, Opener(sums=f"{'0' * 64}  Brujula-0.3.0-x86_64.AppImage\n".encode()))
    with pytest.raises(NxpmError) as error:
        run(backend, "nodalia:brujula", "install")
    assert "SHA-256" in error.value.message
    assert not list(applications_dir().iterdir())
    assert "brujula" not in load_catalog()["apps"]


def test_a_missing_checksum_file_installs_nothing(home, catalog):
    backend = NodaliaBackend(catalog, Opener(with_sums=False))
    with pytest.raises(NxpmError):
        run(backend, "nodalia:brujula", "install")
    assert not list(applications_dir().iterdir())


def test_a_file_that_is_not_an_appimage_is_refused(home, catalog):
    junk = b"not an appimage"
    sums = f"{hashlib.sha256(junk).hexdigest()}  Brujula-0.3.0-x86_64.AppImage\n".encode()
    backend = NodaliaBackend(catalog, Opener(image=junk, sums=sums))
    with pytest.raises(NxpmError):
        run(backend, "nodalia:brujula", "install")
    assert not list(applications_dir().iterdir())


def test_update_is_offered_when_the_release_is_newer_and_applies(home, catalog):
    backend = NodaliaBackend(catalog, Opener(tag="v0.3.0"))
    run(backend, "nodalia:brujula", "install")
    assert backend.list_updates() == []

    newer_image = elf_with_update("gh-releases-zsync|a|b|latest|Y-*.AppImage.zsync")
    newer = Opener(tag="v0.4.0", image=newer_image)
    newer.sums = f"{hashlib.sha256(newer_image).hexdigest()}  Brujula-0.3.0-x86_64.AppImage\n".encode()
    now = {"t": 10_000.0}
    backend = NodaliaBackend(catalog, newer, clock=lambda: now["t"])
    (update,) = backend.list_updates()
    assert update.id == "nodalia:brujula" and update.update is True
    assert update.version == "0.3.0" and update.available_version == "0.4.0"
    run(backend, "nodalia:brujula", "update")
    assert (applications_dir() / "brujula.AppImage").read_bytes() == newer_image
    assert load_catalog()["apps"]["brujula"]["version"] == "0.4.0"
    assert backend.list_updates() == []
    assert run(backend, "nodalia:brujula", "update") == ["Looking up the latest release", "Already up to date"]


def test_offline_update_check_is_quiet(home, catalog):
    run(NodaliaBackend(catalog, Opener()), "nodalia:brujula", "install")

    class Offline:
        def open(self, *_args, **_kwargs):
            raise OSError("no network")

    assert NodaliaBackend(catalog, Offline()).list_updates() == []
    with pytest.raises(NxpmError):
        run(NodaliaBackend(catalog, Offline()), "nodalia:brujula", "update")


def test_release_lookups_are_cached(home, catalog):
    opener = Opener()
    now = {"t": 0.0}
    backend = NodaliaBackend(catalog, opener, clock=lambda: now["t"])
    backend.details("nodalia:brujula")
    backend.details("nodalia:brujula")
    assert len([url for url in opener.calls if "api.github.com" in url]) == 1
    now["t"] = nodalia_module.CACHE_SECONDS + 1
    backend.details("nodalia:brujula")
    assert len([url for url in opener.calls if "api.github.com" in url]) == 2


def test_remove_deletes_the_file_and_the_menu_entry(home, catalog):
    backend = NodaliaBackend(catalog, Opener())
    run(backend, "nodalia:brujula", "install")
    run(backend, "nodalia:brujula", "remove")
    assert not list(applications_dir().iterdir())
    assert not (home / "data" / "applications" / "os.nodalix.nxpm.brujula.desktop").exists()
    assert "brujula" not in load_catalog()["apps"]
    with pytest.raises(NxpmError):
        run(backend, "nodalia:brujula", "remove")


def test_nodalia_apps_do_not_show_up_as_plain_appimages(home, catalog):
    from nxpm.backends.appimage import AppImageBackend

    run(NodaliaBackend(catalog, Opener()), "nodalia:brujula", "install")
    assert AppImageBackend().list_installed() == []


def test_a_managed_app_is_read_from_its_folder_and_updated_by_nodalix_apps(home, catalog, monkeypatch):
    backend = NodaliaBackend(catalog, Opener(tag="v1.2.0"))
    (vinilo,) = [p for p in backend.list_catalog() if p.id == "nodalia:vinilo"]
    assert vinilo.installed is False

    versions = home / "opt" / "vinilo" / "versions"
    (versions / "1.1.0").mkdir(parents=True)
    (versions / "1.1.0" / "version").write_text("1.1.0\n", encoding="utf-8")
    (home / "opt" / "vinilo" / "current").symlink_to(versions / "1.1.0")

    (update,) = backend.list_updates()
    assert update.id == "nodalia:vinilo" and update.version == "1.1.0" and update.available_version == "1.2.0"

    commands = []

    class Done:
        returncode = 0
        stdout = ""
        stderr = ""

    def fake_run(argv, **_kwargs):
        commands.append(argv)
        return Done()

    monkeypatch.setattr(nodalia_module.subprocess, "run", fake_run)
    run(backend, "nodalia:vinilo", "update")
    assert commands == [["nodalix-apps", "update", "vinilo"]]
    with pytest.raises(NxpmError) as removal:
        run(backend, "nodalia:vinilo", "remove")
    assert removal.value.code == "unsupported"
    with pytest.raises(NxpmError):
        run(backend, "nodalia:vinilo", "install")


def test_a_failing_nodalix_apps_reports_its_last_line(home, catalog, monkeypatch):
    backend = NodaliaBackend(catalog, Opener(tag="v1.2.0"))

    class Failed:
        returncode = 1
        stdout = ""
        stderr = "starting\nNo pude descargar la release\n"

    monkeypatch.setattr(nodalia_module.subprocess, "run", lambda *_a, **_k: Failed())
    with pytest.raises(NxpmError) as error:
        run(backend, "nodalia:vinilo", "install")
    assert error.value.message == "No pude descargar la release"


def test_the_engine_installs_and_lists_the_catalogue(home, catalog):
    engine = Engine(load_config(Path("/does/not/exist")), {"nodalia": NodaliaBackend(catalog, Opener())})
    assert [p.id for p in engine.list_catalog("nodalia")] == ["nodalia:brujula", "nodalia:vinilo"]
    record = engine.install("nodalia:brujula", "nodalia").record()
    assert record.package_id == "nodalia:brujula" and record.state == "succeeded"
    assert load_catalog()["apps"]["brujula"]["version"] == "0.3.0"
    with pytest.raises(NxpmError):
        engine.install("nodalia:brujula", "flatpak")
    with pytest.raises(NxpmError):
        engine.list_catalog("flatpak")
