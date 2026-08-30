#!/bin/sh
# Every shared library this application opens, against what the package
# declares — the Linux half of the pair `packaging/windows/check-imports.ps1`
# is the other half of.
#
# **It exists because the same defect shipped on both platforms in one day.**
# Windows linked `VCRUNTIME140.dll`, which is not part of Windows, and 0.1.1
# failed Store certification on a clean machine. Looking for the Linux
# equivalent the same afternoon found one: `libxkbcommon-x11-0` was opened by
# name on the X11 path, was absent from `Depends`, and is reachable from
# nothing else in the list — so an X11 machine that installed the package got
# an application that panicked at startup. Measured rather than reasoned about,
# by putting a broken copy earlier on the search path:
#
#     thread 'main' panicked at xkbcommon-dl-0.4.2/src/x11.rs:59:28:
#     Library libxkbcommon-x11.so could not be loaded.
#
# **A dependency on the toolchain is invisible from inside the toolchain**, and
# a dependency on the desktop is invisible from inside the desktop. Every
# machine this project builds on has the whole display stack, so neither
# platform's miss could be seen by running the application where it was built.
#
# It is a command and never a test, for the reason the conformance corpus is:
# it needs a display, and a test that has to choose between skipping quietly
# and failing on a machine that was never going to have one is worse than a
# command run on purpose.
#
#     ./packaging/linux/check-libraries.sh              # both backends
#     ./packaging/linux/check-libraries.sh --backend x11
#
# **Both backends, because they load disjoint sets.** A Wayland session never
# opens libX11 or libxkbcommon-x11, and an X11 session never opens
# libwayland-client. Half the list is unexercised by any single run, which is
# exactly how the gap survived: this machine is Wayland, so nobody had run the
# half that was broken.
#
# Author: David M. Anderson
# Built with AI assistance (Claude, Anthropic)
set -eu

here=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
root=$(CDPATH= cd -- "${here}/../.." && pwd)
binary=""
backend="both"

usage() {
    cat <<'USAGE'
usage: check-libraries.sh [--binary PATH] [--backend wayland|x11|both]

  --binary PATH   the executable to check (default: the release build)
  --backend WHICH which display backend to exercise (default: both)

Refuses if a library the running application opens belongs to a package that
`Depends` in packaging/debian/control.in does not reach.
USAGE
}

while [ $# -gt 0 ]; do
    case "$1" in
        --binary) binary="${2:?--binary needs a path}"; shift 2 ;;
        --backend) backend="${2:?--backend needs a value}"; shift 2 ;;
        -h|--help) usage; exit 0 ;;
        *) echo "check-libraries.sh: unknown argument $1" >&2; usage >&2; exit 2 ;;
    esac
done

case "$backend" in
    wayland|x11|both) ;;
    *) echo "check-libraries.sh: --backend must be wayland, x11 or both" >&2; exit 2 ;;
esac

# Cargo is asked where its target directory is, for the reason `build-deb.sh`
# gives: `[build] target-dir` moves it and no environment variable then says so.
if [ -z "$binary" ]; then
    target_dir=$(cd "$root" && cargo metadata --format-version 1 --no-deps |
        sed -n 's/.*"target_directory":"\([^"]*\)".*/\1/p')
    binary="${target_dir}/release/slipcase-desktop"
fi
[ -x "$binary" ] || {
    echo "check-libraries.sh: no executable at $binary — cargo build --release first" >&2
    exit 1
}

for tool in dpkg apt-cache; do
    command -v "$tool" >/dev/null || {
        echo "check-libraries.sh: needs $tool, so this runs on a dpkg machine only" >&2
        exit 1
    }
done

control="${root}/packaging/debian/control.in"
[ -f "$control" ] || { echo "check-libraries.sh: no $control" >&2; exit 1; }

stage=$(mktemp -d)
trap 'rm -rf "$stage"' EXIT INT TERM

# The application needs a container to open, and it must be a conformant one:
# a refusal leaves the window up but takes a different path through the code
# and may not reach the parts that open a graphics driver.
"${root}/packaging/demo-container.sh" --out "$stage" >/dev/null

# Which backends can actually be exercised here. Asking rather than assuming,
# because a session that has one and not the other should say which it skipped
# rather than silently checking half of what it claims.
wants_wayland=no
wants_x11=no
case "$backend" in
    both)    wants_wayland=yes; wants_x11=yes ;;
    wayland) wants_wayland=yes ;;
    x11)     wants_x11=yes ;;
esac
[ -n "${WAYLAND_DISPLAY:-}" ] || wants_wayland=no
[ -n "${DISPLAY:-}" ] || wants_x11=no

if [ "$wants_wayland" = no ] && [ "$wants_x11" = no ]; then
    echo "check-libraries.sh: no display to run against — needs WAYLAND_DISPLAY or DISPLAY" >&2
    exit 1
fi

# Run once and record every shared object the process has mapped. `/proc/maps`
# rather than `ldd`, because `ldd` reports what the linker recorded and almost
# nothing here is recorded: the display stack and the driver loader are opened
# by name at run time, which is the whole reason this script exists.
observe() {
    # $1 = a label, $2.. = the environment prefix for the run
    label=$1
    shift
    "$@" "$binary" "${stage}/quarterly-report.pdf.slpc" >"${stage}/${label}.log" 2>&1 &
    pid=$!
    # Wait for the window to have brought its libraries in. There is no signal
    # to wait on from out here, so this waits for the map count to stop
    # growing, which is a property of the thing being measured rather than a
    # guessed number of seconds. The interval is real: without one this loop
    # spins through every iteration before the driver has been opened and finds
    # a count that is stable because nothing has happened yet.
    last=0
    same=0
    i=0
    while [ "$i" -lt 100 ]; do
        kill -0 "$pid" 2>/dev/null || break
        now=$(awk '$6 ~ /\.so/ {print $6}' "/proc/$pid/maps" 2>/dev/null | sort -u | wc -l)
        if [ "$now" -eq "$last" ] && [ "$now" -gt 0 ]; then
            same=$((same + 1))
            [ "$same" -ge 5 ] && break
        else
            same=0
        fi
        last=$now
        i=$((i + 1))
        sleep 0.2
    done
    if ! kill -0 "$pid" 2>/dev/null; then
        echo "check-libraries.sh: the application exited under $label rather than starting" >&2
        cat "${stage}/${label}.log" >&2
        exit 1
    fi
    awk '$6 ~ /^\// && $6 ~ /\.so/ {print $6}' "/proc/$pid/maps" | sort -u > "${stage}/${label}.libs"
    kill "$pid" 2>/dev/null || true
    wait "$pid" 2>/dev/null || true
    count=$(wc -l < "${stage}/${label}.libs")
    # **A pass on a measurement of nothing is worse than no check**, and this
    # script produced one before the interval above existed: it sampled the map
    # while the process was still starting, found four objects, and reported
    # that everything the application opens is covered. A window that has drawn
    # has the display stack and a graphics driver mapped, which is dozens. Well
    # under that means the sampling is wrong, not that the package is clean.
    if [ "$count" -lt 20 ]; then
        echo "check-libraries.sh: only $count objects mapped under $label, which is too" >&2
        echo "  few for a window that has drawn — the sampling finished early and this" >&2
        echo "  run proves nothing. Not reporting a pass on it." >&2
        exit 1
    fi
    cat "${stage}/${label}.libs" >> "${stage}/libs"
    echo "  $label: $count objects mapped"
}

: > "${stage}/libs"
echo "running:"
[ "$wants_wayland" = yes ] && observe wayland env WINIT_UNIX_BACKEND=wayland
[ "$wants_x11" = yes ] && observe x11 env -u WAYLAND_DISPLAY WINIT_UNIX_BACKEND=x11
sort -u "${stage}/libs" -o "${stage}/libs"

[ "$wants_wayland" = yes ] || echo "  wayland: SKIPPED, no WAYLAND_DISPLAY in this session"
[ "$wants_x11" = yes ] || echo "  x11: SKIPPED, no DISPLAY in this session"

# What `Depends` reaches, transitively. A library does not have to be named in
# the list — it has to be *reachable* from it, which is what apt will actually
# install. Computing the closure rather than comparing names is the difference
# between this and a check that would have demanded libxcb be listed.
python3 - "$control" "${stage}/libs" <<'PY'
import collections, subprocess, sys, re

control, libs_file = sys.argv[1], sys.argv[2]

# Alternatives are taken at their first element, which is what apt prefers and
# therefore the case a machine is most likely to be in. The weaker alternative
# is the one that would hide a gap, so the stricter reading is the useful one.
line = next(l for l in open(control) if l.startswith('Depends:'))
declared = [alt.split('|')[0].strip().split()[0]
            for alt in line.split(':', 1)[1].split(',')]

seen, queue = set(), collections.deque(declared)
while queue:
    pkg = queue.popleft()
    if pkg in seen:
        continue
    seen.add(pkg)
    out = subprocess.run(
        ['apt-cache', 'depends', '--no-recommends', '--no-suggests', '--no-conflicts',
         '--no-breaks', '--no-replaces', '--no-enhances', pkg],
        capture_output=True, text=True).stdout
    for l in out.splitlines():
        l = l.strip()
        if l.startswith(('Depends:', 'PreDepends:')):
            dep = l.split(':', 1)[1].strip().lstrip('<').rstrip('>')
            if dep and not dep.startswith('|'):
                queue.append(dep)

# Packages that provide something the application opened but that nothing in
# Depends reaches. Each needs a decision rather than an automatic entry: some
# are genuinely optional and the list records which and why.
#
# mesa-vulkan-drivers is optional and that was measured, not assumed. With
# VK_DRIVER_FILES pointing at nothing the application starts and draws through
# GL, which is what `libvulkan1 | libgl1` in the alternative is for. Declaring a
# hardware-specific driver package as a hard dependency would be wrong.
OPTIONAL = {'mesa-vulkan-drivers'}

libs = [l.strip() for l in open(libs_file) if l.strip()]
missing = {}
for lib in libs:
    pkg = subprocess.run(['dpkg', '-S', lib], capture_output=True, text=True).stdout
    pkg = pkg.split(':', 1)[0].strip()
    if not pkg or pkg in seen or pkg in OPTIONAL:
        continue
    missing.setdefault(pkg, []).append(lib.rsplit('/', 1)[-1])

print(f"\n{len(libs)} objects opened, {len(seen)} packages reachable from Depends")
if not missing:
    print("every library the application opened is reachable from Depends")
    sys.exit(0)

print("\nNOT reachable from Depends — the package would install and fail to start:")
for pkg, ls in sorted(missing.items()):
    print(f"  {pkg}")
    for l in ls:
        print(f"      {l}")
print("\nAdd each to Depends in packaging/debian/control.in, or record here why it")
print("is optional — with the measurement that shows the application starts without")
print("it, the way mesa-vulkan-drivers is recorded above.")
sys.exit(1)
PY
