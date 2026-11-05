#!/usr/bin/python3
"""Keep GDM's intrinsic logo size small without changing the original artwork."""
import base64
from pathlib import Path
import sys

script = Path(__file__).resolve()
source = script.parents[2] / 'docs/images/nodalix-logo.png'
template = script.with_name('nodalix-login-logo.svg').read_text(encoding='utf-8')
encoded = base64.b64encode(source.read_bytes()).decode('ascii')
sys.stdout.write(template.replace('@NODALIX_LOGO_DATA_URI@', 'data:image/png;base64,' + encoded))
