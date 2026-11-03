"""Exercise the packaged terminal welcome script through a real PTY."""
import errno
import fcntl
import os
from pathlib import Path
import pty
import runpy
import struct
import subprocess
import tempfile
import termios
import unittest

ROOT = Path(__file__).resolve().parents[1]
TERMINAL = ROOT/'gnome/session/terminal'


class TerminalWelcomeTests(unittest.TestCase):
    def setUp(self):
        directory = tempfile.TemporaryDirectory()
        self.addCleanup(directory.cleanup)
        self.directory = Path(directory.name)
        script = (TERMINAL/'nodalix-terminal-logo').read_text()
        for suffix in ('', '-compact'):
            path = self.directory/('terminal-logo'+suffix+'.txt')
            path.write_bytes((TERMINAL/('nodalix-logo'+suffix+'.txt')).read_bytes())
            script = script.replace('/usr/share/nodalix/terminal-logo'+suffix+'.txt', str(path))
        self.script = self.directory/'nodalix-terminal-logo'
        self.script.write_text(script)
        self.env = dict(os.environ, TERM_PROGRAM='ghostty')
        for key in ('SSH_CONNECTION', 'NODALIX_TERMINAL_LOGO'):
            self.env.pop(key, None)

    def draw(self, rows, columns, **variables):
        master, slave = pty.openpty()
        fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack('HHHH', rows, columns, 0, 0))
        try:
            with subprocess.Popen(['/bin/sh', str(self.script), '--welcome'], stdin=slave,
                                  stdout=slave, stderr=subprocess.PIPE, env={**self.env, **variables}) as process:
                os.close(slave)
                slave = None
                process.wait(timeout=5)
                self.assertEqual(process.returncode, 0, process.stderr.read().decode())
                chunks = []
                while True:
                    try:
                        chunk = os.read(master, 8192)
                    except OSError as error:
                        if error.errno != errno.EIO:
                            raise
                        break
                    if not chunk:
                        break
                    chunks.append(chunk)
                return b''.join(chunks).replace(b'\r\n', b'\n')
        finally:
            if slave is not None:
                os.close(slave)
            os.close(master)

    def test_selects_exact_artwork_and_fits_very_small_windows(self):
        self.assertEqual(self.draw(52, 100), (TERMINAL/'nodalix-logo.txt').read_bytes())
        self.assertEqual(self.draw(24, 80), (TERMINAL/'nodalix-logo-compact.txt').read_bytes())
        reduced = self.draw(13, 24).splitlines()
        self.assertTrue(reduced)
        self.assertLessEqual(len(reduced), 11)
        self.assertLessEqual(max(map(len, reduced)), 23)

    def test_skips_remote_and_noninteractive_startup_and_honors_opt_out(self):
        self.assertEqual(self.draw(24, 80, SSH_CONNECTION='remote'), b'')
        self.assertEqual(self.draw(24, 80, NODALIX_TERMINAL_LOGO='0'), b'')
        self.assertEqual(self.draw(24, 80, TERM_PROGRAM='other'), b'')
        result = subprocess.check_output(['/bin/sh', str(self.script), '--welcome'], env=self.env)
        self.assertEqual(result, b'')

    def test_shell_hook_preserves_personal_configuration_and_runs_once(self):
        configure = runpy.run_path(str(ROOT/'gnome/migration/user_preferences.py'))['configure_terminal_welcome']
        existing = b'alias personal="echo personal"\n'
        startup = self.directory/'.bashrc'
        startup.write_bytes(existing)
        configure(self.directory, '/bin/bash')
        first = startup.read_bytes()
        self.assertTrue(first.startswith(existing))
        self.assertEqual(first.count(b'. /usr/lib/nodalix/nodalix-welcome.sh'), 1)
        configure(self.directory, '/bin/bash')
        self.assertEqual(startup.read_bytes(), first)
        link = self.directory/'.zshrc'
        link.symlink_to(startup)
        configure(self.directory, '/usr/bin/zsh')
        self.assertTrue(link.is_symlink())
        self.assertEqual(startup.read_bytes(), first)
