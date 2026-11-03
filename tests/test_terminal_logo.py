"""Exercise the packaged terminal welcome script through a real PTY."""
import errno
import fcntl
import os
from pathlib import Path
import pty
import re
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
        fixtures = {
            '/etc/nodalix-release': 'VERSION_ID="0.4.0"\n',
            '/proc/meminfo': 'MemTotal:       33554432 kB\nMemAvailable:   25165824 kB\n',
            '/proc/uptime': '19200.20 40000.00\n',
        }
        for source, contents in fixtures.items():
            fixture = self.directory/Path(source).name
            fixture.write_text(contents)
            script = script.replace(source, str(fixture))
        for command, result in {
            'uname': 'Linux 7.3.1 x86_64',
            'df': 'Type  1K-blocks  Used  Use%\nbtrfs  1073741824  536870912  50%',
        }.items():
            stub = self.directory/command
            stub.write_text(f"#!/bin/sh\nprintf '%s\\n' '{result}'\n")
            stub.chmod(0o700)
        self.script = self.directory/'nodalix-terminal-logo'
        self.script.write_text(script)
        self.env = dict(os.environ, TERM_PROGRAM='ghostty', TERM='xterm-256color', LC_ALL='C', NO_COLOR='1',
                        PATH=f'{self.directory}:{os.environ["PATH"]}')
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

    def test_selects_exact_artwork_and_puts_live_information_beside_it(self):
        for suffix, rows, columns in (('', 52, 128), ('-compact', 24, 80)):
            with self.subTest(suffix=suffix):
                art = (TERMINAL/('nodalix-logo'+suffix+'.txt')).read_bytes().splitlines()
                width = max(map(len, art))
                lines = self.draw(rows, columns, LC_ALL='es_ES.UTF-8').splitlines()
                self.assertEqual([line[:width].rstrip() for line in lines], art)
                information = [line[width+4:] for line in lines if len(line) > width]
                self.assertEqual(information, [
                    b'Nodalix OS 0.4.0',
                    b'-' * len(b'Disco     512/1024 GiB 50% btrfs'),
                    b'Sistema   x86_64',
                    b'Kernel    Linux 7.3.1',
                    b'Actividad 5 h 20 min',
                    b'Memoria   8.0/32.0 GiB 25%',
                    b'Disco     512/1024 GiB 50% btrfs',
                ])

    def test_color_stays_inside_the_welcome_and_preserves_layout(self):
        color = self.draw(24, 80, NO_COLOR='')
        self.assertGreater(len(set(re.findall(rb'\x1b\[38;2;[\d;]+m', color))), 3)
        for line in color.splitlines():
            if b'\x1b[' in line:
                self.assertGreater(line.rfind(b'\x1b[0m'), line.rfind(b'\x1b[38;2;'))
        plain = re.sub(rb'\x1b\[[\d;]+m', b'', color).replace('─'.encode(), b'-')
        self.assertEqual(plain, self.draw(24, 80))
        self.assertEqual(self.draw(24, 80, TERM='dumb', NO_COLOR=''), plain)

    def test_information_moves_below_art_in_a_narrow_window(self):
        narrow = self.draw(32, 38).splitlines()
        self.assertIn(b'Nodalix OS 0.4.0', narrow)
        self.assertEqual(narrow[-1], b'Disk      512/1024 GiB 50% btrfs')
        self.assertGreater(narrow.index(b'Nodalix OS 0.4.0'), 5)
        reduced = self.draw(13, 24).splitlines()
        self.assertTrue(reduced)
        self.assertLessEqual(len(reduced), 11)
        self.assertLessEqual(max(map(len, reduced)), 23)
        self.assertIn(b'Nodalix OS 0.4.0', reduced)

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

    def test_only_the_known_arch_card_is_removed_and_backed_up(self):
        prefs = runpy.run_path(str(ROOT/'gnome/migration/user_preferences.py'))
        config = self.directory/'fish/config.fish'
        config.parent.mkdir()
        custom = b'# My prompt\nstarship init fish | source\n'
        original = prefs['LEGACY_TERMINAL_CARD'] + custom
        config.write_bytes(original)
        config.chmod(0o640)
        starship = self.directory/'starship.toml'
        prompt = 'format = """\n[](bold #1793d1) $directory$git_branch$git_status$character"""\n'
        starship.write_text(prompt+'[character]\nsuccess_symbol = ">"\n')
        retire = prefs['retire_legacy_terminal_card']
        retire(self.directory)
        retire(self.directory)
        self.assertEqual(config.read_bytes(), custom)
        self.assertEqual(config.stat().st_mode & 0o777, 0o640)
        self.assertEqual(config.with_name('config.fish.pre-nodalix-terminal').read_bytes(), original)
        self.assertEqual(starship.read_text(),
                         prompt.replace('[](bold #1793d1) ', '')+'[character]\nsuccess_symbol = ">"\n')
