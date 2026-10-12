"""Cached static companions for Hanabi. Never executed at every login."""
import hashlib
import json
import os
from pathlib import Path
import subprocess
import tempfile


def poster_for(video, cache=None, stills=Path('/usr/share/nodalix/wallpaper-stills')):
    video = Path(video).resolve(strict=True)
    stat = video.stat()
    signature = f'{video}:{stat.st_size}:{stat.st_mtime_ns}:{stat.st_ino}'
    key = hashlib.sha256(signature.encode()).hexdigest()
    cache = cache or Path(os.environ.get('XDG_CACHE_HOME', str(Path.home()/'.cache')))/'nodalix/wallpaper-posters'
    cache.mkdir(parents=True, exist_ok=True, mode=0o700)
    target = cache/(key+'.jpg')
    if target.is_file() and not target.is_symlink() and target.stat().st_size:
        return target
    # The packaged collection already includes posters; use only verified assets.
    manifest = Path('/usr/share/nodalix/animated-collection.json')
    shipped = stills/(video.stem+'.jpg')
    if video.parent == Path('/usr/share/backgrounds/nodalix/animated') and manifest.is_file() and shipped.is_file():
        rows = json.loads(manifest.read_text())
        item = next((row for row in rows if row['file'] == video.name), None)
        if item and item['size'] == stat.st_size and item['sha256'] == hashlib.file_digest(video.open('rb'), 'sha256').hexdigest():
            return shipped
    fd, temporary = tempfile.mkstemp(prefix='poster-', suffix='.jpg', dir=cache)
    os.close(fd)
    try:
        subprocess.run(['ffmpeg','-nostdin','-hide_banner','-loglevel','error',
                        '-ss','1','-i',str(video),'-frames:v','1','-update','1',
                        '-q:v','2','-y',temporary], check=True, timeout=60,
                       stdout=subprocess.DEVNULL, stderr=subprocess.PIPE)
        if not Path(temporary).stat().st_size:
            raise ValueError('Empty wallpaper poster')
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
