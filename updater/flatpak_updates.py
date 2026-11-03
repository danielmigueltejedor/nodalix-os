"""Installed-origin Flatpak updates; an unused broken remote cannot block others."""
import argparse
from concurrent.futures import ThreadPoolExecutor
import json
import subprocess
from urllib.parse import urlparse


def installations(scope):
    import gi
    gi.require_version('Flatpak', '1.0')
    from gi.repository import Flatpak
    result = []
    if scope in ('all', 'user'):
        result.append(Flatpak.Installation.new_user(None))
    if scope in ('all', 'system'):
        result.extend(Flatpak.get_system_installations(None))
    return result


def origin_groups(installation):
    installed = {}
    for ref in installation.list_installed_refs(None):
        installed.setdefault(ref.get_origin(), []).append(ref.format_ref())
    remotes = {r.get_name(): r for r in installation.list_remotes(None)}
    groups, manual = [], []
    for origin, refs in installed.items():
        remote = remotes.get(origin)
        if not remote or remote.get_disabled() or not remote.get_url():
            manual.extend(refs)
            continue
        url = remote.get_url()
        # A build cache removed after installing a local bundle is not a server.
        # Keep real file:// repositories with a summary available for offline use.
        parsed = urlparse(url)
        if parsed.scheme == 'file':
            from pathlib import Path
            from urllib.parse import unquote
            repo = Path(unquote(parsed.path))
            if not (repo/'summary').is_file() and not (repo/'summary.idx').is_file():
                manual.extend(refs)
                continue
        selector = '--user' if installation.get_is_user() else '--installation='+installation.get_id()
        groups.append((selector, origin, refs))
    return groups, manual


def query_group(group, runner=subprocess.run):
    selector, origin, refs = group
    try:
        result = runner(['/usr/bin/flatpak', 'remote-ls', selector, '--updates', '--all',
                         '--columns=ref', '--', origin], capture_output=True, text=True, timeout=90)
        if result.returncode:
            raise RuntimeError(result.stderr.strip() or 'El repositorio no respondió')
        # remote-ls omits app/runtime in some versions. Match the full remainder.
        available = set(result.stdout.splitlines())
        updates = [ref for ref in refs if ref in available or ref.split('/', 1)[1] in available]
        return {'scope': selector, 'origin': origin, 'updates': updates, 'error': ''}
    except (OSError, RuntimeError, subprocess.SubprocessError) as error:
        return {'scope': selector, 'origin': origin, 'updates': [], 'error': str(error)[:220]}


def check(scope='all'):
    groups, manual = [], []
    for installation in installations(scope):
        targets, local = origin_groups(installation)
        groups.extend(targets)
        manual.extend(local)
    with ThreadPoolExecutor(max_workers=4) as pool:
        origins = list(pool.map(query_group, groups))
    count = sum(len(r['updates']) for r in origins)
    failures = [r['origin'] for r in origins if r['error']]
    detail = f'{count} actualizaciones disponibles' if count else 'Aplicaciones y componentes Flatpak al día'
    if manual:
        detail += f' · {len(manual)} instalaciones locales sin repositorio de actualización'
    if failures:
        successful = any(not row['error'] for row in origins)
        detail = (f'{count} actualizaciones en los repositorios comprobados' if successful else 'No se pudieron comprobar las actualizaciones Flatpak')
        detail += ' · No se pudo consultar: '+', '.join(dict.fromkeys(failures))
    return {'detail': detail, 'count': count, 'manual': len(manual), 'origins': origins,
            'partial': bool(failures), 'error': bool(failures) and not any(not r['error'] for r in origins)}


def update(scope='all', runner=subprocess.run):
    state = check(scope)
    failed = state['partial']
    print(state['detail'], flush=True)
    # Explicit refs avoid refreshing unrelated/obsolete remotes or appstream.
    # Dependencies and related refs are still handled by Flatpak itself.
    for row in state['origins']:
        if not row['updates']:
            continue
        result = runner(['/usr/bin/flatpak', 'update', row['scope'], '--noninteractive', '-y', '--', *row['updates']])
        failed |= result.returncode != 0
    return 1 if failed else 0


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('action', choices=('check', 'update'))
    parser.add_argument('--scope', choices=('all', 'user', 'system'), default='all')
    args = parser.parse_args()
    if args.action == 'update':
        return update(args.scope)
    state = check(args.scope)
    print(json.dumps(state, ensure_ascii=False))
    return int(state['error'])


if __name__ == '__main__':
    raise SystemExit(main())
