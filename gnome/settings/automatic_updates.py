"""Explicit persistent consent for a fixed set of unattended update timers."""
import fcntl
import grp
import json
import os
from pathlib import Path
import pwd
import subprocess
import sys
import tempfile

KINDS=('nodalix','system','flatpak','firmware')
STATE=Path('/var/lib/nodalix-updater/automatic-updates.json')
SYSTEMCTL='/usr/bin/systemctl'


def units(kind,uid):
    if kind not in KINDS:raise ValueError('Tipo de actualización automática inválido')
    result=['nodalix-update-'+kind+'.timer']
    if kind=='flatpak':
        if type(uid) is not int or uid<=0:raise ValueError('Se necesita un usuario de escritorio válido')
        result.append(f'nodalix-update-flatpak-user@{uid}.timer')
    return result


def read_state(path=STATE):
    try:
        state=json.loads(path.read_text())
        if not isinstance(state,dict):raise ValueError('Formato de preferencias inválido')
        return state
    except FileNotFoundError:return {}


def write_state(path,state):
    path.parent.mkdir(parents=True,exist_ok=True)
    fd,name=tempfile.mkstemp(prefix='.automatic-',dir=path.parent)
    try:
        with os.fdopen(fd,'w') as output:json.dump(state,output)
        os.chmod(name,0o644)
        os.replace(name,path)
    finally:Path(name).unlink(missing_ok=True)


def unit_states(names,runner=subprocess.run):
    result=runner([SYSTEMCTL,'show','--no-pager','--property=Id,LoadState,UnitFileState,ActiveState',*names],
                  capture_output=True,text=True,timeout=15,check=True)
    rows={}
    for block in result.stdout.strip().split('\n\n'):
        values=dict(line.split('=',1) for line in block.splitlines() if '=' in line)
        if values.get('Id'):rows[values['Id']]=values
    return rows


def snapshot(uid=None,path=STATE,runner=subprocess.run):
    uid=os.getuid() if uid is None else uid
    config=read_state(path)
    names=[name for kind in KINDS for name in units(kind,uid)]
    states=unit_states(names,runner)
    result={}
    for kind in KINDS:
        record=config.get(kind,{})
        owner=record.get('owner_uid')
        if kind=='flatpak' and type(owner) is int and owner>0 and owner!=uid:
            names=units(kind,owner)
            states.update(unit_states(names,runner))
        else:names=units(kind,uid)
        rows=[states.get(name,{}) for name in names]
        enabled=all(row.get('UnitFileState') in ('enabled','enabled-runtime') for row in rows)
        result[kind]={'enabled':enabled,'active':all(row.get('ActiveState')=='active' for row in rows),
            'available':all(row.get('LoadState')=='loaded' for row in rows),
            'partial':any(row.get('UnitFileState') in ('enabled','enabled-runtime') for row in rows) and not enabled,
            'owned':owner in (None,uid),'frequency':'weekly' if kind=='firmware' else 'daily'}
    legacy=runner([SYSTEMCTL,'--user','is-enabled','nodalix-update-apps.timer'],capture_output=True,text=True,timeout=10)
    result['flatpak']['system_enabled']=result['flatpak']['enabled'] or result['flatpak']['partial']
    if legacy.returncode==0 and legacy.stdout.strip() in ('enabled','enabled-runtime'):
        result['flatpak'].update(enabled=True,partial=True,legacy_user_timer=True)
    return result


def administrator(uid):
    if uid==0:return True
    user=pwd.getpwuid(uid)
    groups={group.gr_name for group in grp.getgrall() if user.pw_name in group.gr_mem or group.gr_gid==user.pw_gid}
    return bool(groups & {'wheel','sudo'})


def configure(kind,enabled,uid,path=STATE,runner=subprocess.run):
    target=units(kind,uid)
    previous=read_state(path)
    record=previous.get(kind,{})
    owner=record.get('owner_uid')
    if not enabled and uid!=0 and owner!=uid and not (owner is None and administrator(uid)):
        raise PermissionError('Solo puede desactivarlo el usuario que lo autorizó o el administrador.')
    old=units(kind,owner) if type(owner) is int and owner>0 else target
    names=list(dict.fromkeys(old+target))
    found=unit_states(names,runner)
    before={name:found.get(name,{'UnitFileState':'disabled','ActiveState':'inactive'}) for name in names}
    updated={**previous,kind:{'enabled':enabled,'owner_uid':uid if enabled else owner}}
    # Consent is on disk before a Persistent timer can start a missed job.
    write_state(path,updated)
    try:
        obsolete=[name for name in old if name not in target]
        if obsolete:runner([SYSTEMCTL,'disable','--now',*obsolete],check=True,capture_output=True,text=True,timeout=30)
        runner([SYSTEMCTL,'enable' if enabled else 'disable','--now',*target],check=True,capture_output=True,text=True,timeout=30)
        after=unit_states(target,runner)
        if enabled and not all(after.get(name,{}).get('UnitFileState') in ('enabled','enabled-runtime') and after.get(name,{}).get('ActiveState')=='active' for name in target):
            raise RuntimeError('No se pudieron activar todos los temporizadores.')
        if not enabled and any(after.get(name,{}).get('UnitFileState') in ('enabled','enabled-runtime') or after.get(name,{}).get('ActiveState')=='active' for name in target):
            raise RuntimeError('No se pudieron desactivar todos los temporizadores.')
    except Exception:
        write_state(path,previous)
        for name,row in before.items():
            runner([SYSTEMCTL,'enable' if row.get('UnitFileState') in ('enabled','enabled-runtime') else 'disable',name],check=False,capture_output=True,text=True,timeout=30)
            runner([SYSTEMCTL,'start' if row.get('ActiveState')=='active' else 'stop',name],check=False,capture_output=True,text=True,timeout=30)
        raise


def privileged_main(enabled):
    if len(sys.argv)!=2 or sys.argv[1] not in KINDS:raise SystemExit('Operación no admitida')
    if os.geteuid()!=0:raise SystemExit('Se requiere autenticación de administrador')
    try:
        uid=int(os.environ.get('PKEXEC_UID','0'))
        pwd.getpwuid(uid)
        with open('/run/nodalix-automatic-updates.lock','w') as lock:
            fcntl.flock(lock,fcntl.LOCK_EX|fcntl.LOCK_NB)
            configure(sys.argv[1],enabled,uid)
    except (OSError,ValueError,KeyError,RuntimeError,subprocess.SubprocessError) as error:
        print('No se pudo guardar la preferencia de actualización automática: '+str(error),file=sys.stderr)
        return 1
    return 0
