#!/usr/bin/python3
"""Compare installed/candidate wallpaper GPU paths in private GNOME sessions."""
import argparse,json,os,re,shutil,subprocess,sys,tempfile,time
from pathlib import Path
from gi.repository import Gio,GLib

UUID='hanabi-extension@jeffshee.github.io'
BUS='io.github.jeffshee.HanabiRenderer'
OBJ='/io/github/jeffshee/HanabiRenderer'


def sample(pid):
    proc=Path('/proc')/str(pid)
    fields=(proc/'stat').read_text().rsplit(')',1)[1].split()
    pss=re.search(r'^Pss:\s+(\d+)',(proc/'smaps_rollup').read_text(),re.M)
    clients=set();engines={};memory={}
    for path in (proc/'fdinfo').iterdir():
        try:values=dict(line.split(':',1) for line in path.read_text().splitlines() if ':' in line)
        except OSError:continue
        if 'drm-client-id' not in values:continue
        ident=tuple(values.get(k) for k in ('drm-driver','drm-pdev','drm-client-id'))
        if ident in clients:continue
        clients.add(ident)
        for key,value in values.items():
            match=re.fullmatch(r'\s*(\d+)\s+(ns|KiB|bytes)\s*',value)
            if not match:continue
            number=int(match[1])*(1024 if match[2]=='KiB' else 1)
            if key.startswith('drm-engine-') and match[2]=='ns':engines[key[11:]]=engines.get(key[11:],0)+number
            if key in ('drm-memory-vram','drm-memory-gtt','drm-total-vram','drm-total-gtt'):
                memory[key]=memory.get(key,0)+number
    return {'cpu_ticks':int(fields[11])+int(fields[12]),'pss_kib':int(pss[1]) if pss else None,'engines':engines,'gpu_memory_bytes':memory}


def delta(a,b,seconds):
    return {'cpu_percent_one_core':round((b['cpu_ticks']-a['cpu_ticks'])/os.sysconf('SC_CLK_TCK')/seconds*100,2),
        'pss_mib':round(b['pss_kib']/1024,2) if b['pss_kib'] is not None else None,
        'gpu_engine_percent':{key:round((value-a['engines'].get(key,value))/seconds/1e9*100,3) for key,value in b['engines'].items()},
        'gpu_memory_bytes':b['gpu_memory_bytes']}


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--extension',type=Path,required=True)
    parser.add_argument('--video',type=Path,required=True)
    parser.add_argument('--label',required=True)
    parser.add_argument('--output',type=Path,required=True)
    parser.add_argument('--seconds',type=int,default=12)
    parser.add_argument('--offload',action='store_true')
    args=parser.parse_args();args.output.mkdir(parents=True,exist_ok=True)
    if os.environ.get('NODALIX_GPU_PRIVATE')!='1':
        with tempfile.TemporaryDirectory(prefix='nodalix-gpu-',ignore_cleanup_errors=True) as directory:
            root=Path(directory);runtime=root/'runtime';runtime.mkdir(mode=0o700)
            env={k:v for k,v in os.environ.items() if k not in ('DISPLAY','WAYLAND_DISPLAY','DBUS_SESSION_BUS_ADDRESS','XDG_SESSION_ID','GDK_BACKEND')}
            env.update(NODALIX_GPU_PRIVATE='1',NODALIX_GPU_HOME=directory,HOME=directory,XDG_RUNTIME_DIR=str(runtime),
                XDG_CONFIG_HOME=str(root/'config'),XDG_DATA_HOME=str(root/'data'),XDG_STATE_HOME=str(root/'state'),XDG_CACHE_HOME=str(root/'cache'),XDG_DATA_DIRS='/usr/local/share:/usr/share',XDG_CURRENT_DESKTOP='GNOME',GTK_A11Y='none')
            result=subprocess.run(['dbus-run-session','--',sys.executable,__file__,*sys.argv[1:]],env=env)
            for name in ('doc','gvfs'):
                subprocess.run(['fusermount3','-uz',str(runtime/name)],stdout=subprocess.DEVNULL,stderr=subprocess.DEVNULL)
            return result.returncode
    home=Path(os.environ['NODALIX_GPU_HOME']);folder=home/'data/gnome-shell/extensions';folder.mkdir(parents=True)
    extension=folder/UUID;shutil.copytree(args.extension,extension)
    metadata=json.loads((extension/'metadata.json').read_text());metadata['version']=999999;(extension/'metadata.json').write_text(json.dumps(metadata))
    qa=folder/'nodalix-gpu-qa@example.invalid';qa.mkdir()
    (qa/'metadata.json').write_text(json.dumps({'uuid':qa.name,'name':'GPU QA','description':'Private compositor only','shell-version':['51']}))
    (qa/'extension.js').write_text("import {Extension} from 'resource:///org/gnome/shell/extensions/extension.js';export default class QA extends Extension{enable(){global.context.unsafe_mode=true;}disable(){global.context.unsafe_mode=false;}}")
    schemas=home/'schemas';schemas.mkdir()
    for source in Path('/usr/share/glib-2.0/schemas').glob('*.xml'):(schemas/source.name).symlink_to(source)
    candidate_schema=args.extension.resolve().parents[2]/'glib-2.0/schemas/io.github.jeffshee.hanabi-extension.gschema.xml'
    if candidate_schema.is_file():
        target=schemas/candidate_schema.name;target.unlink(missing_ok=True);target.symlink_to(candidate_schema)
    subprocess.run(['glib-compile-schemas','--strict',str(schemas)],check=True)
    os.environ['GSETTINGS_SCHEMA_DIR']=str(schemas)
    settings=Gio.Settings.new('io.github.jeffshee.hanabi-extension')
    for key,value in {'video-path':str(args.video.resolve()),'mute':True,'debug-mode':False,'enable-va':True,'prefer-clappersink':False,'force-mediafile':False,'random-start-position':False,'change-wallpaper':False,'show-panel-menu':False,'show-on-lock-screen':False,'pause-on-focus':False,'pause-on-mpris-playing':False}.items():
        if isinstance(value,bool):settings.set_boolean(key,value)
        else:settings.set_string(key,value)
    settings.set_boolean('enable-graphics-offload',args.offload)
    settings.set_int('pause-on-maximize-or-fullscreen',0);settings.set_int('pause-on-battery',0)
    Gio.Settings.new('org.gnome.desktop.screensaver').set_boolean('lock-enabled',False)
    Gio.Settings.new('org.gnome.desktop.session').set_uint('idle-delay',0)
    shell=Gio.Settings.new('org.gnome.shell');shell.set_strv('enabled-extensions',[qa.name,UUID]);Gio.Settings.sync()
    log=(args.output/(args.label+'.log')).open('w')
    process=subprocess.Popen(['gnome-shell','--headless','--wayland','--no-x11','--debug-control','--virtual-monitor','1920x1080','--virtual-monitor','1920x1080'],stdout=log,stderr=log)
    try:
        bus=Gio.bus_get_sync(Gio.BusType.SESSION,None)
        def call(name,path,interface,method,params=None):return bus.call_sync(name,path,interface,method,params,None,Gio.DBusCallFlags.NONE,3000,None).unpack()
        def evaluate(code):
            result=call('org.gnome.Shell','/org/gnome/Shell','org.gnome.Shell','Eval',GLib.Variant('(s)',(code,)))
            if not result[0]:raise AssertionError(result)
            return json.loads(result[1])
        for attempt in range(120):
            if process.poll() is not None:raise AssertionError('Compositor exited')
            try:
                owner=call('org.freedesktop.DBus','/org/freedesktop/DBus','org.freedesktop.DBus','GetNameOwner',GLib.Variant('(s)',(BUS,)))[0]
                playing=call(BUS,OBJ,'org.freedesktop.DBus.Properties','Get',GLib.Variant('(ss)',(BUS,'isPlaying')))[0]
                if isinstance(playing,GLib.Variant):playing=playing.unpack()
                if playing:break
            except GLib.Error:pass
            time.sleep(.25)
        else:raise AssertionError('Wallpaper renderer never played')
        pid=call('org.freedesktop.DBus','/org/freedesktop/DBus','org.freedesktop.DBus','GetConnectionUnixProcessID',GLib.Variant('(s)',(owner,)))[0]
        evaluate('Main.overview.hide();true');time.sleep(4)
        state=evaluate("({extension:Main.extensionManager.lookup('hanabi-extension@jeffshee.github.io').state,error:Main.extensionManager.lookup('hanabi-extension@jeffshee.github.io').error,backgrounds:Main.layoutManager._bgManagers.length,monitors:Main.layoutManager.monitors.length,hasClip:typeof Main.panel.has_clip,hasEffects:typeof Main.panel.get_effects,groupChildren:Main.layoutManager._backgroundGroup.get_children().length,mode:Main.sessionMode.currentMode})")
        assert state['extension']==1 and state['backgrounds']==2,state
        visible=evaluate("Main.layoutManager._bgManagers.map(m=>m.backgroundActor.get_children().filter(a=>a.constructor.name.includes('LiveWallpaper')).map(a=>({opacity:a.opacity,clone:!!a.wallpaper?.get_source()})))")
        assert all(items and items[0]['opacity']==255 and items[0]['clone'] for items in visible),visible
        def capture(name):
            path=args.output/(args.label+'-'+name+'.png')
            result=call('org.gnome.Shell.Screenshot','/org/gnome/Shell/Screenshot','org.gnome.Shell.Screenshot','Screenshot',GLib.Variant('(bbs)',(False,False,str(path.resolve()))))
            assert result[0],result
            return path
        first=capture('before')
        start=time.monotonic();before={name:sample(value) for name,value in [('shell',process.pid),('renderer',pid)]}
        time.sleep(args.seconds)
        elapsed=time.monotonic()-start;after={name:sample(value) for name,value in [('shell',process.pid),('renderer',pid)]}
        second=capture('after')
        from PIL import Image,ImageChops,ImageStat
        a=Image.open(first).convert('RGB');b=Image.open(second).convert('RGB')
        motion=[]
        for x in (0,1920):
            box=(x+100,100,x+1820,980)
            left=a.crop(box);right=b.crop(box)
            diff=ImageChops.difference(left,right)
            change=sum(ImageStat.Stat(diff).mean)
            assert sum(ImageStat.Stat(right).mean)>1,'Wallpaper is black'
            assert change>.01,'Wallpaper has no changing video pixels'
            motion.append(round(change,4))
        result={'visible_motion_rgb_difference':motion,'label':args.label,'duration_seconds':round(elapsed,2),'video':json.loads(subprocess.check_output(['ffprobe','-v','error','-select_streams','v:0','-show_entries','stream=codec_name,width,height,r_frame_rate','-of','json',str(args.video)],text=True))['streams'],
            'playing':{name:delta(before[name],after[name],elapsed) for name in before},
            'wallpaper_windows':evaluate("global.display.list_all_windows().filter(w=>w.get_gtk_application_id()==='io.github.jeffshee.HanabiRenderer').length"),
            'background_effects':evaluate("Main.layoutManager._bgManagers.map(m=>m.backgroundActor.get_effects().map(e=>({type:e.constructor.name,enabled:e.get_enabled()})))")}
        if settings.props.settings_schema.has_key('pause-when-covered'):
            # Eight ordinary, unmaximized windows cover both monitors. The old
            # maximize/fullscreen policy cannot account for this workload.
            fixture="""import gi
gi.require_version('Gtk','4.0')
from gi.repository import Gtk
app=Gtk.Application(application_id='com.nodalix.CoverageQA')
def activate(app):
    for i in range(8):
        win=Gtk.ApplicationWindow(application=app,title='Nodalix Coverage QA '+str(i),default_width=200,default_height=100)
        win.set_decorated(False);win.set_child(Gtk.Label(label='Wallpaper coverage QA'));win.present()
app.connect('activate',activate);app.run([])
"""
            runtime=Path(os.environ['XDG_RUNTIME_DIR'])
            env={**os.environ,'GDK_BACKEND':'wayland','WAYLAND_DISPLAY':next(p.name for p in runtime.glob('wayland-*') if p.is_socket())}
            windows=subprocess.Popen([sys.executable,'-c',fixture],env=env,stdout=log,stderr=log)
            try:
                for _ in range(60):
                    count=evaluate("global.display.list_all_windows().filter(w=>w.title?.startsWith('Nodalix Coverage QA ')).length")
                    if count==8:break
                    time.sleep(.1)
                assert count==8,count
                time.sleep(.7)
                geometry=evaluate("global.display.list_all_windows().filter(w=>w.title?.startsWith('Nodalix Coverage QA ')).sort((a,b)=>a.title.localeCompare(b.title)).map((w,i)=>{let m=Math.floor(i/4),n=i%4,a=Main.layoutManager.getWorkAreaForMonitor(m),x=a.x+(n%2)*Math.floor(a.width/2),y=a.y+Math.floor(n/2)*Math.floor(a.height/2);w.move_resize_frame(false,x,y,Math.ceil(a.width/2),Math.ceil(a.height/2));return {maximized:w.is_maximized(),fullscreen:w.fullscreen};})")
                assert not any(w['maximized'] or w['fullscreen'] for w in geometry),geometry
                def is_playing():
                    value=call(BUS,OBJ,'org.freedesktop.DBus.Properties','Get',GLib.Variant('(ss)',(BUS,'isPlaying')))[0]
                    return value.unpack() if isinstance(value,GLib.Variant) else value
                for _ in range(50):
                    if not is_playing():break
                    time.sleep(.1)
                assert not is_playing(),'Covered desktop kept decoding'
                time.sleep(.5)
                before={name:sample(value) for name,value in [('shell',process.pid),('renderer',pid)]};start=time.monotonic();time.sleep(3);elapsed=time.monotonic()-start
                after={name:sample(value) for name,value in [('shell',process.pid),('renderer',pid)]}
                result['covered_by_tiled_windows']={name:delta(before[name],after[name],elapsed) for name in before}
                evaluate("global.display.list_all_windows().filter(w=>w.title?.startsWith('Nodalix Coverage QA ')).forEach(w=>w.minimize());true")
                for _ in range(50):
                    if is_playing():break
                    time.sleep(.1)
                assert is_playing(),'Revealed desktop did not resume'
                result['resumed_after_reveal']=True
                evaluate('Main.overview.show();true');time.sleep(1)
                result['overview_effects']=evaluate("(() => {let out=[];function visit(a){for(let e of a.get_effects())if(e.constructor.name.includes('RoundedCornersEffect'))out.push({type:e.constructor.name,enabled:e.get_enabled()});for(let c of a.get_children())visit(c);}visit(global.stage);return out;})()")
                assert any(e['enabled'] for e in result['overview_effects']),result['overview_effects']
                evaluate('Main.overview.hide();true');time.sleep(1)
            finally:
                windows.terminate();windows.wait(timeout=5)
        call(BUS,OBJ,BUS,'setPause');time.sleep(.5)
        before={name:sample(value) for name,value in [('shell',process.pid),('renderer',pid)]};start=time.monotonic();time.sleep(3);elapsed=time.monotonic()-start
        after={name:sample(value) for name,value in [('shell',process.pid),('renderer',pid)]}
        result['paused']={name:delta(before[name],after[name],elapsed) for name in before}
        log.flush();text=(args.output/(args.label+'.log')).read_text()
        result['render_log']=[line for line in text.splitlines() if any(tag in line for tag in ('using glsinkbin','using gtk4paintablesink','Nodalix video route:','Nodalix video decoder:'))]
        (args.output/(args.label+'.json')).write_text(json.dumps(result,indent=2)+'\n')
        print(json.dumps(result),flush=True)
    finally:
        shell.set_strv('enabled-extensions',[]);Gio.Settings.sync();time.sleep(.3)
        process.terminate()
        try:process.wait(timeout=8)
        except subprocess.TimeoutExpired:process.kill();process.wait()
        log.close()
    return 0

if __name__=='__main__':raise SystemExit(main())
