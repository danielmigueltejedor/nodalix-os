#!/usr/bin/env python3
import argparse, hashlib, json, re, sys
from pathlib import Path
import gi
gi.require_version('ECal','2.0'); gi.require_version('EDataServer','1.2')
from gi.repository import ECal, EDataServer
ROOT=Path.home()/'.local/share/vdirsyncer'; KHAL=Path.home()/'.config/khal/config'; STATE=Path.home()/'.local/state/nodalix/calendar-bridge.json'
EDS_UID='nodalix-calendar'; EDS_NAME='Nodalix'; VERSION=1

def unfold(text):
    out=[]
    for line in text.replace('\r\n','\n').replace('\r','\n').split('\n'):
        if line.startswith((' ','\t')) and out: out[-1]+=line[1:]
        else: out.append(line)
    return out

def vevents(text):
    out=[]; pos=0
    while True:
        a=text.find('BEGIN:VEVENT',pos)
        if a<0: break
        b=text.find('END:VEVENT',a)
        if b<0: break
        b+=len('END:VEVENT'); out.append(text[a:b]+'\r\n'); pos=b
    return out

def comp(text): return ECal.Component.new_from_string(text)

def ident(text):
    c=comp(text)
    if c is None or not c.get_uid(): return None
    return c.get_uid(), c.get_recurid_as_string() or ''

def marker(text):
    for line in unfold(text):
        if line.startswith('X-NODALIX-SOURCE-FILE:'): return line.split(':',1)[1].strip()
    return ''

def strip_marker(text):
    return '\r\n'.join(x for x in unfold(text) if x and not x.startswith('X-NODALIX-SOURCE-FILE:'))+'\r\n'

def tagged(text, rel):
    lines=unfold(strip_marker(text)); out=[]; done=False
    for line in lines:
        if line=='END:VEVENT' and not done: out.append('X-NODALIX-SOURCE-FILE:'+rel); done=True
        if line: out.append(line)
    if not done: raise ValueError('VEVENT without END:VEVENT')
    return '\r\n'.join(out)+'\r\n'

def clean_local(text):
    return '\r\n'.join(x for x in unfold(text) if x and not x.startswith(('X-EVOLUTION-','X-NODALIX-')))+'\r\n'

def canonical(text):
    volatile = {"DTSTAMP", "LAST-MODIFIED", "CREATED", "SEQUENCE"}
    entries = []
    lines = unfold(text)

    def normalize(line):
        if not line:
            return None

        left, sep, value = line.partition(":")
        if not sep:
            return line

        prop = left.split(";", 1)[0].upper()

        if prop == "URL" and not value.strip():
            return None

        if prop == "DESCRIPTION":
            value = value.lstrip(" 	")
            return left + ":" + value

        if prop in {"SUMMARY", "LOCATION"}:
            value = value.strip()
            return left + ":" + value

        if prop == "RRULE":
            parts = []

            for part in value.split(";"):
                part = part.strip()

                # RFC 5545 defaults WKST to Monday.
                # EDS commonly removes an explicit WKST=MO.
                if part.upper() == "WKST=MO":
                    continue

                if part:
                    parts.append(part.upper())

            value = ";".join(sorted(parts))

            # Evolution Data Server adds this internal parameter when it
            # calculates the final occurrence of COUNT-based recurrences.
            # It is metadata, not part of the recurrence semantics.
            left_parts = left.split(";")
            kept_params = [
                item
                for item in left_parts[1:]
                if not item.upper().startswith(
                    "X-EVOLUTION-ENDDATE="
                )
            ]

            normalized_left = ";".join(
                [left_parts[0]] + kept_params
            )

            return normalized_left + ":" + value

        return line

    i = 0

    while i < len(lines):
        line = lines[i]

        if (
            not line
            or line in ("BEGIN:VEVENT", "END:VEVENT")
            or line.startswith(("X-EVOLUTION-", "X-NODALIX-"))
        ):
            i += 1
            continue

        prop = line.split(";", 1)[0].split(":", 1)[0].upper()

        if prop in volatile:
            i += 1
            continue

        if line.startswith("BEGIN:"):
            name = line[6:]
            block = []
            i += 1

            while i < len(lines) and lines[i] != "END:" + name:
                inner = lines[i]
                ip = (
                    inner.split(";", 1)[0].split(":", 1)[0].upper()
                    if inner
                    else ""
                )

                if (
                    inner
                    and not inner.startswith((
                        "X-EVOLUTION-",
                        "X-NODALIX-",
                    ))
                    and ip not in volatile
                ):
                    normalized = normalize(inner)
                    if normalized is not None:
                        block.append(normalized)

                i += 1

            entries.append(
                "@" + name + "{" + chr(10).join(sorted(block)) + "}"
            )
            i += 1
            continue

        normalized = normalize(line)

        if normalized is not None:
            entries.append(normalized)

        i += 1

    return chr(10).join(sorted(entries))

def h(text): return hashlib.sha256(canonical(text).encode()).hexdigest()
def gh(events): return hashlib.sha256('\n'.join(r+'\0'+h(v['text']) for r,v in sorted(events.items())).encode()).hexdigest()

def family_dir():
    current=''
    for raw in KHAL.read_text(encoding='utf-8',errors='replace').splitlines():
        line=raw.strip()
        if line.startswith('[[') and line.endswith(']]'): current=line[2:-2].strip(); continue
        if current=='family' and line.startswith('path') and '=' in line:
            p=Path(line.split('=',1)[1].strip().strip('"\'')).expanduser().resolve(); p.relative_to(ROOT.resolve()); return p
    raise SystemExit('No [[family]] path in '+str(KHAL))

def relpath(path): return str(path.resolve().relative_to(ROOT.resolve()))
def abspath(rel):
    p=(ROOT/rel).resolve(); p.relative_to(ROOT.resolve()); return p

def writable(path,family):
    try: path.resolve().relative_to(family.resolve()); return True
    except ValueError: return False

def scan_local(family):
    groups={}; errors=[]
    for path in sorted(ROOT.rglob('*.ics')):
        raw=path.read_text(encoding='utf-8',errors='replace'); events={}
        for text in vevents(raw):
            ii=ident(text)
            if not ii: continue
            uid,rid=ii; events.setdefault(uid,{})[rid]={'text':text,'hash':h(text)}
        for uid,ev in events.items():
            if uid in groups and groups[uid]['source_file']!=relpath(path): errors.append(uid); continue
            groups[uid]={'events':ev,'source_file':relpath(path),'writable':writable(path,family),'hash':gh(ev)}
    return groups,errors

def connect():
    reg=EDataServer.SourceRegistry.new_sync(None); src=reg.ref_source(EDS_UID)
    if src is None:
        src=EDataServer.Source.new_with_uid(EDS_UID,None); src.set_display_name(EDS_NAME); src.set_enabled(True)
        src.get_extension(EDataServer.SOURCE_EXTENSION_CALENDAR).set_backend_name('local'); reg.commit_source_sync(src,None); src=reg.ref_source(EDS_UID)
    return ECal.Client.connect_sync(src,ECal.ClientSourceType.EVENTS,10,None)

def scan_eds(client):
    ok,cs=client.get_object_list_as_comps_sync('#t',None)
    if not ok: raise RuntimeError('EDS query failed')
    groups={}
    for c in cs:
        uid=c.get_uid() or ''
        if not uid: continue
        rid=c.get_recurid_as_string() or ''; text=c.get_icalcomponent().as_ical_string()
        g=groups.setdefault(uid,{'events':{},'source_file':'','hash':''}); g['events'][rid]={'text':text,'hash':h(text)}
        g['source_file']=g['source_file'] or marker(text)
    for g in groups.values(): g['hash']=gh(g['events'])
    return groups

def load_state():
    if not STATE.exists(): return {'version':VERSION,'uids':{}}
    data=json.loads(STATE.read_text());
    if data.get('version')!=VERSION or not isinstance(data.get('uids'),dict): raise SystemExit('Invalid calendar bridge state')
    return data

def save_state(data):
    STATE.parent.mkdir(parents=True,exist_ok=True); tmp=STATE.with_suffix('.tmp'); tmp.write_text(json.dumps(data,indent=2,sort_keys=True)+'\n'); tmp.replace(STATE)

def default_rel(uid,family): return relpath(family/(hashlib.sha256(uid.encode()).hexdigest()+'.ics'))
def pick_rel(uid,l,e,prev,family):
    for candidate in ((prev or {}).get('source_file',''), (l or {}).get('source_file',''), (e or {}).get('source_file','')):
        if candidate:
            try:
                if writable(abspath(candidate),family): return candidate
            except Exception: pass
    return default_rel(uid,family)

def write_group(rel,events):
    path=abspath(rel)
    if not events:
        path.unlink(missing_ok=True); return
    body=''.join(clean_local(v['text']).strip()+'\r\n' for _,v in sorted(events.items(),key=lambda x:(x[0]!='',x[0])))
    if path.exists():
        raw=path.read_text(encoding='utf-8',errors='replace'); base=re.sub(r'BEGIN:VEVENT.*?END:VEVENT\s*','',raw,flags=re.S); at=base.rfind('END:VCALENDAR')
        if at<0: raise RuntimeError('Invalid VCALENDAR '+str(path))
        out=base[:at].rstrip('\r\n')+'\r\n'+body+base[at:]
    else:
        out='BEGIN:VCALENDAR\r\nVERSION:2.0\r\nPRODID:-//Nodalix//Calendar Bridge//EN\r\nCALSCALE:GREGORIAN\r\n'+body+'END:VCALENDAR\r\n'
    path.parent.mkdir(parents=True,exist_ok=True); tmp=path.with_name(path.name+'.nodalix.tmp'); tmp.write_text(out); tmp.replace(path)

def mod_all():
    value=getattr(ECal.ObjModType,'ALL',None)
    return value if value is not None else ECal.ObjModType(7)

def sync_eds_group(client,uid,local_events,eds_events,rel,dry):
    if dry: return
    if not local_events:
        if eds_events: client.remove_object_sync(uid,None,mod_all(),ECal.OperationFlags.NONE,None)
        return
    if '' not in local_events and '' in eds_events: raise RuntimeError('Refusing master deletion with remaining detached instances: '+uid)
    for rid,item in sorted(local_events.items(),key=lambda x:(x[0]!='',x[0])):
        text=tagged(item['text'],rel); c=comp(text)
        if rid not in eds_events:
            if rid: client.modify_object_sync(c.get_icalcomponent(),ECal.ObjModType.THIS,ECal.OperationFlags.NONE,None)
            else: client.create_object_sync(c.get_icalcomponent(),ECal.OperationFlags.NONE,None)
        elif item['hash']!=eds_events[rid]['hash']:
            mode=mod_all() if not rid and c.has_recurrences() else ECal.ObjModType.THIS
            client.modify_object_sync(c.get_icalcomponent(),mode,ECal.OperationFlags.NONE,None)
    for rid in sorted(set(eds_events)-set(local_events),reverse=True):
        if rid: client.remove_object_sync(uid,rid,ECal.ObjModType.ONLY_THIS,ECal.OperationFlags.NONE,None)

def entry(l,e,rel,is_write): return {'source_file':rel,'writable':bool(is_write),'local_hash':l['hash'] if l else None,'eds_hash':e['hash'] if e else None}

def export_phase(client,family,dry,verbose):
    state=load_state(); st=state['uids']
    if not st: print('State not initialized; export deferred until first import.'); return 0
    local,dupes=scan_local(family); eds=scan_eds(client)
    if dupes: print('Duplicate UID across local files:',*dupes,file=sys.stderr); return 2
    next_st=dict(st); changes=deferred=conflicts=0
    for uid in sorted(set(st)|set(local)|set(eds)):
        prev=st.get(uid); l=local.get(uid); e=eds.get(uid); rel=pick_rel(uid,l,e,prev,family)
        is_write=bool((prev or {}).get('writable')) or writable(abspath(rel),family)
        if not is_write: continue
        if prev is None:
            if e and not l:
                if verbose: print('EDS -> local CREATE',uid,rel)
                if not dry: write_group(rel,e['events']); l={'hash':e['hash']}
                next_st[uid]=entry(l,e,rel,True); changes+=1
            elif l and e:
                if l['hash']==e['hash']: next_st[uid]=entry(l,e,rel,True)
                else: print('CONFLICT initial',uid); conflicts+=1
            elif l and not e: deferred+=1
            continue
        lh=l['hash'] if l else None; eh=e['hash'] if e else None
        lc=lh!=prev.get('local_hash'); ec=eh!=prev.get('eds_hash')
        if ec and not lc:
            if verbose: print('EDS -> local', 'UPDATE' if e else 'DELETE',uid,rel)
            if not dry: write_group(rel,e['events'] if e else {})
            if e: next_st[uid]={'source_file':rel,'writable':True,'local_hash':eh,'eds_hash':eh}
            else: next_st.pop(uid,None)
            changes+=1
        elif lc and not ec: deferred+=1
        elif lc and ec:
            if lh==eh: next_st[uid]=entry(l,e,rel,True)
            else: print('CONFLICT changed on both sides',uid); conflicts+=1
    if not dry and not conflicts: state['uids']=next_st; save_state(state)
    print(f'Export: changes={changes} deferred={deferred} conflicts={conflicts} dry_run={dry}')
    return 2 if conflicts else 0

def import_phase(client,family,dry,verbose):
    state=load_state(); st=state['uids']; local,dupes=scan_local(family); eds=scan_eds(client)
    if dupes: print('Duplicate UID across local files:',*dupes,file=sys.stderr); return 2
    next_st=dict(st); to_eds=to_local=deferred=conflicts=0
    for uid in sorted(set(st)|set(local)|set(eds)):
        prev=st.get(uid); l=local.get(uid); e=eds.get(uid)
        rel=((prev or {}).get('source_file') or (l or {}).get('source_file') or (e or {}).get('source_file') or '')
        if not rel and e: rel=default_rel(uid,family)
        is_write=writable(abspath(rel),family) if rel else False
        if not is_write:
            if l:
                if not e or l['hash']!=e['hash']:
                    if verbose: print('local -> EDS READONLY',uid,rel)
                    sync_eds_group(client,uid,l['events'],e['events'] if e else {},rel,dry); to_eds+=1
                next_st[uid]=entry(l, {'hash':l['hash']} if (dry or not e or l['hash']!=e['hash']) else e, rel, False)
            elif e and prev and not prev.get('writable'):
                if verbose: print('local -> EDS DELETE READONLY',uid)
                sync_eds_group(client,uid,{},e['events'],rel,dry); next_st.pop(uid,None); to_eds+=1
            continue
        if prev is None:
            if l and not e:
                if verbose: print('local -> EDS CREATE',uid,rel)
                sync_eds_group(client,uid,l['events'],{},rel,dry); next_st[uid]={'source_file':rel,'writable':True,'local_hash':l['hash'],'eds_hash':l['hash']}; to_eds+=1
            elif e and not l:
                if verbose: print('EDS -> local CREATE',uid,rel)
                if not dry: write_group(rel,e['events'])
                next_st[uid]={'source_file':rel,'writable':True,'local_hash':e['hash'],'eds_hash':e['hash']}; to_local+=1
            elif l and e:
                if l['hash']==e['hash']: next_st[uid]=entry(l,e,rel,True)
                else: print('CONFLICT initial mismatch',uid); conflicts+=1
            continue
        lh=l['hash'] if l else None; eh=e['hash'] if e else None
        lc=lh!=prev.get('local_hash'); ec=eh!=prev.get('eds_hash')
        if lc and not ec:
            if verbose: print('local -> EDS', 'UPDATE' if l else 'DELETE',uid,rel)
            sync_eds_group(client,uid,l['events'] if l else {},e['events'] if e else {},rel,dry)
            if l: next_st[uid]={'source_file':rel,'writable':True,'local_hash':lh,'eds_hash':lh}
            else: next_st.pop(uid,None)
            to_eds+=1
        elif ec and not lc: deferred+=1
        elif lc and ec:
            if lh==eh: next_st[uid]=entry(l,e,rel,True)
            else: print('CONFLICT changed on both sides',uid); conflicts+=1
        else: next_st[uid]=entry(l,e,rel,True)
    if not dry and not conflicts: state['uids']=next_st; save_state(state)
    print(f'Import: to_eds={to_eds} to_local={to_local} deferred={deferred} conflicts={conflicts} dry_run={dry} state_uids={len(next_st)}')
    return 2 if conflicts else 0

def main():
    p=argparse.ArgumentParser(); m=p.add_mutually_exclusive_group(required=True); m.add_argument('--export',action='store_true'); m.add_argument('--import',dest='do_import',action='store_true'); m.add_argument('--sync',action='store_true'); p.add_argument('--dry-run',action='store_true'); p.add_argument('--verbose',action='store_true'); a=p.parse_args()
    family=family_dir(); family.mkdir(parents=True,exist_ok=True); client=connect()
    if a.export: return export_phase(client,family,a.dry_run,a.verbose)
    if a.do_import: return import_phase(client,family,a.dry_run,a.verbose)
    code=export_phase(client,family,a.dry_run,a.verbose)
    return code or import_phase(client,family,a.dry_run,a.verbose)
if __name__=='__main__': raise SystemExit(main())
