"""Exercise real Nautilus GI and the same GJS bridge on a private D-Bus bus."""
import importlib.util,json,subprocess,time
from pathlib import Path
from gi.repository import GLib
ROOT=Path(__file__).resolve().parents[1]
spec=importlib.util.spec_from_file_location('nautilus_local',ROOT/'gnome/nautilus/nodalix_localsend.py')
module=importlib.util.module_from_spec(spec);spec.loader.exec_module(module)
service=subprocess.Popen(['gjs','-m',str(ROOT/'tests/localsend-nautilus-service.mjs')])
def until(predicate):
 deadline=time.monotonic()+5
 while time.monotonic()<deadline:
  while GLib.MainContext.default().pending():GLib.MainContext.default().iteration(False)
  if predicate():return
  time.sleep(.01)
 raise AssertionError('LocalSend state did not reach Nautilus')
try:
 until(lambda: bool(subprocess.run(['gdbus','call','--session','--dest','com.nodalix.LocalSend','--object-path','/com/nodalix/LocalSend','--method','com.nodalix.LocalSend1.GetStatus'],capture_output=True).returncode==0))
 provider=module.NodalixLocalSendMenu()
 updates=[]
 provider.connect('items-updated',lambda *_:updates.append(True))
 until(lambda:len(provider._current_devices())==1)
 assert provider._current_devices()[0]['fingerprint']=='PHONE'
 assert module.request('/status')['devices'][0]['fingerprint']=='PHONE'
 module.request('/announce',{})
 until(lambda:provider._current_devices()[0]['fingerprint']=='TABLET')
 assert updates,'Nautilus did not invalidate its menu on discovery change'
 print('PASS: Nautilus receives the GNOME quick settings devices and invalidates menus')
finally:
 service.terminate();service.wait(timeout=5)
