#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")"
DIRNAME="$PWD"
TMP="$DIRNAME/tmp"
WHALE='#003048'
SVG="$DIRNAME/public/assets/logo.svg"
OUT="$DIRNAME/public/favicon.ico"

mkdir -p "$TMP"
cd "$TMP"

# Rasterize the 4K Cam wordmark SVG at each ICO size (do not downscale a PNG).
for size in 16 32 48 128 256; do
    rsvg-convert -w "$size" -h "$size" -f png -o "wordmark_${size}.png" "$SVG"
    magick -size "${size}x${size}" "xc:${WHALE}" "wordmark_${size}.png" \
        -compose over -composite "logo_${size}.png"
done

python3 - "$OUT" << 'PY'
import io
import struct
import sys
from pathlib import Path

from PIL import Image

out = Path(sys.argv[1])
sizes = (16, 32, 48, 128, 256)
pngs = []
for size in sizes:
    image = Image.open(f"logo_{size}.png").convert("RGBA")
    buf = io.BytesIO()
    image.save(buf, format="PNG", optimize=True)
    pngs.append((size, buf.getvalue()))

count = len(pngs)
header = struct.pack("<HHH", 0, 1, count)
offset = 6 + 16 * count
entries = b""
payload = b""
for size, data in pngs:
    width = 0 if size == 256 else size
    height = 0 if size == 256 else size
    entries += struct.pack("<BBBBHHII", width, height, 0, 0, 1, 32, len(data), offset)
    payload += data
    offset += len(data)

out.write_bytes(header + entries + payload)
PY

cd "$DIRNAME"
rm -rf "$TMP"

echo "favicon creation completed!"
