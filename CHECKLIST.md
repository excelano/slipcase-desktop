# Checklist: the things only a hand can test

Every window defect this project has found was found by somebody looking at a
window, and none of them by a test. A console window behind the application, the
egui logo in the Dock, a payload name reading `reportfdp.exe` two rows under a
card that escaped it, a remove control laid out past the right edge — the suite
passed and the conformance corpus agreed through all of them. This file is what
a person runs instead.

**It is the list, not the log.** What each run found is in `git log`, which this
repository treats as a document and writes to be read. Keeping the findings here
as well cost more than it bought: several documents summarised the same runs and
went stale, and `packaging/windows/README.md` still records the day the record
was right and every summary of it was wrong. A finding earns a place here only
when it changes *how you run the list*, and those are gathered at the end under
**What earlier runs cost**.

When a run finds something, fix it and put the finding in the commit. Add an
item here only if the next person would run the list differently for knowing it.

Run against a release build. Several items are properties of the release profile
and pass vacuously in debug.

---

## Every platform: the card's lines, what a save keeps, where a payload waits

Both card lines are drawn rather than returned, so no test in this repository
reaches either: the fact each line rests on is unit-tested and the rendering is
not. That gap is what this section is for.

Every fixture is in the conformance corpus, so nothing is built by hand:

    cargo run --bin corpus -- /path/to/slipcase/conformance   # generates them
    ./target/release/slipcase-desktop \
        /path/to/slipcase/conformance/cases/accept/payload-setuid-external-attributes.slpc

1. **The executable line appears, and says what it should.** Open
   `accept/payload-setuid-external-attributes.slpc`, whose payload records mode
   04755. The card should carry *The payload is an executable file; the extracted
   copy will not be executable.* in the warning colour, below the size and the
   *Opens with* line and above the buttons. On Windows it should not appear at
   all — a mode bit is not what makes a file executable there, and `DESIGN.md`
   §5 gates the line to Unix for that reason.
2. **It is absent for an ordinary container.** Open `accept/minimal.slpc`, which
   records mode 0644. No line. Then open
   `accept/payload-no-mode-recorded.slpc`, which records none, and check there
   is still no line: that silence is the whole reason `payload_mode` reads the
   external attributes rather than asking the ZIP crate, which would have
   invented `0o664` and answered confidently.

   Those are two different questions, and the second fixture is the one that
   discriminates: `payload_mode` returns the high sixteen bits of the external
   attributes and refuses a zero, and that case records `0x20`, whose high half
   is zero. A ZIP crate asked the same question invents `0o664` for a DOS entry
   and answers confidently, which is the defect the silence guards against.
3. **The payload name is escaped, and the card does not lie about it.** Open
   `accept/payload-name-bidi-override.slpc`, whose `payload.file` carries U+202E
   RIGHT-TO-LEFT OVERRIDE. The card should read `report\u{202E}fdp.exe`, ending
   in `.exe`, and the Open button beside it should still work.

   **Look at the tree as well as the card**, at the `file` row under `payload`.
   It should read the same. The card escaped and the tree did not for three
   releases, and this item said *look at the card*, so two platforms ticked it
   correctly and neither looked down.

   What this is checking is not what it looks like. egui does not apply the
   override: it lays glyphs out in logical order and gives
   every bidirectional formatting character zero advance width, so before the
   escaping the card rendered `reportfdp.exe` — a name one character short of
   the file on disk rather than a name pretending to be a PDF. The spoof
   `SPEC.md` §3 describes is real in a terminal and was never real here. So what
   a hand is confirming is that the escape now shows the character that was
   always there, and that nothing about the layout broke.

4. **Saving an edit does not launder the container.** Mark a container the way a
   download would — `xattr -w com.apple.quarantine '0083;68ae0000;Safari;' c.slpc`
   on macOS, a `Zone.Identifier` stream on Windows — open it, change one value,
   press Save, and check the mark is still there afterwards. The card's *arrived
   from elsewhere* line should read the same before and after.

   Linux and Windows get this from `Destination::in_place`, which the library's
   own tests cover.
   **macOS does not**: `src/staging.rs` replaces the container through
   `-[NSFileManager replaceItemAtURL:…]` and carries the mark itself, and nothing
   in either repository can test that arm. It is the one to run first.

   Then the sandboxed case, which is a separate answer rather than the same one:
   the platform marks what a sandboxed process writes, so the container will be
   marked whatever happens. What to look at is whether the card still says the
   container arrived from elsewhere, because the agent field will now name this
   application and `DESIGN.md` §5 has the card disregarding those. A save that
   quietly turns *arrived from elsewhere* into nothing is the failure, even
   though the file stays gated.

**Items 4 and 5 are now covered on Linux by `tests/handover.rs`**, which walks
the path a payload takes to the operating system: extracted into the scratch
directory the window makes, at the mode it makes it, read back from a separate
process — which is the handler's position on this platform — and a save that
keeps the mark the container arrived with. That file exists because the question
*what tests the handover?* had no answer until 2026-08-27. It cannot stand in
for the runs below on the other two platforms, and it deliberately does not call
`opener::open`: launching somebody's PDF viewer in the middle of a test run is
not a test, and under the macOS sandbox the handler is not in the same position
anyway, which is item 6.

5. **The temporary directories are private.** Two are made: the handover
   directory a payload is extracted into, and the probe directory `opens_with`
   uses on Linux. Both ask for mode 0700 rather than taking the umask's answer —
   `tempfile` puts its directories through the umask, and this repository once
   recorded the opposite as fact. Open a container, press Open, and look at what
   was made *while the window is still up*, which is the only time it exists:

       ls -ld /tmp/slipcase-*                    # Linux, macOS

   On Windows there is nothing to ask for: `%TEMP%` is inside the user's profile
   and inherits its access list, so the check there is that the extracted
   payload is not somewhere else. Confirm with Explorer's Properties, Security
   tab, that only the account and the usual system principals are listed.

   On macOS under the sandbox the directory is inside the container at
   `~/Library/Containers/com.excelano.slipcase-desktop/Data/tmp`, which is
   already private — the question there is the one below it.
6. **An application launched through Open can actually read the payload, under
   the macOS sandbox.** Launch Services normally grants the opened application a
   scope for the URL it was launched with, but the payload now sits in a 0700
   directory inside another application's container and nobody has walked it. If
   this fails, Open fails for every container on the Store build, so it is worth
   doing early and it is cheap: open any container, press Open, and see whether
   Preview shows the PDF or an error.

### Not yet done by hand

All six are done on every platform they apply to; item 6 is macOS only. The last
platform to run the list found what the first two had ticked past, which is the
argument for running a hand item on every arm rather than on the one that owns
the code.

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

### Not yet done by hand

- **Whether the shipped build starts where 0.1.1 did not** — a Windows machine
  with no Visual C++ Redistributable. This is the only check that reproduces
  what the certification tester saw, and the only one that would catch the fix
  being wrong rather than merely absent. Windows Sandbox is the cheap way to it
  and is not installed here; `Enable-WindowsOptionalFeature -Online -FeatureName
  Containers-DisposableClientVM -All` needs elevation and a reboot. Until then
  what stands is `check-imports.ps1`, which says the loader cannot ask for the
  DLL — a narrower claim than the one worth having.

  **The install is *Get* on the listing now**, so this needs a machine and not a
  machine plus a sideload plus a trusted certificate. Take the Store copy rather
  than a local build: it is what the person who found the defect had.
- **Whether the binary the Store serves is the binary that was built.** The
  Store re-signs what it distributes, so the package it serves is not the file
  that was uploaded and no hash kept here describes what a customer receives.
  What should still hold is the executable inside. Install from the Store, find
  it under `C:\Program Files\WindowsApps\`, and hash it against the number
  `SUBMITTING.local.md` records for the submitted build; point
  `check-imports.ps1` at the same copy, which turns the import-table argument
  from a claim about what was built into one about what is being served. Every
  other measurement in this repository was taken on a file this project
  produced, and this is the first link in the chain that is somebody else's.
- **Whether an alternate stream's *name* matters** to what the shell gates on. A
  packaged application reads a `Zone.Identifier` it did not write, unvirtualised;
  whether it would read a differently-named stream is a question about NTFS
  rather than about packaging, and it is unmeasured.

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

### Not yet done by hand

- **A machine that has never had it.** Item 12 runs where every dependency is
  already satisfied, so `apt` pulls nothing and the `Depends` list is never
  exercised. `check-libraries.sh` covers which libraries the application opens,
  which is the half that shipped broken twice; what it does not cover is whether
  `apt` can satisfy the list on a minimal install.
- **A second desktop.** Everything here is GNOME on Wayland with Adwaita. The
  icon defect was a property of which theme carried which name, so KDE or XFCE
  could reach a different answer by the same mechanism.

### What lintian gates on

`linux.yml` runs it and gates on `error,warning`. The two tags it once produced
were decisions rather than defects and both were taken: the package carries a
changelog and a manual page, and `DESIGN.md` §8 has the reasoning for each.

Two `info` tags sit below the gate, untriaged, which is what `info` means here
rather than a claim that they do not matter.
`I: binary-has-unneeded-section .comment` costs a few hundred bytes and
`-R .comment` would remove it; `I: hardening-no-fortify-functions` is lintian
asking a C question of something that is not C. Neither has been decided, and
the gate stops at `warning` so that neither is decided by accident.

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

### Not yet done by hand

- **The interface on a real high-density panel.** Layout is asserted rather than
  looked at: `a_long_comment_leaves_room_for_the_remove_button` runs at
  `pixels_per_point` 1, 1.5, 2 and 3, which is the arithmetic a Retina panel
  produces, and every `.icns` entry was compared byte for byte against a
  re-render from the SVG. What is left is whether it *looks* right, which is an
  idle five minutes on the next Retina Mac. Ask the machine rather than trusting
  a note about it, because monitors get swapped:

      system_profiler SPDisplaysDataType | grep 'UI Looks like'

- **A container on an external drive, and on a network share.** A mounted disk
  image is done and passes. Those two are what an image does not stand in for; a
  share is a different `EXDEV` story again and may not permit `.TemporaryItems`
  at all.
- **What Slipcase shows when a sandboxed save cannot replace its file.** No test
  in either repository provokes it, and whether it fails safely is a `DESIGN.md`
  §5 question. Two commands reproduce it: `chmod 1777` a directory, put a
  container in it owned by somebody else, and press Save. In a sticky directory
  only the owner may rename or delete, and `replaceItemAtURL:` replaces rather
  than writing in place, so the save is refused while the bytes stay writable.

---

## What earlier runs cost

Method rather than findings. Each was paid for once and would be paid for again
by anybody running the list without it.

**Photograph the window, not the screen.** `screencapture` on macOS returns the
desktop with every window omitted, and a region on Wayland returns whatever is
in front. Both platforms have a script that captures by window id and refuses a
result of the wrong size: `packaging/macos/screenshot.sh` and
`packaging/windows/screenshot.ps1`. Park the pointer first — a shot once came
back 2,292 pixels different from its predecessor with none of the difference
being the change it was taken for, because egui drew a field hovered and
focus-ringed under a resting cursor.

**Measure the region you are asking about.** A pixel count taken over the whole
screen answers a question about the terminal behind the window. The card's
contrast figures are sampled inside the card's own rectangle for that reason,
and every colour measurement here carries a control, because a count once
returned the expected answer for the wrong reason.

**Use a container holding a real payload for anything about the handover.** The
conformance fixtures carry 47-byte placeholders named `report.pdf`, correct for
container mechanics and not documents: handing one to a viewer produces *PDF
document is damaged*, which reads exactly like a defect in the handover and is
not one. `packaging/demo-container.sh` builds one that opens.

**A Store-signed macOS build cannot be launched anywhere but the Store or
TestFlight.** AMFI refuses a restricted entitlement without a profile covering
the machine, and a Mac App Store profile covers none. Anything needing a running
application uses a Developer ID build; anything needing the real article goes
through TestFlight.

**Installing the Windows package here needs two administrator actions**, and
`RELEASE.md` has them: the certificate import the shell will not deploy without,
and the elevated prompt the certification kit wants.

**A dependency on the toolchain is invisible from inside the toolchain**, and a
dependency on the desktop is invisible from inside the desktop. Two releases
shipped a library that every build machine had and a clean machine did not, one
per platform. Neither was reachable by running the application where it was
built, which is why both platforms check the artefact instead:
`packaging/windows/check-imports.ps1` and `packaging/linux/check-libraries.sh`.
