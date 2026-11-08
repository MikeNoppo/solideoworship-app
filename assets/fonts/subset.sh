#!/bin/sh
set -eu

INTER_URL=https://github.com/rsms/inter/releases/download/v4.1/Inter-4.1.zip
INTER_SHA256=9883fdd4a49d4fb66bd8177ba6625ef9a64aa45899767dde3d36aa425756b11e
MONO_URL=https://github.com/JetBrains/JetBrainsMono/releases/download/v2.304/JetBrainsMono-2.304.zip
MONO_SHA256=6f6376c6ed2960ea8a963cd7387ec9d76e3f629125bc33d1fdcd7eb7012f7bbf
LUCIDE_URL=https://registry.npmjs.org/lucide-static/-/lucide-static-1.70.0.tgz
LUCIDE_SHA256=3d1f09df1c468abe512ee6d6596db0dc773fa46c61050e365f0c5fee5120f16a

TEXT_RANGES=U+0020-007E,U+00A0-017F,U+0218-021B,U+2010-2027,U+2030-203A,U+20AC,U+2122,U+2190-2193,U+21B5,U+2212
TEXT_FEATURES=kern,liga,calt,ccmp,locl,mark,mkmk,tnum

ICONS="arrow-left arrow-right book-open book-x check chevron-down circle-alert circle-check
circle-dashed circle-x corner-down-left database download eraser eye file-check file-code
file-plus file-text folder-open history info laptop library list-plus loader-circle monitor
monitor-off monitor-up monitor-x mouse-pointer-click music palette pencil play plus refresh-cw
rows-3 save search search-x square tag text-align-center text-align-end text-align-start
trash-2 unplug x"

here=$(cd "$(dirname "$0")" && pwd)
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT

fetch() {
    curl -sSfL -o "$work/$2" "$1"
    echo "$3  $work/$2" | sha256sum -c --quiet -
}

fetch "$INTER_URL" inter.zip "$INTER_SHA256"
fetch "$MONO_URL" mono.zip "$MONO_SHA256"
fetch "$LUCIDE_URL" lucide.tgz "$LUCIDE_SHA256"
unzip -q "$work/inter.zip" -d "$work/inter"
unzip -q "$work/mono.zip" -d "$work/mono"
tar -xzf "$work/lucide.tgz" -C "$work"

for weight in Regular Medium SemiBold Bold; do
    pyftsubset "$work/inter/extras/ttf/Inter-$weight.ttf" \
        --unicodes="$TEXT_RANGES" \
        --layout-features="$TEXT_FEATURES" \
        --name-IDs="*" \
        --no-hinting \
        --output-file="$here/Inter-$weight.ttf"
done
cp "$work/inter/LICENSE.txt" "$here/Inter-LICENSE.txt"

pyftsubset "$work/mono/fonts/ttf/JetBrainsMono-Regular.ttf" \
    --unicodes="$TEXT_RANGES" \
    --layout-features="$TEXT_FEATURES" \
    --name-IDs="*" \
    --no-hinting \
    --output-file="$here/JetBrainsMono-Regular.ttf"
cp "$work/mono/OFL.txt" "$here/JetBrainsMono-LICENSE.txt"

codepoints=$(python3 -I - "$work/package/font/codepoints.json" $ICONS <<'PY'
import json
import sys

table = json.load(open(sys.argv[1]))
print(",".join(f"U+{table[name]:04X}" for name in sys.argv[2:]))
PY
)
pyftsubset "$work/package/font/lucide.ttf" \
    --unicodes="$codepoints" \
    --no-hinting \
    --output-file="$here/lucide.ttf"
cp "$work/package/LICENSE" "$here/Lucide-LICENSE.txt"

python3 -I - "$work/package/font/codepoints.json" $ICONS <<'PY'
import json
import sys

table = json.load(open(sys.argv[1]))
for name in sys.argv[2:]:
    print(f"{name.upper().replace('-', '_')} = '\\u{{{table[name]:x}}}'")
PY
