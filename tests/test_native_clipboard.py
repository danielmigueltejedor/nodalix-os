"""Native-shortcut clipboard operations cannot become a general input injector."""
import importlib.util
import errno
from pathlib import Path
import unittest
from unittest import mock

ROOT=Path(__file__).resolve().parents[1]
spec=importlib.util.spec_from_file_location('native_clipboard',ROOT/'gnome/session/keyboard/native_clipboard.py')
native=importlib.util.module_from_spec(spec);spec.loader.exec_module(native)


class NativeClipboard(unittest.TestCase):
    def test_only_the_active_unlocked_local_seat_user_is_allowed(self):
        self.assertTrue(native.allowed_seat(1000,1000,True,False,'wayland',False))
        for args in [(1001,1000,True,False,'wayland',False),(1000,1000,False,False,'wayland',False),
                     (1000,1000,True,True,'wayland',False),(1000,1000,True,False,'tty',False),
                     (1000,1000,True,False,'wayland',True)]:
            self.assertFalse(native.allowed_seat(*args))

    def test_waits_for_real_super_and_control_release_instead_of_a_fixed_delay(self):
        states=iter([True,True,False]);sleeps=[]
        native.wait_released([7],pressed=lambda fds:next(states),clock=lambda:0,sleep=sleeps.append)
        self.assertEqual(len(sleeps),2)

    def test_a_held_modifier_times_out_without_injecting_input(self):
        clock=iter([0,4])
        with self.assertRaises(TimeoutError):
            native.wait_released([],pressed=lambda fds:True,clock=lambda:next(clock),sleep=lambda _:None)

    def test_modifier_probe_uses_a_bitmap_and_never_reads_key_events(self):
        def ioctl(fd,request,buffer,mutate):buffer[125//8]|=1<<(125%8)
        with mock.patch.object(native.fcntl,'ioctl',side_effect=ioctl),mock.patch.object(native.os,'read') as read:
            self.assertTrue(native.modifiers_held([7]));read.assert_not_called()

    def test_unknown_operations_do_not_open_any_device(self):
        keyboard=native.Keyboard()
        with mock.patch.object(native.os,'open') as opened:
            with self.assertRaises(ValueError):keyboard.edit('arbitrary-command')
            opened.assert_not_called()

    def test_unreadable_modifier_state_fails_closed(self):
        with mock.patch.object(native.fcntl,'ioctl',side_effect=OSError(errno.EACCES,'denied')):
            with self.assertRaises(OSError):native.modifiers_held([7])
        with mock.patch.object(native.fcntl,'ioctl',side_effect=OSError(errno.ENODEV,'disconnected')):
            self.assertFalse(native.modifiers_held([7]))

    def test_clipboard_chords_never_send_ctrl_c_to_a_terminal_process(self):
        self.assertEqual(set(native.CHORDS),{'copy','paste'})
        self.assertTrue(all(key==110 for modifier,key in native.CHORDS.values()))
        self.assertNotIn((29,46),native.CHORDS.values())


if __name__=='__main__':unittest.main()
