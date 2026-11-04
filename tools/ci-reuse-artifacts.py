#!/usr/bin/python3
"""Reuse verified build artifacts only when package and ISO sources are unchanged.

A manual CI retry can change the disposable guest verifier without rebuilding
identical release binaries. The subsequent jobs still boot and test those bytes.
"""
import argparse, hashlib, json, os, re, subprocess, time
from pathlib import Path

QA_FILES = {'.github/workflows/ci.yml', 'tools/ci-boot-migrated.sh',
            'tools/ci-guest-verify.py', 'tools/ci-boot-iso.py', 'tools/ci-reuse-artifacts.py',
            '.github/workflows/release.yml', 'tools/release_iso.py',
            'tools/nodalix-download-iso.py', 'tools/finalize-release.py',
            'tests/test_release_iso_parts.py', 'docs/iso.md',
            'docs/releases/v0.3.0.md', 'docs/migration-0.3.0.md'}
REPO = os.environ.get('GITHUB_REPOSITORY', 'danielmigueltejedor/nodalix-os')

def api(path):
    return json.loads(subprocess.check_output(['gh', 'api', f'repos/{REPO}/'+path]))

def verify_source(run):
    if run['head_repository']['full_name'] != REPO:
        raise ValueError('Artifacts must originate in this repository')
    commit = run['head_sha']
    if not re.fullmatch(r'[0-9a-f]{40}', commit):
        raise ValueError('Invalid source commit SHA')
    changed = set(subprocess.check_output(
        ['git', 'diff', '--name-only', '--end-of-options', commit, 'HEAD', '--'],
        text=True).splitlines())
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
    if not re.fullmatch(r'[0-9]+', args.run_id):
        raise ValueError('Numeric source run ID required')
    run=api(f'actions/runs/{args.run_id}')
    verify_source(run)
    require_job(args.run_id,'packages' if args.kind=='packages' else 'iso',wait=args.kind=='iso')
    version=Path('VERSION').read_text().strip()
    suffix = 'candidate' if args.kind == 'packages' else 'installable-iso'
    name = f'nodalix-{version}-{suffix}'
    artifacts = api(f'actions/runs/{args.run_id}/artifacts?per_page=100')['artifacts']
    if not any(artifact['name'] == name and not artifact['expired'] for artifact in artifacts):
        # The older CI workflow labels its candidates 0.4.0 for every version.
        name = f'nodalix-0.4.0-{suffix}'
    output = args.output.resolve()
    output.mkdir(parents=True, exist_ok=True)
    subprocess.run(['gh', 'run', 'download', '--repo', REPO, '--name', name,
                    '--dir', str(output), '--', args.run_id], check=True)
    if args.kind=='packages':
        manifest=json.loads((output/'nodalix-manifest.json').read_text())
        if manifest['version'] != version:
            raise ValueError('Downloaded manifest belongs to another version')
        expected=json.loads(Path('release/components.json').read_text())['components']
        if {r['id'] for r in manifest['components']} != {r['id'] for r in expected}:
            raise ValueError('Downloaded manifest has unexpected components')
        for row in manifest['components']:
            name = row['asset']
            if not re.fullmatch(r'[A-Za-z0-9_@+.-]+\.pkg\.tar\.zst', name):
                raise ValueError('Unsafe filename in downloaded manifest')
            path = output / name
            if path.is_symlink() or path.stat().st_size != row['size'] or digest(path) != row['sha256']:
                raise ValueError(f'Invalid package artifact: {name}')
    else:
        path=output/f'nodalix-{version}-x86_64.iso'
        if path.with_suffix('.iso.sha256').read_text().strip() != f'{digest(path)}  {path.name}':
            raise ValueError('Invalid ISO artifact checksum')
    print(f'Reused verified {args.kind} from run {args.run_id}; all release sources are unchanged',flush=True)

if __name__=='__main__':main()
