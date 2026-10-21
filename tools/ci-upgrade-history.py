#!/usr/bin/python3
"""Validate every earlier published package set in a disposable Arch container."""
import hashlib, json, os, shutil, subprocess, urllib.request
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
REPOSITORY = 'danielmigueltejedor/nodalix-os'
from versioning import compare_versions

def fetch(url):
    request = urllib.request.Request(url, headers={'User-Agent':'Nodalix-upgrade-tests'})
    with urllib.request.urlopen(request, timeout=60) as response:
        return response.read()

def main():
    if os.environ.get('NODALIX_CI_CONTAINER') != '1':
        raise SystemExit('Disposable CI container required')
    subprocess.run(['pacman','-Syu','--noconfirm'], check=True)
    subprocess.run(['pacman','-S','--needed','--noconfirm','python','libarchive','git','diffutils'], check=True)
    releases = json.loads(fetch(f'https://api.github.com/repos/{REPOSITORY}/releases?per_page=100'))
    candidates = ROOT/'dist/packages'
    evidence = ROOT/'dist/history-evidence'
    evidence.mkdir(parents=True, exist_ok=True)
    results = []
    version = (ROOT/'VERSION').read_text().strip()
    for release in reversed(releases):
        tag = release['tag_name']
        if release.get('draft') or compare_versions(tag.removeprefix('v'), '0.1.1') < 0 or compare_versions(tag.removeprefix('v'), version) >= 0:
            continue
        asset = next((a for a in release['assets'] if a['name']=='nodalix-manifest.json'), None)
        if asset is None:
            raise RuntimeError('Published release has no manifest: '+tag)
        manifest = json.loads(fetch(asset['browser_download_url']))
        folder = ROOT/'dist/history-baselines'/tag
        folder.mkdir(parents=True, exist_ok=True)
        urls = {a['name']:a['browser_download_url'] for a in release['assets']}
        for component in manifest['components']:
            name = component['asset']
            if Path(name).name != name:
                raise ValueError('Unsafe asset filename')
            path = folder/name
            if not path.is_file():
                path.write_bytes(fetch(urls[name]))
            if hashlib.sha256(path.read_bytes()).hexdigest() != component['sha256']:
                raise ValueError('Baseline checksum mismatch: '+tag+'/'+name)
        log = evidence/(tag+'.log')
        with log.open('w') as output:
            subprocess.run(['bash',str(ROOT/'tools/test-package-upgrade.sh'),str(folder),str(candidates),version], stdout=output, stderr=subprocess.STDOUT, check=True)
        results.append({'from':tag,'to':version,'result':'passed'})
        (evidence/'results.json').write_text(json.dumps(results,indent=2)+'\n')
        print(tag+' -> '+version+' passed', flush=True)
        shutil.rmtree(folder)
    if not results:
        raise RuntimeError('No published historical release was tested')

if __name__ == '__main__':
    main()
