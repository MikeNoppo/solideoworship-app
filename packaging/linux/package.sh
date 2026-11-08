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
tar -C "$dist" --owner=0 --group=0 --numeric-owner -czf "$dist/$name.tar.gz" "$name"
echo "$dist/$name.tar.gz"
