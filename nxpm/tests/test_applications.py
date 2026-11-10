import io
import json
import stat
import struct
import tarfile
from pathlib import Path

import pytest

from nxpm.applications import applications_dir, load_catalog
from nxpm.backends.appimage import AppImageBackend
from nxpm.elfinfo import update_information
from nxpm.engine import Engine
from nxpm.errors import NxpmError
from nxpm.foreign import extract_deb
from nxpm.permissions import classify, parse_metadata, reset_override
from nxpm.updates_appimage import apply_update


def elf_with_update(text: str) -> bytes:
    info = text.encode() + b"\x00"
    names = b"\x00.upd_info\x00.shstrtab\x00"
    info_offset = 64
    names_offset = info_offset + len(info)
    table_offset = names_offset + len(names)

    def header(name: int, offset: int, size: int) -> bytes:
        return struct.pack("<IIQQQQIIQQ", name, 1, 0, 0, offset, size, 0, 0, 1, 0)

    sections = bytes(64) + header(1, info_offset, len(info)) + header(11, names_offset, len(names))
    prefix = bytearray(64)
    prefix[0:4] = b"\x7fELF"
    prefix[4] = 2
    prefix[5] = 1
    prefix[6] = 1
    # AppImage Type-2 magic lives at offset 8.
    prefix[8:11] = b"AI\x02"
    struct.pack_into("<Q", prefix, 40, table_offset)
    struct.pack_into("<HHH", prefix, 58, 64, 3, 2)
    return bytes(prefix) + info + names + sections


def ar_archive(members: dict[str, bytes]) -> bytes:
    blob = bytearray(b"!<arch>\n")
    for name, payload in members.items():
        header = f"{name:<16}{'0':<12}{'0':<6}{'0':<6}{'100644':<8}{len(payload):<10}`\n"
        assert len(header) == 60
        blob.extend(header.encode())
        blob.extend(payload)
        if len(payload) % 2:
            blob.append(10)
    return bytes(blob)


def tar_bytes(files: dict[str, tuple[bytes, int]]) -> bytes:
    stream = io.BytesIO()
    with tarfile.open(fileobj=stream, mode="w:gz") as archive:
        for name, (payload, mode) in files.items():
            info = tarfile.TarInfo(name)
            info.size = len(payload)
            info.mode = mode
            archive.addfile(info, io.BytesIO(payload))
    return stream.getvalue()


def debian_package(extra: dict | None = None) -> bytes:
    files = {
        "usr/share/applications/hello.desktop": (b"[Desktop Entry]\nName=Hello Nxpm\nExec=hello\n", 0o644),
        "usr/bin/hello": (b"#!/bin/sh\necho hi\n", 0o4755),
    }
    if extra:
        files.update(extra)
    control = tar_bytes({"postinst": (b"#!/bin/sh\ntouch nxpm-should-not-run\n", 0o755)})
    return ar_archive(
        {
            "debian-binary": b"2.0\n",
            "control.tar.gz": control,
            "data.tar.gz": tar_bytes(files),
        }
    )


@pytest.fixture
def home(monkeypatch, tmp_path):
    monkeypatch.setenv("NXPM_APPLICATIONS_DIR", str(tmp_path / "Applications"))
    monkeypatch.setenv("XDG_STATE_HOME", str(tmp_path / "state"))
    monkeypatch.setenv("XDG_DATA_HOME", str(tmp_path / "data"))
    return tmp_path


def test_update_section_is_read_without_running_the_file(tmp_path):
    path = tmp_path / "Demo.AppImage"
    path.write_bytes(elf_with_update("gh-releases-zsync|owner|repo|latest|Demo-*.AppImage.zsync"))
    assert update_information(path).startswith("gh-releases-zsync|")


def test_appimage_import_registers_a_menu_entry(home):
    source = home / "Demo.AppImage"
    source.write_bytes(elf_with_update("gh-releases-zsync|owner|repo|latest|Demo-*.AppImage.zsync"))
    package = AppImageBackend().import_file("appimage", str(source), lambda *_args: None)
    assert package.id == "appimage:Demo"
    assert package.update is True
    assert source.is_file()
    stored = applications_dir() / "Demo.AppImage"
    assert stored.is_file()
    assert stored.stat().st_mode & 0o111
    menu = home / "data" / "applications" / "os.nodalix.nxpm.Demo.desktop"
    assert f'Exec="{stored}"' in menu.read_text(encoding="utf-8")
    assert AppImageBackend().list_updates()[0].id == "appimage:Demo"


def test_deb_import_does_not_run_scripts_or_keep_setuid(home):
    package_file = home / "hello.deb"
    package_file.write_bytes(debian_package())
    imported = AppImageBackend().import_file("deb", str(package_file), lambda *_args: None)
    assert imported.id == "appimage:hello"
    folder = applications_dir() / "hello.AppDir"
    program = folder / "usr" / "bin" / "hello"
    assert program.is_file()
    assert not (program.stat().st_mode & (stat.S_ISUID | stat.S_ISGID))
    assert not list(folder.rglob("postinst"))
    assert not (home / "nxpm-should-not-run").exists()
    assert "exec \"$HERE/usr/bin/hello\"" in (folder / "AppRun").read_text(encoding="utf-8")
    menu = (home / "data" / "applications" / "os.nodalix.nxpm.hello.desktop").read_text(encoding="utf-8")
    assert "AppRun" in menu


def test_deb_paths_cannot_escape_the_bundle(home):
    package_file = home / "escape.deb"
    package_file.write_bytes(
        debian_package({"../outside": (b"escaped\n", 0o644)})
    )
    with pytest.raises(NxpmError):
        extract_deb(package_file, home / "Applications" / "escape.AppDir")
    assert not (home / "outside").exists()


def test_github_update_replaces_only_an_elf(home):
    installed = applications_dir() / "Demo.AppImage"
    installed.write_bytes(elf_with_update("old"))
    published = elf_with_update("new")

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
        def open(self, request, timeout=30):
            if "api.github.com" in request.full_url:
                body = json.dumps(
                    {
                        "assets": [
                            {
                                "name": "Demo-2-x86_64.AppImage",
                                "browser_download_url": "https://github.com/owner/repo/releases/download/2/Demo-2-x86_64.AppImage",
                            }
                        ]
                    }
                ).encode()
                return Body(body)
            return Body(published)

    message = apply_update(
        installed,
        "gh-releases-zsync|owner|repo|latest|Demo-*-x86_64.AppImage.zsync",
        Opener(),
    )
    assert "replaced" in message
    assert installed.read_bytes() == published

    with pytest.raises(NxpmError) as zsync:
        apply_update(installed, "zsync|https://example.com/app.zsync", Opener())
    assert zsync.value.code == "unsupported"


def test_flatpak_permission_reset_removes_only_the_named_override(tmp_path):
    rows = parse_metadata("[Context]\nfilesystems=host;xdg-download;\ndevices=dri;\n", "declared")
    assert classify("filesystems", "host") == "high"
    assert any(row["value"] == "host" and row["risk"] == "high" for row in rows)
    assert any(row["value"] == "dri" and row["risk"] == "low" for row in rows)
    overrides = tmp_path / "overrides"
    overrides.mkdir()
    (overrides / "org.example.App").write_text("[Context]\nfilesystems=home;\n", encoding="utf-8")
    assert reset_override("org.example.App", overrides) is True
    assert not (overrides / "org.example.App").exists()
    with pytest.raises(NxpmError):
        reset_override("../org.example.App", overrides)


def test_engine_import_and_remove_use_the_applications_folder(home):
    source = home / "Demo.AppImage"
    source.write_bytes(elf_with_update("gh-releases-zsync|owner|repo|latest|Demo-*.AppImage.zsync"))
    from nxpm.config import load_config

    engine = Engine(load_config(Path("/does/not/exist")), {"appimage": AppImageBackend()})
    imported = engine.import_file("appimage", str(source))
    assert imported.state == "succeeded"
    removed = engine.remove("appimage:Demo", "appimage")
    assert removed.state == "succeeded"
    assert load_catalog()["apps"] == {}
    assert not (applications_dir() / "Demo.AppImage").exists()


def test_appimage_import_rejects_elf_without_type2_marker(home):
    source = home / "Plain.AppImage"
    payload = bytearray(elf_with_update("gh-releases-zsync|owner|repo|latest|Demo-*.AppImage.zsync"))
    payload[8:11] = b"\x00\x00\x00"
    source.write_bytes(payload)
    with pytest.raises(NxpmError) as error:
        AppImageBackend().import_file("appimage", str(source), lambda *_args: None)
    assert error.value.code == "invalid-input"


def test_catalog_write_is_atomic_and_quoted_exec(home):
    from nxpm.applications import catalog_path, desktop_exec_escape, update_catalog, write_desktop

    update_catalog(lambda data: data["apps"].__setitem__("Demo", {"name": "Demo"}))
    path = catalog_path()
    assert path.is_file()
    assert not path.with_name(path.name + ".tmp").exists()
    assert '"Demo"' in path.read_text(encoding="utf-8")

    target = applications_dir() / "Demo App.AppImage"
    target.write_bytes(elf_with_update("none"))
    menu = write_desktop("Demo", "Demo", target, "comment")
    text = menu.read_text(encoding="utf-8")
    assert 'Exec="' in text
    assert "Demo App.AppImage" in text
    assert desktop_exec_escape('/tmp/a b%$`"\\c') == '"/tmp/a b%%\\$\\`\\"\\\\c"'
    assert desktop_exec_escape("/opt/O'Brien/app") == '"/opt/O\'Brien/app"'
    assert desktop_exec_escape("/opt/app;rm") == '"/opt/app;rm"'


def test_engine_refuses_pacman_work_while_updater_lock_is_held(home, monkeypatch):
    import fcntl
    import os

    from nxpm.config import load_config

    lock = home / "update.lock"
    lock.write_text("busy\n", encoding="utf-8")
    monkeypatch.setenv("NXPM_UPDATER_LOCK", str(lock))
    monkeypatch.setenv("NXPM_PACMAN_LOCK", str(home / "missing-db.lck"))
    fd = os.open(lock, os.O_RDWR)
    fcntl.flock(fd, fcntl.LOCK_EX | fcntl.LOCK_NB)

    class DummyPacman:
        def upgrade_system(self, progress):
            raise AssertionError("pacman upgrade must not run while the updater lock exists")

    try:
        engine = Engine(load_config(Path("/does/not/exist")), {"pacman": DummyPacman()})
        transaction = engine.upgrade_system()
        assert transaction.state == "failed"
        assert "already applying" in (transaction.error or transaction.message).lower()
    finally:
        fcntl.flock(fd, fcntl.LOCK_UN)
        os.close(fd)
