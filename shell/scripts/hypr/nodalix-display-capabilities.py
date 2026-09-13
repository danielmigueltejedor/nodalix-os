#!/usr/bin/env python3
"""Report color/HDR capabilities advertised by connected DRM displays."""

import glob
import json
import os
import re
import subprocess


def number(text, pattern, default=-1.0):
    match = re.search(pattern, text, re.IGNORECASE)
    return float(match.group(1)) if match else default


result = {}
for connector in glob.glob("/sys/class/drm/card*-*"):
    status = os.path.join(connector, "status")
    edid = os.path.join(connector, "edid")
    try:
        if open(status, encoding="ascii").read().strip() != "connected":
            continue
        decoded = subprocess.run(
            ["edid-decode", edid], capture_output=True, text=True, check=False
        ).stdout
        if "EDID Structure Version" not in decoded:
            continue
    except (OSError, UnicodeError):
        continue

    name = re.sub(r"^card\d+-", "", os.path.basename(connector))
    hdr_block = "HDR Static Metadata Data Block" in decoded
    pq = "SMPTE ST2084" in decoded
    bit_depth = int(number(decoded, r"Bits per primary color channel:\s*(\d+)", 8))
    result[name] = {
        "hdr": hdr_block and pq,
        "wideColor": "BT2020RGB" in decoded,
        "bitDepth": bit_depth,
        "maxLuminance": number(
            decoded, r"Desired content max luminance:.*?\(([0-9.]+) cd/m\^2\)"
        ),
        "maxAverageLuminance": number(
            decoded,
            r"Desired content max frame-average luminance:.*?\(([0-9.]+) cd/m\^2\)",
        ),
        "minLuminance": number(
            decoded, r"Desired content min luminance:.*?\(([0-9.]+) cd/m\^2\)"
        ),
    }

print(json.dumps(result, separators=(",", ":")))
