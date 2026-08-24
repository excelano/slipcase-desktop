#!/bin/sh
# Assemble the application bundle DESIGN.md §8 describes: the executable, the
# property list that exports the slipcase type and claims it, and the icon.
#
# The bundle is the unit of everything on macOS. A bare executable can draw a
# window, and it was how this application was first run here, but it has no
# bundle identifier, Launch Services files it as a nameless foreground process,
# and nothing can be registered or associated with it. `lsappinfo` reports
# `bundleID=[ NULL ]` for one, which is the whole reason this script exists.
#
# Code signing and notarization are out of scope. `README.md` beside this file
# records what an unsigned bundle does when it is double-clicked, so that
# decision has a measurement behind it when somebody takes it.
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
usage: build-app.sh [--binary PATH] [--outdir DIR]

  --binary PATH  the executable to bundle (default: the release build)
  --outdir DIR   where to write Slipcase.app (default: ./dist)
USAGE
}

while [ $# -gt 0 ]; do
    case "$1" in
        --binary) binary="${2:?--binary needs a path}"; shift 2 ;;
        --outdir) outdir="${2:?--outdir needs a directory}"; shift 2 ;;
        -h|--help) usage; exit 0 ;;
        *) echo "build-app.sh: unknown argument $1" >&2; usage >&2; exit 2 ;;
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
    echo "build-app.sh: no executable at $binary — run 'cargo build --release' first" >&2
    exit 1
}

version=$(sed -n 's/^version *= *"\([^"]*\)".*/\1/p' "${root}/Cargo.toml" | head -1)
[ -n "$version" ] || { echo "build-app.sh: no version in Cargo.toml" >&2; exit 1; }

app="${outdir}/Slipcase.app"
rm -rf "$app"
mkdir -p "${app}/Contents/MacOS" "${app}/Contents/Resources"

# The icon comes from the one drawing every platform's icon comes from, which
# `packaging/README.md` names as the source. macOS wants a raster at ten sizes
# in an `.iconset` directory, and `iconutil` turns that into the `.icns`.
#
# `sips` reads the SVG, but it rasterizes at whatever width and height the
# document declares and then resamples to the size asked for, so rendering the
# 64-unit source at 1024 gives a soft upscale of a 64-pixel bitmap. Rewriting
# the declared size first makes each rendering a true one at that size, which
# is also how the Linux icon theme draws the same file. Measured both ways at
# 16 pixels before this was written.
stage=$(mktemp -d)
trap 'rm -rf "$stage"' EXIT
iconset="${stage}/slipcase-desktop.iconset"
mkdir -p "$iconset"
svg="${root}/packaging/linux/icons/slipcase-desktop.svg"
[ -f "$svg" ] || { echo "build-app.sh: no icon source at $svg" >&2; exit 1; }

render() {
    size=$1
    out=$2
    sed -E "s/(<svg[^>]*)width=\"[0-9]+\" height=\"[0-9]+\"/\1width=\"${size}\" height=\"${size}\"/" \
        "$svg" > "${stage}/at-${size}.svg"
    sips -s format png "${stage}/at-${size}.svg" --out "$out" >/dev/null 2>&1
    # The rewrite above is a substitution on someone else's file and would fail
    # silently if the attributes were ever written differently, leaving every
    # icon a 64-pixel upscale. Checked rather than trusted.
    got=$(sips -g pixelWidth "$out" | sed -n 's/.*pixelWidth: *//p')
    [ "$got" = "$size" ] || {
        echo "build-app.sh: asked for ${size}px and got ${got}px — the SVG's width and height attributes are not where the substitution expects them" >&2
        exit 1
    }
}

for pair in 16:16x16 32:16x16@2x 32:32x32 64:32x32@2x \
            128:128x128 256:128x128@2x 256:256x256 512:256x256@2x \
            512:512x512 1024:512x512@2x
do
    render "${pair%%:*}" "${iconset}/icon_${pair#*:}.png"
done
iconutil --convert icns "$iconset" --output "${app}/Contents/Resources/slipcase-desktop.icns"

sed "s/@VERSION@/${version}/g" "${here}/Info.plist.in" > "${app}/Contents/Info.plist"
# A malformed property list is not an error Finder reports; it is a bundle that
# quietly does not associate. Parsed here so the failure is loud.
plutil -lint "${app}/Contents/Info.plist" >/dev/null

install -m 0755 "$binary" "${app}/Contents/MacOS/slipcase-desktop"

echo "built ${app} from ${binary}"
echo
echo "register it and check that it took:"
echo "  /System/Library/Frameworks/CoreServices.framework/Frameworks/LaunchServices.framework/Support/lsregister -f ${app}"
echo "  mdls -name kMDItemContentType SOME.slpc      # com.excelano.slipcase"
echo "  open SOME.slpc"
