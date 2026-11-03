#!/usr/bin/python3
"""Opt-in, local and privacy-aware Nodalix support and performance reports.

No command lines, environment variables, serials, network addresses, account
names, notification bodies, clipboard content or personal files are collected.
Unavailable sensors are reported as unavailable, never as zero consumption.
"""
import argparse
import json
import os
from pathlib import Path
import platform
import re
import subprocess
import time


def command(*args):
    try:
        result = subprocess.run(args, capture_output=True, text=True, timeout=8)
        return result.stdout.strip() if result.returncode == 0 else None
    except (OSError, subprocess.SubprocessError): return None


def read(path):
    try: return Path(path).read_text().strip()
    except OSError: return None


def number(path):
    try: return int(read(path))
    except (TypeError,ValueError): return None


def memory():
    text = read('/proc/meminfo') or ''
    wanted = {'MemTotal','MemAvailable','SwapTotal','SwapFree'}
    return {key:int(value)*1024 for key,value in re.findall(r'^(\w+):\s+(\d+) kB',text,re.M) if key in wanted}


def processes(proc=Path('/proc')):
    result = {}
    allowed = ('gnome-shell','gjs','rclone','pipewire','wireplumber','nodalix','steam-auto','global-menu','python')
    for directory in proc.glob('[0-9]*'):
        try:
            name=(directory/'comm').read_text().strip()
            # Python is included only when its cgroup names an installed Nodalix unit.
            cgroup=(directory/'cgroup').read_text()
            if name.startswith('python'):
                if not re.search(r'/(?:nodalix-|steam-(?:auto|desktop))[^/]*\.service',cgroup):continue
                name='nodalix-python'
            elif not name.startswith(allowed):continue
            stat=(directory/'stat').read_text().rsplit(')',1)[1].split()
            status=(directory/'status').read_text()
            rss=re.search(r'^VmRSS:\s+(\d+)',status,re.M)
            switches=sum(int(v) for v in re.findall(r'^\w*voluntary_ctxt_switches:\s+(\d+)',status,re.M))
            pss=None
            try:
                match=re.search(r'^Pss:\s+(\d+)',(directory/'smaps_rollup').read_text(),re.M)
                if match:pss=int(match[1])*1024
            except OSError:pass
            # Start time prevents PID reuse from becoming a false CPU delta.
            result[(directory.name,stat[19])]={'name':name, 'ticks':int(stat[11])+int(stat[12]),
                'rss_bytes':int(rss[1])*1024 if rss else 0,'pss_bytes':pss,'context_switches':switches}
        except (OSError,ValueError,IndexError):continue
    return result


def sensors():
    gpu=[]
    for card in Path('/sys/class/drm').glob('card[0-9]'):
        gpu.append({'driver':(card/'device/driver').resolve().name if (card/'device/driver').exists() else None,
                    'busy_percent':number(card/'device/gpu_busy_percent')})
    batteries=[]
    for battery in Path('/sys/class/power_supply').glob('*'):
        if read(battery/'type')!='Battery':continue
        power=number(battery/'power_now')
        if power is None:
            current,voltage=number(battery/'current_now'),number(battery/'voltage_now')
            if current is not None and voltage is not None:power=current*voltage/1e6
        batteries.append({'status':read(battery/'status'),'power_watts':power/1e6 if power is not None else None,
                          'capacity_percent':number(battery/'capacity')})
    return {'gpu':gpu,'batteries':batteries,
            'gpu_limit':'Utilization is available only for supported drivers; it is not a power measurement.'}


def display_state():
    try:
        from gi.repository import Gio
        bus=Gio.bus_get_sync(Gio.BusType.SESSION,None)
        state=bus.call_sync('org.gnome.Mutter.DisplayConfig','/org/gnome/Mutter/DisplayConfig',
            'org.gnome.Mutter.DisplayConfig','GetCurrentState',None,None,Gio.DBusCallFlags.NO_AUTO_START,2000,None).unpack()
        monitors=[]
        for monitor in state[1]:
            modes=[{'width':mode[1],'height':mode[2],'refresh_hz':round(mode[3],3),
                    'current':bool(mode[6].get('is-current',False))} for mode in monitor[1]]
            # Connector, EDID vendor/product/serial and display names are excluded.
            monitors.append({'modes':modes,'vrr_capable':monitor[2].get('is-vrr-capable'),
                             'vrr_state':monitor[2].get('vrr-mode')})
        return {'monitors':monitors,'logical_scales':[m[2] for m in state[2]],
                'note':'VRR properties are reported only when Mutter exposes them. No display configuration is changed.'}
    except Exception:return {'available':False}


def support_report():
    release=read('/etc/nodalix-release') or ''
    match=re.search(r'^VERSION_ID="?([^"\n]+)',release,re.M)
    packages=command('pacman','-Q','gnome-shell','mutter','pipewire','wireplumber','power-profiles-daemon','zram-generator')
    versions={line.split()[0]:line.split()[1] for line in (packages or '').splitlines() if len(line.split())==2}
    # Query each separately so a missing optional package does not hide all versions.
    for name in ('gnome-shell','mutter','pipewire','wireplumber','power-profiles-daemon','zram-generator'):
        value=command('pacman','-Q',name)
        if value and len(value.split())==2:versions[name]=value.split()[1]
    def failed(user):
        output=command('systemctl',*(['--user'] if user else []),'--failed','--no-legend','--plain','--no-pager')
        return {'count':len(output.splitlines()) if output is not None else None,
                'nodalix_units':[line.split()[0] for line in (output or '').splitlines() if line.startswith(('nodalix-','steam-auto','steam-desktop'))]}
    recovery={}
    try:
        data=json.loads(read('/var/lib/nodalix-recovery/status.json') or '{}')
        recovery={key:data.get(key) for key in ('supported','automatic','rebootRequired','freeBytes')}
        recovery['snapshot_count']=len(data.get('snapshots',[]))
    except (ValueError,TypeError):pass
    updates={}
    raw=command('nodalix-updater','status','--json')
    try:
        data=json.loads(raw or '{}')
        updates={key:data.get(key) for key in ('current_version','target_version','status','stage','reboot_required','reboot_mandatory')}
    except ValueError:pass
    cpu=re.search(r'^model name\s*:\s*(.+)',read('/proc/cpuinfo') or '',re.M)
    return {'schema_version':1,'nodalix_version':match[1] if match else None,
        'identity_health':'present' if match else 'missing-or-invalid','kernel':platform.release(),
        'cpu':cpu[1] if cpu else None,
        'architecture':platform.machine(),'session_type':os.environ.get('XDG_SESSION_TYPE','unknown'),
        'packages':versions,'memory':memory(),'sensors':sensors(),'displays':display_state(),
        'failed_system_units':failed(False),'failed_user_units':failed(True),
        'power_profile':command('powerprofilesctl','get'),
        'updates':updates,'recovery':recovery,
        'portals':{name:command('systemctl','--user','is-active',name) for name in
                   ('xdg-desktop-portal','xdg-desktop-portal-gnome','xdg-desktop-portal-gtk')},
        'root_filesystem':command('findmnt','-n','-o','FSTYPE','/'),
        'zram':command('zramctl','--noheadings','--output','ALGORITHM,DISKSIZE,DATA,COMPR,TOTAL'),
        'boot_time':command('systemd-analyze','time'),
        'privacy':'Local report: no host/user names, device identifiers, command lines, network data or personal content.'}


def sample(seconds):
    start=processes();clock=time.monotonic();time.sleep(seconds)
    end=processes();elapsed=time.monotonic()-clock;hz=os.sysconf('SC_CLK_TCK')
    groups={}
    for key,row in end.items():
        old=start.get(key)
        group=groups.setdefault(row['name'],{'processes':0,'cpu_percent_one_core':0,'rss_bytes':0,
                                           'pss_bytes':0,'pss_available':True,'context_switches_per_second':0})
        group['processes']+=1;group['rss_bytes']+=row['rss_bytes']
        if row['pss_bytes'] is None:group['pss_available']=False
        else:group['pss_bytes']+=row['pss_bytes']
        if old:
            group['cpu_percent_one_core']+=max(0,row['ticks']-old['ticks'])/hz/elapsed*100
            group['context_switches_per_second']+=max(0,row['context_switches']-old['context_switches'])/elapsed
    return {'sample_seconds':elapsed,'process_groups':groups,'memory':memory(),'sensors':sensors(),
            'limitations':['Context switches are not hardware wakeups.','Processes that exit during the sample are not included.',
                'RSS double-counts shared pages; prefer PSS when available.',
                'GPU, disk I/O, frame times and package watts require separate hardware-specific tools.',
                'A busy working session is not an idle or upstream Arch baseline.']}


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--sample',type=float,default=0,metavar='SECONDS')
    parser.add_argument('--json',action='store_true')
    args=parser.parse_args()
    if args.sample and not 1<=args.sample<=60:parser.error('Choose a sample duration from 1 to 60 seconds.')
    data=support_report()
    if args.sample:data['performance']=sample(args.sample)
    print(json.dumps(data,ensure_ascii=False,indent=2))


if __name__=='__main__':main()
