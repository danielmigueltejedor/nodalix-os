#!/usr/bin/env python3
"""Dock window actions for Hyprland's Lua API."""
import argparse
import json
import re
import subprocess

HIDDEN = 'special:nodalix-minimized'

def query(name):
    return json.loads(subprocess.check_output(['hyprctl', name, '-j'], text=True))

def dispatch(expression):
    reply = subprocess.check_output(['hyprctl', 'dispatch', expression], text=True)
    if 'error:' in reply.lower():
        raise RuntimeError(reply.strip())

def act(action, address, monitor_name=''):
    address = address if address.startswith('0x') else '0x' + address
    if not re.fullmatch(r'0x[0-9a-fA-F]+', address):
        raise ValueError('invalid window address')
    clients = query('clients')
    window = next((w for w in clients if w['address'] == address), None)
    if not window:
        return
    selector = json.dumps('address:' + address)
    monitors = query('monitors')
    monitor = next((m for m in monitors if m['name'] == monitor_name), None)
    monitor = monitor or next((m for m in monitors if m['id'] == window['monitor']), monitors[0])
    if action == 'minimize':
        dispatch('hl.dsp.window.move({window=' + selector + ',workspace="' + HIDDEN + '",follow=false})')
        # Repair a scratchpad left visible by older versions of the dock.
        refreshed = query('monitors')
        shown = next((m for m in refreshed if m.get('specialWorkspace', {}).get('name') == HIDDEN), None)
        if shown:
            focused = next((m for m in refreshed if m.get('focused')), None)
            dispatch('hl.dsp.focus({monitor=' + json.dumps(shown['name']) + '})')
            dispatch('hl.dsp.workspace.toggle_special("nodalix-minimized")')
            if focused and focused['name'] != shown['name']:
                dispatch('hl.dsp.focus({monitor=' + json.dumps(focused['name']) + '})')
    elif action == 'activate':
        if window.get('workspace', {}).get('name') == HIDDEN:
            workspace = monitor['activeWorkspace']['id']
            dispatch('hl.dsp.window.move({window=' + selector + ',workspace=' + str(workspace) + ',follow=false})')
        dispatch('hl.dsp.focus({window=' + selector + '})')
        dispatch('hl.dsp.window.alter_zorder({window=' + selector + ',mode="top"})')
    elif action == 'maximize':
        dispatch('hl.dsp.window.fullscreen({window=' + selector + ',mode="maximized",action="toggle"})')

def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('action', choices=('minimize', 'activate', 'maximize'))
    parser.add_argument('address')
    parser.add_argument('--monitor', default='')
    args = parser.parse_args()
    act(args.action, args.address, args.monitor)

if __name__ == '__main__': main()
