#!/bin/sh
set -eu

cd "$(dirname "$0")"
mkdir -p png

render() {
    rsvg-convert --width "$2" --height "$3" "$1.svg" --output "png/$4.png"
}

for size in 16 24; do
    render app-icon-16 "$size" "$size" "app-icon-$size"
done
render app-icon-32 32 32 app-icon-32
for size in 48 64 128 256 512 1024; do
    render app-icon "$size" "$size" "app-icon-$size"
done
render installer-wizard 202 386 installer-wizard-202x386
render installer-wizard 404 772 installer-wizard-404x772
render social-preview 1280 640 social-preview

python3 - solideoworship.ico png/app-icon-16.png png/app-icon-24.png png/app-icon-32.png \
    png/app-icon-48.png png/app-icon-64.png png/app-icon-128.png png/app-icon-256.png <<'PY'
import struct
import sys

target, *sources = sys.argv[1:]
images = [open(path, "rb").read() for path in sources]
header = struct.pack("<HHH", 0, 1, len(images))
offset = len(header) + 16 * len(images)
entries = b""
for image in images:
    width, height = struct.unpack(">II", image[16:24])
    entries += struct.pack("<BBBBHHII", width % 256, height % 256, 0, 0, 1, 32, len(image), offset)
    offset += len(image)
with open(target, "wb") as icon:
    icon.write(header + entries + b"".join(images))
PY
