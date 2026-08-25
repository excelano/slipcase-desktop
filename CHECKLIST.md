# Checklist: the things only a hand can test

`CLAUDE.md` and both handoff briefs have referred to this file since before it
existed. It did not: `git log --diff-filter=A` finds no commit that ever added
it. Windows wrote the first section, macOS the second, and Linux the third,
after running the association walkthrough that the other two had already been
through.

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

- **Provenance, which has never run on this platform.** `src/provenance.rs`
  carries a container's `Zone.Identifier` stream onto the payload extracted
  from it, and the Windows arm compiles from Linux but has never executed.
  Download a container with a browser so it carries a real stream, confirm the
  card says it arrived from elsewhere, extract the payload to a chosen folder,
  and read the stream off the copy — `Get-Content -Stream Zone.Identifier`.
  Then the question that decides whether the Open button should have been
  disabled instead: press Open, which extracts into the temp directory and
  hands the payload over, **and see whether Windows shows the Open File
  security warning for a zoned file there**. If a zoned file in the temp
  directory is treated as trusted, the reporting on the card is not enough and
  the decision recorded in DESIGN.md §5 has to be reopened.
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
12. **Install it**, after `./packaging/linux/uninstall.sh` — the `~/.local`
    copy shadows a system one and the test proves nothing while it is there.
    `sudo apt install ./dist/slipcase-desktop_0.1.0_amd64.deb`. Watch for three
    things: whether anything is pulled beyond the eleven declared dependencies,
    whether triggers are processed for `hicolor-icon-theme`, `shared-mime-info`
    and `desktop-file-utils`, and whether dpkg complains about the package's
    root directory. Then check the association again from `/usr`, and run
    `dpkg -V slipcase-desktop`, which is silent only if `md5sums` is both
    present and correct.
13. **Enter, from a double-click onwards.** Double-click a container in Files,
    look for the focus ring on Open, and press Enter without touching the
    mouse: the payload opens. Then Tab, which must move focus off Open rather
    than sticking to it — a request made every frame pins the keyboard there
    and leaves the tree and the Save unreachable. Then a container whose
    payload cannot be decoded, which must show no ring at all, the flag being
    meant to stay unspent rather than land on a disabled button. Two tests
    cover the logic and neither can see a pixel, so the ring being legible
    against the card is only checkable here.

    **Use a container holding a real payload.** The conformance fixtures carry
    47-byte placeholders named `report.pdf`, which are correct for testing
    container mechanics and are not documents: handing one to Document Viewer
    produces *PDF document is damaged*, which reads exactly like a defect in
    the handover and is not one. It cost a round trip here. Build a container
    around a file that actually opens before concluding anything about this
    step.

14. **Remove it.** `sudo apt remove slipcase-desktop`. The type reverts to
    `application/zip`, nothing matching `slipcase` is left under
    `/usr/share/{mime,applications,icons}`, the mime cache no longer contains
    the string, and `dpkg-query -W` finds no package — not even the `rc` state
    a package with conffiles would leave.

### What the first walkthrough found

All fixed in August, listed here because a count is not a record and the count
this file carried was wrong. Eight defects across six commits, none of which
59 tests or the conformance corpus could reach.

1. The window reported as hung while the file dialog was open, and GNOME
   offered to force quit it. The dialog blocks in another process, so blocking
   here bought nothing and cost the compositor its answers. `e02af29`
2. Integers drew hard against the right edge, past the end of the row and
   invisible inside a scroll area that scrolls both ways. `eb246e1`
3. The buttons that remove a key drew as empty squares and read as checkboxes:
   U+2715 is in none of egui's default fonts. `71f3643`
4. A refused date said so in weak grey text that nobody notices. `71f3643`
5. `9:00` was refused where `19:00` was accepted, because TOML wants two digits
   in an hour and nothing said so. `f885ff2`
6. The refusal message appeared for one frame and vanished, while the text it
   was about stayed on screen. `f885ff2`
7. Keys inside an inline table carried add, rename, and remove buttons that did
   nothing at all. `609f263`
8. Saving into a read-only directory did nothing and said nothing. `c2be197`

Seven if 4 and 5 are folded together, which is defensible — the same field in
the same sitting, split across two commits only because the colour was fixed
before the parsing was. Written as eight here because the list is the record
and the number is not.

One further finding is not a defect of ours: dropping a file on the window does
nothing, because winit 0.30's Wayland backend carries no data-device plumbing
and `DroppedFile` reaches only its X11 backend. Unexercised rather than wrong,
and it wants trying under X11.

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

Enter from a double-click was walked through and works: the ring is visible on
Open, Enter opens the payload without a reach for the mouse, Tab moves away
afterwards, an encrypted payload leaves Open disabled and unringed, and a
container marked as arriving from elsewhere says so on the card in a colour
that reads. The only thing that went wrong was the test data, recorded in item
13 so it does not cost anybody else the same round trip.

The package itself found nothing. Installing pulled no dependency the machine
did not already have, which is the first evidence that the hand-derived
`Depends` is right rather than merely plausible — the linker names four
libraries and the list names eleven, and a wrong one would have shown here or
at the first launch. All three trigger owners fired, so a package carrying no
maintainer scripts still registered the type, and `dpkg -V` was silent against
the installed tree. Removing it took the association with it and left no `rc`
state, there being no conffiles to leave.

### Not yet done by hand

- **A machine that has never had it.** Item 12 was run here, where every
  dependency was already satisfied, so `apt` pulled nothing and the `Depends`
  list was never actually exercised. A minimal install would exercise it.
- **A second desktop.** Everything above is GNOME on Wayland with Adwaita. The
  icon defect was a property of which theme carried which name, so KDE or XFCE
  could reach a different answer by the same mechanism.

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

### What the sandbox sitting found

Run 2026-08-25 against `dist-sandboxed/Slipcase.app`: the release bundle signed
with an Apple Development certificate and the two entitlements a Store build
would carry. No distribution certificate was needed for any of it. Four
results, one of which contradicts the item that asked for them.

**Save fails, and on the sibling rather than the rename.** Opening
`~/Documents/sandbox-save-test.slpc` through the application's own Open dialog,
editing a key, and pressing Save gives *cannot write into
/Users/anderix/Documents: Operation not permitted (os error 1)* at path
`/Users/anderix/Documents/.tmpos50lA`. The grant covers the file a person chose
and nothing else in that directory, exactly as the item predicted. The
application handles it properly — a clean refusal naming the path, no crash and
no half-written container.

**The handover survives, and the prediction that it would not was wrong.** The
item said `opener` is `Command::new("open")`, that the sandbox denies executing
outside the bundle, and so Open would do nothing at all. Open opened the
payload in TextEdit. There is no `process-exec` denial anywhere in the log: the
sandbox permits exec, the child inherits the sandbox, and `open` reaches Launch
Services over Mach IPC from inside it. `NSWorkspace::openURL:` is still the
tidier route but it is no longer forced by the sandbox.

**The sandbox writes `com.apple.quarantine` itself, wherever the file lands.**
That payload came from a container carrying no quarantine attribute at all —
only `com.apple.macl`, the sandbox's own record of the open-panel grant — and
the extracted `notes.txt` in the container's temp directory carried
`com.apple.quarantine: 0086;6a8dbd0d;slipcase-desktop;`. Extracting the same
payload to `~/Documents`, a location chosen through the save panel, produced
`0082;6a8dbf00;slipcase-desktop;`. Different flags, same agent, and
`provenance::carry` returned `Silent` and wrote neither of them. The platform
marks what a sandboxed process creates. So extraction under a sandbox cannot
launder provenance whether or not `src/provenance.rs` exists — and it marks
payloads from containers that had nothing to carry, which is a behaviour change
nobody chose.

**Carrying provenance is refused, and it takes Extract and Open down with it.**
A container downloaded through Safari, carrying
`0083;6a8dbb61;Safari;B8AC643B-…`, cannot be extracted: *i/o error: Operation
not permitted (os error 1)*, with no path, which is what an `xattr::set`
failure looks like when `src/provenance.rs` hands the raw error back. Pressing
Open on that container reports the same line, leaves nothing in the container's
temp directory, and never reaches Preview. Three measurements eliminate
everything else: the card said the container arrived from elsewhere, so
`xattr::get` on the source is permitted; extracting from the unquarantined
container into a chosen folder succeeded, so creating the file is permitted;
only the write of the attribute is left.

**Why the write is refused is a hypothesis and not a measurement.** The likely
reason is that the file already carries the mark the platform put there, so the
write attempts to replace one quarantine value with another, which is how
provenance forgery would work. Whether a sandboxed process may set that
attribute on a file carrying none was not tested, and testing it needs code
rather than a click.

**What it costs, if the Store is wanted.** A container that arrived from
elsewhere can be neither extracted nor opened under a sandbox, and those are
precisely the containers the Store channel exists to serve. Two things have to
change and neither is cosmetic. The save path needs
`NSFileManager.replaceItemAt…`, which takes its replacement from the
application's own container and swaps using the grant on the destination file;
the item's claim that a denial on the sibling condemns that route was wrong,
because it never creates a sibling. And §5's policy of failing extraction when
provenance cannot be carried has to learn that a platform which has already
marked the file has done the job itself. Both are decisions rather than
repairs, so they belong with David.

**The save path was changed the same day, and the change was run under the same
sandbox.** `src/staging.rs` stages the rewrite in a scratch directory of the
application's own and lands it with `-[NSFileManager replaceItemAtURL:…]`.
Saving `~/Documents/sandbox-save-test.slpc` then wrote the edit, read back
conformant, and said *Saved.*, with the comment on an untouched key surviving.

That run showed one more thing, and a screenshot rather than a terminal caught
it. The container had no `com.apple.quarantine` when it was opened and carried
`0082;…;slipcase-desktop;` afterwards, so the card now says *This container
arrived from elsewhere* about a file that never left the machine. The staged
file is written by a sandboxed process, the platform marks it, and
`replaceItemAtURL:` carries that mark onto the original — the original's
`com.apple.macl` and last-used date survive and the quarantine attribute does
not. Whether a sandboxed process may remove the attribute it did not ask for is
untested, and testing it needs code rather than a click.

One incidental: the only sandbox violation the log attributes to this
application is `hid-control`, winit asking WindowServer for raw input. It is
harmless and it is not ours.

Two things about running this at all. `log` is a zsh builtin, so a capture
needs `/usr/bin/log` spelled out. And most of what the sandbox refuses here is
never reported as a violation — neither the save nor the quarantine write
produced a `deny(1)` line — so the application's own error text was the record
and the log served mainly to rule things out.

### Not yet done by hand

- **Provenance, which has never run on this platform.** `src/provenance.rs`
  carries a container's `com.apple.quarantine` attribute onto the payload
  extracted from it, and the macOS arm compiles but has never executed.
  Download a container with a browser so it carries a real attribute, confirm
  the card says it arrived from elsewhere, extract the payload somewhere
  chosen, and read the attribute off the copy — `xattr -p
  com.apple.quarantine`. Then the question that decides whether the Open button
  should have been disabled instead: press Open, and **see what macOS actually
  gates**. The suspicion is that quarantine bites on executables and
  application bundles and says nothing at all about a quarantined document
  handed to Preview, in which case carrying it matters for the dangerous case
  and not the ordinary one. If a quarantined file there is treated as trusted,
  DESIGN.md §5's decision to report rather than gate has to be reopened.

  The sandbox sitting produced weak evidence for that and not the measurement:
  a `notes.txt` carrying `com.apple.quarantine` opened in TextEdit with no
  prompt of any kind. The mark was the sandbox's own, flagged `0082` and naming
  `slipcase-desktop` rather than the `0083` a Safari download carries, and
  whether Gatekeeper reads those flags differently is exactly what is in
  question — so this still wants running against the unsigned bundle and a
  container a browser downloaded.

  **An executable payload cannot be the dangerous case, and finding that out
  cost nothing.** This item used to say to test one. `copy` in `src/lib.rs`
  creates the extracted file with `std::fs::File::create` and nothing chmods
  it afterward, so the copy lands at 0644 whatever mode the member carried
  inside the container, and `open` on a script or a Mach-O binary fails on the
  permission bit before Gatekeeper is ever consulted. What macOS gates without
  needing that bit is a disk image or an installer package, so the fixture is
  `a-disk-image.slpc`. Confirm the 0644 first — `a-command.slpc` carries a
  `run-me.command` stored `rwxr-xr-x`, so extracting it and reading the mode
  measures the claim in this paragraph rather than trusting it.
- **A high-density display.** Every size above was checked on a 2560x1440
  display at 100%. The `.icns` carries entries to 1024 and none of the `@2x`
  ones has been looked at on a display that would ask for one.
- **A signed bundle.** Everything above is an unsigned bundle that never left
  the machine that built it. `mdls` reporting the wrong type is suspected to be
  a consequence of that and is untested either way.
- **A downloaded bundle**, carrying `com.apple.quarantine`, to see what
  Gatekeeper actually shows a person rather than what `spctl` reports.
- **A second user account**, and an upgrade over an existing install.
