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
icons=$data_home/icons/hicolor
binary=$bin_dir/$app
desktop=$applications/$app.desktop

usage() {
    cat <<USAGE
Usage: $0 [--yes] [--uninstall]

Installs SolideoWorship for the current user, without root:
  $binary
  $desktop
  $icons/*/apps/$app.*

--yes accepts the privacy policy without prompting.
--uninstall removes them, along with a menu entry and icons left in
~/.local/share by an install that used a different XDG_DATA_HOME.
Songs, Bibles and settings in $data_home/$app are kept either way.
USAGE
}

confirm_privacy() {
    [ "$assume_yes" = true ] && return 0
    [ -t 0 ] || return 0

    cat <<PRIVACY
SolideoWorship Privacy Policy (https://solideoworship.com/privacy)
- Songs, Bibles, schedules, and files outside the app never leave your computer.
- Anonymous daily usage stats (app version, OS) can be disabled in Settings -> General.
- Crash reports are saved locally first and sent only with your consent.

PRIVACY
    printf "Do you accept the Privacy Policy and GPL-3.0 license? [y/N]: "
    read -r answer
    case $answer in
        [yY] | [yY][eE][sS] | [yY][aA]) ;;
        *)
            echo "Installation cancelled." >&2
            exit 1
            ;;
    esac
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

    confirm_privacy

    mkdir -p "$bin_dir" "$applications"
    install -m 755 "$here/$app" "$binary"
    {
        grep -v '^Exec=' "$here/$app.desktop"
        printf 'Exec="%s"\n' "$(desktop_exec_argument "$binary")"
    } >"$desktop"
    chmod 644 "$desktop"
    for png in "$here"/icons/*.png; do
        size=$(basename "$png" .png)
        install -D -m 644 "$png" "$icons/${size}x$size/apps/$app.png"
    done
    install -D -m 644 "$here/icons/scalable.svg" "$icons/scalable/apps/$app.svg"
    refresh_icons "$icons"
    echo "Installed $binary"
    echo "Installed $desktop"
    echo "Installed icons in $icons"
    echo "Privacy policy: https://solideoworship.com/privacy"
}

refresh_icons() {
    [ -d "$1" ] || return 0
    touch "$1"
    if [ -f "$1/icon-theme.cache" ] && command -v gtk-update-icon-cache >/dev/null 2>&1; then
        gtk-update-icon-cache -q -t -f "$1" || true
    fi
}

remove_icons() {
    found=false
    for file in "$1"/*/apps/"$app".png "$1/scalable/apps/$app.svg"; do
        if [ -e "$file" ] || [ -L "$file" ]; then
            rm -f "$file"
            found=true
        fi
    done
    if [ "$found" = true ]; then
        refresh_icons "$1"
        echo "Removed icons from $1"
        removed=true
    fi
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
    remove_icons "$icons"
    remove_icons "$HOME/.local/share/icons/hicolor"

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

assume_yes=false

case "${1:-}" in
    "") install_app ;;
    -y | --yes)
        assume_yes=true
        install_app
        ;;
    --uninstall) uninstall_app ;;
    -h | --help) usage ;;
    *)
        usage >&2
        exit 64
        ;;
esac
