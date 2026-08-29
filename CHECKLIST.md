# Checklist: the things only a hand can test

`CLAUDE.md` and both handoff briefs have referred to this file since before it
existed. It did not: `git log --diff-filter=A` finds no commit that ever added
it. Windows wrote the first section, macOS the second, and Linux the third,
after running the association walkthrough that the other two had already been
through.

A section per platform. Each item says what to do, what should happen, and —
where a run found something — what actually happened.

---

## Every platform: the card's new lines, what a save keeps, and where a payload waits

Added 2026-08-27 with `slpc` 0.3.6. Both are lines on the payload card, so no
test in this repository reaches either: there is no UI harness here and the card
is drawn, not returned. The fact each line rests on is unit-tested and the
rendering is not, which is what this file is for.

Both fixtures are in the conformance corpus, so nothing has to be built by hand:

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

   **This item named the wrong fixture until 2026-08-29 and could not be run as
   written.** It said `accept/name-cp437-bit11-clear.slpc`, which `unzip -Z`
   reports as `3.0 unx` with mode 0644 — so followed literally it tested
   `minimal.slpc` twice, and would have been ticked for the case it exists for.
   macOS found that, built a container by hand, and wrote that it belonged in
   the corpus rather than on one machine. It is there now, as
   `excelano/slipcase` `996dcca`: creator system 0 at version 2.0, external
   attributes `0x20`, which `unzip -Z` reports as `2.0 fat`.
3. **The payload name is escaped, and the card does not lie about it.** Open
   `accept/payload-name-bidi-override.slpc`, whose `payload.file` carries U+202E
   RIGHT-TO-LEFT OVERRIDE. The card should read `report\u{202E}fdp.exe`, ending
   in `.exe`, and the Open button beside it should still work.

   What this is checking is not what it looks like. Measured 2026-08-27, egui
   does not apply the override: it lays glyphs out in logical order and gives
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

   This is the defect the whole section owes its existence to, found on Linux by
   measurement on 2026-08-27 and fixed in `slpc` 0.3.7. Linux and Windows get the
   fix from `Destination::in_place`, which the library's own tests cover.
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

5. **The temporary directories are private, on this platform.** Two are made:
   the handover directory a payload is extracted into, and the probe directory
   `opens_with` uses on Linux. Both ask for mode 0700 as of 2026-08-27, having
   been 0755 under the ordinary umask before it — `tempfile` puts its
   directories through the umask and this repository had recorded the opposite
   as fact. Open a container, press Open, and look at what was made:

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

- ~~**All five that apply are done on Linux** as of 2026-08-29, under *What the
  card's three lines looked like here* in the Linux section. Item 6 is macOS
  only. **Windows has had none of them**, and is the only platform left.~~
  **Windows did all five on 2026-08-29 and this section is complete on every
  platform.** Its own *What the card's three lines looked like here* holds them.
  Item 6 is macOS only and was done there.

  **The last platform to run it found the defect the first two had ticked past**,
  which is the argument for running a hand item on every arm rather than on the
  one that owns the code. Item 3 says to look at the card, and macOS and Linux
  both looked at the card and were right. Two rows below it the tree was showing
  the same payload name unescaped, and the item did not point anybody there.
- **Items 1 to 4 were written the day the lines landed and run on neither.** The Linux build launches against both fixtures
  without panicking and draws a window, which is what could be checked from a
  session with no way to take a screenshot — GNOME refused the capture — and it
  is not the check.
- **All six are now done on macOS.** Items 5 and 6 on 2026-08-28 under *What the
  sandboxed handover found*, item 4 under *What saving a downloaded container
  under the sandbox found*, and items 1 to 3 under *What the card's three lines
  looked like on macOS*. Item 6 exists only on this platform. Item 5 still wants
  Linux and Windows, which ask different questions of a different directory.
- ~~**Item 2 cannot be run as written on any platform.**~~ **Fixed at the source
  on 2026-08-29 rather than worked around a third time.** The fixture it named
  records a mode like `minimal.slpc` does, so following the item literally
  tested the same thing twice; macOS built one by hand and wrote that it
  belonged in the corpus. It is there now — `accept/payload-no-mode-recorded`,
  `excelano/slipcase` `996dcca` — so the item runs from the corpus on all three
  platforms and nobody builds a container to run it. Verified to discriminate
  rather than assumed: `payload_mode` returns the high sixteen bits of the
  external attributes and refuses a zero, and this case records `0x20`, whose
  high half is zero.

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

### Before an MSIX is built, three questions

The channel is the Microsoft Store and the format is MSIX, which
`packaging/windows/README.md` records with the reasoning. None of what follows
is paperwork, and macOS is why that is worth saying: the App Sandbox was taken
for a formality there and cost a new module, a rewritten save path, and a
reopened section of `DESIGN.md`. MSIX is a container too, and this application
does three things a container has opinions about.

Run these against a package built from the current binary, not against the
executable. The point is what the container changes, so a measurement taken
outside one measures nothing. A self-signed certificate and a sideloaded
package are enough; a Store account is not needed to answer any of this, in the
same way an Apple Development certificate was enough to answer the sandbox.

1. **Does `opens_with` still get real answers?** `src/opens_with.rs` walks the
   registry along the path the shell takes, to say what would open a payload.
   MSIX virtualises a package's registry writes into a per-package hive; what
   it does to *reads* of the keys other applications wrote is the question, and
   the card is wrong rather than empty if a packaged build sees a private view.
   Put a container holding a `.pdf` and one holding a `.txt` on disk, open each
   from the packaged build, and check the card names the same applications the
   unpackaged build names on the same machine.
2. **Does the Open button still hand a payload over?** `opener::open` reaches
   the shell, and on macOS the equivalent claim — that a sandbox would refuse
   the handover — was asserted and then measured to be wrong, so assert nothing
   here either way. Press Open on a container from inside the package and see
   whether the payload reaches its application. If it does not, the failure
   mode matters as much as the fact: a refusal that reports itself is a
   different problem from one that silently does nothing.
3. **Does a declared association beat a stale `UserChoice`?** MSIX declares
   file types in `AppxManifest.xml` rather than by writing the keys
   `install.ps1` writes, and `UserChoice` is the key that outranks everything
   and the one an uninstaller forgets — the script walkthrough already found
   that. Install the scripts, associate `.slpc`, uninstall them, then install
   the package and double-click a container. Whether the manifest wins, the
   stale key wins, or Windows shows the *how do you want to open this* dialog
   is the answer, and it decides whether the package needs to clean up after
   the scripts.

Two things that are known and need no run. MSIX must be signed for the shell to
accept it, so signing stops being optional on this platform the way it did on
macOS. And the two PowerShell scripts stay whatever these three find: they are
the per-user route for somebody who wants no Store account, and a listing is no
reason to withdraw it.

### What the MSIX sitting found

Run 2026-08-26 against a package built from the release binary, signed with a
self-signed certificate, and installed. Two of the three questions are answered
and the third is answered in every part a script can reach; what is left of it
is one click, below.

**Getting a package installed at all costs one administrator action, and only
one.** Three routes were measured and two are dead without elevation.
Registering an unsigned loose layout fails `0x80073CFF`, which wants Developer
Mode. Installing the signed package fails `0x800B0109` until its certificate is
trusted, and importing that certificate into `CurrentUser\TrustedPeople` does
not do it — deployment still refuses, so the per-user store is not the one it
reads. `LocalMachine\TrustedPeople` is, and writing there is `E_ACCESSDENIED`
without administrator. Everything else — `makeappx`, `New-SelfSignedCertificate`,
`signtool`, and `Add-AppxPackage` itself — needs none. Both SDK tools are stock
on this machine at `10.0.26100.0`.

**The process really was inside the container**, which is worth establishing
before anything measured inside it means anything. `reg add` run through
`Invoke-CommandInDesktopPackage` reported success and the key was not there
afterwards from outside, so the package's registry virtualisation was in force.
A file written to an ordinary path by the same route did land where it was
asked to, so the virtualisation is not general.

**1. `opens_with` gets the same answers inside as outside.** The `opens-with`
example ships inside the package with an execution alias, so the query runs
with package identity and keeps its standard output; the same binary outside
the package is the control. Twenty payload names — `.pdf`, `.txt`, `.html`,
`.zip`, `.png`, `.jpg`, `.docx`, `.xlsx`, `.csv`, `.rtf`, `.xml`, `.json`,
`.md`, `.ps1`, `.bat`, `.exe`, `.mp3`, `.mp4`, `.slpc`, and one extension
nothing is registered for. **Zero rows differed.** Six named an application
(Microsoft Edge for `.pdf`, `.html` and `.xml`, Windows Explorer for `.zip`,
Notepad for `.md` and `.ps1`) and the rest declined, identically both ways. So
MSIX virtualises what a package writes and not what it reads, and the card is
not going to be wrong inside a package. One row is not a control: `.slpc`
answered `Slipcase` from the package's own declared association, because the
package was already installed when the outside run was taken.

**2. The Open button still hands a payload over.** Measured through
`opener::open` at the version `src/main.rs` uses, in a binary built for nothing
else, run inside the container by the route above. It returned success and the
handler appeared — `msedge` went from eight processes to nine. The claim that a
container would refuse the handover is wrong here in the same way it was wrong
on macOS, and it was measured rather than asserted for that reason.

The failure mode was measured too, because the question asks for it. A payload
whose extension nothing is registered for also returns success, inside and
outside alike, and Windows puts up its *how do you want to open this file*
picker. That is not an MSIX behaviour and not a difference — but it does mean
`opener::open` returning `Ok` is not evidence that anything opened, on either
build, which is worth knowing before that return value is ever trusted.

**3. A declared association works, and it does not clear up after the
scripts.** Three states were put to the shell by launching a `.slpc` and
reading which executable started.

| Registered | What a double-click does |
| --- | --- |
| The package alone | Launches `…\WindowsApps\Excelano.Slipcase_…\slipcase-desktop.exe` |
| The package and the scripts together | Neither. Windows shows the *how do you want to open this file* picker |
| The scripts alone | The earlier walkthrough's answer, unchanged |

So the manifest does not need the keys `install.ps1` writes, and does not beat
them either: with both present Windows declines to choose. A package installed
over a live script installation degrades a working association into a prompt,
which answers the question the checklist actually asked — the package does have
to clean up after the scripts, or say plainly that they must be uninstalled
first.

**And a packaged handler has no executable to find.** `AssocQueryString` for
`.slpc` returns the friendly names — `Slipcase` and `Slipcase Container` — and
`ERROR_NO_APPLICATION_ASSOCIATED` for `ASSOCSTR_EXECUTABLE` and
`ASSOCSTR_COMMAND`, because a packaged application is activated through the app
model and there is no command line to report. It declines the same way for the
scripts' own ProgID, which is the 18-in-260 disagreement `src/opens_with.rs`
was written around, arriving from the other direction. Anything checking an
install by looking for an executable path will call a correctly registered
package no association at all, exactly as `assoc` and `ftype` do.

**And one defect fell out of the setup rather than the questions.** Getting
between the three states above meant running `uninstall.ps1` repeatedly, from
the checkout rather than from the install directory. It printed *left
…\uninstall.ps1 behind: it is the script now running* and left the file and its
directory there — and the running script was the checkout's copy, so the
sentence was untrue and the file was an ordinary one it could have removed. The
usual run really is the self-deleting case, because Add/Remove Programs points
at the installed copy, so the branch existed for a good reason and covered both
cases with one. It tells them apart now, by full path. Both were then run: from
the checkout the directory goes entirely, and from the install directory the
message is printed and is true.

**What one click still owes.** Whether a declared association beats a *stale*
`UserChoice` is not answered, and no script can answer it: `UserChoice` carries
a hash Windows validates and its key denies the user write access, which is
deliberate and is why choosing a default is a thing a person does. It is in
*Not yet done by hand* below with the sequence.

**Most of this sitting could be a CI step, and is not one yet.** Everything
above was measured once, by hand, and nothing revisits it. The measurement
itself says why that is fixable: `makeappx`, `New-SelfSignedCertificate`,
`signtool` and `Add-AppxPackage` all ran without administrator, and the one
action that needed it — the certificate reaching `LocalMachine\TrustedPeople` —
is available on a GitHub runner anyway, which is administrator by default. So
building a package, signing it, installing it, and asking `opens_with` the same
twenty questions inside the container as outside is mechanical from end to end.
That is most of questions 1 and 2, turned from a thing somebody did once into a
thing that fails when it stops being true.

`windows.yml` is where it would go. It was written from Linux on 2026-08-26,
for the ordinary reason — eight `#[cfg(target_os = "windows")]` tests ran on no
machine anything revisited — and it does not need to change hands to be added
to: `.github/workflows` is not one of the per-platform directories `CLAUDE.md`
scopes a session to, and re-authoring a working file to change whose name is on
it costs the reasoning in its comments and buys nothing. What does want this
platform is deciding *what* to assert, which has to be learned on the machine
first. That is the order this file exists to serve: walk it, write it down, then
automate the half that needs no eyes.

**Three things that cannot go to CI**, so that a later reader does not try. The
window, and everything about it. A real double-click, and the picker that
appears when Windows chooses neither handler. And the stale-`UserChoice`
sequence, for the reason the paragraph above gives — the key cannot be forged
into place, which is the point of it.

**And one trap to carry into any such step.** `AssocQueryString` answers
`ERROR_NO_APPLICATION_ASSOCIATED` for a packaged handler's executable and
command line while still returning its friendly names, because activation goes
through the app model rather than through a path. An assertion that checks for
an executable will therefore call a correct package no association at all. That
is the `assoc` and `ftype` trap again, and it is the failure a CI step would
most plausibly be written into.

### What running the corpus on Windows found, and what it cost to clear

The conformance corpus had never been run on this platform — it needs Python to
generate its cases and there was none here until 2026-08-26. Run that day
against Python 3.13, with the cases generated and self-checked by the
generator: **76 of the 77 agreed, and the seventy-seventh hung.** It passes now; what it
took is below the diagnosis.

`accept/payload-name-windows-reserved` carries a payload named `CON`. The
container is conformant, the manifest expects `accept`, and `SPEC.md` §2.3 has
a non-normative note about the Windows difference — so nothing here is the
corpus being wrong. Win32 resolves `CON` to the console device wherever the
name appears, so the extracted payload is not a file in the directory, it is
the console. `std::fs::read` of that path never returns: it waits on input a
windowed application will never supply. The run stops there with no output and
no CPU. It was killed at ten minutes, twice, before the case was identified,
which is worth saying because a corpus run that hangs looks exactly like a
corpus run that is slow.

Three names, three different answers, measured one at a time:

| Payload name | `File::create` | `write_all` | Result |
| --- | --- | --- | --- |
| `CON` | `Ok` | `Ok` | No file exists; the bytes went to the console. `metadata` is error 87, `read` never returns |
| `LPT1` | `Err(NotFound)` | — | Fails cleanly |
| `NUL` | `Ok` | `Ok` | Succeeds and discards |
| `report.` | `Ok` | `Ok` | Lands as `report`, the dot silently stripped |

The exposure was the Open button rather than the corpus. `src/lib.rs` named the
extracted file `into.join(payload_name())`, which is right for every other name
and is where this arrived. `DESIGN.md` §8 carries the amendment, and a second
one recording that the repair turned out not to be any of the three choices it
first set out.

**Cleared the same day, and the repair was none of the three above.** The
three candidates were all ways of deciding `CON` is a name this build will not
honour. A fourth was measured after they were written: Windows looks for those
device names while it parses a path, and a path in the `\\?\` verbatim form is
not parsed that way, so the name stops being a device without anybody calling
it a bad name. `fs::canonicalize` answers in that form, so `extract` asks it of
the directory and joins the container's name onto the answer.

Measured across every name that misbehaved. `CON`, `CON.txt`, `con`, `COM1`,
`AUX`, `LPT1`, `PRN` and `NUL` each then wrote, read back byte for byte,
carried a `Zone.Identifier` stream and were removable, exactly as
`ordinary.txt` does — where before, three of them hung, two failed with
`NotFound` and one discarded the bytes. **The corpus then passed all 77 on
Windows, which it had never done.**

It took two changes rather than one, and the second was only visible by
running it. With `extract` repaired the run still hung, further along: the
runner builds a path out of the container's payload name too, to test replacing
a payload under its own name, and that path was still the console. `destination`
is public so the two share the rule instead of each keeping a copy.

The handover is not fixed and now says so. `opener::open` on a device-named
file returns *the specified device name is invalid*, which is an error the
application already has a sentence for, where before it was a hang or a
silence. A payload named `CON` extracts and will not open, and that is the
truth about that container on this platform rather than a defect left standing.

One thing it cost, paid separately. The path handed back is the verbatim one,
because that is what addresses the file, and an existing test caught *Extracted
to* about to show a person `\\?\C:\…` — a spelling they have never seen and
could not type. `shown` takes the prefix off for display and nothing else does.
That test failing is the reason this was noticed rather than shipped.

**The other 76 agree, and that is a real result rather than a consolation.**
Re-run with the one case removed from `manifest.toml` and its container
deleted, so that the runner's own check for ungoverned files stays honest: 76
cases, all agree, 67 metadata trees, 36 payload cards, 35 of 36 payloads
extracted at their declared length with the thirty-sixth refused before
anything was pressed because its member is encrypted, 36 rewritten through
Repack and read back conformant, 36 renamed, 36 replaced. So the provenance
repair above changed nothing the corpus can see.

Two things about running it here at all, for whoever runs it next. The
generator needs Python 3.11 or later and nothing else; 3.13 installed per-user
through `winget` with no administrator. And a freshly built `corpus.exe`
refused to start once with *Access is denied* and ran on the next attempt,
which is Defender scanning a new binary rather than anything about the build —
worth knowing before it is mistaken for a permissions problem.

### What the provenance walkthrough found

Run 2026-08-26. The Windows arm of `src/provenance.rs` had compiled since it was
written on Linux and had never executed against a real zone stream. It works,
and the open question in `DESIGN.md` §5 is answered in the direction the design
assumed.

**What a browser really writes was read rather than imagined.** Twenty-four
files in this machine's Downloads folder carry a `Zone.Identifier`, every one of
them `ZoneId=3` followed by `ReferrerUrl` and then `HostUrl`. That is the shape
the tests and the walkthrough both use. It also confirms the ordering assumption
in the arm's parser is not the only one that occurs, since a real stream carries
two keys after the zone rather than none.

**The container was built here and given a stream of that shape**, rather than
downloaded. Nothing public serves a `.slpc`, and hosting one to download would
mean publishing a file to the internet for a test. Everything downstream reads
the stream and nothing else, so a container carrying one is a downloaded
container to every part of this code — but it is worth saying plainly rather
than letting the record imply a browser was involved.

Walked below the window, through the same calls the buttons make:

| Step | Result |
| --- | --- |
| Card, on a container carrying a stream | `from_elsewhere` is true |
| Card, on a container built here | false — the line is not on everything |
| Extract to a chosen path | payload written, bytes match |
| The chosen payload's stream | all 109 bytes, byte for byte |
| Extract into a directory for handover | payload written |
| The handover payload's stream | all 109 bytes, byte for byte |
| Control: extraction from an unmarked container | no stream invented on the copy |

**The temporary directory is gated exactly as anywhere else, so the Open button
was right not to be disabled.** `§5` chose to report provenance on the card and
let the person decide, and recorded that the choice rested on something
unmeasured: whether the platform treats a marked file in a temporary directory
the same as one anywhere else. It does. A `.cmd` carrying `ZoneId=3` was opened
through `opener::open` — the call the Open button makes — from the temporary
tree and from an ordinary folder, with an unmarked copy in each as a control:

| Location | Marked | What happened |
| --- | --- | --- |
| Temp | yes | the call blocked on the security warning; the file did not run |
| Temp | no | returned; the file ran |
| Ordinary folder | yes | the call blocked on the security warning; the file did not run |
| Ordinary folder | no | returned; the file ran |

The blocking is the measurement: the *Open File — Security Warning* is modal
inside the calling process, so a call that has not returned is a warning on
screen. Both marked cases were killed rather than answered, so nothing ran. A
`.cmd` was used because the warning is shown for file types the shell treats as
risky and not for, say, a PDF — so this says the temporary directory is not a
trusted location, which is what `§5` needed, and does not say every payload
raises a prompt.

**One gap it closed that was not on the list.** The card's provenance line had
no test on this platform: `a_container_from_elsewhere_says_so_and_a_local_one_does_not`
is `#[cfg(target_os = "linux")]` because it marks the container with an extended
attribute. The Windows counterpart is written now, marking with a zone stream
instead, and it was watched to fail — with `arrived_from_elsewhere` returning
false it reports that a container carrying a stream was not called downloaded.

**Still not done by hand: the window itself.** Everything above is the code the
window calls rather than the window. Nobody has watched the card draw that
sentence in the warning colour on this platform, or pressed Open on a downloaded
container and read the warning Windows puts up. That is below in *Not yet done
by hand*, and it is now a much smaller item than it was.

### What the window walkthrough found

Run 2026-08-26 against a release build, with four containers built for it: one
made here, one carrying a zone stream, one whose payload is named `CON`, and one
carrying a zone stream whose payload is `setup.cmd`. It found one defect, in a
sentence this application writes.

**The card is right about provenance, both ways.** A container carrying a zone
stream draws *This container arrived from elsewhere, and the payload will carry
that.* in the warning colour, above the buttons. The container built here draws
nothing there, so the line means something rather than appearing on everything.

**The Open button hands the payload over, and the payload carries the mark.**
Pressing Open on the downloaded container extracted `report.pdf` into a
temporary directory and Edge opened it. Read off the copy afterwards, it carried
all 109 bytes of the container's stream — the card's *the payload will carry
that* is literally true and not a figure of speech.

**A zoned payload in the temporary directory raises the security warning, in the
window.** Pressing Open on the container holding `setup.cmd` put up *Open File —
Security Warning*, naming the file in the temporary directory, *Unknown
Publisher*, *Windows Command Script*, with Run and Cancel. That is what `§5`
needed and it now has it from both directions: measured below the window on the
same day, and watched here. Two things worth knowing. The dialog is a window of
this application's own process, which is why `opener::open` blocks on it rather
than returning. And the window stays alive while it blocks — the card shows the
progress bar at 100% with a Cancel button — so a person who leaves the warning
sitting there does not see a frozen application.

**A payload named `CON` extracts and will not open, and both halves are now
visible.** The card shows `CON`, 32 bytes. Pressing Open wrote a real file — the
directory listing has to be taken through the verbatim form to see it, which is
the whole reason `destination` exists — and the handover failed rather than
hanging. The path in the message carries no `\\?\` prefix, so `shown` is doing
its job in the running application and not only in a test.

**The defect: the sentence said `IO error` and nothing else.** Both refusals
above first reported *the system would not open it: IO error*, which tells a
person nothing about what happened or what to do. `opener::OpenError` keeps its
`Display` to a category and puts the platform's words in `source()`, so
formatting the error threw away the only part worth reading. This is a sentence
the application writes rather than one the platform hands over, so it is this
application's to get right, and `DESIGN.md` §8 had already claimed the person
would see the platform's wording — which was untrue until now. Repaired, and
both cases were run again:

| Pressing Open on | What the person reads now |
| --- | --- |
| A payload named `CON` | *…\CON was extracted, and the system would not open it: The specified device name is invalid. (os error 1200)* |
| A zoned `setup.cmd`, warning cancelled | *…\setup.cmd was extracted, and the system would not open it: The operation was canceled by the user. (os error 1223)* |

**And the card stays quiet where the platform is quiet.** Neither `setup.cmd`
nor `CON` drew an *Opens with* line, because nothing is registered for either.
That is `§3`'s rule working in the window: where the platform will not answer,
the card says nothing rather than guessing.

The walkthrough was driven by photographing the window rather than by watching
it, which is worth recording because it is not the same thing. Everything above
was read off a capture of the application's own window rectangle. Two things
that cost time and are not defects: Windows refuses a foreground change to a
process that did not just launch, so keystrokes go astray unless the window is
clicked, and two warning dialogs stack at the same screen position, so a blind
click at fixed coordinates can dismiss the wrong one.

### What the stale UserChoice run found

Run 2026-08-26, the one part of question 3 no script can reach. David made the
choice by hand — *Open with → Choose another app → Always use this app* — which
wrote `UserChoice` with `ProgId = Excelano.Slipcase` and a hash Windows computed.
That key cannot be forged, which is why this waited for a person.

The script install was then removed **by hand** rather than with
`uninstall.ps1`, because the uninstaller removes the `UserChoice` and would have
destroyed what was being tested. That is also the realistic way to arrive here:
somebody deletes the folder, or moves it, and never runs the uninstaller.

**The answer is that the manifest does not win, and the association is dead
rather than merely wrong.** Three states, each opened through `ShellExecute`,
which is what a double-click performs:

| State | What a double-click does |
| --- | --- |
| `UserChoice` names a ProgID that exists, whose command names a deleted executable | **Refused: *Application not found*.** The package is ignored and no picker appears |
| `UserChoice` names a ProgID that no longer exists at all | The package wins and launches from `WindowsApps` |
| No `UserChoice` | The package wins |

So the package must be installed onto a machine where `uninstall.ps1` has been
run, or onto one that never had the scripts. The middle row is the trap: the
person is left with an extension that opens nothing, an installed application
that Windows will not reach, and no dialog offering a way out. Installing the
package does not touch `UserChoice` — measured, before and after.

**It also refines what this repository already recorded about a dead ProgID.**
`packaging/windows/README.md` says that leaving a `UserChoice` behind while
removing the class keys leaves the extension pointing at a ProgID that no longer
exists, and that Windows does not fall back — it treats the extension as having
none. That was measured with the scripts alone and it stands for a machine-wide
association. A *packaged* association is reached in that state: the second row
above is exactly that case, and the package opened the container. So the two
findings are about different fallbacks and neither replaces the other.

What that leaves for the packaging, when it is built: an MSIX cannot run code at
install time, so the package cannot clear a `UserChoice` itself. Either the
listing tells a person to run `uninstall.ps1` first, or the application clears a
stale one at startup — which is a decision rather than a repair, because it
means writing to the key that exists to record a person's own choice.

**Amended 2026-08-28: the second option does not exist. It was built, measured,
and reverted the same day.** David chose it, it was written, and it works
everywhere except the one place it is needed.

The code was not the problem. `clear_stale_user_choice` removed the key only
where the recorded choice named this application's own `ProgID` *and* that
`ProgID`'s open command named an executable that was not there — never another
application's choice, never a live install's, and never on a command it could
not parse. Four tests, each broken deliberately and watched to fail. The dead
state was reconstructed rather than described: the `ProgID` was recreated
pointing at the deleted `%LOCALAPPDATA%` executable, a double-click gave
*Application not found* with the installed package ignored, and running the
**unpackaged** release binary removed the key and restored the association.

**The packaged build did nothing at all, and MSIX registry virtualisation is
why.** The same state, the application activated through the app model, and the
key untouched afterwards. Confirmed with a probe that spent no state: a key
created under the same `FileExts` path from outside, deleted from inside the
container through `Invoke-CommandInDesktopPackage`, was still there afterwards;
deleted from outside as a control, it went. The container swallows the write.
This is the other face of the 2026-08-26 finding that a package's *reads* are
not virtualised while its writes are — that one was good news and this one is
the bill for it.

**And it is not worth keeping for the side-loaded install either**, which is
what settled it. Clearing the key only helps if something else then supplies a
working association, and that something is the package. Without the package the
class keys still name the missing executable, so the extension stays broken
after the repair. It helps only the packaged case and cannot run in the packaged
case. Working code that cannot act where it is needed is worse than none,
because it reads like the problem is handled.

So telling people is the only option left — but not in the store listing, which
took asking who it is for. There are no script-install users: the scripts have
only ever existed in this repository and the application has never been
released. The problem is prospective rather than retrospective, because the
scripts stay after this release deliberately, so the warning goes where those
people are, beside the command they would be running:
`packaging/windows/README.md` carries it, with both failure states and the
reason `uninstall.ps1` is the thing to run. One route was not tried and is
recorded as refused rather
than missed: `unvirtualizedResources` is a restricted capability that would turn
the virtualisation off, needs Microsoft's approval, and would spend the
capability story `AppxManifest.xml.in` is built around — one capability,
`runFullTrust`, and nothing a person has to read and wonder about.

A third possibility is untried rather than rejected: a packaged application can
still *read* the registry, so it could detect this state and tell the person how
to clear it in Settings. That is informing rather than repairing, and it is a
question about what the window says, which nobody has asked yet.

**The state this section was measured against is spent.** The `UserChoice` was
removed by the unpackaged run, and it cannot be forged — anything further here
needs somebody to choose *always open with* again.

**Reproduced 2026-08-28 against the real package, and the second row holds.**
The run above was made against a package carrying an invented identity, because
the name had not been reserved yet. This one was made against the package
`build-msix.ps1` builds from the identity Partner Center assigned that morning:
the same `UserChoice` from 2026-08-26 was still in place with its hash intact,
the script install was removed by hand again leaving that key alone, and
`ShellExecute` on a container launched
`C:\Program Files\WindowsApps\…\slipcase-desktop.exe` with no picker.
So nothing about the answer depended on the identity being a real one, which was
not obvious beforehand.

**And it was double-clicked from Explorer**, later the same day, by David at the
machine — the packaged application opened the container. That is a separate item
from the paragraph above rather than a restatement of it: everything measured
here reached the shell through `ShellExecute`, which is what a double-click
performs and is not somebody performing one. The two agree, which is the answer
that was wanted and not the one that was owed.

It is worth saying what this run did *not* re-establish. It reproduced one row
of three. The first row — a `UserChoice` naming a ProgID that still exists whose
command names a deleted executable — is the trap, and it was not set up again,
because doing so needs a fresh human choice and the one on this machine is spent
on the second row.

**A free check came out of it.** Windows computed the installed package's family
name, and it matched byte for byte what Partner Center calculated from the same
two identity values. That hash is over `Name` and `Publisher`, so a single wrong
character in the `Publisher` GUID would produce a different family name — which
makes `Get-AppxPackage` after a local install a check that the identity was
transcribed correctly, without logging in
to anything. It costs nothing and it catches the mistake that is otherwise found
at upload.

### What the high-density run found

Run 2026-08-26 at 125% and 200%, David changing the setting and this session
photographing the result. The drawing holds up at every size it was looked at,
and a justification written beside it does not.

At 125% Explorer's Details view draws the icon's own 20-pixel entry: the case,
the card and both lines all read, and the outline greys the way `packaging/windows/README.md`
already records for the small sizes. Large icons come from the 128-pixel PNG and
are clean. The Type column says *Slipcase Container* at scaled DPI as it does at
100%. The task bar and the title bar read cleanly at 125% and at 200%.

**The claim that failed is about which entry the window gets.** `src/main.rs`
hands the 64-pixel entry to the window and said 64 was *a whole multiple of every
size Windows draws it at*, on the reasoning that a high-density display doubles
16 and 32 to 32 and 64. Windows does not only double. 125% asks for 20 and 40,
150% for 24 and 48, 175% for 28 and 56, and 64 divides none of those:

| Scaling | Title bar | Task bar |
| --- | --- | --- |
| 100% | 16 (64 ÷ 4) | 32 (64 ÷ 2) |
| 125% | 20 (× 3.2) | 40 (× 1.6) |
| 150% | 24 (× 2.67) | 48 (× 1.33) |
| 175% | 28 (× 2.29) | 56 (× 1.14) |
| 200% | 32 (64 ÷ 2) | 64 (64 ÷ 1) |

So at the scaling most people who turn scaling on actually use, the window icon
is resampled rather than downsampled evenly. It reads fine anyway, which is why
this is a correction to a comment rather than a change to the code: 64 stays the
choice because it is the largest entry no scaling has to enlarge. The comment and
the README paragraph both say that now, and both say they were written before
anybody had looked.

One limit on the looking. Neighbouring task bar icons sit inside any crop wide
enough to hold this one, so the sizes were judged from magnified captures rather
than from a measured bounding box per size. *Legible and clean at every size
looked at* is the claim, and it is not the same as *every size was measured*.

### What the upgrade run found

Run 2026-08-26. An upgrade over an existing install works, and an upgrade over a
*running* install fails in the right direction and used to say so badly.

Over a stale install, with nothing running: the executable and the icon are both
replaced — checked by hash, after deliberately overwriting both with rubbish —
and one install remains one install. One Add/Remove entry, one Start menu
shortcut, one ProgID under `OpenWithProgids`, three files in the directory.
Afterwards a double-click still launches the installed copy and the type query
still answers `Slipcase`. A file left behind by an imagined older version stays
where it is: this script writes its own files and does not clear the directory,
which is deliberate and is also why `uninstall.ps1` leaves a directory holding
anything it did not put there.

Over a running install, Windows will not let the executable be replaced. The run
stops at `Copy-Item` and — this is the part worth keeping — stops *before* the
registry stage, so nothing is left half-registered and the previous install goes
on working. Proved with a marker written into the class key beforehand: it was
still there afterwards.

**What it said about that was the defect.** The script stopped with a .NET
`IOException` and a stack trace naming `Copy-Item`, which is true and tells
nobody what to do. It names the running application and the path now, says to
close it and run again, and says that nothing has been changed — which is a
claim the marker above is what proves. Both paths were run again afterwards: the
running one gives the sentence and changes nothing, and the closed one installs
and rewrites the marker away.

### What the second account found

Run 2026-08-26 with a standard local account, `slipcasetest`, signed in on the
console while the installing account stayed logged on. The per-user install is
invisible to it, which is what `packaging/windows/README.md` claims and what
nothing had checked:

| Asked of the second account | Answer |
| --- | --- |
| `HKCU` `.slpc`, the ProgID, `FileExts\.slpc`, the Add/Remove entry | all absent |
| `%LOCALAPPDATA%\Programs\Slipcase`, the Start menu shortcut | absent |
| `HKLM` `.slpc`, `HKLM` ProgID, the all-users Start menu | absent — the machine-wide half was never written |
| The merged `HKCR\.slpc` | no such key |
| This application's own type query, on `.slpc` | *the platform did not answer* |
| The same query on `report.pdf` | Microsoft Edge — a machine-wide association, so it answers |

The last row is the control that makes the rest mean something: the account is
not simply blind to every association, it is blind to this one.

**One line of the run is void, and it is the harness's fault rather than a
finding.** The check also asked what a double-click would do and reported
*LAUNCHED from* with no path. `Get-Process` sees processes in every session, so
it found the copy running in the installing account's session and could not read
its path, that process belonging to another user. Nothing launched. The script
filters by session now, so a re-run would answer it, and the registry rows above
already settle the question the item was asked for. Recorded rather than
quietly dropped, because a green line that came from the wrong process is the
shape of thing this file exists to catch.

### What light mode found

The walkthroughs above were all run in dark mode, which is not a neutral choice:
the card colours two of its lines on purpose, and egui picks those colours
against a dark background. Looked at in light mode on 2026-08-26 and then
measured, against the card's own fill:

| Line | Light, before | Dark, before |
| --- | --- | --- |
| *This container arrived from elsewhere…* | **2.79:1** | 7.53:1 |
| A failure the card reports | **3.76:1** | **4.31:1** |
| Ordinary text beside them | 7.59:1 | 5.12:1 |

WCAG asks 4.5:1 for body text and 3:1 for large text. The provenance line failed
both. That line is coloured because a walkthrough found that nobody reads a weak
grey one — and in the theme half the world runs, it had become the least
readable thing on the card. The dark red was under the bar too, so the theme
that *had* been looked at was failing quietly as well.

`warn_colour` and `error_colour` in `src/main.rs` choose per theme now, all four
call sites go through them, and a test holds both themes to 4.5:1. Measured
after: 5.18:1 and 6.72:1 on the light card, 7.53:1 and 5.34:1 on the dark one.

**Every dark-mode figure in this section was wrong when it was first written,
and the sentence that was supposed to catch it is why they survived.** The dark
column read 7.12:1 and 4.07:1, the paragraph above read 5.06:1, and the
body-text row read 19.77:1. The dark figures had been computed against grey 32
— `faint_bg_color` composited over the panel, which nothing draws behind the
card — rather than against the panel's own grey 27; the body-text figure was
pure black on the light panel, and egui draws body text in grey 80. Reviewed on
Linux on 2026-08-26 and recomputed against `Visuals::panel_fill` both ways,
which is what `Frame::group` leaves showing through, since it sets a margin, a
radius and a stroke and no fill.

The check this section claimed was *the test reproduces the 2.79 exactly when
the repair is removed, which is also what says the fill it uses is the fill the
screen showed*. Removing the repair does reproduce 2.79 — and 3.76, and every
other light figure, because light was computed against the panel. It reproduces
4.31 where the table said 4.07, and nobody ran it in dark mode to see. A
cross-check that only covers the half you were already looking at reads exactly
like one that covers both. Every figure above is now one the test prints, and
the test asks for the body-text row too rather than having it written alongside.

None of the conclusions move: 4.31:1 is under 4.5:1 the same way 4.07:1 was, and
the repair was needed in both themes.

**Looked at on 2026-08-28, which had not happened: the repair was measured and
never seen.** Everything above repaired the colours and proved the repair with a
test, and every walkthrough on this platform ran in dark mode — so the light card
had been looked at while it was broken and not since it was fixed. David switched
the desktop to light while the screenshots were being taken and it cost two more
captures. The provenance line reads clearly. It is a look rather than a
measurement, which is exactly what this section had been missing.

**And the pixels were measured too, out of the screenshots rather than out of the
code**, because the whole embarrassment above was figures computed against a
colour the screen never showed.

| | Card fill, as drawn | Darkest ink pixel | Contrast |
| --- | --- | --- | --- |
| Light | `#F8F8F8` | `#B95312` | **4.59:1** |
| Dark | `#1B1B1B` | `#FE8E00` | **7.45:1** |

Dark agrees with the test's 7.53:1 within antialiasing. Light comes out 4.59
against the test's 5.18, and the difference is not a defect in either: the
configured colour is `rgb(180, 70, 0)`, whose contrast on `#F8F8F8` computes to
5.18:1 exactly, and no antialiased glyph pixel quite reaches its own colour. So
the two numbers say different things and both are worth having — 5.18:1 is the
colour, 4.59:1 is the worst pixel an eye actually receives, and **both clear
4.5:1**. The second is the one this section had no way of knowing before, since
it can only be taken off a screen.

Looked at as well as measured, which is the point of this file: the line is a
deeper rust orange on the light card, plainly legible and still plainly a
warning rather than body text. The rendered pixels are `rgb(180,70,0)`, the
colour asked for.

### What looking at the tiles found

Looked at 2026-08-28, David at the machine, against the installed package. The
Start menu tile and the application list entry read correctly, which settles the
one part of the assets that was a reading of Microsoft's guidance rather than a
measurement: the drawing at two thirds of a tile and filling an icon-shaped
asset is right.

**The taskbar was not right, and nobody had thought to look there.** The icon
sat on a purple square. That is not the drawing and not the desktop: the
manifest declares `BackgroundColor="transparent"`, so where Windows draws a
*plated* icon it fills the plate with the user's accent colour, and this
machine's accent is `#744DA9` — read out of `HKCU\…\DWM\ColorizationColor` and
identical to the purple on the screen. The asset itself is transparent at all
four corners, checked pixel by pixel.

**So the packaged application and the side-loaded one wore different faces**,
which is exactly what `AppxManifest.xml.in`'s comment about mirroring
`install.ps1` exists to prevent: the script install draws the same icon unplated
from `slipcase.ico`.

The repair is an `altform-unplated` asset, and it took two changes rather than
one. `make-ico` now writes the target-size and unplated variants — and the
`scale-` variants beside them, which were missing for the same reason and would
have had the shell rescaling one bitmap at every display scaling this repository
already cares enough about to render nine `.ico` sizes for. **And a variant is
inert without a resource index**: a qualifier is resolved through
`resources.pri` and nowhere else, so all forty-five images would have shipped
with the shell reading only the five the manifest names by literal path.
`build-msix.ps1` runs `makepri` now, and `makepri dump` confirms the unplated
qualifier is indexed rather than merely present.

**Looked at after the change, the same day: the plate is gone.** David, at the
machine, against the rebuilt package installed from `build-msix.ps1 -SelfSign`.
So the repair is measured from both sides rather than reasoned about from one —
which matters here more than usual, because two of its three parts were changes
whose absence looked exactly like their presence. The images were in the package
before `makepri` ran, and the scale variants were in `resources.pri` before the
`autoResourcePackage` elements came out of the configuration; in both states the
package built, installed, launched, and drew a purple square.

### What the certification kit found, over three runs

Run 2026-08-28 by David from an elevated prompt, which is what the kit needs;
`build-msix.ps1 -SelfSign -Certify` drove it. It has never been run before, and
`packaging/windows/README.md` had it listed as unrun since the channel was
chosen. **Overall: WARNING.** The package it tested carried five images and no
resource index — the run started before the assets below landed, so it will want
repeating.

**Two tests did not pass, and the kit's own overall verdict hid one of them.**
`Blocked executables` read FAIL while the report's `OVERALL_RESULT` said
WARNING, so a gate reading only the overall would have called a failing test a
warning. `build-msix.ps1` now refuses on either and prints every non-passing
test with its messages; the first version printed nothing at all, because it
looked for an `OVERALL_RESULT` attribute and only the report element has one.

**FAIL — Blocked executables.** Five messages against `slipcase-desktop.exe`:
references to `kernel32.dll!CreateProcessW` and `shell32.dll!ShellExecuteW`, and
blocked references to `"CsI"`, `"cmd.exe"` and `"\cmd.exe"`.

Traced rather than argued about. The two `cmd.exe` strings are in the **Rust
standard library**: both sit beside a `library\std\src\sys\…` path in the
binary, and the longer one is `cmd.exe /e:ON /v:OFF /d /c`, which is std's
batch-file spawn. A hello-world Rust binary contains neither string, so they
arrive with `std::process`, not with the toolchain unconditionally. Every
`std::process::Command` in this repository is inside a `#[cfg]` arm for Linux
(`opens_with.rs`, asking `xdg-mime`) or macOS (`staging.rs`, driving `hdiutil`
in a test), so none of it compiles on Windows — the string is reachable code in
std that this build never calls.

`ShellExecuteW` is `opener`, and it is the application's whole declared purpose:
`opener` 0.8.5's Windows arm calls `ShellExecuteW` and nothing else, which was
read rather than assumed. Handing a payload to whatever the system registered
for it is what the Open button is.

~~`"CsI"` was not traced. It is three characters and the kit matched it in a
binary; that is as much as is known.~~ **Traced on 2026-08-29, and it is a
coincidence in a data table.** The string is `CSi` — this file had the
capitalisation wrong, which is worth knowing because three characters is all
there is to go on. It occurs exactly once in the binary, at `0xa4e8e4`, inside
`.rdata`, in the middle of a run of four-byte values that all look alike:

    1c 58 69 ff   4e 58 69 ff   fe 51 69 ff   54 57 69 ff
    43 53 69 ff   86 57 69 ff   c0 53 69 ff   c4 55 69 ff

Read as little-endian words those are `0xff69581c`, `0xff69584e`, and so on, and
`CSi` is the low three bytes of `0xff695343`. It is a table of addresses, not a
string, and the kit's scan is a substring match over the whole file. **So the
answer is that the binary does not contain the word at all**, which is a
different statement from *it was matched and we do not know why*.

**It comes and goes between builds, and that is the corroboration.** The report
kept from 2026-08-28 carries four messages for this test and no `CSi`; the
2026-08-29 run of the same code at 0.1.1 carries five and has it; and the run
after the macOS row fix, later the same day, carries six — `CSi` back as `Csi`
and a `REg` beside it. Three builds, three different sets. Nothing about
the source changed to cause that — the addresses in that table move when the
binary is relaid out, so whether the three bytes `43 53 69` fall next to each
other is chance. A finding that appears and disappears with a rebuild is exactly
what a coincidental byte match looks like, and it is why the gate is written
against finding names rather than message counts.

**What is not known is whether the Store minds.** The kit did not escalate it,
which is a hint and not an answer, and this is a submission policy question that
`RELEASE.md` deliberately stops short of. It is recorded here as a decision
rather than a repair, because the alternative — removing `ShellExecuteW` — is
removing the application.

**Amended: the kit says in its own configuration that this test is optional for
this kind of application, and that is a measurement rather than a hint.** Asked
on 2026-08-28 after the third run, of the files the kit ships rather than of any
documentation. `en-us\configuration_locdata.xml` gives *Blocked executables* the
task id `069E8A26-F39D-4402-81E0-112A2B2E8538`, and `configuration.xml` defines
that task as

    INTERNAL_NAME="DetectBlockedExes" ... OPTIONAL_FOR_APP_TYPES="Centennial"
    REQUIREMENT_ID="2AFBE4A0-…"   (requirement title: "Package sanity test")

and the report's own root element says `APP_TYPE="Centennial"`, which is what a
packaged desktop application is. So the failing test is one the kit marks
optional for exactly this application type — which is also the mechanism behind
the thing that had only been a hint: an optional test failing is why three runs
reported `WARNING` overall over a test reading `FAIL`.

`DPIAwarenessValidation` is the contrast worth noting, because it shows the
attribute means something. Task `A7B07CFF-24B6-41D0-B677-302C7AA2DB7A`, under
the requirement *High-DPI support*, carries **no** `OPTIONAL_FOR_APP_TYPES` at
all — it is not optional, and it still only warns, so the two knobs are separate:
whether a test applies, and how loudly it complains.

**This is still not the Store's answer and must not be written down as one.**
What was measured is how the kit classifies the test on this machine.
Certification runs this suite, so it is a good deal more than a guess — but a
human reviewer applies policy on top of a suite, and nothing local can measure
that. The decision in `RELEASE.md` stands; it is merely far better informed than
it was an hour ago.

**The second run is void, and is kept here rather than dropped.** Attempted the
same day against the package carrying the forty-five assets and the resource
index. `appcert` refuses to overwrite an existing report — *Specified report …
already exists. Please specify a unique report file name.* — and stopped before
running a single test, which is visible in its output as the thirty task lines
of the first run being replaced by none. `build-msix.ps1` then found the report
present, parsed it, and printed **the first run's findings as this run's**,
identical to the letter, which is exactly why they looked so reassuring.

That is the failure the script's own comment claimed to guard against. It
distinguished a kit that ran and failed from a kit that never ran, and *stale*
is a third state neither word covers. The report is now removed before the kit
is started and the one that appears is required to be newer than the run; the
predicate was checked both ways, though only against a constructed file, because
proving the whole path needs an elevated run.

**The third run is the real one for the packaged assets, and it found nothing
new.** Run the same day, elevated, against the forty-eight-file package: the
forty-five images, `resources.pri`, the manifest and the executable. Twenty-four
tests, twenty-two of them passing, and the two below unchanged to the letter.
The report was written under a minute before it was read, which is now checked
rather than assumed — and the guard passing silently on a real run is the only
evidence there is that it does not misfire, since the void run only ever proved
the other direction.

**The four tests the assets could have moved all pass**: `App resources`,
`Resource Packages`, `Branding`, `App manifest`. So the resource index is
accepted, nothing objects to forty-five images where there were five, and
`Resource Packages` in particular is content with one package carrying its own
qualified resources — which was the arrangement `makepri`'s default
configuration had to be talked out of.

Both remaining findings are about the executable and neither has anything to do
with packaging, which is what the run was for establishing.

**And the gate those three runs were feeding was useless, which the third run is
what showed.** `-Certify` refused on any test that was not PASS, and
`Blocked executables` fails on every run this project will ever do — so it
refused every time, identically, whatever the package contained. `CLAUDE.md` has
the name for that in the paragraph about the check for compiled C: *a check whose
red is the normal state announces nothing.* It was rebuilt against the same
argument. The two findings are recorded in `KNOWN_FINDINGS` in the script, with
what each was traced to, and the gate now refuses on a finding that is **new** or
**worse than recorded** and says so when a recorded one stops being reported.

**Recording a finding there is not accepting it**, and the comment above the list
says so: whether to submit with `Blocked executables` failing is a decision, it
is David's, and `RELEASE.md` carries it. The list exists so that the next run
after that decision can still be surprised by something else.

**It was taken on 2026-08-28 — submit with it failing — and `RELEASE.md` did not
say so until 2026-08-29.** Named here as well as pointed at, because a reader
following this pointer for a day found the argument and no answer.

`-ReadReport` applies the gate to a report that already exists and does nothing
else — no elevation, no build. That is what makes the gate checkable, and it was
checked three ways: a finding taken out of the list is refused as not in it, a
recorded verdict changed to a worse one is refused the same way, and a listed
finding the kit does not report prints *gone* and exits zero, because that is
news rather than a failure. Without `-ReadReport` each of those costs an elevated
session and several minutes, which is the sort of price that stops a check from
being re-checked.

**WARNING — DPIAwarenessValidation.** Two messages: *Failed to process the
binary* and *The app … is not DPI Aware*.

**Measured against the running process, and it is not true of the behaviour.**
`GetWindowDpiAwarenessContext` on the packaged application's own window, while
it was running from `WindowsApps`, reports `PER_MONITOR_AWARE` — winit sets that
at startup. The kit reads the PE application manifest, which carries no
`dpiAware` element, so what it found is a missing *declaration*. The hand run at
125% and 200% recorded above agrees with the process and not with the kit.

Declaring it statically needs a Win32 manifest embedded in the executable, and
that is the same wall the window icon hit: `rc.exe` and `windres` are build
steps `DESIGN.md` §2 keeps out, which is why `main.rs` carries the icon through
`include_bytes!`. There is a route that does not compile anything — the MSVC
linker takes `/MANIFESTINPUT` with `/MANIFEST:EMBED` through `-C link-arg` —
but taking it is a `DESIGN.md` §2 decision and not this section's.

**Taken, on 2026-08-28, with David's agreement.** `build.rs` is the crate's
first build script and prints those two linker arguments on `windows-msvc`;
`packaging/windows/slipcase-desktop.manifest` is what it embeds, and it declares
`dpiAware` and `dpiAwareness` and nothing else. `DESIGN.md` §2 is amended with
why a linker argument is not the build step that section keeps out. It reads
`CARGO_CFG_TARGET_*` rather than `cfg!`, because a build script is compiled for
the host and would take the wrong branch under the Linux cross-check.

Measured after, and **the honest result is that it changed nothing about how the
application behaves**. The manifest is embedded — 654 bytes, read back out of
the built binary as an `RT_MANIFEST` resource, with the linker's own
`asInvoker` trust block merged into it. The application launches and its window
context is `PER_MONITOR_AWARE_V2` exactly. But so was the packaged build from
*before* the change, tested the same way: winit was already setting V2 inside
`EventLoop::new`, so nothing was broken and nothing got fixed.

What it does buy is two things, and neither is the one that would have justified
it on its own. The awareness is now set before any of this program's code runs
rather than a moment into it, which closes a window that was never observed to
matter. And a tool reading the binary can see the declaration, which is the
whole reason the kit complained. Whether the kit is satisfied is the open item:
the first message it gave was *Failed to process the binary*, which may mean it
never read the file rather than read it and found nothing declared.

**The fourth run says it worked, and the kit now passes overall.** Run the same
day, elevated, against a package built from the manifested binary. The gate
printed *gone — DPIAwarenessValidation is no longer reported*, the finding was
taken out of `KNOWN_FINDINGS` on the run that reported it, and the report reads
**`OVERALL_RESULT="PASS"`**: twenty-three tests passing and one failing, where
the run before it was twenty-two passing, one failing and one warning.

**That the overall is PASS while a test reads FAIL is the strongest thing here,
and it is not about DPI.** It confirms the reading of the kit's own
configuration taken earlier: `Blocked executables` carries
`OPTIONAL_FOR_APP_TYPES="Centennial"`, so failing it does not stop the kit
passing the package. Before this run that was an inference from an XML attribute
and a WARNING; now the same package with the same failing test comes out PASS
because the one *non*-optional complaint was answered. The two knobs behaved
exactly as the configuration said they would.

**And a fifth message disappeared that nobody repaired.** The first three runs
reported `a blocked executable reference to "CsI"` alongside the two `cmd.exe`
ones; this run reports four messages and `CsI` appears nowhere in the report.
Nothing was done that could remove a real reference to a program of that name,
and no such program was ever identified — the earlier entry says plainly that it
was not traced. Embedding a manifest changes the layout of the binary, and a
three-character match in binary data is exactly the kind of thing that moves
when layout does. **That is the likeliest explanation and it is not a
measurement**, and it is written down chiefly so that nobody later reads its
absence as something having been fixed.

Two things worth keeping for whoever touches this next. winit's run-time call
must now be failing, because awareness cannot be changed once set — and it does
not care: the application launches and behaves identically, which was checked
rather than assumed, because "the library probably ignores that error" is the
shape of claim this repository keeps catching. And the linker argument is
attached to one named binary rather than to `-bins`: `corpus.exe` was checked
and carries no manifest, which is right, since a console runner has no window to
be aware about.

### What certification said at 0.1.1

Run 2026-08-29 against `Slipcase-0.1.1.0-x64.msix`, self-signed, elevated.
**`OVERALL_RESULT="PASS"`**, with `Blocked executables` failing as recorded and
nothing else. The gate recognised it — *FAIL Blocked executables (known - see
CHECKLIST.md)* — and passed the run, which is the first time the rebuilt gate has
been exercised by a real run it should not refuse.

The five messages are `CreateProcessW`, `ShellExecuteW`, `cmd.exe`, `\cmd.exe`
and `CSi`, all traced above.

**Run again the same day after the macOS row fix, and it passed again** —
`OVERALL_RESULT="PASS"`, the gate recognising the finding. That is the run the
submission rests on, since the tree changed twice in a day and the three runs of
2026-08-28 were against a binary that no longer exists.

**It came back with six messages rather than five, and the two extra ones finish
the `CSi` argument.** They are `Csi` — the same three letters, differently
capitalised — and `REg`, which was not there before. Located in the new binary,
one occurrence each, and both in `.text`:

    Csi   0x474e76   ... 48 8d 05  43 73 69 00  48 89 85 ...
    REg   0x49eb30   ... 48 8d 05  52 45 67 00  41 b8 0d ...

`48 8d 05` is `lea rax, [rip+disp32]`, so in both cases the matched bytes are
three of the four bytes of a RIP-relative displacement: 0x00697343 and
0x00674552. Not strings, not data the compiler put there to be read, but the
operand of an instruction — and the previous run's `CSi` was in an address table
in `.rdata` instead, which is the same accident landing somewhere else.

**And the capitalisation is the tell.** `csi` is the C# interactive compiler and
`reg` is `reg.exe`; the kit is matching blocked executable *names*
case-insensitively and echoing back whatever bytes it found, which is why the
same finding reads `CSi` in one run and `Csi` in the next. Three letters against
a six-megabyte binary will keep hitting by chance, and which ones hit changes
whenever the code is relaid out.

So the honest summary of this task, for whoever reads the report next: two of the
messages are real and understood — `ShellExecuteW` is the Open button and
`CreateProcessW` and the `cmd.exe` strings are the Rust standard library — and
the rest are noise from a substring scan. Nothing about them changes between runs
except which random three bytes happen to spell an executable's name.

### What taking the screenshots found

Taken 2026-08-28 by `packaging/windows/screenshot.ps1`, against the packaged
application, and the entry that asked for them was wrong about them. `RELEASE.md`
had screenshots under *by hand, because no script can*; a script does the whole
mechanical part. What it genuinely cannot do turned out to be narrower than the
entry claimed and is now said by the script itself when it finishes: which
container to open is editorial, and whether the picture is a good advertisement
is a look.

**Two measurements, both a handful of pixels, and both would have produced an
upload refused at the far end.** `SetWindowPos` sizes the *window rect*, which on
this platform carries an invisible resize border outside the visible frame:
asking for 1366 x 768 produced a visible frame of 1352 x 761, sixteen pixels
under the Store's minimum in one dimension and seven in the other. The visible
frame is `DWMWA_EXTENDED_FRAME_BOUNDS` and the border measured 14 by 7.

And that frame's top edge is one pixel above what is actually drawn, so a capture
at exactly the frame rect includes a sliver of whatever is behind the window. It
arrived as a strip of console text across the top of the first two attempts,
which is the sort of thing that reaches a store listing because nobody looks at
the top eight pixels of their own screenshot. The capture is two rows taller than
wanted and the top two are cropped. The script refuses if the frame does not come
back the size it expects, which was checked by breaking the border constant and
watching it refuse.

**The containers are a demonstration built for this**, because the walkthrough
fixtures have three metadata keys between them and the tree is the thing worth
photographing. A real one-page PDF, and metadata covering a string, two dates, an
array, integers, a float, a boolean, nested tables and an array of tables — one
of each thing `src/tree.rs` has a renderer for. The second shot is the same
container carrying a `Zone.Identifier`, so the card's provenance line is in it.
`packaging/store-listing.md` records both.

**Retaken 2026-08-29, and the reason is the reason the script exists.** The
containers above were built by hand on this machine and were in no repository,
so the four pictures could not be reproduced anywhere — and when Linux wrote
`packaging/demo-container.sh` to fix that, `store-listing.md` gained a sentence
saying the shots were of what the script builds, which they were not. All four
were taken again from the script's container, against the packaged 0.1.1, both
themes: 1366 x 768, and the light pair with `AppsUseLightTheme` and
`SystemUsesLightTheme` set to 1 and put back to 0 afterwards.

**Taken a third time later the same day, after the macOS row fix landed**, so
the pictures are of the binary that was certified rather than of the one before
it. Rebuilding at an unchanged version needed the install removed first, which is
the `0x80073CFB` case the build script's output names.

**And the retake is what found the pointer.** The first shot back differed from
its predecessor in 2292 pixels, which was not the row fix at all: the mouse
happened to be resting over an *add a key* field, so egui drew it hovered and
focus-ringed, and the pointer being inside the scroll area drew the scroll bar
as well. Neither is wrong and both read, in a store listing, as an interface
doing something. `screenshot.ps1` now moves the pointer to the far corner of the
virtual screen before it captures — the corner rather than a constant, because
1900 x 1200 is off-screen on a smaller display and Windows would clamp it to an
edge the window might be occupying. Checked by putting the pointer *over* the
window at 600, 400 and taking a shot: it came back clean, and the pointer had
moved.

That is the second time this script has been wrong about something outside the
window rather than inside it, the first being the sliver of console text along
the top edge. The size is the part it guarantees; the resting state was assumed
until it was not.

**Two things fell out of the second round, neither of which was the point of it.**

`Add-AppxPackage` installed 0.1.1.0 straight over 0.1.0.0 with no removal — the
`0x80073CFB` that made *remove the installed one first* a line in the build
script's output is about redeploying the *same* version, and an upgrade to a
higher one is not that. That is the second upgrade this platform has run, and
the first over a package rather than over a script install.

And it answers most of the origin-note question this file was holding open. The
packaged application read `Zone.Identifier` off a container in
`C:\Users\david\slipcase-shots\` — a file outside the package, written by
something else — and drew the provenance line from it, which is in shot 02 and
in shot 04. **So an MSIX process reads an alternate data stream it did not write,
on a user's file, unvirtualised**, and that was the part of the question anybody
doubted: registry virtualisation covers a package's writes and not its reads, and
nobody had asked the same of the file system. What is still strictly unmeasured
is whether the stream's *name* matters, and nothing about NTFS suggests it could
— `Zone.Identifier` is a stream the shell agrees to care about, not a stream the
file system treats differently. It does not change the recommendation, which
rests on `Unblock-File` rather than on this.

### What the card's three lines looked like on Windows

Run 2026-08-29 against the release build of `d8b61d4`, in the dark theme —
`AppsUseLightTheme` is 0, the light desktop of the night before having been
switched back — with the corpus at `996dcca` generated on this machine. **The
first time any of these had been run on this platform.** macOS did all six on
2026-08-28 and Linux all five that apply on 2026-08-29.

**Items 1 and 2 are both absences here, and that is what makes them weak.**
`DESIGN.md` §5 gates the executable line to Unix, so no fixture on this platform
draws it and the checks have no positive control between them. The provenance
line is drawn in the same colour by the same code, so a downloaded container
supplied one:

    payload-setuid-external-attributes        0 exact /   0 near
    minimal                                   0       /   0
    payload-no-mode-recorded                  0       /   0
    payload-name-bidi-override                0       /   0
    quarterly-report-downloaded  (control)   93       / 440

rgb(255, 143, 0), which is `warn_fg_color` in the dark theme, counted exactly
and within 12 per channel over the whole captured window — the capture is the
application's window and nothing else, which is the region mistake Linux made
and had to correct. Zero against a control of 93 says the line is absent rather
than the counting broken.

**The first count was taken in the light palette and returned zero for the
control as well.** rgb(180, 70, 0) is what `warn_colour` gives in light mode and
this desktop is dark. A measurement that returns the expected answer for the
wrong reason is the whole argument for having a control at all.

**Item 3 passes on the card and failed two rows under it, which no item asked
anybody to look at.** The card reads `report\u{202E}fdp.exe` through
`slpc::display_name`, ending in `.exe`, exactly as macOS and Linux recorded. The
metadata tree below it read `reportfdp.exe`. `src/tree.rs` rendered a string
straight into its `TextEdit`, and egui gives U+202E zero advance width — so the
one field this application will not let anybody edit, `payload.file` being in
`is_protected`, was showing the spoof SPEC §3's escaping exists to prevent,
under a card that was not.

`CHANGELOG.md` says *names are shown with the Unicode characters that reorder
text escaped*. That was true of the card and false of the tree, and
`RELEASE.md`'s readiness review calls a sentence written before anybody looked
the class of error this project has caught most often. This is one, caught
inside the repository rather than in a listing.

**Fixed the same day**, in `displayed`: a protected string is escaped and an
editable one is not. Escaping a field somebody can type into writes the escape
back into their document the moment they touch it, which is the reasoning
`src/main.rs` already records where the Extract-to dialog prefills a filename.
Two tests, each broken deliberately — with the escape removed the protected test
fails, with it applied to everything the editable test fails — and then looked
at again in the window, where the `file` row reads `report\u{202E}fdp.exe` like
the card above it.

**The card carries no *Opens with* line for this payload**, as on both other
platforms. Asked directly rather than inferred from the window:

    report.exe: (the platform did not answer)
    report.pdf: Microsoft Edge
    notes.txt:  (the platform did not answer)

`.txt` answering nothing is not new and is not a defect here: `src/opens_with.rs`
already records that this machine's `.txt` `UserChoice` names a packaged
application that is no longer installed, and that the arm deliberately does not
fall back to the machine-wide `txtfile` rather than name something the platform
would not use.

**Item 4 passes, and the mark was read back rather than looked at.**
`quarterly-report-downloaded.pdf.slpc` went from 1090 bytes to 1095 with its
title edited, so a rewrite genuinely happened, and `Zone.Identifier` came
through it: 26 bytes, `[ZoneTransfer]`, `ZoneId=3`. David watched the card's
provenance line before and after in the same window without reopening.

**Item 5 passes, and it asks a different question here.** There is nothing to
ask the platform for: `%TEMP%` is inside the profile and inherits its access
list, so the check is that the payload is not somewhere else. Explorer's
Properties, Security tab, on the live directory while the window was up.

**Its other half is the privacy entry's own sentence, measured for the first
time.** `packaging/privacy-entry.html` says the folder is *removed when Slipcase
exits*, and that a kill leaves it until the operating system clears its
temporary directory. Both halves were on this machine at once:

    slipcase-gMqeXG   created 2026-08-29 11:54   report.pdf, 47 bytes
    slipcase-uPzXFR   created 2026-08-28 16:44   report.pdf, 187 bytes

The first belonged to the running window and was gone after it was closed with
its X. The second is what a kill leaves, from the screenshot sitting the day
before, a day old and still holding a payload. The sentence is now measured on
this platform rather than reasoned from `TempDir`. Neither is a probe directory:
that one is inside the Linux `#[cfg]` arm and never exists here.

### What an unrecognised stream is worth on this platform

`HANDOFF.md` asks whether the origin note macOS added — `slpc` 0.3.10's
`com.excelano.slipcase.origin` — should be written here too, and named three
things needing measurement. Two were taken on 2026-08-29 and the third is
below.

**The shell ignores a stream it does not recognise, which is the answer that was
wanted.** A `.ps1` carrying only `com.excelano.slipcase.origin` runs under
`-ExecutionPolicy RemoteSigned`; the same script carrying `ZoneId=3` is refused
as unsigned. A note gates nothing and cannot be mistaken for a mark, exactly as
`carries_a_mark` deliberately disregards it on macOS.

**Survival is identical to `Zone.Identifier` everywhere except one path, and
that path goes the wrong way.**

    Copy-Item, Move-Item, robocopy, xcopy   both survive
    Compress-Archive then Expand-Archive    both stripped
    Unblock-File                            zone removed, note survives

**The last row is the finding, and it argues against writing the note here.**
Unblocking is a person saying they have looked at a file and trust it. On macOS
`arrived_from_elsewhere` consults the note, so porting that design would leave
the card saying a container arrived from elsewhere after its owner deliberately
cleared the mark, with nothing in this application's interface to clear. The
note would survive the one erasure that is intentional and be stripped by the
ones that are accidents.

That is not the case macOS was solving. There the platform forced its own mark
over ours and the note recovered what the sandbox destroyed; nothing here takes
the information away except a person choosing to remove it.

**Two things are unmeasured and neither changes the recommendation.** Whether a
packaged install sees such a stream at all is the third question `HANDOFF.md`
asks, and it belongs to the next sitting that has the MSIX installed. And a
volume with no alternate streams — a FAT32 or exFAT stick — is where the zone
stream is lost outright, which is also where a note could not be written; this
machine has no second volume and no administrator to make one, so that is
recorded as a gap rather than inferred past. `Unblock-File` is the scriptable
form of Explorer's Unblock checkbox and was measured; the checkbox itself was
not, and they are assumed to be one operation.

### Not yet done by hand

- **`carries_a_mark` answered the wrong question**, and no longer does. Settled
  on 2026-08-26 rather than walked, because it was a defect rather than a
  walkthrough: the predicate asked whether the `Zone.Identifier` stream exists,
  and a stream that exists carrying no `ZoneId` line is a file the shell does
  not gate. Reproduced first by denying the stream write, then repaired against
  a measurement of what the shell stops for. `HANDOFF.md` says what it cost and
  `DESIGN.md` §8 carries the table. The hand item it pointed at — provenance,
  which had never run on this platform — was walked later the same day and has
  its own section above.

- ~~**Whether a packaged application sees an alternate stream it did not write.**~~
  **Answered on 2026-08-29 by the screenshot run, which is what *for free* meant.**
  The packaged application read a `Zone.Identifier` written by something else, on
  a file outside the package, and drew the provenance line from it — twice, and
  both are in the store screenshots. An MSIX process reads an alternate data
  stream it did not write, unvirtualised. *What taking the screenshots found* has
  it, including the one part left strictly unmeasured: whether the stream's name
  matters, which is a question about NTFS rather than about packaging.

Nothing else is waiting, including the two items 2026-08-28 added and closed:
the certification kit, run three times, and the screenshots, which turned out to
need a script rather than a person. Every item this section held on 2026-08-26
was run that day: provenance, the window, the stale `UserChoice`, a scaled
display, an upgrade, and a second account. Anything added below should say what
it needs
that a test cannot give it, the way those did.

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

### What looking for the light card found, and why there was none

`HANDOFF.md` left one item for this platform: the contrast repair is shared
code, every Windows walkthrough ran in dark mode and macOS had never looked, so
open a container in light mode and look at the card. macOS answered on
2026-08-28 and this was the last arm.

**It cannot be run here, and the reason is the finding.** Slipcase does not
follow the desktop theme on Linux. It is dark on a light desktop, it was dark
after the appearance setting was moved to `prefer-light`, and no desktop setting
reaches it.

Run 2026-08-28 on GNOME 48.7, Wayland, against the release build. One container
carrying both coloured lines at once — the `accept/encrypted-payload` case from
the corpus, copied out and given `user.xdg.origin.url`, so the card says
*Cannot be opened here: the member is encrypted (SPEC 2.5)* and *This container
arrived from elsewhere, and the payload will carry that.* together.

| | Portal `color-scheme` | Titlebar | Card |
| --- | --- | --- | --- |
| GNOME set to Light | 0, no preference | dark | dark |
| `color-scheme` forced to `prefer-light` | 2, prefer light | **light** | **dark** |

The second row is the one that settles it. The window's own frame followed the
setting and the card did not, in the same screenshot — so this is not the
desktop failing to say what it wants, and it is not GNOME's Light being spelled
`default` rather than `prefer-light`. The setting arrives and the application
never asks.

**Traced rather than inferred.** `winit-0.30.13`, `src/platform_impl/linux/mod.rs`
line 909: `system_theme()` returns `None` on Linux, unconditionally, with no
body but that word. The Windows arm calls `should_use_dark_mode()` and the macOS
arm reads `NSApplication`'s `effectiveAppearance`, which is why those two
platforms could look at the light card by switching the system and this one
cannot. `egui-winit` puts that `Option` straight into `egui_input.system_theme`,
and egui falls back to dark when it is `None`. Nothing else in the tree reads a
theme *for the application*: there is no `dark-light` or `ashpd` dependency, and
the `zbus` in `Cargo.lock` arrives under `accesskit_unix` for accessibility.
`rfd` on this target pulls no D-Bus crate at all.

**The titlebar's answer comes from somewhere else in the same process**, which
is what makes the second row of the table possible. `sctk-adwaita-0.10.1`,
`src/config.rs`: `prefer_dark()` spawns `dbus-send` with a 100ms reply timeout,
asks the portal for `org.freedesktop.appearance color-scheme`, and tests whether
the output ends in `uint32 1`. A subprocess and a string match, inside the
window that draws the card. So the portal is already consulted here; it is
consulted for the frame and not for the contents.

**So every Linux user sees the dark card, and the light one is unreachable
here.** That is a larger statement than the item asked for and it is why this
section exists rather than a tick. What the repair bought on this platform is
nothing, because the theme it repairs is one no Linux desktop can select — and
the test in `src/main.rs` holding both themes to 4.5:1 goes on passing, because
it constructs `Visuals::light()` directly and never asks how a window would get
there.

**Fixed the same day, and then both questions were answered.**
`src/system_theme.rs` reads the portal and follows it, `DESIGN.md` §3 carries
the reasoning, and the run below is what the repair produced.

| Setting | Portal | Card |
| --- | --- | --- |
| `prefer-dark` | 1 | dark |
| `prefer-light` | 2 | **light** |
| `default` — what GNOME's Light actually sets | 0 | **light** |

Then the setting was changed *while the window was open*, from `prefer-dark` to
`prefer-light`, with no relaunch: the card went light. That is the case the
watch thread exists for, and it is what the other two platforms get from
`ThemeChanged` without anybody being involved.

**And the light card reads here, which is what the item originally asked.**
Sampled off the screen at `color-scheme=default`, the setting a GNOME user
actually has:

| | On screen | Recorded |
| --- | --- | --- |
| Card fill | rgb(248, 248, 248) | grey 248 |
| Error line | rgb(180, 0, 0) — 6.72:1 | 6.72:1 |
| Warning line | rgb(180, 70, 0) — 5.18:1 | 5.18:1 |

The macOS figures to the digit, on a second platform and a different display.
So the contrast repair was not dead code on this arm after all — it was correct
and unreachable, and it is now both.

### What the card's three lines looked like here

Run 2026-08-29 against the release build, GNOME 48.7 on Wayland, in the dark
theme — the light one having been looked at and measured the night before, under
*What looking for the light card found*.

**Item 1 passes, and in the place the item specifies.**
`payload-setuid-external-attributes.slpc`, whose payload records 04755, draws
*The payload is an executable file; the extracted copy will not be executable.*
below the size and the *Opens with Document Viewer* line and above the three
buttons. Sampled off the screen rather than judged: the line is rgb(255, 143, 0)
at 7.53:1 against the card, which is `warn_fg_color` and the recorded dark
figure to the digit.

**Item 2 passes, both halves, and this is the first time anywhere that its
second half has been runnable from the corpus.**
`accept/payload-no-mode-recorded` went into `excelano/slipcase` this morning for
exactly this; before it, the item named a fixture recording 0644 and tested
`minimal.slpc` twice. Both containers draw no executable line, and that was
measured rather than looked at, because "I do not see it" is a weak assertion
about a line that is one row of text: **zero warning-coloured pixels inside the
card region**, against 1193 for item 1. The first count was taken over the whole
screen and found hundreds in both, which was the terminal's own amber text and
not the card — the region matters and the first measurement was wrong.

**Item 3 passes.** `payload-name-bidi-override.slpc` reads
`report\u{202E}fdp.exe` on the card, ending in `.exe`, so the escape shows the
character that was always there. As on macOS there is **no *Opens with* line**,
which is correct and for the same reason: nothing here is registered for `.exe`
and `DESIGN.md` §3 says nothing rather than guessing.

**Item 5's first half passes.** The probe directory `opens_with` makes while the
card works out what would open the payload is `drwx------`, owned by the user.
Catching it took a poll loop: it exists for a fraction of a second and the
listing raced its own removal — `stat` reported *No such file or directory* on
the same directory it had just described, which is the disappearance the privacy
entry claims and nothing had watched happen.

**Item 4 passes, and the half a test cannot reach passes with it.**
`tests/handover.rs` already drives `Opened::save` and asserts the mark survives,
so what a hand adds is the Save *button* reaching that path and the card still
reading afterwards. Both: the fixture went from 1090 bytes to 1093 and its
modification time moved, so a rewrite genuinely happened, and
`user.xdg.origin.url` came through it carrying the same URL. David looked at the
window and the orange *This container arrived from elsewhere, and the payload
will carry that.* line still read, in the same window, without reopening. A card
that goes blank there would leave the file gated and the person misinformed,
which is why the item wants eyes.

**Item 5's second half passes.** Open was pressed and the handover directory
looked at while the window was still up, which is the only time it exists:

    drwx------ 2 anderix anderix 60 /tmp/slipcase-tcbcWF
    -rw-rw-r-- 1 anderix anderix 584 quarterly-report.pdf

0700 as asked rather than the 0775 the umask would have given, and gone after
the application quit. **The payload inside it is 0664 and that is correct, not a
gap left over**: SPEC §3 requires the permissions a newly created file would
ordinarily receive and forbids applying the archive's bits, so the mode is the
umask's and the privacy comes from the directory. It is also not executable,
which is the promise item 1's line makes.

**Item 6 is macOS only, so the Linux side of this section is complete.**

### Not yet done by hand

- **A machine that has never had it.** Item 12 was run here, where every
  dependency was already satisfied, so `apt` pulled nothing and the `Depends`
  list was never actually exercised. A minimal install would exercise it.
- **A second desktop.** Everything above is GNOME on Wayland with Adwaita. The
  icon defect was a property of which theme carried which name, so KDE or XFCE
  could reach a different answer by the same mechanism.

### What lintian says, and what was decided

Run for the first time by `linux.yml`, which produced two tags. Both were
decisions rather than defects, which is why the step was a report and not a
gate until they were taken. **Both are now answered and the step gates on
`error,warning`.**

`E: no-changelog usr/share/doc/slipcase-desktop/changelog.gz (native package)`.
Debian policy wants a changelog in every binary package and this one shipped
none. It is an error rather than a warning because somebody installing from an
apt repository has no other way to find out what changed between two versions,
and DESIGN.md §8 sends this package through one.

**Decided: hand-written, and `build-deb.sh` refuses to build a package whose
changelog names a different version.** Generating it from `git log` was the
alternative and it was rejected on three counts. The script would need a git
checkout, which `--binary` exists so that a build does not need. There are no
release tags, so with nothing to divide the history into versions every release
would re-list every commit. And a commit subject here is written for whoever
maintains this — *Address a directory so a payload named CON is a file* — rather
than for whoever is deciding whether to upgrade. `git log` stays the record of
why the code is the way it is; the changelog is the record of what changed for a
person who installed it, and they are not the same document.

The one thing generation would have bought is that it cannot go stale. The
version check buys the same thing: bump `Cargo.toml` without touching the
changelog and the build stops with *the changelog's newest entry is 0.1.0, and
Cargo.toml says 0.2.0*. Watched to fail on 2026-08-26, and watched to pass again
with the two in agreement.

`W: no-manual-page [usr/bin/slipcase-desktop]`. There was no
`slipcase-desktop(1)`.

**Decided: write it, rather than override the tag.** The argument for an
override was that this is a windowed application normally started by
double-clicking a container, so the tag is a convention about `/usr/bin` rather
than a gap anybody would notice. What settled it the other way is that
`src/main.rs:181` reads `args_os().nth(1)` and there is no `--help` anywhere, so
the page is the only place that one argument is written down. An override would
have cost about what the page cost and left the argument undocumented. The
page is `packaging/debian/slipcase-desktop.1.in`, templated on `@VERSION@` the
way `control.in` is so the header cannot drift from the package carrying it, and
it points at `slipcase(1)` for the command-line interface that this is not.

Both documents are gzipped with `-9n`, so neither carries a name or a timestamp
of its own and two builds of the same source produce the same bytes.

**Checked that the gate bites**, on 2026-08-26 against lintian 2.122.0. A
package built with both documents left out reproduces exactly the two tags above
and exits 2; the package as it ships is silent and exits 0.

Two tags sit below the gate, untriaged, which is what `info` means here rather
than a claim that they do not matter.

- `I: binary-has-unneeded-section .comment`. `strip --strip-unneeded` does not
  remove `.comment`, and `-R .comment` would. It costs a few hundred bytes.
- `I: hardening-no-fortify-functions`. `FORTIFY_SOURCE` is a glibc facility a
  Rust binary does not call into, so this is lintian asking a C question of
  something that is not C.

Neither has been decided, and the step gates on `error,warning` rather than
`info` so that neither is decided by accident.

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
~/Documents: Operation not permitted (os error 1)* at path
`~/Documents/.tmpos50lA`, the home directory abridged here and spelled out in
full by the application. The grant covers the file a person chose
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
untested, and testing it needs code rather than a click. Both were then fixed, in that
order and in separate commits: `DESIGN.md` §5 was reopened first on the
extraction failure, which blocked the channel, and then on this, which
misstates one line on the card. The card now disregards a mark whose agent is
this application, which is a change of stance about a value the module
otherwise treats as opaque and is why it was taken as its own decision rather
than settled beside a bug fix.

One incidental: the only sandbox violation the log attributes to this
application is `hid-control`, winit asking WindowServer for raw input. It is
harmless and it is not ours.

Two things about running this at all. `log` is a zsh builtin, so a capture
needs `/usr/bin/log` spelled out. And most of what the sandbox refuses here is
never reported as a violation — neither the save nor the quarantine write
produced a `deny(1)` line — so the application's own error text was the record
and the log served mainly to rule things out.

### What the provenance sitting found

Run 2026-08-25 against the unsigned bundle, then against the sandboxed one.
`src/provenance.rs` had compiled on this platform since it was written and had
never executed here. It does now, and the item it answers had been carrying a
suspicion since Linux: that quarantine bites on executables and says nothing
about a document, in which case carrying it matters for the dangerous case and
not the ordinary one. That is right, and the sitting went further than
confirming it.

**A quarantined document is not gated at all.** `a-pdf.slpc`, downloaded through
Safari and carrying `0083;…;Safari;…`, shows the card's line about arriving from
elsewhere; pressing Open hands the payload to Preview, which displays it with no
prompt of any kind. So the mark bought nothing visible there, and §5's decision
to report rather than gate is the only reason a person is told anything.

**A quarantined disk image is not blocked either, but it is examined.** Pressing
Open on a container carrying a `.dmg` mounts it silently. The log shows
`DiskImageMounter` running `QuarantineFileHandler applyMountPointsWithBSDName:`
before attaching, so macOS consults the mark and propagates it to the mount
rather than refusing.

**The application inside it is refused, and that is where carrying earns its
keep.** The first fixture was an empty disk image and proved nothing — there was
nothing on it for macOS to gate, which is a lesson about fixtures as much as
about quarantine. `an-application.slpc` carries a `.dmg` containing a
three-line unsigned `Probe.app`. Downloaded through Safari, opened here, mounted:
double-clicking the application gives *Apple could not verify "Probe" is free of
malware that may harm your Mac or compromise your privacy.*

**And the counterfactual, because a result that would have happened anyway
proves nothing.** The same extracted image, copied byte for byte, with
`com.apple.quarantine` removed and nothing else changed, mounts beside the
first one and the same application runs: *The payload ran.* One extended
attribute is the whole difference between an unsigned application from the
internet executing and being stopped. `provenance::carry` is not hygiene. It is
the only thing between a container and the laundering `DESIGN.md` §5 describes.

**Both mark shapes gate, by different mechanisms, and that validates the change
made earlier the same day.** §5 now accepts a mark the platform wrote instead of
the container's own, which would be a hole if the platform's mark were weaker.
It is not. Under the sandbox the extracted image carries
`0086;…;slipcase-desktop;` rather than Safari's value, and the application on it
is still refused — reported by Finder as *The application "Probe" can't be
opened*, which reads like a launch failure and is not one. The kernel log says
what happened:

    (Quarantine) exec of …/Probe.app/Contents/MacOS/probe denied since it was
    quarantined by slipcase-desktop; and created without user consent,
    qtn-flags was 0x00000086

Safari's `0083` goes through Gatekeeper's user-facing assessment, which explains
itself and offers a way past. This application's `0086` is denied in the kernel
with no such affordance, and the flag encodes *created without user consent*.
Different wording, different path, same verdict, and if anything the stricter of
the two. macOS also translocates the application to a randomised read-only path
before refusing, which is its normal treatment for a quarantined application on
a mounted image.

**Measured afterwards: the mode claim is true and the reason for the refusal was
not.** `a-command.slpc` stores `run-me.command` as `rwxr-xr-x`; extracting it
puts `rw-r--r--` on disk, so `File::create` drops the executable bit exactly as
the code said. What was wrong was the prediction that `open` would then fail on
that bit before quarantine was consulted. It never got that far. The temp copy
carries `com.apple.quarantine: 0086;…;slipcase-desktop;`, macOS consulted that
first, and refused with *"run-me.command" is damaged and can't be opened. You
should move it to the Bin.*, naming this application as the file's creator.

**That container carried no quarantine of its own.** It was made on this machine
and never downloaded. The mark came entirely from the sandbox marking what a
sandboxed process writes, which means a Store build refuses to open a script
payload out of a container a person made themselves.

**Scope it before calling it a defect.** Three payloads extracted under the
sandbox carry the identical `0082;…;slipcase-desktop;` mark: a PDF, which
Preview opened without a word; a text file, which TextEdit opened without a
word; and this script, which was refused. The mark is on everything; macOS
consults it only when something is about to execute. That is what it does to
every sandboxed application's output and to anything a browser downloads, and on
the same day's evidence it is the behaviour to want — stripping this attribute
is the difference between an unsigned application from the internet running and
being stopped. A container is an archive, and macOS declining to run code
straight out of an archive is the protection working.

The cost is bounded and lands on one person: somebody who deliberately packages
a script and wants to run it from the card. They can extract it and run it at
the price of `chmod +x` and clearing the attribute, which is what Safari charges
for a downloaded script today.

**The unsandboxed build refuses it too, for the other reason.** Same container,
same button, no signature and so no sandbox and no mark: *The file
"run-me.command" could not be executed because you do not have appropriate
access privileges.* That is the 0644 the extraction wrote, which is what the
prediction said would stop it and which the sandboxed run never reached. So an
executable payload does not run from the Open button on **either** build. The
sandbox does not create that; it only changes which refusal a person sees, and
which of the two messages is honest — the unsandboxed one describes what is
actually wrong, and *damaged, move it to the Bin* does not.

**One thing here is ours.** The card knows before anybody presses anything: a
member's mode is in the container's central directory, so the application can
say that a payload stored executable will not be executable once extracted.
That is a fact read out of the container rather than a guess from its name, so
`DESIGN.md` §3's rule against a filename table does not bite. Not fixed here —
it is a change to what the card says and belongs with David — but it is no
longer waiting on a measurement, because the pair above shows the behaviour is
not sandbox-specific.

**A correction to this file.** The item used to say that what macOS gates
without needing an executable bit is a disk image or an installer package. That
is half right in a way worth writing down: the image itself is not gated, it
mounts. What is gated is what you then launch from it, and the mark reaches
there by propagation rather than by being on the thing that gets refused.

### What the second-volume sitting found

Run 2026-08-25, against the plain build, after Linux read the arm from another
machine and asked whether `replaceItemAtURL:` had ever crossed a volume. It had
not, and it does not.

**Save was broken for every container that is not on the boot volume.**
`tempfile::TempDir` answers `TMPDIR`, which is on the boot volume, so the
rewrite always waited there; `replaceItemAtURL:` then refused to move it onto a
container anywhere else, with `NSCocoaErrorDomain` 512 over `NSPOSIXErrorDomain`
18, `EXDEV`. Measured against mounted images formatted APFS, HFS+, FAT32 and
exFAT — all four refuse, so it is the crossing and not the filesystem. The
original was untouched every time and the error did reach the person, so nothing
was corrupted and nothing was silent; it simply never worked. An external drive,
a mounted image, or a share, and Save could not be used at all.

**The fix is an API Apple provides for exactly this**, and it is now
`Scratch::on_the_volume_holding`. `URLForDirectory:` asked for
`NSItemReplacementDirectory` with `appropriateForURL:` makes a fresh directory
on the volume that URL is on. For a container on a second volume it returns one
there — `/Volumes/…/.TemporaryItems/folders.501/TemporaryItems/NSIRD_…` — and
the replacement then succeeds on all four filesystems. For a container on the
boot volume it returns one under the same per-user temporary area `TempDir` was
already using, so the reason this module exists is untouched: the rewrite still
waits nowhere near the container, and the sandbox still sees no sibling being
created. Nothing was worked around and no unsafe was added; the binding is
another safe function on `NSFileManager`.

**Three things cost time and are worth the next person's while.**

Cocoa's message hides the fact. `localizedDescription` says only *The file
“x.slpc” couldn’t be saved in the folder “y”*, which names nothing that could be
acted on; the `EXDEV` beneath it took a probe through `underlyingErrors` to
reach. `Landing::replace_original` now appends whatever is under the sentence to
the message it reports, so the next reader of this failure gets the errno for
free.

The first version of the regression test passed against the defect it was
written for. It read the mount point out of `hdiutil attach`'s output with
`.find_map(…nth(2))`, and that output's first line names a device and leaves the
mount point empty, so the mount point parsed as the empty string, every
container was written into the working directory, and every run replaced a file
on the boot volume with a file on the boot volume. It crossed nothing and passed
green — including the deliberate break that was supposed to prove it bites. The
test now asserts that the container and the working directory are on different
devices, and that the rewrite is not waiting in the boot volume's temporary
directory, before it does anything at all. Both assertions were confirmed to
fire.

Mounting volumes from a test leaks them when the test is wrong. The detach guard
was given the same misparsed empty string, so `hdiutil detach ""` did nothing
and eleven images stayed mounted across the runs it took to notice. It detaches
by the `/dev/diskN` device now, which is the field that cannot be silently
empty.

**What was measured about what survives the replacement**, on a second volume,
matching what the same-volume run found in the sandbox sitting: the original's
mode is kept — 0600 on APFS, and 0700 on FAT32 and exFAT because those have no
POSIX permissions and the mount forces the execute bit — and `com.apple.macl`
and the last-used date survive while `com.apple.quarantine` does not.

### What the sandboxed handover found

**Item 6 passes.** Run 2026-08-28 against `dist-sandboxed/Slipcase.app`: today's
release binary, signed with an Apple Development certificate, its signature
carrying `com.apple.security.app-sandbox`, and the only bundle Launch Services
held for `com.excelano.slipcase` — the unsigned August 25 one was unregistered
first, because a stale bundle answering the door would have made the whole
measurement worthless.

Opening a container and pressing Open brings Preview forward showing the PDF.
The card's *Opens with* line reads Preview, which is the association section's
item 6 answered in the same glance. Launch Services does grant the launched
application a scope for the URL it was launched with, and a 0700 directory
inside this application's container is no obstacle to it.

That was the question worth being afraid of. It was raised because
`tests/handover.rs` proves the Linux case by reading the payload back from a
separate process running as the same user, and under the sandbox the handler is
a different application with a container of its own — so the Linux proof did not
transfer and nothing on that platform could take the measurement. ~~**Open works
on the Store build.**~~ No `DESIGN.md` §5 decision is owed.

**Corrected 2026-08-29: that last sentence was a promotion and not a
measurement.** What ran was the bundle named four paragraphs above — signed with
an *Apple Development* certificate — and a Store build is a different artefact
signed with a different certificate. The measurement is sound and the conclusion
about the sandbox holds; the sentence claimed a stronger thing than the run
supported, in the one place a later reader would take it for the run itself.
`RELEASE.md` had inherited the same sentence three lines from a paragraph
correctly saying every sandbox measurement to date was development-signed, and
the two contradicted each other. **A Store build could not have been launched
here at all**, which the section below measures.

Four things measured around it, while the process was still alive, because the
handover directory goes when it does:

- **The payload lands inside the container**, at
  `~/Library/Containers/com.excelano.slipcase-desktop/Data/tmp/slipcase-XXXXXX`,
  and nothing was written to `/tmp` or to the per-user temporary area outside
  it. This is also the only trustworthy runtime proof that the process really
  was sandboxed: `ps eww` cannot answer it. macOS refuses to expose another
  process's environment, so it echoes the *caller's* `HOME` and `TMPDIR` back
  and reads exactly like a process that was never sandboxed. Checked against
  Safari, which is certainly sandboxed and returns nothing at all for the same
  query. Anybody reaching for `ps eww` here will get a false negative.
- **The directory is 0700 and the payload inside it 0644**, which is what
  2026-08-27 asked for and what item 5 wanted looked at on this platform.
- **The extraction is byte for byte.** The payload's SHA-256 matches the file
  that went into the container.
- **The platform marks what a sandboxed process writes, and the disregard
  works.** The container carries no `com.apple.quarantine` — it was made on this
  machine — and the extracted payload carries
  `0086;6a91c46e;slipcase-desktop;` anyway, written by the platform rather than
  by this application. The agent field is the executable's own filename, not the
  display name `Slipcase`, and `slpc::provenance::this_process_wrote` compares
  against `current_exe().file_name()` rather than a string spelled out anywhere
  — so it matches, and a binary renamed keeps agreeing with itself. That
  confirms on the extraction path what the library's comment measured on
  2026-08-25. Preview opened the marked payload without a prompt, a flags field
  of `0086` being a mark the shell records rather than one it gates on.

**What this run does not settle is item 4**, which is the save path rather than
the extraction path, and is the one place `slpc` 0.3.7's provenance fix meets
`src/staging.rs`. It stays below.

### What a downloaded container did under the sandbox

Run 2026-08-28, immediately after the above and against the same bundle, because
`packaging/macos/README.md` claimed a container that arrived from elsewhere could
be **neither extracted nor opened** under a sandbox and named that as the reason
`DESIGN.md` §5 had to be reopened before the Mac App Store could be taken. A copy
of the walkthrough container was marked the way a download leaves one —
`xattr -w com.apple.quarantine '0083;68ae0000;Safari;9C1A2B3C-…'` — opened in the
signed bundle, and Open pressed.

**It extracts and it opens.** The card carried *This container arrived from
elsewhere, and the payload will carry that.* in the warning colour, Open put the
PDF in front of Preview, and nothing refused anything. That README paragraph
predates `Mark::AlreadyMarked` and is stale; the note there is amended rather
than deleted.

**What the payload carries is not what the container carried, and that is the
finding.** Measured on the extracted file:

    container:  0083;68ae0000;Safari;9C1A2B3C-4D5E-6F70-8192-A3B4C5D6E7F8
    payload:    0086;6a91cfe4;slipcase-desktop;

The platform marks whatever a sandboxed process writes and then refuses to have
that mark replaced, so `slpc::provenance::carry` returns `AlreadyMarked` rather
than `Carried` and the source's value is lost. The gate the mark exists for is in
place. The origin it recorded is not.

**Set beside the unmarked run half an hour earlier, the two are the same shape:**

    from an unmarked container:   0086;6a91c5c5;slipcase-desktop;
    from a downloaded container:  0086;6a91cfe4;slipcase-desktop;

Only the timestamp differs. So under the sandbox a payload extracted from a
container that arrived from elsewhere and one extracted from a container made on
this machine are indistinguishable by their marks. Unsandboxed — Linux, Windows,
and a Developer ID build — `carry` writes the source's own value and they are
not. **This is a difference between the Store build and every other build**, and
it is a property of the platform rather than a defect in either repository.

**The gate is not weakened, and that was measured before today.** *What the
provenance sitting found* above, 2026-08-25, put an extracted script carrying
`…;slipcase-desktop;` in front of the system and it was refused outright —
*damaged and can't be opened* — denied in the kernel, naming this application,
with no way past, where Safari's `0083` goes through Gatekeeper's assessment and
is offered an override. The counterfactual was run in the same sitting: the same
extracted disk image with `com.apple.quarantine` stripped and nothing else
changed mounts and runs an unsigned application. **So the sandbox mark gates at
least as hard as the one it replaced, and harder for anything that executes.**
Nothing here is laundered.

One discrepancy between that sitting and this one, left as a discrepancy rather
than edited away. It records the three extracted payloads as carrying
`0082;…;slipcase-desktop;`, and both extractions measured here on 2026-08-28
read `0086`, as did the file `Probe.app` created. Nothing in either run's
conclusion turns on it — both are the sandbox marking its own process's output,
and the refusal that sitting measured is the behaviour either way — but one of
the two transcriptions is wrong and it should be somebody's next reader who
finds out which, rather than nobody.

**It still reaches the store listing, which is a public claim.**
`packaging/store-listing.md` says the payload *carries that marking onward, so
whatever opens it next raises the same warning the container would have*. The
first half holds. The second is not false so much as **not the same warning**:
the next application is warned by this application's mark rather than the
container's, and where the container would have produced an explicable
Gatekeeper prompt the payload produces a blunter kernel refusal. `RELEASE.md`'s
readiness review names this class — a sentence written before anybody looked —
and this is one it has to catch.

Preview opened both marked payloads with no Gatekeeper prompt, which is expected
for a PDF and says nothing either way about an executable payload. That is not
tested here.

**Asked afterwards whether that is a defect rather than a constraint, and
measured rather than argued.** The suspicion was reasonable: this application
sets the mark with a raw `setxattr`, and Apple's supported route is the Launch
Services property dictionary — `NSURL`'s `quarantinePropertiesKey`, carrying
`LSQuarantineAgentName`, `LSQuarantineOriginURL` and `LSQuarantineDataURL`.
Sandboxed downloaders mark their downloads, so the sandbox plainly permits some
path to it, and the library might simply have been using the wrong one.

A throwaway bundle — `Probe.app`, signed with this application's own
entitlements so the sandbox is the same one — created a file, read the mark the
platform gave it, and then tried both routes with a foreign value:

    1. as created by a sandboxed process : 0086;6a91d108;probe;
    2. raw setxattr of a foreign value   : rc=-1 errno=1 (Operation not permitted)
       value now                         : 0086;6a91d108;probe;
    3. setResourceValue(quarantineProperties): OK
       value now                         : 0087;6a91d108;probe;E31A5F7D-…
    4. dictionary reads back            : LSQuarantineAgentName = probe
                                          LSQuarantineEventIdentifier = E31A5F7D-…
                                          LSQuarantineIsOwnedByCurrentUser = 1

**Both routes refuse, and the second refuses more quietly than the first.** The
raw write fails outright with `EPERM`, which is what `provenance::carry` catches
and what `AlreadyMarked` exists for. The supported route *succeeds* — the flags
move from `0086` to `0087` and an event identifier is added — and then macOS
substitutes its own agent: `Safari` was asked for and `probe` was written. The
supplied origin and data URLs are absent from the readback, and the event
identifier never reached `~/Library/Preferences/com.apple.LaunchServices.QuarantineEventsV2`,
whose modification time still predates the run, so the origin was not recorded
anywhere rather than merely being hidden from the readback.

**So a sandboxed process cannot attribute a file's origin to anyone but
itself.** That is the platform's guarantee rather than this library's shortfall,
`Mark::AlreadyMarked` is the correct and only available behaviour, and there is
nothing to file against `excelano/slpc-rust`. The consequence stands where it
was: the payload is gated, the origin is lost, and it is the *wording* of the
Mac App Store listing that has to change, because the behaviour cannot.

### What saving a downloaded container under the sandbox found

**Item 4. The save succeeds, the file stays gated, and the card stops saying
where the container came from.** Run 2026-08-28 against the signed sandboxed
bundle, predicted in full before the button was pressed, and the prediction was
right — which is worth recording because it means the mechanism below is
understood rather than guessed at.

    before:  0083;68ae0000;Safari;9C1A2B3C-4D5E-6F70-8192-A3B4C5D6E7F8
    after:   0083;6a91d265;slipcase-desktop;

The edit landed — `title` reads what was typed, and the other keys came back in
the order they were written. `com.apple.macl` and the last-used date survived.
The flags stayed `0083`, so **the container is still gated exactly as hard as it
was**; what changed is the agent, the timestamp, and the event identifier, which
is now absent.

`arrived_from_elsewhere` reads that agent, recognises it as this executable's own
filename, and answers false. So the card's *This container arrived from
elsewhere, and the payload will carry that.* was there before the save and gone
after it, on a container whose history did not change. `RELEASE.md` named this
failure in advance and in these words: *a save that quietly turns arrived from
elsewhere into nothing is the failure, even though the file stays gated.*

**The mechanism, and why no test could have caught it.** `src/staging.rs` calls
`carry(original, staged)` to put the container's mark on the rewrite before it
becomes the container. The staged file was written by this sandboxed process, so
the platform had already marked it, so the write is refused with `EPERM` — the
probe above measured that directly — and the refusal is deliberately non-fatal,
so the save proceeds. `replaceItemAtURL:` then swaps them.

`tests/handover.rs:246`, `saving_an_edit_keeps_where_the_container_came_from`,
asserts the opposite and **passes**, on this machine, today, through this same
macOS arm. It runs unsandboxed, where `carry` succeeds and the mark is preserved
intact. The test is not wrong and the code it guards is not broken: no test in
either repository can enter the App Sandbox, and the sandbox is the only place
this happens. **A green suite and a Store build that launders the card are
consistent with each other**, which is the whole argument for this file.

**Resolved the same day, and not by any of the three ways out first proposed.**
Two of them made the card remember within a session and the third warned before
the save; all three accepted the loss and argued about how to narrate it. The
question that unstuck it was whether information could be *added* to the mark
rather than substituted into it.

It can — just not to the platform's mark. Measured inside a signed sandboxed
bundle: the refusal is specific to `com.apple.quarantine`, an attribute of this
project's own goes on without complaint, and it survives
`-[NSFileManager replaceItemAtURL:]`, which is the operation that destroys the
attribution in the first place. So the source's value is kept beside the
platform's under `com.excelano.slipcase.origin`, and the fact is on the file
rather than in a window.

That fixes the case none of the three reached. The card is right after a save
**and after a close and reopen**, because `Opened::open` re-deriving provenance
from disk now gets the right answer — so there is no sticky flag, no session
state, and nothing for the interface to remember. The work is in
`excelano/slpc-rust` as `Mark::Recorded`, which is where `CLAUDE.md` says
behaviour the library lacks belongs, and **this repository needs no code change
at all**: `arrived_from_elsewhere` is already what the card asks and `carry` is
already what the save calls. It needs the dependency bumped when 0.3.10 ships.

Proven end to end on 2026-08-28 before the library change was committed, through
a path override that was reverted afterwards: a downloaded container edited and
saved kept its card line, kept it across a quit and reopen, and the file carried
`0083;…;slipcase-desktop;` as the gate with
`0083;68ae0000;Safari;…` beside it as the record.

Not measured, and worth knowing before the decision is taken: whether a *second*
save behaves differently now that the container's mark is already this
application's, and what a person sees if they close and reopen the container
afterwards — the line will stay gone, because the file genuinely no longer
records Safari.

### What the card's three lines looked like on macOS

Items 1, 2 and 3, run 2026-08-28 against the signed universal bundle. They had
been written, unit-tested, and rendered in front of nobody on any platform.

**Item 1 passes, exactly as specified.** `payload-setuid-external-attributes.slpc`
records 04755 and the card carries *The payload is an executable file; the
extracted copy will not be executable.* in the warning colour, below the size and
the *Opens with* line and above the buttons — which is where the item says it
should be, and placement was worth checking rather than presence alone.

**Item 2's first half passes.** `minimal.slpc` records 0644 and there is no line.

**Item 2's second half could not be run as written, and that is the finding.**
The item asks for the silence on a container that records *no* mode, because
that silence is the whole reason `payload_mode` reads the external attributes
rather than asking the ZIP crate, which would invent `0o664` and answer
confidently. It names `name-cp437-bit11-clear.slpc` — and `unzip -Z` says that
fixture is `-rw-r--r-- 3.0 unx`, a Unix mode of 0644. **It tests the same thing
`minimal.slpc` tests.** Checked across the whole corpus: every `accept` fixture
is written by a Unix tool and records a mode, so no fixture there exercises this
at all, and the item would have been ticked twice over for the case it was
written for.

The item's other option — *any container a Windows tool wrote* — is the real
one, so one was made: both members written with creator system MS-DOS and the
DOS archive bit for external attributes, which `unzip -Z` reports as
`-rw-a-- 2.0 fat`. Under MS-DOS the high sixteen bits are not a Unix mode and
there is nothing to read. The card shows `notes.txt`, 48 bytes, *Opens with
TextEdit*, `conformant`, and **no executable line**. That is the case the item
exists for and it passes; it is kept at
`~/Documents/slipcase-walkthrough/card-items/no-mode-recorded.slpc` and is four
lines of `zipfile` to rebuild.

Worth raising with `excelano/slipcase` rather than fixed here: the corpus has no
container that records no mode, and this application is not the only reader that
would want one.

**Item 3 passes.** `payload-name-bidi-override.slpc` carries U+202E in its
payload name — `report\256\200\256fdp.exe` in the raw bytes — and the card reads
`report\u{202E}fdp.exe`, ending in `.exe`. The escape shows the character that
was always there.

Two things about that card beyond the item. There is **no *Opens with* line**,
which is correct: macOS has nothing registered for `.exe`, and `DESIGN.md` §3
says nothing rather than guessing. And Open is enabled and holds the focus ring,
which is the item's *the Open button beside it should still work*. Pressing it
was not done here — there is no handler for `.exe` on this machine, so what it
would produce is a refusal from the platform rather than anything about this
application. What the button would do with that name **is** covered: the
conformance runner extracts every payload including this one, and all 87 cases
agree, so the escaping is a display transformation and does not reach the file
that gets written.

### What Apple silicon answered, and what it still cannot

**A machine opens a container on arm64 now, on every push.** Added 2026-08-28
to `.github/workflows/apple-silicon.yml`, which had been building and testing
that code natively all along without ever starting it. `open` reaches Launch
Services and delivers the same Apple Event Finder does, so this is
`src/opened_document.rs` — the only `unsafe` in this crate — running on the
architecture nobody had run it on.

First green run reported:

    on-screen windows: 9, from 6 applications
    owners: Control Center, Dock, Finder, Slipcase, Spotlight, Window Server
      Slipcase window: 900 x 668 at layer 0
    the container's folder was remembered: …/conformance/cases/accept

900 by 668 is `DESIGN.md` §6's declared width, so the number is independently
right rather than merely non-zero. The photograph the job uploads shows
`minimal.slpc` loaded and named, the verdict `conformant`, the card reading
*Opens with Preview* — which is `opens_with`'s `objc2` path answering on arm64 —
the tree drawn, Open carrying the focus ring, and **no dialog**.

**Two assertions, and the second exists because the first is not enough.** The
window check asks the window server rather than looking at a screenshot, for the
reason *What the first run found* records: `screencapture` here returned the
desktop with every window omitted and reported no error, so a pixel assertion
would go green against a build that drew nothing. But the refusal this whole
item guards against — *Slipcase cannot open files in the "Slipcase container"
format* — was written up as opening an **empty window** before anybody read the
dialog. So the job also checks that the container's folder was remembered, which
happens only when a document was actually opened.

Both were broken deliberately. Removing the folder check leaves a launch with no
container passing; that was run, and the window check does pass on it while the
document check fails, which is the whole argument for having both.

**A measurement that cost some confusion.** The folder is written to
`$HOME/.local/state/slipcase-desktop/last-folder`, and under the App Sandbox it
is not: the sandbox redirects `HOME` into the container, so a signed build
writes it to
`~/Library/Containers/com.excelano.slipcase-desktop/Data/.local/state/…`
instead. The signed bundle here appeared to record nothing at all until that was
found. `packaging/privacy-entry.html` is unaffected and was checked rather than
assumed — it says *inside the per-application state directory your operating
system provides*, which is true of both.

**What a runner cannot be**, and what still wants a real Apple silicon machine:
the App Sandbox, which is inert without a signing identity no runner has; a
high-density display; and Finder itself — the document icon, Get Info's Kind,
and a warm double-click into a running window. Those stay below.

### What a hand found on Apple silicon that a runner could not

Run 2026-08-29 on a rented Scaleway Mac mini — Apple M1, **macOS 26.6.1**, two
major versions ahead of the machine everything else here was measured on. The
article was a Developer ID signed universal sandboxed bundle rather than the
Store package, for the reason the section above it records: a Store build cannot
be launched off the Store. Driven over SSH for everything mechanical, with a
person in a Remote Desktop session for everything needing eyes.

**The window, and a document arriving in it.**

    on-screen windows: 21, from 7 applications
    owners: Control Centre, Dock, Finder, Notification Centre, Slipcase,
            Terminal, Window Server
      Slipcase window: 900 x 672 at layer 0
    VERDICT: passed — Slipcase has 1 ordinary window(s) on screen

`check-install.sh` reported `ARM64 on arm64`, so the arm64 slice is what ran
rather than the x86_64 one under Rosetta. `last-folder` read
`/Users/m1/slipcase-kit`, which is only written when a document was delivered —
so `src/opened_document.rs`, the crate's only `unsafe`, works on Apple silicon
inside a signed sandboxed bundle. CI had reached the window; it had never
reached it signed or sandboxed.

**Item 6, and item 5.** Open brought Preview forward showing the PDF. The
handover directory was 0700 and the payload inside it 0644, at
`~/Library/Containers/com.excelano.slipcase-desktop/Data/tmp/slipcase-MPdaTb/`,
with the payload marked `0086;…;slipcase-desktop;` by the platform. Nothing was
written to `/tmp`.

**Item 4, which is the one worth the trip.** A container marked the way Safari
leaves one, opened, edited and saved:

    before  com.apple.quarantine:         0083;68ae0000;Safari;
    after   com.apple.quarantine:         0083;6a93280f;slipcase-desktop;
            com.excelano.slipcase.origin: 0083;68ae0000;Safari;

The sandbox replaced the agent with our own binary's name, as it always does,
and the origin note survived it still naming Safari. The card read *arrived from
elsewhere* before the save and after it. That is `slpc` 0.3.10 working on Apple
silicon, sandboxed, in a signed bundle — the fix having been written and
measured only on x86_64.

**The second volume, which no test can enter.** A 20MB APFS image was mounted and
a marked container opened from it through the open panel, edited and saved.

    before  1090 bytes, com.apple.quarantine: 0083;68ae0000;Safari;
    after   1093 bytes, com.apple.quarantine: 0083;6a932a27;slipcase-desktop;
                        com.excelano.slipcase.origin: 0083;68ae0000;Safari;

The write landed on a volume nobody granted us, `NSItemReplacementDirectory`
left nothing behind on it, the container is still conformant with both members,
and the card still said the container arrived from elsewhere. **Apple's
documented position that the sandbox grant extends to the replacement directory
holds, and is now measured rather than believed.** `packaging/macos/README.md`
had carried it as unresolved.

**Gatekeeper, on a bundle marked the way a download leaves one.** Rejected:
`source=Unnotarized Developer ID`. That is about the Developer ID hedge and says
nothing about the Store, which reaches people through a channel that does not
quarantine. **It means the hedge must be notarized before it could ever be handed
to anyone**, which no note in this repository had said.

**What the hour could not buy.** A high-density display: the instance is
headless at 1920x1080 at 1x, so there is no backing scale of 2 to test against
and the `@2x` item is still open. And a second user account or an upgrade over
an existing install, which need an admin password that was on a portal the
Remote Desktop session had taken the screen from. Neither is architecture
specific, so nothing was lost by not doing them there.

**One defect, found by eye and by nobody's test.** Zooming the window clipped
the row carrying a comment and pushed the control that removes the key off the
right edge. It is written up under *What a long comment does to the row it is
on* below, because it turned out to have nothing to do with zoom, with Apple
silicon, or with that machine.

### What a long comment does to the row it is on

**Found by zooming on Apple silicon, and it is not about zoom.** A row is a key,
a value, whatever comment the document wrote beside it, and the control that
removes the key — laid out in that order. The comment had no width limit, so it
took every point left in the row and the control after it was laid out past the
right edge. Zoom shrinks the points a row has, which is why zooming showed it.

**Reproduced at 1x on an ordinary 1920x1080 display**, by building a container
whose comment is one line and long, which is what established it was nothing to
do with zoom or the machine. Both platforms saw the same thing.

Two fixes, and the first one was wrong. Anchoring the control to the right edge
with a right-to-left layout does stop the clipping, and it spreads every row to
the full window width — which `an_integer_stays_beside_its_key` forbids, for its
own reason, and that test caught it within a minute. The fix that landed caps the
comment instead: it truncates into whatever is left after the room the control
needs, and rows without long comments are laid out exactly as before.

**The regression test had to be rewritten before it was worth anything.** Written
first the obvious way — render into a 900 point width and assert the tree did not
spread past it — it passed with the fix deliberately removed, because egui holds
`min_rect` to the maximum it was given and it reads the same whether the control
landed inside the row or a hundred points past the end of it. The test now goes
through the shapes egui emitted and finds the one drawing the wastebasket. Broken
deliberately again, it fails, and the failure it reports is that the glyph is not
among the shapes **at all**: past the edge, egui culls it rather than drawing it
somewhere unreachable. That is the defect stated exactly — a control the window
offers no way to reach.

### What the light card looked like here

`HANDOFF.md` left this for the two platforms that had never seen it: the contrast
repair is shared code, every Windows walkthrough ran in dark mode, so the light
card had been looked at while it was broken and not once since it was fixed.
macOS had never looked at either. Run 2026-08-28 against the signed sandboxed
bundle with the system switched to Light and switched back after.

**It reads.** One container was made to carry both coloured lines at once — an
encrypted payload for *Cannot be opened here: the member is encrypted (SPEC
2.5)* in the error colour, marked as a download for *This container arrived from
elsewhere, and the payload will carry that.* in the warning colour. Both are
legible against the light card, and Open and Extract are correctly disabled
beside them while Replace stays live.

**Measured off the screen rather than recomputed**, which is the distinction that
section is about. Sampled from the screenshot:

| | On screen | Recorded |
| --- | --- | --- |
| Card fill | rgb(248, 248, 248) | grey 248 |
| Error line | rgb(180, 0, 0) — 6.72:1 | 6.72:1 |
| Warning line | rgb(180, 70, 0) — 5.18:1 | 5.18:1 |

Exactly the figures the repair predicted, so on this display there is no gap at
all between the colour computed and the colour shown.

**The *antialiased worst case* is not a comparable number, and this is where to
say so.** The Windows entry gives 4.59:1 against a colour computing to 5.18:1,
without saying what counts as an ink pixel. It has to be a coverage threshold,
and the answer moves with it — measured here on the warning line: 4.41:1 at 90%
coverage, 4.78:1 at 95%, 5.15:1 at 99%, which brackets Windows's figure rather
than disagreeing with it. Below about half coverage the number collapses toward
the fill by construction and means nothing, which is true of every glyph ever
antialiased. Anybody comparing that figure across platforms is comparing two
unstated thresholds; the core colour is the number that travels.

### What looking at the Dock found

**The Dock showed the egui logo instead of this application's icon**, on every
macOS build ever made here, and it was found on 2026-08-28 by a person glancing
at the Dock while doing something else. Nothing else in this project could have
found it, and several things that look like they should have did not.

What was checked and came back clean, before the Dock itself was looked at:

- `Contents/Resources/slipcase-desktop.icns` holds the card-in-a-case at all ten
  sizes, and `build-app.sh` already verifies each rendering's dimensions.
- `CFBundleIconFile` names it.
- `NSWorkspace.icon(forFile:)` on the bundle returns the correct drawing.
- `NSRunningApplication(processIdentifier:).icon` for the live process returns
  the correct drawing.

Two Cocoa APIs answering correctly while the Dock shows something else is the
whole shape of this defect. The cause is `eframe`: `epi_integration.rs:209`
substitutes its own `data/icon.png` for any viewport that names no icon, and
`app_icon.rs` hands that to `-[NSApplication setApplicationIconImage:]`, which
outranks the bundle. `src/main.rs` called `with_icon` only under
`#[cfg(target_os = "windows")]`, on the correct reasoning that macOS takes its
icon from the bundle — correct about macOS, and silent about eframe.

The repair is to decline rather than to supply: `AppTitleIconSetter::new` turns
an `IconData::default()` into `None`, and the macOS arm calls
`setApplicationIconImage:` only where there is an image, so an empty icon leaves
the bundle's alone. Handing over the drawing a second time would also work and
would carry a redundant copy in the binary in order to overwrite the bundle's
with a worse-scaled equal.

**The before and after are both captured**, which is this defect's substitute for
a test that bites: the Dock tile was a white hexagon on black, and after the
rebuild it is the card-in-a-case, in the same tile position with the same running
dot.

Worth knowing for anyone repeating any of this: `screencapture` works on this
machine now. `What the first run found` above records it returning the desktop
with every window omitted, and two byte-identical empty captures as the only
tell. Screen Recording was granted to the terminal on 2026-08-28 and captures now
show windows, the Dock, and dialogs. That is what made the icon measurable from a
session rather than only describable by a person.


**A postscript, 2026-08-29, because the fix has a visible consequence and it
reads like a defect.** Run the *bare executable* rather than the bundle —
`./target/release/slipcase-desktop`, which is what a developer testing a layout
change does all day — and the Dock shows a black square marked `exec`. That is
macOS's generic icon for a Unix executable with no bundle, and it is correct:
declining the icon is what stops eframe substituting the egui logo, so where
there is no bundle to supply one there is now nothing.

Before the fix the same command showed a white hexagon on black, which looked
like an icon and was the wrong one. Checked here rather than reasoned about: the
bundle launched beside it draws the card-in-a-case correctly with the running
dot beneath it.

Handing the drawing over programmatically would restore an icon for the bare
case, at the price of a second copy of it inside the binary whose only job is to
overwrite the bundle's with a worse-scaled equal. `src/main.rs` says so where the
decision is, and this is the note for whoever sees the Dock before they see the
comment.
### What a Store-signed build did when it was launched

**It was killed by the kernel, and that is the correct answer rather than a
defect.** Run 2026-08-29 against `dist/Slipcase.pkg` built by
`build-app.sh --store` from `b75673b` — universal, sandboxed, signed with Apple
Distribution, wrapped by `productbuild` and signed with the installer
certificate. The bundle inside it was launched to check it before carrying it to
a rented Apple silicon machine. It did not start:

    Exception Type:      EXC_CRASH (SIGKILL (Code Signature Invalid))
    Termination Reason:  CODESIGNING 1 Taskgated Invalid Signature

The reason is in the system log rather than the crash report, and it is exact:

    taskgated-helper  Disallowing com.excelano.slipcase-desktop because
                      no eligible provisioning profiles found
    amfid             not valid: Code=-413 "No matching profile found"
    kernel  (AppleMobileFileIntegrity) Code has restricted entitlements, but
            the validation of its code signature failed.

`com.apple.application-identifier` is a **restricted** entitlement: AMFI will
not honour it without a provisioning profile that covers the machine. A Mac App
Store profile covers none — ours carries no `ProvisionedDevices` key at all,
which was checked by decoding it rather than assumed — because a Store build is
authorised by having arrived from the Store. It is the same rule as iOS, where
an App Store build will not run on a device either, and the log says so in as
many words: *Only Development Provisioning Profiles can be installed in System
Settings. Production Provisioning Profiles are imported within Xcode.*

**Nothing is wrong with the package.** Everything checkable about it is right,
and was checked before the launch rather than after the failure explained
itself: `x86_64 arm64` slices, `app-sandbox` and
`9K6W5PMFYP.com.excelano.slipcase-desktop` read back out of the signature,
`keychain-access-groups` absent as intended, the chain reaching the Apple Root
CA, and `0.1.1 (160)` declared. `packaging/macos/check-install.sh` asks all of
that in one command and was written here for it.

**Two things follow, and the second one costs.**

`spctl -a -vvv` rejects both the app and the package, naming our own
certificates as the origin. That is expected for this channel and must not be
written down as a finding: `spctl` assesses the Developer ID and notarization
policy, which a Store build is not distributed under. The script above says so
rather than counting it.

And the item this file has been carrying as *a distribution-signed bundle
carrying a provisioning profile* — the one `RELEASE.md` calls the run most
likely to find something — **cannot be run by hand at all.** Not on this
machine, not on a rented one, not on any machine a developer can reach with a
copy of the package. The only two contexts that authorise it are the Mac App
Store and TestFlight, and TestFlight is the reachable one: an upload gives the
build a receipt and a matching profile. `SUBMITTING.local.md` had already named
TestFlight as the *cheapest* route to an Apple silicon machine. It is not
cheapest, it is the only one, and that sentence should be read as a requirement.

**What this cost was one launch and an hour that had not been spent yet.** The
plan for the rented machine had been to carry the signed package over and
install it, which would have failed there, on the clock, with the machine
already paid for. It is written down here because the next person to build a
Store package will want to launch it for exactly the same reason.

### What the first upload to App Store Connect found

**Build 165 was accepted and then refused, and the refusal arrived only by
email.** Uploaded 2026-08-29 at 14:42 with `xcrun altool --upload-app`, which
answered `UPLOAD SUCCEEDED with no errors` and a delivery UUID. Nothing then
appeared in App Store Connect — not a processing build, not a failed one,
nothing at all — and twenty-five minutes later the reason was in a mail:

    ITMS-91109: Invalid package contents - The package contains one or more
    files with the com.apple.quarantine extended file attribute, such as
    "…/Slipcase.app/Contents/embedded.provisionprofile". This attribute isn't
    permitted in macOS apps distributed on TestFlight or the App Store.

**The cause is that a provisioning profile is downloaded in a browser.** Ours
carried `0083;6a923c60;Safari;`, macOS `cp` preserves extended attributes, and
`build-app.sh` copied it into the bundle. Exactly one file was affected, which
was confirmed by walking the whole bundle rather than assumed from the message's
one example. It also carried `kMDItemWhereFroms` holding the portal URL with the
team and profile identifiers in it, which would have shipped inside the
application.

Fixed by `xattr -cr` on the assembled bundle before signing, and a refusal
afterwards that names any file still marked. Both directions were run: with the
strip removed the build refuses and names the file; with it in place the bundle
carries no extended attributes at all.

**Three things this cost that are worth more than the fix.**

`altool --validate-app` passed the broken package. It answered `VERIFY SUCCEEDED
with no errors` on the bundle that ingestion then refused, so it checks
structure, signatures and entitlements and does **not** check extended
attributes. A validation pass is a weaker guarantee than its wording suggests,
and this repository had been treating it as the gate before an upload.

**App Store Connect showed nothing, which is the shape of this failure.** A
delivery that is accepted and then fails ingestion leaves no trace in the
interface — the natural reading is that processing is slow. What settled it was
querying the App Store Connect API directly and finding zero builds for the app
and ten for the other two apps in the account. `xcrun altool --generate-jwt`
produces a token the general API **rejects**, because it omits `iat`; a token
made by hand with `openssl` works. `SUBMITTING.local.md` carries the recipe.

**The refusal check was written wrong and hid its own breakage.** `xargs`
answers 123 when the command it ran was false, which is the normal case, and
under `set -eu` that killed the command substitution and the script — silently,
exit 1, no output, on *every* build including clean ones. It read as a success
because the previous `dist/` was still on disk to be measured. `find -exec`
instead. Second time in one day that a stale artefact nearly produced a false
pass; the other was a Windows screenshot question answered against images taken
from an earlier build.

**A good build appears in about ninety seconds.** Build 167 went from
`UPLOAD SUCCEEDED` to `processingState: VALID` inside that, which retrospectively
makes the twenty-five minute wait on 165 diagnostic on its own. It also reported
`usesNonExemptEncryption: False` with nobody answering a form, which is
`ITSAppUsesNonExemptEncryption` in `Info.plist.in` doing its job on the first
upload after it landed.

### What the distribution-signed walkthrough found, through TestFlight

**It passes, and it is a closer artefact than the package we uploaded.** Run
2026-08-29 against build 167 installed from TestFlight on this Intel Mac. All
three of the questions this item existed for answered yes: it launches, Open
brings Preview forward showing the PDF, and a container marked as a download
keeps its *arrived from elsewhere* line across an edit and a Save.

**A fourth kind of build, which nothing here had anticipated.** Apple does not
distribute the bundle we signed. It strips the embedded profile, re-signs, and
what arrives is:

    signed:      TestFlight Beta Distribution
    Gatekeeper:  accepted — source=Testflight
    profile:     none
    entitlements: app-sandbox, files.user-selected.read-write,
                  com.apple.application-identifier, team-identifier

So it carries the restricted entitlement with **no profile at all**, and AMFI
allows it — the receipt is what authorises it instead. That is the mechanism
that explains the section above: our locally signed Store package was killed
because a Mac App Store profile provisions no devices, and this one is not
relying on a profile in the first place.

**It is therefore closer to what the App Store will actually serve than the
`.pkg` is**, because the Store re-signs as well. This run is worth more than the
one it replaced, rather than being a substitute for it, and the entry that called
a hand-run against a distribution-signed bundle *the run most likely to find
something* was retired on the wrong grounds — it was unreachable, and then it
turned out to be reachable by a route that gives a better answer.

`check-install.sh` gained the fourth kind. It had been written for three, said
so in a comment, and was four within a day — so the comment now points at the
list rather than counting it, for the same reason `CLAUDE.md` stopped counting
conformance cases.

**What this does not settle.** The receipt is not validated by this application
and never will be, so nothing here exercises it. And App Review runs a build
signed for the Store rather than for TestFlight; those differ in the certificate
and in nothing else observable from here.

### Not yet done by hand

- **A high-density display, half done.** The `@2x` entries have now been
  checked, by a better method than looking at them: each was re-rendered from
  the SVG at its own pixel size and compared against what the `.icns` holds.
  All five match byte for byte — 32, 64, 256, 512 and 1024 — so none of them is
  an upscale and `build-app.sh`'s size check did its job. The two smallest were
  then looked at as well, since this drawing failed at 48 pixels once on Linux:
  the card-in-a-case is legible at 32 and clean at 64.

  One thing found on the way that looks like a defect and is not.
  `iconutil --convert iconset` hands back a 32-pixel `icon_16x16@2x.png`
  identical to our render and a 32-pixel `icon_32x32.png` that differs. Reading
  the `.icns` explains it: `ic04` and `ic05`, the 16 and 32 pixel 1x entries,
  are stored as raw ARGB while every other entry is PNG, so extracting those two
  re-encodes them. A format difference and not data loss. Written down because
  the next person to hash those files will think they have found something.

  What is still not done is the part that needs a display: the application's own
  interface at 2x. Nothing has ever drawn a frame of it at that scale, and
  `DESIGN.md` §6's layout numbers were all measured at 1x, including the tree
  row that came to 916 pixels in a 900-pixel window and pushed a button off the
  edge. ~~This machine's panel is 2560x1440 at 1x; whether macOS offers it a
  HiDPI scaled mode has not been checked~~ — **struck 2026-08-29, because a
  display is not a property of a machine.** Monitors here get swapped, so a
  resolution written into this file is a fact with a shelf life and the next
  reader has no way to tell it has expired. Ask instead, at the moment it
  matters:

      system_profiler SPDisplaysDataType | grep 'UI Looks like'

  What the item needs is a backing scale of 2, and no display attached to date
  has offered one — the reading the day this was struck was *UI Looks like 1920
  x 1080*, at 1x. So it waits for a Retina display, which is the Apple silicon
  machine the arm64 walkthrough needs anyway.
- **A signed bundle**, partly done. Signing with an Apple Development
  certificate answered the `mdls` question: the type is flagged `trusted`
  rather than `untrusted`, Spotlight reports `com.excelano.slipcase`, and the
  Kind reads `Slipcase container`. Everything above it is an unsigned bundle that
  never left the machine that built it.

  ~~What is still unrun is the walkthrough against a *distribution*-signed
  bundle carrying a provisioning profile.~~ **Struck 2026-08-29: it cannot be
  run by hand, on any machine.** AMFI refuses a restricted entitlement without a
  profile covering the machine, and a Mac App Store profile covers none — see
  *What a Store-signed build did when it was launched*. The only two contexts
  that authorise it are the Mac App Store and TestFlight. **This item is now a
  TestFlight item and belongs to whoever does the upload**, not to somebody with
  a Mac. What was run instead, on Apple silicon, was a *Developer ID* signed
  sandboxed bundle, which is a real signature and a real sandbox and is not the
  same context.
- ~~**A downloaded bundle**, carrying `com.apple.quarantine`~~ — **done
  2026-08-29 on Apple silicon**, against a Developer ID signed bundle marked the
  way a download leaves one. `spctl` rejects it: `source=Unnotarized Developer
  ID`. So **the hedge must be notarized before it is handed to anyone**, which
  nothing here had said. It says nothing about the Store build, which reaches
  people through a channel that does not quarantine.
- ~~**an upgrade over an existing install**~~ — **done 2026-08-29, and nobody
  set it up.** TestFlight installed build 167 over the Developer ID build 164
  that had been copied into `/Applications` half an hour earlier for the
  second-account test. It is a better upgrade than one staged on purpose would
  have been, because the certificate changed as well as the version:
  `Developer ID Application` to `TestFlight Beta Distribution`.

  What was checked afterwards rather than assumed: the bundle is replaced in
  place and reports 167, Launch Services still resolves the association, and the
  per-user state survived — the sandbox container still dates from 2026-08-25
  rather than being remade, and `last-folder` still holds the path the *previous*
  build wrote. So an upgrade does not orphan a container or lose what a person
  had open.
- **A second user account.** Run 2026-08-29 on `davidanderix` against a Developer
  ID build: the double-click reached Slipcase, Open reached Preview, and a marked
  container kept its provenance across a save — **on copies in that account's own
  Drop Box**, which is write-only to everybody else, so the attributes could not
  be read back from the other side. It is a report rather than a measurement and
  is left open for that reason.

  **The run against the shared copies failed, and the cause was the staging.**
  `/Users/Shared/slipcase-test` had been made `1777`. In a sticky directory only
  a file's owner may rename or delete it, and `-[NSFileManager
  replaceItemAtURL:]` replaces rather than writing in place — so the other
  account could write the bytes and not perform the operation Save performs. The
  directory is `0777` now. **What Slipcase showed when that save was refused has
  not been recorded**, and it is the more interesting half: a sandboxed save that
  cannot replace its file is a failure mode no test here provokes, and whether it
  fails safely is a `DESIGN.md` §5 question.
- ~~**A container on a second volume, under the sandbox.**~~ **Done 2026-08-29
  on Apple silicon**, and it passes — a marked container on a mounted APFS image,
  opened through the open panel, edited and saved, with the origin note intact
  afterwards and nothing left behind in `.TemporaryItems`. *What a hand found on
  Apple silicon that a runner could not* holds the run. **Apple's position that
  the sandbox grant extends to `NSItemReplacementDirectory` is now measured
  rather than believed**, which is what this entry was written to refuse to take
  on trust.

  **Two of the three cases it named are still not run**, and they are the two a
  disk image does not stand in for: an external drive, and a network share. A
  share is a different `EXDEV` story again and may not permit `.TemporaryItems`
  at all.
