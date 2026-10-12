#!/usr/bin/python3
"""Keep GDM's intrinsic logo size small without changing the original artwork."""
import base64
from pathlib import Path
import sys

source, target = map(Path, sys.argv[1:])
template = Path(__file__).with_name('nodalix-login-logo.svg').read_text()
encoded = base64.b64encode(source.read_bytes()).decode('ascii')
target.parent.mkdir(parents=True, exist_ok=True)
target.write_text(template.replace('@NODALIX_LOGO_DATA_URI@', 'data:image/png;base64,'+encoded))
