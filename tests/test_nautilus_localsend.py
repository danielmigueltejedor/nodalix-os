import subprocess,unittest
from pathlib import Path
class NautilusLocalSendTests(unittest.TestCase):
 def test_real_bridge_discovery_reaches_nautilus(self):
  result=subprocess.run(['dbus-run-session','--','/usr/bin/python3',str(Path(__file__).with_name('localsend-nautilus-client.py'))],capture_output=True,text=True,timeout=15)
  self.assertEqual(result.returncode,0,result.stdout+result.stderr)
