#!/bin/sh
# Install the desktop integration DESIGN.md §8 describes: the desktop entry and
# the application icon. Optionally the binary alongside them.
#
# The media type and the icon a container is drawn with are not here. They are
# `slipcase-common`'s, declared once for every slipcase product because two
# packages cannot ship one path; install that first, or the entry below has no
# type to be associated with.
#
# For a person installing by hand and for testing the association without
# building a package. The Excelano apt repository ships the same files from
# `packaging/debian`, and the two must agree about where things go.
#
# Author: David M. Anderson
# Built with AI assistance (Claude, Anthropic)
set -eu

here=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
prefix="${HOME}/.local"
binary=""
found_binary=""

usage() {
    cat <<'USAGE'
usage: install.sh [--prefix DIR] [--binary PATH] [--no-binary]

  --prefix DIR   where to install (default: ~/.local; use /usr/local for all users)
  --binary PATH  the executable to install into PREFIX/bin
  --no-binary    install the desktop integration only

With neither --binary nor --no-binary, a built executable is looked for under
CARGO_TARGET_DIR and ./target, release before debug, and installed if found.
USAGE
}

while [ $# -gt 0 ]; do
    case "$1" in
        --prefix) prefix="${2:?--prefix needs a directory}"; shift 2 ;;
        --binary) binary="${2:?--binary needs a path}"; shift 2 ;;
        --no-binary) binary="none"; shift ;;
        -h|--help) usage; exit 0 ;;
        *) echo "install.sh: unknown argument $1" >&2; usage >&2; exit 2 ;;
    esac
done

# The executable, where one was not named and one was not refused.
#
# Cargo is asked where its target directory is rather than guessed at, because
# `[build] target-dir` in a Cargo configuration file moves it and no environment
# variable then says so. Guessing found nothing on the machine this was written
# on, which is how the guessing came out.
if [ -z "$binary" ]; then
    target_dir=""
    if command -v cargo >/dev/null 2>&1; then
        target_dir=$(cd "${here}/../.." && cargo metadata --format-version 1 --no-deps 2>/dev/null |
            sed -n 's/.*"target_directory":"\([^"]*\)".*/\1/p')
    fi
    [ -n "$target_dir" ] || target_dir="${here}/../../target"

    for candidate in "${target_dir}/release/slipcase-desktop" "${target_dir}/debug/slipcase-desktop"
    do
        if [ -x "$candidate" ]; then found_binary="$candidate"; break; fi
    done
elif [ "$binary" != "none" ]; then
    [ -x "$binary" ] || { echo "install.sh: $binary is not an executable" >&2; exit 1; }
    found_binary="$binary"
fi

mkdir -p \
    "${prefix}/share/applications" \
    "${prefix}/share/icons/hicolor/scalable/apps"

install -m 0644 "${here}/slipcase-desktop.desktop" \
    "${prefix}/share/applications/slipcase-desktop.desktop"
install -m 0644 "${here}/icons/slipcase-desktop.svg" \
    "${prefix}/share/icons/hicolor/scalable/apps/slipcase-desktop.svg"

if [ -n "$found_binary" ]; then
    mkdir -p "${prefix}/bin"
    install -m 0755 "$found_binary" "${prefix}/bin/slipcase-desktop"
    echo "installed ${prefix}/bin/slipcase-desktop from ${found_binary}"
else
    echo "no executable installed; slipcase-desktop must be on PATH for the entry to work"
fi

# Each is absent on a minimal system and each failure is survivable: the files
# are in place either way and the next login or the next package installation
# rebuilds these caches.
[ -x "$(command -v update-desktop-database || true)" ] &&
    update-desktop-database "${prefix}/share/applications" || true
[ -x "$(command -v gtk-update-icon-cache || true)" ] &&
    gtk-update-icon-cache -q -t -f "${prefix}/share/icons/hicolor" || true

echo "installed the slipcase desktop entry and application icon under ${prefix}"

# Said rather than assumed. An entry naming a type nothing has declared is an
# entry no file manager will offer, and the symptom looks like an association
# fight rather than a missing package. Asked of `share/mime/types`, which is
# what `update-mime-database` writes, rather than of the filenames in
# `packages/`: every product names its declaration differently.
if ! grep -qsx 'application/x.slipcase+zip' \
        "${prefix}/share/mime/types" \
        /usr/local/share/mime/types \
        /usr/share/mime/types
then
    echo
    echo "The slipcase media type is not declared on this machine."
    echo "Install slipcase-common, or run its install.sh, or nothing will"
    echo "associate a .slpc with this application."
fi

echo
echo "check it with:"
echo "  xdg-mime query filetype SOME.slpc     # application/x.slipcase+zip"
echo "  xdg-mime query default application/x.slipcase+zip"
