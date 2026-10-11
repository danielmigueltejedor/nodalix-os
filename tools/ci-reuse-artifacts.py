#!/usr/bin/python3
"""Reuse verified build artifacts only when package and ISO sources are unchanged.

A manual CI retry can change the disposable guest verifier without rebuilding
identical release binaries. The subsequent jobs still boot and test those bytes.
"""
import argparse, hashlib, json, os, subprocess, time
from pathlib import Path

QA_FILES = {'.github/workflows/ci.yml', 'tools/ci-boot-migrated.sh',
            'tools/ci-guest-verify.py', 'tools/ci-boot-iso.py', 'tools/ci-reuse-artifacts.py'}
REPO = os.environ.get('GITHUB_REPOSITORY', 'danielmigueltejedor/nodalix-os')

def api(path):
    return json.loads(subprocess.check_output(['gh', 'api', f'repos/{REPO}/'+path]))

def verify_source(run):
    if run['head_repository']['full_name'] != REPO:
        raise ValueError('Artifacts must originate in this repository')
    changed = set(subprocess.check_output(
        ['git', 'diff', '--name-only', run['head_sha'], 'HEAD'], text=True).splitlines())
    if changed - QA_FILES:
        raise ValueError('Release sources changed: '+', '.join(sorted(changed-QA_FILES)))

def require_job(run_id, name, wait=False):
    deadline=time.monotonic()+(3600 if wait else 0)
    previous=None
    while True:
        jobs=api(f'actions/runs/{run_id}/jobs?per_page=100')['jobs']
        job=next((j for j in jobs if j['name']==name), None)
        if job and job['conclusion']=='success':return
        if job and job['status']=='completed':
            raise ValueError(f'Source {name} job did not pass: {job["conclusion"]}')
        if not wait or time.monotonic()>=deadline:
            raise ValueError(f'Source {name} job has not completed successfully')
        status=job['status'] if job else 'pending'
        if status!=previous:print(f'Waiting for source {name} job: {status}',flush=True);previous=status
        time.sleep(30)

def digest(path):
    with path.open('rb') as stream:return hashlib.file_digest(stream,'sha256').hexdigest()

def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('kind',choices=['packages','iso'])
    parser.add_argument('run_id')
    parser.add_argument('output',type=Path)
    args=parser.parse_args()
    if not args.run_id.isdecimal():raise ValueError('Numeric source run ID required')
    run=api(f'actions/runs/{args.run_id}')
    verify_source(run)
    require_job(args.run_id,'packages' if args.kind=='packages' else 'iso',wait=args.kind=='iso')
    name='nodalix-0.3.0-candidate' if args.kind=='packages' else 'nodalix-0.3.0-installable-iso'
    args.output.mkdir(parents=True,exist_ok=True)
    subprocess.run(['gh','run','download',args.run_id,'--repo',REPO,'--name',name,'--dir',str(args.output)],check=True)
    version=Path('VERSION').read_text().strip()
    if args.kind=='packages':
        manifest=json.loads((args.output/'nodalix-manifest.json').read_text())
        assert manifest['version']==version
        expected=json.loads(Path('release/components.json').read_text())['components']
        assert {r['id'] for r in manifest['components']}=={r['id'] for r in expected}
        for row in manifest['components']:
            assert Path(row['asset']).name==row['asset']
            path=args.output/row['asset']
            assert path.stat().st_size==row['size'] and digest(path)==row['sha256']
    else:
        path=args.output/f'nodalix-{version}-x86_64.iso'
        assert path.stat().st_size<2147483647,'ISO exceeds GitHub release asset size limit'
        assert path.with_suffix('.iso.sha256').read_text().strip()==f'{digest(path)}  {path.name}'
    print(f'Reused verified {args.kind} from run {args.run_id}; all release sources are unchanged',flush=True)

if __name__=='__main__':main()
