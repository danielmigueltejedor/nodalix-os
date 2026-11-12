#!/usr/bin/python3
"""Prepare the pinned sources of the Archivador package (Nautilus + the Nodalix layer).

Sources can be pre-fetched with --nautilus-source and --libgxdp-source. The Nautilus tree is
exported from the pinned commit with its libgxdp subproject, so the build needs no network.
"""
import argparse
import importlib.util
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
NAUTILUS_TAG = '51.1'
NAUTILUS_REV = '14015deb303ae645e411bd32ba485e71233e1164'
# Pinned by Nautilus 51.1's own subprojects/libgxdp.wrap.
LIBGXDP_REV = 'df896e3412b749947bc6f62a91a1aac8e6b6d19b'
VERSION = '51.1.0'

_spec = importlib.util.spec_from_file_location('native_builder', ROOT / 'tools/build-settings-packages.py')
_builder = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(_builder)


def main():
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument('--output', type=Path, default=ROOT / 'dist/native-archivador')
    ap.add_argument('--nautilus-source')
    ap.add_argument('--libgxdp-source')
    ap.add_argument('--prepare-only', action='store_true')
    args = ap.parse_args()
    out = args.output.resolve()
    out.mkdir(parents=True, exist_ok=True)
    cache = out / 'cache'
    cache.mkdir(exist_ok=True)
    nautilus = _builder.checkout(args.nautilus_source, cache, 'nautilus',
                                 'https://github.com/GNOME/nautilus.git', NAUTILUS_REV)
    libgxdp = _builder.checkout(args.libgxdp_source, cache, 'libgxdp',
                                'https://gitlab.gnome.org/GNOME/libgxdp.git', LIBGXDP_REV)
    wrap = (nautilus / 'subprojects/libgxdp.wrap').read_text()
    if f'revision = {LIBGXDP_REV}' not in wrap:
        raise SystemExit('Nautilus pins a different libgxdp: update LIBGXDP_REV in tools/build-archivador-package.py')
    with tempfile.TemporaryDirectory(dir=out) as temp:
        stage = Path(temp)
        _builder.export(nautilus, stage / 'nautilus')
        _builder.export(libgxdp, stage / 'nautilus/subprojects/libgxdp')
        source = out / f'nautilus-{NAUTILUS_TAG}.tar.zst'
        _builder.archive(stage, source, ['nautilus'])
        # The layer reads the shared NDL sheets from gnome/control-center/ndl: one copy for all apps.
        changes = out / f'nodalix-archivador-{VERSION}.tar.zst'
        _builder.archive(ROOT, changes, ['gnome/archivador', 'gnome/control-center/ndl'])
        folder = _builder.recipe('nodalix-archivador', out, [source, changes])
    if not args.prepare_only:
        _builder.run(['makepkg', '-f', '--nodeps', '--noconfirm'], cwd=folder)
    print(out)


if __name__ == '__main__':
    main()
