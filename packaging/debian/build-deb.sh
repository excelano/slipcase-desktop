#!/bin/sh
# Build the binary package DESIGN.md §8 ships through the Excelano apt
# repository: the executable, the media type, the desktop entry, and the icons.
#
# A binary package rather than a source package. Everything here is one static
# Rust executable and four data files, and the archive is assembled from a
# staging tree the same way `dpkg-deb` would assemble it from `debian/rules`.
#
# Author: David M. Anderson
# Built with AI assistance (Claude, Anthropic)
set -eu

here=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
root=$(CDPATH= cd -- "${here}/../.." && pwd)
binary=""
outdir="${root}/dist"

usage() {
    cat <<'USAGE'
usage: build-deb.sh [--binary PATH] [--outdir DIR]

  --binary PATH  the executable to package (default: the release build)
  --outdir DIR   where to write the .deb (default: ./dist)
USAGE
}

while [ $# -gt 0 ]; do
    case "$1" in
        --binary) binary="${2:?--binary needs a path}"; shift 2 ;;
        --outdir) outdir="${2:?--outdir needs a directory}"; shift 2 ;;
        -h|--help) usage; exit 0 ;;
        *) echo "build-deb.sh: unknown argument $1" >&2; usage >&2; exit 2 ;;
    esac
done

# Cargo is asked where its target directory is. `[build] target-dir` in a Cargo
# configuration file moves it and no environment variable then says so.
if [ -z "$binary" ]; then
    target_dir=$(cd "$root" && cargo metadata --format-version 1 --no-deps |
        sed -n 's/.*"target_directory":"\([^"]*\)".*/\1/p')
    binary="${target_dir}/release/slipcase-desktop"
fi
[ -x "$binary" ] || {
    echo "build-deb.sh: no executable at $binary — run 'cargo build --release' first" >&2
    exit 1
}

version=$(sed -n 's/^version *= *"\([^"]*\)".*/\1/p' "${root}/Cargo.toml" | head -1)
[ -n "$version" ] || { echo "build-deb.sh: no version in Cargo.toml" >&2; exit 1; }
arch=$(dpkg-architecture -qDEB_HOST_ARCH)
name="slipcase-desktop_${version}_${arch}"

stage=$(mktemp -d)
trap 'rm -rf "$stage"' EXIT
# mktemp makes it 0700, and dpkg-deb records the staging root as the package's
# own `./`, so without this every install leaves an unreadable directory mode
# behind it.
chmod 0755 "$stage"

mkdir -p \
    "${stage}/DEBIAN" \
    "${stage}/usr/bin" \
    "${stage}/usr/share/mime/packages" \
    "${stage}/usr/share/applications" \
    "${stage}/usr/share/icons/hicolor/scalable/apps" \
    "${stage}/usr/share/icons/hicolor/scalable/mimetypes" \
    "${stage}/usr/share/doc/slipcase-desktop"

install -m 0755 "$binary" "${stage}/usr/bin/slipcase-desktop"
install -m 0644 "${here}/../linux/application-x.slipcase+zip.xml" \
    "${stage}/usr/share/mime/packages/application-x.slipcase+zip.xml"
install -m 0644 "${here}/../linux/slipcase-desktop.desktop" \
    "${stage}/usr/share/applications/slipcase-desktop.desktop"
install -m 0644 "${here}/../linux/icons/slipcase-desktop.svg" \
    "${stage}/usr/share/icons/hicolor/scalable/apps/slipcase-desktop.svg"
install -m 0644 "${here}/../linux/icons/application-x.slipcase+zip.svg" \
    "${stage}/usr/share/icons/hicolor/scalable/mimetypes/application-x.slipcase+zip.svg"
install -m 0644 "${root}/LICENSE" "${stage}/usr/share/doc/slipcase-desktop/copyright"

# Stripped here rather than by the build profile, so a developer's release
# binary keeps its symbols and only the packaged copy loses them.
strip --strip-unneeded "${stage}/usr/bin/slipcase-desktop" 2>/dev/null || true

# The umask of whoever ran this is not a packaging decision. `install -m`
# already fixed every file; this fixes the directories they sit in, so the
# archive does not carry one builder's 0775 onto every machine that installs it.
find "$stage" -type d -exec chmod 0755 {} +

sed -e "s/@VERSION@/${version}/" -e "s/@ARCH@/${arch}/" \
    -e "s/@SIZE@/$(du -ks "${stage}/usr" | cut -f1)/" \
    "${here}/control.in" > "${stage}/DEBIAN/control"


# `dpkg -V` verifies an installed copy against this file, and its absence is
# what lintian tags `no-md5sums-control-file`. Generated after the strip above,
# so the hash recorded is the hash of the binary that actually ships. `%P`
# prints the path without the leading `./` that dpkg does not want, and
# `DEBIAN` is excluded because a package does not checksum its own control
# files.
(
    cd "$stage"
    find . -type f ! -path './DEBIAN/*' -printf '%P\0' \
        | sort -z \
        | xargs -0 --no-run-if-empty md5sum > DEBIAN/md5sums
)
chmod 0644 "${stage}/DEBIAN/md5sums"

mkdir -p "$outdir"
dpkg-deb --root-owner-group --build "$stage" "${outdir}/${name}.deb" >/dev/null
echo "${outdir}/${name}.deb"

# What the executable links, against what the package declares. Most of what
# this application needs is opened by name at run time and appears in neither
# list, which is why `Depends` below is written by hand and why this prints the
# comparison rather than deriving one from the other.
echo
echo "linked at load time:"
objdump -p "$binary" | awk '/NEEDED/ {print "  " $2}'
echo "declared in Depends:"
sed -n 's/^Depends: //p' "${stage}/DEBIAN/control" | tr ',' '\n' | sed 's/^ */  /'
