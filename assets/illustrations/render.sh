#!/bin/sh
set -eu

cd "$(dirname "$0")"
mkdir -p png

for name in songs bible media schedule preview search; do
    rsvg-convert --width 280 --height 176 "$name.svg" --output "png/$name.png"
done
