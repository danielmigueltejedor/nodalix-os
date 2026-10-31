"""Pure update-state helpers shared by the native Settings service and tests."""
import json
import os
from pathlib import Path
import re
import tempfile


def identity_label(release=Path('/etc/nodalix-release'), config=Path('/etc/nodalix/updater.json')):
    try:
        match = re.search(r'^VERSION_ID="?([^"\n]+)', release.read_text(), re.M)
        if not match or not re.fullmatch(r'\d+\.\d+\.\d+(?:-[A-Za-z0-9.-]+)?',match[1]) or match[1]=='0.0.0':
            raise ValueError('Invalid identity')
        channel=json.loads(config.read_text()).get('channel') if config.is_file() else None
        return 'Nodalix '+match[1]+(' · '+channel if channel in ('stable','beta') else ' · canal desconocido')
    except (OSError,ValueError):
        return 'No se puede verificar la versión instalada. Revisa /etc/nodalix-release y el paquete nodalix-release.'


def sanitize(text):
    text = re.sub(r'(https?://)[^\s/@]+:[^\s/@]+@', r'\1[redacted]@', text)
    text = re.sub(r'/home/[^/\s]+', '/home/[user]', text)
    return re.sub(r'(?i)(token|password|authorization)([=: ]+)\S+', r'\1\2[redacted]', text)


def save(path, state):
    path = Path(path)
    path.parent.mkdir(parents=True, exist_ok=True, mode=0o700)
    fd, name = tempfile.mkstemp(prefix='.update-', dir=path.parent)
    try:
        with os.fdopen(fd, 'w') as stream:
            json.dump(state, stream, ensure_ascii=False)
        os.replace(name, path)
    finally:
        Path(name).unlink(missing_ok=True)


def restore(path, boot_id):
    try:
        state = json.loads(Path(path).read_text())
        if not isinstance(state, dict): return {}
        result = {key: state[key] for key in ('log', 'message', 'checked_at', 'phase',
                  'reboot_boot_id', 'reboot_reason') if isinstance(state.get(key),str)}
        result['log'] = result.get('log','')[-24000:]
        result['reboot_required'] = state.get('reboot_required') is True
        if state.get('busy') or state.get('checking'):
            result.update(phase='failed', message='La operación se interrumpió. Revisa el registro antes de reintentar.')
        if result.get('reboot_boot_id') != boot_id:
            result.update(reboot_required=False, reboot_reason='')
        return result
    except (OSError, ValueError):
        return {}


def package_updates(output):
    rows = []
    for line in output.splitlines():
        match = re.fullmatch(r'([a-zA-Z0-9@._+:-]+) (\S+) -> (\S+)', line.strip())
        if match:
            rows.append(dict(zip(('name', 'installed', 'available'), match.groups())))
    return rows


def failure_hint(text):
    lower = text.lower()
    cases = [
        (('signature', 'gpg', 'keyring', 'firma', 'unknown public key'),
         'No se pudo verificar la confianza de los paquetes. Revisa el keyring y las firmas; no desactives SigLevel.'),
        (('db.lck', 'unable to lock', 'locked', 'bloqueo'),
         'Otro gestor de paquetes está trabajando. Espera a que termine y vuelve a intentarlo; no borres el bloqueo.'),
        (('no space', 'not enough free', 'espacio'), 'Falta espacio libre. Libera espacio y vuelve a comprobar.'),
        (('404', 'failed retrieving', 'resolve host', 'download', 'descarga'),
         'No se pudo descargar un paquete. Revisa la conexión y los mirrors y vuelve a comprobar.'),
        (('conflicting files', 'exists in filesystem', 'conflict', 'dependencies'),
         'Hay un conflicto de paquetes o dependencias. Revisa el registro antes de reintentar.'),
        (('hook',), 'Falló un hook. Revisa el registro y el estado de los paquetes antes de reiniciar.'),
    ]
    return next((message for needles, message in cases if any(word in lower for word in needles)),
                'Revisa el registro de la operación antes de volver a intentarlo.')
