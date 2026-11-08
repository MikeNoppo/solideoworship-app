#!/bin/sh
set -eu

if [ $# -ne 1 ]; then
    echo "Usage: $0 <version>" >&2
    exit 64
fi

version=$1
here=$(cd "$(dirname "$0")" && pwd)
root=$(cd "$here/../.." && pwd)

cd "$root"
cargo build --release --locked -p sdw-app

target=$(cargo metadata --format-version 1 --no-deps | sed -n 's/.*"target_directory":"\([^"]*\)".*/\1/p')

if [ -z "$target" ]; then
    echo "Could not read the cargo target directory." >&2
    exit 1
fi
name=SolideoWorship-$version-linux-$(uname -m)
dist=$target/dist
stage=$dist/$name

rm -rf "$stage" "$dist/$name.tar.gz"
mkdir -p "$stage"
install -m 755 "$target/release/sdw-app" "$stage/solideoworship"
install -m 755 "$here/install.sh" "$stage/install.sh"
install -m 644 "$here/solideoworship.desktop" "$stage/solideoworship.desktop"
install -m 644 "$root/LICENSE" "$stage/LICENSE"
install -m 644 "$root/assets/fonts/Inter-LICENSE.txt" "$stage/LICENSE-Inter.txt"
install -m 644 "$root/assets/fonts/Lucide-LICENSE.txt" "$stage/LICENSE-Lucide.txt"
mkdir -p "$stage/icons"
for size in 16 24 32 48 64 128 256; do
    install -m 644 "$root/assets/logo/png/app-icon-$size.png" "$stage/icons/$size.png"
done
install -m 644 "$root/assets/logo/app-icon.svg" "$stage/icons/scalable.svg"
tar -C "$dist" --owner=0 --group=0 --numeric-owner -czf "$dist/$name.tar.gz" "$name"
echo "$dist/$name.tar.gz"
