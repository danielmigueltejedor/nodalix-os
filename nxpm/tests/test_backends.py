from nxpm.backends.flatpak import package_from_ref
from nxpm.backends.pacman import PacmanBackend


class Ref:
    def __init__(self, formatted: str):
        self.formatted = formatted

    def format_ref(self):
        return self.formatted

    def get_name(self):
        return self.formatted.split("/")[1]

    def get_appdata_name(self):
        return "Calculator"

    def get_appdata_summary(self):
        return "Do sums"

    def get_appdata_version(self):
        return "47"


def test_flatpak_ref_becomes_a_stable_id():
    package = package_from_ref("user", Ref("app/org.gnome.Calculator/x86_64/stable"), installed=True, update=True)
    assert package.id == "flatpak:user:app/org.gnome.Calculator/x86_64/stable"
    assert package.appstream_id == "org.gnome.Calculator"
    assert package.kind == "app"
    assert package.update is True


class Pkg:
    def __init__(self, name, version, desc=""):
        self.name = name
        self.version = version
        self.desc = desc
        self.isize = 10
        self.size = 10


class Database:
    def __init__(self, name, packages):
        self.name = name
        self.pkgcache = packages

    def get_pkg(self, name):
        return next((pkg for pkg in self.pkgcache if pkg.name == name), None)

    def search(self, query):
        return [pkg for pkg in self.pkgcache if query in pkg.name or query in pkg.desc]


class Handle:
    def __init__(self):
        self.ignorepkgs = ["ignored"]
        self.local = Database("local", [Pkg("bash", "1.0", "shell"), Pkg("ignored", "1", "skip")])
        self.sync = [Database("core", [Pkg("bash", "2.0", "shell"), Pkg("ignored", "2", "skip"), Pkg("zsh", "1", "other")])]

    def get_localdb(self):
        return self.local

    def get_syncdbs(self):
        return self.sync


def fake_pyalpm(monkeypatch):
    """Provide the one pyalpm function the backend needs, on any host."""
    import sys
    import types

    def vercmp(left, right):
        key = lambda value: [int(part) if part.isdigit() else 0 for part in value.split(".")]
        return (key(left) > key(right)) - (key(left) < key(right))

    module = types.ModuleType("pyalpm")
    module.vercmp = vercmp
    monkeypatch.setitem(sys.modules, "pyalpm", module)


def test_pacman_updates_compare_versions_and_honor_ignore(monkeypatch):
    fake_pyalpm(monkeypatch)
    backend = PacmanBackend(handle_factory=Handle, appstream_ids={"bash": "org.gnu.Bash"})
    updates = backend.list_updates()
    assert [item.id for item in updates] == ["pacman:bash"]
    assert updates[0].available_version == "2.0"
    assert updates[0].appstream_id == "org.gnu.Bash"
    found = backend.search("zsh")
    assert any(item.id == "pacman:zsh" and item.installed is False for item in found)
