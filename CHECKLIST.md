# Checklist: the things only a hand can test

`CLAUDE.md` and both handoff briefs have referred to this file since before it
existed. It did not: `git log --diff-filter=A` finds no commit that ever added
it. Windows wrote the first section, macOS the second, and Linux the third,
after running the association walkthrough that the other two had already been
through. What Linux still owes is narrower: the seven defects its first
walkthrough found are recorded in commit messages rather than here.

A section per platform. Each item says what to do, what should happen, and —
where a run found something — what actually happened.

---

## Windows

Run against a release build, because the console-window item below is a
property of the release profile and passes vacuously in debug.

    cargo build --release
    powershell -ExecutionPolicy Bypass -File packaging\windows\install.ps1

### The association

1. **Explorer draws the icon.** Put a `.slpc` in a folder and look at it in
   Large Icons and in Details. Both should show the card-in-a-case, not the
   blank page Windows gives an unknown extension.
2. **The type reads as a name.** The Type column should say `Slipcase
   Container`, not `SLPC File`.
3. **Double-clicking opens the container, not an empty window.** This is the
   argument path, and until Windows ran it, it had only ever been exercised on
   Linux. The window should show the container's name, its path, `conformant`,
   the payload card, and the metadata tree — not the empty state.
4. **The card names an application for the payload.** A container whose payload
   is `report.pdf` should say what would open it. Silence is a legitimate
   answer for a payload nothing is registered for, so use a name the machine
   has an association for.
5. **The Start menu entry works** and carries the icon.
6. **Uninstalling leaves nothing.** Before uninstalling, open a `.slpc` with
   "always open with" so a `UserChoice` exists — that is the key that outranks
   everything else and the one an uninstaller forgets. Afterwards, no key under
   `HKCU\Software\Classes` for the type or the extension, no `FileExts\.slpc`,
   no Add/Remove Programs entry, no shortcut. Explorer should go back to
   calling it `SLPC File`.

### What the first run found

Two defects, both invisible to 61 tests and both fixed in the same commit.

**A console window opened behind the application.** The executable was built
console-subsystem, so a file manager launching it — which is not attached to a
terminal — got a black console window with the executable's path as its title,
sitting behind the window a person actually wanted. Fixed with
`windows_subsystem = "windows"`, off in debug builds so a panic still has
somewhere to go.

**The window had no icon.** The title bar and the task bar showed the generic
default. `APP_ID` is what answers this on Linux and it does nothing here —
`with_app_id` is Wayland's, and neither egui, eframe, nor winit turns it into
anything on Windows. Fixed by embedding the `.ico` and handing its 64-pixel
entry to the window at startup, because compiling a resource the normal way
needs a tool `DESIGN.md` §2 keeps out of the build.

A third was in the packaging rather than the application: `install.ps1` closed
by telling the reader to check with `assoc` and `ftype`, and both report no
association at all after a successful per-user install. They read the
machine-wide half of the class root only.

### Not yet done by hand

- **A high-density display.** Every size above was checked at 100%. The icon
  carries 20, 40, and 64 for the scaled sizes and none of them has been looked
  at on a display that would ask for one.
- **A second user account**, to confirm a per-user install is invisible to one.
- **An upgrade over an existing install**, rather than a first install.

---

## Linux

### The association

Run `./packaging/linux/install.sh` first. Log out and back in if the desktop
does not notice: GNOME reads the mime database once per session.

1. **The type is ours.** `xdg-mime query filetype SOME.slpc` says
   `application/x.slipcase+zip`. Before installing it says `application/zip`,
   which is the right answer for a file the desktop knows nothing about, so
   check it before as well as after.
2. **The handler is ours.** `xdg-mime query default application/x.slipcase+zip`
   says `slipcase-desktop.desktop`.
3. **The folder shows the drawing.** Open a folder of containers in Files. The
   `.slpc` files carry the slipcase icon rather than a generic archive, and the
   type reads *Slipcase container*. Put a file beside them whose name does not
   match `*.slpc` — a copy with any other suffix will do. It should draw as a
   plain archive, and the two being identical is the tell that the icon is not
   being reached.
4. **A double-click loads the container.** Slipcase starts with that container
   open, not on the empty state. This is the argument path nothing else
   exercises.
5. **The window carries the icon**, in the dash and in the task switcher. That
   is `APP_ID` matching the desktop entry's basename; a mismatch draws a blank.
6. **The archive manager is still offered.** Right-click a container. Slipcase
   is the only entry in the *Open With* submenu, and that is correct rather
   than a defect: `sub-class-of application/zip` makes File Roller a *fallback*
   application, and GIO surfaces fallbacks inside the full *Open With…* dialog
   rather than in the context menu. Open that dialog and check File Roller is
   listed, then open a container with it.
7. **The loop closes on itself.** `cargo run --example opens-with -- some.slpc`
   says *Slipcase*, by the same code that says *Document Viewer* for a
   `report.pdf`. Pass a bare payload name and not a path: `opens_with` rejects
   anything carrying a separator, which is correct and reads as a failure if
   you forget.

### The package

8. `cargo build --release && ./packaging/debian/build-deb.sh`. It names the file
   it wrote, then prints what the executable links beside what the package
   declares. The two lists barely overlap, which is the point.
9. `dpkg-deb --contents`. Everything `root/root`, directories `0755`, the
   executable `0755`, data files `0644`.
10. `dpkg-deb --ctrl-tarfile … | tar t` holds `control` and `md5sums` and
    nothing else — no maintainer scripts, because the three tool packages in
    `Depends` own the triggers that rebuild the caches. Confirm those triggers
    exist rather than assuming: `grep /usr/share/mime/packages
    /var/lib/dpkg/triggers/File` names `shared-mime-info`, and the other two
    directories name `desktop-file-utils` and `hicolor-icon-theme`.
11. `md5sum -c` the extracted tree against the package's own `md5sums`, then
    change a byte and watch it fail.

### What the association run found

Three defects. All three passed 59 tests, the whole conformance corpus, and a
release build without a murmur.

**Neither icon could be loaded by a file manager.** gdk-pixbuf sniffs for
`<svg` within the first 256 bytes and the authorship comment put the opening
tag at byte 502, so both files were refused as an unrecognised format. It had
been checked at four sizes with `rsvg-convert`, which parses rather than sniffs
and rendered them correctly throughout. The lesson is not about SVG: a check
that exercises a different code path from the one that ships is not a check.

**Containers still drew as plain archives once the icons loaded.** The mime
package declared `package-x-generic` as the generic icon, and GTK4 searches
theme-major — every name in Adwaita before any name in hicolor — so the generic
won regardless of order. The tell was in a screenshot: files that did not match
the glob drew identically to files that did.

**The `.deb` shipped no `md5sums`**, so `dpkg -V` could say nothing about an
installed copy. Found by looking inside the archive rather than by any test.

### Not yet done by hand

- **The package installed and removed.** Items 8 to 11 inspect the `.deb`;
  nothing has yet run `apt install` on it. The `~/.local` install shadows a
  system one, so uninstall that first or the test proves nothing.
- **A machine that has never had it**, to see what `apt` actually pulls.
- **A second desktop.** Everything above is GNOME on Wayland with Adwaita. The
  icon defect was a property of which theme carried which name, so KDE or XFCE
  could reach a different answer by the same mechanism.
- **The seven defects the first walkthrough found** — layout geometry, font
  coverage, frame timing, and controls drawn but inert — are recorded in
  `git log` and still not reconstructed here.

---

## macOS

Run against a bundle, because almost nothing below is true of a bare
executable: it has no bundle identifier, Launch Services files it as a nameless
foreground process, and nothing can be associated with it.

    cargo build --release
    ./packaging/macos/build-app.sh
    /System/Library/Frameworks/CoreServices.framework/Frameworks/LaunchServices.framework/Support/lsregister -f dist/Slipcase.app

### The association

1. **Finder draws the icon.** Put a `.slpc` in a folder and look at it as an
   icon and in list view. Both should show the card-in-a-case rather than the
   blank page macOS gives an unknown extension.
2. **Get Info names the type.** The Kind should read `Slipcase container`,
   which is the bundle's own `UTTypeDescription`, not `Document` and not
   `slpc File`.
3. **Double-clicking opens the container, cold.** With Slipcase *not* running,
   double-click a `.slpc`. The window should come up showing that container —
   its name, `conformant`, the payload card, the tree — and **no dialog**. The
   failure this guards against is specific and was the state of things for a
   while: *The document could not be opened. Slipcase cannot open files in the
   "Slipcase container" format.*
4. **Double-clicking opens the container, warm.** With Slipcase already running
   and a different container open, double-click another. It should replace what
   is on screen. This is a different code path from the cold launch and one of
   the three registration moments passes it while failing the cold one, so
   testing only this would have hidden the defect.
5. **The Open dialog starts where Finder was.** After opening a container by
   double-clicking, press Open a slipcase. The dialog should start in that
   container's folder.
6. **The card names an application for the payload.** A container carrying
   `report.pdf` should say Preview. Silence is legitimate for a payload nothing
   is registered for, so use a name the machine has an association for.
7. **A container opens Slipcase from the card.** With a `.slpc` as the payload
   of another container, the card should say `Slipcase`, which is this bundle
   naming itself through the same code that names Preview.

### What the first run found

The first thing a hand found here was not in the application at all. The
walkthrough could not be run by machine: `screencapture` returns the desktop
and the menu bar with every window omitted unless the terminal has been given
Screen Recording permission, and it reports no error when it does that. Two
captures of a screen with windows on it came back byte-identical and empty,
which is the only tell.

The double-click was the defect, and it took a person pressing it to find. The
tests, the corpus, and every command-line check passed while it was broken:
`open some.slpc` returns zero and the process starts, and the refusal is a
dialog that a terminal cannot see. It was recorded here as opening an empty
window before a person looked and read out the actual message.

A test fixture was wrong in a way that looked like an application defect. The
first container carried a hand-written stub `report.pdf` with no xref table,
and Preview refused it. Preview was right. Payloads for this walkthrough are
made with `cupsfilter` now, and the round trip is checked byte for byte before
the container is used.

### Not yet done by hand

- **A high-density display.** Every size above was checked on a 2560x1440
  display at 100%. The `.icns` carries entries to 1024 and none of the `@2x`
  ones has been looked at on a display that would ask for one.
- **A signed bundle.** Everything above is an unsigned bundle that never left
  the machine that built it. `mdls` reporting the wrong type is suspected to be
  a consequence of that and is untested either way.
- **A downloaded bundle**, carrying `com.apple.quarantine`, to see what
  Gatekeeper actually shows a person rather than what `spctl` reports.
- **A second user account**, and an upgrade over an existing install.
