# Packaging

`DESIGN.md` §8. One directory per platform, plus `debian` for the way Linux is
distributed.

Each platform's own README says what was decided there and what was measured
rather than assumed.

## linux

The freedesktop half: the desktop entry and the application icon. Install it
into a prefix, which defaults to `~/.local`:

    ../slipcase-common/install.sh
    ./packaging/linux/install.sh
    ./packaging/linux/install.sh --prefix /usr/local     # for everyone
    ./packaging/linux/uninstall.sh

The script installs the executable too, found by asking `cargo metadata` where
the target directory is, because `[build] target-dir` in a Cargo configuration
file can move it and no environment variable then says so.

Check that it took:

    xdg-mime query filetype some.slpc
    # application/vnd.excelano.slipcase+zip
    xdg-mime query default application/vnd.excelano.slipcase+zip
    # slipcase-desktop.desktop

Before the media type is installed a `.slpc` reports as `application/zip`, which
is true and useless: it is what every Slipcase is underneath.

**The media type and the icon a container is drawn with are `slipcase-common`'s**,
which this package depends on. Two packages cannot ship one path (dpkg refuses
the second install outright), and `slipcase-open` claims the same association,
so the type is declared once and every product depends on it.
`slipcase-common`'s README says why the icon is named as the generic icon as
well as the icon.

**The desktop entry names every content file family, not one type.** `slipcase-common` declares a
content file family per icon — `application/x.slipcase-pdf+zip` against `*.pdf.slpc`,
and others like it — so that a container named for its content file can be drawn with a
mark for it. Each is a `sub-class-of application/vnd.excelano.slipcase+zip`, and
that inheritance carries the default application but not the *recommended* list,
which is matched on the exact type. A family missing from `MimeType=` therefore
still opens on a double-click and still drops out of the top of Open With. The
list is the one part of the arrangement that could not stay in `slipcase-common`,
and adding a family there means adding it here.

`install.sh` says so when the machine has no declaration of the type, asked
of `share/mime/types` rather than of the filenames in `packages/`, since each
product names its declaration differently.

## macos

The application bundle, the exported type declaration, and the icon:

    cargo build --release
    ./packaging/macos/build-app.sh

`packaging/macos/README.md` is the detail, including why double-clicking works
only because the Apple Event handler is registered at
`applicationWillFinishLaunching:`, and why `mdls` reports the right type only
once the bundle is signed.

## windows

The same job with no freedesktop database to do it in: the extension, the media
type, the icon, and the entry that opens a container, all written to the
registry. Per-user, needing no administrator, which is the counterpart of the
Linux script's default of `~/.local`.

    powershell -ExecutionPolicy Bypass -File packaging\windows\install.ps1
    powershell -ExecutionPolicy Bypass -File packaging\windows\uninstall.ps1

`packaging/windows/README.md` says where each key goes and why, and records
what was measured there rather than assumed, including that
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
`packaging/linux/check-libraries.sh` keeps it accurate: it runs the window
under both display backends and refuses any library whose package `Depends`
does not transitively reach. Run it after touching a dependency.

The package carries no maintainer scripts. `desktop-file-utils` and
`hicolor-icon-theme` own dpkg triggers on the two directories this package
writes into, so the desktop and icon caches are rebuilt by dpkg without a
`postinst` asking for it. Both are in `Depends` for that reason as much as for
anything they provide at run time. `shared-mime-info` is not named, because
`slipcase-common` pulls it in.

## The icon

`packaging/linux/icons/slipcase-desktop.svg`, drawn on a 64-unit grid: a card
sliding into an open-topped case. This is the *application* icon; the icon a
container is drawn with is `slipcase-common`'s, a different role and free to
diverge from this one. It is the source for every platform's application icon:
`packaging/windows/make-ico` converts it to `.ico`, and
`packaging/macos/build-app.sh` renders it into the `.icns`.

It is deliberately two shapes and one band, because finer detail turns to mud at
48 pixels. Check any change at 16, 24, 32, and 48 before committing it.
