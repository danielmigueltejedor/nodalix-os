#!/usr/bin/python
from pathlib import Path
import subprocess, tempfile, shutil, os, json, threading, configparser
import gi
gi.require_version('Gtk','3.0')
from gi.repository import Gtk, GLib

class Access(Gtk.Application):
    def __init__(self):
        super().__init__(application_id='com.nodalix.iCloudAccess')
        self.connect('activate',self.show_window);self.state=None;self.temp=None
    def show_window(self,*args):
        if hasattr(self,'window'): self.window.present();return
        self.window=Gtk.ApplicationWindow(application=self,title='Acceso a iCloud');self.window.set_default_size(450,240)
        box=Gtk.Box(orientation=Gtk.Orientation.VERTICAL,spacing=16);box.set_border_width(24);self.window.add(box)
        self.label=Gtk.Label(label='Renueva el acceso a iCloud para conectar tus archivos y continuar los respaldos pendientes.');self.label.set_line_wrap(True);self.label.set_xalign(0);box.pack_start(self.label,True,True,0)
        self.code=Gtk.Entry();self.code.set_placeholder_text('Código de Apple de seis cifras');self.code.set_max_length(6);self.code.set_input_purpose(Gtk.InputPurpose.DIGITS);box.pack_start(self.code,False,False,0)
        self.button=Gtk.Button(label='Solicitar verificación de Apple');self.button.connect('clicked',self.start);box.pack_start(self.button,False,False,0)
        self.again=Gtk.Button(label='Solicitar otro código');self.again.connect('clicked',self.retry);box.pack_start(self.again,False,False,0)
        self.window.show_all();self.code.hide();self.again.hide();self.window.present()
    def start(self,*args):
        code=self.code.get_text().strip() if self.state else None
        if self.state and (len(code)!=6 or not code.isdigit()): self.label.set_text('Introduce el código de seis cifras que muestra Apple.');return
        self.button.set_sensitive(False);self.again.set_sensitive(False);self.code.set_sensitive(False);self.label.set_text('Conectando con Apple…')
        threading.Thread(target=self.request,args=(code,),daemon=True).start()
    def retry(self,*args):
        self.state=None
        if self.temp:self.temp.unlink(missing_ok=True)
        self.temp=None;self.start()
    def request(self,code):
        try:
            config=Path.home()/'.config/rclone/rclone.conf'
            if self.temp is None:
                fd,path=tempfile.mkstemp(prefix='icloud-renovar-',suffix='.conf',dir=config.parent);os.close(fd);self.temp=Path(path);shutil.copyfile(config,self.temp);self.temp.chmod(0o600)
            command=['rclone','config','update','icloud','--config',str(self.temp),'--non-interactive']
            if self.state: command+=['--continue','--state',self.state,'--result',code]
            result=subprocess.run(command,capture_output=True,text=True,timeout=90)
            if result.returncode: raise RuntimeError('No se ha podido conectar con Apple. Comprueba la conexión y vuelve a intentarlo.')
            data=json.loads(result.stdout) if result.stdout.strip() else {}
            if data.get('State'):
                if data.get('Option',{}).get('Name')!='config_2fa': raise RuntimeError('Apple solicita una comprobación adicional. Autoriza el acceso a iCloud en tu dispositivo y vuelve a intentarlo.')
                self.state=data['State'];GLib.idle_add(self.ask_code,bool(data.get('Error')));return
            probe=subprocess.run(['rclone','lsf','icloud:','--config',str(self.temp),'--max-depth','1','--retries','1','--low-level-retries','1'],capture_output=True,text=True,timeout=60)
            if probe.returncode: raise RuntimeError('Apple todavía no permite acceder a iCloud. Comprueba el aviso de autorización en tu dispositivo.')
            # Merge only this account, preserving any other configured cloud drives.
            original=configparser.ConfigParser(interpolation=None);original.read(config);updated=configparser.ConfigParser(interpolation=None);updated.read(self.temp);original['icloud']=dict(updated['icloud'])
            fd,path=tempfile.mkstemp(prefix='icloud-config-',dir=config.parent)
            with os.fdopen(fd,'w') as stream: original.write(stream);stream.flush();os.fsync(stream.fileno())
            os.chmod(path,0o600);os.replace(path,config)
            subprocess.run(['systemctl','--user','restart','nodalix-icloud-drive.service'],check=True,timeout=60)
            if subprocess.run(['systemctl','--user','cat','nodalix-icloud-backup-upload.service'],stdout=subprocess.DEVNULL,stderr=subprocess.DEVNULL).returncode==0: subprocess.run(['systemctl','--user','start','--no-block','nodalix-icloud-backup-upload.service'],check=True)
            self.temp.unlink(missing_ok=True);self.temp=None;self.state=None;GLib.idle_add(self.success)
        except Exception as error: GLib.idle_add(self.failure,str(error))
    def ask_code(self,invalid):
        self.label.set_text(('El código no es válido o ha caducado. ' if invalid else '')+'Pulsa «Permitir» en el aviso de Apple e introduce aquí el código que aparece.');self.code.show();self.code.set_sensitive(True);self.code.set_text('');self.code.grab_focus();self.button.set_label('Verificar código');self.button.set_sensitive(True);self.again.show();self.again.set_sensitive(True);return False
    def success(self):
        self.label.set_text('iCloud vuelve a estar conectado. Los respaldos pendientes se retomarán en segundo plano.');self.code.hide();self.again.hide();self.button.set_label('Cerrar');self.button.set_sensitive(True);self.button.disconnect_by_func(self.start);self.button.connect('clicked',lambda *_:self.quit());return False
    def failure(self,message):
        self.label.set_text(message);self.button.set_sensitive(True);self.again.set_sensitive(True);self.code.set_sensitive(True);return False

app=Access()
try: app.run([])
finally:
    if app.temp: app.temp.unlink(missing_ok=True)
