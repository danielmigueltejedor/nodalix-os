import Gio from 'gi://Gio';
import GLib from 'gi://GLib';
import {SharingBridge} from '../gnome/extensions/glocalsend@donnybeelo.github.com/sharingBridge.js';
const service={enabled:true,alias:'Test',fingerprint:'SELF',httpPort:53317,
 peers:[{alias:'Phone',fingerprint:'PHONE',ip:'192.0.2.1',port:53317,protocol:'https'}],
 refreshPeers(){this.peers=[{alias:'Tablet',fingerprint:'TABLET'}];bridge.changed();},
 toggleEnabled(){this.enabled=!this.enabled;},applySettings(){}};
const bridge=new SharingBridge(service,{get_strv(){return [];},connect(){return 1;},disconnect(){}});
const loop=new GLib.MainLoop(null,false);
loop.run();
