import Gio from 'gi://Gio';
import GLib from 'gi://GLib';
import {SharingBridge} from '../gnome/extensions/glocalsend@donnybeelo.github.com/sharingBridge.js';
let refreshed = false, toggled = 0, applied = null, changed = null, disconnected = false;
const service={enabled:true, alias:'Test', fingerprint:'SELF', httpPort:53317,
    peers:[{alias:'A',fingerprint:'OTHER'},{alias:'Z',fingerprint:'FAVORITE'}],
    refreshPeers(){refreshed=true;}, toggleEnabled(){this.enabled=!this.enabled;toggled++;}, applySettings(key){applied=key;}};
const bridge=new SharingBridge(service,{get_strv(){return ['FAVORITE'];},connect(signal,callback){changed=callback;return 1;},disconnect(id){disconnected=id===1;}});
const bus=Gio.DBus.session;
function call(method, args=null){return new Promise((resolve,reject)=>bus.call(bus.get_unique_name(),'/com/nodalix/LocalSend','com.nodalix.LocalSend1',method,args,null,Gio.DBusCallFlags.NONE,1000,null,(connection,result)=>{try{resolve(connection.call_finish(result));}catch(e){reject(e);}}));}
const state=JSON.parse((await call('GetStatus')).deep_unpack()[0]);
if(state.devices[0].fingerprint!=='FAVORITE'||!state.devices[0].favorite||state.identity.protocol!=='https')throw Error('Invalid bridge metadata');
await call('Refresh');
if(!refreshed)throw Error('Refresh not delegated');
await call('SetEnabled',new GLib.Variant('(b)',[false]));
await call('SetEnabled',new GLib.Variant('(b)',[false]));
if(service.enabled||toggled!==1)throw Error('Enable operation is not idempotent');
changed(null,'alias');if(applied!=='alias')throw Error('Settings not applied');
bridge.destroy();if(!disconnected)throw Error('Settings subscription not released');
let removed=false;
try{await call('GetStatus');}catch(e){removed=true;}
if(!removed)throw Error('Object still exported after disable');
print('PASS: D-Bus bridge, favorites, refresh and cleanup');
