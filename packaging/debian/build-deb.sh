#!/bin/sh
# Build the binary package DESIGN.md §8 ships through the Excelano apt
# repository: the executable, the desktop entry, and the application icon.
#
# Not the media type and not the icon a container is drawn with. Those are
# `slipcase-common`'s, which this package depends on: two packages cannot ship
# one path, and the type has to be declared once for every slipcase product
# rather than once per product.
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

# Through `version.sh`, which is the only thing here that reads Cargo.toml. It
# exits non-zero and says why if there is no version to read.
version=$("${here}/../version.sh")
# The changelog names the version being built, or the package ships release
# notes for something else. This check is what lets the changelog be
# hand-written: the one thing generating it from `git log` would buy is that it
# cannot go stale, and a file refused unless it matches cannot either.
changelog_version=$(sed -n '1s/^[^(]*(\([^)]*\)).*/\1/p' "${here}/changelog")
[ "$changelog_version" = "$version" ] || {
    echo "build-deb.sh: the changelog's newest entry is ${changelog_version:-unreadable}," \
         "and Cargo.toml says ${version}" >&2
    echo "build-deb.sh: add an entry to packaging/debian/changelog before building" >&2
    exit 1
}

arch=$(dpkg-architecture -qDEB_HOST_ARCH)

# The architecture the package declares has to be the architecture the
# executable actually is. `dpkg-architecture` answers about this machine, which
# is right for a native build and says nothing about a binary handed over with
# `--binary` — and a package declaring amd64 while carrying an arm64 executable
# installs perfectly and then does not run. `build-msix.ps1` reads the PE header
# for this exact reason and this is the same check in ELF's shape.
#
# `e_machine` is two little-endian bytes at offset 18, after the four magic
# bytes have said this is an ELF at all. 62 is x86-64 and 183 is AArch64.
magic=$(od -An -tx1 -N4 "$binary" | tr -d ' \n')
[ "$magic" = "7f454c46" ] || {
    echo "build-deb.sh: $binary is not an ELF executable" >&2
    exit 1
}
lo=$(od -An -tu1 -j18 -N1 "$binary" | tr -d ' ')
hi=$(od -An -tu1 -j19 -N1 "$binary" | tr -d ' ')
machine=$((lo + hi * 256))
case "$arch" in
    amd64) want=62 ;;
    arm64) want=183 ;;
    *) want="" ;;
esac
if [ -n "$want" ] && [ "$machine" != "$want" ]; then
    echo "build-deb.sh: this machine is ${arch}, which wants ELF machine ${want}," \
         "and ${binary} is machine ${machine}" >&2
    echo "build-deb.sh: build the package on the architecture it is for, or the" \
         ".deb will declare one thing and carry another" >&2
    exit 1
fi

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
    "${stage}/usr/share/applications" \
    "${stage}/usr/share/icons/hicolor/scalable/apps" \
    "${stage}/usr/share/man/man1" \
    "${stage}/usr/share/doc/slipcase-desktop"

install -m 0755 "$binary" "${stage}/usr/bin/slipcase-desktop"
install -m 0644 "${here}/../linux/slipcase-desktop.desktop" \
    "${stage}/usr/share/applications/slipcase-desktop.desktop"
install -m 0644 "${here}/../linux/icons/slipcase-desktop.svg" \
    "${stage}/usr/share/icons/hicolor/scalable/apps/slipcase-desktop.svg"
install -m 0644 "${root}/LICENSE" "${stage}/usr/share/doc/slipcase-desktop/copyright"

# Debian policy wants a changelog in every binary package, and lintian makes
# its absence an error rather than a warning: somebody installing from an apt
# repository has no other way to see what changed between two versions, and
# DESIGN.md §8 sends this package through one.
#
# Hand-written rather than derived from `git log`, which was the alternative
# considered. Deriving it would need a git checkout, which `--binary` exists so
# that a build does not need; there are no release tags to divide the history
# into versions, so every release would re-list every commit; and a commit
# subject here is written for whoever maintains this rather than for whoever is
# deciding whether to upgrade. `git log` stays the record of why the code is
# the way it is. The changelog is the record of what changed for a person who
# installed it, and they are not the same document.
#
# `-n` so the compressed copy carries no name or timestamp of its own. That is
# what lintian's `package-contains-timestamped-gzip` is about, and it is also
# what makes two builds of the same source produce the same bytes.
gzip -9nc "${here}/changelog" \
    > "${stage}/usr/share/doc/slipcase-desktop/changelog.gz"
chmod 0644 "${stage}/usr/share/doc/slipcase-desktop/changelog.gz"

# `slipcase-desktop` reads one optional positional argument and has no
# `--help`, so this page is the only place that argument is written down.
# `@VERSION@` is substituted the same way `control.in`'s is, so the version in
# the page header cannot drift from the version of the package carrying it.
sed "s/@VERSION@/${version}/" "${here}/slipcase-desktop.1.in" \
    | gzip -9nc > "${stage}/usr/share/man/man1/slipcase-desktop.1.gz"
chmod 0644 "${stage}/usr/share/man/man1/slipcase-desktop.1.gz"

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
