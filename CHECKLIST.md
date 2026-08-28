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
   records mode 0644. No line. Then open any container a Windows tool wrote, or
   `accept/name-cp437-bit11-clear.slpc`, and check there is still no line: the
   silence for a container that records no mode is the whole reason
   `payload_mode` reads the external attributes rather than asking the ZIP crate,
   which would have invented `0o664` and answered confidently.
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

- **All six items, on all three platforms.** Written the day the lines landed
  and run on none of them. The Linux build launches against both fixtures
  without panicking and draws a window, which is what could be checked from a
  session with no way to take a screenshot — GNOME refused the capture — and it
  is not the check.

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

Looked at as well as measured, which is the point of this file: the line is a
deeper rust orange on the light card, plainly legible and still plainly a
warning rather than body text. The rendered pixels are `rgb(180,70,0)`, the
colour asked for.

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

Nothing else is waiting. Every item this section held on 2026-08-26 was run
that day: provenance, the window, the stale `UserChoice`, a scaled display, an
upgrade, and a second account. Anything added below should say what it needs
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
  edge. This machine's panel is 2560x1440 at 1x; whether macOS offers it a HiDPI
  scaled mode has not been checked, and failing that it waits for the Apple
  silicon machine the arm64 walkthrough needs anyway.
- **A signed bundle**, partly done. Signing with an Apple Development
  certificate answered the `mdls` question: the type is flagged `trusted`
  rather than `untrusted`, Spotlight reports `com.excelano.slipcase`, and the
  Kind reads `Slipcase container`. What is still unrun is the walkthrough
  against a *distribution*-signed bundle carrying a provisioning profile, which
  is a different sandbox context from the development-signed one everything
  else here was measured against. Everything above is an unsigned bundle that never left
  the machine that built it. `mdls` reporting the wrong type is suspected to be
  a consequence of that and is untested either way.
- **A downloaded bundle**, carrying `com.apple.quarantine`, to see what
  Gatekeeper actually shows a person rather than what `spctl` reports.
- **A second user account**, and an upgrade over an existing install.
- **A container on a second volume, under the sandbox.** The section above
  settles this for the plain build across four filesystems and a test now holds
  it. What a test cannot enter is the sandbox, and the sandbox is the reason
  this module exists. `NSItemReplacementDirectory` is documented as the
  sandbox-safe way to do this and the directory it hands back on a second volume
  is one nobody granted us — `/Volumes/…/.TemporaryItems/…` is not the file the
  person chose through the open panel. Apple's position is that the grant
  extends to it; this repository does not take documentation for a measurement.
  Open a container from a mounted image in the signed bundle, edit a key, and
  save. Then do it from an external drive and a network share, which are the two
  the images here do not stand in for: a share is a different `EXDEV` story
  again and may not permit `.TemporaryItems` at all.
