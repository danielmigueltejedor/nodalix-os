import importlib.util
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

spec = importlib.util.spec_from_file_location('controls', Path(__file__).parents[1] / 'shell/scripts/nodalix-window-controls.py')
controls = importlib.util.module_from_spec(spec)
spec.loader.exec_module(controls)

class NativeWindowControlsTest(unittest.TestCase):
    def test_incompatible_compositor_never_loads_plugin(self):
        with tempfile.TemporaryDirectory() as tmp:
            plugin=Path(tmp)/'plugin.so'; plugin.touch()
            stamp=Path(tmp)/'commit'; stamp.write_text('compiled-build')
            with patch.object(controls,'PLUGIN',plugin),patch.object(controls,'STAMP',stamp),patch.object(controls.subprocess,'check_output',side_effect=['[]','{"commit":"different-build"}']),patch.object(controls.subprocess,'run') as run:
                self.assertEqual(controls.main(),0)
                run.assert_not_called()

    def test_already_loaded_is_not_loaded_twice(self):
        with patch.object(controls.subprocess,'check_output',return_value='[{"name":"nodalix-window-controls"}]'),patch.object(controls.subprocess,'run') as run:
            self.assertEqual(controls.main(),0)
            run.assert_not_called()
