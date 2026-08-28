# Windows packaging

`DESIGN.md` §8. The extension and the media type registered by the installer,
on the platform that has no freedesktop database to put them in.

    powershell -ExecutionPolicy Bypass -File packaging\windows\install.ps1
    powershell -ExecutionPolicy Bypass -File packaging\windows\uninstall.ps1

`install.ps1 -NoBinary` registers the association without copying an executable,
and `-Prefix DIR` puts the files somewhere else. `uninstall.ps1 -KeepFiles`
removes the association and leaves them.

## What is here

| File | What it is |
| --- | --- |
| `install.ps1` | Writes the registry keys, copies the files, makes the Start menu shortcut |
| `uninstall.ps1` | Removes all of it. Copied into the install directory, because Add/Remove Programs points at it and a checkout may be gone |
| `slipcase.ico` | The icon, nine sizes. Built from the Linux SVG, not drawn separately |
| `assets/` | The five PNGs `AppxManifest.xml` names, from the same SVG. Committed for the same reason the `.ico` is |
| `make-ico/` | The tool that builds both |
| `AppxManifest.xml.in` | The MSIX manifest, with the identity and the version left as placeholders |
| `identity.psd1` | What Partner Center assigned when the name was reserved. The one place those values live |
| `build-msix.ps1` | Builds the package from a release binary, and optionally signs it and runs the certification kit |

## Two scripts rather than an installer

The lightest thing that registers an extension properly and uninstalls cleanly,
which is what the application needs: one executable, one icon, and no runtime
files of its own. MSI through WiX, Inno Setup, and NSIS were all considered and
all rejected for the same reason — each needs a toolchain that is not on a
stock Windows and not in this repository's build, to produce a package that
would do what forty lines of registry writes do. `packaging/linux` is a pair of
shell scripts for the same reason, and these two are its counterpart, argument
for argument.

An MSI becomes worth building when there is a channel to ship it through, the
way `packaging/debian` exists because the Excelano apt repository does. There
is no such channel for Windows yet, and building a package with nowhere to send
it would be guessing at what that channel will want.

**Amended: there is a channel now, and it is the Microsoft Store.** The
paragraph above is left standing because its reasoning was right and it is what
chose the two scripts; what changed is the premise. The Store was taken for the
reason macOS took the Mac App Store: a person who has been sent a container
double-clicks it, Windows offers to search the Store by file type, and outside
the Store that search returns nothing. `packaging/macos/README.md` has the same
argument written out at length.

That decides the format against MSI rather than for it. The Store takes MSIX,
WiX builds MSI, and the two are not steps on one path — so WiX stays rejected,
by the paragraph above and now also by the channel. The two scripts stay as
well. They are the per-user, no-toolchain, no-account route, and a Store
listing is not a reason to take that away from somebody who would rather not
have one.

Three things have to be measured before any of it is built, and none of them is
paperwork. macOS is the reason to say that plainly: the App Sandbox was assumed
to be a formality there, and it turned out to need a new module, a rewritten
save path, and a reopened section of `DESIGN.md`. MSIX is a container too. The
questions are in `CHECKLIST.md` under Windows; the short version is that this
application reads the registry to answer what would open a payload, hands files
to the shell, and registers a file type — and MSIX has its own opinion about
all three.

**Measured: MSIX changes none of the three, and the package still has to clean
up after these scripts.** The paragraph above is left standing because it was
right to ask. Run 2026-08-26 against a signed package built from the release
binary and installed; `CHECKLIST.md` holds the run and the numbers.

`opens_with` gets the same answers inside the container as outside — twenty
payload types, no row different — because MSIX virtualises what a package
writes and not what it reads, which was confirmed by watching a registry write
from inside the container fail to appear outside it. `opener::open` reaches the
shell from inside and the handler starts. And the association declared in
`AppxManifest.xml` works on its own: a double-clicked container launches the
packaged binary with no registry key written by anybody.

What is not free is the overlap. With the package and these scripts both
registered, Windows chooses neither and puts up its *how do you want to open
this file* picker, so installing the package over a live script installation
turns a working association into a prompt. Whether a `UserChoice` a person
actually chose then outranks the manifest is the one part still unmeasured, and
it stays unmeasured on purpose: that key is hash-validated and write-denied, so
it cannot be forged and a person has to make the choice.

**Amended: it was measured later the same day, and this paragraph was stale for
two days without anybody noticing.** David made the choice by hand on
2026-08-26 and `CHECKLIST.md`'s *What the stale UserChoice run found* holds the
result, in three rows. The short version is that the answer depends on what the
stale key points at, which is not a distinction anyone had anticipated:

| `UserChoice` names | What a double-click does |
| --- | --- |
| A ProgID that exists, whose command names a deleted executable | **Refused: *Application not found***. The package is ignored and no picker appears |
| A ProgID that no longer exists at all | The package wins and launches from `WindowsApps` |
| Nothing | The package wins |

Reproduced on 2026-08-28 against a package carrying the real reserved identity,
for the middle row. The other two were not set up again: a fresh human choice is
needed for the first, and this machine's one is spent.

**The middle row is a trap and the first is worse**, and neither is something an
MSIX can clear: a package runs no code at install time, so it cannot remove a
`UserChoice`. That leaves a decision, and it is recorded in `RELEASE.md` rather
than settled here.

Worth noticing how this was found: `CHECKLIST.md` was written and this file, and
`HANDOFF.md`, and `RELEASE.md` all went on saying the question was open. The
record was right and three summaries of it were wrong, which is the argument for
reading the record rather than the summary.

One thing to carry into any check of a packaged install: `AssocQueryString`
answers `ERROR_NO_APPLICATION_ASSOCIATED` for the executable and the command
line of a packaged handler while still returning its friendly names, because
there is no command line — activation goes through the app model. That is the
same trap as `assoc` and `ftype` above, and it means a script that verifies an
install by looking for an executable path will report a correct package as no
association at all.

Getting a package installed at all costs exactly one administrator action:
`makeappx`, `New-SelfSignedCertificate`, `signtool` and `Add-AppxPackage` all
run as an ordinary user, but the signing certificate has to reach
`LocalMachine\TrustedPeople`, and the per-user store is not read for this —
importing there leaves deployment failing `0x800B0109` just the same. The
Developer Mode route is the other way in and needs the same elevation.

**Per-user, under `HKCU` and `%LOCALAPPDATA%`**, which is the counterpart of the
Linux script's default of `~/.local`. There is no all-users variant: the
machine-wide half of every key here needs elevation, and a script that
sometimes needs administrator and sometimes does not is worse than one that
never does.

## What gets written

Everything under `HKEY_CURRENT_USER`.

| Key | Value | Why |
| --- | --- | --- |
| `Software\Classes\.slpc` | `Excelano.Slipcase` | The extension names the type |
| `Software\Classes\.slpc` → `Content Type` | `application/x.slipcase+zip` | SPEC §4's media type, name to type |
| `Software\Classes\MIME\Database\Content Type\application/x.slipcase+zip` → `Extension` | `.slpc` | The same statement, type to name |
| `Software\Classes\Excelano.Slipcase` | `Slipcase Container` | What Explorer's Type column shows |
| `…\DefaultIcon` | `slipcase.ico,0` | What Explorer draws |
| `…\shell\open\command` | `"…\slipcase-desktop.exe" "%1"` | What a double-click runs |
| `…\Application` → `ApplicationName` | `Slipcase` | The name a person recognises |
| `Software\Classes\Applications\slipcase-desktop.exe` | `FriendlyAppName` | The Open With list |
| `Software\Microsoft\Windows\CurrentVersion\Uninstall\Slipcase` | — | Add/Remove Programs |

`Excelano.Slipcase` is chosen here; `.slpc` and `application/x.slipcase+zip`
are not, and come from `SPEC.md` §4, which is the authority. SPEC §4 reserves
no magic bytes, so the extension is the only identification Windows has: there
is no content type to sniff and no `sub-class-of` to fall back on the way
shared-mime-info has one.

`FriendlyTypeName` is written as a plain string. The usual form is a reference
into a binary's resource table — `@C:\path\thing.dll,-123` — which needs
`SHLoadIndirectString` to read back, and this application's own type query
refuses those rather than show a person the reference. Registering one would
have meant `src/opens_with.rs` could not read what the installer wrote.

## Things that were measured rather than assumed

<!-- This heading said "Three" until a fourth arrived. `CLAUDE.md` has a
     paragraph about a count in prose that was wrong for three days and could
     not be checked, so the number is gone rather than incremented. -->

**Windows PowerShell 5.1 reads a script with no byte order mark as ANSI, and an
em dash in a string literal is then a syntax error.** The UTF-8 bytes of `—`
decode under Windows-1252 to `â€"`, and that last character is `U+201D`, which
PowerShell accepts as a *string delimiter* — so the string ends in the middle of
a sentence and the parser reports a missing terminator two hundred lines later.
`build-msix.ps1` hit this on its first run.

The repair is that every string a script prints is ASCII, which it should have
been anyway: these messages go to a console whose code page is nobody's to
predict, and an em dash there is mojibake even when it parses. Comments keep
theirs, because a comment is never parsed as a string — which is why
`install.ps1` has carried one at line 183 from the beginning without anybody
noticing. Adding a byte order mark was the other repair available and was not
taken: none of the three scripts here has one, and one file that differs is a
trap of its own.

**`assoc` and `ftype` do not see any of this.** They report `.slpc` as having no
association at all after a successful install, because they read and write the
machine-wide half of the class root only. They were in this README and in the
script's closing advice first, and printed exactly the message a failed install
would have. `reg query HKCU\Software\Classes\.slpc /s` is the check that works,
and `cargo run --example opens-with -- some.slpc` is the better one: it answers
`Slipcase`, which is this application's own type query reading the registration
this directory just made.

**PowerShell's registry provider cannot write the media type key.** The name is
`application/x.slipcase+zip`, and the provider reads the forward slash as a
path separator: it creates `application` with a child `x.slipcase+zip` and
reports success. Both scripts use `[Microsoft.Win32.Registry]` instead, which
takes the whole string as one name.

**A stale `UserChoice` is the dead association to worry about.** Choosing
"always open with" writes `Software\Microsoft\Windows\CurrentVersion\Explorer\FileExts\.slpc\UserChoice`,
and it outranks every key in the table above. Removing the class keys and
leaving that one behind leaves the extension pointing at a ProgID that no
longer exists — and Windows does not fall back to the machine-wide association
then, it treats the extension as having none. `uninstall.ps1` removes it.

## The icon

`slipcase.ico` is built from `packaging/linux/icons/slipcase-desktop.svg`, which
is the source for every platform's icon and is not duplicated here:

    cd packaging/windows/make-ico && cargo run --release

Nine sizes — 16, 20, 24, 32, 40, 48, 64, 128, 256. The three the shell asks for
are 16, 32, and 48; the rest are those again at the display scalings Windows
offers, plus 256 for the extra-large view. Entries above 48 are stored as PNG
and the rest as bitmaps, which is the convention and saves a quarter of a
megabyte on the 256 alone.

`make-ico` is its own package rather than a member of the application's, so
nothing it depends on reaches the shipped binary. It renders with `resvg` and
assembles with `ico`, both pure Rust; `cargo tree -i cc` finds nothing in it
either.

It was checked at 16, 32, 48, and 256 before it was committed, which is what
`packaging/README.md` asks of any change to the drawing. At 16 the outline goes
grey and the card's rounded corners disappear, and the silhouette still reads.

## The window's own icon, and the taskbar

**Neither egui, eframe, nor winit sets an AppUserModelID.** `APP_ID` in
`src/main.rs` is `with_app_id`, which is Wayland's `xdg_toplevel.set_app_id`
and does nothing at all on Windows. Measured by reading all three crates.

That is left alone deliberately, and the Start menu shortcut carries no
AppUserModelID either. Setting one on the shortcut without the process
declaring the same identity through `SetCurrentProcessExplicitAppUserModelID`
would break the pairing rather than fix it — and that call is raw FFI, which
`#![forbid(unsafe_code)]` puts out of reach. With neither side declaring one,
Windows derives both from the executable's path, they agree, and pinning and
taskbar grouping work.

**The window icon is embedded as bytes, not compiled into a resource.** Windows
takes a window's icon from a resource in the executable, and building one needs
`rc.exe` or `windres` — a build step `DESIGN.md` §2 keeps out. So `main.rs`
carries `slipcase.ico` through `include_bytes!` and hands the 64-pixel entry to
the window at startup: 64 is a whole multiple of the sizes a display at 100% or
200% asks for — 16 and 32 in the title bar, 32 and 64 in the task bar — so each
of those is an integer downsample rather than a resample of a resample.

**Amended: not of every size, and this was written before anybody had looked.**
125% asks for 20 and 40 and 150% for 24 and 48, and 64 divides none of them, so
those are resampled. Looked at on 2026-08-26 at 125% and 200%: both read
cleanly, so the cost is nothing a person notices and 64 stays the choice,
because it is the largest entry no scaling has to enlarge. `CHECKLIST.md` holds
the run.

This is why `slipcase.ico` is a committed artifact in a repository that
otherwise holds only sources: the executable references it at compile time, and
Windows has no step that would rasterize the SVG for either purpose.

## What a Store build would need

**The account exists; nothing Windows does.** There is a Microsoft Partner
Center account, so registration, identity verification and the agreements are
behind us — which matters because verification is the one step here measured in
days rather than minutes. Recorded because a session that has to ask this
question loses an afternoon to it, and the macOS README carries the same
sentence about Apple for the same reason.

**What is absent is the build, and it is now half absent.**
`AppxManifest.xml.in` landed on 2026-08-28, templated the way `control.in` and
`Info.plist.in` are, with four placeholders: three identity values Partner
Center assigns and the version, which `packaging/version.sh` answers when asked
for the appx spelling. What is still missing is `build-msix.ps1`. The MSIX that
answered the three questions in `CHECKLIST.md` was built by hand at a prompt and
nothing in the tree rebuilds it.

**Amended: `build-msix.ps1` landed on 2026-08-28 and the tree rebuilds it now.**
It takes a release binary, stages the executable, the manifest and the assets,
substitutes the identity and the version, and calls `makeappx`.
`packaging/macos/build-app.sh` is the model, and so is its rule: it refuses
rather than producing something subtly wrong. Four refusals were verified by
breaking what each one guards.

Two of the four are read out of the PE header, because neither is visible in a
finished package. The architecture, because the manifest declares x64. And the
subsystem, because `src/main.rs` carries `windows_subsystem = "windows"` only
when `debug_assertions` is off — so a debug binary packaged by mistake is a
console subsystem one, and *a console window behind the application* is a defect
this platform's walkthrough already found by hand once. Packaging
`target\debug\slipcase-desktop.exe` is refused, which was measured rather than
reasoned about.

The other two are the manifest's: a placeholder that survived substitution, and
a `Publisher` that is not an X.500 string. The second is the display name typed
into the wrong field, which is the identity mistake that gets rejected at upload
rather than at review.

**The manifest names image assets that do not exist yet.** `StoreLogo.png`,
`Square150x150Logo.png`, `Square44x44Logo.png`, `Wide310x150Logo.png` and
`slipcase.png`, all under `Assets\`. `packaging/windows/slipcase.ico` is the
source for the last of those and `packaging/linux/icons/` holds the scalable
originals for the rest, so nothing has to be drawn — but something has to
produce PNGs at the sizes the Store asks for, and that belongs in the build
script beside everything else mechanical. The `.ico` generator in
`packaging/windows/make-ico` is the model: committed output, checked in CI
against a rebuild.

**Amended: they exist, and the model was taken rather than imitated.** The
paragraph above put the work in the build script and then named `make-ico` as
the model, and those are two different places. `make-ico` won, because a build
script that rasterizes has to carry a rasterizer, and nothing on Windows does
that from a shell: the argument that gave this directory an icon converter in
the first place gives it these five PNGs too. It writes `../assets` beside the
`.ico` now, `windows.yml` compares the whole directory against a rebuild, and
`build-msix.ps1` only copies. The `.ico` is byte-identical across the change,
which is the check that the shared renderer did not quietly alter it.

**One thing about those assets is guidance rather than measurement.** The
dimensions are the Store's and are not a choice. How much of each canvas the
drawing occupies is: a tile is drawn on a coloured plate and Microsoft's tile
guidance leaves the icon about two thirds of it, where an icon-shaped asset is
drawn at the size it is given and wants all of it — which is also what
`slipcase.ico` does at every size it holds. So the two square tiles and the wide
one are at two thirds and the rest fill their canvas. **Nobody has looked at a
real tile**, and until somebody has, that split is a reading of a document.
`CHECKLIST.md` is where the look goes.

**The association in the manifest mirrors `install.ps1` deliberately.** Same
extension, same content type, same friendly name. If the packaged application
and a side-loaded one ever claim `.slpc` differently, a person with both sees
the wrong one win and has nothing to explain it.

**Three values have to come out of Partner Center before a manifest is real.**
`Package/Identity/Name`, `Publisher` and `PublisherDisplayName` are assigned
when the name is reserved and must appear in `AppxManifest.xml` exactly as
Partner Center gives them; a package whose identity disagrees is rejected at
upload rather than at review. Reserving the name is cheap and blocks the listing
work, so it is worth doing before the manifest is written rather than after.

**Done: `Slipcase` was reserved on 2026-08-28 and the three values are in
`identity.psd1`.** They went there rather than into `AppxManifest.xml.in`
because `RELEASE.md` asked for one place, and because more than the manifest
wants them: the package family name is what `Get-AppxPackage` is asked for when
checking an install, and `build-msix.ps1 -SelfSign` builds a certificate subject
out of `Publisher` rather than having it typed a second time — `signtool`
refuses a package whose manifest publisher and certificate subject differ, and
the throwaway certificate left over from the 2026-08-26 measurement carries a
subject invented before the reservation existed and cannot sign this.

**`identity.psd1` is not committed and `identity.psd1.example` is.** None of
those values is a credential — `Publisher` appears in the manifest of every
package the Store distributes and the store id is in the public listing URL —
and the first version of this file was committed on exactly that reasoning. It
is still true and it was still the wrong call: these are an account's
identifiers on a public record, which is the judgement
`packaging/macos/SUBMITTING.local.md` already got, and *not secret* is a weaker
claim than *belongs in public*. The commit was rewritten before it was pushed so
the values were never published — `CLAUDE.md` records what taking an identifier
back out of this history has cost twice, once after pushing, and neither time
was it cheap.

What is committed is the template. `build-msix.ps1` names it in the refusal
rather than reporting a missing path, because a build script whose first failure
is *no such file* teaches nothing to the checkout that hit it. The MSA
application id on the same Partner Center page is needed by nothing here and is
recorded nowhere.

**The self-signed certificate does not carry forward, and that is fine.**
`New-SelfSignedCertificate` and `signtool` were how the questions got answered
on one machine; the Store signs what it distributes, so a submission uploads a
package it has not signed itself. The measurement was never wasted — it is what
established that the container does not change what `opens_with` sees — but no
certificate from it goes near a submission.

**The version is not the one in `Cargo.toml`.** A Store package declares
`Major.Minor.Build.Revision` and the revision must be `0`, which is a different
shape from the crate version and from what macOS wants in
`CFBundleShortVersionString`. One number, three spellings, and the scheme is
worth deciding once rather than per platform.

**Unrun: the Windows App Certification Kit.** Certification runs it and a
submission that fails it comes back. It has never been run here, and it is
mechanical, so it belongs in a build script rather than in somebody's memory.

**It is in the build script now, behind `-Certify`, and it is still unrun.**
`appcert.exe` is stock on this machine at `C:\Program Files (x86)\Windows Kits\
10\App Certification Kit\`, so nothing has to be installed. What it needs is
elevation, and it installs the package it tests, so it needs a signed one as
well — `-Certify` refuses without `-SelfSign` rather than producing a kit run
against nothing.

The verdict is read out of the report's `OVERALL_RESULT` rather than out of an
exit code, and a report with no verdict in it is a refusal too. A kit that ran
and failed and a kit that never ran are different things, and the second must
never be reported as a pass — which is the same failure `preflight.sh` found in
its own CI check, where a run still in progress was being counted as a result.

**Amended: run three times on 2026-08-28, and both of those sentences turned out
to be missing a case.** `CHECKLIST.md` holds the runs.

*Stale* is a third state that neither *ran and failed* nor *never ran* covers.
`appcert` refuses to overwrite an existing report and stops before running a
single test, so the second run parsed the first run's file and printed its
findings as though they were the new package's. The report is deleted before the
kit starts and the one that appears must be newer than the run.

And the gate refused on any test that was not PASS, which meant it refused every
time: `Blocked executables` fails on every run this application will ever do.
That is the *red is the normal state* problem this file's own rule about compiled
C is written around. The known findings are named in `KNOWN_FINDINGS` at the top
of the script with what each was traced to, and the gate fires on a finding that
is new or worse than recorded. Naming one there is not accepting it — that
decision is `RELEASE.md`'s.

`-ReadReport <path>` applies the gate to an existing report and does nothing
else, which is how the gate gets checked without an elevated session: take a
finding out of the list and watch it refuse.
