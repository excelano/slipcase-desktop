# Packaging

`DESIGN.md` §8. One directory per platform, plus `debian` for the way Linux is
distributed.

All four exist. Each platform's own README says what was decided there and what
was measured rather than assumed.

## linux

The freedesktop half: the media type, the desktop entry, and two icons. Install
it into a prefix, which defaults to `~/.local`:

    ./packaging/linux/install.sh
    ./packaging/linux/install.sh --prefix /usr/local     # for everyone
    ./packaging/linux/uninstall.sh

The script installs the executable too, found by asking `cargo metadata` where
the target directory is. Guessing at `./target` was tried first and found
nothing, because `[build] target-dir` in a Cargo configuration file moves it and
no environment variable then says so.

Check that it took:

    xdg-mime query filetype some.slpc              # application/x.slipcase+zip
    xdg-mime query default application/x.slipcase+zip   # slipcase-desktop.desktop

Before the media type is installed a `.slpc` reports as `application/zip`, which
is true and useless: it is what every slipcase is underneath.

## macos

The application bundle, the exported type declaration, and the icon:

    cargo build --release
    ./packaging/macos/build-app.sh

`packaging/macos/README.md` is the detail, including two things worth knowing
before reading anything else. A double-clicked container is refused with an error
dialog, because macOS delivers an opened document as an Apple Event rather than
as `argv[1]`. And `mdls` reports the wrong type for a `.slpc` even after the
bundle is registered, while Launch Services reports the right one. Both are
measured and neither is hidden.

## windows

The same job with no freedesktop database to do it in: the extension, the media
type, the icon, and the entry that opens a container, all written to the
registry. Per-user, needing no administrator, which is the counterpart of the
Linux script's default of `~/.local`.

    powershell -ExecutionPolicy Bypass -File packaging\windows\install.ps1
    powershell -ExecutionPolicy Bypass -File packaging\windows\uninstall.ps1

`packaging/windows/README.md` says where each key goes and why, and records
three things that were measured there rather than assumed — including that
`assoc` and `ftype` cannot see a per-user registration and report a successful
install as no association at all.


## debian

The package the Excelano apt repository ships:

    cargo build --release
    ./packaging/debian/build-deb.sh

It writes `dist/slipcase-desktop_VERSION_ARCH.deb` and then prints what the
executable links beside what the package declares, because those two lists are
almost disjoint and that is the trap this package exists to avoid.

**The executable links libc, libm, and libgcc, and nothing else.** Everything
that draws a window — the Wayland client library, the keyboard map library, the
EGL and Vulkan loaders, the X11 libraries — is opened by name at run time.
`dpkg-shlibdeps` sees none of it, so a package built from the linker's answer
alone installs cleanly on a machine with no display stack and then fails to
start. `Depends` in `control.in` is therefore written by hand, and each entry
was measured: the application was run, and `/proc/PID/maps` was read to find
what it had actually loaded.

The package carries no maintainer scripts. `shared-mime-info`,
`desktop-file-utils`, and `hicolor-icon-theme` own dpkg triggers on the three
directories this package writes into, so the mime, desktop, and icon caches are
rebuilt by dpkg without a `postinst` asking for it. Those three packages are in
`Depends` for that reason as much as for anything they provide at run time.

## The icon

`packaging/linux/icons/slipcase-desktop.svg`, drawn on a 64-unit grid: a card
sliding into an open-topped case. It is the source for every platform's icon —
macOS wants `.icns` and Windows wants `.ico`, both converted from this.
`packaging/windows/make-ico` is the converter for the second; a `.icns` one has
still to be written.

It is deliberately two shapes and one band. The first version had a third text
line on the card and an inset lip on the case, and both turned to mud at 48
pixels. Check any change at 16, 24, 32, and 48 before committing it.
