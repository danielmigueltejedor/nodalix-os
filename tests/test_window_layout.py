import importlib.util
import json
import os
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch, Mock
ROOT=Path(__file__).resolve().parents[1]
spec=importlib.util.spec_from_file_location('layout_mode',ROOT/'shell/scripts/nodalix-layout-mode.py')
mode=importlib.util.module_from_spec(spec);spec.loader.exec_module(mode)
class WindowLayout(unittest.TestCase):
    def test_failed_reload_restores_saved_profile(self):
        with tempfile.TemporaryDirectory() as folder, patch.dict(os.environ,{'XDG_STATE_HOME':folder}):
            state=Path(folder)/'nodalix';state.mkdir()
            (state/'layout.generated.lua').write_text('old');(state/'layout-mode').write_text('tiling\n')
            with patch.object(mode.subprocess,'run',return_value=Mock(returncode=1)):
                self.assertEqual(mode.apply('desktop'),1)
            self.assertEqual((state/'layout.generated.lua').read_text(),'old')
            self.assertEqual((state/'layout-mode').read_text(),'tiling\n')
    def test_desktop_and_restore_use_lua_and_preserve_existing_floats(self):
        with tempfile.TemporaryDirectory() as folder, patch.dict(os.environ,{'XDG_STATE_HOME':folder}):
            clients=[{'address':'0xabc','floating':False,'workspace':{'id':1}}, {'address':'0xdef','floating':True,'workspace':{'id':1}}]
            with patch.object(mode.subprocess,'run',return_value=Mock(returncode=0)) as run, patch.object(mode.subprocess,'check_output',return_value=json.dumps(clients)):
                self.assertEqual(mode.apply('desktop'),0)
                commands=[c.args[0] for c in run.call_args_list]
                self.assertIn(['hyprctl','dispatch','hl.dsp.window.float({action="set",window="address:0xabc"})'],commands)
                run.reset_mock();self.assertEqual(mode.apply('tiling'),0)
                commands=[c.args[0] for c in run.call_args_list]
                self.assertIn(['hyprctl','dispatch','hl.dsp.window.float({action="unset",window="address:0xabc"})'],commands)
                self.assertFalse(any('0xdef' in str(c) for c in commands))
    def test_tab_navigation_does_not_replace_overview(self):
        self.assertNotIn('hl.bind("SUPER + TAB"',mode.lua_for('tabs'))
