import importlib.util
from pathlib import Path
import unittest
from unittest.mock import patch

spec = importlib.util.spec_from_file_location('window_actions', Path(__file__).parents[1] / 'shell/scripts/nodalix-window-action.py')
actions = importlib.util.module_from_spec(spec)
spec.loader.exec_module(actions)

class WindowActionsTest(unittest.TestCase):
    def setUp(self):
        self.window = {'address': '0xab', 'monitor': 1, 'workspace': {'name': '2'}}
        self.monitor = {'id': 1, 'name': 'DP-1', 'activeWorkspace': {'id': 2}, 'focused': True}

    def run_action(self, action, monitors=None):
        def query(name):
            return [self.window] if name == 'clients' else (monitors or [self.monitor])
        with patch.object(actions, 'query', side_effect=query), patch.object(actions, 'dispatch') as dispatch:
            actions.act(action, 'ab', 'DP-1')
        return [call.args[0] for call in dispatch.call_args_list]

    def test_minimize_does_not_follow_hidden_workspace(self):
        commands = self.run_action('minimize')
        self.assertEqual(len(commands), 1)
        self.assertIn('follow=false', commands[0])
        self.assertIn(actions.HIDDEN, commands[0])

    def test_restoring_hidden_window_uses_clicked_monitor_workspace(self):
        self.window['workspace']['name'] = actions.HIDDEN
        commands = self.run_action('activate')
        self.assertIn('workspace=2,follow=false', commands[0])
        self.assertIn('hl.dsp.focus', commands[1])
        self.assertIn('alter_zorder', commands[2])

    def test_existing_window_is_focused_without_moving(self):
        self.assertFalse(any('window.move' in command for command in self.run_action('activate')))

    def test_old_visible_scratchpad_is_closed(self):
        self.monitor['specialWorkspace'] = {'name': actions.HIDDEN}
        self.assertTrue(any('toggle_special' in command for command in self.run_action('minimize')))

    def test_invalid_address_rejected_before_query(self):
        with patch.object(actions, 'query') as query, self.assertRaises(ValueError):
            actions.act('activate', 'invalid')
        query.assert_not_called()

    def test_compositor_errors_are_not_success(self):
        with patch.object(actions.subprocess, 'check_output', return_value='error: invalid dispatcher'), self.assertRaises(RuntimeError):
            actions.dispatch('bad')
