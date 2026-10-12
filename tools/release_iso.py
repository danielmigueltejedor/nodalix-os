#!/usr/bin/env python3
"""Describe and split complete ISO images for GitHub's per-asset size limit."""
import argparse, hashlib, json, os
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
            temporary=target.with_name(target.name+'.tmp')
            part=hashlib.sha256();written=0
            try:
                with temporary.open('wb') as output:
                    while written<part_bytes:
                        chunk=source.read(min(1024*1024,part_bytes-written))
                        if not chunk:break
                        output.write(chunk);written+=len(chunk);part.update(chunk);whole.update(chunk)
                os.replace(temporary,target)
            finally:temporary.unlink(missing_ok=True)
            parts.append({'asset':target.name,'sha256':part.hexdigest(),'size':written})
            index+=1
    if whole.hexdigest()!=checksum or sum(p['size'] for p in parts)!=size:
        raise ValueError('ISO changed while splitting')
    return {'filename':image.name,'sha256':checksum,'size':size,'parts':parts}

def publication_assets(directory):
    directory=Path(directory)
    iso=json.loads((directory/'nodalix-manifest.json').read_text()).get('iso',{})
    assembled=iso.get('filename') if iso.get('parts') else None
    return [p for p in sorted(directory.iterdir()) if p.is_file() and p.name!=assembled and not p.name.endswith('.tmp')]

if __name__=='__main__':
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--assets-dir',type=Path,required=True)
    args=parser.parse_args()
    for path in publication_assets(args.assets_dir):print(path)
