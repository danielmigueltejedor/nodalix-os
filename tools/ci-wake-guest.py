#!/usr/bin/python3
"""Wake a disposable QEMU guest after observing an actual ACPI suspend."""
import json
import socket
import time
from pathlib import Path


def wake_guest(connection, delay=5):
    suspended = False
    woke = False
    with connection.makefile('rwb') as channel:
        greeting = json.loads(channel.readline())
        if 'QMP' not in greeting:
            raise RuntimeError('Missing QMP greeting')
        channel.write(b'{"execute":"qmp_capabilities"}\n')
        channel.flush()
        for line in channel:
            message = json.loads(line)
            if 'error' in message:
                raise RuntimeError(message['error'])
            if message.get('event') == 'SUSPEND':
                suspended = True
                time.sleep(delay)
                channel.write(b'{"execute":"system_wakeup"}\n')
                channel.flush()
            if message.get('event') == 'WAKEUP':
                woke = True
                print('QMP observed ACPI suspend and wakeup', flush=True)
    if not (suspended and woke):
        raise RuntimeError('The guest did not complete ACPI suspend/wakeup')


def main():
    path = Path.cwd() / 'qmp.sock'
    deadline = time.monotonic() + 30
    with socket.socket(socket.AF_UNIX) as connection:
        while True:
            try:
                connection.connect(str(path))
                break
            except (FileNotFoundError, ConnectionRefusedError):
                if time.monotonic() >= deadline:
                    raise
                time.sleep(.1)
        connection.settimeout(900)
        wake_guest(connection)


if __name__ == '__main__':
    main()
