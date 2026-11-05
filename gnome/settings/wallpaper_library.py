"""User-owned animated wallpaper library and media metadata."""
from collections import defaultdict
from fractions import Fraction
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import tempfile

from wallpaper_poster import poster_for, signature, xdg_directory


FORMATS = {'.mp4', '.webm', '.mkv'}
SYSTEM_ROOTS = (Path('/usr/share/backgrounds/nodalix/animated'),
                Path('/usr/share/backgrounds/nodalix/Animados'))


def read_json(path, fallback):
    try:
        return json.loads(path.read_text(encoding='utf-8'))
    except (OSError, ValueError):
        return fallback


def write_json(path, data):
    path.parent.mkdir(mode=0o700, parents=True, exist_ok=True)
    with tempfile.NamedTemporaryFile(mode='w', encoding='utf-8', dir=path.parent,
                                     delete=False) as stream:
        temporary = Path(stream.name)
        try:
            json.dump(data, stream, ensure_ascii=False)
            stream.close()
            temporary.replace(path)
        finally:
            temporary.unlink(missing_ok=True)


def fingerprint(path):
    with path.open('rb') as source:
        return hashlib.file_digest(source, 'sha256').hexdigest()


def inspect_video(path):
    result = subprocess.run(
        ['ffprobe', '-v', 'error', '-protocol_whitelist', 'file',
         '-format_whitelist', 'mov,matroska,webm', '-show_entries', 'stream=codec_name,width,height,avg_frame_rate:format=duration,format_name',
         '-select_streams', 'V:0', '-of', 'json', str(path)],
        capture_output=True, check=True, timeout=20)
    data = json.loads(result.stdout)
    video = next(iter(data.get('streams', [])), {})
    media_format = data.get('format', {})
    if not video.get('width') or not video.get('height') or not (
            set(media_format.get('format_name', '').split(',')) & {'mp4', 'matroska', 'webm'}):
        raise ValueError('Se necesita un vídeo MP4, WebM o MKV válido')
    try:
        fps = round(float(Fraction(video.get('avg_frame_rate', '0'))), 2)
    except (ValueError, ZeroDivisionError):
        fps = 0
    try:
        duration = float(media_format.get('duration', 0))
    except ValueError:
        duration = 0
    return dict(width=video['width'], height=video['height'], fps=fps,
                codec=video.get('codec_name', ''), duration=duration)


class WallpaperLibrary:
    def __init__(self, personal=None, system_roots=SYSTEM_ROOTS, cache=None, state=None,
                 previews=Path('/usr/share/nodalix/wallpaper-previews')):
        self.personal = personal or xdg_directory('XDG_DATA_HOME', '.local/share') / 'backgrounds/nodalix/animated'
        self.system_roots = system_roots
        self.cache = cache or xdg_directory('XDG_CACHE_HOME', '.cache') / 'nodalix/wallpaper-library'
        self.favorites = (state or xdg_directory('XDG_STATE_HOME', '.local/state') / 'nodalix') / 'wallpaper-favorites.json'
        self.previews = previews

    def entries(self):
        seen = set()
        for directory in (*self.system_roots, self.personal):
            if directory == self.personal and (directory.is_symlink() or directory.resolve() in {
                    root.resolve() for root in self.system_roots}):
                continue
            for path in sorted(directory.glob('*')):
                if path.name.startswith('.') or path.suffix.lower() not in FORMATS or not path.is_file() or path.is_symlink():
                    continue
                path = path.resolve()
                if path not in seen:
                    seen.add(path)
                    yield path, directory == self.personal

    def catalog(self):
        favorites = read_json(self.favorites, [])
        for path, personal in self.entries():
            try:
                key = signature(path)
                size = path.stat().st_size
            except OSError:
                continue
            info = read_json(self.cache / (key + '.json'), {})
            preview = self.cache / (key + '.jpg') if personal else self.previews / (path.stem + '.jpg')
            title = re.sub(r'[-_.]+', ' ', path.stem.removeprefix('nodalix-').removesuffix('-4k'))
            yield dict(info, path=str(path), title=title.strip().title(), size=size,
                       preview=str(preview) if preview.is_file() else '', personal=personal,
                       favorite=str(path) in favorites)

    def enrich(self):
        changed = False
        for path, personal in self.entries():
            try:
                key = signature(path)
            except OSError:
                continue
            record = self.cache / (key + '.json')
            if record.is_file() and (not personal or (self.cache / (key + '.jpg')).is_file()
                                     or read_json(record, {}).get('unreadable')):
                continue
            try:
                details = inspect_video(path)
                if personal:
                    poster_for(path, cache=self.cache, thumbnail=True)
            except (OSError, ValueError, subprocess.SubprocessError):
                details = {'unreadable': True}
            try:
                if key == signature(path):
                    write_json(record, details)
                    changed = True
            except OSError:
                continue
        return changed

    def hash_for(self, path):
        key = signature(path)
        cache_file = self.cache / (key + '.sha256')
        digest = read_json(cache_file, '')
        if isinstance(digest, str) and re.fullmatch(r'[0-9a-f]{64}', digest):
            return digest
        digest = fingerprint(path)
        if key == signature(path):
            try:
                write_json(cache_file, digest)
            except OSError:
                pass
        return digest

    def import_videos(self, filenames):
        self.personal.mkdir(mode=0o700, parents=True, exist_ok=True)
        if self.personal.is_symlink() or self.personal.resolve() in {root.resolve() for root in self.system_roots}:
            raise ValueError('La colección personal debe ser una carpeta propia')
        digests = set()
        for path, _ in self.entries():
            try:
                digests.add(self.hash_for(path))
            except OSError:
                continue
        result = {'imported': 0, 'duplicates': 0, 'errors': []}
        for filename in filenames:
            path = Path(filename)
            try:
                self.import_video(path, digests)
            except FileExistsError:
                result['duplicates'] += 1
            except (OSError, ValueError, subprocess.SubprocessError) as error:
                message = 'No se pudo leer el vídeo' if isinstance(error, subprocess.SubprocessError) else str(error)
                result['errors'].append(f'{path.name}: {message}')
            else:
                result['imported'] += 1
        return result

    def import_video(self, path, digests):
        if path.suffix.lower() not in FORMATS or not path.is_file():
            raise ValueError('Formato no admitido. Elige un archivo MP4, WebM o MKV')
        fd, temporary = tempfile.mkstemp(suffix=path.suffix.lower(), prefix='.import-', dir=self.personal)
        temporary = Path(temporary)
        try:
            checksum = hashlib.sha256()
            with os.fdopen(fd, 'wb') as destination, path.open('rb') as source:
                while chunk := source.read(1024 * 1024):
                    destination.write(chunk)
                    checksum.update(chunk)
            digest = checksum.hexdigest()
            if digest in digests:
                raise FileExistsError(path)
            inspect_video(temporary)
            stem = path.stem.lstrip('.') or 'Vídeo'
            for number in range(1, 10000):
                suffix = '' if number == 1 else f' ({number})'
                target = self.personal / (stem + suffix + temporary.suffix)
                try:
                    os.link(temporary, target)
                except FileExistsError:
                    continue
                digests.add(digest)
                return
            raise ValueError('Hay demasiados fondos con el mismo nombre')
        finally:
            temporary.unlink(missing_ok=True)

    def optimize(self, filename, cancelled):
        path = next((path for path, _ in self.entries() if str(path) == filename), None)
        if path is None:
            raise ValueError('El fondo ya no está en la colección')
        details = inspect_video(path)
        if not 0 < details['fps'] < 60 or not details['duration']:
            raise ValueError('Se necesita un vídeo con menos de 60 FPS y una duración conocida')
        self.personal.mkdir(mode=0o700, parents=True, exist_ok=True)
        if self.personal.is_symlink() or self.personal.resolve() in {root.resolve() for root in self.system_roots}:
            raise ValueError('La colección personal debe ser una carpeta propia')
        target = self.personal / (path.stem + ' (60 FPS).mkv')
        if target.exists():
            raise ValueError('La colección ya contiene una copia a 60 FPS de este fondo')
        if shutil.disk_usage(self.personal).free < max(path.stat().st_size, 64 * 1024 * 1024):
            raise ValueError('No hay suficiente espacio para crear una copia')
        fd, temporary = tempfile.mkstemp(suffix='.mkv', prefix='.interpolate-', dir=self.personal)
        os.close(fd)
        temporary = Path(temporary)
        try:
            command = ['nice', '-n', '10', 'ffmpeg', '-nostdin', '-loglevel', 'error',
                       '-protocol_whitelist', 'file', '-format_whitelist', 'mov,matroska,webm',
                       '-filter_threads', '2', '-i', str(path),
                       '-map', '0:V:0', '-map', '0:a?', '-vf',
                       'tpad=stop_mode=clone:stop_duration=1,minterpolate=fps=60',
                       '-t', str(details['duration']), '-c:v', 'libx264', '-preset', 'fast',
                       '-crf', '18', '-threads', '2', '-c:a', 'copy', '-y', str(temporary)]
            with subprocess.Popen(command, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL) as job:
                while job.poll() is None:
                    if not cancelled.wait(0.25):
                        continue
                    job.terminate()
                    try:
                        job.wait(timeout=5)
                    except subprocess.TimeoutExpired:
                        job.kill()
                    break
            if cancelled.is_set():
                raise ValueError('Interpolación cancelada')
            if job.returncode:
                raise ValueError('No se pudo generar la copia a 60 FPS')
            inspect_video(temporary)
            os.link(temporary, target)
        finally:
            temporary.unlink(missing_ok=True)

    def duplicates(self, selected):
        matches = defaultdict(list)
        for path, personal in self.entries():
            try:
                matches[self.hash_for(path)].append((path, personal))
            except OSError:
                continue
        duplicates = []
        for group in matches.values():
            if len(group) < 2:
                continue
            # Keep packaged videos and the selected copy even if there are other copies.
            ordered = sorted(group, key=lambda item: (item[1], str(item[0]) != selected, item[0]))
            duplicates.extend(str(path) for path, personal in ordered[1:]
                              if personal and str(path) != selected)
        return duplicates

    def delete(self, filenames, selected, duplicates_only=False):
        allowed = ({Path(path) for path in self.duplicates(selected)} if duplicates_only
                   else {path for path, personal in self.entries() if personal})
        paths = set(map(Path, filenames))
        if not paths <= allowed or Path(selected) in paths:
            raise ValueError('Solo se pueden borrar fondos personales que no estén seleccionados')
        for path in paths:
            key = signature(path)
            path.unlink()
            for suffix in ('.jpg', '.json', '.sha256'):
                (self.cache / (key + suffix)).unlink(missing_ok=True)
        self.prune_cache()
        if paths:
            favorites = read_json(self.favorites, [])
            write_json(self.favorites, [path for path in favorites if Path(path) not in paths])

    def prune_cache(self):
        live = set()
        for path, _ in self.entries():
            try:
                live.add(signature(path))
            except OSError:
                continue
        for entry in self.cache.iterdir() if self.cache.is_dir() else ():
            if re.fullmatch(r'[0-9a-f]{64}\.(jpg|json|sha256)', entry.name) and entry.stem not in live:
                entry.unlink(missing_ok=True)

    def favorite(self, path, enabled):
        if not any(str(item) == path for item, _ in self.entries()):
            raise ValueError('El fondo ya no está en la colección')
        favorites = set(read_json(self.favorites, []))
        if enabled:
            favorites.add(path)
        else:
            favorites.discard(path)
        write_json(self.favorites, sorted(favorites))
