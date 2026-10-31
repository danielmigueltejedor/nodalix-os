#!/usr/bin/env python3
"""Download and verify a complete Nodalix ISO, including multipart releases."""
import argparse, hashlib, json, os, re, tempfile, urllib.parse, urllib.request
from pathlib import Path

REPOSITORY='danielmigueltejedor/nodalix-os'

def valid_file(row):
    name=row.get('asset','')
    if not name or Path(name).name!=name or '\\' in name:raise ValueError('Nombre de descarga no válido')
    if type(row.get('size')) is not int or row['size']<1:raise ValueError('Tamaño no válido')
    if not re.fullmatch('[0-9a-f]{64}',row.get('sha256','')):raise ValueError('SHA-256 no válido')

def download_iso(manifest,base_url,output=None):
    iso=manifest['iso'];name=iso.get('filename',iso.get('asset',''))
    if not re.fullmatch(r'nodalix-[0-9A-Za-z.-]+-x86_64\.iso',name):raise ValueError('Nombre de ISO no válido')
    expected={'asset':name,'size':iso.get('size'),'sha256':iso.get('sha256')};valid_file(expected)
    parts=iso.get('parts') or [expected]
    for part in parts:valid_file(part)
    if len({p['asset'] for p in parts})!=len(parts) or sum(p['size'] for p in parts)!=iso['size']:
        raise ValueError('Partes de ISO no válidas')
    output=Path(output) if output else Path(name)
    if output.exists():
        with output.open('rb') as stream:checksum=hashlib.file_digest(stream,'sha256').hexdigest()
        if output.stat().st_size==iso['size'] and checksum==iso['sha256']:
            print(f'ISO ya verificada: {output}');return output
        raise FileExistsError(f'Ya existe un archivo diferente: {output}')
    output.parent.mkdir(parents=True,exist_ok=True)
    fd,temporary=tempfile.mkstemp(prefix='.nodalix-iso-',dir=output.parent)
    whole=hashlib.sha256()
    try:
        with os.fdopen(fd,'wb') as destination:
            for index,part in enumerate(parts,1):
                print(f'Descargando ISO: parte {index} de {len(parts)}',flush=True)
                url=base_url.rstrip('/')+'/'+urllib.parse.quote(part['asset'])
                request=urllib.request.Request(url,headers={'User-Agent':'Nodalix-ISO-downloader'})
                checksum=hashlib.sha256();size=0
                with urllib.request.urlopen(request,timeout=90) as source:
                    while chunk:=source.read(1024*1024):
                        size+=len(chunk)
                        if size>part['size']:raise ValueError('La descarga supera el tamaño esperado')
                        destination.write(chunk);checksum.update(chunk);whole.update(chunk)
                if size!=part['size'] or checksum.hexdigest()!=part['sha256']:
                    raise ValueError('Descarga incompleta o SHA-256 incorrecto: '+part['asset'])
            destination.flush();os.fsync(destination.fileno())
        if whole.hexdigest()!=iso['sha256']:raise ValueError('SHA-256 de la ISO completa incorrecto')
        # Never overwrite a file created by another process during the download.
        os.link(temporary,output)
        print(f'ISO completa y verificada: {output}',flush=True)
        return output
    finally:Path(temporary).unlink(missing_ok=True)

def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--version',default='0.4.0')
    parser.add_argument('--output',type=Path)
    args=parser.parse_args()
    if not re.fullmatch(r'[0-9]+\.[0-9]+\.[0-9]+(?:-(?:beta|rc)\.[0-9.]+)?',args.version):raise ValueError('Versión no válida')
    base=f'https://github.com/{REPOSITORY}/releases/download/{args.version}'
    request=urllib.request.Request(base+'/nodalix-manifest.json',headers={'User-Agent':'Nodalix-ISO-downloader'})
    with urllib.request.urlopen(request,timeout=60) as response:manifest=json.load(response)
    if manifest['version']!=args.version:raise ValueError('El manifiesto pertenece a otra versión')
    download_iso(manifest,base,args.output)

if __name__=='__main__':main()
