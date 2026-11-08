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

--uninstall removes them, along with a menu entry left in
~/.local/share/applications by an install that used a different
XDG_DATA_HOME. Songs, Bibles and settings in $data_home/$app
are kept either way.
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
    removed=false
    menu_removed=false

    for file in "$binary" "$desktop" "$HOME/.local/share/applications/$app.desktop"; do
        if [ -e "$file" ] || [ -L "$file" ]; then
            rm -f "$file"
            echo "Removed $file"
            removed=true
            [ "$file" = "$binary" ] || menu_removed=true
        fi
    done

    if [ "$removed" = false ]; then
        echo "SolideoWorship is not installed for this user." >&2
        exit 1
    fi

    if [ "$menu_removed" = false ]; then
        echo "No menu entry found in $applications. If it was installed with another XDG_DATA_HOME, run --uninstall again with that value." >&2
    fi

    if [ -d "$data_home/$app" ]; then
        echo "Kept $data_home/$app"
    fi
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
