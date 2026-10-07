#!/usr/bin/python3
"""Prepare pinned native Settings packages and the installed video collection.

Sources can be pre-fetched using --control-center-source and --hanabi-source.
Video assets are hash-checked against the tracked source collection.
"""
import argparse
import hashlib
import json
from pathlib import Path
import re
import shutil
import subprocess
import tarfile
import tempfile

ROOT = Path(__file__).resolve().parents[1]
CC_REV = '6ae712c0454f2586f00821c3486150c1e8feb3e7'
VERSION = (ROOT/'VERSION').read_text().strip().replace('-', '')
HANABI_REV = 'b18e0414447a7b5eb472c6ede7b32782f972c4ae'

def run(args, **kwargs):
    return subprocess.run([str(a) for a in args], check=True, **kwargs)

def checkout(source, cache, name, url, revision):
    target = Path(source).resolve() if source else cache/name
    if not (target/'.git').exists():
        run(['git','clone',url,target])
        run(['git','checkout',revision],cwd=target)
    actual = subprocess.check_output(['git','rev-parse','HEAD'],cwd=target,text=True).strip()
    if actual != revision:
        raise SystemExit(f'{name}: expected {revision}, got {actual}')
    return target

def export(source, dest):
    dest.mkdir(parents=True,exist_ok=True)
    with tempfile.TemporaryFile() as archive:
        run(['git','archive','HEAD'],cwd=source,stdout=archive)
        archive.seek(0)
        with tarfile.open(fileobj=archive) as tar:
            tar.extractall(dest,filter='data')

def archive(root, output, paths):
    run(['tar','--zstd','-C',root,'-cf',output,'--exclude=*/__pycache__','--exclude=*.pyc','--exclude=gnome/settings/animated',*paths])

def recipe(name, output, archives):
    dest = output/name
    dest.mkdir(parents=True,exist_ok=True)
    for item in (ROOT/'packaging'/name).glob('*.install'):
        shutil.copy2(item,dest/item.name)
    text = (ROOT/'packaging'/name/'PKGBUILD').read_text()
    for path in archives:
        shutil.copy2(path,dest/path.name)
    sums=' '.join("'"+hashlib.file_digest(p.open('rb'),'sha256').hexdigest()+"'" for p in archives)
    text=re.sub(r'sha256sums=\([^)]*\)',f'sha256sums=({sums})',text,count=1)
    (dest/'PKGBUILD').write_text(text)
    return dest

def main():
    ap=argparse.ArgumentParser(description=__doc__)
    ap.add_argument('--output',type=Path,default=ROOT/'dist/native-settings')
    ap.add_argument('--control-center-source')
    ap.add_argument('--hanabi-source')
    ap.add_argument('--collection',type=Path,default=ROOT/'gnome/settings/animated')
    ap.add_argument('--prepare-only',action='store_true')
    args=ap.parse_args();out=args.output.resolve();out.mkdir(parents=True,exist_ok=True)
    cache=out/'cache';cache.mkdir(exist_ok=True)
    cc=checkout(args.control_center_source,cache,'control-center','https://github.com/GNOME/gnome-control-center.git',CC_REV)
    hanabi=checkout(args.hanabi_source,cache,'hanabi','https://github.com/jeffshee/gnome-ext-hanabi.git',HANABI_REV)
    with tempfile.TemporaryDirectory(dir=out) as temp:
        stage=Path(temp)
        export(cc,stage/'control-center')
        for name,url,pin in [('gvc','https://gitlab.gnome.org/GNOME/libgnome-volume-control.git','d2442f455844e5292cb4a74ffc66ecc8d7595a9f'),('libgxdp','https://gitlab.gnome.org/GNOME/libgxdp.git','df896e3412b749947bc6f62a91a1aac8e6b6d19b'),('blueprint-compiler','https://gitlab.gnome.org/jwestman/blueprint-compiler.git','8eca521ae720655c837fd1885940ed91f7517e6f')]:
            source=checkout(str(cc/'subprojects'/name),cache,name,url,pin)
            export(source,stage/'control-center/subprojects'/name)
        export(hanabi,stage/'hanabi')
        cc_archive=out/'control-center-51.0.tar.zst';archive(stage,cc_archive,['control-center'])
        hanabi_archive=out/'hanabi-b18e041.tar.zst';archive(stage,hanabi_archive,['hanabi'])
        videos=json.loads((ROOT/'gnome/settings/animated-collection.json').read_text())
        (stage/'collection').mkdir();(stage/'previews').mkdir();(stage/'stills').mkdir()
        for item in videos:
            video=args.collection/item['file']
            if not video.is_file():
                parts=sorted(args.collection.glob(item['file']+'.part[0-9][0-9][0-9]'))
                if not parts or [p.name for p in parts] != [item['file']+f'.part{i:03d}' for i in range(len(parts))]:
                    raise SystemExit(f'Missing collection parts: {video}')
                source=stage/'inputs';source.mkdir(exist_ok=True)
                video=source/item['file']
                with video.open('wb') as dest:
                    for part in parts:
                        with part.open('rb') as stream:shutil.copyfileobj(stream,dest)
            if not video.is_file() or video.stat().st_size != item['size'] or hashlib.file_digest(video.open('rb'),'sha256').hexdigest() != item['sha256']:
                raise SystemExit(f'Missing or changed collection asset: {video}')
            shutil.copy2(video,stage/'collection'/video.name)
            run(['ffmpeg','-hide_banner','-loglevel','error','-ss','1','-i',video,'-frames:v','1','-vf','scale=480:-2','-y',stage/'previews'/(video.stem+'.jpg')])
            run(['ffmpeg','-hide_banner','-loglevel','error','-ss','1','-i',video,'-frames:v','1','-q:v','2','-y',stage/'stills'/(video.stem+'.jpg')])
        collection=out/'nodalix-animated-collection.tar.zst';archive(stage,collection,['collection','previews','stills'])
        settings=out/f'nodalix-settings-{VERSION}.tar.zst';archive(ROOT,settings,['gnome/settings','docs/images/nodalix-logo.png'])
        changes=out/'nodalix-control-center-51.0.0.tar.zst';archive(ROOT,changes,['gnome/control-center'])
        renderer=out/f'nodalix-video-wallpapers-{VERSION}.tar.zst';archive(ROOT,renderer,['gnome/settings/animated-collection.json','packaging/nodalix-video-wallpapers/gnome51.patch','packaging/nodalix-video-wallpapers/pnpm-lock.yaml'])
        dirs=[recipe('nodalix-settings',out,[settings]),recipe('nodalix-control-center',out,[cc_archive,changes]),recipe('nodalix-video-wallpapers',out,[hanabi_archive,renderer,collection])]
    if not args.prepare_only:
        for folder in dirs:
            run(['makepkg','-f','--nodeps','--noconfirm'],cwd=folder)
    print(out)
if __name__=='__main__': main()
