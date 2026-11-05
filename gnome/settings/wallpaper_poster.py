"""Cached static companions for Hanabi. Never executed at every login."""
import hashlib
import json
import os
from pathlib import Path
import subprocess
import tempfile


def xdg_directory(variable, default):
    value = Path(os.environ.get(variable, ''))
    return value if value.is_absolute() else Path.home() / default


def signature(video):
    stat = video.stat()
    identity = f'{video}:{stat.st_dev}:{stat.st_ino}:{stat.st_size}:{stat.st_mtime_ns}:{stat.st_ctime_ns}'
    return hashlib.sha256(os.fsencode(identity)).hexdigest()


def poster_for(video, cache=None, stills=Path('/usr/share/nodalix/wallpaper-stills'), thumbnail=False):
    video = Path(video).resolve(strict=True)
    key = signature(video)
    cache = cache or xdg_directory('XDG_CACHE_HOME', '.cache') / 'nodalix/wallpaper-posters'
    cache.mkdir(parents=True, exist_ok=True, mode=0o700)
    target = cache/(key+'.jpg')
    if target.is_file() and not target.is_symlink() and target.stat().st_size:
        return target
    # The packaged collection already includes posters; use only verified assets.
    manifest = Path('/usr/share/nodalix/animated-collection.json')
    shipped = stills/(video.stem+'.jpg')
    if (not thumbnail and video.parent == Path('/usr/share/backgrounds/nodalix/animated')
            and manifest.is_file() and shipped.is_file()):
        rows = json.loads(manifest.read_text())
        item = next((row for row in rows if row['file'] == video.name), None)
        if item and item['size'] == video.stat().st_size:
            with video.open('rb') as source:
                if item['sha256'] == hashlib.file_digest(source, 'sha256').hexdigest():
                    return shipped
    fd, temporary = tempfile.mkstemp(prefix='poster-', suffix='.jpg', dir=cache)
    os.close(fd)
    try:
        command = ['ffmpeg', '-nostdin', '-hide_banner', '-loglevel', 'error',
                   '-protocol_whitelist', 'file', '-format_whitelist', 'mov,matroska,webm']
        filters = (['-vf', 'scale=288:216:force_original_aspect_ratio=increase,crop=288:216']
                   if thumbnail else [])
        for position in ('1', '0'):
            result = subprocess.run([*command, '-ss', position, '-i', str(video),
                            '-map', '0:V:0', '-frames:v', '1', *filters, '-update', '1',
                            '-q:v', '2', '-y', temporary], check=False, timeout=60,
                           stdout=subprocess.DEVNULL, stderr=subprocess.PIPE)
            if result.returncode == 0 and Path(temporary).stat().st_size:
                break
        else:
            raise ValueError('No se pudo generar una miniatura del vídeo')
        os.replace(temporary, target)
    finally:
        Path(temporary).unlink(missing_ok=True)
    return target


def sync_login(poster):
    # The privileged helper reads bytes from stdin, never a path in a user's HOME.
    with Path(poster).open('rb') as source:
        result = subprocess.run(['pkexec','/usr/lib/nodalix/sync-login-background'],
                                stdin=source, capture_output=True, timeout=90)
    if result.returncode:
        raise RuntimeError('Login wallpaper synchronization failed or was cancelled')
