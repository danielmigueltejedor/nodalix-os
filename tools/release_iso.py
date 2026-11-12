#!/usr/bin/env python3
"""Describe and split complete ISO images for GitHub's per-asset size limit."""
import argparse, hashlib, json, os, stat, tempfile
from pathlib import Path

PART_BYTES = 2_000_000_000

def digest(path):
    with path.open('rb') as stream:return hashlib.file_digest(stream,'sha256').hexdigest()

def iso_metadata(image, part_bytes=PART_BYTES):
    image=Path(image)
    if part_bytes<1:raise ValueError('Positive part size required')
    size=image.stat().st_size
    if not size:raise ValueError('Empty ISO')
    checksum=digest(image)
    if size<=part_bytes:
        return {'asset':image.name,'sha256':checksum,'size':size}
    parts=[];whole=hashlib.sha256()
    with image.open('rb') as source:
        index=1
        while source.tell()<size:
            target=image.with_name(image.name+f'.part{index:02d}')
            fd, temporary = tempfile.mkstemp(prefix='.iso-part-', suffix='.tmp', dir=target.parent)
            part=hashlib.sha256();written=0
            try:
                with os.fdopen(fd, 'wb') as output:
                    while written<part_bytes:
                        chunk=source.read(min(1024*1024,part_bytes-written))
                        if not chunk:break
                        output.write(chunk);written+=len(chunk);part.update(chunk);whole.update(chunk)
                os.replace(temporary,target)
            finally:Path(temporary).unlink(missing_ok=True)
            parts.append({'asset':target.name,'sha256':part.hexdigest(),'size':written})
            index+=1
    if whole.hexdigest()!=checksum or sum(p['size'] for p in parts)!=size:
        raise ValueError('ISO changed while splitting')
    return {'filename':image.name,'sha256':checksum,'size':size,'parts':parts}

def publication_assets(directory):
    directory=Path(directory)
    manifest = directory / 'nodalix-manifest.json'
    fd = os.open(manifest, os.O_RDONLY | os.O_NOFOLLOW | os.O_NONBLOCK)
    with os.fdopen(fd, encoding='utf-8') as stream:
        if not stat.S_ISREG(os.fstat(stream.fileno()).st_mode):
            raise ValueError('Release manifest is not a regular file')
        iso = json.load(stream).get('iso', {})
    assembled = iso.get('filename') if iso.get('parts') else None
    return [
        path for path in sorted(directory.iterdir())
        if not path.is_symlink() and path.is_file()
        and path.name != assembled and not path.name.endswith('.tmp')
    ]

def release_directory():
    workspace = Path.cwd().resolve()
    directory = workspace / 'dist' / 'release'
    if not directory.resolve().is_relative_to(workspace):
        raise ValueError('The release directory must be inside the workspace')
    return directory


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    # Keep the release workflow's flag while publishing only its staging directory.
    parser.add_argument('--assets-dir', type=Path, choices=[Path('dist/release')], required=True)
    parser.parse_args()
    for path in publication_assets(release_directory()):
        print(path)
