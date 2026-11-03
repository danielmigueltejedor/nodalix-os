#!/usr/bin/python3
"""Skip the optional mount unless its named remote is configured; never prompt."""
import os
from pathlib import Path
import subprocess


def configured(config):
    try:
        result=subprocess.run(['/usr/bin/rclone','listremotes','--ask-password=false','--config='+str(config)],
                              stdin=subprocess.DEVNULL,capture_output=True,text=True,timeout=10)
        return result.returncode==0 and 'icloud:' in result.stdout.splitlines()
    except (OSError,subprocess.SubprocessError):return False


if __name__=='__main__':
    config=Path(os.environ.get('XDG_CONFIG_HOME',str(Path.home()/'.config')))/'rclone/rclone.conf'
    raise SystemExit(0 if configured(config) else 1)
