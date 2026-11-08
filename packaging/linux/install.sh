#!/bin/sh
set -eu

app=solideoworship
here=$(cd "$(dirname "$0")" && pwd)

case "${XDG_DATA_HOME:-}" in
    /*) data_home=$XDG_DATA_HOME ;;
    *) data_home=$HOME/.local/share ;;
esac

bin_dir=$HOME/.local/bin
applications=$data_home/applications
binary=$bin_dir/$app
desktop=$applications/$app.desktop

usage() {
    cat <<USAGE
Usage: $0 [--uninstall]

Installs SolideoWorship for the current user, without root:
  $binary
  $desktop

--uninstall removes those two files. Songs, Bibles and settings in
$data_home/$app are kept either way.
USAGE
}

desktop_exec_argument() {
    printf '%s' "$1" | sed -e 's/[\\"`$]/\\&/g' -e 's/\\/\\\\/g'
}

install_app() {
    case $binary in
        *%*)
            echo "$binary contains '%', which desktop launchers cannot start." >&2
            exit 1
            ;;
    esac

    mkdir -p "$bin_dir" "$applications"
    install -m 755 "$here/$app" "$binary"
    {
        grep -v '^Exec=' "$here/$app.desktop"
        printf 'Exec="%s"\n' "$(desktop_exec_argument "$binary")"
    } >"$desktop"
    chmod 644 "$desktop"
    echo "Installed $binary"
    echo "Installed $desktop"
}

uninstall_app() {
    rm -f "$binary" "$desktop"
    echo "Removed $binary"
    echo "Removed $desktop"
    echo "Kept $data_home/$app"
}

if [ "$(id -u)" -eq 0 ]; then
    echo "Run this as your normal user, not root: it installs into your home directory." >&2
    exit 1
fi

case "${1:-}" in
    "") install_app ;;
    --uninstall) uninstall_app ;;
    -h | --help) usage ;;
    *)
        usage >&2
        exit 64
        ;;
esac
