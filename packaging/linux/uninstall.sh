#!/bin/sh
# Remove what install.sh put in place, and rebuild the caches that indexed it.
#
# Author: David M. Anderson
# Built with AI assistance (Claude, Anthropic)
set -eu

prefix="${HOME}/.local"
keep_binary=""

while [ $# -gt 0 ]; do
    case "$1" in
        --prefix) prefix="${2:?--prefix needs a directory}"; shift 2 ;;
        --keep-binary) keep_binary=yes; shift ;;
        -h|--help)
            echo "usage: uninstall.sh [--prefix DIR] [--keep-binary]"; exit 0 ;;
        *) echo "uninstall.sh: unknown argument $1" >&2; exit 2 ;;
    esac
done

rm -f \
    "${prefix}/share/mime/packages/application-x.slipcase+zip.xml" \
    "${prefix}/share/applications/slipcase-desktop.desktop" \
    "${prefix}/share/icons/hicolor/scalable/apps/slipcase-desktop.svg" \
    "${prefix}/share/icons/hicolor/scalable/mimetypes/application-x.slipcase+zip.svg"

[ -n "$keep_binary" ] || rm -f "${prefix}/bin/slipcase-desktop"

[ -x "$(command -v update-mime-database || true)" ] &&
    update-mime-database "${prefix}/share/mime" || true
[ -x "$(command -v update-desktop-database || true)" ] &&
    update-desktop-database "${prefix}/share/applications" || true
[ -x "$(command -v gtk-update-icon-cache || true)" ] &&
    gtk-update-icon-cache -q -t -f "${prefix}/share/icons/hicolor" || true

echo "removed the slipcase desktop integration from ${prefix}"
