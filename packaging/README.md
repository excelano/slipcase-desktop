# Packaging

`DESIGN.md` §8. One directory per platform, plus `debian` for the way Linux is
distributed.

All four exist. Each platform's own README says what was decided there and what
was measured rather than assumed.

## linux

The freedesktop half: the desktop entry and the application icon. Install it
into a prefix, which defaults to `~/.local`:

    ../slipcase-common/install.sh
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
is true and useless: it is what every Slipcase is underneath.

**The media type and the icon a container is drawn with left this repository.**
They are `slipcase-common`'s, which this package depends on. Two packages cannot
ship one path — dpkg refuses the second install outright — so as soon as
`slipcase-open` claimed the same association the type could be declared here or
there and not both, and the icon could only ever be in one of them. Every
container on a machine with the other product drew as a blank generic document.
Declaring it once and depending on it is the arrangement that has no such side.

The reasoning that used to sit in the XML's comments went with it, including why
the icon has to be named as the generic icon as well as the icon. That finding
was made here and is recorded in `slipcase-common`'s README, which measured it
again from the other direction.

`install.sh` now says so when the machine has no declaration of the type, asked
of `share/mime/types` rather than of the filenames in `packages/`, since each
product names its declaration differently.

## macos

The application bundle, the exported type declaration, and the icon:

    cargo build --release
    ./packaging/macos/build-app.sh

`packaging/macos/README.md` is the detail. Two things there are worth knowing
before reading anything else. Double-clicking works, but only because the Apple
Event handler is registered at `applicationWillFinishLaunching:` — the two other
plausible moments are tabulated there, and both fail. And `mdls` reports the
wrong type for a `.slpc` even after the bundle is registered, while Launch
Services reports the right one, which is measured and unresolved.

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

**Why the two lists barely overlap is `DESIGN.md` §2's**, and it is not
restated here. What follows from it is this directory's: `Depends` in
`control.in` is written by hand rather than derived, and
`packaging/linux/check-libraries.sh` is what keeps it honest — it runs the
window under both display backends and refuses any library whose package
`Depends` does not transitively reach. Run it after touching a dependency. Two
releases shipped without `libxkbcommon-x11-0` before it existed.

The package carries no maintainer scripts. `desktop-file-utils` and
`hicolor-icon-theme` own dpkg triggers on the two directories this package
writes into, so the desktop and icon caches are rebuilt by dpkg without a
`postinst` asking for it. Both are in `Depends` for that reason as much as for
anything they provide at run time. `shared-mime-info` is no longer named,
because `slipcase-common` pulls it in.

## The icon

`packaging/linux/icons/slipcase-desktop.svg`, drawn on a 64-unit grid: a card
sliding into an open-topped case. This is the *application* icon. The icon a
container is drawn with was the same file under another name and now lives in
`slipcase-common`, which is a different role and free to diverge from this one.
It is the source for every platform's icon —
macOS wants `.icns` and Windows wants `.ico`, both converted from this.
`packaging/windows/make-ico` is the converter for the second; a `.icns` one has
still to be written.

It is deliberately two shapes and one band. The first version had a third text
line on the card and an inset lip on the case, and both turned to mud at 48
pixels. Check any change at 16, 24, 32, and 48 before committing it.
